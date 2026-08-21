//! P8.18 amendment (flagged — outside this step's own declared `Files:`
//! list, which names only `msc-application`/`msc-infrastructure` files):
//! pack-format detection and manifest parsing, pure logic that belongs in
//! the domain crate per this workspace's own established layering, ported
//! from `CurseForgeModpack.swift` (`detectKind`/`isCurseForgeModpackManifest`/
//! `parseLoaderId`/`CurseForgeMetadata.from`/`manualDownloads`) and
//! `MrpackManifest`/`MrpackMetadata` (the equivalent `.mrpack` side),
//! against `fixtures/curseforge-modpack/` and `fixtures/modpack-pinning/`
//! (pre-existing Phase 6/7 characterization, not yet ported by any prior
//! step — confirmed by grep before writing this file).
//!
//! **Scope boundary, deliberately drawn:** `fixtures/modpack-pinning/`'s
//! own `forge-maven-*` cases (Forge's installer/build Maven-XML lookup)
//! and `fixtures/curseforge-modpack/`'s loader-*build*-resolution concerns
//! are NOT ported here — those resolve a pinned loader *version string* to
//! an actual downloadable *build*, which is Phase 7 provisioning's job
//! (`msc-application::provisioning`) once P8.21 wires a staged pack into
//! server creation, not modpack *inspection*. This module only reports the
//! pinned version *strings* a manifest declares, per P8.18's own "report
//! pinned Minecraft/loader versions" text — it never resolves or downloads
//! anything.
//!
//! **`ditto`'s mode-000 fixture needs no special handling here.** MSC 1
//! shells out to macOS's `ditto` because `/usr/bin/unzip` extracts a
//! stored-mode-000 entry as unreadable on disk before it can be read back.
//! This port never round-trips a manifest through disk at all —
//! [`msc_infrastructure::archive::read_entry_bytes`] decompresses straight
//! into memory — so an entry's stored Unix mode never gates whether its
//! *content* can be read; only extraction (`extract_zip`, unaffected —
//! see that function's own executable-bit handling) touches disk.

use std::collections::HashMap;

use serde_json::Value;

// ---------------------------------------------------------------------
// Format detection
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedPackKind {
    Modrinth,
    CurseForge,
    Unknown,
}

/// `CurseForgeModpack.detectKind` (source line ~140-170): Modrinth wins
/// when both markers are present in the same root
/// (`fixtures/curseforge-modpack/modrinth-wins-when-both-markers-present.json`).
/// `manifest_json` is `manifest.json`'s own decoded bytes when that entry
/// exists at all — `None` (not present) and `Some(bytes that don't parse
/// as a genuine CurseForge pack manifest)` both fall through to `Unknown`.
pub fn detect_kind(has_modrinth_index: bool, manifest_json: Option<&[u8]>) -> DetectedPackKind {
    if has_modrinth_index {
        return DetectedPackKind::Modrinth;
    }
    if let Some(bytes) = manifest_json
        && let Ok(text) = std::str::from_utf8(bytes)
        && is_curseforge_modpack_manifest(text)
    {
        return DetectedPackKind::CurseForge;
    }
    DetectedPackKind::Unknown
}

/// `CurseForgeModpack.isCurseForgeModpackManifest` (source line ~183):
/// malformed JSON, and JSON that parses but has no `manifestType ==
/// "minecraftModpack"`, both report `false` rather than throwing —
/// this is a classification probe, not a parse.
pub fn is_curseforge_modpack_manifest(json_text: &str) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(json_text) else {
        return false;
    };
    v.get("manifestType").and_then(Value::as_str) == Some("minecraftModpack")
}

// ---------------------------------------------------------------------
// Shared loader flavor + pinned-version-entry shape
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderFlavor {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
}

impl LoaderFlavor {
    pub fn label(self) -> &'static str {
        match self {
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedVersionEntry {
    pub mc_version: String,
    pub loader_version: Option<String>,
    pub build_label: Option<String>,
    /// An em dash (`—`, U+2014) between MC version and loader version —
    /// copied exactly from the oracle's own source assertion
    /// (`fixtures/curseforge-modpack/forge-manifest-metadata-pins-loader.json`'s
    /// own note), not a hyphen.
    pub id: String,
}

/// `MrpackMetadata.versionEntry`/`CurseForgeMetadata.versionEntry`, shared:
/// `None` when there's no pinned Minecraft version at all — a loader
/// without an MC version has nothing to pin a picker entry to
/// (`fixtures/modpack-pinning/version-entry-nil-when-no-minecraft-version.json`).
pub fn pinned_version_entry(
    minecraft_version: Option<&str>,
    loader_flavor: Option<LoaderFlavor>,
    loader_version: Option<&str>,
) -> Option<PinnedVersionEntry> {
    let mc = minecraft_version?;
    let build_label = match (loader_flavor, loader_version) {
        (Some(flavor), Some(v)) => Some(format!("{} {v}", flavor.label())),
        _ => None,
    };
    Some(PinnedVersionEntry {
        mc_version: mc.to_string(),
        loader_version: loader_version.map(str::to_string),
        build_label,
        id: format!("{mc}\u{2014}{}", loader_version.unwrap_or("")),
    })
}

// ---------------------------------------------------------------------
// CurseForge manifest.json
// ---------------------------------------------------------------------

#[derive(Debug)]
pub enum CurseForgeManifestError {
    MalformedManifest,
    UnknownLoader(String),
    MalformedLoaderId,
}

/// `CurseForgeModpack.parseLoaderId` (source line ~98-130): splits on the
/// FIRST `-` only (so a loader version that itself contains a hyphen,
/// e.g. `fabric-0.16.9-beta.1`, keeps its version intact —
/// `fixtures/curseforge-modpack/parse-loader-id-neoforge-with-hyphenated-version.json`).
/// A blank/whitespace-only id, or one with no `-` at all, is malformed;
/// a recognized-shape id whose loader name isn't one of the four known
/// flavors is `UnknownLoader`, not silently guessed.
pub fn parse_loader_id(loader_id: &str) -> Result<(LoaderFlavor, String), CurseForgeManifestError> {
    let trimmed = loader_id.trim();
    if trimmed.is_empty() {
        return Err(CurseForgeManifestError::MalformedLoaderId);
    }
    let (name, version) = trimmed
        .split_once('-')
        .ok_or(CurseForgeManifestError::MalformedLoaderId)?;
    let flavor = match name {
        "forge" => LoaderFlavor::Forge,
        "neoforge" => LoaderFlavor::NeoForge,
        "fabric" => LoaderFlavor::Fabric,
        "quilt" => LoaderFlavor::Quilt,
        other => return Err(CurseForgeManifestError::UnknownLoader(other.to_string())),
    };
    if version.is_empty() {
        return Err(CurseForgeManifestError::MalformedLoaderId);
    }
    Ok((flavor, version.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeManifestMetadata {
    pub name: String,
    pub version_id: String,
    pub minecraft_version: String,
    pub loader_flavor: Option<LoaderFlavor>,
    pub loader_version: Option<String>,
    pub overrides_folder: String,
}

impl CurseForgeManifestMetadata {
    pub fn version_entry(&self) -> PinnedVersionEntry {
        pinned_version_entry(
            Some(&self.minecraft_version),
            self.loader_flavor,
            self.loader_version.as_deref(),
        )
        .expect("minecraft_version is always Some here")
    }
}

/// `CurseForgeMetadata.from` (source line ~30-55): the *primary*
/// `modLoaders` entry (`primary == true`) wins; when none is marked
/// primary, the first entry is used; an empty/absent `modLoaders` list
/// pins no loader at all (loader_flavor/loader_version both `None`) rather
/// than erroring — only a loader id that IS present but unrecognized
/// throws (`fixtures/curseforge-modpack/malformed-loader-id-throws.json`).
pub fn parse_curseforge_metadata(
    manifest_json: &str,
) -> Result<CurseForgeManifestMetadata, CurseForgeManifestError> {
    let v: Value = serde_json::from_str(manifest_json)
        .map_err(|_| CurseForgeManifestError::MalformedManifest)?;
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let version_id = v
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let minecraft_version = v
        .pointer("/minecraft/version")
        .and_then(Value::as_str)
        .ok_or(CurseForgeManifestError::MalformedManifest)?
        .to_string();
    let overrides_folder = v
        .get("overrides")
        .and_then(Value::as_str)
        .unwrap_or("overrides")
        .to_string();

    let loaders = v
        .pointer("/minecraft/modLoaders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let primary = loaders
        .iter()
        .find(|l| l.get("primary").and_then(Value::as_bool) == Some(true))
        .or_else(|| loaders.first());
    let (loader_flavor, loader_version) =
        match primary.and_then(|l| l.get("id")).and_then(Value::as_str) {
            Some(id) => {
                let (flavor, version) = parse_loader_id(id)?;
                (Some(flavor), Some(version))
            }
            None => (None, None),
        };

    Ok(CurseForgeManifestMetadata {
        name,
        version_id,
        minecraft_version,
        loader_flavor,
        loader_version,
        overrides_folder,
    })
}

/// `CurseForgeModpack.manualDownloads` (source line ~191-210): a matching
/// project record supplies the real mod name and CurseForge-reported
/// website URL; an unmatched `modId` falls back to the blocked file's own
/// filename and a generated search link (proven to `contains("curseforge.com")`
/// by the oracle's own test, not an exact URL —
/// `fixtures/curseforge-modpack/manual-downloads-assembly.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualDownloadEntry {
    pub mod_name: String,
    pub file_name: String,
    pub project_page_url: String,
}

pub fn manual_downloads(
    blocked_files: &[crate::addon_provider::CurseForgeFile],
    projects: &[crate::addon_provider::CurseForgeMod],
) -> Vec<ManualDownloadEntry> {
    let by_id: HashMap<i64, &crate::addon_provider::CurseForgeMod> =
        projects.iter().map(|p| (p.id, p)).collect();
    blocked_files
        .iter()
        .map(|f| match by_id.get(&f.mod_id) {
            Some(p) => ManualDownloadEntry {
                mod_name: p.name.clone(),
                file_name: f.file_name.clone(),
                project_page_url: p.website_url().map(str::to_string).unwrap_or_else(|| {
                    format!("https://www.curseforge.com/minecraft/mc-mods/{}", p.slug)
                }),
            },
            None => ManualDownloadEntry {
                mod_name: f.file_name.clone(),
                file_name: f.file_name.clone(),
                project_page_url: format!(
                    "https://www.curseforge.com/minecraft/search?search={}",
                    urlencode(&f.file_name)
                ),
            },
        })
        .collect()
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

// ---------------------------------------------------------------------
// Modrinth modrinth.index.json
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrpackFileHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MrpackFileEntry {
    pub path: String,
    pub hashes: MrpackFileHashes,
    pub env: Option<crate::modpack::MrpackEnv>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone)]
pub struct MrpackManifest {
    pub name: String,
    pub version_id: String,
    pub game: String,
    /// Raw `dependencies` map (`"minecraft"`/`"forge"`/`"fabric-loader"`/
    /// `"neoforge"`/`"quilt-loader"` -> pinned version string), preserved
    /// as-is so an unrecognized future key isn't silently dropped.
    pub dependencies: HashMap<String, String>,
    pub files: Vec<MrpackFileEntry>,
}

#[derive(Debug)]
pub enum MrpackReadError {
    /// No `modrinth.index.json` entry in the archive at all.
    ManifestAbsent,
    /// The entry exists but isn't valid JSON, or is missing a required
    /// field.
    ManifestMalformed,
}

/// `AppViewModel.readMrpackManifest`'s decode half (the archive-open half
/// is `msc-infrastructure`'s job — see `modpacks.rs`'s own
/// `inspect_staged_archive`, which calls
/// [`msc_infrastructure::archive::read_entry_bytes`] and hands this
/// function the bytes).
pub fn parse_mrpack_manifest(manifest_json: &str) -> Result<MrpackManifest, MrpackReadError> {
    let v: Value =
        serde_json::from_str(manifest_json).map_err(|_| MrpackReadError::ManifestMalformed)?;
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let version_id = v
        .get("versionId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let game = v
        .get("game")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let dependencies: HashMap<String, String> = v
        .get("dependencies")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let files = v
        .get("files")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(parse_mrpack_file_entry).collect())
        .unwrap_or_default();

    Ok(MrpackManifest {
        name,
        version_id,
        game,
        dependencies,
        files,
    })
}

fn parse_mrpack_file_entry(v: &Value) -> Option<MrpackFileEntry> {
    let path = v.get("path")?.as_str()?.to_string();
    let hashes = MrpackFileHashes {
        sha1: v
            .pointer("/hashes/sha1")
            .and_then(Value::as_str)
            .map(str::to_string),
        sha512: v
            .pointer("/hashes/sha512")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    let env = v.get("env").map(|e| crate::modpack::MrpackEnv {
        client: e.get("client").and_then(Value::as_str).map(str::to_string),
        server: e.get("server").and_then(Value::as_str).map(str::to_string),
    });
    let downloads = v
        .get("downloads")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|d| d.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let file_size = v.get("fileSize").and_then(Value::as_u64).unwrap_or(0);
    Some(MrpackFileEntry {
        path,
        hashes,
        env,
        downloads,
        file_size,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrpackMetadata {
    pub minecraft_version: Option<String>,
    pub loader_flavor: Option<LoaderFlavor>,
    pub loader_version: Option<String>,
}

const MRPACK_LOADER_KEYS: &[(&str, LoaderFlavor)] = &[
    ("forge", LoaderFlavor::Forge),
    ("neoforge", LoaderFlavor::NeoForge),
    ("fabric-loader", LoaderFlavor::Fabric),
    ("quilt-loader", LoaderFlavor::Quilt),
];

/// `MrpackMetadata.from` (source line ~90-150): reads the pinned Minecraft
/// version and, from whichever known loader key is present in
/// `dependencies` (a real manifest carries at most one), the loader flavor
/// and its own pinned version. No loader key present at all is not an
/// error — a manifest can legitimately pin only a Minecraft version
/// (`fixtures/modpack-pinning/manifest-with-no-loader-has-nil-flavor.json`).
pub fn mrpack_metadata(manifest: &MrpackManifest) -> MrpackMetadata {
    let minecraft_version = manifest.dependencies.get("minecraft").cloned();
    let mut loader_flavor = None;
    let mut loader_version = None;
    for (key, flavor) in MRPACK_LOADER_KEYS {
        if let Some(v) = manifest.dependencies.get(*key) {
            loader_flavor = Some(*flavor);
            loader_version = Some(v.clone());
            break;
        }
    }
    MrpackMetadata {
        minecraft_version,
        loader_flavor,
        loader_version,
    }
}

impl MrpackMetadata {
    pub fn version_entry(&self) -> Option<PinnedVersionEntry> {
        pinned_version_entry(
            self.minecraft_version.as_deref(),
            self.loader_flavor,
            self.loader_version.as_deref(),
        )
    }
}
