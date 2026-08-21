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
//! `expected_checksum` if the caller has one, write through
//! [`crate::atomic_write`] (itself a stage-then-rename), and return a
//! [`CachedFile`] recording where the download came from. Verifying
//! before writing anything means a checksum mismatch never touches disk
//! at all — no temp file to create and then have to remember to clean up.
//!
//! **P7.35 amendment (the explicit P3.14 amendment Codex's 2026-08-20
//! Phase 7 review required):** this primitive was originally SHA-1-only,
//! the nearest MSC 1 precedent (`ResourcePackManager.sha1Hex`) available
//! when P3.14 built it. Real Phase 7 provider evidence
//! (`corpus/providers/README.md`'s P7.28 finding) shows the six server-jar
//! providers don't agree on an algorithm: Mojang's per-version metadata
//! publishes SHA-1, Paper's fill v3 build entries publish SHA-256, and
//! Purpur's per-build API publishes MD5 — Fabric's composed download
//! endpoint and NeoForge's/Forge's Maven publish no digest at all for the
//! jar/installer they serve. [`ExpectedChecksum`] carries which algorithm
//! a caller's digest is in, so `stage_download` can verify whichever one
//! the exact provider response actually published, without forcing every
//! caller through SHA-1. `None` still means exactly what it always meant:
//! no digest to verify against, staged and moved unverified.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use std::fmt;
use std::path::{Path, PathBuf};

/// Which algorithm an [`ExpectedChecksum`]'s hex digest is in — see this
/// module's own P7.35 amendment note for why real providers need all
/// three, not just SHA-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Sha1,
    Sha256,
    Sha512,
    Md5,
}

impl ChecksumAlgorithm {
    fn digest_hex(self, bytes: &[u8]) -> String {
        match self {
            ChecksumAlgorithm::Sha1 => sha1_hex(bytes),
            ChecksumAlgorithm::Sha256 => sha256_hex(bytes),
            ChecksumAlgorithm::Sha512 => sha512_hex(bytes),
            ChecksumAlgorithm::Md5 => md5_hex(bytes),
        }
    }
}

impl fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChecksumAlgorithm::Sha1 => write!(f, "sha1"),
            ChecksumAlgorithm::Sha256 => write!(f, "sha256"),
            ChecksumAlgorithm::Sha512 => write!(f, "sha512"),
            ChecksumAlgorithm::Md5 => write!(f, "md5"),
        }
    }
}

/// A digest a provider published for a download, extracted by the caller
/// from the exact response used to choose the download (never invented or
/// looked up separately) — the shape `stage_download`'s `Option<&Self>`
/// verifies against when a provider publishes one at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedChecksum {
    pub algorithm: ChecksumAlgorithm,
    pub hex: String,
}

impl ExpectedChecksum {
    pub fn sha1(hex: impl Into<String>) -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Sha1,
            hex: hex.into(),
        }
    }
    pub fn sha256(hex: impl Into<String>) -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Sha256,
            hex: hex.into(),
        }
    }
    pub fn sha512(hex: impl Into<String>) -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Sha512,
            hex: hex.into(),
        }
    }
    pub fn md5(hex: impl Into<String>) -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Md5,
            hex: hex.into(),
        }
    }
}

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
    /// `bytes`' own digest didn't match `expected_checksum`. Carries the
    /// algorithm plus both hex strings so a caller can log what was
    /// expected vs. what was actually received.
    ChecksumMismatch {
        algorithm: ChecksumAlgorithm,
        expected: String,
        actual: String,
    },
    Write(AtomicWriteError),
}

impl fmt::Display for DownloadStagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadStagingError::ChecksumMismatch {
                algorithm,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{algorithm} checksum mismatch: expected {expected}, got {actual}"
                )
            }
            DownloadStagingError::Write(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DownloadStagingError {}

/// Stages `bytes` at `dest`, verifying `expected_checksum` first when the
/// caller has one (compared case-insensitively — providers vary in hex
/// case), against whichever algorithm that checksum names. A mismatch
/// returns [`DownloadStagingError::ChecksumMismatch`] without writing
/// anything. Otherwise writes through [`atomic_write`], which is itself
/// the "lands in a temporary location... moved into active use only after
/// validation" step — a leftover temp file from a prior interrupted
/// attempt is safely overwritten in full, never appended to, since
/// [`crate::fs::FileSystem::write`] always replaces a file's entire
/// contents.
pub fn stage_download(
    fs: &dyn FileSystem,
    dest: &Path,
    bytes: &[u8],
    origin_url: &str,
    version: &str,
    expected_checksum: Option<&ExpectedChecksum>,
) -> Result<CachedFile, DownloadStagingError> {
    if let Some(expected) = expected_checksum {
        let actual = expected.algorithm.digest_hex(bytes);
        if !actual.eq_ignore_ascii_case(&expected.hex) {
            return Err(DownloadStagingError::ChecksumMismatch {
                algorithm: expected.algorithm,
                expected: expected.hex.clone(),
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

/// A from-scratch SHA-256 (FIPS 180-4), hex-encoded lowercase — Paper's
/// fill v3 API publishes SHA-256 for a build's `server:default` download
/// (this module's own P7.35 amendment note), the same "written out rather
/// than pulled from a crate" precedent [`sha1_hex`] set. **Moved here from
/// `java_runtime_install.rs`** (P7.16 wrote it first, for Adoptium's own
/// SHA-256-published archives) rather than hand-writing a second copy —
/// `java_runtime_install` now re-exports this one so its own public API
/// and tests are unaffected.
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

/// A from-scratch SHA-512 (FIPS 180-4), hex-encoded lowercase — P8.16's
/// exact-hash add-on identity: Modrinth's `version_file`/`version_files`
/// endpoints key on a jar's SHA-512, and `fixtures/modpack-archive-safety/
/// manifest-hash-never-verified-against-downloaded-bytes.json`'s gap
/// (P8.19) is closed against a `.mrpack` file entry's own published
/// `hashes.sha512`. Same "written out rather than pulled from a crate"
/// precedent [`sha1_hex`]/[`sha256_hex`]/[`md5_hex`] set — this crate has
/// no `sha2` dependency, and a publisher-integrity checksum doesn't need
/// one. 64-bit words/rotations and 80 rounds, not SHA-256's 32-bit/64;
/// tested against the FIPS 180-4 published test vectors for `""`/`"abc"`.
pub fn sha512_hex(data: &[u8]) -> String {
    const K: [u64; 80] = [
        0x428a2f98d728ae22,
        0x7137449123ef65cd,
        0xb5c0fbcfec4d3b2f,
        0xe9b5dba58189dbbc,
        0x3956c25bf348b538,
        0x59f111f1b605d019,
        0x923f82a4af194f9b,
        0xab1c5ed5da6d8118,
        0xd807aa98a3030242,
        0x12835b0145706fbe,
        0x243185be4ee4b28c,
        0x550c7dc3d5ffb4e2,
        0x72be5d74f27b896f,
        0x80deb1fe3b1696b1,
        0x9bdc06a725c71235,
        0xc19bf174cf692694,
        0xe49b69c19ef14ad2,
        0xefbe4786384f25e3,
        0x0fc19dc68b8cd5b5,
        0x240ca1cc77ac9c65,
        0x2de92c6f592b0275,
        0x4a7484aa6ea6e483,
        0x5cb0a9dcbd41fbd4,
        0x76f988da831153b5,
        0x983e5152ee66dfab,
        0xa831c66d2db43210,
        0xb00327c898fb213f,
        0xbf597fc7beef0ee4,
        0xc6e00bf33da88fc2,
        0xd5a79147930aa725,
        0x06ca6351e003826f,
        0x142929670a0e6e70,
        0x27b70a8546d22ffc,
        0x2e1b21385c26c926,
        0x4d2c6dfc5ac42aed,
        0x53380d139d95b3df,
        0x650a73548baf63de,
        0x766a0abb3c77b2a8,
        0x81c2c92e47edaee6,
        0x92722c851482353b,
        0xa2bfe8a14cf10364,
        0xa81a664bbc423001,
        0xc24b8b70d0f89791,
        0xc76c51a30654be30,
        0xd192e819d6ef5218,
        0xd69906245565a910,
        0xf40e35855771202a,
        0x106aa07032bbd1b8,
        0x19a4c116b8d2d0c8,
        0x1e376c085141ab53,
        0x2748774cdf8eeb99,
        0x34b0bcb5e19b48a8,
        0x391c0cb3c5c95a63,
        0x4ed8aa4ae3418acb,
        0x5b9cca4f7763e373,
        0x682e6ff3d6b2b8a3,
        0x748f82ee5defb2fc,
        0x78a5636f43172f60,
        0x84c87814a1f0ab72,
        0x8cc702081a6439ec,
        0x90befffa23631e28,
        0xa4506cebde82bde9,
        0xbef9a3f7b2c67915,
        0xc67178f2e372532b,
        0xca273eceea26619c,
        0xd186b8c721c0c207,
        0xeada7dd6cde0eb1e,
        0xf57d4f7fee6ed178,
        0x06f067aa72176fba,
        0x0a637dc5a2c898a6,
        0x113f9804bef90dae,
        0x1b710b35131c471b,
        0x28db77f523047d84,
        0x32caab7b40c72493,
        0x3c9ebe0a15c9bebc,
        0x431d67c49c100d4c,
        0x4cc5d4becb3e42b6,
        0x597f299cfc657e2a,
        0x5fcb6fab3ad6faec,
        0x6c44198c4a475817,
    ];
    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    let bit_len = (data.len() as u128) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 128 != 112 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(128) {
        let mut w = [0u64; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&chunk[8 * i..8 * i + 8]);
            *word = u64::from_be_bytes(bytes);
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
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

    h.iter().map(|word| format!("{word:016x}")).collect()
}

/// A from-scratch MD5 (RFC 1321), hex-encoded lowercase — Purpur's
/// per-build API (`/v2/purpur/{version}/{build}`) publishes MD5 only,
/// confirmed against the real live API (this module's own P7.35 amendment
/// note); the same "written out rather than pulled from a crate"
/// precedent [`sha1_hex`]/[`sha256_hex`] set. MD5's known collision
/// weaknesses matter even less here than SHA-1's already-noted
/// irrelevance — Purpur itself is the only algorithm choice; the
/// alternative to accepting MD5 here is verifying nothing at all.
pub fn md5_hex(data: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    let bit_len = (data.len() as u64) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    // MD5 is little-endian throughout, unlike SHA-1/SHA-256's big-endian
    // length suffix and word layout above.
    message.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in message.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    [a0, b0, c0, d0]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
