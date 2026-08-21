//! P7.22: turn a failed/soft-failed start into structured, actionable
//! problems, and the four Phase 7-owned health cards (directory, Java
//! runtime, RAM allocation, last startup).
//!
//! Ports `writeLastStartupResult`/`checkLastStartup`/`checkDirectory`/
//! `checkJavaRuntime`/`checkRAMAllocation` (`AppViewModel+HealthCards
//! .swift`), `diagnoseUnexpectedStop`/`scanPaperSoftFailures`
//! (`AppViewModel+OutputHandling.swift:184-274`), and `mapProblem`'s
//! `availableActions` plus the disable/delete repair dispatch
//! (`AppViewModel+APIWiringBackupsHealth.swift:120-137, 224-237`,
//! `AppViewModel+ModManagement.swift:192-233`,
//! `AppViewModel+PluginManagement.swift:112-166`).
//!
//! **Health card dates are not formatted here.** `checkLastStartup`'s
//! `formatDate` is `DateFormatter`'s locale-dependent medium/short
//! style — this crate has no locale infrastructure (the same "no locale
//! infra" boundary `templates.rs`'s `jar_summary` already draws). Every
//! card here takes an already-formatted date string from the caller
//! rather than reformatting `LastStartupResult.started_at` itself.
//!
//! **`checkJavaRuntime` is a real, separate implementation from P7.7/
//! P7.12's create/launch-time runtime selection**, confirmed by reading
//! both: this card hardcodes `major >= 21` with no awareness of the
//! server's actual required major, and returns on the first candidate
//! that produces *any* non-empty output at exit 0 — even output that
//! fails to parse a version at all. `check_java_runtime` here is a
//! faithful, deliberately un-unified port of that separate algorithm,
//! not a call into `msc_domain::java_runtime::parse_major` (P7.12's own
//! banner parser) — `fixtures/startup-problems/check-java-runtime-found-
//! major-below-21-yellow.json`'s own note flags this exact divergence
//! and leaves the choice to whichever step builds the card; this port
//! preserves the oracle's bug rather than silently fixing it.
//!
//! **`disable`/`delete` are a deliberate strengthening over the
//! oracle.** Source's `toggleMod`/`removeMod`/`togglePlugin`/
//! `removePlugin` never verify the rename/delete actually landed before
//! reporting success (a bare `do { try ... } catch { log }`, confirmed
//! by reading all four). This port re-stats the filesystem after the
//! operation and returns [`RepairError::VerificationFailed`] if the
//! expected end state doesn't hold — the "re-checked after the fact"
//! language this phase's own plan text uses for exactly this reason:
//! `msc2-product.md`'s promise that MSC never reports success it hasn't
//! confirmed.
//!
//! **What stays out of this module, and why:**
//! - `StartupCrashAnalyzer.analyzePaperPlugins` (Paper/Spigot plugin-log
//!   parser) was deliberately never ported (`crash_analysis.rs`'s own
//!   doc). [`scan_paper_soft_failures`] takes the already-analyzed
//!   `Vec<StartupProblem>` as a parameter, matching every
//!   `scan-paper-soft-failures-*` fixture's own `analyzerReturns` shape.
//! - The generic "Server Stopped Unexpectedly" alert `diagnoseUnexpectedStop`
//!   shows when analysis finds nothing is UI presentation with no agent
//!   equivalent; not built here.
//! - `update`/`install` repairs are real as of P8.23, but not built in
//!   *this* module: both need real Modrinth network access
//!   (`crate::addon_updates::repair_update`/
//!   `repair_install_missing_dependency`), unlike [`repair_problem`]'s own
//!   pure-filesystem `Disable`/`Delete`, which stays exactly as it was.
//!   [`available_actions`] still lists all four uniformly (schema
//!   completeness); the route layer dispatches `update`/`install` to
//!   `addon_updates.rs` instead of here.
//! - The stateful "in-memory problems vs. disk-reconstructed problems,
//!   keyed by the selected server" reconciliation
//!   `healthProblemsProvider`/`repairHealthProblemProvider` do, and the
//!   `invalid_action`/`no_active_server`/`problem_not_found` guards that
//!   depend on it, are the route layer's job (P7.23) — this module
//!   exposes [`read_last_startup_result`] for the disk-fallback half and
//!   pure [`available_actions`]/[`repair_problem`] for a caller that has
//!   already resolved which `StartupProblem` is being acted on.
//! - The port-reachability card stays Phase 9 per this phase's own "Not
//!   in this phase" list.
//!
//! **P8.23: the "Add-on Jars" (`componentJars`) card is real add-on data,
//! not a network probe.** [`check_component_jars`] reports the same two
//! signals this route already has on hand for free — a real disk count of
//! installed mods/plugins ("folder" findings) and the already-persisted
//! `last_startup_result.json` problem list's own `IncompatibleVersion`/
//! `MissingDependency` counts ("version"/"dependency" findings), which
//! `crash_analysis::analyze` already computes on every real server exit
//! (P7.32/P7.36). Deliberately NOT a live Modrinth resolve: `GET /v1/health`
//! is this agent's one route that runs outside the bearer-auth gate and is
//! expected to answer fast and offline — giving it an outbound network
//! dependency (`addon_updates::resolve_addon_updates`) would be a real,
//! undiscussed behavior change to an unauthenticated endpoint, not a
//! faithful reading of "add-on folder/version/dependency findings." The
//! *mutating* half of this step — `update`/`install` health repairs — DOES
//! call Modrinth for real, but only from the already-authenticated
//! (`Settings` permission) `POST /v1/health/repair` (`addon_updates.rs`'s
//! own [`crate::addon_updates::repair_update`]/
//! [`crate::addon_updates::repair_install_missing_dependency`]).

use msc_domain::crash_analysis::{self, ModEntry, StartupProblem, StartupProblemKind};
use msc_domain::identity::AddOnKind;
use msc_infrastructure::fs::FileSystem;
use std::fmt;
use std::path::Path;

// ---------------------------------------------------------------------
// LastStartupResult persistence
// ---------------------------------------------------------------------

/// `LastStartupResult` (`HealthCardModels.swift`), persisted to
/// `{serverDir}/last_startup_result.json`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastStartupResult {
    pub started_at: String,
    pub was_clean: bool,
    pub fatal_errors: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problems: Option<Vec<StartupProblem>>,
}

const LAST_STARTUP_RESULT_FILENAME: &str = "last_startup_result.json";

/// `writeLastStartupResult` (`AppViewModel+HealthCards.swift:917-929`):
/// intentionally best-effort, matching source's own `try? data.write`
/// (silent on failure). `problems.isEmpty` is written as JSON `null`,
/// not `[]` — `fixtures/startup-problems/write-last-startup-result-
/// record-shape-and-path.json`'s own point, preserved rather than
/// "improved" into a distinguishable empty array.
pub fn write_last_startup_result(
    fs: &dyn FileSystem,
    server_dir: &Path,
    started_at: &str,
    was_clean: bool,
    fatal_errors: Vec<String>,
    warnings: Vec<String>,
    problems: Vec<StartupProblem>,
) {
    let result = LastStartupResult {
        started_at: started_at.to_string(),
        was_clean,
        fatal_errors,
        warnings,
        problems: if problems.is_empty() {
            None
        } else {
            Some(problems)
        },
    };
    if let Ok(bytes) = serde_json::to_vec(&result) {
        let _ = fs.write(&server_dir.join(LAST_STARTUP_RESULT_FILENAME), &bytes);
    }
}

/// `checkLastStartup`'s file read (`AppViewModel+HealthCards.swift:856-
/// 858`) and `loadPersistedProblems`'s identical read
/// (`AppViewModel+APIWiringBackupsHealth.swift:112-118`) share this same
/// primitive. `None` for a missing file, unreadable bytes, or malformed
/// JSON alike — a corrupted result file reads exactly like "never
/// started," matching `fixtures/startup-problems/last-startup-card-
/// never-started-gray.json`'s own note.
pub fn read_last_startup_result(
    fs: &dyn FileSystem,
    server_dir: &Path,
) -> Option<LastStartupResult> {
    let bytes = fs
        .read(&server_dir.join(LAST_STARTUP_RESULT_FILENAME))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// **P7.36.** MSC 1 has no equivalent — it keeps a separate in-memory
/// `startupProblems` array for the running session and only unconditionally
/// does `startupProblems.removeAll { $0.id == problem.id }` there
/// (`AppViewModel+APIWiringBackupsHealth.swift`'s repair dispatcher),
/// never touching the persisted JSON at all; the disk file is only ever a
/// fallback for a fresh launch. MSC 2's agent is headless and keeps no
/// such session cache — `health_repair` (`routes/health.rs`) reads
/// `last_startup_result.json` fresh on every call, so *this* is the only
/// place "remove only the repaired problem after verification; preserve
/// it on failure" (this phase's own working exit criterion) can become
/// real. Called only after [`repair_problem`] has already returned `Ok`
/// (a verified rename/delete), never speculatively — a failed repair
/// leaves the persisted record completely untouched, problem included.
/// Every other field (`started_at`/`was_clean`/`fatal_errors`/`warnings`)
/// is preserved byte-for-byte; only the matching problem is dropped from
/// `problems`, `None` again if that empties the list, matching
/// [`write_last_startup_result`]'s own `problems.isEmpty() -> null` rule.
/// Returns `Ok(false)` when there's no persisted record or `problem_id`
/// isn't present in it. A failed persistence write is returned to the
/// caller so the API cannot claim the complete repair state was saved.
pub fn remove_repaired_problem(
    fs: &dyn FileSystem,
    server_dir: &Path,
    problem_id: &str,
) -> Result<bool, std::io::Error> {
    let Some(mut record) = read_last_startup_result(fs, server_dir) else {
        return Ok(false);
    };
    let Some(problems) = record.problems.as_mut() else {
        return Ok(false);
    };
    let before = problems.len();
    problems.retain(|p| p.id() != problem_id);
    if problems.len() == before {
        return Ok(false);
    }
    if problems.is_empty() {
        record.problems = None;
    }
    let bytes = serde_json::to_vec(&record).map_err(std::io::Error::other)?;
    fs.write(&server_dir.join(LAST_STARTUP_RESULT_FILENAME), &bytes)?;
    Ok(true)
}

// ---------------------------------------------------------------------
// diagnoseUnexpectedStop / scanPaperSoftFailures
// ---------------------------------------------------------------------

/// `diagnoseUnexpectedStop(reachedReadyState:)` (`AppViewModel+
/// OutputHandling.swift:184-232`), minus the generic-alert UI
/// presentation (this module's own doc). `is_modded`/`flavor` mirror
/// `cfg.isModded`/`cfg.javaFlavor` — `flavor` only needs to be a loader
/// family string (`crash_analysis::analyze`'s own parameter shape, P1.7),
/// not the full `JavaServerFlavor` enum.
///
/// Persistence exactly matches source's three-way split: problems found
/// -> `wasClean:false` with per-problem summaries as `fatalErrors`;
/// `isHardFail` but no problems found -> `wasClean:false` with the
/// generic "stopped before reaching ready state" fatal error; reached
/// ready state (not `isHardFail`) -> **nothing written at all**, the
/// real gap `fixtures/startup-problems/diagnose-unexpected-stop-reached-
/// ready-state-no-persistence-but-alert-shown.json` and P7.8's own
/// finding #5 both flag (a mid-session crash after a clean boot leaves
/// the Last Startup card showing the prior clean result).
#[allow(clippy::too_many_arguments)]
pub fn diagnose_unexpected_stop(
    fs: &dyn FileSystem,
    server_dir: &Path,
    now: &str,
    reached_ready_state: bool,
    is_modded: bool,
    flavor: &str,
    console_excerpt: &[String],
    installed_mods: &[ModEntry],
) -> Vec<StartupProblem> {
    let is_hard_fail = !reached_ready_state;
    let should_analyze = is_hard_fail && is_modded;
    let problems = if should_analyze {
        crash_analysis::analyze(flavor, console_excerpt, installed_mods)
    } else {
        Vec::new()
    };

    if !problems.is_empty() {
        let summaries: Vec<String> = problems.iter().map(problem_summary_line).collect();
        write_last_startup_result(
            fs,
            server_dir,
            now,
            false,
            summaries,
            Vec::new(),
            problems.clone(),
        );
    } else if is_hard_fail {
        write_last_startup_result(
            fs,
            server_dir,
            now,
            false,
            vec!["Server stopped before reaching ready state.".to_string()],
            Vec::new(),
            Vec::new(),
        );
    }
    problems
}

/// `"\(offenderName): \(requirement ?? kind.title)"` (source line 213,
/// 267) — shared by both `diagnoseUnexpectedStop` and
/// `scanPaperSoftFailures`'s own per-problem summary line.
fn problem_summary_line(p: &StartupProblem) -> String {
    format!(
        "{}: {}",
        p.offender_name,
        p.requirement.as_deref().unwrap_or(p.kind.title())
    )
}

/// `scanPaperSoftFailures(for:)` (`AppViewModel+OutputHandling.swift:
/// 256-274`), minus `StartupCrashAnalyzer.analyzePaperPlugins` itself
/// (this module's own doc) — `problems` is that analyzer's already-
/// computed output. Returns `false` (no-op, nothing written) when the
/// guard refuses (source line 257) or the analysis found nothing (line
/// 265's `guard !problems.isEmpty else { return }`); `true` when it
/// wrote a record. `wasClean` stays `true` throughout — the server did
/// start; these are non-fatal.
pub fn scan_paper_soft_failures(
    fs: &dyn FileSystem,
    server_dir: &Path,
    now: &str,
    is_java: bool,
    add_on_kind_is_plugin: bool,
    problems: Vec<StartupProblem>,
) -> bool {
    if !is_java || !add_on_kind_is_plugin || problems.is_empty() {
        return false;
    }
    let warnings: Vec<String> = problems.iter().map(problem_summary_line).collect();
    write_last_startup_result(fs, server_dir, now, true, Vec::new(), warnings, problems);
    true
}

// ---------------------------------------------------------------------
// Health cards
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Green,
    Yellow,
    Red,
    Gray,
}

/// `HealthCardResult`, the four fields `HealthCardDTO` (P7.23) needs
/// beyond its own presentation strings (`title`/`shortLabel`/
/// `iconSystemName`, which `healthCardTitle`/`healthCardShort`/
/// `healthCardIcon` derive from `id`+`status` alone — a route-layer
/// lookup table, not per-card logic this module owns), plus `help_id`
/// per `helpid-contract.md` §4 (assigned here since it's a static,
/// kind-driven constant, the same choice P7.8's own fixtures already
/// made by embedding it in `expected`).
#[derive(Debug, Clone, PartialEq)]
pub struct HealthCardResult {
    pub id: &'static str,
    pub status: HealthStatus,
    pub detected_value: String,
    pub action_label: Option<&'static str>,
    /// `"locateFolder"` / `"openURL:<url>"` / `"diagnoseStartup"` /
    /// `"openConsoleLog"` — `healthActionCode`'s own wire vocabulary
    /// (`AppViewModel+APIWiringBackupsHealth.swift`), reused verbatim
    /// rather than re-encoded as a Rust enum P7.23 would just have to
    /// flatten back to these same strings.
    pub action_type: Option<&'static str>,
    pub help_id: &'static str,
}

/// `checkDirectory(for:)`'s already-resolved inputs
/// (`AppViewModel+HealthCards.swift:156-186`) — `writable`/`readable`
/// come from `FileManager.isWritableFile`/`isReadableFile`, a real
/// `access()`-backed permission probe with no portable equivalent in
/// this crate's `FileSystem` trait (which tracks only an `executable`
/// bit); the caller (P7.23, using real `std::fs`/platform calls) probes
/// permissions and passes the result in, the same "pure branching logic,
/// I/O supplied by caller" split `fixtures/startup-problems/check-
/// directory-*` fixtures already model at the input level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryProbe {
    pub exists: bool,
    pub is_dir: bool,
    pub writable: bool,
    pub readable: bool,
}

pub fn check_directory(server_dir: &str, probe: DirectoryProbe) -> HealthCardResult {
    if !probe.exists || !probe.is_dir {
        return HealthCardResult {
            id: "directory",
            status: HealthStatus::Red,
            detected_value: format!("Path not found:\n{server_dir}"),
            action_label: Some("Locate Folder"),
            action_type: Some("locateFolder"),
            help_id: "health.directory",
        };
    }
    if probe.writable && probe.readable {
        HealthCardResult {
            id: "directory",
            status: HealthStatus::Green,
            detected_value: server_dir.to_string(),
            action_label: None,
            action_type: None,
            help_id: "health.directory",
        }
    } else {
        HealthCardResult {
            id: "directory",
            status: HealthStatus::Yellow,
            detected_value: format!("Exists but may have permission issues:\n{server_dir}"),
            action_label: Some("Locate Folder"),
            action_type: Some("locateFolder"),
            help_id: "health.directory",
        }
    }
}

/// One candidate `checkJavaRuntime` tried (`AppViewModel+HealthCards
/// .swift:210-214`): whether the path existed on disk, and — only if it
/// did — the already-run `-version` invocation's exit code and combined
/// stdout+stderr. The caller builds the full candidate list (configured
/// path, `JAVA_HOME`, `java_home` output, the hardcoded macOS list) and
/// runs each `-version` probe; this function owns only the selection
/// logic over already-collected results.
#[derive(Debug, Clone, Copy)]
pub struct JavaCandidateProbe<'a> {
    pub path: &'a str,
    pub exists: bool,
    pub version_check: Option<(i32, &'a str)>,
}

pub fn check_java_runtime(candidates: &[JavaCandidateProbe]) -> HealthCardResult {
    for candidate in candidates {
        if !candidate.exists {
            continue;
        }
        let Some((exit_code, output)) = candidate.version_check else {
            continue;
        };
        if exit_code != 0 || output.is_empty() {
            continue;
        }

        let parsed = extract_java_version_and_major(output);
        if let Some((version_string, major)) = parsed {
            return if major >= 21 {
                HealthCardResult {
                    id: "java",
                    status: HealthStatus::Green,
                    detected_value: format!(
                        "{version_string} \u{2014} minimum is Java 21 \u{2713}\nPath: {}",
                        candidate.path
                    ),
                    action_label: None,
                    action_type: None,
                    help_id: "health.java",
                }
            } else {
                HealthCardResult {
                    id: "java",
                    status: HealthStatus::Yellow,
                    detected_value: format!(
                        "{version_string} detected \u{2014} minimum is Java 21\nPath: {}",
                        candidate.path
                    ),
                    action_label: Some("Download Java"),
                    action_type: Some("openURL:https://adoptium.net"),
                    help_id: "health.java",
                }
            };
        }

        return HealthCardResult {
            id: "java",
            status: HealthStatus::Yellow,
            detected_value: format!(
                "Java found but version unreadable.\nOutput: {}\nPath: {}",
                truncate_chars(output, 120),
                candidate.path
            ),
            action_label: Some("Download Java"),
            action_type: Some("openURL:https://adoptium.net"),
            help_id: "health.java",
        };
    }

    HealthCardResult {
        id: "java",
        status: HealthStatus::Red,
        detected_value: format!(
            "Java not found. Checked {} locations.\nInstall Adoptium Temurin 21 or set your Java path in Preferences.",
            candidates.len()
        ),
        action_label: Some("Download Java"),
        action_type: Some("openURL:https://adoptium.net"),
        help_id: "health.java",
    }
}

/// `extractJavaVersion`/`parseJavaMajorVersion`
/// (`AppViewModel+HealthCards.swift:953-967`) fused into one pass: the
/// first `"..."` quoted token, then a legacy `1.x` major derived from
/// its second dotted component. Deliberately **not** shared with
/// `msc_domain::java_runtime::parse_major` (P7.12's own banner parser)
/// even though the algorithms are similar — this module's own doc
/// explains why source itself never unifies the two.
fn extract_java_version_and_major(output: &str) -> Option<(String, i64)> {
    let start = output.find('"')? + 1;
    let rest = &output[start..];
    let end = rest.find('"')?;
    let quoted = rest[..end].to_string();
    let mut parts = quoted.split('.');
    let first: i64 = parts.next()?.parse().ok()?;
    let major = if first == 1 {
        parts.next()?.parse().ok()?
    } else {
        first
    };
    Some((quoted, major))
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// `checkRAMAllocation(for:)` (`AppViewModel+HealthCards.swift:433-
/// 467`). `ram_gb_label`'s `"%g"` formatting (drops a trailing `.0` for
/// whole-number GB, `AppConfig.swift:78`) is reproduced exactly —
/// `fixtures/startup-problems/check-ram-allocation-high-fraction-
/// yellow.json`'s own note names this.
pub fn check_ram_allocation(allocated_gb: f64, physical_gb: i64) -> HealthCardResult {
    let allocated_label = ram_gb_label(allocated_gb);
    if allocated_gb <= 0.0 || (physical_gb > 0 && allocated_gb > physical_gb as f64) {
        return HealthCardResult {
            id: "ram",
            status: HealthStatus::Red,
            detected_value: format!(
                "Configured: {allocated_label} GB \u{2014} Physical RAM: {physical_gb} GB\nAllocation exceeds physical memory."
            ),
            action_label: None,
            action_type: None,
            help_id: "health.ram",
        };
    }

    let fraction = if physical_gb > 0 {
        allocated_gb / physical_gb as f64
    } else {
        0.0
    };

    if fraction > 0.8 {
        HealthCardResult {
            id: "ram",
            status: HealthStatus::Yellow,
            detected_value: format!(
                "Configured: {allocated_label} GB \u{2014} Physical RAM: {physical_gb} GB\n{}% of system RAM \u{2014} may cause instability.",
                (fraction * 100.0) as i64
            ),
            action_label: None,
            action_type: None,
            help_id: "health.ram",
        }
    } else {
        HealthCardResult {
            id: "ram",
            status: HealthStatus::Green,
            detected_value: format!(
                "Configured: {allocated_label} GB \u{2014} Physical RAM: {physical_gb} GB"
            ),
            action_label: None,
            action_type: None,
            help_id: "health.ram",
        }
    }
}

/// `Double.ramGBLabel` (`AppConfig.swift:78`, `String(format: "%g",
/// self)`): drops a trailing `.0` for a whole-number value, otherwise
/// the shortest round-tripping decimal — matched exactly for the
/// whole-number case this crate's own callers always pass (RAM is
/// configured in whole/half GB steps); a value needing more than 6
/// significant digits (never produced by this app) would diverge from
/// C's `%g`, which this port doesn't reproduce byte-for-byte.
fn ram_gb_label(value: f64) -> String {
    if value == value.trunc() {
        format!("{}", value as i64)
    } else {
        let s = format!("{value}");
        s
    }
}

/// P8.23: the real "Add-on Jars" card — see this module's own doc for why
/// this is disk-plus-persisted-record only, never a live provider call.
/// `add_on_kind` is `None` for Vanilla (no add-on folder at all — `Gray`,
/// nothing to report). `installed_count` is a real `add_on_inventory::
/// scan_mods`/`scan_plugins` length; `problems` is the same
/// `LastStartupResult.problems` list `check_last_startup`/
/// `health_problems` already read.
pub fn check_component_jars(
    add_on_kind: Option<AddOnKind>,
    installed_count: usize,
    problems: &[StartupProblem],
) -> HealthCardResult {
    if add_on_kind.is_none() {
        return HealthCardResult {
            id: "componentJars",
            status: HealthStatus::Gray,
            detected_value: "This server has no add-on folder.".to_string(),
            action_label: None,
            action_type: None,
            help_id: "health.component-jars",
        };
    }
    if installed_count == 0 {
        return HealthCardResult {
            id: "componentJars",
            status: HealthStatus::Gray,
            detected_value: "No mods or plugins installed.".to_string(),
            action_label: None,
            action_type: None,
            help_id: "health.component-jars",
        };
    }

    let incompatible = problems
        .iter()
        .filter(|p| p.kind == StartupProblemKind::IncompatibleVersion)
        .count();
    let missing_dependency = problems
        .iter()
        .filter(|p| p.kind == StartupProblemKind::MissingDependency)
        .count();

    let noun = |n: usize| if n == 1 { "add-on" } else { "add-ons" };
    if incompatible > 0 || missing_dependency > 0 {
        let mut parts = Vec::new();
        if incompatible > 0 {
            parts.push(format!(
                "{incompatible} {} incompatible",
                noun(incompatible)
            ));
        }
        if missing_dependency > 0 {
            parts.push(format!("{missing_dependency} missing a dependency"));
        }
        return HealthCardResult {
            id: "componentJars",
            status: HealthStatus::Red,
            detected_value: format!(
                "{installed_count} add-on(s) installed. {}.",
                parts.join("; ")
            ),
            action_label: Some("Diagnose Add-ons"),
            action_type: Some("diagnoseStartup"),
            help_id: "health.component-jars",
        };
    }

    HealthCardResult {
        id: "componentJars",
        status: HealthStatus::Green,
        detected_value: format!("{installed_count} add-on(s) installed. No known problems."),
        action_label: None,
        action_type: None,
        help_id: "health.component-jars",
    }
}

/// `checkLastStartup(for:)` (`AppViewModel+HealthCards.swift:852-913`).
/// `formatted_started_at` is already-formatted display text — see this
/// module's own doc on why formatting isn't done here.
pub fn check_last_startup(
    result: Option<&LastStartupResult>,
    formatted_started_at: &str,
) -> HealthCardResult {
    let Some(result) = result else {
        return HealthCardResult {
            id: "lastStartup",
            status: HealthStatus::Gray,
            detected_value: "Start your server for the first time to see health data here."
                .to_string(),
            action_label: None,
            action_type: None,
            help_id: "health.last-startup",
        };
    };

    if result.was_clean {
        let soft_problems = result.problems.as_deref().unwrap_or(&[]);
        if !soft_problems.is_empty() {
            return HealthCardResult {
                id: "lastStartup",
                status: HealthStatus::Yellow,
                detected_value: format!(
                    "Last start: {formatted_started_at}\nServer started, but {} add-on{} failed to load.",
                    soft_problems.len(),
                    if soft_problems.len() == 1 { "" } else { "s" }
                ),
                action_label: Some("Diagnose Add-ons"),
                action_type: Some("diagnoseStartup"),
                help_id: "health.last-startup",
            };
        }
        return HealthCardResult {
            id: "lastStartup",
            status: HealthStatus::Green,
            detected_value: format!(
                "Last start: {formatted_started_at}\nNo fatal errors detected."
            ),
            action_label: None,
            action_type: None,
            help_id: "health.last-startup",
        };
    }

    if !result.fatal_errors.is_empty() {
        let preview = result
            .fatal_errors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let has_problems = result.problems.as_deref().is_some_and(|p| !p.is_empty());
        return HealthCardResult {
            id: "lastStartup",
            status: HealthStatus::Red,
            detected_value: format!("Last start: {formatted_started_at}\n{preview}"),
            action_label: Some(if has_problems {
                "Diagnose Startup"
            } else {
                "View Full Console Log"
            }),
            action_type: Some(if has_problems {
                "diagnoseStartup"
            } else {
                "openConsoleLog"
            }),
            help_id: "health.last-startup",
        };
    }

    if !result.warnings.is_empty() {
        return HealthCardResult {
            id: "lastStartup",
            status: HealthStatus::Yellow,
            detected_value: format!(
                "Last start: {formatted_started_at}\n{} warning(s) logged.",
                result.warnings.len()
            ),
            action_label: None,
            action_type: None,
            help_id: "health.last-startup",
        };
    }

    HealthCardResult {
        id: "lastStartup",
        status: HealthStatus::Yellow,
        detected_value: format!("Last start: {formatted_started_at}\nResult inconclusive."),
        action_label: None,
        action_type: None,
        help_id: "health.last-startup",
    }
}

// ---------------------------------------------------------------------
// availableActions / repair
// ---------------------------------------------------------------------

/// `mapProblem`'s three independent `if`s (`AppViewModel+APIWiringBackupsHealth
/// .swift:120-137`) — all three can fire at once; `update`/`install` are
/// listed here for schema completeness even though [`repair_problem`]
/// doesn't implement them this phase (see this module's own doc).
pub fn available_actions(problem: &StartupProblem) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if problem.kind == StartupProblemKind::IncompatibleVersion && problem.installed_file.is_some() {
        actions.push("update");
    }
    if problem.kind == StartupProblemKind::MissingDependency && problem.missing_dependency.is_some()
    {
        actions.push("install");
    }
    if problem.installed_jar_stem.is_some() {
        actions.push("disable");
        actions.push("delete");
    }
    actions
}

/// `diagnostics.crash.<kind-kebab-case>` (`helpid-contract.md` §4),
/// including the two kinds `StartupCrashAnalyzer` never actually
/// constructs (`Duplicate`/`Unknown`) — kept for contract completeness,
/// the same precedent P7.8 already established for those two dead
/// variants.
pub fn crash_help_id(kind: StartupProblemKind) -> &'static str {
    match kind {
        StartupProblemKind::MissingDependency => "diagnostics.crash.missing-dependency",
        StartupProblemKind::IncompatibleVersion => "diagnostics.crash.incompatible-version",
        StartupProblemKind::Duplicate => "diagnostics.crash.duplicate",
        StartupProblemKind::LoadError => "diagnostics.crash.load-error",
        StartupProblemKind::Unknown => "diagnostics.crash.unknown",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairAction {
    Disable,
    Delete,
}

#[derive(Debug)]
pub enum RepairError {
    /// The route-level running-server guard (source line 196-198) fires
    /// before the problem is even looked up — a caller-supplied bool,
    /// matching `server_versions::change_version`'s own `is_running`
    /// shape.
    ServerRunning,
    /// `"action_unavailable"` (source line 225-227, 232-234): this
    /// problem has no `installed_jar_stem` to act on.
    ActionUnavailable,
    Io(std::io::Error),
    /// This port's own strengthening over the oracle — see this
    /// module's own doc.
    VerificationFailed,
}

impl fmt::Display for RepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepairError::ServerRunning => write!(f, "server is running"),
            RepairError::ActionUnavailable => write!(f, "no repair target for this problem"),
            RepairError::Io(e) => write!(f, "{e}"),
            RepairError::VerificationFailed => {
                write!(f, "repair did not produce the expected on-disk state")
            }
        }
    }
}

impl std::error::Error for RepairError {}

/// `toggleMod`/`removeMod`/`togglePlugin`/`removePlugin`
/// (`AppViewModel+ModManagement.swift:192-233`,
/// `AppViewModel+PluginManagement.swift:112-166`) collapsed into one
/// port: all four are the same rename-to-`.disabled`/remove-file
/// operation keyed by a jar stem, over `mods/` or `plugins/` depending
/// on `add_on_dir`. Source resolves the current filename via a live
/// `discoveredMods`/`discoveredPlugins` scan; this port instead tries
/// the two filenames a stem can be under directly (`<stem>.jar` /
/// `<stem>.jar.disabled`), since `installed_jar_stem` alone is all a
/// caller here has.
pub fn repair_problem(
    fs: &dyn FileSystem,
    add_on_dir: &Path,
    problem: &StartupProblem,
    action: RepairAction,
    is_running: bool,
) -> Result<(), RepairError> {
    if is_running {
        return Err(RepairError::ServerRunning);
    }
    let stem = problem
        .installed_jar_stem
        .as_deref()
        .ok_or(RepairError::ActionUnavailable)?;
    let enabled_path = add_on_dir.join(format!("{stem}.jar"));
    let disabled_path = add_on_dir.join(format!("{stem}.jar.disabled"));

    match action {
        RepairAction::Delete => {
            let target = if fs.stat(&enabled_path).is_ok() {
                &enabled_path
            } else {
                &disabled_path
            };
            fs.remove(target).map_err(RepairError::Io)?;
            if fs.stat(&enabled_path).is_ok() || fs.stat(&disabled_path).is_ok() {
                return Err(RepairError::VerificationFailed);
            }
        }
        RepairAction::Disable => {
            if fs.stat(&enabled_path).is_ok() {
                fs.rename(&enabled_path, &disabled_path)
                    .map_err(RepairError::Io)?;
            }
            if fs.stat(&disabled_path).is_err() {
                return Err(RepairError::VerificationFailed);
            }
        }
    }
    Ok(())
}
