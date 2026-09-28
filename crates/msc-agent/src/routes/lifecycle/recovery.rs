//! Startup reconciliation and recovery for interrupted world transactions.

use super::*;

/// Per-server outcome of world reconciliation and interrupted-mutation
/// recovery. The map is live rather than a startup-only snapshot because
/// imports can add servers after the agent has started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationStatus {
    /// The server has been placed in the registry but its world data has
    /// not yet passed reconciliation. Selection and mutation stay closed.
    Reconciling,
    /// Startup reconciliation completed for this server; world/backup
    /// mutation routes are reachable.
    Ready,
    /// Startup reconciliation (or restart-transaction recovery) failed
    /// for this server. `reason` is the first failure encountered, for
    /// operator diagnosis. Every world/backup mutation route must refuse
    /// this server with one structured error instead of running.
    Degraded { reason: String },
}

/// The idempotent P6.1 world/`world_slots` handoff
/// (`msc_application::worlds::reconcile_imported_worlds`), followed by
/// P6.13/P6.18/P6.33's interrupted activation, restore, and active-world
/// replacement recovery —
/// run once per registered server, in that order, before this registry
/// (and therefore any world/backup mutation route built over it) becomes
/// reachable. **Corrected post-gate-review:** a failure here used to be
/// logged and then silently ignored, leaving every mutation route for
/// that server reachable against unreconciled — possibly unsafe — disk
/// state. Now the first failure for a server (from either stage) is
/// recorded as [`ReconciliationStatus::Degraded`] and returned to the
/// caller, who threads it into every world/backup mutation route's guard
/// (`routes/worlds.rs`'s `active_server_or_response`, `routes/
/// backups.rs`'s `active_server_or_response`). The agent itself still
/// comes up — a damaged server does not block startup, and other,
/// healthy servers are entirely unaffected — matching the "keep the
/// agent available for diagnosis" requirement.
pub(super) fn reconcile_server(server: &ConfigServer) -> ReconciliationStatus {
    let mut first_failure = None;
    let now = iso8601_now();
    let server_dir = Path::new(&server.server_dir);
    let pending_swaps = [".activation", ".restore", ".replace"]
        .iter()
        .filter(|name| server_dir.join("world_slots").join(name).exists())
        .count();
    if pending_swaps > 1 {
        return ReconciliationStatus::Degraded {
            reason: "multiple world replacement transactions need manual repair".to_string(),
        };
    }
    if let Err(err) =
        msc_application::worlds::reconcile_interrupted_activation(&StdFileSystem, server_dir, &now)
    {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted activation for {}: {err}",
            server.server_dir
        );
        first_failure
            .get_or_insert_with(|| format!("interrupted activation recovery failed: {err}"));
    }
    if first_failure.is_none()
        && let Err(err) =
            msc_application::backups::reconcile_interrupted_restore(&StdFileSystem, server_dir)
    {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted restore for {}: {err}",
            server.server_dir
        );
        first_failure.get_or_insert_with(|| format!("interrupted restore recovery failed: {err}"));
    }
    if first_failure.is_none()
        && let Err(err) = msc_application::worlds::reconcile_interrupted_world_replace(
            &StdFileSystem,
            server_dir,
            server.server_type,
        )
    {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted active-world replacement for {}: {err}",
            server.server_dir
        );
        first_failure
            .get_or_insert_with(|| format!("interrupted world replacement recovery failed: {err}"));
    }

    // Import reconciliation inspects the live folders. It must see a
    // completed swap, and it must not run against a transaction requiring
    // manual repair.
    if first_failure.is_none()
        && let Err(err) = msc_application::worlds::reconcile_imported_worlds(
            &StdFileSystem,
            server_dir,
            server.server_type,
            None,
            &now,
        )
    {
        eprintln!(
            "[worlds] Warning: could not reconcile imported world data for {}: {err}",
            server.server_dir
        );
        first_failure = Some(format!("world reconciliation failed: {err}"));
    }

    first_failure.map_or(ReconciliationStatus::Ready, |reason| {
        ReconciliationStatus::Degraded { reason }
    })
}

pub(super) fn reconcile_servers_at_startup(
    servers: &[ConfigServer],
) -> BTreeMap<String, ReconciliationStatus> {
    servers
        .iter()
        .map(|server| (server.id.clone(), reconcile_server(server)))
        .collect()
}

/// `MSC2_AUDIT_LOG_DIR`-overridable, mirroring
/// `OperationsState::default_journaled`'s `MSC2_OPERATION_JOURNAL_DIR`
/// pattern — the Phase 6 world/backup mutation audit trail
/// (`routes/worlds.rs`/`routes/backups.rs`) lives alongside the
/// operation journal by default, not inside a server directory.
pub(super) fn audit_log_dir() -> PathBuf {
    std::env::var_os("MSC2_AUDIT_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("msc2-audit-log"))
}
