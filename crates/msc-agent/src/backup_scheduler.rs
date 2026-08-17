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
//! the timer-fired skip-when-not-running/skip-when-no-players policy
//! this module's own [`fire`] re-derives is fixture-tested in isolation
//! against `msc_application::backups::scheduled_tick`
//! (`crates/msc-application/tests/backup_retention.rs`), but P6.31
//! stopped [`LiveSchedulerBackend::run_scheduled_backup`] from reaching
//! production backup creation *through* `scheduled_tick` — that function
//! always hands `create_backup` `console: None` and `should_cancel: ||
//! false`, so a live server's flush/pause protocol and cooperative
//! cancellation could never run on a scheduled tick, and nothing gated
//! it against a concurrent activation/restore/conversion/replacement/
//! backup on the same server. [`LiveSchedulerBackend::run_scheduled_backup`]
//! now calls `crate::backup_operations::start_backup` directly instead —
//! the same shared entry point `routes/backups.rs::now` uses — which
//! journals a real per-server operation, builds a real
//! `LiveBackupConsole`, and wires a real `should_cancel`. This module's
//! own tests (`crates/msc-agent/tests/backup_scheduler.rs`, plus the
//! `mod tests` below) cover the tokio-driven cadence/reconfiguration
//! this crate adds and (below) the real exclusivity `start_backup` now
//! provides; `backup_retention.rs`'s coverage of `scheduled_tick` itself
//! stays valid as a description of the timer policy, just no longer the
//! path a real scheduled backup travels.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use msc_domain::app_config_schema::ConfigServer;
use tokio::task::JoinHandle;

use crate::routes::lifecycle::{AgentAppConfigStore, LifecycleRoutesState};

/// What one tick needs to decide/do for a given server. P6.31 closed the
/// exclusivity gate this module's own doc used to flag as unwired:
/// [`LiveSchedulerBackend::run_scheduled_backup`] now reaches production
/// backup creation through `crate::backup_operations::start_backup`,
/// which performs the identical `LifecycleOperations::begin_running`
/// per-server admission every other Phase 6 mutation route already
/// makes — so a scheduled tick that loses the race against an in-flight
/// activation/restore/conversion/replacement/backup is refused, not
/// merely logged.
pub trait SchedulerBackend: Send + Sync {
    fn is_running(&self, server_id: &str) -> bool;
    fn online_player_count(&self, server_id: &str) -> usize;
    fn run_scheduled_backup(&self, server_id: &str);
}

/// `fire`'s own gate order — running, then players — each cheaper than
/// the next, so a quiet server never pays for a filesystem-touching
/// `run_scheduled_backup` call it was always going to skip. Real
/// exclusivity admission happens inside `run_scheduled_backup` itself
/// (see this module's own doc) — cheaper than a separate pre-check here
/// since it's the same call that would have to run to actually start the
/// backup anyway.
fn fire(server_id: &str, backend: &dyn SchedulerBackend) {
    if !backend.is_running(server_id) {
        return;
    }
    if backend.online_player_count(server_id) == 0 {
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
/// `LifecycleRoutesState` (process/player state, and now — P6.31 —
/// operation admission/journaling) and `AgentAppConfigStore` (persisted
/// server config) to `crate::backup_operations::start_backup`.
pub struct LiveSchedulerBackend {
    lifecycle: LifecycleRoutesState,
    app_config: &'static AgentAppConfigStore,
}

impl LiveSchedulerBackend {
    pub fn new(lifecycle: LifecycleRoutesState, app_config: &'static AgentAppConfigStore) -> Self {
        Self {
            lifecycle,
            app_config,
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

    /// Reached only once [`fire`] has already confirmed `server_id` is
    /// running with players online — `running: true` below reflects that
    /// already-known state, matching every other caller in this codebase
    /// that resolves liveness itself rather than asking the callee to
    /// re-derive it (`crate::backup_operations::start_backup`'s own
    /// `running` parameter doc).
    fn run_scheduled_backup(&self, server_id: &str) {
        let Some(server) = self.find_server(server_id) else {
            return;
        };
        let max_count = server.auto_backup_max_count;
        match crate::backup_operations::start_backup(
            &self.lifecycle,
            server,
            true,
            true,
            Some(max_count),
        ) {
            Ok(_) => {}
            Err(msc_application::operations::LifecycleOperationError::Conflict(_)) => {
                // Another operation already holds this server's
                // exclusivity (activation, restore, conversion,
                // replacement, or another backup) -- this tick is
                // skipped, not queued; the next tick tries again.
            }
            Err(error) => {
                eprintln!(
                    "[backup-scheduler] scheduled backup failed to start for {server_id}: {error}"
                );
            }
        }
    }
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

    // ---- start_backup's real exclusivity, reached from the scheduler
    //      (P6.31) ----
    //
    // `fire()`'s own gate order no longer has an `admit_backup` pre-check
    // — real admission now lives inside
    // `crate::backup_operations::start_backup`, the same shared entry
    // point `LiveSchedulerBackend::run_scheduled_backup` and
    // `routes/backups.rs::now` both call. These two tests prove that
    // admission directly, against a real `LifecycleRoutesState`/
    // `LifecycleOperations` pair rather than a scripted fake. Every
    // Phase 6 mutation (activation, restore, conversion, replacement,
    // backup) admits through the identical per-target `OperationJournal`
    // call with no special case for operation type, so proving one
    // representative competitor (`world-activate`) and one same-type
    // competitor (a second backup) are both refused proves the general
    // rule, the same way `world_backup_routes_restore_guard_order_and_capability_unavailable`
    // already established for a manual route.

    fn scheduled_backup_exclusivity_test_server(tag: &str) -> ConfigServer {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backup-scheduler-exclusivity-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("world")).unwrap();
        std::fs::write(dir.join("world/level.dat"), b"fake").unwrap();
        ConfigServer::new(
            format!("sched-excl-{tag}"),
            "Scheduler Exclusivity Server",
            dir.to_string_lossy().to_string(),
            "",
            1.0,
            2.0,
        )
    }

    #[tokio::test]
    async fn scheduler_scheduled_backup_refused_while_another_operation_holds_the_server() {
        use crate::routes::lifecycle::LifecycleRoutesState;
        use crate::routes::operations::OperationsState;
        use crate::ws::console::ConsoleState;
        use msc_application::operations::LifecycleOperationError;

        let lifecycle = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server = scheduled_backup_exclusivity_test_server("activation");
        lifecycle
            .merge_config_servers(vec![server.clone()])
            .unwrap();
        lifecycle.select_active_server(server.id.clone()).unwrap();

        // Simulate an in-flight activation already holding this server's
        // exclusivity -- the same `begin_lifecycle` call
        // `routes/worlds.rs::activate` itself makes.
        lifecycle
            .operations()
            .begin_lifecycle("world-activate", Some(server.id.clone()), "Activating.")
            .unwrap();

        let result = crate::backup_operations::start_backup(&lifecycle, server, true, true, None);
        assert!(
            matches!(result, Err(LifecycleOperationError::Conflict(_))),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn scheduler_scheduled_backup_cannot_overlap_a_second_backup_on_the_same_server() {
        use crate::routes::lifecycle::LifecycleRoutesState;
        use crate::routes::operations::OperationsState;
        use crate::ws::console::ConsoleState;
        use msc_application::operations::LifecycleOperationError;

        let lifecycle = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server = scheduled_backup_exclusivity_test_server("second-backup");
        lifecycle
            .merge_config_servers(vec![server.clone()])
            .unwrap();
        lifecycle.select_active_server(server.id.clone()).unwrap();

        let first =
            crate::backup_operations::start_backup(&lifecycle, server.clone(), true, true, None);
        assert!(first.is_ok(), "{first:?}");

        let second = crate::backup_operations::start_backup(&lifecycle, server, true, true, None);
        assert!(
            matches!(second, Err(LifecycleOperationError::Conflict(_))),
            "{second:?}"
        );
    }

    #[tokio::test]
    async fn backup_scheduler_uses_configured_java_level_name() {
        use crate::routes::lifecycle::LifecycleRoutesState;
        use crate::routes::operations::OperationsState;
        use crate::ws::console::ConsoleState;

        let lifecycle = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server = scheduled_backup_exclusivity_test_server("custom-level-name");
        let server_dir = std::path::PathBuf::from(&server.server_dir);
        std::fs::rename(server_dir.join("world"), server_dir.join("family-realm")).unwrap();
        std::fs::create_dir_all(server_dir.join("family-realm_nether")).unwrap();
        std::fs::create_dir_all(server_dir.join("family-realm_the_end")).unwrap();
        std::fs::write(
            server_dir.join("server.properties"),
            "level-name=family-realm\n",
        )
        .unwrap();

        let operation_id =
            crate::backup_operations::start_backup(&lifecycle, server, false, true, Some(3))
                .unwrap();
        for _ in 0..200 {
            if lifecycle
                .operations()
                .snapshot(operation_id.as_str())
                .is_some_and(|record| {
                    matches!(
                        record.state,
                        msc_api::dto::OperationStateDto::Succeeded
                            | msc_api::dto::OperationStateDto::Failed
                            | msc_api::dto::OperationStateDto::Cancelled
                    )
                })
            {
                break;
            }
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
        let record = lifecycle
            .operations()
            .snapshot(operation_id.as_str())
            .expect("scheduled backup operation exists");
        assert_eq!(record.state, msc_api::dto::OperationStateDto::Succeeded);

        let zip_path = std::fs::read_dir(server_dir.join("backups"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|ext| ext == "zip"))
            .expect("scheduled backup zip exists");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect();
        for folder in [
            "family-realm",
            "family-realm_nether",
            "family-realm_the_end",
        ] {
            assert!(
                names.iter().any(|name| name.starts_with(folder)),
                "{names:?}"
            );
        }
    }
}
