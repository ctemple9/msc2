//! The pure decisions server creation makes before it ever touches a disk.
//!
//! Ported from `AppViewModel+ServerCreation.swift`'s `createNewServer`, in
//! source order: name trim/refusal, folder-name derivation, the add-on
//! folder per flavor, the modded RAM default, imported-metadata overrides,
//! the exact `server.properties` key set, the `recordLoaderVersion` guard,
//! the `ConfigServer` field set, and the archive-first shortcut's gate.
//!
//! Scope call (P7.12): `fixtures/server-creation/`'s other 13 cases are
//! filesystem operations -- the pre-existing-folder refusal, the actual
//! directory/jar/properties writes, the install-step-vs-download-and-go
//! branch dispatch, the cross-play template copy, both `WorldSource`
//! copy-failure paths, initial-world-slot failure cleanup, and the
//! top-level `catch` cleanup -- and belong to P7.17/P7.18's application
//! service, not this domain-only step. `fixtures/jar-templates/`'s 10
//! cases are entirely about a real template directory (listing, archiving,
//! reading a template's version from its filename) and belong to P7.15's
//! infrastructure store; none of them are pure decisions with no
//! filesystem in the loop, so none are ported here.

use crate::identity::{AddOnKind, JavaServerCategory, JavaServerFlavor};
use std::collections::BTreeMap;

/// `name.trimmingCharacters(in: .whitespacesAndNewlines)` followed by the
/// empty-after-trim refusal (`AppViewModel+ServerCreation.swift:146-147`).
/// `None` means "refuse before touching a directory at all."
pub fn trimmed_server_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `folderName = safeName.replacingOccurrences(of: " ", with: "_").lowercased()`
/// (`AppViewModel+ServerCreation.swift:169`). Only the literal ASCII space
/// is replaced; other whitespace or punctuation passes through unchanged.
/// Runs on the already-trimmed name.
pub fn folder_name_from_safe_name(safe_name: &str) -> String {
    safe_name.replace(' ', "_").to_lowercase()
}

/// The add-on folder `createNewServer` creates, if any
/// (`AppViewModel+ServerCreation.swift:314`, via `JavaServerFlavor.add_on_kind`).
/// `None` for Vanilla (no plugin/mod API); this is provisioning-kind
/// independent -- Forge/NeoForge (install-step) and Fabric (download-and-go)
/// all take this same path once the two branches rejoin.
pub fn add_on_folder_name(flavor: JavaServerFlavor) -> Option<&'static str> {
    flavor.add_on_kind().map(AddOnKind::folder_name)
}

/// The stable filename for a newly provisioned Java server's primary jar.
/// This is deliberately flavor-specific: `ConfigServer.paper_jar_path` is a
/// legacy field name, but it stores the actual launch jar path for every
/// download-based Java flavor rather than implying that every server is Paper.
pub fn primary_jar_filename(flavor: JavaServerFlavor) -> &'static str {
    match flavor {
        JavaServerFlavor::Paper => "paper.jar",
        JavaServerFlavor::Purpur => "purpur.jar",
        JavaServerFlavor::Pufferfish => "pufferfish.jar",
        JavaServerFlavor::Vanilla => "vanilla.jar",
        JavaServerFlavor::Fabric => "fabric-server-launch.jar",
        JavaServerFlavor::NeoForge => "neoforge.jar",
        JavaServerFlavor::Spigot => "spigot.jar",
        JavaServerFlavor::Forge => "forge.jar",
        JavaServerFlavor::Quilt => "quilt-server-launch.jar",
    }
}

/// The `ConfigServer` initializer's RAM default (2/4 GB), overridden to
/// 3/6 GB for any modded-category flavor
/// (`AppViewModel+ServerCreation.swift:339,345-348`).
pub fn default_ram_gb(flavor: JavaServerFlavor) -> (f64, f64) {
    if flavor.category() == JavaServerCategory::Modded {
        (3.0, 6.0)
    } else {
        (2.0, 4.0)
    }
}

/// A world's difficulty/gamemode/seed as imported from a backup zip or
/// existing folder's `level.dat` (`WorldSlotManager.ImportedWorldMetadata`).
/// A fresh world source has none of these (all `None`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportedWorldMetadata {
    pub difficulty: Option<String>,
    pub gamemode: Option<String>,
    pub seed: Option<String>,
}

/// The result of resolving wizard-chosen values against imported metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveWorldSettings {
    pub difficulty: String,
    pub gamemode: String,
    pub world_seed: Option<String>,
}

/// `effectiveDifficulty = importedMetadata.difficulty ?? difficulty`, same
/// pattern for `gamemode`, and `effectiveWorldSeed = normalizedWorldSeed ??
/// importedMetadata.seed` (`AppViewModel+ServerCreation.swift:160-162`) --
/// note the seed's fallback order is reversed from the other two: the
/// wizard-normalized seed wins over an imported one, not the other way
/// around, matching source exactly rather than "fixing" the asymmetry.
pub fn effective_world_settings(
    wizard_difficulty: &str,
    wizard_gamemode: &str,
    normalized_world_seed: Option<&str>,
    imported: &ImportedWorldMetadata,
) -> EffectiveWorldSettings {
    EffectiveWorldSettings {
        difficulty: imported
            .difficulty
            .clone()
            .unwrap_or_else(|| wizard_difficulty.to_string()),
        gamemode: imported
            .gamemode
            .clone()
            .unwrap_or_else(|| wizard_gamemode.to_string()),
        world_seed: normalized_world_seed
            .map(str::to_string)
            .or_else(|| imported.seed.clone()),
    }
}

/// The exact `server.properties` key set `createNewServer` writes for a
/// freshly created server (`AppViewModel+ServerCreation.swift:298-309`,
/// via `ServerPropertiesManager.writeProperties`, which replaces the whole
/// file rather than merging with a template -- this is the complete set,
/// not a subset). `level-seed` is present only when a seed was resolved.
/// `BTreeMap` gives deterministic iteration for tests; callers write it out
/// however their properties-file format needs.
#[allow(clippy::too_many_arguments)]
pub fn fresh_server_properties(
    port: u16,
    motd: &str,
    difficulty: &str,
    gamemode: &str,
    level_name: &str,
    world_seed: Option<&str>,
) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert("server-port".to_string(), port.to_string());
    props.insert("motd".to_string(), motd.to_string());
    props.insert("max-players".to_string(), "20".to_string());
    props.insert("online-mode".to_string(), "true".to_string());
    props.insert("difficulty".to_string(), difficulty.to_string());
    props.insert("gamemode".to_string(), gamemode.to_string());
    props.insert("level-name".to_string(), level_name.to_string());
    if let Some(seed) = world_seed {
        props.insert("level-seed".to_string(), seed.to_string());
    }
    props
}

/// `if cfgServer.javaFlavor.category == .modded, let mc = ..., let loader =
/// ... { recordLoaderVersion(...) }` (`AppViewModel+ServerCreation.swift:383-386`).
/// All three conditions are required: a standard-category flavor never
/// calls this even though `minecraftVersion` is always set for it, and a
/// modded flavor with no resolved loader version (shouldn't happen in
/// practice for NeoForge/Forge) also skips it.
pub fn should_record_loader_version(
    flavor: JavaServerFlavor,
    minecraft_version: Option<&str>,
    loader_version: Option<&str>,
) -> bool {
    flavor.category() == JavaServerCategory::Modded
        && minecraft_version.is_some()
        && loader_version.is_some()
}

/// The archive-first shortcut's gate: `flavor == .paper &&
/// configManager.config.saveDownloadedJars`
/// (`AppViewModel+ServerCreation.swift:258`). `false` means "skip straight
/// to `ServerJarProvider.downloadLatest`, no archive/metadata check."
pub fn should_use_archive_first_shortcut(
    flavor: JavaServerFlavor,
    save_downloaded_jars: bool,
) -> bool {
    flavor == JavaServerFlavor::Paper && save_downloaded_jars
}

/// The fields `createNewServer` sets on a newly constructed `ConfigServer`,
/// beyond the base initializer's `id`/`displayName`/`serverDir`/
/// `paperJarPath`/`minRamGB`/`maxRamGB`/`notes`
/// (`AppViewModel+ServerCreation.swift:338-354`). `bedrock_port` is set
/// only when cross-play is enabled and a port was resolved -- absent here
/// otherwise, matching source exactly rather than defaulting it to some
/// sentinel value.
#[derive(Debug, Clone, PartialEq)]
pub struct NewServerConfigFields {
    pub id: String,
    pub display_name: String,
    pub server_dir: String,
    pub paper_jar_path: String,
    pub min_ram_gb: f64,
    pub max_ram_gb: f64,
    pub notes: String,
    pub java_flavor: JavaServerFlavor,
    pub minecraft_version: Option<String>,
    pub server_build: Option<String>,
    pub loader_version: Option<String>,
    pub banner_color_hex: String,
    pub playit_enabled: bool,
    pub xbox_broadcast_enabled: bool,
    pub bedrock_port: Option<u16>,
}

/// `ConfigServer(id: newId, displayName: safeName, serverDir: newDir.path,
/// paperJarPath: primaryJarPath, minRamGB: 2, maxRamGB: 4, notes: "")`
/// followed by one field-assignment statement per remaining field
/// (`AppViewModel+ServerCreation.swift:338-354`) -- not part of the
/// initializer itself, but preserved here as one function since nothing
/// downstream needs the two steps separated. `id` is a freshly generated
/// UUID, the same `Uuid::new_v4()` precedent
/// `app_config_schema::normalize`'s duplicate-id repair already uses.
/// `min_ram_gb`/`max_ram_gb` come from [`default_ram_gb`] -- pass its
/// result in rather than recomputing it here, so a caller that already
/// resolved a modded RAM override doesn't have two sources of truth.
#[allow(clippy::too_many_arguments)]
pub fn new_server_config_fields(
    display_name: &str,
    server_dir: &str,
    paper_jar_path: &str,
    min_ram_gb: f64,
    max_ram_gb: f64,
    flavor: JavaServerFlavor,
    resolved_version: Option<&str>,
    resolved_build: Option<&str>,
    resolved_loader: Option<&str>,
    default_banner_color_hex: &str,
    enable_playit: bool,
    enable_xbox_broadcast: bool,
    cross_play_bedrock_port: Option<u16>,
) -> NewServerConfigFields {
    NewServerConfigFields {
        id: uuid::Uuid::new_v4().to_string(),
        display_name: display_name.to_string(),
        server_dir: server_dir.to_string(),
        paper_jar_path: paper_jar_path.to_string(),
        min_ram_gb,
        max_ram_gb,
        notes: String::new(),
        java_flavor: flavor,
        minecraft_version: resolved_version.map(str::to_string),
        server_build: resolved_build.map(str::to_string),
        loader_version: resolved_loader.map(str::to_string),
        banner_color_hex: default_banner_color_hex.to_string(),
        playit_enabled: enable_playit,
        xbox_broadcast_enabled: enable_xbox_broadcast,
        bedrock_port: cross_play_bedrock_port,
    }
}
