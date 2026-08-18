//! Port of 12 of `fixtures/server-creation/`'s 24 cases (P7.6) against
//! `msc_domain::provisioning` (P7.12) -- the pure decisions
//! `createNewServer` makes before touching a disk. The other 12 cases are
//! filesystem operations, deferred to P7.17/P7.18's application service;
//! see `provisioning.rs`'s own module doc for the full scope call.
//! `fixtures/jar-templates/`'s 10 cases are entirely template-directory
//! I/O and are P7.15's job, not ported here at all.

mod support;

use msc_domain::identity::JavaServerFlavor;
use msc_domain::provisioning::*;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("server-creation/{case}.json")))
}

fn flavor_of(name: &str) -> JavaServerFlavor {
    JavaServerFlavor::from_raw_value(name).unwrap_or_else(|| panic!("unknown flavor {name}"))
}

#[test]
fn provisioning_addon_folder_mods_for_modded_flavor() {
    let fixture = load("addon-folder-mods-for-modded-flavor");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    assert_eq!(add_on_folder_name(flavor), Some("mods"));
    assert_eq!(fixture.expected["addOnKind"].as_str(), Some("mod"));
}

#[test]
fn provisioning_addon_folder_none_for_vanilla() {
    let fixture = load("addon-folder-none-for-vanilla");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    assert_eq!(add_on_folder_name(flavor), None);
    assert!(fixture.expected["addOnKind"].is_null());
}

#[test]
fn provisioning_addon_folder_plugins_for_plugin_flavor() {
    let fixture = load("addon-folder-plugins-for-plugin-flavor");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    assert_eq!(add_on_folder_name(flavor), Some("plugins"));
    assert_eq!(fixture.expected["addOnKind"].as_str(), Some("plugin"));
}

#[test]
fn provisioning_configserver_ram_default_plugin_2_4gb_vs_modded_3_6gb() {
    let fixture = load("configserver-ram-default-plugin-2-4gb-vs-modded-3-6gb");
    let paper = flavor_of(fixture.input["pluginFlavor"].as_str().unwrap());
    let fabric = flavor_of(fixture.input["moddedFlavor"].as_str().unwrap());

    assert_eq!(default_ram_gb(paper), (2.0, 4.0));
    assert_eq!(default_ram_gb(fabric), (3.0, 6.0));

    assert_eq!(fixture.expected["paper"]["minRamGB"].as_f64(), Some(2.0));
    assert_eq!(fixture.expected["paper"]["maxRamGB"].as_f64(), Some(4.0));
    assert_eq!(fixture.expected["fabric"]["minRamGB"].as_f64(), Some(3.0));
    assert_eq!(fixture.expected["fabric"]["maxRamGB"].as_f64(), Some(6.0));
}

#[test]
fn provisioning_folder_name_lowercased_and_spaces_to_underscores() {
    let fixture = load("folder-name-lowercased-and-spaces-to-underscores");
    let safe_name = fixture.input["safeName"].as_str().unwrap();
    assert_eq!(
        folder_name_from_safe_name(safe_name),
        fixture.expected["folderName"].as_str().unwrap()
    );
}

#[test]
fn provisioning_name_trimmed_before_use() {
    let fixture = load("name-trimmed-before-use");
    let name = fixture.input["name"].as_str().unwrap();
    let safe_name = trimmed_server_name(name).expect("non-empty after trim");
    assert_eq!(safe_name, fixture.expected["safeName"].as_str().unwrap());
    assert_eq!(
        folder_name_from_safe_name(&safe_name),
        fixture.expected["folderName"].as_str().unwrap()
    );
}

#[test]
fn provisioning_empty_name_after_trim_refused_no_directory_created() {
    let fixture = load("empty-name-after-trim-refused-no-directory-created");
    let name = fixture.input["name"].as_str().unwrap();
    assert_eq!(trimmed_server_name(name), None);
    assert_eq!(fixture.expected["returns"].as_bool(), Some(false));
}

#[test]
fn provisioning_server_properties_exact_key_set_fresh_world() {
    let fixture = load("server-properties-exact-key-set-fresh-world");
    let props = fresh_server_properties(
        fixture.input["port"].as_u64().unwrap() as u16,
        fixture.input["safeName"].as_str().unwrap(),
        fixture.input["difficulty"].as_str().unwrap(),
        fixture.input["gamemode"].as_str().unwrap(),
        fixture.input["safeName"].as_str().unwrap(),
        None,
    );

    let expected_keys: Vec<String> = fixture.expected["propertiesFileContainsExactlyKeys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let mut actual_keys: Vec<String> = props.keys().cloned().collect();
    actual_keys.sort();
    let mut expected_sorted = expected_keys.clone();
    expected_sorted.sort();
    assert_eq!(actual_keys, expected_sorted);

    for (key, expected_value) in fixture.expected["values"].as_object().unwrap() {
        assert_eq!(props.get(key).map(String::as_str), expected_value.as_str());
    }
    assert!(!props.contains_key("level-seed"));
}

#[test]
fn provisioning_imported_metadata_overrides_difficulty_gamemode_seed() {
    let fixture = load("imported-metadata-overrides-difficulty-gamemode-seed");
    let imported_json = &fixture.input["importedMetadata"];
    let imported = ImportedWorldMetadata {
        difficulty: imported_json["difficulty"].as_str().map(str::to_string),
        gamemode: imported_json["gamemode"].as_str().map(str::to_string),
        seed: imported_json["seed"].as_str().map(str::to_string),
    };
    let effective = effective_world_settings(
        fixture.input["wizardDifficulty"].as_str().unwrap(),
        fixture.input["wizardGamemode"].as_str().unwrap(),
        fixture.input["wizardWorldSeed"].as_str(),
        &imported,
    );
    assert_eq!(
        effective.difficulty,
        fixture.expected["effectiveDifficulty"].as_str().unwrap()
    );
    assert_eq!(
        effective.gamemode,
        fixture.expected["effectiveGamemode"].as_str().unwrap()
    );
    assert_eq!(
        effective.world_seed.as_deref(),
        fixture.expected["effectiveWorldSeed"].as_str()
    );
}

#[test]
fn provisioning_record_loader_version_called_only_for_modded_category() {
    let fixture = load("record-loader-version-called-only-for-modded-category");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    let called = should_record_loader_version(
        flavor,
        fixture.input["resolvedVersion"].as_str(),
        fixture.input["resolvedLoader"].as_str(),
    );
    assert_eq!(
        called,
        fixture.expected["recordLoaderVersionCalled"]
            .as_bool()
            .unwrap()
    );

    // Standard-category flavors never call it, even with both values present.
    assert!(!should_record_loader_version(
        JavaServerFlavor::Paper,
        Some("1.21.4"),
        Some("ignored")
    ));
    // Modded with no resolved loader version skips it too.
    assert!(!should_record_loader_version(
        JavaServerFlavor::NeoForge,
        Some("1.21.4"),
        None
    ));
}

#[test]
fn provisioning_archive_shortcut_skipped_when_save_downloaded_jars_disabled_or_non_paper() {
    let fixture = load("archive-shortcut-skipped-when-save-downloaded-jars-disabled-or-non-paper");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    let save_downloaded_jars = fixture.input["saveDownloadedJars"].as_bool().unwrap();
    assert!(!should_use_archive_first_shortcut(
        flavor,
        save_downloaded_jars
    ));

    // The sibling half this fixture's own notes say is implied: Paper with
    // the setting off also skips the shortcut.
    assert!(!should_use_archive_first_shortcut(
        JavaServerFlavor::Paper,
        false
    ));
    // Only Paper + the setting on takes the shortcut.
    assert!(should_use_archive_first_shortcut(
        JavaServerFlavor::Paper,
        true
    ));
}

#[test]
fn provisioning_configserver_field_set_newly_created_server() {
    let fixture = load("configserver-field-set-newly-created-server");
    let flavor = flavor_of(fixture.input["flavor"].as_str().unwrap());
    let (min_ram, max_ram) = default_ram_gb(flavor);

    let fields = new_server_config_fields(
        fixture.input["safeName"].as_str().unwrap(),
        "<newDir>.path",
        fixture.input["primaryJarPath"].as_str().unwrap(),
        min_ram,
        max_ram,
        flavor,
        fixture.input["resolvedVersion"].as_str(),
        fixture.input["resolvedBuild"].as_str(),
        fixture.input["resolvedLoader"].as_str(),
        fixture.input["defaultBannerColorHex"].as_str().unwrap(),
        fixture.input["enablePlayit"].as_bool().unwrap(),
        fixture.input["enableXboxBroadcast"].as_bool().unwrap(),
        None,
    );

    assert!(!fields.id.is_empty());
    assert_eq!(
        fields.display_name,
        fixture.expected["displayName"].as_str().unwrap()
    );
    assert_eq!(
        fields.server_dir,
        fixture.expected["serverDir"].as_str().unwrap()
    );
    assert_eq!(
        fields.paper_jar_path,
        fixture.expected["paperJarPath"].as_str().unwrap()
    );
    assert_eq!(
        fields.min_ram_gb,
        fixture.expected["minRamGB"].as_f64().unwrap()
    );
    assert_eq!(
        fields.max_ram_gb,
        fixture.expected["maxRamGB"].as_f64().unwrap()
    );
    assert_eq!(fields.notes, fixture.expected["notes"].as_str().unwrap());
    assert_eq!(
        fields.java_flavor.raw_value(),
        fixture.expected["javaFlavor"].as_str().unwrap()
    );
    assert_eq!(
        fields.minecraft_version.as_deref(),
        fixture.expected["minecraftVersion"].as_str()
    );
    assert_eq!(
        fields.server_build.as_deref(),
        fixture.expected["serverBuild"].as_str()
    );
    assert_eq!(
        fields.loader_version.as_deref(),
        fixture.expected["loaderVersion"].as_str()
    );
    assert_eq!(
        fields.banner_color_hex,
        fixture.expected["bannerColorHex"].as_str().unwrap()
    );
    assert_eq!(
        fields.playit_enabled,
        fixture.expected["playitEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        fields.xbox_broadcast_enabled,
        fixture.expected["xboxBroadcastEnabled"].as_bool().unwrap()
    );
    assert!(fields.bedrock_port.is_none());
    assert!(fixture.expected["bedrockPort"].is_null());
}
