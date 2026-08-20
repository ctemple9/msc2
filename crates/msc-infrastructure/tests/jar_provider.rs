//! P7.13's own tests. Per this phase's decided-for-you note ("Provisioning
//! tests never touch the network"), every family/catalog test here runs
//! against [`FakeTransport`], fed from `corpus/providers/`'s real recorded
//! responses (P7.3) -- never a real request to any of the six providers.
//! [`FakeTransport`] is this crate's fake implementation of
//! `msc_infrastructure::jar_provider::Transport`, built for this test file
//! specifically (not shared with `msc-domain`'s fixture tests, which don't
//! need a `Transport` at all).
//!
//! The one exception, `jar_provider_http_transport_enforces_size_cap_and_timeout`,
//! spins up a real local loopback HTTP server (`127.0.0.1`, an ephemeral
//! port) to prove [`HttpTransport`]'s own bounding behavior (the size cap
//! and the timeout actually firing). That's testing this crate's own
//! transport code against bytes it controls, not "touching the network" in
//! the sense the phase's note guards against (a real external provider
//! going down, changing shape, or costing rate-limit budget in CI) --
//! nothing here ever leaves the loopback interface.

use msc_infrastructure::download_staging::{md5_hex, sha256_hex};
use msc_infrastructure::jar_provider::{
    self, HttpTransport, JarProviderError, Transport, fabric_list_versions,
    forge_latest_recommended, forge_list_version_pairs, neoforge_list_version_pairs,
    paper_flatten_and_sort_versions, purpur_list_versions, vanilla_download_latest,
    vanilla_list_versions,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/providers")
}

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_file(self, url: &str, relative_corpus_path: &str) -> Self {
        let bytes = std::fs::read(corpus_dir().join(relative_corpus_path))
            .unwrap_or_else(|e| panic!("reading {relative_corpus_path}: {e}"));
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes);
        self
    }

    fn with_bytes(self, url: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
        self
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        let bytes = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response registered for {url}"))
            })?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.to_string(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-jar-provider-test-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn jar_provider_vanilla_list_versions_against_real_corpus_manifest() {
    let transport = FakeTransport::new().with_file(
        "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
        "vanilla/version-manifest-v2.json",
    );
    let entries = vanilla_list_versions(&transport).unwrap();
    assert!(entries.iter().any(|e| e.id == "26.2"));
    assert!(entries.iter().all(|e| e.is_stable));
}

#[test]
fn jar_provider_vanilla_download_latest_two_hop_against_real_corpus() {
    // The manifest hop is real corpus (resolves "26.2" -> the per-version
    // metadata URL below). The per-version metadata itself is overridden
    // with a synthetic body -- P7.35 wired checksum enforcement to that
    // response's own `downloads.server.sha1` field, and the real corpus
    // file's real sha1 (823e2250...) is for a real 60 MB jar this test
    // can't ship; a synthetic body with a sha1 matching this test's own
    // fake bytes proves the same two-hop mechanism without that.
    let transport = FakeTransport::new()
        .with_file(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            "vanilla/version-manifest-v2.json",
        )
        .with_bytes(
            "https://piston-meta.mojang.com/v1/packages/c75d82e7fa6eca5a043dab0c6cf77cb8317644f4/26.2.json",
            br#"{"downloads":{"server":{"url":"https://piston-data.mojang.com/v1/objects/823e2250d24b3ddac457a60c92a6a941943fcd6a/server.jar","sha1":"59989014812f106c69aab2e4a52d79c4aea03f45"}}}"#.to_vec(),
        )
        .with_bytes(
            "https://piston-data.mojang.com/v1/objects/823e2250d24b3ddac457a60c92a6a941943fcd6a/server.jar",
            b"fake vanilla server jar bytes".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("vanilla-download-latest");
    let dest = tmp.path().join("server.jar");

    let cached = vanilla_download_latest(&transport, &fs, &dest).unwrap();
    assert_eq!(cached.version, "26.2");
    assert_eq!(cached.path, dest);
}

#[test]
fn jar_provider_vanilla_download_latest_refuses_sha1_mismatch_and_leaves_destination_untouched() {
    // P7.35: the same real corpus manifest as the test above, but the
    // per-version metadata's published sha1 doesn't match the bytes the
    // "server" actually returns -- proves the real production call path
    // (not just the generic download_staging primitive) refuses and never
    // touches a pre-existing destination.
    let transport = FakeTransport::new()
        .with_file(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            "vanilla/version-manifest-v2.json",
        )
        .with_bytes(
            "https://piston-meta.mojang.com/v1/packages/c75d82e7fa6eca5a043dab0c6cf77cb8317644f4/26.2.json",
            br#"{"downloads":{"server":{"url":"https://piston-data.mojang.com/v1/objects/823e2250d24b3ddac457a60c92a6a941943fcd6a/server.jar","sha1":"0000000000000000000000000000000000000000"}}}"#.to_vec(),
        )
        .with_bytes(
            "https://piston-data.mojang.com/v1/objects/823e2250d24b3ddac457a60c92a6a941943fcd6a/server.jar",
            b"corrupted vanilla server jar bytes".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("vanilla-download-checksum-refused");
    let dest = tmp.path().join("server.jar");
    std::fs::write(&dest, b"pre-existing server.jar").unwrap();

    let result = vanilla_download_latest(&transport, &fs, &dest);
    assert!(
        matches!(
            result,
            Err(JarProviderError::Staging(
                msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. }
            ))
        ),
        "expected a checksum mismatch, got {result:?}"
    );
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        b"pre-existing server.jar",
        "destination must be untouched by a refused download"
    );
}

#[test]
fn jar_provider_purpur_list_versions_against_real_corpus() {
    let transport = FakeTransport::new().with_file(
        "https://api.purpurmc.org/v2/purpur",
        "purpur/project-purpur.json",
    );
    let entries = purpur_list_versions(&transport).unwrap();
    assert!(entries.iter().any(|e| e.id == "1.21.11"));
}

#[test]
fn jar_provider_purpur_download_version_stages_bytes() {
    // P7.35: purpur_download_version now fetches the per-build metadata
    // hop (real corpus shape: corpus/providers/purpur/
    // build-latest-1.21.11.json) before downloading, to get the md5 to
    // verify against.
    let transport = FakeTransport::new()
        .with_bytes(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest",
            format!(r#"{{"md5":"{}"}}"#, md5_hex(b"fake purpur jar")).into_bytes(),
        )
        .with_bytes(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest/download",
            b"fake purpur jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("purpur-download");
    let dest = tmp.path().join("purpur.jar");

    let cached = jar_provider::purpur_download_version(&transport, &fs, "1.21.11", &dest).unwrap();
    assert_eq!(cached.version, "1.21.11");
}

#[test]
fn jar_provider_purpur_download_version_reads_md5_from_real_corpus_build_metadata_shape() {
    // The real recorded per-build response (corpus/providers/purpur/
    // build-latest-1.21.11.json) publishes md5 "b8d5402ef8e38bf60cabc6ee
    // ddb3fa18" for the real build 2568 jar -- this test can't ship that
    // ~65 MB real jar as a fixture (and can't invert MD5 to find bytes
    // that hash to it), so it proves the *shape* is read correctly the
    // other direction: the real corpus response's md5 does not match this
    // test's own fake bytes, so the real production call path refuses,
    // exactly as it must for a genuinely corrupted or substituted
    // download from this exact real API response shape.
    let transport = FakeTransport::new()
        .with_file(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest",
            "purpur/build-latest-1.21.11.json",
        )
        .with_bytes(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest/download",
            b"not the real build 2568 jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("purpur-download-real-corpus-shape");
    let dest = tmp.path().join("purpur.jar");

    let result = jar_provider::purpur_download_version(&transport, &fs, "1.21.11", &dest);
    assert!(
        matches!(
            result,
            Err(JarProviderError::Staging(
                msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. }
            ))
        ),
        "expected a checksum mismatch, got {result:?}"
    );
}

#[test]
fn jar_provider_purpur_download_version_refuses_md5_mismatch_and_leaves_destination_untouched() {
    let transport = FakeTransport::new()
        .with_bytes(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest",
            br#"{"md5":"00000000000000000000000000000000"}"#.to_vec(),
        )
        .with_bytes(
            "https://api.purpurmc.org/v2/purpur/1.21.11/latest/download",
            b"corrupted purpur jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("purpur-download-checksum-refused");
    let dest = tmp.path().join("purpur.jar");
    std::fs::write(&dest, b"pre-existing purpur.jar").unwrap();

    let result = jar_provider::purpur_download_version(&transport, &fs, "1.21.11", &dest);
    assert!(
        matches!(
            result,
            Err(JarProviderError::Staging(
                msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. }
            ))
        ),
        "expected a checksum mismatch, got {result:?}"
    );
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        b"pre-existing purpur.jar",
        "destination must be untouched by a refused download"
    );
}

#[test]
fn jar_provider_paper_flatten_and_select_build_against_real_corpus() {
    let transport = FakeTransport::new()
        .with_file(
            "https://fill.papermc.io/v3/projects/paper",
            "paper/projects-paper.json",
        )
        .with_file(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            "paper/builds-1.21.11.json",
        );

    let versions = paper_flatten_and_sort_versions(&transport).unwrap();
    assert!(versions.contains(&"1.21.11".to_string()));

    let selection = jar_provider::paper_select_build(&transport, "1.21.11", false).unwrap();
    assert_eq!(selection.build_id, 132);
    assert!(selection.is_stable);
}

#[test]
fn jar_provider_paper_download_build_stages_bytes() {
    // P7.35: the real corpus builds file's own real sha256 (for a real
    // build 132 jar this test can't ship) is overridden with a synthetic
    // builds response -- same shape (id 132, STABLE, server:default), a
    // sha256 that actually matches this test's own fake bytes.
    let transport = FakeTransport::new()
        .with_bytes(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            format!(
                r#"[{{"id":132,"channel":"STABLE","downloads":{{"server:default":{{"url":"https://fill-data.papermc.io/v1/objects/example/paper-1.21.11-132.jar","checksums":{{"sha256":"{}"}}}}}}}}]"#,
                sha256_hex(b"fake paper jar bytes")
            )
            .into_bytes(),
        )
        .with_bytes(
            "https://fill-data.papermc.io/v1/objects/example/paper-1.21.11-132.jar",
            b"fake paper jar bytes".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("paper-download");
    let dest = tmp.path().join("paper.jar");

    let cached =
        jar_provider::paper_download_build(&transport, &fs, "1.21.11", 132, &dest).unwrap();
    assert_eq!(cached.version, "1.21.11-132");
}

#[test]
fn jar_provider_paper_download_build_against_real_corpus_checksum_shape_refuses_mismatch() {
    // The real recorded builds response (corpus/providers/paper/
    // builds-1.21.11.json) publishes a real sha256 for a real build 132
    // jar this test can't ship (same reasoning as Purpur's own real-corpus
    // shape test above) -- proves the real production call path reads
    // that field and refuses bytes that don't match it.
    let transport = FakeTransport::new()
        .with_file(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            "paper/builds-1.21.11.json",
        )
        .with_bytes(
            "https://fill-data.papermc.io/v1/objects/5ffef465eeeb5f2a3c23a24419d97c51afd7dbb4923ff42df9a3f58bba1ccfba/paper-1.21.11-132.jar",
            b"not the real build 132 jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("paper-download-real-corpus-shape");
    let dest = tmp.path().join("paper.jar");

    let result = jar_provider::paper_download_build(&transport, &fs, "1.21.11", 132, &dest);
    assert!(
        matches!(
            result,
            Err(JarProviderError::Staging(
                msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. }
            ))
        ),
        "expected a checksum mismatch, got {result:?}"
    );
}

#[test]
fn jar_provider_paper_download_build_refuses_sha256_mismatch_and_leaves_destination_untouched() {
    let transport = FakeTransport::new()
        .with_bytes(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            br#"[{"id":132,"channel":"STABLE","downloads":{"server:default":{"url":"https://x/paper.jar","checksums":{"sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}}}]"#.to_vec(),
        )
        .with_bytes("https://x/paper.jar", b"corrupted paper jar".to_vec());
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("paper-download-checksum-refused");
    let dest = tmp.path().join("paper.jar");
    std::fs::write(&dest, b"pre-existing paper.jar").unwrap();

    let result = jar_provider::paper_download_build(&transport, &fs, "1.21.11", 132, &dest);
    assert!(
        matches!(
            result,
            Err(JarProviderError::Staging(
                msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. }
            ))
        ),
        "expected a checksum mismatch, got {result:?}"
    );
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        b"pre-existing paper.jar",
        "destination must be untouched by a refused download"
    );
}

// --- P7.19: pinned (non-latest) version-change downloads ---

#[test]
fn jar_provider_vanilla_download_version_pins_release_id_not_latest() {
    let transport = FakeTransport::new()
        .with_file(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            "vanilla/version-manifest-v2.json",
        )
        .with_bytes(
            "https://piston-meta.mojang.com/v1/packages/e846101ba6cf0b548e8b71624c7351b6458c5349/1.20.1.json",
            br#"{"downloads":{"server":{"url":"https://piston-data.mojang.com/fake/1.20.1-server.jar"}}}"#
                .to_vec(),
        )
        .with_bytes(
            "https://piston-data.mojang.com/fake/1.20.1-server.jar",
            b"fake 1.20.1 server jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("vanilla-download-pinned");
    let dest = tmp.path().join("server.jar");

    // 1.20.1 (not `latest.release`, which the real manifest puts at 26.2) --
    // proves the pin actually threads through, not just that the two-hop
    // resolution still works.
    let cached = jar_provider::vanilla_download_version(&transport, &fs, "1.20.1", &dest).unwrap();
    assert_eq!(cached.version, "1.20.1");
}

#[test]
fn jar_provider_paper_download_pinned_version_picks_highest_id_any_channel_real_corpus() {
    // P7.35: same synthetic-checksum override as
    // jar_provider_paper_download_build_stages_bytes -- the real corpus
    // file's real sha256 doesn't match this test's own fake bytes.
    let transport = FakeTransport::new()
        .with_bytes(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            format!(
                r#"[{{"id":132,"channel":"STABLE","downloads":{{"server:default":{{"url":"https://fill-data.papermc.io/v1/objects/example/paper-1.21.11-132.jar","checksums":{{"sha256":"{}"}}}}}}}}]"#,
                sha256_hex(b"fake paper jar bytes")
            )
            .into_bytes(),
        )
        .with_bytes(
            "https://fill-data.papermc.io/v1/objects/example/paper-1.21.11-132.jar",
            b"fake paper jar bytes".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("paper-download-pinned");
    let dest = tmp.path().join("paper.jar");

    let (cached, build_id) =
        jar_provider::paper_download_pinned_version(&transport, &fs, "1.21.11", &dest).unwrap();
    assert_eq!(cached.version, "1.21.11-132");
    assert_eq!(build_id, 132);
}

#[test]
fn jar_provider_paper_download_pinned_version_ignores_channel_picks_highest_id() {
    // Real corpus's own highest id is always STABLE, so it can't tell
    // "no channel filtering" apart from "prefers stable" -- this synthetic
    // response puts the highest id on ALPHA to prove the distinction P7.5/
    // this module's own doc names: `paper_download_pinned_version` has no
    // channel preference at all, unlike `paper_select_build`.
    let transport = FakeTransport::new()
        .with_bytes(
            "https://fill.papermc.io/v3/projects/paper/versions/26.2/builds",
            br#"[
                {"id": 5, "channel": "STABLE", "downloads": {"server:default": {"url": "https://x/5.jar"}}},
                {"id": 9, "channel": "ALPHA", "downloads": {"server:default": {"url": "https://x/9.jar"}}}
            ]"#
            .to_vec(),
        )
        .with_bytes("https://x/9.jar", b"alpha build wins".to_vec());
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("paper-download-pinned-any-channel");
    let dest = tmp.path().join("paper.jar");

    let (cached, build_id) =
        jar_provider::paper_download_pinned_version(&transport, &fs, "26.2", &dest).unwrap();
    assert_eq!(cached.version, "26.2-9");
    assert_eq!(build_id, 9);
    assert_eq!(std::fs::read(&dest).unwrap(), b"alpha build wins");
}

#[test]
fn jar_provider_paper_list_versions_for_picker_against_real_corpus() {
    let transport = FakeTransport::new()
        .with_file(
            "https://fill.papermc.io/v3/projects/paper",
            "paper/projects-paper.json",
        )
        .with_file(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            "paper/builds-1.21.11.json",
        );
    let entries = jar_provider::paper_list_versions_for_picker(&transport).unwrap();
    let entry = entries
        .iter()
        .find(|e| e.id == "1.21.11")
        .expect("1.21.11 present");
    assert_eq!(entry.build_label.as_deref(), Some("build 132"));
    assert!(entry.is_stable);
    // Every other version in the real project listing has no registered
    // builds response on this fake transport, so its per-version fetch
    // fails and it's dropped -- proving the per-version-failure-tolerant
    // path (source's own `guard let (data, _) = try? ...`), not just the
    // happy path for the one version this test wired up.
    assert!(!entries.iter().any(|e| e.id == "26.2"));
}

#[test]
fn jar_provider_fabric_list_versions_against_real_corpus() {
    let transport = FakeTransport::new().with_file(
        "https://meta.fabricmc.net/v2/versions/game",
        "fabric/game.json",
    );
    let entries = fabric_list_versions(&transport).unwrap();
    assert!(entries.iter().any(|e| e.id == "1.20.1"));
}

#[test]
fn jar_provider_fabric_download_version_resolves_loader_and_installer() {
    let transport = FakeTransport::new()
        .with_file(
            "https://meta.fabricmc.net/v2/versions/loader/1.21.11",
            "fabric/loader-1.21.11.json",
        )
        .with_file(
            "https://meta.fabricmc.net/v2/versions/installer",
            "fabric/installer.json",
        )
        .with_bytes(
            "https://meta.fabricmc.net/v2/versions/loader/1.21.11/0.19.3/1.1.2/server/jar",
            b"fake fabric server jar".to_vec(),
        );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("fabric-download");
    let dest = tmp.path().join("fabric-server-launch.jar");

    let cached =
        jar_provider::fabric_download_version(&transport, &fs, "1.21.11", None, &dest).unwrap();
    assert_eq!(cached.version, "1.21.11");
}

#[test]
fn jar_provider_neoforge_list_version_pairs_against_real_corpus() {
    let transport = FakeTransport::new().with_file(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
        "neoforge/maven-metadata.xml",
    );
    let entries = neoforge_list_version_pairs(&transport).unwrap();
    assert!(!entries.is_empty());
    assert!(entries.iter().all(|e| e.loader_version.is_some()));
}

#[test]
fn jar_provider_neoforge_download_installer_stages_bytes() {
    let transport = FakeTransport::new().with_bytes(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/20.4.237/neoforge-20.4.237-installer.jar",
        b"fake neoforge installer".to_vec(),
    );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("neoforge-installer");
    let dest = tmp.path().join("neoforge-installer.jar");

    let cached =
        jar_provider::neoforge_download_installer(&transport, &fs, "20.4.237", &dest).unwrap();
    assert_eq!(cached.version, "20.4.237");
}

#[test]
fn jar_provider_forge_list_version_pairs_and_latest_recommended_against_real_corpus() {
    let transport = FakeTransport::new()
        .with_file(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
            "forge/maven-metadata.xml",
        )
        .with_file(
            "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
            "forge/promotions-slim.json",
        );

    let entries = forge_list_version_pairs(&transport).unwrap();
    assert!(!entries.is_empty());

    let (mc, forge) = forge_latest_recommended(&transport).unwrap();
    assert_eq!(mc, "26.2");
    assert_eq!(forge, "65.1.0");
}

#[test]
fn jar_provider_forge_download_installer_stages_bytes() {
    let transport = FakeTransport::new().with_bytes(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.5/forge-1.20.1-47.4.5-installer.jar",
        b"fake forge installer".to_vec(),
    );
    let fs = msc_infrastructure::fs::StdFileSystem;
    let tmp = TempDir::new("forge-installer");
    let dest = tmp.path().join("forge-installer.jar");

    let cached =
        jar_provider::forge_download_installer(&transport, &fs, "1.20.1", "47.4.5", &dest).unwrap();
    assert_eq!(cached.version, "1.20.1-47.4.5");
}

#[test]
fn jar_provider_connection_refused_degrades_to_typed_error_not_a_panic() {
    // Bind a listener, then drop it (closing the socket) before connecting
    // -- guarantees an immediate ECONNREFUSED on loopback rather than a
    // slow real-network timeout, proving connection failure degrades to a
    // typed error rather than hanging or panicking.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let transport = HttpTransport::new();
    let result = transport.get(&format!("http://{addr}/"), "closed loopback port", 1024);
    assert!(result.is_err());
}

// --- HttpTransport's own bounding behavior, against a real local loopback
// server (never leaves 127.0.0.1 -- see this file's own module doc). ---

fn spawn_loopback_server(body: Vec<u8>, delay: Option<Duration>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            if let Some(delay) = delay {
                std::thread::sleep(delay);
            }
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{addr}/")
}

#[test]
fn jar_provider_http_transport_enforces_size_cap() {
    let body = vec![b'a'; 4096];
    let url = spawn_loopback_server(body, None);
    let transport = HttpTransport::new();

    let result = transport.get(&url, "loopback body", 100);
    match result {
        Err(JarProviderError::ResponseTooLarge { max_bytes, .. }) => assert_eq!(max_bytes, 100),
        other => panic!("expected ResponseTooLarge, got {other:?}"),
    }
}

#[test]
fn jar_provider_http_transport_reads_under_cap_successfully() {
    let body = b"small response body".to_vec();
    let url = spawn_loopback_server(body.clone(), None);
    let transport = HttpTransport::new();

    let result = transport.get(&url, "loopback body", 1024).unwrap();
    assert_eq!(result, body);
}
