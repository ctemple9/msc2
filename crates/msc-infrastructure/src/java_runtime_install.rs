//! Managed Java runtime install: Phase 7's new agent-owned behavior
//! (QUESTION 1's answer (a) — "MSC 2 installs Java itself", per
//! `docs/msc2/families/phase7-scope.md`'s D-006 addendum). MSC 1 has no
//! equivalent to port — `JavaInstaller.swift` fetches a macOS-only
//! Temurin `.pkg` for a *human* to double-click through
//! `Installer.app`, with no cross-platform archive, no checksum step,
//! and no unpack step at all.
//!
//! Two fixtures characterize this: `fixtures/java-runtime-selection/
//! adoptium-archive-url-per-os-architecture-with-checksum-and-no-asset-
//! fallback.json` (the query, against a real, live-captured Adoptium
//! API response) and `.../adoptium-unpack-atomic-staged-download-and-
//! interrupted-install-cleanup.json` (the unpack/rollback design, which
//! that fixture's own notes are explicit is a pinned *design*, not an
//! observed run — nothing in MSC 1 to observe).
//!
//! One deliberate simplification against that second fixture's own
//! "interrupted-mid-download" scenario: [`Transport::get`] (this crate's
//! existing HTTP boundary, P7.13) is not a resumable, streams-to-disk
//! downloader — it buffers the whole response in memory and returns it
//! complete or not at all. A process kill mid-download therefore can't
//! leave a *partial* file on disk in this design at all (there is
//! nothing to write until the full, checksummed bytes already exist in
//! memory), which trivially satisfies the fixture's actual invariant
//! ("the final runtime directory is only ever written after both the
//! download completes AND its checksum verifies") without building
//! chunked-resume support Phase 7 was never scoped to build.

use std::fmt;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive as TarArchive;
use zip::ZipArchive;

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use crate::jar_provider::{JarProviderError, Transport};
use crate::java_runtime_detection::HostOs;

/// Re-exported so this module's own public API and test file are
/// unaffected by the P7.35 move of the shared SHA-256 implementation into
/// `download_staging` (which now needs it too, for Paper's published
/// digest — see that module's own doc).
pub use crate::download_staging::sha256_hex;

/// Adoptium release-metadata queries are tiny JSON documents.
pub const ADOPTIUM_QUERY_MAX_BYTES: u64 = 1024 * 1024; // 1 MB
/// Real Temurin JDK archives run 130-210 MB in the live evidence this
/// fixture captured; 400 MB leaves headroom without accepting an
/// unbounded stream.
pub const JDK_ARCHIVE_MAX_BYTES: u64 = 400 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    TarGz,
    Zip,
}

impl HostOs {
    /// The Adoptium API's own `os=` query parameter spelling.
    pub fn adoptium_os_param(self) -> &'static str {
        match self {
            HostOs::Mac => "mac",
            HostOs::Linux => "linux",
            HostOs::Windows => "windows",
        }
    }

    /// `archiveKindByOS` from the unpack fixture: every platform ships a
    /// `.tar.gz` except Windows, which ships a `.zip` — confirmed by the
    /// query fixture's own three real asset names (`...tar.gz` for
    /// linux/x64 and mac/aarch64, `...zip` for windows/x64).
    pub fn archive_kind(self) -> ArchiveKind {
        match self {
            HostOs::Windows => ArchiveKind::Zip,
            HostOs::Mac | HostOs::Linux => ArchiveKind::TarGz,
        }
    }
}

/// One Adoptium release's plain archive, as
/// `fixtures/java-runtime-selection/adoptium-archive-url-per-os-
/// architecture-with-checksum-and-no-asset-fallback.json` characterizes
/// it: `binary.package`'s `name`/`link`/`checksum`/`size`, read directly
/// rather than `binary.installer` (the `.pkg` MSC 1's own flow reads —
/// this is the plain archive that `.pkg` is itself built from).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptiumAsset {
    pub asset_name: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub archive_kind: ArchiveKind,
}

#[derive(Debug)]
pub enum JavaRuntimeInstallError {
    Transport(JarProviderError),
    InvalidResponse(String),
    /// The query returned an empty asset array — real, live-verified for
    /// Windows/aarch64 at major 17 (the fixture's own fourth case).
    /// Per the fixture's own `action`: reject outright, never fall back
    /// to a different architecture.
    NoAsset {
        major: u32,
        os: HostOs,
        arch: String,
    },
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    Extract(String),
    Io(io::Error),
}

impl fmt::Display for JavaRuntimeInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JavaRuntimeInstallError::Transport(e) => write!(f, "{e}"),
            JavaRuntimeInstallError::InvalidResponse(m) => {
                write!(f, "Adoptium response was not the expected shape: {m}")
            }
            JavaRuntimeInstallError::NoAsset { major, os, arch } => write!(
                f,
                "no Adoptium build of major {major} for {}/{arch}",
                os.adoptium_os_param()
            ),
            JavaRuntimeInstallError::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            JavaRuntimeInstallError::Extract(m) => write!(f, "{m}"),
            JavaRuntimeInstallError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for JavaRuntimeInstallError {}

impl From<io::Error> for JavaRuntimeInstallError {
    fn from(e: io::Error) -> Self {
        JavaRuntimeInstallError::Io(e)
    }
}

impl From<AtomicWriteError> for JavaRuntimeInstallError {
    fn from(e: AtomicWriteError) -> Self {
        match e {
            AtomicWriteError::Io(e) => JavaRuntimeInstallError::Io(e),
            AtomicWriteError::MissingParentDirectory(p) => JavaRuntimeInstallError::Io(
                io::Error::new(io::ErrorKind::NotFound, p.display().to_string()),
            ),
        }
    }
}

fn adoptium_query_url(major: u32, os: HostOs, arch: &str) -> String {
    format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?os={}&image_type=jdk&vendor=eclipse&architecture={arch}",
        os.adoptium_os_param(),
    )
}

/// The query half of the archive-url fixture: builds the request URL
/// exactly as its `queryTemplate` names, fetches it, and reads
/// `[0].binary.package`. An empty response array becomes
/// [`JavaRuntimeInstallError::NoAsset`] — the fixture's fourth case
/// (Windows/aarch64, major 17) — rather than silently returning nothing
/// or falling back to a different architecture.
pub fn query_adoptium_latest(
    transport: &dyn Transport,
    major: u32,
    os: HostOs,
    arch: &str,
) -> Result<AdoptiumAsset, JavaRuntimeInstallError> {
    let url = adoptium_query_url(major, os, arch);
    let bytes = transport
        .get(&url, "Adoptium release query", ADOPTIUM_QUERY_MAX_BYTES)
        .map_err(JavaRuntimeInstallError::Transport)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| JavaRuntimeInstallError::InvalidResponse("response was not UTF-8".into()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| JavaRuntimeInstallError::InvalidResponse(e.to_string()))?;
    let releases = value
        .as_array()
        .ok_or_else(|| JavaRuntimeInstallError::InvalidResponse("expected a JSON array".into()))?;

    let Some(first) = releases.first() else {
        return Err(JavaRuntimeInstallError::NoAsset {
            major,
            os,
            arch: arch.to_string(),
        });
    };

    let package = &first["binary"]["package"];
    let field = |name: &'static str| -> Result<&str, JavaRuntimeInstallError> {
        package[name].as_str().ok_or_else(|| {
            JavaRuntimeInstallError::InvalidResponse(format!("missing binary.package.{name}"))
        })
    };

    Ok(AdoptiumAsset {
        asset_name: field("name")?.to_string(),
        download_url: field("link")?.to_string(),
        sha256: field("checksum")?.to_string(),
        size_bytes: package["size"].as_u64().ok_or_else(|| {
            JavaRuntimeInstallError::InvalidResponse("missing binary.package.size".into())
        })?,
        archive_kind: os.archive_kind(),
    })
}

/// `<runtimes_root>/.staging/<runtime_name>.download` — the unpack
/// fixture's `.staging/<uuid>.tar.gz`, keyed on `runtime_name` (already
/// unique per OS/arch/major) rather than inventing a UUID generator this
/// crate has no other use for.
fn staged_archive_path(runtimes_root: &Path, runtime_name: &str) -> PathBuf {
    runtimes_root.join(".staging").join(runtime_name)
}

/// Downloads `asset`, verifies its SHA-256 against the checksum Adoptium
/// already published (no separate checksum-file fetch — one of the
/// query fixture's own findings), stages the verified bytes, extracts
/// them into `<runtimes_root>/<runtime_name>/` with the archive's single
/// top-level directory stripped (so the result holds the JDK's own tree
/// directly — `bin/java` on Linux/Windows, `Contents/Home/bin/java` on
/// Mac), and cleans up the staging file. Extraction itself lands in a
/// `<runtime_name>.extracting` sibling first and is renamed into place
/// only on success, so a failure partway through extraction — not one of
/// the fixture's four named scenarios, but implied by its own
/// "never a partially-unpacked... runtime directory" invariant — leaves
/// no half-written runtime directory behind either.
pub fn install_managed_runtime(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    runtimes_root: &Path,
    runtime_name: &str,
    asset: &AdoptiumAsset,
) -> Result<PathBuf, JavaRuntimeInstallError> {
    let staged_path = staged_archive_path(runtimes_root, runtime_name);
    // Best-effort: discard any stale leftover from a prior interrupted
    // attempt rather than trying to resume it (see this module's own doc
    // for why resumability is out of scope).
    let _ = fs.remove(&staged_path);

    let bytes = transport
        .get(
            &asset.download_url,
            "Adoptium JDK archive",
            JDK_ARCHIVE_MAX_BYTES,
        )
        .map_err(JavaRuntimeInstallError::Transport)?;

    let actual_sha256 = sha256_hex(&bytes);
    if !actual_sha256.eq_ignore_ascii_case(&asset.sha256) {
        // Nothing written to disk at all on this path.
        return Err(JavaRuntimeInstallError::ChecksumMismatch {
            expected: asset.sha256.clone(),
            actual: actual_sha256,
        });
    }

    if let Some(staging_dir) = staged_path.parent() {
        fs.create_dir_all(staging_dir)?;
    }
    atomic_write(fs, &staged_path, &bytes)?;

    let dest = runtimes_root.join(runtime_name);
    let extracting = runtimes_root.join(format!("{runtime_name}.extracting"));
    let _ = fs.remove(&extracting);

    let extract_result = match asset.archive_kind {
        ArchiveKind::TarGz => extract_tar_gz_stripped(fs, &bytes, &extracting),
        ArchiveKind::Zip => extract_zip_stripped(fs, &bytes, &extracting),
    };

    let outcome = match extract_result {
        Ok(()) => fs.rename(&extracting, &dest).map_err(Into::into),
        Err(e) => {
            let _ = fs.remove(&extracting);
            Err(e)
        }
    };

    // "Delete the staged archive" — the fixture's own step, on both the
    // success and the (non-checksum) failure path.
    let _ = fs.remove(&staged_path);

    outcome.map(|()| dest)
}

/// Strips an archive entry's first path component (the top-level
/// `jdk-21.0.12+8/`-style directory every Adoptium archive wraps its
/// contents in) and rejects anything that would then still contain a
/// `..` component or resolve absolute — the same zip-slip discipline
/// `crate::archive::validate_archive_safety` already applies to world/
/// backup archives, applied here by hand since that function works
/// against a real on-disk zip path rather than in-memory bytes.
fn strip_top_level(raw: &Path) -> Option<PathBuf> {
    let mut components = raw.components();
    components.next()?; // the top-level directory itself
    let rest: PathBuf = components.collect();
    if rest.as_os_str().is_empty() {
        return None; // the top-level directory entry itself, not a file under it
    }
    if rest.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) || rest.is_absolute()
    {
        return None;
    }
    Some(rest)
}

fn extract_tar_gz_stripped(
    fs: &dyn FileSystem,
    bytes: &[u8],
    dest_root: &Path,
) -> Result<(), JavaRuntimeInstallError> {
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = TarArchive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let raw_path = entry
            .path()
            .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?
            .into_owned();
        let Some(relative) = strip_top_level(&raw_path) else {
            continue;
        };
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;

        let dest_path = dest_root.join(&relative);
        if let Some(parent) = dest_path.parent() {
            fs.create_dir_all(parent)?;
        }
        let mode = entry.header().mode().unwrap_or(0);
        if mode & 0o111 != 0 {
            fs.write_executable(&dest_path, &contents)?;
        } else {
            fs.write(&dest_path, &contents)?;
        }
    }
    Ok(())
}

fn extract_zip_stripped(
    fs: &dyn FileSystem,
    bytes: &[u8],
    dest_root: &Path,
) -> Result<(), JavaRuntimeInstallError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;

    for i in 0..archive.len() {
        let mut zip_entry = archive
            .by_index(i)
            .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;
        if zip_entry.is_dir() {
            continue;
        }
        let Some(raw_path) = zip_entry.enclosed_name() else {
            continue;
        };
        let Some(relative) = strip_top_level(&raw_path) else {
            continue;
        };
        let mut contents = Vec::new();
        zip_entry
            .read_to_end(&mut contents)
            .map_err(|e| JavaRuntimeInstallError::Extract(e.to_string()))?;

        let executable = zip_entry.unix_mode().is_some_and(|mode| mode & 0o111 != 0);
        let dest_path = dest_root.join(&relative);
        if let Some(parent) = dest_path.parent() {
            fs.create_dir_all(parent)?;
        }
        if executable {
            fs.write_executable(&dest_path, &contents)?;
        } else {
            fs.write(&dest_path, &contents)?;
        }
    }
    Ok(())
}
