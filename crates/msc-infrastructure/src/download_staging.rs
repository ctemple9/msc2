//! `stage_download`: the shared stage-verify-move primitive for every
//! download-and-install workflow.
//!
//! `msc2-engineering.md` §7: "Downloads land in a temporary location, are
//! checksum-verified where the provider publishes one, and are moved into
//! active use only after validation. Interrupted downloads are safely
//! retryable. Cached files record origin and version." Every real MSC 1
//! download workflow repeats this shape without ever sharing a primitive:
//! `AppViewModel+PaperTemplateDownload.downloadLatestPaperTemplate`
//! (temp file in the templates dir, `moveItem` over the final name once
//! the download succeeds, `removeItem` the temp file on failure),
//! `AppViewModel+XboxBroadcastDownload.downloadOrUpdateXboxBroadcastJar`,
//! and `AppViewModel+PluginManagement.downloadLatestForPlugin` ("streams
//! the download to a temp file... moves the new file into place"). None
//! of those three actually checksum-verifies what they download — the
//! nearest existing precedent for *that* is `ResourcePackManager.sha1Hex`
//! (hex-encoded SHA1, used for `resource-pack-sha1` in
//! `server.properties`), which is why this primitive verifies against a
//! hex SHA1 rather than an algorithm invented for this step.
//!
//! This primitive takes the download's bytes already fully in memory
//! (network streaming is the caller's job — Phase 7-9's per-provider
//! workflows) and does three things in order: verify against
//! `expected_sha1_hex` if the caller has one, write through
//! [`crate::atomic_write`] (itself a stage-then-rename), and return a
//! [`CachedFile`] recording where the download came from. Verifying
//! before writing anything means a checksum mismatch never touches disk
//! at all — no temp file to create and then have to remember to clean up.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use std::fmt;
use std::path::{Path, PathBuf};

/// What survives a successful [`stage_download`] call: the final path,
/// plus the origin URL and version `msc2-engineering.md` §7 requires
/// every cached file to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFile {
    pub path: PathBuf,
    pub origin_url: String,
    pub version: String,
}

#[derive(Debug)]
pub enum DownloadStagingError {
    /// `bytes`' own SHA1 didn't match `expected_sha1_hex`. Carries both
    /// hex strings so a caller can log what was expected vs. what was
    /// actually received.
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    Write(AtomicWriteError),
}

impl fmt::Display for DownloadStagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadStagingError::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            DownloadStagingError::Write(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DownloadStagingError {}

/// Stages `bytes` at `dest`, verifying `expected_sha1_hex` first when the
/// caller has one (compared case-insensitively — providers vary in hex
/// case). A mismatch returns [`DownloadStagingError::ChecksumMismatch`]
/// without writing anything. Otherwise writes through [`atomic_write`],
/// which is itself the "lands in a temporary location... moved into
/// active use only after validation" step — a leftover temp file from a
/// prior interrupted attempt is safely overwritten in full, never
/// appended to, since [`crate::fs::FileSystem::write`] always replaces a
/// file's entire contents.
pub fn stage_download(
    fs: &dyn FileSystem,
    dest: &Path,
    bytes: &[u8],
    origin_url: &str,
    version: &str,
    expected_sha1_hex: Option<&str>,
) -> Result<CachedFile, DownloadStagingError> {
    if let Some(expected) = expected_sha1_hex {
        let actual = sha1_hex(bytes);
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(DownloadStagingError::ChecksumMismatch {
                expected: expected.to_string(),
                actual,
            });
        }
    }

    atomic_write(fs, dest, bytes).map_err(DownloadStagingError::Write)?;

    Ok(CachedFile {
        path: dest.to_path_buf(),
        origin_url: origin_url.to_string(),
        version: version.to_string(),
    })
}

/// A from-scratch SHA1 (FIPS 180-4), hex-encoded lowercase — matching
/// `ResourcePackManager.sha1Hex`'s own output shape. Written out rather
/// than pulled from a crate: this is an integrity checksum against a
/// publisher-supplied hash, the same role MSC 1 already uses SHA1 for
/// (`resource-pack-sha1`), not a security boundary — SHA1's known
/// collision weaknesses don't matter for "does this download match what
/// the provider published." `pub` (rather than private) so
/// `tests/download_staging.rs` can check it directly against the FIPS
/// 180-4 published test vectors, independent of this crate's own JSON
/// fixtures — those fixtures' own expected hashes were computed
/// out-of-band (Python's `hashlib`), so between the two, a checksum-logic
/// bug has nowhere to hide.
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let bit_len = (data.len() as u64) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    format!("{h0:08x}{h1:08x}{h2:08x}{h3:08x}{h4:08x}")
}
