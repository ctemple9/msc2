//! Port of 4 of the 7 fixtures in `fixtures/config-roundtrip/`: the
//! `AppConfig`/`ConfigServer` schema round-trip and missing-fields cases.
//! The remaining 3 (`config-manager-corrupt-config-copy-path-is-nil-on-normal-load`,
//! `r3-corrupt-file-algorithm`, `r3-corrupt-file-does-not-wipe-original`)
//! exercise corruption-recovery composition and belong to P5.6.
//!
//! Fixture `input`/`expected` keys are Swift property names (camelCase) —
//! these fixtures were extracted from `AppConfigRoundTripTests.swift`,
//! which builds an `AppConfig`/`ConfigServer` via Swift's memberwise
//! initializer and asserts on the decoded struct's Swift properties, not
//! raw wire JSON. Each test below sets the equivalent Rust (snake_case)
//! field directly rather than constructing wire JSON by hand.
//!
//! The two `*_missing_optional_fields_get_defaults` tests below are two of
//! P5.23's `fixtures/config-corpus-dimensions/` matrix entries
//! (`missing-fields-default-app-config`, `missing-fields-default-config-
//! server`) — that directory cross-references every configuration
//! dimension the port plan names, this file's coverage included.

mod support;

use msc_domain::app_config_schema::{AppConfig, ConfigServer, RemoteApiSharedAccessEntry};
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("config-roundtrip/{case}.json")))
}

fn str_opt(v: &serde_json::Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_string)
}

/// The fixture stores `xboxBroadcastIPMode` as the Swift *case name*
/// (`publicIP`), not the wire `raw_value()` (`public_ip`) — unlike
/// `serverType`/`javaFlavor`, whose case names and raw values coincide.
fn xbox_ip_mode_case_name(m: msc_domain::app_config_schema::XboxBroadcastIpMode) -> &'static str {
    use msc_domain::app_config_schema::XboxBroadcastIpMode::*;
    match m {
        Auto => "auto",
        PublicIp => "publicIP",
        PrivateIp => "privateIP",
    }
}

fn xbox_ip_mode_from_case_name(s: &str) -> msc_domain::app_config_schema::XboxBroadcastIpMode {
    use msc_domain::app_config_schema::XboxBroadcastIpMode::*;
    match s {
        "auto" => Auto,
        "publicIP" => PublicIp,
        "privateIP" => PrivateIp,
        other => panic!("unknown xboxBroadcastIPMode case name: {other}"),
    }
}

#[test]
fn app_config_schema_app_config_full_round_trip() {
    let fixture = load("app-config-full-round-trip");
    let overrides = &fixture.input["overrides"];

    let mut config = AppConfig::default_config("/tmp/test-servers");
    config.java_path = str_opt(overrides, "javaPath").unwrap();
    config.extra_flags = str_opt(overrides, "extraFlags").unwrap();
    config.servers_root = str_opt(overrides, "serversRoot").unwrap();
    config.plugin_template_dir = str_opt(overrides, "pluginTemplateDir").unwrap();
    config.paper_template_dir = str_opt(overrides, "paperTemplateDir").unwrap();
    config.active_server_id = str_opt(overrides, "activeServerId");
    config.initial_setup_done = overrides["initialSetupDone"].as_bool().unwrap();
    config.remote_api_port = overrides["remoteAPIPort"].as_i64().unwrap();
    config.remote_api_token = str_opt(overrides, "remoteAPIToken").unwrap();
    config.remote_api_expose_on_lan = overrides["remoteAPIExposeOnLAN"].as_bool().unwrap();
    config.remote_api_preferred_pairing_host = str_opt(overrides, "remoteAPIPreferredPairingHost");
    config.duckdns_hostname = str_opt(overrides, "duckdnsHostname");
    config.playit_java_address = str_opt(overrides, "playitJavaAddress");
    config.playit_bedrock_address = str_opt(overrides, "playitBedrockAddress");
    config.playit_voice_address = str_opt(overrides, "playitVoiceAddress");
    config.playit_agent_id = str_opt(overrides, "playitAgentId");
    config.has_shown_handbook = overrides["hasShownHandbook"].as_bool().unwrap();
    config.has_shown_concept_guide = overrides["hasShownConceptGuide"].as_bool().unwrap();
    config.xbox_broadcast_jar_path = str_opt(overrides, "xboxBroadcastJarPath");
    config.xbox_broadcast_auto_start_enabled = overrides["xboxBroadcastAutoStartEnabled"]
        .as_bool()
        .unwrap();
    config.minecraft_username = str_opt(overrides, "minecraftUsername");
    config.minecraft_bedrock_gamertag = str_opt(overrides, "minecraftBedrockGamertag");
    config.minecraft_avatar_edition_raw_value =
        str_opt(overrides, "minecraftAvatarEditionRawValue");
    config.default_banner_color_hex = str_opt(overrides, "defaultBannerColorHex");
    config.error_popups_enabled = overrides["errorPopupsEnabled"].as_bool().unwrap();
    config.save_downloaded_jars = overrides["saveDownloadedJars"].as_bool().unwrap();
    config.use_vm_bedrock_backend = overrides["useVMBedrockBackend"].as_bool().unwrap();

    let entry = &overrides["remoteAPISharedAccess"][0];
    config.remote_api_shared_access = vec![RemoteApiSharedAccessEntry {
        id: str_opt(entry, "id").unwrap(),
        label: str_opt(entry, "label").unwrap(),
        token: str_opt(entry, "token").unwrap(),
        role: str_opt(entry, "role").unwrap(),
        created_at_iso8601: str_opt(entry, "createdAtISO8601"),
        permissions: entry["permissions"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect()),
        expires_at_iso8601: str_opt(entry, "expiresAtISO8601"),
    }];

    let encoded = config.encode();
    assert!(
        encoded.get("remote_api_token").is_none(),
        "remote_api_token must never be written to JSON"
    );

    let defaults = AppConfig::default_config("/tmp/test-servers");
    let decoded = AppConfig::decode(&encoded, &defaults).expect("decode should succeed");

    let expected = &fixture.expected;
    assert_eq!(decoded.java_path, expected["javaPath"].as_str().unwrap());
    assert_eq!(
        decoded.extra_flags,
        expected["extraFlags"].as_str().unwrap()
    );
    assert_eq!(
        decoded.servers_root,
        expected["serversRoot"].as_str().unwrap()
    );
    assert_eq!(
        decoded.plugin_template_dir,
        expected["pluginTemplateDir"].as_str().unwrap()
    );
    assert_eq!(
        decoded.paper_template_dir,
        expected["paperTemplateDir"].as_str().unwrap()
    );
    assert_eq!(
        decoded.active_server_id.as_deref(),
        expected["activeServerId"].as_str()
    );
    assert_eq!(
        decoded.initial_setup_done,
        expected["initialSetupDone"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.remote_api_port,
        expected["remoteAPIPort"].as_i64().unwrap()
    );
    assert_eq!(
        decoded.remote_api_token,
        expected["remoteAPIToken"].as_str().unwrap(),
        "remote_api_token must decode back to \"\" regardless of what it was set to before encoding"
    );
    assert_eq!(
        decoded.remote_api_expose_on_lan,
        expected["remoteAPIExposeOnLAN"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.remote_api_preferred_pairing_host.as_deref(),
        expected["remoteAPIPreferredPairingHost"].as_str()
    );
    assert_eq!(
        decoded.duckdns_hostname.as_deref(),
        expected["duckdnsHostname"].as_str()
    );
    assert_eq!(
        decoded.playit_java_address.as_deref(),
        expected["playitJavaAddress"].as_str()
    );
    assert_eq!(
        decoded.playit_bedrock_address.as_deref(),
        expected["playitBedrockAddress"].as_str()
    );
    assert_eq!(
        decoded.playit_voice_address.as_deref(),
        expected["playitVoiceAddress"].as_str()
    );
    assert_eq!(
        decoded.playit_agent_id.as_deref(),
        expected["playitAgentId"].as_str()
    );
    assert_eq!(
        decoded.has_shown_handbook,
        expected["hasShownHandbook"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.has_shown_concept_guide,
        expected["hasShownConceptGuide"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.xbox_broadcast_jar_path.as_deref(),
        expected["xboxBroadcastJarPath"].as_str()
    );
    assert_eq!(
        decoded.xbox_broadcast_auto_start_enabled,
        expected["xboxBroadcastAutoStartEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.minecraft_username.as_deref(),
        expected["minecraftUsername"].as_str()
    );
    assert_eq!(
        decoded.minecraft_bedrock_gamertag.as_deref(),
        expected["minecraftBedrockGamertag"].as_str()
    );
    assert_eq!(
        decoded.minecraft_avatar_edition_raw_value.as_deref(),
        expected["minecraftAvatarEditionRawValue"].as_str()
    );
    assert_eq!(
        decoded.default_banner_color_hex.as_deref(),
        expected["defaultBannerColorHex"].as_str()
    );
    assert_eq!(
        decoded.error_popups_enabled,
        expected["errorPopupsEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.save_downloaded_jars,
        expected["saveDownloadedJars"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.use_vm_bedrock_backend,
        expected["useVMBedrockBackend"].as_bool().unwrap()
    );

    assert_eq!(decoded.remote_api_shared_access.len(), 1);
    let decoded_entry = &decoded.remote_api_shared_access[0];
    let expected_entry = &expected["remoteAPISharedAccess"][0];
    assert_eq!(decoded_entry.id, expected_entry["id"].as_str().unwrap());
    assert_eq!(
        decoded_entry.label,
        expected_entry["label"].as_str().unwrap()
    );
    assert_eq!(
        decoded_entry.token,
        expected_entry["token"].as_str().unwrap()
    );
    assert_eq!(decoded_entry.role, expected_entry["role"].as_str().unwrap());
    let expected_permissions: Vec<String> = expected_entry["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(decoded_entry.permissions, Some(expected_permissions));
}

#[test]
fn app_config_schema_registered_server_migrates_host_setup_to_complete() {
    let defaults = AppConfig::default_config("/tmp/test-servers");
    let mut encoded = defaults.encode();
    encoded["initial_setup_done"] = serde_json::Value::Bool(false);
    encoded["servers"] = serde_json::Value::Array(vec![
        ConfigServer::new(
            "survival",
            "Survival",
            "/tmp/test-servers/java/survival",
            "server.jar",
            2.0,
            4.0,
        )
        .encode(),
    ]);

    let decoded = AppConfig::decode(&encoded, &defaults).expect("decode should succeed");

    assert!(decoded.initial_setup_done);
}

#[test]
fn app_config_schema_app_config_missing_optional_fields_get_defaults() {
    let fixture = load("app-config-missing-optional-fields-get-defaults");
    let defaults = AppConfig::default_config("/tmp/MinecraftServers");
    let decoded = AppConfig::decode(&fixture.input["json"], &defaults)
        .expect("a minimal old-schema config must decode, not throw");

    assert_eq!(
        decoded.servers.len(),
        1,
        "the servers array must survive, not be wiped"
    );
    let server = &decoded.servers[0];
    let expected_server = &fixture.expected["servers"][0];
    assert_eq!(server.id, expected_server["id"].as_str().unwrap());
    assert_eq!(
        server.display_name,
        expected_server["displayName"].as_str().unwrap()
    );
    assert_eq!(
        server.server_type.raw_value(),
        expected_server["serverType"].as_str().unwrap()
    );
    assert_eq!(
        server.bedrock_enabled,
        expected_server["bedrockEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        server.auto_backup_enabled,
        expected_server["autoBackupEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        server.auto_backup_interval_minutes,
        expected_server["autoBackupIntervalMinutes"]
            .as_i64()
            .unwrap()
    );
    assert_eq!(
        server.auto_backup_max_count,
        expected_server["autoBackupMaxCount"].as_i64().unwrap()
    );
    assert_eq!(
        server.xbox_broadcast_enabled,
        expected_server["xboxBroadcastEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        server.playit_enabled,
        expected_server["playitEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        server.java_flavor.raw_value(),
        expected_server["javaFlavor"].as_str().unwrap()
    );
    assert_eq!(server.minecraft_version, None);
    assert_eq!(server.loader_version, None);
    assert_eq!(
        server.resource_pack_host_port,
        expected_server["resourcePackHostPort"].as_i64().unwrap()
    );

    let expected = &fixture.expected;
    assert_eq!(
        decoded.remote_api_expose_on_lan,
        expected["remoteAPIExposeOnLAN"].as_bool().unwrap()
    );
    assert_eq!(decoded.duckdns_hostname, None);
    assert_eq!(decoded.playit_java_address, None);
    assert_eq!(
        decoded.has_shown_handbook,
        expected["hasShownHandbook"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.xbox_broadcast_auto_start_enabled,
        expected["xboxBroadcastAutoStartEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.save_downloaded_jars,
        expected["saveDownloadedJars"].as_bool().unwrap()
    );
}

#[test]
fn app_config_schema_config_server_full_round_trip() {
    let fixture = load("config-server-full-round-trip");
    let overrides = &fixture.input["overrides"];

    let mut server = ConfigServer::new(
        str_opt(overrides, "id").unwrap(),
        str_opt(overrides, "displayName").unwrap(),
        str_opt(overrides, "serverDir").unwrap(),
        str_opt(overrides, "paperJarPath").unwrap(),
        overrides["minRamGB"].as_f64().unwrap(),
        overrides["maxRamGB"].as_f64().unwrap(),
    );
    server.bedrock_port = overrides["bedrockPort"].as_i64();
    server.bedrock_enabled = overrides["bedrockEnabled"].as_bool().unwrap();
    server.public_host_override = str_opt(overrides, "publicHostOverride");
    server.notes = str_opt(overrides, "notes").unwrap();
    server.banner_color_hex = str_opt(overrides, "bannerColorHex");
    server.join_card_color_hex = str_opt(overrides, "joinCardColorHex");
    server.has_ever_started = overrides["hasEverStarted"].as_bool().unwrap();
    server.has_shown_first_start_popup = overrides["hasShownFirstStartPopup"].as_bool().unwrap();
    server.auto_backup_enabled = overrides["autoBackupEnabled"].as_bool().unwrap();
    server.auto_backup_interval_minutes = overrides["autoBackupIntervalMinutes"].as_i64().unwrap();
    server.auto_backup_max_count = overrides["autoBackupMaxCount"].as_i64().unwrap();
    server.xbox_broadcast_enabled = overrides["xboxBroadcastEnabled"].as_bool().unwrap();
    server.xbox_broadcast_ip_mode =
        xbox_ip_mode_from_case_name(overrides["xboxBroadcastIPMode"].as_str().unwrap());
    server.xbox_broadcast_host_override = str_opt(overrides, "xboxBroadcastHostOverride");
    server.xbox_broadcast_port_override = overrides["xboxBroadcastPortOverride"].as_i64();
    server.resource_pack_host_port = overrides["resourcePackHostPort"].as_i64().unwrap();
    server.server_type =
        msc_domain::identity::ServerType::from_raw_value(overrides["serverType"].as_str().unwrap())
            .unwrap();
    server.bedrock_version = str_opt(overrides, "bedrockVersion");
    server.java_flavor = msc_domain::identity::JavaServerFlavor::from_raw_value(
        overrides["javaFlavor"].as_str().unwrap(),
    )
    .unwrap();
    server.minecraft_version = str_opt(overrides, "minecraftVersion");
    server.loader_version = str_opt(overrides, "loaderVersion");
    server.server_build = str_opt(overrides, "serverBuild");
    server.playit_enabled = overrides["playitEnabled"].as_bool().unwrap();
    server.playit_voice_chat_enabled = overrides["playitVoiceChatEnabled"].as_bool().unwrap();
    let prefs = &overrides["notificationPrefs"];
    server.notification_prefs = msc_domain::app_config_schema::ServerNotificationPrefs {
        notify_on_start: prefs["notifyOnStart"].as_bool().unwrap(),
        notify_on_stop: prefs["notifyOnStop"].as_bool().unwrap(),
        notify_on_player_join: prefs["notifyOnPlayerJoin"].as_bool().unwrap(),
        notify_on_player_leave: prefs["notifyOnPlayerLeave"].as_bool().unwrap(),
    };
    // Set to a real value before encoding -- the key negative case: it
    // must never appear on the wire, so it decodes back to None.
    server.xbox_broadcast_alt_password = str_opt(overrides, "xboxBroadcastAltPassword");

    let encoded = server.encode();
    assert!(
        encoded.get("xbox_broadcast_alt_password").is_none(),
        "xbox_broadcast_alt_password must never be written to JSON"
    );

    let decoded = ConfigServer::decode(&encoded).expect("decode should succeed");

    let expected = &fixture.expected;
    assert_eq!(decoded.id, expected["id"].as_str().unwrap());
    assert_eq!(
        decoded.display_name,
        expected["displayName"].as_str().unwrap()
    );
    assert_eq!(decoded.server_dir, expected["serverDir"].as_str().unwrap());
    assert_eq!(
        decoded.paper_jar_path,
        expected["paperJarPath"].as_str().unwrap()
    );
    assert_eq!(decoded.min_ram_gb, expected["minRamGB"].as_f64().unwrap());
    assert_eq!(decoded.max_ram_gb, expected["maxRamGB"].as_f64().unwrap());
    assert_eq!(decoded.bedrock_port, expected["bedrockPort"].as_i64());
    assert_eq!(
        decoded.bedrock_enabled,
        expected["bedrockEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.public_host_override.as_deref(),
        expected["publicHostOverride"].as_str()
    );
    assert_eq!(decoded.notes, expected["notes"].as_str().unwrap());
    assert_eq!(
        decoded.banner_color_hex.as_deref(),
        expected["bannerColorHex"].as_str()
    );
    assert_eq!(
        decoded.join_card_color_hex.as_deref(),
        expected["joinCardColorHex"].as_str()
    );
    assert_eq!(
        decoded.has_ever_started,
        expected["hasEverStarted"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.auto_backup_enabled,
        expected["autoBackupEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.auto_backup_interval_minutes,
        expected["autoBackupIntervalMinutes"].as_i64().unwrap()
    );
    assert_eq!(
        decoded.auto_backup_max_count,
        expected["autoBackupMaxCount"].as_i64().unwrap()
    );
    assert_eq!(
        decoded.xbox_broadcast_enabled,
        expected["xboxBroadcastEnabled"].as_bool().unwrap()
    );
    assert_eq!(
        xbox_ip_mode_case_name(decoded.xbox_broadcast_ip_mode),
        expected["xboxBroadcastIPMode"].as_str().unwrap()
    );
    assert_eq!(
        decoded.server_type.raw_value(),
        expected["serverType"].as_str().unwrap()
    );
    assert_eq!(
        decoded.bedrock_version.as_deref(),
        expected["bedrockVersion"].as_str()
    );
    assert_eq!(
        decoded.java_flavor.raw_value(),
        expected["javaFlavor"].as_str().unwrap()
    );
    assert_eq!(
        decoded.minecraft_version.as_deref(),
        expected["minecraftVersion"].as_str()
    );
    assert_eq!(
        decoded.loader_version.as_deref(),
        expected["loaderVersion"].as_str()
    );
    assert_eq!(
        decoded.server_build.as_deref(),
        expected["serverBuild"].as_str()
    );
    assert_eq!(
        decoded.playit_enabled,
        expected["playitEnabled"].as_bool().unwrap()
    );

    let expected_prefs = &expected["notificationPrefs"];
    assert_eq!(
        decoded.notification_prefs.notify_on_start,
        expected_prefs["notifyOnStart"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.notification_prefs.notify_on_player_join,
        expected_prefs["notifyOnPlayerJoin"].as_bool().unwrap()
    );
    assert_eq!(
        decoded.notification_prefs.notify_on_stop,
        expected_prefs["notifyOnStop"].as_bool().unwrap()
    );

    assert_eq!(
        decoded.xbox_broadcast_alt_password, None,
        "must decode back to None, not the value it was set to before encoding"
    );
}

#[test]
fn app_config_schema_config_server_missing_optional_fields_get_defaults() {
    let fixture = load("config-server-missing-optional-fields-get-defaults");
    let decoded = ConfigServer::decode(&fixture.input["json"])
        .expect("a bare ConfigServer with only originally-required keys must decode");

    let expected = &fixture.expected;
    assert_eq!(decoded.id, expected["id"].as_str().unwrap());
    assert_eq!(
        decoded.server_type.raw_value(),
        expected["serverType"].as_str().unwrap()
    );
    assert_eq!(
        decoded.java_flavor.raw_value(),
        expected["javaFlavor"].as_str().unwrap()
    );
    assert_eq!(
        decoded.auto_backup_enabled,
        expected["autoBackupEnabled"].as_bool().unwrap()
    );
    assert_eq!(decoded.addon_links, None);
    assert_eq!(decoded.plugin_sources, None);
}
