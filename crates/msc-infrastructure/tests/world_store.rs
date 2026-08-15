//! Exercises `world_store`'s I/O half of `WorldSlotManager`'s directory
//! layout/load/save against [`FakeFileSystem`] — the domain-level policy
//! this wraps (decode tolerance, sort order, active-slot resolution) is
//! already fixture-tested in `msc-domain`'s `tests/world.rs` (P6.9); these
//! tests prove `world_store` wires that policy to real directory-listing/
//! read/write/atomic-write behavior correctly. Test functions are
//! prefixed `world_store_` so the plan's Verify command
//! (`-E 'test(/world_(store|archive)/)'`, which — per the same nextest
//! gap P5.13/P5.14/P5.16 already hit — matches on test *name*, not
//! binary) selects them.

use msc_domain::identity::ServerType;
use msc_domain::world::build_fresh_slot;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::world_store::{
    load_explicit_active_slot_id, load_slots, save_metadata, save_thumbnail, set_active_slot_id,
    thumbnail_dest_size,
};
use std::path::Path;

#[test]
fn world_store_load_slots_missing_world_slots_directory_returns_empty() {
    let fs = FakeFileSystem::new();
    let slots = load_slots(&fs, Path::new("/servers/paper"));
    assert!(slots.is_empty());
}

#[test]
fn world_store_load_slots_tolerates_corrupt_entries_and_sorts_newest_first() {
    let fs = FakeFileSystem::new()
        .with_file(
            "/servers/paper/world_slots/slot-a/slot.json",
            br#"{"id":"slot-a","name":"A","created_at":"2026-01-01T00:00:00Z"}"#.to_vec(),
            false,
        )
        .with_file(
            "/servers/paper/world_slots/slot-b/slot.json",
            br#"{"id":"slot-b","name":"B","created_at":"2026-03-01T00:00:00Z"}"#.to_vec(),
            false,
        )
        .with_file(
            "/servers/paper/world_slots/slot-corrupt/slot.json",
            b"{not valid json".to_vec(),
            false,
        )
        .with_file(
            "/servers/paper/world_slots/slot-b/world.zip",
            vec![0u8; 42],
            false,
        );

    let slots = load_slots(&fs, Path::new("/servers/paper"));
    let ids: Vec<&str> = slots.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["slot-b", "slot-a"]);
    assert_eq!(slots[0].zip_size_bytes, Some(42));
    assert_eq!(slots[1].zip_size_bytes, None);
}

#[test]
fn world_store_active_marker_round_trips_and_trims() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/servers/paper");

    assert_eq!(load_explicit_active_slot_id(&fs, server_dir), None);

    set_active_slot_id(&fs, server_dir, Some("slot-a")).unwrap();
    assert_eq!(
        load_explicit_active_slot_id(&fs, server_dir).as_deref(),
        Some("slot-a")
    );

    set_active_slot_id(&fs, server_dir, None).unwrap();
    assert_eq!(load_explicit_active_slot_id(&fs, server_dir), None);
    // Removing an already-absent marker is not an error.
    set_active_slot_id(&fs, server_dir, None).unwrap();
}

#[test]
fn world_store_save_metadata_then_load_slots_round_trips() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/servers/paper");
    let slot = build_fresh_slot(
        "slot-fresh".to_string(),
        "My World",
        Some("12345"),
        ServerType::Java,
        "2026-01-01T00:00:00Z".to_string(),
    );

    save_metadata(&fs, server_dir, &slot).unwrap();

    let loaded = load_slots(&fs, server_dir);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "slot-fresh");
    assert_eq!(loaded[0].name, "My World");
    assert_eq!(loaded[0].world_seed.as_deref(), Some("12345"));
    assert_eq!(loaded[0].zip_size_bytes, None);
}

#[test]
fn world_store_thumbnail_dest_size_preserves_aspect_ratio_and_never_upscales() {
    // Wider than the 800x450 box: width is the binding constraint.
    assert_eq!(thumbnail_dest_size(1600.0, 900.0), (800.0, 450.0));
    // Already smaller than the box: source's `min(..., 1.0)` never
    // upscales.
    assert_eq!(thumbnail_dest_size(400.0, 225.0), (400.0, 225.0));
    // Taller aspect ratio: height is the binding constraint.
    assert_eq!(thumbnail_dest_size(900.0, 1800.0), (225.0, 450.0));
}

#[test]
fn world_store_save_thumbnail_writes_file_and_updates_metadata() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/servers/paper");
    let slot = build_fresh_slot(
        "slot-fresh".to_string(),
        "My World",
        None,
        ServerType::Java,
        "2026-01-01T00:00:00Z".to_string(),
    );
    save_metadata(&fs, server_dir, &slot).unwrap();

    let updated = save_thumbnail(&fs, server_dir, &slot, b"fake jpeg bytes").unwrap();
    assert_eq!(
        updated.thumbnail_file_name.as_deref(),
        Some("thumbnail.jpg")
    );

    let bytes = fs
        .read(Path::new(
            "/servers/paper/world_slots/slot-fresh/thumbnail.jpg",
        ))
        .unwrap();
    assert_eq!(bytes, b"fake jpeg bytes");

    let reloaded = load_slots(&fs, server_dir);
    assert_eq!(
        reloaded[0].thumbnail_file_name.as_deref(),
        Some("thumbnail.jpg")
    );
}
