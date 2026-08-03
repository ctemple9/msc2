//! Port of `fixtures/audit-log/`: one test per case, each loading its
//! fixture, building a `FakeFileSystem` from its `existingFiles`, and
//! exercising `AuditLog::log`/`prune_old_files`. No MSC 1 test file
//! exercises `AuditLogger` directly (see each fixture's own `notes`), so
//! these fixtures were characterized straight from `AuditLogger.swift`.

use msc_infrastructure::audit_log::{AuditLog, Entry};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audit-log")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        input: json["input"].clone(),
        expected: json["expected"].clone(),
    }
}

fn build_fs(input: &Value) -> FakeFileSystem {
    let mut fs = FakeFileSystem::new();
    if let Some(files) = input["existingFiles"].as_array() {
        for entry in files {
            let path = entry["path"].as_str().expect("existingFiles[].path");
            let contents = entry["contents"].as_str().unwrap_or("");
            fs = fs.with_file(path, contents.as_bytes().to_vec(), false);
        }
    }
    fs
}

fn entry_from(json: &Value) -> Entry {
    Entry {
        timestamp: time_from(json, "timestampUnixNanos"),
        client_ip: json["clientIp"].as_str().expect("clientIp").to_string(),
        token_label: json["tokenLabel"].as_str().expect("tokenLabel").to_string(),
        method: json["method"].as_str().expect("method").to_string(),
        path: json["path"].as_str().expect("path").to_string(),
        status_code: json["statusCode"].as_u64().expect("statusCode") as u16,
    }
}

fn time_from(json: &Value, field: &str) -> SystemTime {
    let nanos = json[field].as_u64().unwrap_or_else(|| panic!("{field}"));
    UNIX_EPOCH + Duration::from_nanos(nanos)
}

#[test]
fn audit_log_single_entry_round_trips_through_jsonl() {
    let case = "single-entry-round-trips-through-jsonl";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let dir = fixture.input["dir"].as_str().expect("dir");
    let entry = entry_from(&fixture.input["entry"]);

    let log = AuditLog::new(&fs, dir);
    log.log(&entry)
        .unwrap_or_else(|e| panic!("case {case}: log failed: {e}"));

    let file_path = PathBuf::from(fixture.expected["filePath"].as_str().expect("filePath"));
    let actual = fs
        .read(&file_path)
        .map(|bytes| String::from_utf8(bytes).expect("audit file is valid UTF-8"))
        .unwrap_or_else(|e| panic!("case {case}: could not read {}: {e}", file_path.display()));

    assert_eq!(
        actual,
        fixture.expected["fileContents"]
            .as_str()
            .expect("fileContents"),
        "case {case}: file contents mismatch"
    );
}

#[test]
fn audit_log_entries_from_concurrent_writers_preserve_call_order() {
    let case = "entries-from-concurrent-writers-preserve-call-order";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let dir = fixture.input["dir"].as_str().expect("dir");

    let log = AuditLog::new(&fs, dir);
    let entries = fixture.input["entries"].as_array().expect("entries");
    for entry_json in entries {
        let entry = entry_from(entry_json);
        log.log(&entry)
            .unwrap_or_else(|e| panic!("case {case}: log failed: {e}"));
    }

    let file_path = PathBuf::from(fixture.expected["filePath"].as_str().expect("filePath"));
    let actual = fs
        .read(&file_path)
        .map(|bytes| String::from_utf8(bytes).expect("audit file is valid UTF-8"))
        .unwrap_or_else(|e| panic!("case {case}: could not read {}: {e}", file_path.display()));
    let actual_lines: Vec<&str> = actual.lines().collect();

    let expected_lines: Vec<&str> = fixture.expected["fileLines"]
        .as_array()
        .expect("fileLines")
        .iter()
        .map(|v| v.as_str().expect("fileLines[] is a string"))
        .collect();

    assert_eq!(
        actual_lines, expected_lines,
        "case {case}: line order mismatch"
    );

    // The fixture above only exercises sequential submission — a JSON
    // fixture can't itself encode genuine thread scheduling. This second
    // half of the same test exercises the property `AuditLog`'s doc
    // comment actually promises: one writer lock *per instance*, so
    // concurrent calls to `log` on the *same* instance never interleave
    // or corrupt each other's lines — the real production shape, where
    // one long-lived `AuditLog` is shared by every caller. An earlier
    // version of this test built a separate `AuditLog` per thread, which
    // gives each thread its own independent lock and doesn't exercise
    // that guarantee at all; it relied on a `thread::sleep` stagger to
    // keep the resulting races from colliding, which was flaky under CI
    // load and occasionally lost entries outright rather than just
    // reordering them. `std::thread::scope` lets every thread borrow the
    // one shared, non-'static `AuditLog` directly, so the real lock is
    // what's under test.
    concurrent_threads_never_interleave_or_corrupt();
}

fn concurrent_threads_never_interleave_or_corrupt() {
    let fs = FakeFileSystem::new().with_file("/srv/app/audit/.keep", Vec::new(), false);
    let log = AuditLog::new(&fs, "/srv/app/audit");
    let barrier = std::sync::Barrier::new(3);

    thread::scope(|scope| {
        for i in 0..3u16 {
            let log = &log;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                let entry = Entry {
                    timestamp: UNIX_EPOCH + Duration::from_secs(1_700_000_000 + u64::from(i)),
                    client_ip: format!("10.0.0.{i}"),
                    token_label: "admin".to_string(),
                    method: "GET".to_string(),
                    path: "/status".to_string(),
                    status_code: 200,
                };
                log.log(&entry).expect("log");
            });
        }
    });

    let bytes = fs
        .read(Path::new("/srv/app/audit/audit-2023-11-14.jsonl"))
        .expect("audit file exists");
    let text = String::from_utf8(bytes).expect("valid UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "all three entries present, none interleaved: {lines:?}"
    );
    for i in 0..3u16 {
        let needle = format!("\"ip\":\"10.0.0.{i}\"");
        let matches = lines.iter().filter(|line| line.contains(&needle)).count();
        assert_eq!(
            matches, 1,
            "expected exactly one line for 10.0.0.{i}, found {matches}: {lines:?}"
        );
    }
}

#[test]
fn audit_log_file_older_than_retention_window_is_pruned() {
    let case = "file-older-than-retention-window-is-pruned";
    run_prune_case(case);
}

#[test]
fn audit_log_file_at_retention_boundary_is_kept() {
    let case = "file-at-retention-boundary-is-kept";
    run_prune_case(case);
}

fn run_prune_case(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let dir = fixture.input["dir"].as_str().expect("dir");
    let now = time_from(&fixture.input, "nowUnixNanos");

    let log = AuditLog::new(&fs, dir);
    log.prune_old_files(now)
        .unwrap_or_else(|e| panic!("case {case}: prune_old_files failed: {e}"));

    let remaining: Vec<String> = fs
        .list(Path::new(dir))
        .unwrap_or_else(|e| panic!("case {case}: list failed: {e}"))
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some(".keep"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let expected: Vec<String> = fixture.expected["remainingFiles"]
        .as_array()
        .expect("remainingFiles")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("remainingFiles[] is a string")
                .to_string()
        })
        .collect();

    assert_eq!(remaining, expected, "case {case}: remaining files mismatch");
}

#[test]
fn audit_log_corrupt_or_partial_line_does_not_crash_writer() {
    let case = "corrupt-or-partial-line-does-not-crash-writer";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let dir = fixture.input["dir"].as_str().expect("dir");
    let entry = entry_from(&fixture.input["entry"]);

    let log = AuditLog::new(&fs, dir);
    log.log(&entry)
        .unwrap_or_else(|e| panic!("case {case}: log failed: {e}"));

    let file_path = PathBuf::from(fixture.expected["filePath"].as_str().expect("filePath"));
    let actual = fs
        .read(&file_path)
        .map(|bytes| String::from_utf8(bytes).expect("audit file is valid UTF-8"))
        .unwrap_or_else(|e| panic!("case {case}: could not read {}: {e}", file_path.display()));

    assert_eq!(
        actual,
        fixture.expected["fileContents"]
            .as_str()
            .expect("fileContents"),
        "case {case}: file contents mismatch"
    );
}
