//! P7.20: the three remaining fleet mutations — delete, rename, and
//! EULA acceptance — against MSC 1's actual semantics.
//!
//! Ports `deleteServerProvider`/`deleteServerFromDisk`/`deleteServer
//! (withId:)` (`AppViewModel+APIWiringServerMgmt.swift:43-67`,
//! `AppViewModel+ConfigHelpers.swift:66-106`), `renameServerProvider`
//! (`AppViewModel+APIWiringServerMgmt.swift:19-41`), and `EULAManager`
//! (`EULAManager.swift`, full file) plus `acceptEULAProvider`.
//!
//! **Unlike `provisioning.rs`/`server_versions.rs`, this module mutates
//! `&mut AppConfig` directly rather than returning data for a caller to
//! apply.** Those two modules build a server that doesn't exist in the
//! fleet yet (or resolve version data for one field); delete and rename
//! *are* fleet-registry mutations by their very nature — there's no
//! "resolved data, apply later" split that makes sense for "remove this
//! entry and reselect the active one." Persisting the mutated `AppConfig`
//! to disk (`configManager.save()`'s real equivalent) stays the route
//! layer's job (P7.23), same as every other cross-cutting I/O concern
//! this phase's modules already push there.
//!
//! **The running-server check is caller-supplied**, matching
//! `server_versions::change_version`'s `is_running`/`is_downloading`
//! shape: source's own check (`self.isServerRunning, cfg.activeServerId
//! == trimmedId`) reads a single global "is *the* active server running"
//! flag `AppViewModel` owns — this crate's own
//! `LifecycleService.active_server()`/`.state()` (`lifecycle.rs`) already
//! models exactly that, so the caller (P7.23) computes the one bool this
//! module needs rather than `fleet.rs` depending on `lifecycle.rs`
//! directly for a single comparison.
//!
//! **A real, dead branch in the frozen contract, ported faithfully:**
//! `openapi.json`'s rename route documents a 409 `server_running`
//! response (the shared `serverMutationStatus` switch,
//! `RemoteAPIServer+ComponentRoutes.swift:16-24`), but the real
//! `renameServerProvider` never returns that message — reading it end to
//! end (source line 19-41) shows no running check anywhere. [`rename_server`]
//! has no running-server parameter at all; the route layer (P7.23) simply
//! never emits that documented-but-unreachable variant, the same
//! "kept for contract completeness though unreachable" precedent P7.8
//! already used for two dead `StartupProblemKind`s.

use msc_domain::app_config_schema::AppConfig;
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::FileSystem;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum DeleteServerError {
    /// `"missing_server_id"` (source line 48-49) — an empty/whitespace-
    /// only id, checked before any lookup.
    EmptyServerId,
    /// `"server_not_found"` (source line 54).
    ServerNotFound,
    /// `"server_running"` (source line 57) — the route-level check, which
    /// fires before `deleteServerFromDisk` is ever called, so its own
    /// (redundant in source) running check never has a chance to throw.
    ServerRunning,
    /// `"delete_failed"` (source line 62-63) — any `removeItem` error
    /// besides "already missing," which source tolerates (line 102-104).
    DeleteFailed(std::io::Error),
}

impl fmt::Display for DeleteServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeleteServerError::EmptyServerId => write!(f, "server id is empty"),
            DeleteServerError::ServerNotFound => write!(f, "server not found"),
            DeleteServerError::ServerRunning => write!(f, "server is running"),
            DeleteServerError::DeleteFailed(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DeleteServerError {}

/// `deleteServer(withId:)`'s own after-the-fact bookkeeping (source line
/// 66-80), returned so a caller can report what actually changed rather
/// than re-deriving it from a mutated `AppConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedServer {
    pub removed_display_name: String,
    /// `appConfig.servers.first?.id` (array-order, post-removal — not
    /// most-recently-used, not by name) when the deleted server was
    /// active; unchanged otherwise. `None` if the fleet is now empty.
    pub new_active_server_id: Option<String>,
}

/// `deleteServerProvider` + `deleteServerFromDisk` + `deleteServer
/// (withId:)` composed into one call (source line 43-67, 82-106):
/// look up by id, refuse while running, remove the on-disk directory
/// (required, not best-effort — a missing folder is tolerated, any other
/// removal error propagates), then remove the fleet entry and reselect
/// the active server if it was the one just deleted.
pub fn delete_server(
    fs: &dyn FileSystem,
    config: &mut AppConfig,
    server_id: &str,
    is_active_and_running: bool,
) -> Result<DeletedServer, DeleteServerError> {
    let trimmed = server_id.trim();
    if trimmed.is_empty() {
        return Err(DeleteServerError::EmptyServerId);
    }
    let idx = config
        .servers
        .iter()
        .position(|s| s.id == trimmed)
        .ok_or(DeleteServerError::ServerNotFound)?;
    if is_active_and_running {
        return Err(DeleteServerError::ServerRunning);
    }

    let server_dir = Path::new(&config.servers[idx].server_dir).to_path_buf();
    if fs.stat(&server_dir).is_ok() {
        fs.remove(&server_dir)
            .map_err(DeleteServerError::DeleteFailed)?;
    }
    // A missing folder is tolerated (source line 102-104): fall through
    // to the registry removal exactly as if the disk step had succeeded.

    let removed = config.servers.remove(idx);
    if config.active_server_id.as_deref() == Some(trimmed) {
        config.active_server_id = config.servers.first().map(|s| s.id.clone());
    }
    Ok(DeletedServer {
        removed_display_name: removed.display_name,
        new_active_server_id: config.active_server_id.clone(),
    })
}

#[derive(Debug)]
pub enum RenameServerError {
    /// `"missing_server_id"` (source line 25-26).
    EmptyServerId,
    /// `"name_required"` (source line 28-29).
    EmptyName,
    /// `"server_not_found"` (source line 33).
    ServerNotFound,
}

impl fmt::Display for RenameServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenameServerError::EmptyServerId => write!(f, "server id is empty"),
            RenameServerError::EmptyName => write!(f, "name is empty"),
            RenameServerError::ServerNotFound => write!(f, "server not found"),
        }
    }
}

impl std::error::Error for RenameServerError {}

/// `renameServerProvider` (source line 19-41). Confirmed by reading the
/// whole function: **only** `ConfigServer.displayName` is written — no
/// directory rename, no `server.properties` `motd` touch, no collision
/// check against other servers' display names (duplicates are allowed,
/// matching source exactly).
pub fn rename_server(
    config: &mut AppConfig,
    server_id: &str,
    new_name: &str,
) -> Result<(), RenameServerError> {
    let trimmed_id = server_id.trim();
    if trimmed_id.is_empty() {
        return Err(RenameServerError::EmptyServerId);
    }
    let trimmed_name = new_name.trim();
    if trimmed_name.is_empty() {
        return Err(RenameServerError::EmptyName);
    }
    let server = config
        .servers
        .iter_mut()
        .find(|s| s.id == trimmed_id)
        .ok_or(RenameServerError::ServerNotFound)?;
    server.display_name = trimmed_name.to_string();
    Ok(())
}

/// `EULAManager.readEULA(in:)`'s three-way result (`EULAManager.swift:
/// 14-31`): `Accepted`/`ExplicitlyFalse` both require a recognized
/// `eula=` line; anything else — no file, unreadable, no `eula=` line at
/// all, **or** an `eula=` line whose value is neither `true` nor `false`
/// (e.g. `eula=maybe`, `eula=`) — reads as [`EulaState::Missing`], the
/// same "absent" bucket source's own `nil` return covers for all of
/// those cases alike (`raw.lowercased().contains("true")` is the only
/// test; anything not containing "true" is `false`, never a third state
/// at the boolean level — the "neither true nor false" case this step's
/// own plan text names is only ever reachable through the *no matching
/// line at all* path, since a malformed value still parses as `false`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EulaState {
    Accepted,
    ExplicitlyFalse,
    Missing,
}

pub fn read_eula(fs: &dyn FileSystem, server_dir: &Path) -> EulaState {
    let Ok(bytes) = fs.read(&server_dir.join("eula.txt")) else {
        return EulaState::Missing;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return EulaState::Missing;
    };
    for line in text.lines() {
        let raw = line.trim().to_lowercase();
        if raw.starts_with("eula=") {
            return if raw.contains("true") {
                EulaState::Accepted
            } else {
                EulaState::ExplicitlyFalse
            };
        }
    }
    EulaState::Missing
}

#[derive(Debug)]
pub enum AcceptEulaError {
    /// `"missing_server_id"`/`"server_not_found"` collapsed: this
    /// module takes an already-resolved server, so an empty id is the
    /// caller's own validation concern (matching `provisioning.rs`'s
    /// "already-resolved values" precedent) — only "no such server" is
    /// this function's own error.
    ServerNotFound,
    /// `"unsupported_server_type"` (`acceptEULAProvider`, `server.
    /// serverType != .java`) — Bedrock has no `eula.txt` concept.
    UnsupportedServerType,
    /// `"eula_write_failed"`.
    WriteFailed(std::io::Error),
}

impl fmt::Display for AcceptEulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceptEulaError::ServerNotFound => write!(f, "server not found"),
            AcceptEulaError::UnsupportedServerType => write!(f, "server is not a Java server"),
            AcceptEulaError::WriteFailed(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AcceptEulaError {}

/// `EULAManager.writeAcceptedEULA(in:)` (source line 33-41): the literal
/// three-line, comment-headed format — genuinely different from
/// `provisioning.rs`'s own bare `eula=false\n` write at creation time;
/// this port doesn't normalize the two to match.
const ACCEPTED_EULA_TEXT: &str = "# EULA accepted via MinecraftServerController\neula=true\n\n";

/// `acceptEULAProvider` (`AppViewModel+APIWiringServerMgmt.swift:207-
/// 234`) composed with `EULAManager.writeAcceptedEULA`. No running-server
/// gate anywhere in source.
pub fn accept_eula(
    fs: &dyn FileSystem,
    config: &AppConfig,
    server_id: &str,
) -> Result<(), AcceptEulaError> {
    let server = config
        .servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or(AcceptEulaError::ServerNotFound)?;
    if server.server_type != ServerType::Java {
        return Err(AcceptEulaError::UnsupportedServerType);
    }
    fs.write(
        &Path::new(&server.server_dir).join("eula.txt"),
        ACCEPTED_EULA_TEXT.as_bytes(),
    )
    .map_err(AcceptEulaError::WriteFailed)
}
