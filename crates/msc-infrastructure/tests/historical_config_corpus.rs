//! P5.24: runs every manifest-listed config in a corpus directory through
//! the real typed repository's load → save → reload path, in an isolated
//! temporary copy — never touching the corpus input itself.
//!
//! Driven entirely by `MSC2_HISTORICAL_CONFIGS_DIR` (a directory shaped
//! like `corpus/configs/`: a `manifest.json` with a `files` array, plus one
//! JSON file per entry) so `tools/phase5/real-corpus-check.py`'s exercise
//! mode can point this at either the real P5.3 corpus (P5.25) or this
//! step's own self-test fixtures (`tools/phase5/fixtures/exercise-pass/`)
//! without a rebuild. If the env var isn't set, this test is a no-op pass —
//! `cargo nextest run --workspace` must keep working on a clone that hasn't
//! run the Phase 5 real-corpus checker.
//!
//! Every file is exercised independently and reported on its own line
//! (`ok <file>` / `FAIL <file>: <reason>`) before the test decides pass or
//! fail, so one bad file doesn't hide the results for the rest.

use msc_domain::app_config_schema::AppConfig;
use msc_infrastructure::config_repository::{load_app_config, save_app_config};
use msc_infrastructure::fs::StdFileSystem;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const CONFIGS_DIR_ENV: &str = "MSC2_HISTORICAL_CONFIGS_DIR";

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-historical-config-corpus-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create isolated temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn historical_config_corpus_round_trips_through_the_real_reader() {
    let Some(dir) = std::env::var(CONFIGS_DIR_ENV).ok() else {
        println!("{CONFIGS_DIR_ENV} not set -- skipping (see P5.24/P5.25)");
        return;
    };
    let configs_dir = PathBuf::from(dir);
    let manifest_path = configs_dir.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path)
        .unwrap_or_else(|e| panic!("{}: {e}", manifest_path.display()));
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .unwrap_or_else(|e| panic!("{}: malformed manifest JSON: {e}", manifest_path.display()));
    let files = manifest
        .get("files")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| {
            panic!(
                "{}: manifest has no 'files' entries",
                manifest_path.display()
            )
        });
    assert!(
        !files.is_empty(),
        "{}: manifest lists no config files",
        manifest_path.display()
    );

    let mut failures = Vec::new();
    for entry in files {
        let name = entry
            .get("file")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| {
                panic!("{}: entry missing 'file': {entry}", manifest_path.display())
            });
        let source_path = configs_dir.join(name);
        match exercise_one_config(&source_path, name) {
            Ok(()) => println!("ok {name}"),
            Err(message) => {
                println!("FAIL {name}: {message}");
                failures.push(format!("{name}: {message}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} historical config(s) failed the real reader:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

/// Copies `source_path` into an isolated temp directory and runs it through
/// decode (`load_app_config`) → encode (`save_app_config`) → decode again,
/// asserting the two decodes agree and that `source_path` itself was never
/// touched. Never opens `source_path` for writing.
fn exercise_one_config(source_path: &Path, label: &str) -> Result<(), String> {
    let original_bytes =
        std::fs::read(source_path).map_err(|e| format!("read corpus file: {e}"))?;

    let temp = TempRoot::new(label);
    let work_path = temp.path.join("config.json");
    std::fs::write(&work_path, &original_bytes).map_err(|e| format!("stage copy: {e}"))?;

    let fs_impl = StdFileSystem;
    let defaults = AppConfig::default_config("/tmp/msc2-historical-config-corpus/servers");
    let now = SystemTime::now();

    let first =
        load_app_config(&fs_impl, &work_path, &defaults, now).map_err(|e| format!("load: {e}"))?;
    if first.corrupt_backup_path.is_some() {
        return Err("load treated this real evidence file as corrupt".to_string());
    }

    save_app_config(&fs_impl, &work_path, &first.config).map_err(|e| format!("save: {e}"))?;

    let second = load_app_config(&fs_impl, &work_path, &defaults, now)
        .map_err(|e| format!("reload: {e}"))?;
    if second.corrupt_backup_path.is_some() {
        return Err("reload treated the freshly-saved file as corrupt".to_string());
    }
    if second.config != first.config {
        return Err("decode -> save -> reload produced a different AppConfig".to_string());
    }

    let untouched_bytes =
        std::fs::read(source_path).map_err(|e| format!("recheck corpus file: {e}"))?;
    if untouched_bytes != original_bytes {
        return Err("corpus source file was mutated during exercise".to_string());
    }

    Ok(())
}
