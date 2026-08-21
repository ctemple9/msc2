//! Safe modpack inspection: identifies a staged `.mrpack`/CurseForge/
//! plain-JAR-ZIP archive, parses and validates its manifest, reports
//! pinned Minecraft/loader versions and CurseForge's D-027 manual-file
//! requirements, and extracts into an operation-owned staging tree with
//! P8.14's own traversal/symlink/size/count-safe `archive::extract_zip`.
//!
//! **Staged-upload redemption is the route layer's job (P8.24), not this
//! module's** — the same boundary `worlds.rs`'s `import_zip_as_new_slot`
//! already established for Phase 6 (its own doc: `source_zip_path` is a
//! caller-resolved path). [`inspect_staged_archive`] takes an already-
//! redeemed real file path; it never touches `POST /v1/staged-uploads`
//! itself, and this crate has no dependency on that primitive at all.
//!
//! **Inspection never mutates a real server.** No function here takes a
//! server directory, writes into one, or touches `ConfigServer` — the
//! only filesystem write this module performs is into `staging_root`, an
//! operation-owned temporary tree the caller supplies and owns (P8.19/
//! P8.20 read from it; nothing here decides a server exists to receive
//! it). "Cleans up expired or invalid uploads": [`inspect_staged_archive`]
//! removes its own operation-owned staging directory before returning any
//! error — a failed or unrecognized inspection leaves no residue under
//! `staging_root`. The staged-upload slot itself (the route layer's own
//! bounded temp storage, outside this crate) is a separate, already-built
//! primitive (Phase 6) this step doesn't touch.

use std::fmt;
use std::path::{Path, PathBuf};

use msc_domain::modpack_manifest::{
    self, CurseForgeManifestMetadata, DetectedPackKind, ManualDownloadEntry, MrpackManifest,
    PinnedVersionEntry,
};

use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::secret_store::SecretStore;

const MODRINTH_INDEX_ENTRY: &str = "modrinth.index.json";
const CURSEFORGE_MANIFEST_ENTRY: &str = "manifest.json";

#[derive(Debug)]
pub enum ModpackInspectionError {
    /// The staged file doesn't exist, or isn't a file.
    SourceMissing,
    /// Neither a `.mrpack`/CurseForge manifest nor any top-level `.jar`
    /// was found — nothing this function knows how to import.
    Unrecognized,
    Archive(ArchiveError),
    ManifestMalformed(String),
    Io(std::io::Error),
}

impl fmt::Display for ModpackInspectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceMissing => write!(f, "staged archive not found"),
            Self::Unrecognized => write!(
                f,
                "archive is not a recognized modpack (.mrpack, CurseForge, or a plain jar collection)"
            ),
            Self::Archive(e) => write!(f, "{e}"),
            Self::ManifestMalformed(m) => write!(f, "{m}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ModpackInspectionError {}

/// Which of the three recognized shapes this archive is, plus its own
/// parsed manifest (for `PlainJarZip`, just the top-level jar names — no
/// manifest exists to parse).
#[derive(Debug)]
pub enum InspectedFormat {
    Mrpack(MrpackManifest),
    CurseForge(CurseForgeManifestMetadata),
    /// New MSC 2 classification, not an oracle port — `detect_kind`'s own
    /// literal port reports this shape as `Unknown` (`fixtures/
    /// curseforge-modpack/detect-kind-plain-zip-is-unknown.json`), the
    /// same as a genuinely unrecognized archive. `rolling-plan.md`'s own
    /// P8.18 text explicitly asks for "supported plain-JAR ZIP" as a
    /// third recognized shape (a client uploading a bundle of loose jars,
    /// meaningful for a local multi-jar install), so this inspection layer
    /// distinguishes it from a genuinely empty/unrecognized archive rather
    /// than reporting both identically.
    PlainJarZip {
        jar_entries: Vec<String>,
    },
}

#[derive(Debug)]
pub struct ModpackInspection {
    pub format: InspectedFormat,
    pub pinned_version: Option<PinnedVersionEntry>,
    /// D-027's manual-file (author-blocked CurseForge file) list — always
    /// empty for `Mrpack`/`PlainJarZip`. Empty for `CurseForge` too when
    /// either no manifest-declared file is blocked, or the CurseForge API
    /// key isn't configured (`curseforge_lookup_available` distinguishes
    /// the two: `false` means this list is simply unknown, not "nothing
    /// blocked").
    pub manual_downloads: Vec<ManualDownloadEntry>,
    pub curseforge_lookup_available: bool,
    /// The operation-owned directory this archive was extracted into.
    pub staged_dir: PathBuf,
}

/// Identifies `archive_path`'s format and parses its manifest — no
/// extraction, no network call, no filesystem write beyond reading the
/// archive itself. Exposed separately from [`inspect_staged_archive`] so a
/// caller that only needs the format (e.g. deciding whether to even show
/// the CurseForge manual-file step) doesn't pay for extraction it won't use.
pub fn identify_format(
    fs: &dyn FileSystem,
    archive_path: &Path,
) -> Result<InspectedFormat, ModpackInspectionError> {
    if !matches!(fs.stat(archive_path), Ok(m) if m.is_file) {
        return Err(ModpackInspectionError::SourceMissing);
    }
    let entries =
        archive::list_entry_names(archive_path).map_err(ModpackInspectionError::Archive)?;
    let has_modrinth_index = entries.iter().any(|n| n == MODRINTH_INDEX_ENTRY);
    let manifest_bytes = if entries.iter().any(|n| n == CURSEFORGE_MANIFEST_ENTRY) {
        archive::read_entry_bytes(archive_path, CURSEFORGE_MANIFEST_ENTRY)
            .map_err(ModpackInspectionError::Archive)?
    } else {
        None
    };

    match modpack_manifest::detect_kind(has_modrinth_index, manifest_bytes.as_deref()) {
        DetectedPackKind::Modrinth => {
            let bytes = archive::read_entry_bytes(archive_path, MODRINTH_INDEX_ENTRY)
                .map_err(ModpackInspectionError::Archive)?
                .ok_or(ModpackInspectionError::Unrecognized)?;
            let text = String::from_utf8(bytes)
                .map_err(|e| ModpackInspectionError::ManifestMalformed(e.to_string()))?;
            let manifest = modpack_manifest::parse_mrpack_manifest(&text).map_err(|_| {
                ModpackInspectionError::ManifestMalformed(format!(
                    "{MODRINTH_INDEX_ENTRY} is not valid"
                ))
            })?;
            Ok(InspectedFormat::Mrpack(manifest))
        }
        DetectedPackKind::CurseForge => {
            let text = String::from_utf8(
                manifest_bytes.expect("CurseForge detection requires manifest bytes"),
            )
            .map_err(|e| ModpackInspectionError::ManifestMalformed(e.to_string()))?;
            let metadata = modpack_manifest::parse_curseforge_metadata(&text).map_err(|_| {
                ModpackInspectionError::ManifestMalformed(format!(
                    "{CURSEFORGE_MANIFEST_ENTRY} is not valid"
                ))
            })?;
            Ok(InspectedFormat::CurseForge(metadata))
        }
        DetectedPackKind::Unknown => {
            let jar_entries: Vec<String> = entries
                .into_iter()
                .filter(|n| n.to_lowercase().ends_with(".jar") && !n.contains('/'))
                .collect();
            if jar_entries.is_empty() {
                Err(ModpackInspectionError::Unrecognized)
            } else {
                Ok(InspectedFormat::PlainJarZip { jar_entries })
            }
        }
    }
}

/// Identifies `archive_path`, extracts it into a fresh subdirectory of
/// `staging_root` (P8.14's `archive::extract_zip` — traversal/symlink/
/// count/size-safe by construction), and — for a CurseForge pack, when a
/// CurseForge API key is configured — resolves which manifest-declared
/// files are author-blocked (D-027's pending list). On any error, the
/// staging subdirectory this call itself created is removed before
/// returning.
pub fn inspect_staged_archive(
    fs: &dyn FileSystem,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    archive_path: &Path,
    staging_root: &Path,
    operation_id: &str,
) -> Result<ModpackInspection, ModpackInspectionError> {
    let format = identify_format(fs, archive_path)?;

    let staged_dir = staging_root.join(operation_id);
    let result = extract_and_enrich(fs, transport, secrets, archive_path, &staged_dir, format);
    if result.is_err() {
        let _ = fs.remove(&staged_dir);
    }
    result
}

fn extract_and_enrich(
    fs: &dyn FileSystem,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    archive_path: &Path,
    staged_dir: &Path,
    format: InspectedFormat,
) -> Result<ModpackInspection, ModpackInspectionError> {
    fs.create_dir_all(staged_dir)
        .map_err(ModpackInspectionError::Io)?;
    archive::extract_zip(archive_path, staged_dir).map_err(ModpackInspectionError::Archive)?;

    let pinned_version = match &format {
        InspectedFormat::Mrpack(manifest) => {
            modpack_manifest::mrpack_metadata(manifest).version_entry()
        }
        InspectedFormat::CurseForge(metadata) => Some(metadata.version_entry()),
        InspectedFormat::PlainJarZip { .. } => None,
    };

    let (manual_downloads, curseforge_lookup_available) = match &format {
        InspectedFormat::CurseForge(_) => {
            resolve_manual_downloads(transport, secrets, staged_dir, fs)
        }
        _ => (Vec::new(), false),
    };

    Ok(ModpackInspection {
        format,
        pinned_version,
        manual_downloads,
        curseforge_lookup_available,
        staged_dir: staged_dir.to_path_buf(),
    })
}

/// Re-reads the already-extracted `manifest.json`'s own `files[]` list,
/// fetches those file ids through the CurseForge API, and reports which
/// ones have no `downloadUrl` (author-blocked, D-027's own pending-file
/// shape) with resolved mod names via a second CurseForge call. Missing
/// API key degrades to an honest "not available" (`curseforge_lookup_available
/// = false`) rather than failing the whole inspection — matching this
/// step's own "inspection never mutates a server" scope: a missing key
/// blocks *import* (`fixtures/modpack-import/
/// curseforge-missing-api-key-stops-import-before-any-file-resolution.json`,
/// P8.20's job), not read-only inspection.
fn resolve_manual_downloads(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    staged_dir: &Path,
    fs: &dyn FileSystem,
) -> (Vec<ManualDownloadEntry>, bool) {
    let Ok(manifest_bytes) = fs.read(&staged_dir.join(CURSEFORGE_MANIFEST_ENTRY)) else {
        return (Vec::new(), false);
    };
    let Ok(text) = String::from_utf8(manifest_bytes) else {
        return (Vec::new(), false);
    };
    let Ok(manifest_json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (Vec::new(), false);
    };
    let file_ids: Vec<i64> = manifest_json
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| f.get("fileID").and_then(|v| v.as_i64()))
                .collect()
        })
        .unwrap_or_default();
    if file_ids.is_empty() {
        return (Vec::new(), true);
    }

    let Ok(files) =
        msc_infrastructure::addon_provider::curseforge_files(transport, secrets, &file_ids)
    else {
        return (Vec::new(), false);
    };
    let blocked: Vec<_> = files
        .into_iter()
        .filter(|f| f.download_url.is_none())
        .collect();
    if blocked.is_empty() {
        return (Vec::new(), true);
    }

    let mod_ids: Vec<i64> = {
        let mut ids: Vec<i64> = blocked.iter().map(|f| f.mod_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let projects =
        msc_infrastructure::addon_provider::curseforge_mods(transport, secrets, &mod_ids)
            .unwrap_or_default();

    (
        modpack_manifest::manual_downloads(&blocked, &projects),
        true,
    )
}
