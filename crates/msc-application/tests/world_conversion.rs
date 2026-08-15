//! Port of `fixtures/world-conversion/`'s 10 cases (P6.7), exercising
//! `msc_application::world_conversion::convert_world` (P6.19) — the
//! transactional port of `AppViewModel+WorldConversion.
//! performWorldConversion` behind a fakeable [`WorldConverter`] Chunker
//! boundary.
//!
//! Real on-disk server directories, the same "genuinely disk-shaped"
//! precedent every other archive-touching test file in this phase
//! already set (`world_slot_crud.rs`, `world_activation.rs`,
//! `world_mutations.rs`) — required here too, since source-zip
//! extraction, Chunker-output packaging, and target-slot archiving all
//! go through the real archive engine, not the injectable `FileSystem`.
//!
//! Test functions are prefixed `world_conversion_` so the plan's Verify
//! command (a plain nextest substring filter on test name) selects them.

use msc_application::world_conversion::{
    ConversionError, ConversionPlacement, WorldConverter, convert_world,
};
use msc_domain::identity::ServerType;
use msc_domain::world::WorldSlot;
use msc_infrastructure::archive;
use msc_infrastructure::fs::{FileSystem, Metadata, StdFileSystem};
use msc_infrastructure::world_store;
use std::cell::Cell;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

// ---------------------------------------------------------------------
// Shared test infrastructure
// ---------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-world-conversion-test-{label}-{}",
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

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, contents) in entries {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

fn write_source_slot_zip(server_dir: &Path, slot_id: &str, entries: &[(&str, &[u8])]) {
    write_zip(&world_store::zip_path(server_dir, slot_id), entries);
}

fn make_slot(id: &str, level_name: Option<&str>) -> WorldSlot {
    WorldSlot {
        id: id.to_string(),
        name: "Source world".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: level_name.map(str::to_string),
        world_seed: None,
        zip_size_bytes: None,
    }
}

/// `ChunkerManager::convert`'s recorded invocation — proves the right
/// input/output/format/java-path made it through, without needing a
/// real process boundary (see the crate's `world_conversion` module
/// doc for why no production `WorldConverter` exists yet).
#[derive(Debug, Clone)]
struct RecordedInvocation {
    input_dir: PathBuf,
    output_dir: PathBuf,
    target_format: String,
    java_path: String,
}

struct FakeWorldConverter {
    java_resolvable: bool,
    installed: bool,
    /// `Err(_)` reproduces a non-zero Chunker exit
    /// (`ChunkerError.conversionFailed`); `Ok(files)` simulates Chunker
    /// writing `files` (relative name, contents) directly into
    /// `output_dir`, matching source's own "Chunker writes world files
    /// DIRECTLY into the output directory" contract.
    convert_result: Result<Vec<(&'static str, &'static [u8])>, String>,
    is_installed_called: Cell<bool>,
    resolve_java_path_called: Cell<bool>,
    invocation: Mutex<Option<RecordedInvocation>>,
}

impl FakeWorldConverter {
    fn ready(convert_result: Result<Vec<(&'static str, &'static [u8])>, String>) -> Self {
        Self {
            java_resolvable: true,
            installed: true,
            convert_result,
            is_installed_called: Cell::new(false),
            resolve_java_path_called: Cell::new(false),
            invocation: Mutex::new(None),
        }
    }

    fn invocation(&self) -> RecordedInvocation {
        self.invocation
            .lock()
            .unwrap()
            .clone()
            .expect("convert() was never called")
    }
}

impl WorldConverter for FakeWorldConverter {
    fn is_installed(&self) -> bool {
        self.is_installed_called.set(true);
        self.installed
    }

    fn resolve_java_path(&self, _configured_java_path: &str) -> Option<String> {
        self.resolve_java_path_called.set(true);
        self.java_resolvable.then(|| "/usr/bin/java".to_string())
    }

    fn convert(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        target_format: &str,
        java_path: &str,
        progress: &mut dyn FnMut(&str),
    ) -> Result<(), String> {
        *self.invocation.lock().unwrap() = Some(RecordedInvocation {
            input_dir: input_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            target_format: target_format.to_string(),
            java_path: java_path.to_string(),
        });
        match &self.convert_result {
            Ok(files) => {
                fs::create_dir_all(output_dir).unwrap();
                for (name, contents) in files {
                    fs::write(output_dir.join(name), contents).unwrap();
                }
                progress("Chunker output line");
                Ok(())
            }
            Err(msg) => Err(msg.clone()),
        }
    }
}

/// Wraps [`StdFileSystem`], failing every `write` to one specific path —
/// deterministically reproduces "the copy/write half of an operation
/// fails after an earlier step already committed," the same shape
/// `FakeFileSystem::with_failing_rename` gives P6.12/13's own tests, but
/// targeting `write` (what this module's zip-copy and activation-
/// manifest writes actually call) and portable across platforms (no
/// `#[cfg(unix)]` permission trick needed).
struct FailWriteAt {
    inner: StdFileSystem,
    fail_path: PathBuf,
}

impl FileSystem for FailWriteAt {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read(path)
    }
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        if path == self.fail_path {
            return Err(io::Error::other("simulated write failure"));
        }
        self.inner.write(path, contents)
    }
    fn stat(&self, path: &Path) -> io::Result<Metadata> {
        self.inner.stat(path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        self.inner.remove(path)
    }
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }
    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.read_link(path)
    }
}

const JAVA_FORMAT: &str = "JAVA_1_21_4";

#[allow(clippy::too_many_arguments)]
fn run(
    fs: &dyn FileSystem,
    converter: &dyn WorldConverter,
    source_dir: &Path,
    source_slot: &WorldSlot,
    source_type: ServerType,
    target_dir: &Path,
    target_type: ServerType,
    target_raw_level_name: Option<&str>,
    placement: ConversionPlacement,
    backup_ok: bool,
    log: &mut Vec<String>,
) -> Result<WorldSlot, ConversionError> {
    convert_world(
        fs,
        converter,
        "",
        source_dir,
        source_slot,
        source_type,
        false,
        target_dir,
        target_type,
        target_raw_level_name,
        JAVA_FORMAT,
        placement,
        false,
        "2026-08-15T00:00:00Z",
        || backup_ok,
        |line: &str| log.push(line.to_string()),
    )
}

// ---------------------------------------------------------------------
// guard-order-java-path-checked-before-jar-installed
// ---------------------------------------------------------------------

#[test]
fn world_conversion_guard_order_java_checked_before_jar_installed() {
    let source = TempDir::new("guard-order-source");
    let target = TempDir::new("guard-order-target");
    let fs = StdFileSystem;
    let converter = FakeWorldConverter {
        java_resolvable: false,
        installed: false,
        convert_result: Ok(vec![]),
        is_installed_called: Cell::new(false),
        resolve_java_path_called: Cell::new(false),
        invocation: Mutex::new(None),
    };
    let slot = make_slot("SRC", Some("world"));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::JavaNotFound)));
    assert!(converter.resolve_java_path_called.get());
    assert!(
        !converter.is_installed_called.get(),
        "jarNotInstalled must never be checked once javaNotFound already fired"
    );
}

// ---------------------------------------------------------------------
// guard-empty-new-slot-name-rejected-before-any-file-work
// ---------------------------------------------------------------------

#[test]
fn world_conversion_guard_empty_new_slot_name_rejected_before_any_file_work() {
    // No source zip exists at all — if the name guard didn't run first,
    // this would fail with NoSourceZip instead, proving the ordering.
    let source = TempDir::new("empty-name-source");
    let target = TempDir::new("empty-name-target");
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Ok(vec![]));
    let slot = make_slot("SRC", Some("world"));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        None,
        ConversionPlacement::NewSlot {
            name: "   ".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::EmptyName)));
    assert!(converter.invocation.lock().unwrap().is_none());
}

// ---------------------------------------------------------------------
// guard-missing-source-slot-archive-aborts-before-temp-dir
// ---------------------------------------------------------------------

#[test]
fn world_conversion_guard_missing_source_archive_aborts_before_temp_dir() {
    let source = TempDir::new("missing-archive-source");
    let target = TempDir::new("missing-archive-target");
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Ok(vec![]));
    let slot = make_slot("SRC", Some("world"));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::NoSourceZip)));
    assert!(converter.invocation.lock().unwrap().is_none());
}

// ---------------------------------------------------------------------
// nested-world-discovery-bedrock-worlds-subfolder-name-match-else-first-subdir
// ---------------------------------------------------------------------

#[test]
fn world_conversion_nested_world_discovery_bedrock_name_match() {
    let source = TempDir::new("bedrock-discovery-1-source");
    let target = TempDir::new("bedrock-discovery-1-target");
    write_source_slot_zip(
        source.path(),
        "SRC",
        &[
            ("worlds/Bedrock level/level.dat", b"a"),
            ("worlds/some-other-world/level.dat", b"b"),
        ],
    );
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Err("halt".to_string()));
    let slot = make_slot("SRC", Some("Bedrock level"));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Bedrock,
        target.path(),
        ServerType::Bedrock,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::ConversionFailed(msg)) if msg == "halt"));
    let invocation = converter.invocation();
    assert_eq!(invocation.input_dir.file_name().unwrap(), "Bedrock level");
    assert_eq!(
        invocation.input_dir.parent().unwrap().file_name().unwrap(),
        "worlds"
    );
}

#[test]
fn world_conversion_nested_world_discovery_bedrock_falls_back_to_first_subdir() {
    let source = TempDir::new("bedrock-discovery-2-source");
    let target = TempDir::new("bedrock-discovery-2-target");
    write_source_slot_zip(
        source.path(),
        "SRC",
        &[("worlds/only-world/level.dat", b"a")],
    );
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Err("halt".to_string()));
    let slot = make_slot("SRC", None);
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Bedrock,
        target.path(),
        ServerType::Bedrock,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::ConversionFailed(_))));
    assert_eq!(
        converter.invocation().input_dir.file_name().unwrap(),
        "only-world"
    );
}

// ---------------------------------------------------------------------
// nested-world-discovery-java-slot-level-name-match-preferred-else-alphabetical-non-dimension-subdir
// ---------------------------------------------------------------------

#[test]
fn world_conversion_nested_world_discovery_java_level_name_match() {
    let source = TempDir::new("java-discovery-1-source");
    let target = TempDir::new("java-discovery-1-target");
    write_source_slot_zip(
        source.path(),
        "SRC",
        &[
            ("world/level.dat", b"a"),
            ("world_nether/level.dat", b"b"),
            ("world_the_end/level.dat", b"c"),
        ],
    );
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Err("halt".to_string()));
    let slot = make_slot("SRC", Some("world"));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::ConversionFailed(_))));
    assert_eq!(
        converter.invocation().input_dir.file_name().unwrap(),
        "world"
    );
}

#[test]
fn world_conversion_nested_world_discovery_java_alphabetical_fallback() {
    let source = TempDir::new("java-discovery-2-source");
    let target = TempDir::new("java-discovery-2-target");
    write_source_slot_zip(
        source.path(),
        "SRC",
        &[
            ("adventure/level.dat", b"a"),
            ("adventure_nether/level.dat", b"b"),
            ("adventure_the_end/level.dat", b"c"),
            ("__MACOSX/adventure/level.dat", b"d"),
            (".DS_Store", b"e"),
        ],
    );
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Err("halt".to_string()));
    let slot = make_slot("SRC", None);
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        None,
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::ConversionFailed(_))));
    assert_eq!(
        converter.invocation().input_dir.file_name().unwrap(),
        "adventure"
    );
}

// ---------------------------------------------------------------------
// output-packaging-java-vs-bedrock-zip-structure-and-empty-output-refused
// ---------------------------------------------------------------------

fn minimal_source(source_dir: &Path) -> WorldSlot {
    write_source_slot_zip(source_dir, "SRC", &[("world/level.dat", b"seed")]);
    make_slot("SRC", Some("world"))
}

#[test]
fn world_conversion_output_packaging_java_target() {
    let source = TempDir::new("packaging-java-source");
    let target = TempDir::new("packaging-java-target");
    let slot = minimal_source(source.path());
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Ok(vec![
        ("level.dat", b"x".as_slice()),
        ("region", b"".as_slice()),
    ]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    let activated = result.expect("conversion should succeed");
    let zip_path = world_store::zip_path(target.path(), &activated.id);
    let names = archive::list_entry_names(&zip_path).unwrap();
    assert!(
        names.iter().any(|n| n.starts_with("converted-world/")),
        "expected a converted-world/ top-level entry, got {names:?}"
    );
    assert!(!names.iter().any(|n| n.starts_with("worlds/")));
}

#[test]
fn world_conversion_output_packaging_bedrock_target() {
    let source = TempDir::new("packaging-bedrock-source");
    let target = TempDir::new("packaging-bedrock-target");
    let slot = minimal_source(source.path());
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Ok(vec![
        ("level.dat", b"x".as_slice()),
        ("levelname.txt", b"Converted world".as_slice()),
    ]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Bedrock,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    let activated = result.expect("conversion should succeed");
    let zip_path = world_store::zip_path(target.path(), &activated.id);
    let names = archive::list_entry_names(&zip_path).unwrap();
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("worlds/converted-world/")),
        "expected a worlds/converted-world/ top-level entry, got {names:?}"
    );
}

#[test]
fn world_conversion_output_packaging_empty_output_dir_refused() {
    let source = TempDir::new("packaging-empty-source");
    let target = TempDir::new("packaging-empty-target");
    let slot = minimal_source(source.path());
    let fs = StdFileSystem;
    // FakeWorldConverter still creates output_dir (Chunker's own CLI is
    // trusted to do that), but writes nothing into it.
    let converter = FakeWorldConverter::ready(Ok(vec![]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::WorldFolderNotFound)));
}

// ---------------------------------------------------------------------
// chunker-cli-arguments-and-nonzero-exit-fails-conversion
// ---------------------------------------------------------------------

#[test]
fn world_conversion_chunker_nonzero_exit_fails_conversion() {
    let source = TempDir::new("chunker-exit-source");
    let target = TempDir::new("chunker-exit-target");
    let slot = minimal_source(source.path());
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Err("Chunker exited with code 1".to_string()));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    match result {
        Err(ConversionError::ConversionFailed(msg)) => {
            assert_eq!(msg, "Chunker exited with code 1")
        }
        other => panic!("expected ConversionFailed, got {other:?}"),
    }

    let invocation = converter.invocation();
    assert_eq!(invocation.target_format, JAVA_FORMAT);
    assert_eq!(invocation.java_path, "/usr/bin/java");
    assert_eq!(invocation.input_dir.file_name().unwrap(), "world");
    assert_eq!(invocation.output_dir.file_name().unwrap(), "chunker_output");
}

// ---------------------------------------------------------------------
// pre-conversion-backup-failure-only-warns-while-activation-failure-aborts-after-slot-already-written
// ---------------------------------------------------------------------

#[test]
fn world_conversion_pre_conversion_backup_failure_only_warns() {
    let source = TempDir::new("backup-warn-source");
    let target = TempDir::new("backup-warn-target");
    let slot = minimal_source(source.path());
    let fs = StdFileSystem;
    let converter = FakeWorldConverter::ready(Ok(vec![("level.dat", b"x".as_slice())]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        false, // pre-conversion backup fails
        &mut log,
    );

    assert!(result.is_ok(), "a failed backup must not abort conversion");
    assert!(
        log.iter()
            .any(|line| line.contains("Warning: pre-conversion backup failed")),
        "expected a warning line, got {log:?}"
    );
}

#[test]
fn world_conversion_activation_failure_leaves_slot_written_but_inactive() {
    let source = TempDir::new("activation-fail-source");
    let target = TempDir::new("activation-fail-target");
    let slot = minimal_source(source.path());
    let std_fs = StdFileSystem;

    // Fail exactly the write activate_slot makes first — its own
    // transaction manifest — so nothing about the new slot itself
    // (already written in step 5, before activation starts) is touched
    // by the injected failure.
    let manifest_path = target
        .path()
        .join("world_slots")
        .join(".activation")
        .join("manifest.json");
    let fs = FailWriteAt {
        inner: StdFileSystem,
        fail_path: manifest_path,
    };
    let converter = FakeWorldConverter::ready(Ok(vec![("level.dat", b"x".as_slice())]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::NewSlot {
            name: "Converted world".to_string(),
        },
        true,
        &mut log,
    );

    match result {
        Err(ConversionError::ConversionFailed(msg)) => {
            assert_eq!(msg, "Failed to activate converted world slot.")
        }
        other => panic!("expected ConversionFailed, got {other:?}"),
    }

    // The slot placed in step 5 is still on disk, unactivated.
    let slots = world_store::load_slots(&std_fs, target.path());
    assert_eq!(slots.len(), 1, "the placed slot must remain on disk");
    assert_eq!(slots[0].name, "Converted world");
    let active = world_store::load_explicit_active_slot_id(&std_fs, target.path());
    assert_ne!(
        active.as_deref(),
        Some(slots[0].id.as_str()),
        "activation must not have committed"
    );
}

// ---------------------------------------------------------------------
// replace-existing-slot-overwrite-is-not-atomic-unlike-other-slot-mutations
// ---------------------------------------------------------------------

#[test]
fn world_conversion_replace_existing_slot_overwrite_is_not_atomic() {
    let source = TempDir::new("replace-not-atomic-source");
    let target = TempDir::new("replace-not-atomic-target");
    let slot = minimal_source(source.path());

    let existing = make_slot("EXISTING", Some("old-world"));
    write_zip(
        &world_store::zip_path(target.path(), &existing.id),
        &[("old-world/level.dat", b"previous")],
    );
    let dest_zip = world_store::zip_path(target.path(), &existing.id);
    assert!(dest_zip.exists(), "test setup: previous archive must exist");

    let fs = FailWriteAt {
        inner: StdFileSystem,
        fail_path: dest_zip.clone(),
    };
    let converter = FakeWorldConverter::ready(Ok(vec![("level.dat", b"x".as_slice())]));
    let mut log = Vec::new();

    let result = run(
        &fs,
        &converter,
        source.path(),
        &slot,
        ServerType::Java,
        target.path(),
        ServerType::Java,
        Some("converted-world"),
        ConversionPlacement::ReplaceExisting {
            slot: existing.clone(),
        },
        true,
        &mut log,
    );

    assert!(matches!(result, Err(ConversionError::Io(_))));
    assert!(
        !dest_zip.exists(),
        "the previous archive must already be gone, and the new one never written"
    );
}
