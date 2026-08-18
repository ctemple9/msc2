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

/// A from-scratch SHA-256 (FIPS 180-4), hex-encoded lowercase — the same
/// "written out rather than pulled from a crate" precedent
/// `download_staging::sha1_hex` set (an integrity checksum against a
/// publisher-supplied hash, not a security boundary this crate needs a
/// vetted implementation for). Adoptium publishes SHA-256, not SHA-1, so
/// `download_staging::stage_download` (SHA-1-only) can't be reused as-is
/// here.
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}
