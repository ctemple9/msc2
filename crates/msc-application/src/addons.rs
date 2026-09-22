//! Add-on install, update, toggle, remove, and source linking — the real
//! mutation half of `AddonUpdateResolver`/`AppViewModel+ModManagement.swift`/
//! `AppViewModel+PluginManagement.swift`, wiring P8.14's verified storage
//! primitives and P8.15's dependency installer to real add-on/plugin
//! mutations. The `pack_mutation_refused` policy seam is passed explicitly
//! through every mutation, including the two `health/repair` actions (P8.23),
//! but imported pack metadata does not currently refuse individual changes.
//!
//! **No `LifecycleOperations`/audit-log wiring here**, the same deferral
//! `addon_updates.rs`'s own module doc already explains for the read path:
//! operation-lifecycle and audit-log wiring are the *route* layer's job
//! (P8.24), once a route exists to own a mutation across an async boundary
//! and to supply the client-IP/token-label the audit primitive needs —
//! neither is available to a synchronous application function. Every
//! function here returns a plain, typed `Result` a route can translate
//! into an operation outcome and an audit entry.
//!
//! **No stopped-server requirement.** Checked directly against the
//! oracle (`AppViewModel+ModManagement.swift`, `AppViewModel+
//! PluginManagement.swift`): neither file checks `isRunning` before
//! mutating a mod/plugin folder — MSC 1 lets you install/update/toggle/
//! remove an add-on while the server is running (it just won't take
//! effect until the next restart). This port preserves that; "enforce
//! stopped-server... rules where required" (`rolling-plan.md`'s own P8.17
//! text) is satisfied by there being no such requirement to enforce,
//! decided rather than silently assumed.
//!
//! **Disabled-state preservation across replacement.** Both update paths
//! below (Modrinth-catalog and plugin-source) compute their final
//! destination with the entry's disabled suffix preserved when the
//! existing file was disabled — an item you disabled stays disabled after
//! an update, matching `rolling-plan.md`'s own explicit requirement for
//! this step.

use std::fmt;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use msc_domain::addon_provider::{self as domain, ModrinthVersionInfo};
use msc_domain::addon_update::{self, PluginVersionDispatch};
use msc_domain::app_config_schema::{
    AddonLink, AddonLinkProvenance, PluginSourceConfig, PluginSourceKind,
};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack::{self, AddonMutationKind};
use msc_domain::plugin_source;

use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::addon_store::{self, AddonStoreError, DISABLED_SUFFIX};
use msc_infrastructure::download_staging::ExpectedChecksum;
use msc_infrastructure::fs::FileSystem;

use crate::addon_dependencies::{self, DependencyInstallReport};
use crate::addon_updates::AddonUpdateItem;

use std::collections::HashMap;

#[derive(Debug)]
pub enum JavaDatapackError {
    Invalid(String),
    IncompatibleVersion,
    Io(String),
}

impl fmt::Display for JavaDatapackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Io(message) => f.write_str(message),
            Self::IncompatibleVersion => f.write_str(
                "This datapack version does not list the selected world's Minecraft version.",
            ),
        }
    }
}

/// Checks a Modrinth datapack archive, then writes its files under the
/// selected Java world's `datapacks` directory inside the slot archive.
/// The prior archive is retained beside the slot before the replacement.
pub fn install_java_datapack(
    world_zip_path: &Path,
    archive_bytes: &[u8],
    version: &ModrinthVersionInfo,
    project_id: &str,
    project_title: &str,
    minecraft_version: &str,
) -> Result<(String, String, Vec<String>, PathBuf), JavaDatapackError> {
    if !version
        .game_versions
        .iter()
        .any(|value| value == minecraft_version)
    {
        return Err(JavaDatapackError::IncompatibleVersion);
    }
    let selected_file = domain::modrinth_primary_file(&version.files)
        .ok_or_else(|| JavaDatapackError::Invalid("The selected version has no archive.".into()))?;
    if let Some(expected) = selected_file.hashes.get("sha512") {
        let actual = msc_infrastructure::download_staging::sha512_hex(archive_bytes);
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(JavaDatapackError::Invalid(
                "The downloaded datapack failed its SHA-512 check.".into(),
            ));
        }
    }

    let mut incoming = zip::ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|error| JavaDatapackError::Invalid(format!("Invalid datapack ZIP: {error}")))?;
    if incoming.is_empty() || incoming.len() > 20_000 {
        return Err(JavaDatapackError::Invalid(
            "The datapack archive is empty or contains too many entries.".into(),
        ));
    }
    let mut pack_files = Vec::<(String, Vec<u8>)>::new();
    let mut expanded_size = 0_u64;
    for index in 0..incoming.len() {
        let mut entry = incoming.by_index(index).map_err(|error| {
            JavaDatapackError::Invalid(format!("Could not read datapack archive: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let path = Path::new(&name);
        if name.starts_with('/')
            || name.contains(':')
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(JavaDatapackError::Invalid(
                "The datapack archive contains an unsafe path or symbolic link.".into(),
            ));
        }
        expanded_size = expanded_size.saturating_add(entry.size());
        if expanded_size > 2 * 1024 * 1024 * 1024 {
            return Err(JavaDatapackError::Invalid(
                "The unpacked datapack exceeds the 2 GB safety limit.".into(),
            ));
        }
        let mut bytes = Vec::with_capacity(entry.size().min(16 * 1024 * 1024) as usize);
        entry.read_to_end(&mut bytes).map_err(|error| {
            JavaDatapackError::Invalid(format!("Could not read datapack file: {error}"))
        })?;
        pack_files.push((name, bytes));
    }
    let (metadata_name, metadata_bytes) = pack_files
        .iter()
        .find(|(name, _)| name == "pack.mcmeta" || name.ends_with("/pack.mcmeta"))
        .ok_or_else(|| {
            JavaDatapackError::Invalid(
                "The archive does not contain Java datapack metadata (pack.mcmeta).".into(),
            )
        })?;
    let wrapper = metadata_name.strip_suffix("pack.mcmeta").unwrap_or("");
    if !wrapper.is_empty()
        && pack_files
            .iter()
            .any(|(name, _)| !name.starts_with(wrapper))
    {
        return Err(JavaDatapackError::Invalid(
            "Datapack files do not share the directory containing pack.mcmeta.".into(),
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(metadata_bytes).map_err(|error| {
        JavaDatapackError::Invalid(format!("The datapack pack.mcmeta is invalid JSON: {error}"))
    })?;
    if metadata
        .pointer("/pack/pack_format")
        .and_then(serde_json::Value::as_u64)
        .is_none_or(|format| format == 0)
        || metadata.pointer("/pack/description").is_none()
    {
        return Err(JavaDatapackError::Invalid(
            "The datapack pack.mcmeta is missing a valid pack format or description.".into(),
        ));
    }
    let pack_folder = format!(
        "{}-{}",
        safe_pack_path(project_id),
        safe_pack_path(&version.id)
    );

    let world_file = std::fs::File::open(world_zip_path)
        .map_err(|error| JavaDatapackError::Io(format!("Could not read world archive: {error}")))?;
    let mut world = zip::ZipArchive::new(world_file).map_err(|error| {
        JavaDatapackError::Invalid(format!("The selected world archive is invalid: {error}"))
    })?;
    let world_root = world
        .file_names()
        .find_map(|name| name.strip_suffix("level.dat"))
        .unwrap_or("")
        .to_string();
    let datapack_root = format!("{world_root}datapacks/{pack_folder}/");
    let existing: std::collections::HashSet<String> =
        world.file_names().map(str::to_string).collect();
    if existing.iter().any(|name| name.starts_with(&datapack_root)) {
        return Err(JavaDatapackError::Invalid(
            "This datapack version is already installed in the selected world.".into(),
        ));
    }
    let temp_path = world_zip_path.with_extension(format!("{}.datapack.tmp", uuid::Uuid::new_v4()));
    let temp_file = std::fs::File::create(&temp_path).map_err(|error| {
        JavaDatapackError::Io(format!("Could not stage the updated world: {error}"))
    })?;
    let mut output = zip::ZipWriter::new(temp_file);
    let options = zip::write::SimpleFileOptions::default();
    for index in 0..world.len() {
        let mut entry = world.by_index(index).map_err(|error| {
            JavaDatapackError::Invalid(format!("Could not read world archive: {error}"))
        })?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            output
                .add_directory(name, options)
                .map_err(|error| JavaDatapackError::Io(error.to_string()))?;
        } else {
            output
                .start_file(name, options)
                .map_err(|error| JavaDatapackError::Io(error.to_string()))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|error| JavaDatapackError::Io(error.to_string()))?;
        }
    }
    let mut installed_paths = Vec::with_capacity(pack_files.len());
    for (name, bytes) in &pack_files {
        let name = name.strip_prefix(wrapper).unwrap_or(name);
        let installed_name = format!("{datapack_root}{name}");
        installed_paths.push(format!("datapacks/{pack_folder}/{name}"));
        output
            .start_file(installed_name, options)
            .map_err(|error| JavaDatapackError::Io(error.to_string()))?;
        output
            .write_all(bytes)
            .map_err(|error| JavaDatapackError::Io(error.to_string()))?;
    }
    output
        .finish()
        .map_err(|error| JavaDatapackError::Io(error.to_string()))?
        .sync_all()
        .map_err(|error| {
            JavaDatapackError::Io(format!("Could not flush updated world: {error}"))
        })?;
    let backup_path =
        world_zip_path.with_extension(format!("{}.pre-datapack.bak", uuid::Uuid::new_v4()));
    std::fs::copy(world_zip_path, &backup_path).map_err(|error| {
        JavaDatapackError::Io(format!("Could not back up the selected world: {error}"))
    })?;
    if world_zip_path.exists() {
        std::fs::remove_file(world_zip_path).map_err(|error| {
            let _ = std::fs::remove_file(&temp_path);
            let _ = std::fs::remove_file(&backup_path);
            JavaDatapackError::Io(format!("Could not replace the world archive: {error}"))
        })?;
    }
    if let Err(error) = std::fs::rename(&temp_path, world_zip_path) {
        let _ = std::fs::remove_file(&temp_path);
        let _ = std::fs::copy(&backup_path, world_zip_path);
        let _ = std::fs::remove_file(&backup_path);
        return Err(JavaDatapackError::Io(format!(
            "Could not replace the world archive: {error}"
        )));
    }
    Ok((
        project_title.to_string(),
        msc_infrastructure::download_staging::sha512_hex(archive_bytes),
        installed_paths,
        backup_path,
    ))
}

#[derive(Debug)]
pub enum BedrockBehaviorPackError {
    Invalid(String),
    IncompatibleVersion,
    Io(String),
}

impl fmt::Display for BedrockBehaviorPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Io(message) => f.write_str(message),
            Self::IncompatibleVersion => f.write_str(
                "This behavior pack requires a newer Bedrock version than the server provides.",
            ),
        }
    }
}

/// Validates a downloaded `.mcpack`/`.mcaddon`, installs its behavior packs
/// (and any bundled linked resource packs) into one Bedrock world's archive,
/// and keeps a recovery copy before replacing that archive.
pub fn install_bedrock_behavior_pack(
    world_zip_path: &Path,
    archive_bytes: &[u8],
    source_name: &str,
    source_version: &str,
    source_url: &str,
    bedrock_version: &str,
) -> Result<(Vec<msc_domain::world_profile::WorldPackRecord>, PathBuf), BedrockBehaviorPackError> {
    use msc_domain::bedrock::{parse_behavior_pack_manifest, parse_resource_pack_manifest};
    use msc_domain::world_profile::{WorldPackDependency, WorldPackRecord, WorldPackSource};

    struct PackContent {
        uuid: String,
        name: String,
        version: String,
        minimum: String,
        dependencies: Vec<msc_domain::bedrock::BehaviorPackDependency>,
        behavior: bool,
        files: Vec<(String, Vec<u8>)>,
        expanded_size: u64,
    }

    fn clean_path(name: &str) -> Result<String, BedrockBehaviorPackError> {
        let normalized = name.replace('\\', "/");
        let path = Path::new(&normalized);
        if normalized.is_empty()
            || normalized.starts_with('/')
            || normalized.contains(':')
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(BedrockBehaviorPackError::Invalid(
                "The add-on archive contains a path outside the pack.".to_string(),
            ));
        }
        Ok(normalized)
    }

    fn read_pack(bytes: &[u8]) -> Result<PackContent, BedrockBehaviorPackError> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
            BedrockBehaviorPackError::Invalid(format!("Invalid Bedrock pack archive: {error}"))
        })?;
        if zip.is_empty() || zip.len() > 20_000 {
            return Err(BedrockBehaviorPackError::Invalid(
                "The Bedrock pack archive is empty or contains too many files.".to_string(),
            ));
        }
        let mut files = Vec::new();
        let mut expanded = 0_u64;
        let mut manifest = None;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|error| {
                BedrockBehaviorPackError::Invalid(format!("Could not read Bedrock pack: {error}"))
            })?;
            if entry.is_dir() {
                continue;
            }
            let name = clean_path(entry.name())?;
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(BedrockBehaviorPackError::Invalid(
                    "The Bedrock pack contains a symbolic link.".to_string(),
                ));
            }
            expanded = expanded.saturating_add(entry.size());
            if expanded > 2 * 1024 * 1024 * 1024 {
                return Err(BedrockBehaviorPackError::Invalid(
                    "The unpacked Bedrock pack exceeds the 2 GB safety limit.".to_string(),
                ));
            }
            let mut contents = Vec::with_capacity(entry.size().min(16 * 1024 * 1024) as usize);
            entry.read_to_end(&mut contents).map_err(|error| {
                BedrockBehaviorPackError::Invalid(format!("Could not read Bedrock pack: {error}"))
            })?;
            if name == "manifest.json" || name.ends_with("/manifest.json") {
                if manifest.is_some() {
                    return Err(BedrockBehaviorPackError::Invalid(
                        "The pack archive contains more than one manifest.".to_string(),
                    ));
                }
                manifest = Some((name.clone(), contents.clone()));
            }
            files.push((name, contents));
        }
        let (manifest_path, manifest_bytes) = manifest.ok_or_else(|| {
            BedrockBehaviorPackError::Invalid(
                "The Bedrock add-on does not contain a pack manifest.".to_string(),
            )
        })?;
        let wrapper = manifest_path.strip_suffix("manifest.json").unwrap_or("");
        if !wrapper.is_empty() && files.iter().any(|(name, _)| !name.starts_with(wrapper)) {
            return Err(BedrockBehaviorPackError::Invalid(
                "Pack files do not share the directory containing manifest.json.".to_string(),
            ));
        }
        let raw: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|error| {
            BedrockBehaviorPackError::Invalid(format!("Invalid Bedrock manifest: {error}"))
        })?;
        let behavior = raw
            .get("modules")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|modules| {
                modules.iter().any(|module| {
                    matches!(
                        module.get("type").and_then(serde_json::Value::as_str),
                        Some("data" | "script")
                    )
                })
            });
        let parsed = if behavior {
            parse_behavior_pack_manifest(&manifest_bytes)
        } else {
            parse_resource_pack_manifest(&manifest_bytes)
        }
        .map_err(BedrockBehaviorPackError::Invalid)?;
        let uuid = parsed.uuid;
        let name = parsed.name;
        let version = parsed.version;
        let minimum = parsed.minimum_bedrock_version;
        let dependencies = parsed.dependencies;
        let relative_files = files
            .into_iter()
            .map(|(name, bytes)| {
                (
                    name.strip_prefix(wrapper).unwrap_or(&name).to_owned(),
                    bytes,
                )
            })
            .collect();
        Ok(PackContent {
            uuid,
            name,
            version,
            minimum,
            dependencies,
            behavior,
            files: relative_files,
            expanded_size: expanded,
        })
    }

    fn version_tuple(value: &str) -> Option<(u64, u64, u64)> {
        let core = value.split(['-', '+']).next()?;
        let parts = core
            .split('.')
            .take(3)
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;
        (parts.len() == 3).then(|| (parts[0], parts[1], parts[2]))
    }

    let mut incoming = zip::ZipArchive::new(Cursor::new(archive_bytes)).map_err(|error| {
        BedrockBehaviorPackError::Invalid(format!("Invalid Bedrock add-on archive: {error}"))
    })?;
    if incoming.is_empty() || incoming.len() > 20_000 {
        return Err(BedrockBehaviorPackError::Invalid(
            "The Bedrock add-on archive is empty or contains too many entries.".to_string(),
        ));
    }
    let mut payloads = Vec::<Vec<u8>>::new();
    let mut total_size = 0_u64;
    for index in 0..incoming.len() {
        let mut entry = incoming.by_index(index).map_err(|error| {
            BedrockBehaviorPackError::Invalid(format!("Could not read add-on archive: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        let name = clean_path(entry.name())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(BedrockBehaviorPackError::Invalid(
                "The add-on archive contains a symbolic link.".into(),
            ));
        }
        total_size = total_size.saturating_add(entry.size());
        if total_size > 2 * 1024 * 1024 * 1024 {
            return Err(BedrockBehaviorPackError::Invalid(
                "The add-on archive exceeds the 2 GB safety limit.".into(),
            ));
        }
        let mut bytes = Vec::with_capacity(entry.size().min(16 * 1024 * 1024) as usize);
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| BedrockBehaviorPackError::Invalid(error.to_string()))?;
        if name.to_ascii_lowercase().ends_with(".mcpack") {
            payloads.push(bytes);
        }
    }
    if payloads.is_empty() {
        payloads.push(archive_bytes.to_vec());
    }
    let mut packs = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut total_expanded_size = 0_u64;
    for payload in payloads {
        let pack = read_pack(&payload)?;
        total_expanded_size = total_expanded_size.saturating_add(pack.expanded_size);
        if total_expanded_size > 2 * 1024 * 1024 * 1024 {
            return Err(BedrockBehaviorPackError::Invalid(
                "The complete Bedrock add-on exceeds the 2 GB unpacked safety limit.".into(),
            ));
        }
        if !seen.insert(pack.uuid.to_ascii_lowercase()) {
            return Err(BedrockBehaviorPackError::Invalid(
                "The add-on contains duplicate pack UUIDs.".into(),
            ));
        }
        let minimum_version = version_tuple(&pack.minimum).ok_or_else(|| {
            BedrockBehaviorPackError::Invalid(format!(
                "{} declares an unreadable minimum Bedrock version.",
                pack.name
            ))
        })?;
        let current_version = version_tuple(bedrock_version).ok_or_else(|| {
            BedrockBehaviorPackError::Invalid(
                "The running Bedrock server version cannot be compared with pack requirements."
                    .to_string(),
            )
        })?;
        if minimum_version > current_version {
            return Err(BedrockBehaviorPackError::IncompatibleVersion);
        }
        packs.push(pack);
    }
    let behavior_ids: std::collections::BTreeSet<String> = packs
        .iter()
        .filter(|pack| pack.behavior)
        .map(|pack| pack.uuid.to_ascii_lowercase())
        .collect();
    if behavior_ids.is_empty() {
        return Err(BedrockBehaviorPackError::Invalid(
            "The add-on contains no behavior pack.".into(),
        ));
    }
    let resource_ids: std::collections::BTreeSet<String> = packs
        .iter()
        .filter(|pack| !pack.behavior)
        .map(|pack| pack.uuid.to_ascii_lowercase())
        .collect();
    let bundled_versions: std::collections::BTreeMap<String, &str> = packs
        .iter()
        .map(|pack| (pack.uuid.to_ascii_lowercase(), pack.version.as_str()))
        .collect();
    for pack in &packs {
        for dependency in &pack.dependencies {
            let dependency_id = dependency.uuid.to_ascii_lowercase();
            if !behavior_ids.contains(&dependency_id) && !resource_ids.contains(&dependency_id) {
                return Err(BedrockBehaviorPackError::Invalid(format!(
                    "{} requires pack {} (version {}) which is not included in this add-on. Include the linked pack before installing. The world was not changed.",
                    pack.name, dependency.uuid, dependency.version
                )));
            }
            if bundled_versions.get(&dependency_id).copied() != Some(dependency.version.as_str()) {
                return Err(BedrockBehaviorPackError::Invalid(format!(
                    "{} requires pack {} version {}, but the bundled pack has a different version. The world was not changed.",
                    pack.name, dependency.uuid, dependency.version
                )));
            }
        }
    }

    let mut world = zip::ZipArchive::new(
        std::fs::File::open(world_zip_path)
            .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?,
    )
    .map_err(|error| {
        BedrockBehaviorPackError::Invalid(format!(
            "The selected Bedrock world archive is invalid: {error}"
        ))
    })?;
    let mut behavior_config = serde_json::Value::Array(Vec::new());
    let mut resource_config = serde_json::Value::Array(Vec::new());
    for (name, value) in [
        ("world_behavior_packs.json", &mut behavior_config),
        ("world_resource_packs.json", &mut resource_config),
    ] {
        for index in 0..world.len() {
            let mut entry = world
                .by_index(index)
                .map_err(|error| BedrockBehaviorPackError::Invalid(error.to_string()))?;
            if entry.name() == name {
                let mut bytes = Vec::new();
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
                *value = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|_| {
                    BedrockBehaviorPackError::Invalid(format!(
                        "The world's {name} file is malformed; no changes were made."
                    ))
                })?;
                if !value.is_array() {
                    return Err(BedrockBehaviorPackError::Invalid(format!(
                        "The world's {name} file is not a pack list; no changes were made."
                    )));
                }
                break;
            }
        }
    }
    let temp_path =
        world_zip_path.with_extension(format!("{}.behavior-pack.tmp", uuid::Uuid::new_v4()));
    let temp_file = std::fs::File::create(&temp_path)
        .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
    let mut output = zip::ZipWriter::new(temp_file);
    let options = zip::write::SimpleFileOptions::default();
    let mut existing = std::collections::BTreeSet::new();
    for index in 0..world.len() {
        let mut entry = world
            .by_index(index)
            .map_err(|error| BedrockBehaviorPackError::Invalid(error.to_string()))?;
        let name = entry.name().to_owned();
        existing.insert(name.clone());
        if name == "world_behavior_packs.json" || name == "world_resource_packs.json" {
            continue;
        }
        if entry.is_dir() {
            output
                .add_directory(name, options)
                .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
        } else {
            output
                .start_file(name, options)
                .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
        }
    }
    let mut installed = Vec::new();
    for pack in &packs {
        let folder = if pack.behavior {
            "behavior_packs"
        } else {
            "resource_packs"
        };
        let root = format!("{folder}/{}/", pack.uuid);
        for (name, bytes) in &pack.files {
            let path = format!("{root}{name}");
            if existing.contains(&path) {
                let _ = std::fs::remove_file(&temp_path);
                return Err(BedrockBehaviorPackError::Invalid(
                    "A pack file conflicts with an existing file in this world.".into(),
                ));
            }
            output
                .start_file(&path, options)
                .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
            output
                .write_all(bytes)
                .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
        }
        installed.push(WorldPackRecord {
            id: pack.uuid.clone(),
            edition: "bedrock".into(),
            kind: "bedrock_behavior_pack".into(),
            name: pack.name.clone(),
            source: WorldPackSource {
                provider: Some("curseforge".into()),
                project_id: Some(source_name.to_owned()),
                version_id: Some(source_version.to_owned()),
                version: Some(pack.version.clone()),
                url: Some(source_url.to_owned()),
            },
            files: pack
                .files
                .iter()
                .map(|(name, _)| format!("{folder}/{}/{name}", pack.uuid))
                .collect(),
            checksum: Some(msc_infrastructure::download_staging::sha512_hex(
                archive_bytes,
            )),
            compatibility: Some(format!(
                "Bedrock {minimum} or newer",
                minimum = pack.minimum
            )),
            minecraft_versions: vec![bedrock_version.to_owned()],
            enabled: true,
            dependencies: pack
                .dependencies
                .iter()
                .map(|dependency| WorldPackDependency {
                    id: dependency.uuid.clone(),
                    kind: if resource_ids.contains(&dependency.uuid.to_ascii_lowercase()) {
                        "linked_resource_pack".into()
                    } else {
                        "behavior_pack".into()
                    },
                    required: true,
                })
                .collect(),
        });
    }
    for pack in &packs {
        let target = if pack.behavior {
            &mut behavior_config
        } else {
            &mut resource_config
        };
        target.as_array_mut().expect("initialized as array").push(serde_json::json!({"pack_id": pack.uuid, "version": pack.version.split('.').filter_map(|part| part.parse::<u64>().ok()).collect::<Vec<_>>() }));
    }
    for (name, config) in [
        ("world_behavior_packs.json", behavior_config),
        ("world_resource_packs.json", resource_config),
    ] {
        output
            .start_file(name, options)
            .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
        output
            .write_all(
                serde_json::to_string_pretty(&config)
                    .unwrap_or_default()
                    .as_bytes(),
            )
            .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
    }
    output
        .finish()
        .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?
        .sync_all()
        .map_err(|error| BedrockBehaviorPackError::Io(error.to_string()))?;
    let backup_path =
        world_zip_path.with_extension(format!("{}.pre-behavior-pack.bak", uuid::Uuid::new_v4()));
    std::fs::copy(world_zip_path, &backup_path).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        BedrockBehaviorPackError::Io(format!("Could not back up the selected world: {error}"))
    })?;
    if let Err(error) = std::fs::remove_file(world_zip_path) {
        let _ = std::fs::remove_file(&temp_path);
        let _ = std::fs::remove_file(&backup_path);
        return Err(BedrockBehaviorPackError::Io(format!(
            "Could not prepare the selected world for replacement: {error}"
        )));
    }
    if let Err(error) = std::fs::rename(&temp_path, world_zip_path) {
        let _ = std::fs::copy(&backup_path, world_zip_path);
        let _ = std::fs::remove_file(&temp_path);
        return Err(BedrockBehaviorPackError::Io(format!(
            "Could not replace the selected world archive; the recovery copy is at {}: {error}",
            backup_path.display()
        )));
    }
    Ok((installed, backup_path))
}

fn safe_pack_path(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug)]
pub enum AddonMutationError {
    /// The target server is pack-managed and this mutation isn't the
    /// sanctioned whole-pack-replace escape hatch (`msc_domain::modpack`,
    /// P8.12).
    PackManaged,
    /// `flavor.add_on_kind()` is `None` (Vanilla has no add-on folder).
    NoAddOnKind,
    /// The chosen Modrinth version has no installable primary file.
    NoPrimaryFile,
    /// `update_one` was asked to update an item whose bucket isn't
    /// `UpdateAvailable`, or whose `available_version` is missing.
    NoUpdateAvailable,
    /// A plugin-source URL this crate could not classify or parse at all.
    UnrecognizedSource,
    Provider(String),
    Store(String),
    Io(String),
}

impl fmt::Display for AddonMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackManaged => write!(f, "this server is managed by a modpack"),
            Self::NoAddOnKind => write!(f, "this server flavor has no add-on folder"),
            Self::NoPrimaryFile => write!(f, "the selected version has no installable file"),
            Self::NoUpdateAvailable => write!(f, "no update is available for this add-on"),
            Self::UnrecognizedSource => write!(f, "this plugin source URL could not be resolved"),
            Self::Provider(m) => write!(f, "{m}"),
            Self::Store(m) => write!(f, "{m}"),
            Self::Io(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AddonMutationError {}

impl From<AddonStoreError> for AddonMutationError {
    fn from(e: AddonStoreError) -> Self {
        Self::Store(e.to_string())
    }
}

impl From<domain::AddonProviderError> for AddonMutationError {
    fn from(e: domain::AddonProviderError) -> Self {
        Self::Provider(e.to_string())
    }
}

fn ensure_not_pack_managed(
    pack_managed: bool,
    kind: AddonMutationKind,
) -> Result<(), AddonMutationError> {
    if modpack::pack_mutation_refused(pack_managed, kind) {
        return Err(AddonMutationError::PackManaged);
    }
    Ok(())
}

fn add_on_folder(
    server_dir: &Path,
    flavor: JavaServerFlavor,
) -> Result<PathBuf, AddonMutationError> {
    let kind = flavor
        .add_on_kind()
        .ok_or(AddonMutationError::NoAddOnKind)?;
    Ok(server_dir.join(kind.folder_name()))
}

/// A `.jar` destination path, with `DISABLED_SUFFIX` appended when the
/// entry being replaced was disabled — "preserve disabled suffixes...
/// across replacement" (this module's own doc).
fn dest_preserving_disabled_state(folder: &Path, filename: &str, was_enabled: bool) -> PathBuf {
    if was_enabled {
        folder.join(filename)
    } else {
        folder.join(format!("{filename}{DISABLED_SUFFIX}"))
    }
}

// ---------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct InstallOutcome {
    pub installed_path: PathBuf,
    pub dependencies: DependencyInstallReport,
}

/// Installs `version`'s primary file into the server's add-on folder, then
/// chases its required dependencies (P8.15). A catalog install always
/// lands enabled — MSC 1's own catalog/search install flow has no
/// disabled-on-install option either.
#[allow(clippy::too_many_arguments)]
pub fn install_from_catalog(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    version: &ModrinthVersionInfo,
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<InstallOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Install)?;
    let folder = add_on_folder(server_dir, flavor)?;
    let primary =
        domain::modrinth_primary_file(&version.files).ok_or(AddonMutationError::NoPrimaryFile)?;

    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = folder.join(&primary.filename);
    let expected_checksum = primary
        .hashes
        .get("sha1")
        .map(|hex| ExpectedChecksum::sha1(hex.clone()));

    addon_store::install_verified_file(
        transport,
        fs,
        &primary.url,
        &version.version_number,
        expected_checksum.as_ref(),
        &dest,
    )?;

    let dependencies = addon_dependencies::install_required_dependencies(
        transport,
        fs,
        version,
        flavor,
        minecraft_version,
        server_dir,
        installed_mod_ids,
        should_cancel,
    );

    Ok(InstallOutcome {
        installed_path: dest,
        dependencies,
    })
}

/// Installs an already-staged local jar (a client-uploaded file redeemed
/// through `POST /v1/staged-uploads` by the caller — P8.24's job, this
/// function only ever sees a real path on disk already) verbatim into the
/// server's add-on folder. No provider, no checksum to verify against
/// (there is no publisher digest for a locally-supplied file), matching
/// `AppViewModel+ModManagement`'s own manual-file-add path, which performs
/// no structural validation beyond the copy itself.
pub fn install_from_staged_local_jar(
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    staged_jar_path: &Path,
    filename: &str,
    pack_managed: bool,
) -> Result<PathBuf, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Install)?;
    let folder = add_on_folder(server_dir, flavor)?;
    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let bytes = fs
        .read(staged_jar_path)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = folder.join(filename);
    fs.write(&dest, &bytes)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    Ok(dest)
}

// ---------------------------------------------------------------------
// Update (Modrinth-catalog-linked add-ons — mods and Modrinth-linked
// plugins alike, the `AddonUpdateResolver`/`addon_updates.rs` path)
// ---------------------------------------------------------------------

/// Updates one item to its already-resolved `available_version`
/// (`addon_updates::resolve_addon_updates`'s own output — no second
/// Modrinth request here, see that module's own P8.17 amendment note on
/// `AddonUpdateItem::available_version`). A filename change (a version
/// bump that renames the jar) removes the stale file only after the new
/// one is verified and staged.
#[allow(clippy::too_many_arguments)]
pub fn update_one(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    item: &AddonUpdateItem,
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<InstallOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Update)?;
    let version = item
        .available_version
        .as_ref()
        .ok_or(AddonMutationError::NoUpdateAvailable)?;
    let folder = add_on_folder(server_dir, flavor)?;
    let primary =
        domain::modrinth_primary_file(&version.files).ok_or(AddonMutationError::NoPrimaryFile)?;

    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = dest_preserving_disabled_state(&folder, &primary.filename, item.is_enabled);
    let expected_checksum = primary
        .hashes
        .get("sha1")
        .map(|hex| ExpectedChecksum::sha1(hex.clone()));

    addon_store::install_verified_file(
        transport,
        fs,
        &primary.url,
        &version.version_number,
        expected_checksum.as_ref(),
        &dest,
    )?;

    if dest.file_name() != Some(std::ffi::OsStr::new(&item.filename)) {
        let old_path = folder.join(&item.filename);
        let _ = addon_store::remove_addon_jar(fs, &old_path);
    }

    let dependencies = addon_dependencies::install_required_dependencies(
        transport,
        fs,
        version,
        flavor,
        minecraft_version,
        server_dir,
        installed_mod_ids,
        should_cancel,
    );

    Ok(InstallOutcome {
        installed_path: dest,
        dependencies,
    })
}

/// One outcome per item attempted, in `items`' own order. A per-item
/// failure never aborts the batch — matching every other "update all" /
/// "install all dependencies" loop this phase has already built
/// (`addon_dependencies.rs`'s own per-dependency non-fatal loop).
pub struct BatchUpdateResult {
    pub filename: String,
    pub outcome: Result<InstallOutcome, AddonMutationError>,
}

#[allow(clippy::too_many_arguments)]
pub fn update_all(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    items: &[AddonUpdateItem],
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Vec<BatchUpdateResult> {
    items
        .iter()
        .filter(|item| item.available_version.is_some())
        .map(|item| {
            let outcome = update_one(
                transport,
                fs,
                server_dir,
                flavor,
                item,
                minecraft_version,
                installed_mod_ids,
                pack_managed,
                should_cancel,
            );
            BatchUpdateResult {
                filename: item.filename.clone(),
                outcome,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------
// Toggle / remove
// ---------------------------------------------------------------------

pub fn toggle(
    fs: &dyn FileSystem,
    path: &Path,
    pack_managed: bool,
) -> Result<PathBuf, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Toggle)?;
    Ok(addon_store::toggle_addon_jar(fs, path)?)
}

pub fn remove(
    fs: &dyn FileSystem,
    path: &Path,
    pack_managed: bool,
) -> Result<(), AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Remove)?;
    Ok(addon_store::remove_addon_jar(fs, path)?)
}

// ---------------------------------------------------------------------
// Manual Modrinth link (`ConfigServer.addon_links`)
// ---------------------------------------------------------------------

/// A user-supplied manual link between an on-disk jar and a Modrinth
/// project — no oracle mutation gate exists for this (it only ever writes
/// config), so no pack-managed check here; linking metadata, unlike an
/// install/remove/toggle/update, changes nothing on disk.
pub fn set_manual_addon_link(
    links: &mut HashMap<String, AddonLink>,
    project_id: &str,
    title: Option<String>,
    slug: Option<String>,
) {
    links.insert(
        project_id.to_string(),
        AddonLink {
            project_id: project_id.to_string(),
            title,
            slug,
            icon_url: None,
            provenance: AddonLinkProvenance::UserLinked,
            installed_version_id: None,
            installed_file_name: None,
            installed_hash: None,
            client_side: None,
            server_side: None,
            extra: Default::default(),
        },
    );
}

pub fn remove_addon_link(links: &mut HashMap<String, AddonLink>, project_id: &str) {
    links.remove(project_id);
}

// ---------------------------------------------------------------------
// Plugin-source set/remove (`ConfigServer.plugin_sources`)
// ---------------------------------------------------------------------

pub fn set_plugin_source(
    sources: &mut HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
    config: PluginSourceConfig,
) {
    addon_update::set_plugin_source(sources, jar_stem, config);
}

pub fn remove_plugin_source(
    sources: HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
) -> Option<HashMap<String, PluginSourceConfig>> {
    addon_update::remove_plugin_source(sources, jar_stem)
}

// ---------------------------------------------------------------------
// Plugin-source update (GitHub / Modrinth / Hangar / Direct — the
// `downloadPluginWithSourceCheck`/`downloadLatestForPlugin` path,
// distinct from the Modrinth-hash-based `update_one` above: this path
// updates a plugin the user manually pointed at a URL, not one
// Modrinth's own hash-identity resolved.)
// ---------------------------------------------------------------------

pub struct PluginSourceUpdateOutcome {
    pub installed_path: PathBuf,
    /// `Some(new_stem)` when the download's own final filename changed the
    /// jar's stem (`msc_domain::addon_update::plugin_source_rekey`) —
    /// `sources` has already been rekeyed to match by the time this
    /// returns; carried here only so a caller also holding
    /// `ConfigServer.addon_links`/other jar-stem-keyed state knows to
    /// follow the rename too.
    pub rekeyed_to: Option<String>,
}

/// Updates one plugin-source-linked plugin (`ConfigServer.plugin_sources`,
/// not `addon_links`) to its provider's current release, mirroring
/// `downloadPluginWithSourceCheck`/`downloadLatestForPlugin`
/// (`AppViewModel+PluginManagement.swift`). `sources` is rekeyed in place
/// when the download changes the jar's stem — the same "strip stale
/// prefix entries, write the new key" operation `set_plugin_source`
/// itself performs (P8.11's `plugin_source_rekey`/`set_plugin_source`).
#[allow(clippy::too_many_arguments)]
pub fn update_plugin_from_source(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    plugins_dir: &Path,
    jar_stem: &str,
    display_name: &str,
    is_enabled: bool,
    source: &PluginSourceConfig,
    minecraft_version: Option<&str>,
    loaders: &[String],
    pack_managed: bool,
    sources: &mut HashMap<String, PluginSourceConfig>,
) -> Result<PluginSourceUpdateOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Update)?;

    let (online_version, download_url, checksum) =
        resolve_source_download(transport, source, minecraft_version, loaders)?;

    let final_filename = addon_update::plugin_final_filename(
        &download_url,
        display_name,
        Some(online_version.as_str()),
    );

    fs.create_dir_all(plugins_dir)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;

    // Remove stale prior copies (both enabled and disabled) BEFORE writing
    // the new file, matching `downloadLatestForPlugin`'s own ordering
    // (`msc_domain::addon_update::stale_jars_to_remove`'s own doc).
    let existing_files: Vec<String> = fs
        .list(plugins_dir)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();
    for stale in addon_update::stale_jars_to_remove(&existing_files, display_name) {
        let _ = addon_store::remove_addon_jar(fs, &plugins_dir.join(stale));
    }

    let dest = dest_preserving_disabled_state(plugins_dir, &final_filename, is_enabled);
    addon_store::install_verified_file(
        transport,
        fs,
        &download_url,
        &online_version,
        checksum.as_ref(),
        &dest,
    )?;

    let rekeyed_to = addon_update::plugin_source_rekey(jar_stem, &final_filename);
    if let Some(new_stem) = &rekeyed_to
        && let Some(config) = sources.remove(jar_stem).or_else(|| Some(source.clone()))
    {
        addon_update::set_plugin_source(sources, new_stem, config);
    }
    Ok(PluginSourceUpdateOutcome {
        installed_path: dest,
        rekeyed_to,
    })
}

fn resolve_source_download(
    transport: &dyn AddonTransport,
    source: &PluginSourceConfig,
    minecraft_version: Option<&str>,
    loaders: &[String],
) -> Result<(String, String, Option<ExpectedChecksum>), AddonMutationError> {
    match addon_update::plugin_version_dispatch(source.source_type) {
        PluginVersionDispatch::DirectImmediate { .. } => {
            let (version, url) = domain::direct_dispatch(&source.url)?;
            Ok((version, url, None))
        }
        PluginVersionDispatch::FetchOnlineFirst => match source.source_type {
            PluginSourceKind::Github => {
                let (owner, repo) = plugin_source::parse_github(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let release = msc_infrastructure::addon_provider::github_latest_release(
                    transport, &owner, &repo,
                )?;
                let asset = domain::github_select_jar_asset(&release.assets)
                    .ok_or(AddonMutationError::NoPrimaryFile)?;
                Ok((asset.name.clone(), asset.browser_download_url.clone(), None))
            }
            PluginSourceKind::Hangar => {
                let (author, slug) = plugin_source::parse_hangar(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let (version, url) = msc_infrastructure::addon_provider::hangar_fetch_latest(
                    transport,
                    &author,
                    &slug,
                    minecraft_version,
                )?;
                Ok((version.name, url, None))
            }
            PluginSourceKind::Modrinth => {
                let slug = plugin_source::parse_modrinth(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let project =
                    msc_infrastructure::addon_provider::modrinth_project(transport, &slug)?;
                let versions = msc_infrastructure::addon_provider::modrinth_project_versions(
                    transport,
                    &project.slug,
                    loaders,
                    minecraft_version,
                )?;
                let best = versions
                    .first()
                    .ok_or(AddonMutationError::NoUpdateAvailable)?;
                let primary = domain::modrinth_primary_file(&best.files)
                    .ok_or(AddonMutationError::NoPrimaryFile)?;
                let checksum = primary
                    .hashes
                    .get("sha1")
                    .map(|hex| ExpectedChecksum::sha1(hex.clone()));
                Ok((best.version_number.clone(), primary.url.clone(), checksum))
            }
            PluginSourceKind::Direct => unreachable!("Direct is DirectImmediate above"),
        },
    }
}
