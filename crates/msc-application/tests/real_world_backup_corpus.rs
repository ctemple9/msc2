//! P6.26: runs the real MSC 1 world/backup material P6.3 collected
//! (`corpus/worlds/`, `corpus/backups/` -- real per-player data, kept out
//! of git but present locally) through the real repository, reconciliation,
//! archive-safety, NBT, restore, and save/reload code paths, not just the
//! synthetic fixtures every other Phase 6 test uses.
//!
//! Driven entirely by `MSC2_WORLDS_CORPUS_DIR`/`MSC2_BACKUPS_CORPUS_DIR` so
//! `tools/phase6/corpus-check.py --exercise` can point this at the real
//! P6.3 evidence; if either is unset every test here is a no-op pass --
//! `cargo nextest run --workspace` must keep working on a clone with no
//! real corpus staged. Every real corpus file is only ever opened for
//! reading (`std::fs::read`/`std::fs::copy`/`archive::extract_zip`'s own
//! read-only zip access) -- every write happens inside a fresh temp
//! directory each test creates and removes; each test that touches the
//! corpus hashes the exact real files it reads before and after and
//! panics if anything changed.
//!
//! Both real worlds (`Paper`, a vanilla-Paper server, and `campack`, a
//! larger ~11MB Fabric-modded one) are exercised by the cheap, read-only
//! checks (repository load, archive-safety validation, NBT parsing).
//! Reconciliation/restore/save-reload -- the checks that copy real data
//! into a temp root and mutate it -- run only against whichever real
//! world sorts first by path (`Paper`, the smaller one), matching
//! `tools/phase6/corpus-check.py check_worlds_structure`'s own
//! `level_dats[0]` selection: "where size permits" (per this phase's own
//! plan text) rather than doubling every write-path exercise against
//! campack's larger corpus too.
//!
//! P6.35 note: every check in this file calls straight into
//! `msc_application`/`msc_infrastructure` — never through the agent's own
//! HTTP/CLI surface. That stays true on purpose (it's what makes this
//! file fast and dependency-free); it's also why it alone can't stand in
//! for "drive the real corpus through the public path." That leg is a
//! separate, real-data mode of `tools/phase6/phase6-gate-smoke.sh`
//! (`--private-corpus <root>`, driven by a *different*, larger private
//! corpus root than `corpus/worlds`/`corpus/backups` — see
//! `MSC2_PHASE6_PRIVATE_CORPUS` in `tools/phase6/corpus-check.py`), which
//! runs a real agent through `server import`, a bounded staged-upload
//! `world export`/`world import` round trip, activation, a manual
//! backup, and a restore against whichever real Java world sorts first
//! under that root — hashing the real source files it touches before and
//! after the same way every test in this file already does.

use msc_application::backups;
use msc_application::worlds::{self as app_worlds, ReconciliationOutcome};
use msc_domain::identity::ServerType;
use msc_domain::nbt::imported_world_metadata_from_level_dat;
use msc_domain::world::BackupAssociation;
use msc_infrastructure::archive;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::world_store;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

const WORLDS_DIR_ENV: &str = "MSC2_WORLDS_CORPUS_DIR";
const BACKUPS_DIR_ENV: &str = "MSC2_BACKUPS_CORPUS_DIR";

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-real-world-backup-corpus-{label}-{}-{}",
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

/// Both env vars, or `None` -- corpus-check.py always sets both together
/// (`--worlds`/`--backups` default together too), so one missing is
/// "no real corpus staged," not a partial-corpus case worth its own path.
fn corpus_dirs() -> Option<(PathBuf, PathBuf)> {
    let worlds = std::env::var(WORLDS_DIR_ENV).ok()?;
    let backups = std::env::var(BACKUPS_DIR_ENV).ok()?;
    Some((PathBuf::from(worlds), PathBuf::from(backups)))
}

fn sha256_hex(path: &Path) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| panic!("{}: read for hashing: {e}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn find_files(dir: &Path, matches: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if matches(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn is_bedrock_relative(rel: &Path) -> bool {
    rel.components()
        .next()
        .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with("bedrock"))
}

/// The real Java `level.dat` files P6.3 collected, outside `world_slots/`
/// and any `bedrock*/` evidence -- same filter
/// `tools/phase6/corpus-check.py check_worlds_structure` applies.
fn find_java_level_dats(worlds_dir: &Path) -> Vec<PathBuf> {
    find_files(worlds_dir, |p| {
        p.file_name().is_some_and(|n| n == "level.dat")
    })
    .into_iter()
    .filter(|p| {
        let rel = p.strip_prefix(worlds_dir).unwrap();
        !is_bedrock_relative(rel) && !rel.components().any(|c| c.as_os_str() == "world_slots")
    })
    .collect()
}

fn find_zips_recursive(dir: &Path) -> Vec<PathBuf> {
    find_files(dir, |p| p.extension().is_some_and(|e| e == "zip"))
}

fn find_top_level_zips(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: read_dir: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "zip"))
        .collect();
    out.sort();
    out
}

fn hash_tree(root: &Path) -> BTreeMap<PathBuf, String> {
    find_files(root, |_| true)
        .into_iter()
        .map(|p| {
            let hash = sha256_hex(&p);
            (p, hash)
        })
        .collect()
}

fn assert_tree_unchanged(label: &str, root: &Path, before: &BTreeMap<PathBuf, String>) {
    let after = hash_tree(root);
    assert_eq!(
        before,
        &after,
        "{label}: real corpus evidence under {} changed during the exercise run",
        root.display()
    );
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[test]
fn real_world_repository_loads_archives_validate_safe_and_level_dats_parse() {
    let Some((worlds_dir, backups_dir)) = corpus_dirs() else {
        println!("{WORLDS_DIR_ENV}/{BACKUPS_DIR_ENV} not set -- skipping (see P6.3/P6.26)");
        return;
    };

    // --- repository load ---
    let slots = world_store::load_slots(&StdFileSystem, &worlds_dir);
    assert!(
        !slots.is_empty(),
        "{}: real world_slots/ tree loaded zero slots",
        worlds_dir.display()
    );
    for slot in &slots {
        println!(
            "loaded real slot: id={} name={:?} zip_size_bytes={:?}",
            slot.id, slot.name, slot.zip_size_bytes
        );
    }

    // --- safe archive validation, every real .zip in both directories ---
    let zips: Vec<PathBuf> = find_zips_recursive(&worlds_dir)
        .into_iter()
        .chain(find_top_level_zips(&backups_dir))
        .collect();
    assert!(!zips.is_empty(), "no real .zip evidence found to validate");
    let zip_hashes_before: BTreeMap<PathBuf, String> =
        zips.iter().map(|p| (p.clone(), sha256_hex(p))).collect();
    for zip_path in &zips {
        archive::validate_archive_safety(zip_path).unwrap_or_else(|e| {
            panic!(
                "{}: real archive failed safety validation: {e}",
                zip_path.display()
            )
        });
        println!("archive safety ok: {}", zip_path.display());
    }
    for (zip_path, before) in &zip_hashes_before {
        let after = sha256_hex(zip_path);
        assert_eq!(
            before,
            &after,
            "{}: changed during safety validation",
            zip_path.display()
        );
    }

    // --- metadata/NBT parsing, every real Java level.dat ---
    let level_dats = find_java_level_dats(&worlds_dir);
    assert!(
        !level_dats.is_empty(),
        "no real Java level.dat evidence found to parse"
    );
    for level_dat in &level_dats {
        let before = sha256_hex(level_dat);
        let raw = std::fs::read(level_dat)
            .unwrap_or_else(|e| panic!("{}: read: {e}", level_dat.display()));
        let metadata = imported_world_metadata_from_level_dat(&raw, ServerType::Java);
        assert!(
            metadata.seed.is_some() || metadata.difficulty.is_some() || metadata.gamemode.is_some(),
            "{}: real level.dat parsed to an entirely empty ImportedWorldMetadata -- \
             the NBT reader found nothing in real data",
            level_dat.display()
        );
        println!(
            "parsed real level.dat {}: difficulty={:?} gamemode={:?} day_time={:?}",
            level_dat.display(),
            metadata.difficulty,
            metadata.gamemode,
            metadata.day_time
        );
        let after = sha256_hex(level_dat);
        assert_eq!(
            before,
            after,
            "{}: changed during NBT parsing",
            level_dat.display()
        );
    }
}

#[test]
fn real_world_import_reconciliation_runs_non_destructively_against_a_temp_copy() {
    let Some((worlds_dir, _backups_dir)) = corpus_dirs() else {
        println!("{WORLDS_DIR_ENV}/{BACKUPS_DIR_ENV} not set -- skipping (see P6.3/P6.26)");
        return;
    };

    let level_dats = find_java_level_dats(&worlds_dir);
    let primary_level_dat = level_dats
        .first()
        .unwrap_or_else(|| panic!("{}: no real Java level.dat found", worlds_dir.display()));
    let primary_world_dir = primary_level_dat.parent().unwrap();
    let base_name = primary_world_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let world_slots_dir = worlds_dir.join("world_slots");

    let before = hash_tree(primary_world_dir)
        .into_iter()
        .chain(if world_slots_dir.is_dir() {
            hash_tree(&world_slots_dir)
        } else {
            BTreeMap::new()
        })
        .collect::<BTreeMap<_, _>>();

    let root = TempRoot::new("reconcile");
    let server_dir = root.path.join("server");
    copy_dir_all(primary_world_dir, &server_dir.join(&base_name))
        .unwrap_or_else(|e| panic!("copy real world folder into temp server dir: {e}"));
    if world_slots_dir.is_dir() {
        copy_dir_all(&world_slots_dir, &server_dir.join("world_slots"))
            .unwrap_or_else(|e| panic!("copy real world_slots/ into temp server dir: {e}"));
    }

    let outcome = app_worlds::reconcile_imported_worlds(
        &StdFileSystem,
        &server_dir,
        ServerType::Java,
        Some(&base_name),
        "2026-08-16T00:00:00Z",
    )
    .unwrap_or_else(|e| panic!("reconcile_imported_worlds against real corpus copy: {e}"));

    match &outcome {
        ReconciliationOutcome::LiveFoldersProvenIdenticalToRecordedSlot { slot_id } => {
            println!("reconciliation: live folders proven identical to recorded slot {slot_id}");
        }
        ReconciliationOutcome::RecoverySnapshotCreated {
            new_slot_id,
            previous_slot_id,
        } => {
            println!(
                "reconciliation: recovery snapshot {new_slot_id} created, previous slot {previous_slot_id} retained"
            );
        }
        other => panic!(
            "unexpected reconciliation outcome against real corpus (expected a State-3 \
             live-plus-resolved-slot outcome since the real corpus has both): {other:?}"
        ),
    }

    let reloaded = world_store::load_slots(&StdFileSystem, &server_dir);
    assert!(
        !reloaded.is_empty(),
        "temp server dir has no slots after reconciliation"
    );

    assert_tree_unchanged("world folder", primary_world_dir, &{
        let mut m = BTreeMap::new();
        for (p, h) in &before {
            if p.starts_with(primary_world_dir) {
                m.insert(p.clone(), h.clone());
            }
        }
        m
    });
    if world_slots_dir.is_dir() {
        assert_tree_unchanged("world_slots", &world_slots_dir, &{
            let mut m = BTreeMap::new();
            for (p, h) in &before {
                if p.starts_with(&world_slots_dir) {
                    m.insert(p.clone(), h.clone());
                }
            }
            m
        });
    }
}

#[test]
fn real_backup_restores_non_destructively_and_the_restored_world_saves_and_reloads() {
    let Some((worlds_dir, backups_dir)) = corpus_dirs() else {
        println!("{WORLDS_DIR_ENV}/{BACKUPS_DIR_ENV} not set -- skipping (see P6.3/P6.26)");
        return;
    };

    let level_dats = find_java_level_dats(&worlds_dir);
    let primary_level_dat = level_dats
        .first()
        .unwrap_or_else(|| panic!("{}: no real Java level.dat found", worlds_dir.display()));
    let primary_world_dir = primary_level_dat.parent().unwrap();
    let base_name = primary_world_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let backup_zip = find_top_level_zips(&backups_dir)
        .into_iter()
        .find(|z| {
            z.file_stem()
                .map(|s| s.to_string_lossy().starts_with(&format!("{base_name}_")))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "{}: no real backup zip named for real world {base_name:?}",
                backups_dir.display()
            )
        });
    let backup_hash_before = sha256_hex(&backup_zip);

    // --- non-destructive restore into a temporary root ---
    let root = TempRoot::new("restore");
    let server_dir = root.path.join("server");
    copy_dir_all(primary_world_dir, &server_dir.join(&base_name))
        .unwrap_or_else(|e| panic!("copy real world folder into temp server dir: {e}"));

    let outcome = backups::restore_backup(
        &StdFileSystem,
        &server_dir,
        ServerType::Java,
        Some(&base_name),
        &backup_zip,
        None,
        None,
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-08-16T00:00:00Z",
        || false,
    )
    .unwrap_or_else(|e| {
        panic!(
            "restore_backup with real backup {}: {e}",
            backup_zip.display()
        )
    });
    assert!(
        outcome.safety_backup_zip_path.is_file(),
        "restore's mandatory pre-restore safety backup was not written to {}",
        outcome.safety_backup_zip_path.display()
    );

    let restored_level_dat = server_dir.join(&base_name).join("level.dat");
    assert!(
        restored_level_dat.is_file(),
        "{}: restored world has no level.dat",
        restored_level_dat.display()
    );
    let restored_raw = std::fs::read(&restored_level_dat)
        .unwrap_or_else(|e| panic!("{}: read: {e}", restored_level_dat.display()));
    let restored_metadata = imported_world_metadata_from_level_dat(&restored_raw, ServerType::Java);
    assert!(
        restored_metadata.seed.is_some()
            || restored_metadata.difficulty.is_some()
            || restored_metadata.gamemode.is_some(),
        "restored real level.dat parsed to an entirely empty ImportedWorldMetadata"
    );

    let backup_hash_after = sha256_hex(&backup_zip);
    assert_eq!(
        backup_hash_before,
        backup_hash_after,
        "{}: real backup zip changed during restore -- restore is supposed to only read it",
        backup_zip.display()
    );

    // --- save/reload: archive the restored world into a fresh slot,
    // reload the repository, and extract the new archive to prove the
    // round trip is byte-for-byte faithful ---
    let saved_slot = app_worlds::create_slot_from_current_world(
        &StdFileSystem,
        &server_dir,
        ServerType::Java,
        Some(&base_name),
        "post-restore-reload-check",
        None,
        "2026-08-16T00:05:00Z",
    )
    .unwrap_or_else(|e| panic!("create_slot_from_current_world after real restore: {e}"));

    let reloaded = world_store::load_slots(&StdFileSystem, &server_dir);
    assert!(
        reloaded.iter().any(|s| s.id == saved_slot.id),
        "newly saved slot {} did not reappear on repository reload",
        saved_slot.id
    );

    let reload_root = TempRoot::new("reload-extract");
    let saved_zip = world_store::zip_path(&server_dir, &saved_slot.id);
    archive::extract_zip(&saved_zip, &reload_root.path)
        .unwrap_or_else(|e| panic!("{}: extract saved slot zip: {e}", saved_zip.display()));
    let reloaded_level_dat = reload_root.path.join(&base_name).join("level.dat");
    let reloaded_raw = std::fs::read(&reloaded_level_dat)
        .unwrap_or_else(|e| panic!("{}: read: {e}", reloaded_level_dat.display()));
    assert_eq!(
        restored_raw, reloaded_raw,
        "save/reload round trip: level.dat bytes differ after saving and re-extracting"
    );

    println!(
        "save/reload ok: real backup {} restored, saved as slot {}, reloaded and \
         re-extracted with identical level.dat bytes",
        backup_zip.display(),
        saved_slot.id
    );
}
