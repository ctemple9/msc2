//! First-use Java map dependencies, kept outside the Minecraft installation.
#[cfg(test)]
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use msc_infrastructure::config_repository::default_app_data_dir;
use msc_infrastructure::jar_provider::{HttpTransport, Transport};
use serde::Deserialize;
use sha1::Digest;
use sha1::Sha1;

use super::TerrainError;

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MAX_METADATA: u64 = 20 * 1024 * 1024;
const MAX_JAR: u64 = 300 * 1024 * 1024;
const MAX_EXTRACTED: u64 = 512 * 1024 * 1024;
const REQUIRED_ASSETS: [&str; 3] = [
    "assets/minecraft/blockstates/stone.json",
    "assets/minecraft/models/block/stone.json",
    "assets/minecraft/textures/block/stone.png",
];

pub(super) fn binary() -> Result<PathBuf, TerrainError> {
    if let Some(path) = std::env::var_os("MSC2_VANTAGE_BIN") {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(TerrainError::new(
                "renderer_missing",
                "The configured Java terrain renderer is missing. Correct MSC2_VANTAGE_BIN on the server host.",
            ))
        };
    }
    let name = if cfg!(windows) {
        "vantage.exe"
    } else {
        "vantage"
    };
    if let Some(path) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join(name)))
        .filter(|path| path.is_file())
    {
        return Ok(path);
    }
    // MSC's persistent tile format requires its bundled renderer change.
    // Falling back to upstream 0.15.1 would silently rebake every cached tile.
    Err(TerrainError::new(
        "renderer_missing",
        "The Java terrain renderer is missing. Repair MSC on the server host to restore the matching renderer.",
    ))
}

pub(super) fn assets(
    world: &Path,
    selected_version: Option<&str>,
) -> Result<PathBuf, TerrainError> {
    let version = saved_version(world).or_else(|| selected_version.filter(|v| safe_version(v)).map(str::to_owned))
        .ok_or_else(|| TerrainError::new("client_assets_unavailable", "Minecraft's version could not be determined for this saved world. Select its Minecraft version on the server host, then reopen the map."))?;
    ensure_assets(&HttpTransport::new(), &default_app_data_dir().join("map-dependencies").join("java-assets"), &version)
        .map_err(|error| setup_error("client_assets_unavailable", &format!("prepare Minecraft {version} map textures; check the host's internet connection and cache permissions, then reopen the map"), error))
}

fn saved_version(world: &Path) -> Option<String> {
    let file = fs::File::open(world.join("level.dat")).ok()?;
    let mut decoded = Vec::new();
    GzDecoder::new(file)
        .take(MAX_METADATA + 1)
        .read_to_end(&mut decoded)
        .ok()?;
    if decoded.len() as u64 > MAX_METADATA {
        return None;
    }
    let root: fastnbt::Value = fastnbt::from_bytes(&decoded).ok()?;
    let fastnbt::Value::Compound(root) = root else {
        return None;
    };
    let fastnbt::Value::Compound(data) = root.get("Data")? else {
        return None;
    };
    let fastnbt::Value::Compound(version) = data.get("Version")? else {
        return None;
    };
    let fastnbt::Value::String(name) = version.get("Name")? else {
        return None;
    };
    safe_version(name).then(|| name.clone())
}

fn safe_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 100
        && version.as_bytes()[0].is_ascii_alphanumeric()
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<VersionEntry>,
}
#[derive(Deserialize)]
struct VersionEntry {
    id: String,
    url: String,
    sha1: String,
}
#[derive(Deserialize)]
struct VersionMetadata {
    id: String,
    downloads: Downloads,
}
#[derive(Deserialize)]
struct Downloads {
    client: ClientDownload,
}
#[derive(Deserialize)]
struct ClientDownload {
    url: String,
    sha1: String,
    size: u64,
}

fn ensure_assets(
    transport: &dyn Transport,
    cache: &Path,
    version: &str,
) -> Result<PathBuf, String> {
    if !safe_version(version) {
        return Err("invalid Minecraft version".into());
    }
    let root = cache.join(version);
    let assets = root.join("assets/minecraft");
    if fs::read_to_string(root.join(".complete")).ok().as_deref() == Some(version)
        && REQUIRED_ASSETS.iter().all(|path| root.join(path).is_file())
    {
        return Ok(assets);
    }
    eprintln!("Java map: preparing official Minecraft {version} client assets");
    let manifest: Manifest = serde_json::from_slice(
        &transport
            .get(MANIFEST, "Minecraft version manifest", MAX_METADATA)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == version)
        .ok_or_else(|| format!("Mojang has no client assets for Minecraft {version}"))?;
    official_url(&entry.url)?;
    let metadata_bytes = transport
        .get(&entry.url, "Minecraft version metadata", MAX_METADATA)
        .map_err(|e| e.to_string())?;
    verify_sha1(&metadata_bytes, &entry.sha1)?;
    let metadata: VersionMetadata =
        serde_json::from_slice(&metadata_bytes).map_err(|e| e.to_string())?;
    if metadata.id != version {
        return Err("Minecraft client metadata version mismatch".into());
    }
    let client = metadata.downloads.client;
    official_url(&client.url)?;
    if client.size == 0 || client.size > MAX_JAR {
        return Err("Minecraft client exceeds the download size limit".into());
    }
    let jar = transport
        .get(&client.url, "Minecraft client assets", client.size)
        .map_err(|e| e.to_string())?;
    if jar.len() as u64 != client.size {
        return Err("Minecraft client download is incomplete".into());
    }
    verify_sha1(&jar, &client.sha1)?;
    let staging = Staging::new(&root).map_err(|e| e.to_string())?;
    extract_assets(&jar, &staging.0)?;
    fs::write(staging.0.join(".complete"), version).map_err(|e| e.to_string())?;
    staging.publish(&root).map_err(|e| e.to_string())?;
    Ok(assets)
}

fn official_url(url: &str) -> Result<(), String> {
    let uri: ureq::http::Uri = url.parse().map_err(|_| "invalid Minecraft download URL")?;
    let host = uri.authority().map(|a| a.as_str()).unwrap_or_default();
    if uri.scheme_str() != Some("https")
        || !matches!(
            host,
            "piston-meta.mojang.com"
                | "piston-data.mojang.com"
                | "launchermeta.mojang.com"
                | "launcher.mojang.com"
        )
    {
        return Err("Minecraft metadata supplied an unofficial download URL".into());
    }
    Ok(())
}

fn verify_sha1(bytes: &[u8], expected: &str) -> Result<(), String> {
    if expected.len() != 40 || format!("{:x}", Sha1::digest(bytes)) != expected.to_ascii_lowercase()
    {
        return Err("Minecraft client assets failed checksum verification".into());
    }
    Ok(())
}

fn extract_assets(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() > 100_000 {
        return Err("Minecraft client has too many archive entries".into());
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if !entry.name().starts_with("assets/minecraft/")
            && !entry.name().starts_with("data/minecraft/")
        {
            continue;
        }
        let relative = entry
            .enclosed_name()
            .ok_or("Minecraft assets contain an unsafe archive path")?;
        if entry.is_symlink() {
            return Err("Minecraft assets contain an unsupported link".into());
        }
        if entry.is_dir() {
            continue;
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Minecraft assets size overflow")?;
        if total > MAX_EXTRACTED || entry.size() > 32 * 1024 * 1024 {
            return Err("Minecraft assets exceed the extraction size limit".into());
        }
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().ok_or("asset has no parent")?)
            .map_err(|e| e.to_string())?;
        let mut file = fs::File::create(target).map_err(|e| e.to_string())?;
        let expected_size = entry.size();
        let written = std::io::copy(&mut entry.by_ref().take(expected_size + 1), &mut file)
            .map_err(|e| e.to_string())?;
        if written != expected_size {
            return Err("Minecraft asset entry size mismatch".into());
        }
    }
    if !REQUIRED_ASSETS
        .iter()
        .all(|path| destination.join(path).is_file())
    {
        return Err("Minecraft client is missing required block textures and models".into());
    }
    Ok(())
}

struct Staging(PathBuf);
impl Staging {
    fn new(destination: &Path) -> std::io::Result<Self> {
        let parent = destination
            .parent()
            .ok_or_else(|| std::io::Error::other("dependency cache has no parent"))?;
        fs::create_dir_all(parent)?;
        let path = parent.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    fn publish(&self, destination: &Path) -> std::io::Result<()> {
        if destination.exists() {
            fs::remove_dir_all(destination)?;
        }
        fs::rename(&self.0, destination)
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup_error(code: &'static str, action: &str, detail: impl std::fmt::Display) -> TerrainError {
    eprintln!("Java map dependency setup could not {action}: {detail}");
    TerrainError::new(
        code,
        format!("MSC could not {action}. See the agent logs for details."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::jar_provider::JarProviderError;
    use std::io::Write;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("msc-map-assets-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct FakeTransport(HashMap<String, Vec<u8>>);
    impl Transport for FakeTransport {
        fn get(&self, url: &str, _: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
            let body = self
                .0
                .get(url)
                .ok_or_else(|| JarProviderError::Network("unexpected request".into()))?;
            assert!(body.len() as u64 <= max_bytes);
            Ok(body.clone())
        }
    }
    fn jar(extra: Option<&str>) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in REQUIRED_ASSETS.iter().copied().chain(extra) {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"fixture").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn transport(bytes: Vec<u8>, digest: String) -> FakeTransport {
        let metadata_url = "https://piston-meta.mojang.com/test/version.json";
        let client_url = "https://piston-data.mojang.com/test/client.jar";
        let metadata = serde_json::to_vec(&serde_json::json!({
            "id": "1.21.1", "downloads": { "client": {
                "url": client_url, "sha1": digest, "size": bytes.len()
            }}
        }))
        .unwrap();
        FakeTransport(HashMap::from([
            (MANIFEST.into(), serde_json::to_vec(&serde_json::json!({"versions": [
                {"id": "26.3", "url": "https://piston-meta.mojang.com/newer.json", "sha1": "0".repeat(40)},
                {"id": "1.21.1", "url": metadata_url, "sha1": format!("{:x}", Sha1::digest(&metadata))}
            ]})).unwrap()),
            (metadata_url.into(), metadata),
            (client_url.into(), bytes),
        ]))
    }

    #[test]
    fn matching_assets_are_verified_cached_and_incomplete_cache_is_repaired() {
        let fixture = Fixture::new();
        let bytes = jar(Some("net/minecraft/Unused.class"));
        let transport = transport(bytes.clone(), format!("{:x}", Sha1::digest(&bytes)));
        let assets = ensure_assets(&transport, &fixture.0, "1.21.1").unwrap();
        assert!(assets.join("textures/block/stone.png").is_file());
        assert!(!fixture.0.join("1.21.1/net").exists());
        assert_eq!(
            ensure_assets(&FakeTransport(HashMap::new()), &fixture.0, "1.21.1").unwrap(),
            assets
        );
        fs::remove_file(assets.join("textures/block/stone.png")).unwrap();
        assert!(ensure_assets(&FakeTransport(HashMap::new()), &fixture.0, "1.21.1").is_err());
        ensure_assets(&transport, &fixture.0, "1.21.1").unwrap();
        assert!(assets.join("textures/block/stone.png").is_file());
    }

    #[test]
    fn bad_download_or_unsafe_archive_never_publishes_assets() {
        let fixture = Fixture::new();
        let valid = jar(None);
        assert!(ensure_assets(&transport(valid, "0".repeat(40)), &fixture.0, "1.21.1").is_err());
        assert!(!fixture.0.join("1.21.1").exists());
        let unsafe_jar = jar(Some("assets/minecraft/../../../escaped"));
        let digest = format!("{:x}", Sha1::digest(&unsafe_jar));
        assert!(ensure_assets(&transport(unsafe_jar, digest), &fixture.0, "1.21.1").is_err());
        assert!(!fixture.0.join("1.21.1").exists());
        assert!(!fixture.0.join("escaped").exists());
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
    }
}
