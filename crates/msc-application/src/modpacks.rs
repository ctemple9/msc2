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

//!
//! # P8.19: transactional `.mrpack` import
//!
//! [`import_mrpack`] builds on this same module's own [`inspect_staged_archive`]
//! (P8.18): given the manifest and staged directory that call already
//! produced, it downloads every server-relevant manifest file (verifying
//! each against its own declared hash — new agent-owned safety, closing
//! `fixtures/modpack-archive-safety/manifest-declares-per-file-hashes-but-download-never-verifies-them.json`'s
//! gap, since the oracle never checks these hashes at all), merges
//! `overrides/` then `server-overrides/` (server-overrides wins on
//! conflict, matching `fixtures/modpack-import/
//! overrides-copied-before-server-overrides-so-server-overrides-wins-on-conflict.json`),
//! and classifies the resulting override jars.
//!
//! **Scope boundary, decided and documented (not silently assumed):**
//! per this crate's own P8.1 finding (`phase8-scope.md`), MSC 1 has no
//! pack-driven server-*creation* primitive at all — `importModpack` always
//! targets an already-existing `ConfigServer`. [`import_mrpack`] preserves
//! that boundary: it operates against an already-registered `server_dir`,
//! the same way the oracle does; wiring a brand-new, not-yet-published
//! server around this function is P8.21's own explicit job, not this
//! step's.
//!
//! **"Restores the exact prior tree" — implemented as "rolls back every
//! file THIS call itself wrote," not a full pre-existing-content
//! snapshot/restore.** The oracle's own `mergeDirectory` has no conflict
//! detection or backup of what it overwrites either
//! (`fixtures/modpack-archive-safety/
//! merge-directory-unconditionally-overwrites-destination-no-conflict-detection.json`)
//! — preserved as-is here, deliberately, rather than inventing a new
//! safety property the working exit criteria don't ask this specific step
//! to add (unlike the per-file hash check above, which they do ask for).
//! What IS new here: every path this call creates or overwrites is
//! tracked, and on cancellation the newly-written files are removed
//! (`addon_dependencies.rs`'s own established rollback shape). Content
//! this import overwrote — a `server-overrides` entry replacing an
//! existing file with the same relative path — is not restorable to its
//! pre-import bytes by this function; that gap is called out explicitly
//! here rather than claimed as covered.
//!
//! **Tier 3 (embedded-jar `environment`) client-only classification for
//! override jars is not built by this step**, flagged honestly rather
//! than silently dropped: `classify_override_jars` runs Tier 0 (hardcoded
//! blocklist) then Tier 2 (Modrinth `server_side`, via the same
//! hash-identify + batched-project-fetch shape `addon_updates.rs`/
//! `addon_dependencies.rs` already established), but never reads an
//! override jar's own embedded `fabric.mod.json`/`mods.toml`
//! `environment` field as the Tier 3 fallback `msc_domain::modpack::
//! client_only_reason`'s own signature supports — Tier 2 already covers
//! the common case (an override jar published on Modrinth), and building
//! a second embedded-metadata reader distinct from
//! `add_on_inventory.rs`'s existing mod-id/name/version one was out of
//! reach for this step's own time budget. A jar with no Modrinth hash hit
//! and a client-only embedded manifest will not be auto-disabled by this
//! function today.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack;
use msc_domain::modpack_manifest::{
    self, CurseForgeManifestMetadata, DetectedPackKind, ManualDownloadEntry, MrpackFileEntry,
    MrpackManifest, PinnedVersionEntry,
};

use msc_infrastructure::addon_provider::{self as provider, AddonTransport};
use msc_infrastructure::addon_store::{self, DisableOutcome};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::download_staging::{ExpectedChecksum, sha512_hex};
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
    /// Files supplied by the archive's `overrides/` and `server-overrides/`
    /// trees, counted separately because they are not manifest downloads.
    pub override_file_count: usize,
    /// The operation-owned directory this archive was extracted into.
    pub staged_dir: PathBuf,
}

/// One file the import could not install automatically. The operation keeps
/// this structured record until the user uploads the exact JAR or chooses to
/// skip it; the client-facing Overview note is only a readable projection.
#[derive(Debug, Clone)]
pub struct UnresolvedModpackFile {
    pub file_id: String,
    pub file_name: String,
    pub project_name: String,
    pub provider: String,
    pub reason: String,
    pub project_url: Option<String>,
    pub pending: crate::curseforge_manual::PendingManualFile,
    pub expected_sha512: Option<String>,
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

    let override_file_count = match &format {
        InspectedFormat::Mrpack(_) | InspectedFormat::CurseForge(_) => {
            count_files(fs, &staged_dir.join("overrides"))
                + count_files(fs, &staged_dir.join("server-overrides"))
        }
        InspectedFormat::PlainJarZip { .. } => 0,
    };

    Ok(ModpackInspection {
        format,
        pinned_version,
        manual_downloads,
        curseforge_lookup_available,
        override_file_count,
        staged_dir: staged_dir.to_path_buf(),
    })
}

fn count_files(fs: &dyn FileSystem, directory: &Path) -> usize {
    let Ok(entries) = fs.list(directory) else {
        return 0;
    };
    entries
        .into_iter()
        .map(|entry| match fs.stat(&entry) {
            Ok(metadata) if metadata.is_file => 1,
            Ok(metadata) if metadata.is_dir => count_files(fs, &entry),
            _ => 0,
        })
        .sum()
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

// ---------------------------------------------------------------------
// P8.19: transactional `.mrpack` import (see this module's own doc)
// ---------------------------------------------------------------------

#[derive(Debug)]
pub enum MrpackImportError {
    PackManaged,
    NoAddOnKind,
}

impl fmt::Display for MrpackImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackManaged => write!(f, "this server is managed by a different modpack"),
            Self::NoAddOnKind => write!(f, "this server flavor has no add-on folder"),
        }
    }
}

impl std::error::Error for MrpackImportError {}

#[derive(Debug, Default)]
pub struct MrpackImportReport {
    /// Manifest-declared files successfully downloaded and verified.
    pub installed_files: Vec<PathBuf>,
    /// `(manifest path, reason)` — a mirror-exhausted or checksum-mismatched
    /// file. Non-fatal to the batch, matching `importModpack`'s own
    /// log-and-continue shape (`fixtures/modpack-import/
    /// file-download-all-mirrors-fail-recorded-in-failed-list-loop-continues-to-next-file.json`).
    pub failed_files: Vec<(String, String)>,
    /// Failed downloads are actionable rather than silently lost. The route
    /// exposes these through the same upload/skip checkpoint as blocked
    /// CurseForge files.
    pub unresolved_files: Vec<UnresolvedModpackFile>,
    /// Override jars this import disabled as client-only (Tier 0/Tier 2 —
    /// see this module's own doc on the Tier 3 gap).
    pub disabled_client_only_overrides: Vec<PathBuf>,
    pub pack_name: String,
    pub pack_version: String,
    /// `true` if `should_cancel` fired before the file-download phase
    /// finished — every file this call itself wrote has already been
    /// rolled back when this is `true`.
    pub cancelled: bool,
}

/// Imports `manifest`'s server-relevant files and override tree into
/// `server_dir` — see this module's own doc for the create-vs-existing-
/// server scope boundary and what "restores the exact prior tree" means
/// here specifically.
#[allow(clippy::too_many_arguments)]
pub fn import_mrpack(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    manifest: &MrpackManifest,
    staged_dir: &Path,
    home_dir: &Path,
    pack_managed: bool,
    explicit_replace_intent: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<MrpackImportReport, MrpackImportError> {
    if modpack::pack_replace_refused(pack_managed, explicit_replace_intent) {
        return Err(MrpackImportError::PackManaged);
    }
    let add_on_kind = flavor.add_on_kind().ok_or(MrpackImportError::NoAddOnKind)?;

    let mut report = MrpackImportReport {
        pack_name: manifest.name.clone(),
        pack_version: manifest.version_id.clone(),
        ..Default::default()
    };
    let mut written: Vec<PathBuf> = Vec::new();

    for file in &manifest.files {
        if should_cancel() {
            report.cancelled = true;
            break;
        }
        install_one_manifest_file(
            transport,
            fs,
            server_dir,
            home_dir,
            file,
            &manifest.version_id,
            &mut report,
            &mut written,
        );
    }

    if report.cancelled {
        rollback_written(fs, &written);
        return Ok(report);
    }

    let mut skipped_client_only_overrides = Vec::new();
    for override_root in ["overrides", "server-overrides"] {
        let source_folder = staged_dir
            .join(override_root)
            .join(add_on_kind.folder_name());
        skipped_client_only_overrides.extend(client_only_override_paths(
            transport,
            fs,
            &source_folder,
        ));
    }
    merge_directory_into_excluding(
        fs,
        &staged_dir.join("overrides"),
        server_dir,
        &mut written,
        &skipped_client_only_overrides,
    );
    merge_directory_into_excluding(
        fs,
        &staged_dir.join("server-overrides"),
        server_dir,
        &mut written,
        &skipped_client_only_overrides,
    );

    let add_on_folder = server_dir.join(add_on_kind.folder_name());
    report.disabled_client_only_overrides = classify_override_jars(transport, fs, &add_on_folder);

    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn install_one_manifest_file(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    home_dir: &Path,
    file: &MrpackFileEntry,
    version_label: &str,
    report: &mut MrpackImportReport,
    written: &mut Vec<PathBuf>,
) {
    if modpack::is_manifest_server_unsupported(file.env.as_ref()) {
        return; // Tier 1 pre-filter: client-only, skipped, not a failure.
    }
    let dest = match addon_store::resolve_pack_file_dest(fs, server_dir, &file.path, home_dir) {
        Ok(d) => d,
        Err(e) => {
            let reason = e.to_string();
            report
                .failed_files
                .push((file.path.clone(), reason.clone()));
            record_mrpack_unresolved(report, fs, file, server_dir, home_dir, reason, None);
            return;
        }
    };
    if let Some(parent) = dest.parent()
        && fs.create_dir_all(parent).is_err()
    {
        let reason = "could not create destination directory".to_string();
        report
            .failed_files
            .push((file.path.clone(), reason.clone()));
        record_mrpack_unresolved_at_dest(report, file, dest, reason, None);
        return;
    }

    let expected = file
        .hashes
        .sha512
        .as_ref()
        .map(|h| ExpectedChecksum::sha512(h.clone()))
        .or_else(|| {
            file.hashes
                .sha1
                .as_ref()
                .map(|h| ExpectedChecksum::sha1(h.clone()))
        });

    for url in &file.downloads {
        match addon_store::install_verified_file(
            transport,
            fs,
            url,
            version_label,
            expected.as_ref(),
            &dest,
        ) {
            Ok(_) => {
                written.push(dest.clone());
                report.installed_files.push(dest);
                return;
            }
            Err(_) => continue,
        }
    }
    let reason = "all mirrors failed or checksum mismatch".to_string();
    report
        .failed_files
        .push((file.path.clone(), reason.clone()));
    record_mrpack_unresolved_at_dest(report, file, dest, reason, file.downloads.first().cloned());
}

fn record_mrpack_unresolved(
    report: &mut MrpackImportReport,
    fs: &dyn FileSystem,
    file: &MrpackFileEntry,
    server_dir: &Path,
    home_dir: &Path,
    reason: String,
    project_url: Option<String>,
) {
    let Ok(dest) = addon_store::resolve_pack_file_dest(fs, server_dir, &file.path, home_dir) else {
        return;
    };
    record_mrpack_unresolved_at_dest(report, file, dest, reason, project_url);
}

fn record_mrpack_unresolved_at_dest(
    report: &mut MrpackImportReport,
    file: &MrpackFileEntry,
    dest: PathBuf,
    reason: String,
    project_url: Option<String>,
) {
    report.unresolved_files.push(UnresolvedModpackFile {
        file_id: format!("mrpack:{}", file.path),
        file_name: Path::new(&file.path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&file.path)
            .to_string(),
        project_name: file.path.clone(),
        provider: "Modrinth".to_string(),
        reason,
        project_url,
        pending: crate::curseforge_manual::PendingManualFile {
            project_id: 0,
            file_id: 0,
            expected_file_name: Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&file.path)
                .to_string(),
            expected_byte_size: file.file_size,
            dest,
        },
        expected_sha512: file.hashes.sha512.clone(),
    });
}

fn rollback_written(fs: &dyn FileSystem, written: &[PathBuf]) {
    for path in written.iter().rev() {
        let _ = fs.remove(path);
    }
}

/// `mergeDirectory(from:to:)` (source, per `fixtures/modpack-archive-safety/
/// merge-directory-unconditionally-overwrites-destination-no-conflict-detection.json`):
/// unconditionally overwrites any existing destination file, no conflict
/// detection — preserved deliberately, this step's own working exit text
/// only asks for hash verification on manifest-declared *downloads*, not a
/// new merge-conflict policy. A missing source directory (no `overrides/`
/// or no `server-overrides/` in this particular archive) is silently
/// skipped, not an error (`fixtures/modpack-import/
/// missing-overrides-folder-in-archive-silently-skipped-not-an-error.json`).
fn merge_directory_into_excluding(
    fs: &dyn FileSystem,
    src_dir: &Path,
    dest_root: &Path,
    written: &mut Vec<PathBuf>,
    excluded: &[PathBuf],
) {
    if !matches!(fs.stat(src_dir), Ok(m) if m.is_dir) {
        return;
    }
    merge_dir_recursive(fs, src_dir, dest_root, written, excluded);
}

fn merge_dir_recursive(
    fs: &dyn FileSystem,
    src_dir: &Path,
    dest_dir: &Path,
    written: &mut Vec<PathBuf>,
    excluded: &[PathBuf],
) {
    if fs.create_dir_all(dest_dir).is_err() {
        return;
    }
    let Ok(entries) = fs.list(src_dir) else {
        return;
    };
    for entry in entries {
        if excluded.contains(&entry) {
            continue;
        }
        let Some(name) = entry.file_name() else {
            continue;
        };
        let dest = dest_dir.join(name);
        let Ok(meta) = fs.stat(&entry) else {
            continue;
        };
        if meta.is_dir {
            merge_dir_recursive(fs, &entry, &dest, written, excluded);
        } else if let Ok(bytes) = fs.read(&entry)
            && fs.write(&dest, &bytes).is_ok()
        {
            written.push(dest);
        }
    }
}

/// Identifies client-only jars in an archive override folder before that
/// folder is merged into the server. The same two-tier rules used for the
/// post-merge classifier are shared here so a client-only override never
/// needs to be written into `mods/` and renamed afterward.
fn client_only_override_paths(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    add_on_folder: &Path,
) -> Vec<PathBuf> {
    let Ok(entries) = fs.list(add_on_folder) else {
        return Vec::new();
    };

    let mut client_only = Vec::new();
    let mut remaining: Vec<(PathBuf, String)> = Vec::new();
    for path in entries {
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if !name.to_lowercase().ends_with(".jar") {
            continue;
        }
        let stem = name.trim_end_matches(".jar");
        if modpack::known_client_only_reason(stem).is_some() {
            client_only.push(path);
        } else {
            remaining.push((path, name));
        }
    }
    if remaining.is_empty() {
        return client_only;
    }

    let mut hash_to_path: HashMap<String, PathBuf> = HashMap::new();
    let mut hashes: Vec<String> = Vec::new();
    for (path, _) in &remaining {
        if let Ok(bytes) = fs.read(path) {
            let hash = sha512_hex(&bytes);
            hash_to_path.insert(hash.clone(), path.clone());
            hashes.push(hash);
        }
    }
    let Ok(identify) = provider::modrinth_versions_from_hashes(transport, &hashes) else {
        return client_only;
    };
    if identify.is_empty() {
        return client_only;
    }
    let project_ids: Vec<String> = identify.values().map(|v| v.project_id.clone()).collect();
    let Ok(projects) = provider::modrinth_projects(transport, &project_ids) else {
        return client_only;
    };
    let by_id: HashMap<&str, &serde_json::Value> = projects
        .iter()
        .filter_map(|p| p.get("id").and_then(|v| v.as_str()).map(|id| (id, p)))
        .collect();

    for (hash, path) in &hash_to_path {
        let Some(version) = identify.get(hash) else {
            continue;
        };
        let Some(project) = by_id.get(version.project_id.as_str()) else {
            continue;
        };
        let server_side = project.get("server_side").and_then(|v| v.as_str());
        let title = project.get("title").and_then(|v| v.as_str());
        if modpack::client_only_reason(server_side, title, None).is_some() {
            client_only.push(path.clone());
        }
    }
    client_only
}

/// Tier 0 (hardcoded blocklist) then Tier 2 (Modrinth `server_side`, via a
/// hash-identify + batched 100-id project fetch — this module's own doc
/// explains why Tier 3 isn't built here) — an override jar Tier 0 already
/// disabled is never also hash-identified.
fn classify_override_jars(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    add_on_folder: &Path,
) -> Vec<PathBuf> {
    client_only_override_paths(transport, fs, add_on_folder)
        .into_iter()
        .filter_map(|path| {
            let mut disabled = Vec::new();
            disable_override_jar(fs, &path, &mut disabled);
            disabled.into_iter().next()
        })
        .collect()
}

fn disable_override_jar(fs: &dyn FileSystem, path: &Path, disabled: &mut Vec<PathBuf>) {
    if let Ok(DisableOutcome::Disabled(p)) = addon_store::disable_for_classification(fs, path) {
        disabled.push(p);
    }
}

/// A blocked CurseForge file may still have an exact, compatible Modrinth
/// counterpart. MSC only uses that route when the project title matches
/// exactly after normalization and the selected Modrinth file has the same
/// filename; a merely similar search result is left for the user to review.
struct ModrinthRecoveryRequest<'a> {
    project_name: Option<&'a str>,
    expected_file_name: &'a str,
}

#[derive(Clone)]
enum ModrinthRecoveryMatch {
    Server {
        download_url: String,
        version_number: String,
    },
    ClientOnly,
}

enum ModrinthRecoveryOutcome {
    Installed(PathBuf),
    ClientOnly,
}

fn find_confident_modrinth_match(
    transport: &dyn AddonTransport,
    flavor: JavaServerFlavor,
    minecraft_version: &str,
    request: ModrinthRecoveryRequest<'_>,
) -> Option<ModrinthRecoveryMatch> {
    let project_name = request
        .project_name
        .filter(|name| !name.trim().is_empty())?;
    let loaders: Vec<String> = flavor
        .modrinth_loader_facets()
        .iter()
        .map(|loader| loader.to_string())
        .collect();
    let Ok(search) = provider::modrinth_search(
        transport,
        project_name,
        "mod",
        &loaders,
        Some(minecraft_version),
        10,
        0,
    ) else {
        return None;
    };
    let project_id =
        crate::addon_updates::find_confident_dependency_match(project_name, &search.hits)?;
    let hit = search
        .hits
        .iter()
        .find(|hit| hit.project_id == project_id)?;
    let Ok(versions) = provider::modrinth_project_versions(
        transport,
        &project_id,
        &loaders,
        Some(minecraft_version),
    ) else {
        return None;
    };
    let expected_lower = request.expected_file_name.to_ascii_lowercase();
    let (download_url, version_number) = versions.iter().find_map(|version| {
        if !version.game_versions.iter().any(|v| v == minecraft_version)
            || !version
                .loaders
                .iter()
                .any(|loader| loaders.iter().any(|wanted| wanted == loader))
        {
            return None;
        }
        let file = msc_domain::addon_provider::modrinth_primary_file(&version.files)?;
        (file.filename.to_ascii_lowercase() == expected_lower)
            .then(|| (file.url.clone(), version.version_number.clone()))
    })?;
    if hit.is_client_only() {
        return Some(ModrinthRecoveryMatch::ClientOnly);
    }
    Some(ModrinthRecoveryMatch::Server {
        download_url,
        version_number,
    })
}

fn install_confident_modrinth_match(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    add_on_folder: &Path,
    expected_file_name: &str,
    version_label: &str,
    matched: ModrinthRecoveryMatch,
) -> Option<ModrinthRecoveryOutcome> {
    let ModrinthRecoveryMatch::Server {
        download_url,
        version_number,
    } = matched
    else {
        return Some(ModrinthRecoveryOutcome::ClientOnly);
    };
    if fs.create_dir_all(add_on_folder).is_err() {
        return None;
    }
    let destination = add_on_folder.join(expected_file_name);
    addon_store::install_verified_file(
        transport,
        fs,
        &download_url,
        if version_number.is_empty() {
            version_label
        } else {
            &version_number
        },
        None,
        &destination,
    )
    .is_ok()
    .then_some(ModrinthRecoveryOutcome::Installed(destination))
}

// ---------------------------------------------------------------------
// P8.20: transactional CurseForge import
// ---------------------------------------------------------------------

/// `CurseForgeAPI.post`'s own missing-key guard (`msc_domain::addon_provider::
/// curseforge_require_api_key`, surfaced through `curseforge_files`) means
/// this whole import can't resolve a single file without a key — unlike
/// P8.18's inspection (which degrades to `curseforge_lookup_available =
/// false` and keeps going), `import_curseforge` stops before ANY file
/// resolution when the key is missing, matching `fixtures/modpack-import/
/// curseforge-missing-api-key-stops-import-before-any-file-resolution.json`
/// exactly: inspection is read-only and can afford to say "unknown";
/// import is a real mutation and a half-resolved pack is worse than a
/// clean, honest refusal.
#[derive(Debug)]
pub enum CurseForgeImportError {
    PackManaged,
    NoAddOnKind,
    MissingApiKey,
    /// A file-resolution call itself failed (network, malformed response,
    /// non-key-related provider error) before any file was downloaded —
    /// stops the import the same way a missing key does, for the same
    /// "half-resolved is worse than a clean refusal" reason.
    Provider(String),
}

impl fmt::Display for CurseForgeImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackManaged => write!(f, "this server is managed by a different modpack"),
            Self::NoAddOnKind => write!(f, "this server flavor has no add-on folder"),
            Self::MissingApiKey => write!(f, "no CurseForge API key is configured"),
            Self::Provider(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CurseForgeImportError {}

#[derive(Debug, Default)]
pub struct CurseForgeImportReport {
    pub installed_files: Vec<PathBuf>,
    /// `(fileID, reason)` — a resolvable file whose download itself
    /// failed (network error, non-2xx status). Distinct from
    /// `blocked_files`: this is a real failure, that's an expected,
    /// non-fatal pending state.
    pub failed_files: Vec<(i64, String)>,
    /// Author-blocked files (`downloadUrl == null`), collected separately
    /// from `failed_files` and never counted as a failure
    /// (`fixtures/modpack-import/
    /// curseforge-blocked-file-collected-separately-from-failed-not-counted-as-a-failure.json`)
    /// — D-027's own pending list, ready for `curseforge_manual::
    /// complete_pending_file` once the client uploads each one.
    pub blocked_files: Vec<crate::curseforge_manual::PendingManualFile>,
    /// Blocked and failed files, enriched with provider links and a safe
    /// upload target for the recovery sheet.
    pub unresolved_files: Vec<UnresolvedModpackFile>,
    /// Files recovered from an exact compatible Modrinth match after the
    /// CurseForge download was unavailable. These are kept separate from
    /// direct CurseForge downloads so the completion summary can explain
    /// where each installed file came from.
    pub recovered_modrinth_files: Vec<PathBuf>,
    pub disabled_client_only_overrides: Vec<PathBuf>,
    pub pack_name: String,
    pub pack_version: String,
    pub cancelled: bool,
}

/// Imports a CurseForge pack's server files and override tree into
/// `server_dir`, resolving every manifest-declared file through the
/// CurseForge API first (`curseforge_files`, batched at
/// [`msc_infrastructure::addon_provider::MAX_BATCH_SIZE`]) before
/// downloading any of them — matching `CurseForgeAPI`'s own
/// resolve-then-download shape. Author-blocked files never fail the
/// import; they're reported in `blocked_files` for the D-027 completion
/// flow (`curseforge_manual.rs`) instead. See this module's own P8.19 doc
/// for the same create-vs-existing-server scope boundary and the same
/// "rolls back files this call itself wrote" rollback shape — both apply
/// here unchanged.
#[allow(clippy::too_many_arguments)]
pub fn import_curseforge(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    metadata: &CurseForgeManifestMetadata,
    staged_dir: &Path,
    pack_managed: bool,
    explicit_replace_intent: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<CurseForgeImportReport, CurseForgeImportError> {
    if modpack::pack_replace_refused(pack_managed, explicit_replace_intent) {
        return Err(CurseForgeImportError::PackManaged);
    }
    let add_on_kind = flavor
        .add_on_kind()
        .ok_or(CurseForgeImportError::NoAddOnKind)?;
    let add_on_folder = server_dir.join(add_on_kind.folder_name());

    let file_ids: Vec<i64> = metadata.files.iter().map(|f| f.file_id).collect();
    let resolved = provider::curseforge_files(transport, secrets, &file_ids).map_err(|e| {
        if matches!(
            e,
            msc_domain::addon_provider::AddonProviderError::MissingApiKey
        ) {
            CurseForgeImportError::MissingApiKey
        } else {
            CurseForgeImportError::Provider(e.to_string())
        }
    })?;
    let by_file_id: HashMap<i64, &msc_domain::addon_provider::CurseForgeFile> =
        resolved.iter().map(|f| (f.id, f)).collect();
    let mod_ids: Vec<i64> = resolved.iter().map(|file| file.mod_id).collect();
    let projects = provider::curseforge_mods(transport, secrets, &mod_ids).unwrap_or_default();
    let projects_by_id: HashMap<i64, &msc_domain::addon_provider::CurseForgeMod> = projects
        .iter()
        .map(|project| (project.id, project))
        .collect();

    let mut report = CurseForgeImportReport {
        pack_name: metadata.name.clone(),
        pack_version: metadata.version_id.clone(),
        ..Default::default()
    };
    let mut written: Vec<PathBuf> = Vec::new();
    let mut modrinth_match_cache: HashMap<(String, String), Option<ModrinthRecoveryMatch>> =
        HashMap::new();

    for manifest_file in &metadata.files {
        if should_cancel() {
            report.cancelled = true;
            break;
        }
        let Some(file) = by_file_id.get(&manifest_file.file_id) else {
            let pending = crate::curseforge_manual::PendingManualFile {
                project_id: manifest_file.project_id,
                file_id: manifest_file.file_id,
                expected_file_name: format!("file-{}.jar", manifest_file.file_id),
                expected_byte_size: 0,
                dest: add_on_folder.join(format!("file-{}.jar", manifest_file.file_id)),
            };
            report.unresolved_files.push(UnresolvedModpackFile {
                file_id: manifest_file.file_id.to_string(),
                file_name: pending.expected_file_name.clone(),
                project_name: "Unknown CurseForge file".to_string(),
                provider: "CurseForge".to_string(),
                reason: "CurseForge did not return metadata for this file.".to_string(),
                project_url: None,
                pending,
                expected_sha512: None,
            });
            report.failed_files.push((
                manifest_file.file_id,
                "CurseForge did not return this file id".to_string(),
            ));
            continue;
        };
        let Some(download_url) = &file.download_url else {
            let pending = crate::curseforge_manual::PendingManualFile {
                project_id: manifest_file.project_id,
                file_id: manifest_file.file_id,
                expected_file_name: file.file_name.clone(),
                expected_byte_size: file.file_length,
                dest: add_on_folder.join(&file.file_name),
            };
            let project = projects_by_id.get(&file.mod_id).copied();
            let project_name = project
                .map(|project| project.name.clone())
                .unwrap_or_else(|| file.file_name.clone());
            let project_url = project
                .and_then(|project| project.website_url().map(str::to_string))
                .or_else(|| {
                    Some(format!(
                        "https://www.curseforge.com/minecraft/search?search={}",
                        file.file_name.replace(' ', "%20")
                    ))
                });

            let match_key = (
                project_name.to_ascii_lowercase(),
                file.file_name.to_ascii_lowercase(),
            );
            let modrinth_match = modrinth_match_cache
                .entry(match_key)
                .or_insert_with(|| {
                    find_confident_modrinth_match(
                        transport,
                        flavor,
                        &metadata.minecraft_version,
                        ModrinthRecoveryRequest {
                            project_name: project.map(|project| project.name.as_str()),
                            expected_file_name: &file.file_name,
                        },
                    )
                })
                .clone();

            match modrinth_match {
                Some(server_match @ ModrinthRecoveryMatch::Server { .. }) => {
                    if let Some(ModrinthRecoveryOutcome::Installed(installed_path)) =
                        install_confident_modrinth_match(
                            transport,
                            fs,
                            &add_on_folder,
                            &file.file_name,
                            &metadata.version_id,
                            server_match,
                        )
                    {
                        written.push(installed_path.clone());
                        report.recovered_modrinth_files.push(installed_path);
                        continue;
                    }
                }
                Some(ModrinthRecoveryMatch::ClientOnly) => continue,
                None => {}
            }

            report.blocked_files.push(pending.clone());
            report.unresolved_files.push(UnresolvedModpackFile {
                file_id: manifest_file.file_id.to_string(),
                file_name: file.file_name.clone(),
                project_name,
                provider: "CurseForge".to_string(),
                reason: "The author blocks CurseForge API downloads for this file.".to_string(),
                project_url,
                pending,
                expected_sha512: None,
            });
            continue;
        };
        let project = projects_by_id.get(&file.mod_id).copied();
        let project_name = project
            .map(|project| project.name.clone())
            .unwrap_or_else(|| file.file_name.clone());
        let match_key = (
            project_name.to_ascii_lowercase(),
            file.file_name.to_ascii_lowercase(),
        );
        let modrinth_match = modrinth_match_cache
            .entry(match_key)
            .or_insert_with(|| {
                find_confident_modrinth_match(
                    transport,
                    flavor,
                    &metadata.minecraft_version,
                    ModrinthRecoveryRequest {
                        project_name: project.map(|project| project.name.as_str()),
                        expected_file_name: &file.file_name,
                    },
                )
            })
            .clone();
        if matches!(modrinth_match, Some(ModrinthRecoveryMatch::ClientOnly)) {
            continue;
        }
        let dest = add_on_folder.join(&file.file_name);
        if fs.create_dir_all(&add_on_folder).is_err() {
            let reason = "could not create destination directory".to_string();
            report
                .failed_files
                .push((manifest_file.file_id, reason.clone()));
            report.unresolved_files.push(UnresolvedModpackFile {
                file_id: manifest_file.file_id.to_string(),
                file_name: file.file_name.clone(),
                project_name: file.file_name.clone(),
                provider: "CurseForge".to_string(),
                reason,
                project_url: None,
                pending: crate::curseforge_manual::PendingManualFile {
                    project_id: manifest_file.project_id,
                    file_id: manifest_file.file_id,
                    expected_file_name: file.file_name.clone(),
                    expected_byte_size: file.file_length,
                    dest: dest.clone(),
                },
                expected_sha512: None,
            });
            continue;
        }
        match addon_store::install_verified_file(
            transport,
            fs,
            download_url,
            &metadata.version_id,
            None,
            &dest,
        ) {
            Ok(_) => {
                written.push(dest.clone());
                report.installed_files.push(dest);
            }
            Err(e) => {
                let reason = e.to_string();
                report
                    .failed_files
                    .push((manifest_file.file_id, reason.clone()));
                report.unresolved_files.push(UnresolvedModpackFile {
                    file_id: manifest_file.file_id.to_string(),
                    file_name: file.file_name.clone(),
                    project_name: projects_by_id
                        .get(&file.mod_id)
                        .map(|project| project.name.clone())
                        .unwrap_or_else(|| file.file_name.clone()),
                    provider: "CurseForge".to_string(),
                    reason,
                    project_url: projects_by_id
                        .get(&file.mod_id)
                        .and_then(|project| project.website_url().map(str::to_string)),
                    pending: crate::curseforge_manual::PendingManualFile {
                        project_id: manifest_file.project_id,
                        file_id: manifest_file.file_id,
                        expected_file_name: file.file_name.clone(),
                        expected_byte_size: file.file_length,
                        dest: dest.clone(),
                    },
                    expected_sha512: None,
                });
            }
        }
    }

    if report.cancelled {
        rollback_written(fs, &written);
        return Ok(report);
    }

    let skipped_client_only_overrides = client_only_override_paths(
        transport,
        fs,
        &staged_dir
            .join(&metadata.overrides_folder)
            .join(add_on_kind.folder_name()),
    );
    merge_directory_into_excluding(
        fs,
        &staged_dir.join(&metadata.overrides_folder),
        server_dir,
        &mut written,
        &skipped_client_only_overrides,
    );
    report.disabled_client_only_overrides = classify_override_jars(transport, fs, &add_on_folder);

    Ok(report)
}
