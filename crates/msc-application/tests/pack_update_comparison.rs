//! Essential P24.9: protect owner edits and deleted items across an upstream
//! update. Existing single-file installation/rollback checks have no old-pack
//! baseline and cannot catch those losses. Pure controlled metadata, no I/O or
//! timing/environment assumptions; expected <5 ms per case. Not run by default.
use msc_application::pack_updates;
use msc_domain::content_collection::{ContentAudience, ContentEntry};
use std::collections::BTreeMap;
fn entry(path: &str, byte: char) -> ContentEntry {
    ContentEntry {
        path: path.into(),
        sha512: byte.to_string().repeat(128),
        byte_size: 1,
        audience: ContentAudience::Both,
        optional: false,
        source: None,
        redistribution_evidence: None,
        classification_evidence: Vec::new(),
        unresolved_reasons: Vec::new(),
        extra: BTreeMap::new(),
    }
}
fn fixture() -> (
    BTreeMap<String, ContentEntry>,
    BTreeMap<String, ContentEntry>,
    BTreeMap<String, ContentEntry>,
) {
    let old = BTreeMap::from([
        ("mods/edited.jar".into(), entry("mods/edited.jar", 'a')),
        ("mods/removed.jar".into(), entry("mods/removed.jar", 'a')),
        ("mods/ordinary.jar".into(), entry("mods/ordinary.jar", 'a')),
    ]);
    let mut current = old.clone();
    current.insert("mods/edited.jar".into(), entry("mods/edited.jar", 'b'));
    current.remove("mods/removed.jar");
    let new = old
        .keys()
        .map(|path| (path.clone(), entry(path, 'c')))
        .collect();
    (old, current, new)
}
#[test]
fn upstream_update_preserves_owner_edits_and_removals_but_updates_unmodified_files() {
    let (old, current, new) = fixture();
    let merged = pack_updates::compare(&old, &current, &new, &[], &[]).unwrap();
    assert_eq!(merged.files["mods/edited.jar"].sha512, "b".repeat(128));
    assert!(!merged.files.contains_key("mods/removed.jar"));
    assert!(
        merged
            .excluded_paths
            .contains(&"mods/removed.jar".to_string())
    );
    assert_eq!(merged.files["mods/ordinary.jar"].sha512, "c".repeat(128));
    assert!(
        merged
            .changes
            .iter()
            .any(|change| change.path == "mods/edited.jar" && change.conflict)
    );
}
#[test]
fn explicit_restoration_changes_only_the_chosen_conflicting_path() {
    let (old, current, new) = fixture();
    let merged =
        pack_updates::compare(&old, &current, &new, &[], &["mods/removed.jar".into()]).unwrap();
    assert_eq!(merged.files["mods/edited.jar"].sha512, "b".repeat(128));
    assert_eq!(merged.files["mods/removed.jar"].sha512, "c".repeat(128));
    assert!(
        !merged
            .excluded_paths
            .contains(&"mods/removed.jar".to_string())
    );
}

#[test]
fn renamed_release_does_not_resurrect_removed_mod_or_duplicate_owner_edited_version() {
    use msc_domain::content_collection::ContentSource;
    let mut old_entry = entry("mods/example-old.jar", 'a');
    old_entry.source = Some(ContentSource {
        provider: "curseforge".into(),
        project_id: "1".into(),
        version_id: "10".into(),
    });
    let mut incoming = entry("mods/example-new.jar", 'c');
    incoming.source = Some(ContentSource {
        provider: "curseforge".into(),
        project_id: "1".into(),
        version_id: "11".into(),
    });
    let old = BTreeMap::from([(old_entry.path.clone(), old_entry.clone())]);
    let new = BTreeMap::from([(incoming.path.clone(), incoming)]);
    let removed = pack_updates::compare(&old, &BTreeMap::new(), &new, &[], &[]).unwrap();
    assert!(removed.files.is_empty());
    let mut owner = old_entry;
    owner.sha512 = "b".repeat(128);
    let current = BTreeMap::from([(owner.path.clone(), owner)]);
    let preserved = pack_updates::compare(&old, &current, &new, &[], &[]).unwrap();
    assert!(preserved.files.contains_key("mods/example-old.jar"));
    assert!(!preserved.files.contains_key("mods/example-new.jar"));
    let restored =
        pack_updates::compare(&old, &current, &new, &[], &["mods/example-new.jar".into()]).unwrap();
    assert!(!restored.files.contains_key("mods/example-old.jar"));
    assert!(restored.files.contains_key("mods/example-new.jar"));
}
