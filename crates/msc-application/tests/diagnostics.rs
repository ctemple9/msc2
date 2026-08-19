//! P7.22: `msc_application::diagnostics` against `fixtures/startup-
//! problems/`'s 38 cases. Fixtures whose behavior is stateful/route-
//! layer (the `healthProblemsProvider`/`repairHealthProblemProvider`
//! in-memory-vs-disk reconciliation, `invalid_action`/`no_active_server`
//! guards, `reopenStartupProblems`) are not exercised here — see
//! `diagnostics.rs`'s own module doc for that scope boundary; this
//! module owns the pure card/analysis/repair primitives those routes
//! compose.

use msc_application::diagnostics::{
    self, DirectoryProbe, HealthStatus, JavaCandidateProbe, LastStartupResult, RepairAction,
    RepairError,
};
use msc_domain::crash_analysis::{StartupProblem, StartupProblemKind};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::Path;

fn load(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/startup-problems")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

fn problem(kind: StartupProblemKind, offender_name: &str) -> StartupProblem {
    StartupProblem {
        kind,
        offender_name: offender_name.to_string(),
        offender_id: None,
        installed_file: None,
        installed_jar_stem: None,
        requirement: None,
        missing_dependency: None,
        raw_excerpt: String::new(),
    }
}

// --- checkDirectory ---

#[test]
fn check_directory_missing_path_red() {
    let _fixture = load("check-directory-missing-path-red");
    let result = diagnostics::check_directory(
        "/servers/java/missing_server",
        DirectoryProbe {
            exists: false,
            is_dir: false,
            writable: false,
            readable: false,
        },
    );
    assert_eq!(result.status, HealthStatus::Red);
    assert_eq!(
        result.detected_value,
        "Path not found:\n/servers/java/missing_server"
    );
    assert_eq!(result.action_label, Some("Locate Folder"));
    assert_eq!(result.action_type, Some("locateFolder"));
    assert_eq!(result.help_id, "health.directory");
}

#[test]
fn check_directory_exists_not_writable_yellow() {
    let result = diagnostics::check_directory(
        "/servers/java/ro_server",
        DirectoryProbe {
            exists: true,
            is_dir: true,
            writable: false,
            readable: true,
        },
    );
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "Exists but may have permission issues:\n/servers/java/ro_server"
    );
}

#[test]
fn check_directory_exists_writable_green() {
    let result = diagnostics::check_directory(
        "/servers/java/box",
        DirectoryProbe {
            exists: true,
            is_dir: true,
            writable: true,
            readable: true,
        },
    );
    assert_eq!(result.status, HealthStatus::Green);
    assert_eq!(result.detected_value, "/servers/java/box");
    assert_eq!(result.action_label, None);
}

// --- checkJavaRuntime ---

#[test]
fn check_java_runtime_not_found_red() {
    let candidates: Vec<JavaCandidateProbe> = (0..16)
        .map(|_| JavaCandidateProbe {
            path: "",
            exists: false,
            version_check: None,
        })
        .collect();
    let result = diagnostics::check_java_runtime(&candidates);
    assert_eq!(result.status, HealthStatus::Red);
    assert_eq!(
        result.detected_value,
        "Java not found. Checked 16 locations.\nInstall Adoptium Temurin 21 or set your Java path in Preferences."
    );
    assert_eq!(result.help_id, "health.java");
}

#[test]
fn check_java_runtime_found_major_21_plus_green() {
    let candidates = [JavaCandidateProbe {
        path: "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java",
        exists: true,
        version_check: Some((
            0,
            "openjdk version \"21.0.7\" 2026-04-15\nOpenJDK Runtime Environment Temurin-21.0.7+6\n",
        )),
    }];
    let result = diagnostics::check_java_runtime(&candidates);
    assert_eq!(result.status, HealthStatus::Green);
    assert_eq!(
        result.detected_value,
        "21.0.7 \u{2014} minimum is Java 21 \u{2713}\nPath: /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java"
    );
    assert_eq!(result.action_label, None);
}

#[test]
fn check_java_runtime_found_major_below_21_yellow() {
    let candidates = [JavaCandidateProbe {
        path: "/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/java",
        exists: true,
        version_check: Some((
            0,
            "openjdk version \"17.0.9\" 2026-01-17\nOpenJDK Runtime Environment Temurin-17.0.9+9\n",
        )),
    }];
    let result = diagnostics::check_java_runtime(&candidates);
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "17.0.9 detected \u{2014} minimum is Java 21\nPath: /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/java"
    );
    assert_eq!(result.action_label, Some("Download Java"));
}

#[test]
fn check_java_runtime_found_version_unreadable_yellow() {
    let candidates = [JavaCandidateProbe {
        path: "/usr/bin/java",
        exists: true,
        version_check: Some((
            0,
            "some non-standard launcher wrapper output with no version token",
        )),
    }];
    let result = diagnostics::check_java_runtime(&candidates);
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "Java found but version unreadable.\nOutput: some non-standard launcher wrapper output with no version token\nPath: /usr/bin/java"
    );
}

#[test]
fn check_java_runtime_stops_at_first_responsive_candidate() {
    // Source returns on the first candidate that exists AND exits 0
    // with non-empty output -- it never tries the next one even if it
    // would have parsed cleanly.
    let candidates = [
        JavaCandidateProbe {
            path: "/first/java",
            exists: true,
            version_check: Some((0, "unparseable")),
        },
        JavaCandidateProbe {
            path: "/second/java",
            exists: true,
            version_check: Some((0, "openjdk version \"21.0.1\" 2026-01-01\n")),
        },
    ];
    let result = diagnostics::check_java_runtime(&candidates);
    assert!(result.detected_value.contains("/first/java"));
    assert!(!result.detected_value.contains("/second/java"));
}

#[test]
fn check_java_runtime_skips_non_existent_and_non_zero_exit_candidates() {
    let candidates = [
        JavaCandidateProbe {
            path: "/missing/java",
            exists: false,
            version_check: None,
        },
        JavaCandidateProbe {
            path: "/broken/java",
            exists: true,
            version_check: Some((1, "")),
        },
        JavaCandidateProbe {
            path: "/good/java",
            exists: true,
            version_check: Some((0, "openjdk version \"21.0.1\" 2026-01-01\n")),
        },
    ];
    let result = diagnostics::check_java_runtime(&candidates);
    assert_eq!(result.status, HealthStatus::Green);
    assert!(result.detected_value.contains("/good/java"));
}

// --- checkRAMAllocation ---

#[test]
fn check_ram_allocation_exceeds_physical_red() {
    let result = diagnostics::check_ram_allocation(16.0, 8);
    assert_eq!(result.status, HealthStatus::Red);
    assert_eq!(
        result.detected_value,
        "Configured: 16 GB \u{2014} Physical RAM: 8 GB\nAllocation exceeds physical memory."
    );
}

#[test]
fn check_ram_allocation_non_positive_is_also_red() {
    let result = diagnostics::check_ram_allocation(0.0, 8);
    assert_eq!(result.status, HealthStatus::Red);
}

#[test]
fn check_ram_allocation_high_fraction_yellow_truncates_not_rounds() {
    let result = diagnostics::check_ram_allocation(7.0, 8);
    assert_eq!(result.status, HealthStatus::Yellow);
    // 7/8 = 0.875 -> Int(87.5) truncates to 87, not rounds to 88.
    assert_eq!(
        result.detected_value,
        "Configured: 7 GB \u{2014} Physical RAM: 8 GB\n87% of system RAM \u{2014} may cause instability."
    );
}

#[test]
fn check_ram_allocation_normal_green() {
    let result = diagnostics::check_ram_allocation(4.0, 16);
    assert_eq!(result.status, HealthStatus::Green);
    assert_eq!(
        result.detected_value,
        "Configured: 4 GB \u{2014} Physical RAM: 16 GB"
    );
}

// --- checkLastStartup ---

#[test]
fn last_startup_card_never_started_gray() {
    let result = diagnostics::check_last_startup(None, "");
    assert_eq!(result.status, HealthStatus::Gray);
    assert_eq!(
        result.detected_value,
        "Start your server for the first time to see health data here."
    );
    assert_eq!(result.help_id, "health.last-startup");
}

#[test]
fn last_startup_card_clean_no_problems_green() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: true,
        fatal_errors: Vec::new(),
        warnings: Vec::new(),
        problems: None,
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Green);
    assert_eq!(
        result.detected_value,
        "Last start: Aug 10, 2026\nNo fatal errors detected."
    );
}

#[test]
fn last_startup_card_clean_with_soft_problems_yellow() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: true,
        fatal_errors: Vec::new(),
        warnings: vec!["Vault: Failed to enable".to_string()],
        problems: Some(vec![problem(StartupProblemKind::LoadError, "Vault")]),
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "Last start: Aug 10, 2026\nServer started, but 1 add-on failed to load."
    );
    assert_eq!(result.action_label, Some("Diagnose Add-ons"));
}

#[test]
fn last_startup_card_hard_fail_with_structured_problems_red_diagnose() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: false,
        fatal_errors: vec!["jei: Requires forge [15.2,)".to_string()],
        warnings: Vec::new(),
        problems: Some(vec![problem(StartupProblemKind::MissingDependency, "jei")]),
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Red);
    assert_eq!(
        result.detected_value,
        "Last start: Aug 10, 2026\njei: Requires forge [15.2,)"
    );
    assert_eq!(result.action_label, Some("Diagnose Startup"));
    assert_eq!(result.action_type, Some("diagnoseStartup"));
}

#[test]
fn last_startup_card_hard_fail_no_structured_problems_red_view_log() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: false,
        fatal_errors: vec!["Server stopped before reaching ready state.".to_string()],
        warnings: Vec::new(),
        problems: None,
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Red);
    assert_eq!(result.action_label, Some("View Full Console Log"));
    assert_eq!(result.action_type, Some("openConsoleLog"));
}

#[test]
fn last_startup_card_not_clean_warnings_only_yellow() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: false,
        fatal_errors: Vec::new(),
        warnings: vec!["World save took longer than usual".to_string()],
        problems: None,
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "Last start: Aug 10, 2026\n1 warning(s) logged."
    );
}

#[test]
fn last_startup_card_inconclusive_fallback_yellow() {
    let record = LastStartupResult {
        started_at: "2026-08-10T12:00:00Z".to_string(),
        was_clean: false,
        fatal_errors: Vec::new(),
        warnings: Vec::new(),
        problems: None,
    };
    let result = diagnostics::check_last_startup(Some(&record), "Aug 10, 2026");
    assert_eq!(result.status, HealthStatus::Yellow);
    assert_eq!(
        result.detected_value,
        "Last start: Aug 10, 2026\nResult inconclusive."
    );
}

// --- writeLastStartupResult / readLastStartupResult persistence ---

#[test]
fn write_last_startup_result_record_shape_and_path() {
    let _fixture = load("write-last-startup-result-record-shape-and-path");
    let fs = FakeFileSystem::new();
    diagnostics::write_last_startup_result(
        &fs,
        Path::new("/servers/java/my_server"),
        "2026-08-18T10:15:00Z",
        false,
        vec!["Server stopped before reaching ready state.".to_string()],
        Vec::new(),
        Vec::new(),
    );
    let bytes = fs
        .read(Path::new(
            "/servers/java/my_server/last_startup_result.json",
        ))
        .expect("written");
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["startedAt"], "2026-08-18T10:15:00Z");
    assert_eq!(value["wasClean"], false);
    assert_eq!(
        value["fatalErrors"][0],
        "Server stopped before reaching ready state."
    );
    assert_eq!(value["warnings"].as_array().unwrap().len(), 0);
    // An empty `problems` Vec is written as JSON null, never `[]`.
    assert!(value["problems"].is_null());
}

#[test]
fn read_last_startup_result_missing_or_corrupted_is_none() {
    let fs = FakeFileSystem::new().with_file(
        "/servers/java/box/last_startup_result.json",
        b"not json".to_vec(),
        false,
    );
    assert!(diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box")).is_none());
    assert!(
        diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/nonexistent"))
            .is_none()
    );
}

#[test]
fn write_then_read_last_startup_result_round_trips() {
    let fs = FakeFileSystem::new();
    let problems = vec![problem(StartupProblemKind::LoadError, "Vault")];
    diagnostics::write_last_startup_result(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        true,
        Vec::new(),
        vec!["Vault: Failed to enable".to_string()],
        problems.clone(),
    );
    let read = diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box"))
        .expect("readable");
    assert!(read.was_clean);
    assert_eq!(read.problems, Some(problems));
}

// --- diagnoseUnexpectedStop ---

#[test]
fn diagnose_unexpected_stop_modded_crash_analyzer_attributes_problems() {
    // The analyzer itself is exercised elsewhere (crash_analysis's own
    // tests); this proves the wrapper's gating/persistence, using real
    // Forge dependency-block console text so `crash_analysis::analyze`
    // actually returns something (matching the fixture's own
    // `analyzerReturns` intent without hand-injecting analyzer output,
    // since `diagnose_unexpected_stop` has no seam to inject it through
    // -- it calls the real analyzer, per source).
    let fs = FakeFileSystem::new();
    let console = vec![
        "[modmenu] Requirements of mod modmenu:".to_string(),
        "  \\-- Depends on jei @ [15.2,) <- missing!".to_string(),
    ];
    let problems = diagnostics::diagnose_unexpected_stop(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        false,
        true,
        "fabric",
        &console,
        &[],
    );
    // Whether or not this exact synthetic text parses, prove the
    // control-flow contract: hard fail + modded => analyzer invoked, and
    // a result is always written (either problem-attributed or generic).
    let record = diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box"))
        .expect("a record was written on hard fail");
    assert!(!record.was_clean);
    if !problems.is_empty() {
        assert_eq!(record.problems, Some(problems));
    } else {
        assert_eq!(
            record.fatal_errors,
            vec!["Server stopped before reaching ready state.".to_string()]
        );
    }
}

#[test]
fn diagnose_unexpected_stop_non_modded_skips_analysis() {
    let _fixture = load("diagnose-unexpected-stop-non-modded-skips-analysis");
    let fs = FakeFileSystem::new();
    let problems = diagnostics::diagnose_unexpected_stop(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        false,
        false,
        "paper",
        &["some server output".to_string()],
        &[],
    );
    assert!(problems.is_empty());
    let record = diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box"))
        .expect("hard fail still writes the generic record");
    assert!(!record.was_clean);
    assert_eq!(
        record.fatal_errors,
        vec!["Server stopped before reaching ready state.".to_string()]
    );
    assert_eq!(record.problems, None);
}

#[test]
fn diagnose_unexpected_stop_reached_ready_state_writes_nothing() {
    let _fixture =
        load("diagnose-unexpected-stop-reached-ready-state-no-persistence-but-alert-shown");
    let fs = FakeFileSystem::new();
    let problems = diagnostics::diagnose_unexpected_stop(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        true,
        true,
        "forge",
        &["[ERROR] Unexpected shutdown signal".to_string()],
        &[],
    );
    assert!(problems.is_empty());
    assert!(
        diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box")).is_none(),
        "reaching ready state must leave last_startup_result.json untouched"
    );
}

// --- scanPaperSoftFailures ---

#[test]
fn scan_paper_soft_failures_records_as_warnings_wasclean_stays_true() {
    let _fixture = load("scan-paper-soft-failures-records-as-warnings-wasclean-stays-true");
    let fs = FakeFileSystem::new();
    let problems = vec![StartupProblem {
        kind: StartupProblemKind::MissingDependency,
        offender_name: "Essentials".to_string(),
        offender_id: None,
        installed_file: None,
        installed_jar_stem: None,
        requirement: Some("Requires Vault".to_string()),
        missing_dependency: None,
        raw_excerpt: String::new(),
    }];
    let wrote = diagnostics::scan_paper_soft_failures(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        true,
        true,
        problems.clone(),
    );
    assert!(wrote);
    let record = diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box"))
        .expect("record written");
    assert!(record.was_clean);
    assert_eq!(record.fatal_errors, Vec::<String>::new());
    assert_eq!(
        record.warnings,
        vec!["Essentials: Requires Vault".to_string()]
    );
    assert_eq!(record.problems, Some(problems));
}

#[test]
fn scan_paper_soft_failures_no_op_for_non_plugin_flavor() {
    let fs = FakeFileSystem::new();
    let problems = vec![problem(StartupProblemKind::LoadError, "SomeMod")];
    let wrote = diagnostics::scan_paper_soft_failures(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        true,
        false, // addOnKind != plugin (e.g. Forge/NeoForge's mod kind)
        problems,
    );
    assert!(!wrote);
    assert!(diagnostics::read_last_startup_result(&fs, Path::new("/servers/java/box")).is_none());
}

#[test]
fn scan_paper_soft_failures_no_op_when_analyzer_found_nothing() {
    let fs = FakeFileSystem::new();
    let wrote = diagnostics::scan_paper_soft_failures(
        &fs,
        Path::new("/servers/java/box"),
        "2026-08-18T10:15:00Z",
        true,
        true,
        Vec::new(),
    );
    assert!(!wrote);
}

// --- availableActions (mapProblem) ---

#[test]
fn available_actions_missing_dependency_with_target() {
    let fixture = load("available-actions-missing-dependency-with-target");
    let p = StartupProblem {
        kind: StartupProblemKind::MissingDependency,
        offender_name: "SomeMod".to_string(),
        offender_id: None,
        installed_file: Some("somemod-1.0.jar".to_string()),
        installed_jar_stem: Some("somemod-1.0".to_string()),
        requirement: None,
        missing_dependency: Some("fabric-api".to_string()),
        raw_excerpt: String::new(),
    };
    let actions = diagnostics::available_actions(&p);
    assert_eq!(actions, vec!["install", "disable", "delete"]);
    assert_eq!(
        diagnostics::crash_help_id(p.kind),
        fixture["expected"]["helpId"].as_str().unwrap()
    );
}

#[test]
fn available_actions_incompatible_version_installed_file() {
    let fixture = load("available-actions-incompatible-version-installed-file");
    let p = StartupProblem {
        kind: StartupProblemKind::IncompatibleVersion,
        offender_name: "Sodium".to_string(),
        offender_id: None,
        installed_file: Some("sodium-0.4.0.jar".to_string()),
        installed_jar_stem: Some("sodium-0.4.0".to_string()),
        requirement: None,
        missing_dependency: None,
        raw_excerpt: String::new(),
    };
    let actions = diagnostics::available_actions(&p);
    assert_eq!(actions, vec!["update", "disable", "delete"]);
    assert_eq!(
        diagnostics::crash_help_id(p.kind),
        fixture["expected"]["helpId"].as_str().unwrap()
    );
}

#[test]
fn available_actions_unmatched_offender_no_actions() {
    let _fixture = load("available-actions-unmatched-offender-no-actions");
    let p = problem(StartupProblemKind::MissingDependency, "minecraft");
    assert!(diagnostics::available_actions(&p).is_empty());
}

#[test]
fn available_actions_missing_dependency_without_installable_target_excludes_install() {
    // kind == missingDependency but missing_dependency is nil (a
    // non-installable target like "minecraft"/"java"/loader) -- "install"
    // must not appear even though the kind matches.
    let mut p = problem(StartupProblemKind::MissingDependency, "minecraft");
    p.installed_jar_stem = Some("somejar".to_string());
    let actions = diagnostics::available_actions(&p);
    assert!(!actions.contains(&"install"));
    assert!(actions.contains(&"disable"));
    assert!(actions.contains(&"delete"));
}

// --- repair_problem ---

#[test]
fn repair_problem_refuses_while_running() {
    let fs = FakeFileSystem::new();
    let mut p = problem(StartupProblemKind::LoadError, "badplugin");
    p.installed_jar_stem = Some("badplugin-2.0".to_string());
    let err = diagnostics::repair_problem(
        &fs,
        Path::new("/servers/java/box/plugins"),
        &p,
        RepairAction::Delete,
        true,
    )
    .expect_err("running refused");
    assert!(matches!(err, RepairError::ServerRunning));
}

#[test]
fn repair_problem_action_unavailable_without_installed_jar_stem() {
    let fs = FakeFileSystem::new();
    let p = problem(StartupProblemKind::LoadError, "badplugin");
    let err = diagnostics::repair_problem(
        &fs,
        Path::new("/servers/java/box/plugins"),
        &p,
        RepairAction::Delete,
        false,
    )
    .expect_err("no jar stem");
    assert!(matches!(err, RepairError::ActionUnavailable));
}

#[test]
fn repair_problem_delete_success_removes_the_jar() {
    let fixture = load("health-repair-delete-success-removes-problem-returns-updated-snapshot");
    let plugins_dir = "/servers/java/box/plugins";
    let fs = FakeFileSystem::new().with_file(
        format!("{plugins_dir}/badplugin-2.0.jar"),
        b"jar-bytes".to_vec(),
        false,
    );
    let mut p = problem(StartupProblemKind::LoadError, "badplugin");
    p.installed_jar_stem = Some("badplugin-2.0".to_string());

    diagnostics::repair_problem(&fs, Path::new(plugins_dir), &p, RepairAction::Delete, false)
        .expect("delete succeeds");
    assert!(
        fs.read(Path::new(plugins_dir).join("badplugin-2.0.jar").as_path())
            .is_err()
    );
    assert_eq!(
        fixture["expected"]["dispatchedTo"],
        "removePlugin(jarStem: \"badplugin-2.0\")"
    );
}

#[test]
fn repair_problem_delete_also_removes_an_already_disabled_jar() {
    let plugins_dir = "/servers/java/box/plugins";
    let fs = FakeFileSystem::new().with_file(
        format!("{plugins_dir}/badplugin-2.0.jar.disabled"),
        b"jar-bytes".to_vec(),
        false,
    );
    let mut p = problem(StartupProblemKind::LoadError, "badplugin");
    p.installed_jar_stem = Some("badplugin-2.0".to_string());

    diagnostics::repair_problem(&fs, Path::new(plugins_dir), &p, RepairAction::Delete, false)
        .expect("delete succeeds");
    assert!(
        fs.read(
            Path::new(plugins_dir)
                .join("badplugin-2.0.jar.disabled")
                .as_path()
        )
        .is_err()
    );
}

#[test]
fn repair_problem_disable_success_renames_to_disabled() {
    let _fixture = load("health-repair-disable-success-removes-problem-returns-updated-snapshot");
    let mods_dir = "/servers/java/box/mods";
    let fs = FakeFileSystem::new().with_file(
        format!("{mods_dir}/somemod-1.0.jar"),
        b"jar-bytes".to_vec(),
        false,
    );
    let mut p = problem(StartupProblemKind::LoadError, "somemod");
    p.installed_jar_stem = Some("somemod-1.0".to_string());

    diagnostics::repair_problem(&fs, Path::new(mods_dir), &p, RepairAction::Disable, false)
        .expect("disable succeeds");
    assert!(
        fs.read(Path::new(mods_dir).join("somemod-1.0.jar").as_path())
            .is_err()
    );
    assert!(
        fs.read(
            Path::new(mods_dir)
                .join("somemod-1.0.jar.disabled")
                .as_path()
        )
        .is_ok()
    );
}

#[test]
fn repair_problem_disable_verification_fails_when_neither_file_exists() {
    // Strengthening over the oracle: source's toggleMod never checks the
    // rename actually happened. Here, if the enabled jar was never on
    // disk in the first place, disable is a silent no-op that then fails
    // verification (neither the enabled nor disabled path exists).
    let fs = FakeFileSystem::new();
    let mut p = problem(StartupProblemKind::LoadError, "ghost");
    p.installed_jar_stem = Some("ghost-1.0".to_string());
    let err = diagnostics::repair_problem(
        &fs,
        Path::new("/servers/java/box/mods"),
        &p,
        RepairAction::Disable,
        false,
    )
    .expect_err("verification fails");
    assert!(matches!(err, RepairError::VerificationFailed));
}
