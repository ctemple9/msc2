//! Port of `fixtures/backup-online-consistency/`'s 8 cases (P6.6),
//! exercising `msc_application::backups`'s flush-consistent save-pause
//! protocol (`pause_saves_for_backup`/`resume_saves_after_backup`/
//! `wait_for_bedrock_save_ready`/`wait_for_java_save_confirmation`)
//! against a [`FakeBackupConsole`] — the fakeable runtime port this
//! step's own scope note calls for — plus two full `create_backup`
//! integration cases that need a live console: the "zip failure still
//! resumes saves" case, and the sidecar-write-failure case that doesn't
//! strictly need one but shares this file's fixture domain tag.
//!
//! Test functions are prefixed `backup_online_consistency_` so the
//! plan's Verify command (`-E 'test(/backup_(creation|online_consistency)/)'`)
//! selects them.

#[cfg(unix)]
use msc_application::backups::{self, BackupError};
use msc_application::backups::{
    BackupConsole, pause_saves_for_backup, resume_saves_after_backup, wait_for_bedrock_save_ready,
    wait_for_java_save_confirmation,
};
#[cfg(unix)]
use msc_domain::identity::ServerType;
#[cfg(unix)]
use msc_domain::world;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
use std::sync::Mutex;
#[cfg(unix)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(unix)]
use msc_infrastructure::fs::StdFileSystem;

#[cfg(unix)]
struct TempDir(PathBuf);

#[cfg(unix)]
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backup-online-test-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(unix)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
fn make_live_folder(server_dir: &Path, name: &str, content: &[u8]) {
    let dir = server_dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("level.dat"), content).unwrap();
}

/// A scripted [`BackupConsole`] double: `send` always returns
/// `send_ok`, `wait_for_line` consumes one entry from a fixed queue
/// per call (`false` once exhausted), and `deadline_reached` fires once
/// `send` has been called `deadline_after` times — see
/// `msc_application::backups::BackupConsole`'s own doc for why a fake
/// this simple is enough: no production implementation exists in this
/// crate yet (P6.21's job).
struct FakeBackupConsole {
    send_ok: bool,
    sent: Mutex<Vec<String>>,
    line_results: Mutex<std::collections::VecDeque<bool>>,
    line_calls: Mutex<u32>,
    deadline_after: usize,
}

impl FakeBackupConsole {
    fn new(send_ok: bool) -> Self {
        Self {
            send_ok,
            sent: Mutex::new(Vec::new()),
            line_results: Mutex::new(std::collections::VecDeque::new()),
            line_calls: Mutex::new(0),
            deadline_after: usize::MAX,
        }
    }

    fn with_line_results(self, results: impl IntoIterator<Item = bool>) -> Self {
        *self.line_results.lock().unwrap() = results.into_iter().collect();
        self
    }

    fn with_deadline_after(mut self, sends: usize) -> Self {
        self.deadline_after = sends;
        self
    }

    fn sent_commands(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }

    fn line_call_count(&self) -> u32 {
        *self.line_calls.lock().unwrap()
    }
}

impl BackupConsole for FakeBackupConsole {
    fn send(&self, command: &str) -> bool {
        self.sent.lock().unwrap().push(command.to_string());
        self.send_ok
    }

    fn wait_for_line(&self, _matches: &dyn Fn(&str) -> bool) -> bool {
        *self.line_calls.lock().unwrap() += 1;
        self.line_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(false)
    }

    fn deadline_reached(&self) -> bool {
        self.sent.lock().unwrap().len() >= self.deadline_after
    }
}

/// `fixtures/backup-online-consistency/bedrock-save-hold-then-poll-save-query-until-ready-to-be-copied.json`.
#[test]
fn backup_online_consistency_bedrock_poll_until_ready() {
    let console = FakeBackupConsole::new(true).with_line_results([false, false, true]);
    let (ready, polls) = wait_for_bedrock_save_ready(&console);
    assert!(ready);
    assert_eq!(polls, 3);
    assert_eq!(
        console.sent_commands(),
        vec!["save query", "save query", "save query"]
    );
}

/// `fixtures/backup-online-consistency/bedrock-save-query-timeout-is-best-effort-proceeds-with-zip.json`.
#[test]
fn backup_online_consistency_bedrock_poll_timeout_is_best_effort() {
    let console = FakeBackupConsole::new(true)
        .with_line_results([false, false, false])
        .with_deadline_after(3);
    let (ready, polls) = wait_for_bedrock_save_ready(&console);
    assert!(!ready);
    assert_eq!(polls, 3);

    // `pauseSavesForBackup` still returns true — the pause was
    // negotiated (`save hold` was accepted) independent of whether
    // readiness was ever confirmed within the deadline.
    let console2 = FakeBackupConsole::new(true)
        .with_line_results([false, false, false])
        .with_deadline_after(3);
    assert!(pause_saves_for_backup(&console2, true));
}

/// `fixtures/backup-online-consistency/java-flush-then-save-off-sent-before-zip-starts.json`.
#[test]
fn backup_online_consistency_java_pause_sends_flush_then_off() {
    let console = FakeBackupConsole::new(true).with_line_results([true]);
    let paused = pause_saves_for_backup(&console, false);
    assert!(paused);
    assert_eq!(console.sent_commands(), vec!["save-all flush", "save-off"]);
}

/// `fixtures/backup-online-consistency/java-neither-flush-nor-off-accepted-skips-pause-zips-live-files.json`.
#[test]
fn backup_online_consistency_java_pause_skipped_when_neither_command_accepted() {
    let console = FakeBackupConsole::new(false);
    let paused = pause_saves_for_backup(&console, false);
    assert!(!paused);
    assert_eq!(console.sent_commands(), vec!["save-all flush", "save-off"]);
    // The confirmation wait is never reached — both sends failed.
    assert_eq!(console.line_call_count(), 0);
}

/// `fixtures/backup-online-consistency/java-save-confirmation-observed-within-timeout.json`.
#[test]
fn backup_online_consistency_java_confirmation_observed() {
    let console = FakeBackupConsole::new(true).with_line_results([true]);
    assert!(wait_for_java_save_confirmation(&console));
}

/// `fixtures/backup-online-consistency/java-save-confirmation-timeout-is-best-effort-proceeds-with-zip.json`.
#[test]
fn backup_online_consistency_java_confirmation_timeout_is_best_effort() {
    let console = FakeBackupConsole::new(true).with_line_results([false]);
    assert!(!wait_for_java_save_confirmation(&console));

    // Still returns true from the pause itself (off_sent), matching
    // source: only the confirmation *wait* timed out.
    let console2 = FakeBackupConsole::new(true).with_line_results([false]);
    assert!(pause_saves_for_backup(&console2, false));
}

/// `fixtures/backup-online-consistency/resume-always-resends-save-on-unconditionally.json`.
#[test]
fn backup_online_consistency_resume_always_resends_save_on() {
    let console = FakeBackupConsole::new(true);
    resume_saves_after_backup(&console, false, true);
    assert_eq!(console.sent_commands(), vec!["save-on"]);
}

/// `fixtures/backup-online-consistency/resume-skipped-when-server-stopped-before-resume-runs.json`.
#[test]
fn backup_online_consistency_resume_skipped_when_stopped() {
    let console = FakeBackupConsole::new(true);
    resume_saves_after_backup(&console, false, false);
    assert!(console.sent_commands().is_empty());
}

/// `fixtures/backup-online-consistency/zip-archive-nonzero-exit-status-fails-backup-saves-still-resumed.json`
/// — full `create_backup` integration: saves are resumed even though the
/// zip itself fails, via the same locked-subfolder failure injection
/// `world_slot_crud.rs`/`backup_creation.rs` already use.
#[cfg(unix)]
#[test]
fn backup_online_consistency_zip_failure_still_resumes_saves() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("zip-failure-resume");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let locked = server_dir.join("world").join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let console = FakeBackupConsole::new(true).with_line_results([true]);
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        Some(&console),
        || true,
        || false,
    );

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(result, Err(BackupError::Archive(_))));
    assert_eq!(
        console.sent_commands(),
        vec!["save-all flush", "save-off", "save-on"]
    );
}

/// `fixtures/backup-online-consistency/sidecar-meta-write-failure-logged-as-warning-backup-still-succeeds.json`.
#[cfg(unix)]
#[test]
fn backup_online_consistency_sidecar_write_failure_does_not_fail_backup() {
    let tmp = TempDir::new("sidecar-failure");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let backups_dir = server_dir.join("backups");
    fs::create_dir_all(&backups_dir).unwrap();

    // The sidecar path a "world" backup at this exact `now` will resolve
    // to — pre-create it as a directory so the sidecar write fails while
    // the (differently-named) zip write succeeds normally.
    let sidecar_path = backups_dir.join("world_manual_20260214-153045.meta.json");
    fs::create_dir_all(&sidecar_path).unwrap();

    let association = world::BackupAssociation::default();
    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    assert!(result.zip_path.is_file());
    assert!(!result.sidecar_written);
}

/// Cancellation during the bounded archive-copy loop removes the partial
/// ZIP and still resumes Minecraft saves before reporting cancellation.
#[cfg(unix)]
#[test]
fn backup_online_consistency_archive_cancellation_cleans_up_and_resumes_saves() {
    let tmp = TempDir::new("archive-cancel-resume");
    let server_dir = tmp.path();
    let world = server_dir.join("world");
    fs::create_dir_all(&world).unwrap();
    fs::write(world.join("region.mca"), vec![0x5a; 256 * 1024]).unwrap();

    let console = FakeBackupConsole::new(true).with_line_results([true]);
    let association = world::BackupAssociation::default();
    let polls = AtomicUsize::new(0);
    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        Some(&console),
        || true,
        || polls.fetch_add(1, Ordering::SeqCst) >= 5,
    );

    assert!(matches!(result, Err(BackupError::Cancelled)));
    assert_eq!(
        console.sent_commands(),
        vec!["save-all flush", "save-off", "save-on"]
    );
    let backups_dir = server_dir.join("backups");
    let leftovers: Vec<_> = fs::read_dir(backups_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(
        leftovers.is_empty(),
        "leftover backup artifacts: {leftovers:?}"
    );
}
