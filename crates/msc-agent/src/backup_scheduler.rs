//! P6.17's scheduled-backup runtime: a per-server tokio interval driving
//! `msc_application::backups::scheduled_tick`'s policy, plus "live
//! reconfiguration" — [`BackupScheduler::reconfigure`] can be called
//! again with an updated server list, and a server whose
//! `auto_backup_enabled`/`auto_backup_interval_minutes` changed gets its
//! timer replaced. This mirrors `AppViewModel+ServerControls.swift`'s
//! `startAutoBackupTimer(for:)`/`stopAutoBackupTimer()`/
//! `setAutoBackupEnabled`/`setAutoBackupInterval` (source lines 761-825)
//! with one deliberate simplification: source ties the timer's own
//! lifecycle to that one server actually being started/stopped (`Timer`
//! objects created in `startServer`, invalidated in `stopServer`), since
//! MSC 1 only ever runs one server process at a time. This port instead
//! runs one independent, always-on interval per *configured* (not
//! necessarily running) server — [`fire`]'s own `is_running` check is
//! what actually gates whether a tick does anything, so the observable
//! outcome (no backup while stopped) is identical, without needing to
//! synchronize timer creation with server-process lifecycle at all.
//!
//! `build_app()` (`main.rs`) constructs one [`BackupScheduler`] at
//! startup and calls [`BackupScheduler::reconfigure`] once against
//! whatever config exists at boot. Re-calling it whenever a server's
//! auto-backup settings change over the API (`POST /v1/settings`) is
//! P6.21's route-wiring job — flagged, not silent, the same deferred-
//! wiring shape this phase has used throughout (`worlds::activate_slot`'s
//! backup closure, `backups::BackupConsole`'s missing production impl).
//!
//! [`BackupScheduler`] itself only ever calls into [`SchedulerBackend`] —
//! every fixture-relevant decision (skip-when-not-running, skip-when-
//! no-players, backup creation, retention) already lives in
//! `msc_application::backups::scheduled_tick` and is tested there
//! (`crates/msc-application/tests/backup_retention.rs`); this module's
//! own tests (`crates/msc-agent/tests/backup_scheduler.rs`) cover only
//! the tokio-driven cadence and reconfiguration this crate adds, against
//! a scripted fake — the real [`LiveSchedulerBackend`] wiring below has
//! no test of its own, the same "real service wiring, no unit test"
//! precedent `main.rs::run_service`/`build_app` already set.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use msc_application::backups::ScheduledTickOutcome;
use msc_domain::app_config_schema::ConfigServer;
use msc_infrastructure::fs::FileSystem;
use tokio::task::JoinHandle;

use crate::routes::lifecycle::{AgentAppConfigStore, LifecycleRoutesState};

/// What one tick needs to decide/do for a given server, and the
/// exclusivity gate this step's own scope calls for ("Scheduler ticks
/// enter through operation exclusivity"). No production exclusivity
/// check is wired yet — [`LiveSchedulerBackend::admit_backup`] always
/// returns `true` today; wiring it to a real `OperationJournal`/
/// `LifecycleOperations` admission (so a scheduled tick can't race an
/// in-flight activation/restore) is P6.21's job, once backup routes
/// exist to share that journal with — the same deferred-exclusivity
/// note `worlds::activate_slot`'s own section doc already left for this
/// exact moment.
pub trait SchedulerBackend: Send + Sync {
    fn is_running(&self, server_id: &str) -> bool;
    fn online_player_count(&self, server_id: &str) -> usize;
    fn admit_backup(&self, server_id: &str) -> bool;
    fn run_scheduled_backup(&self, server_id: &str);
}

/// `fire`'s own gate order — running, then players, then admission —
/// each cheaper than the next, so a quiet or already-busy server never
/// pays for a filesystem-touching `run_scheduled_backup` call it was
/// always going to skip.
fn fire(server_id: &str, backend: &dyn SchedulerBackend) {
    if !backend.is_running(server_id) {
        return;
    }
    if backend.online_player_count(server_id) == 0 {
        return;
    }
    if !backend.admit_backup(server_id) {
        return;
    }
    backend.run_scheduled_backup(server_id);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduleKey {
    interval_minutes: i64,
}

pub struct BackupScheduler {
    backend: Arc<dyn SchedulerBackend>,
    tasks: Mutex<HashMap<String, (ScheduleKey, JoinHandle<()>)>>,
}

impl BackupScheduler {
    pub fn new(backend: Arc<dyn SchedulerBackend>) -> Self {
        Self {
            backend,
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// Starts/stops/restarts per-server interval tasks to match
    /// `servers` — "live reconfiguration". A server whose
    /// `auto_backup_enabled`/`auto_backup_interval_minutes` changed gets
    /// its task aborted and restarted with the new interval (every other
    /// input — max-count, association, running/player state — is
    /// re-derived fresh from [`SchedulerBackend`] on each tick, so only
    /// the interval itself needs a restart to take effect); a
    /// disabled/removed server's task is aborted outright; an unchanged
    /// server's task is left running untouched, so reconfiguring one
    /// server never resets another's in-flight timer.
    pub fn reconfigure(&self, servers: &[ConfigServer]) {
        let mut tasks = self.tasks.lock().unwrap();

        let desired: HashMap<String, ScheduleKey> = servers
            .iter()
            .filter(|s| s.auto_backup_enabled && s.auto_backup_interval_minutes > 0)
            .map(|s| {
                (
                    s.id.clone(),
                    ScheduleKey {
                        interval_minutes: s.auto_backup_interval_minutes,
                    },
                )
            })
            .collect();

        let stale: Vec<String> = tasks
            .iter()
            .filter(|(id, (key, _))| desired.get(*id) != Some(key))
            .map(|(id, _)| id.clone())
            .collect();
        for id in stale {
            if let Some((_, handle)) = tasks.remove(&id) {
                handle.abort();
            }
        }

        for (id, key) in desired {
            if tasks.contains_key(&id) {
                continue;
            }
            let backend = self.backend.clone();
            let server_id = id.clone();
            let handle = tokio::spawn(async move {
                run_server_loop(server_id, key.interval_minutes, backend).await;
            });
            tasks.insert(id, (key, handle));
        }
    }

    #[cfg(test)]
    pub fn scheduled_server_ids(&self) -> Vec<String> {
        self.tasks.lock().unwrap().keys().cloned().collect()
    }
}

impl Drop for BackupScheduler {
    fn drop(&mut self) {
        for (_, handle) in self.tasks.lock().unwrap().drain().map(|(_, v)| v) {
            handle.abort();
        }
    }
}

async fn run_server_loop(
    server_id: String,
    interval_minutes: i64,
    backend: Arc<dyn SchedulerBackend>,
) {
    let period = Duration::from_secs((interval_minutes.max(1) as u64) * 60);
    let mut interval = tokio::time::interval(period);
    // `Timer.scheduledTimer(repeats: true)` fires only after the first
    // full interval elapses — `tokio::time::interval`'s own first tick
    // completes immediately, so this discards that too-early one.
    interval.tick().await;
    loop {
        interval.tick().await;
        fire(&server_id, backend.as_ref());
    }
}

/// The real [`SchedulerBackend`]: bridges a running server's
/// `LifecycleRoutesState` (process/player state) and `AgentAppConfigStore`
/// (persisted server config) to `msc_application::backups::scheduled_tick`.
pub struct LiveSchedulerBackend {
    lifecycle: LifecycleRoutesState,
    app_config: &'static AgentAppConfigStore,
    fs: &'static dyn FileSystem,
}

impl LiveSchedulerBackend {
    pub fn new(
        lifecycle: LifecycleRoutesState,
        app_config: &'static AgentAppConfigStore,
        fs: &'static dyn FileSystem,
    ) -> Self {
        Self {
            lifecycle,
            app_config,
            fs,
        }
    }

    fn find_server(&self, server_id: &str) -> Option<ConfigServer> {
        self.app_config
            .servers()
            .into_iter()
            .find(|s| s.id == server_id)
    }
}

impl SchedulerBackend for LiveSchedulerBackend {
    fn is_running(&self, server_id: &str) -> bool {
        self.lifecycle.active_server_id().as_deref() == Some(server_id)
            && self.lifecycle.status_snapshot().running
    }

    fn online_player_count(&self, server_id: &str) -> usize {
        if self.lifecycle.active_server_id().as_deref() != Some(server_id) {
            return 0;
        }
        self.lifecycle
            .performance_snapshot()
            .players_online
            .unwrap_or(0)
            .max(0) as usize
    }

    fn admit_backup(&self, _server_id: &str) -> bool {
        // No production exclusivity check wired yet -- see this
        // module's own doc.
        true
    }

    fn run_scheduled_backup(&self, server_id: &str) {
        let Some(server) = self.find_server(server_id) else {
            return;
        };
        let server_dir = PathBuf::from(&server.server_dir);
        let raw_level_name = msc_application::worlds::read_java_level_name(self.fs, &server_dir);
        let slots = msc_infrastructure::world_store::load_slots(self.fs, &server_dir);
        let explicit =
            msc_infrastructure::world_store::load_explicit_active_slot_id(self.fs, &server_dir);
        let active_slot_id = msc_domain::world::resolve_active_slot_id(&slots, explicit.as_deref());
        let association = msc_domain::world::effective_backup_association(
            &slots,
            active_slot_id.as_deref(),
            None,
            None,
        );

        let outcome = msc_application::backups::scheduled_tick(
            self.fs,
            &server_dir,
            server.server_type,
            raw_level_name.as_deref(),
            &association,
            Some(server.id.as_str()),
            Some(server.display_name.as_str()),
            server.auto_backup_max_count,
            &iso8601_now(),
            self.is_running(server_id),
            self.online_player_count(server_id),
        );

        if let ScheduledTickOutcome::Fired(Err(error)) = outcome {
            eprintln!("[backup-scheduler] scheduled backup failed for {server_id}: {error}");
        }
    }
}

fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86_400;
    let remainder = secs % 86_400;
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` — the same small duplicate this
/// codebase already carries in `routes/servers.rs`/`routes/lifecycle.rs`
/// rather than adding a shared date/time dependency for one formatted
/// timestamp.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

/// This crate has no `lib.rs` — its own tests otherwise only reach the
/// compiled binary as a black-box process (`tests/cli_lifecycle.rs`,
/// `tests/startup_secret_migration.rs`'s own `CARGO_BIN_EXE_msc`
/// pattern), which can't observe a scheduler tick without a real
/// wall-clock wait per case. These unit tests live inside the crate
/// instead, compiled with `cargo nextest run -p msc-agent` like any
/// other in-binary test — the plan's own substring Verify filter
/// (`backup_scheduler`) matches them by this module's own path
/// regardless of which binary/test-target they're embedded in, the same
/// way it matches an external `tests/backup_scheduler.rs` file. That
/// external file still exists (a real, `CARGO_BIN_EXE_msc`-driven smoke
/// test proving `build_app()`'s new scheduler wiring doesn't crash
/// startup) — this module is what actually exercises the scheduler's
/// own logic, fast and deterministically, via `tokio::time`'s paused
/// clock rather than a real 60-second wait.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration as StdDuration;

    #[derive(Default)]
    struct FakeSchedulerBackend {
        running: Mutex<HashMap<String, bool>>,
        players: Mutex<HashMap<String, usize>>,
        admit: Mutex<HashMap<String, bool>>,
        fire_count: AtomicUsize,
        fired_ids: Mutex<Vec<String>>,
    }

    impl FakeSchedulerBackend {
        fn new() -> Self {
            Self::default()
        }

        fn set_running(&self, id: &str, running: bool) {
            self.running.lock().unwrap().insert(id.to_string(), running);
        }

        fn set_players(&self, id: &str, count: usize) {
            self.players.lock().unwrap().insert(id.to_string(), count);
        }

        fn set_admit(&self, id: &str, admit: bool) {
            self.admit.lock().unwrap().insert(id.to_string(), admit);
        }

        fn fire_count(&self) -> usize {
            self.fire_count.load(Ordering::SeqCst)
        }

        fn fired_ids(&self) -> Vec<String> {
            self.fired_ids.lock().unwrap().clone()
        }
    }

    impl SchedulerBackend for FakeSchedulerBackend {
        fn is_running(&self, server_id: &str) -> bool {
            self.running
                .lock()
                .unwrap()
                .get(server_id)
                .copied()
                .unwrap_or(false)
        }

        fn online_player_count(&self, server_id: &str) -> usize {
            self.players
                .lock()
                .unwrap()
                .get(server_id)
                .copied()
                .unwrap_or(0)
        }

        fn admit_backup(&self, server_id: &str) -> bool {
            self.admit
                .lock()
                .unwrap()
                .get(server_id)
                .copied()
                .unwrap_or(true)
        }

        fn run_scheduled_backup(&self, server_id: &str) {
            self.fire_count.fetch_add(1, Ordering::SeqCst);
            self.fired_ids.lock().unwrap().push(server_id.to_string());
        }
    }

    fn server(id: &str, enabled: bool, interval_minutes: i64) -> ConfigServer {
        let mut s = ConfigServer::new(id, id, "/tmp/server", "/tmp/server/paper.jar", 1.0, 2.0);
        s.auto_backup_enabled = enabled;
        s.auto_backup_interval_minutes = interval_minutes;
        s
    }

    // ---- fire()'s own gate order — plain sync, no tokio needed ----

    #[test]
    fn fire_skips_when_not_running() {
        let backend = FakeSchedulerBackend::new();
        backend.set_running("s1", false);
        backend.set_players("s1", 5);
        fire("s1", &backend);
        assert_eq!(backend.fire_count(), 0);
    }

    #[test]
    fn fire_skips_when_no_players_online() {
        let backend = FakeSchedulerBackend::new();
        backend.set_running("s1", true);
        backend.set_players("s1", 0);
        fire("s1", &backend);
        assert_eq!(backend.fire_count(), 0);
    }

    #[test]
    fn fire_skips_when_admission_refused() {
        let backend = FakeSchedulerBackend::new();
        backend.set_running("s1", true);
        backend.set_players("s1", 3);
        backend.set_admit("s1", false);
        fire("s1", &backend);
        assert_eq!(backend.fire_count(), 0);
    }

    #[test]
    fn fire_runs_scheduled_backup_when_every_gate_passes() {
        let backend = FakeSchedulerBackend::new();
        backend.set_running("s1", true);
        backend.set_players("s1", 3);
        fire("s1", &backend);
        assert_eq!(backend.fire_count(), 1);
        assert_eq!(backend.fired_ids(), vec!["s1".to_string()]);
    }

    // ---- BackupScheduler cadence/reconfiguration, tokio paused-time driven ----

    #[tokio::test(start_paused = true)]
    async fn scheduler_fires_after_first_full_interval_not_immediately() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        backend.set_running("s1", true);
        backend.set_players("s1", 1);
        let scheduler = BackupScheduler::new(backend.clone());
        scheduler.reconfigure(&[server("s1", true, 1)]);

        tokio::task::yield_now().await;
        assert_eq!(
            backend.fire_count(),
            0,
            "must not fire before the first interval elapses"
        );

        tokio::time::advance(StdDuration::from_secs(60)).await;
        tokio::task::yield_now().await;
        assert_eq!(backend.fire_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_ticks_repeatedly_at_the_configured_interval() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        backend.set_running("s1", true);
        backend.set_players("s1", 1);
        let scheduler = BackupScheduler::new(backend.clone());
        scheduler.reconfigure(&[server("s1", true, 1)]);
        // The freshly spawned task needs one scheduler pass to consume
        // its own immediate first tick and park on the real deadline
        // before `advance` can reliably drive it.
        tokio::task::yield_now().await;

        for expected in 1..=3 {
            tokio::time::advance(StdDuration::from_secs(60)).await;
            tokio::task::yield_now().await;
            assert_eq!(backend.fire_count(), expected);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_does_not_start_a_task_for_a_disabled_server() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        let scheduler = BackupScheduler::new(backend);
        scheduler.reconfigure(&[server("s1", false, 30)]);
        assert!(scheduler.scheduled_server_ids().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_reconfigure_stops_a_now_disabled_server() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        let scheduler = BackupScheduler::new(backend);
        scheduler.reconfigure(&[server("s1", true, 1)]);
        assert_eq!(scheduler.scheduled_server_ids(), vec!["s1".to_string()]);

        scheduler.reconfigure(&[server("s1", false, 1)]);
        assert!(scheduler.scheduled_server_ids().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_reconfigure_leaves_an_unchanged_server_running() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        backend.set_running("s1", true);
        backend.set_players("s1", 1);
        let scheduler = BackupScheduler::new(backend.clone());
        scheduler.reconfigure(&[server("s1", true, 1)]);
        tokio::task::yield_now().await;

        // Advance close to, but not past, the first interval.
        tokio::time::advance(StdDuration::from_secs(59)).await;
        tokio::task::yield_now().await;

        // Reconfiguring with the SAME key must not reset progress toward
        // that first tick.
        scheduler.reconfigure(&[server("s1", true, 1)]);
        tokio::time::advance(StdDuration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(backend.fire_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_reconfigure_restarts_a_server_whose_interval_changed() {
        let backend = Arc::new(FakeSchedulerBackend::new());
        backend.set_running("s1", true);
        backend.set_players("s1", 1);
        let scheduler = BackupScheduler::new(backend.clone());
        scheduler.reconfigure(&[server("s1", true, 5)]);
        tokio::task::yield_now().await;

        // Almost due under the OLD 5-minute interval.
        tokio::time::advance(StdDuration::from_secs(4 * 60)).await;
        tokio::task::yield_now().await;

        // Interval changes to 1 minute -- the task restarts, so the next
        // fire is 1 minute from NOW, not 1 minute left on the old timer.
        scheduler.reconfigure(&[server("s1", true, 1)]);
        tokio::task::yield_now().await;
        tokio::time::advance(StdDuration::from_secs(60)).await;
        tokio::task::yield_now().await;
        assert_eq!(backend.fire_count(), 1);
    }
}
