//! A saved Bedrock tile, exported from a consistent BDS copy on first load.
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use msc_domain::app_config_schema::ConfigServer;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{ArtifactQuery, MAX_ARTIFACT, is_direct_child};
use crate::routes::worlds::{WorldsRoutesState, error_response, is_safe_level_folder};
use msc_infrastructure::fs::StdFileSystem;

const SAMPLES_URL: &str = "https://github.com/Mojang/bedrock-samples/releases/download/v1.26.50.4/bedrock-samples-v1.26.50.4-full.zip";
const SAMPLES_SHA256: &str = "c0b6151f5f9a0c31ebe3c909dfc82d97f12473ed2c6e039aa7e22d667042026c";
const MAX_DOWNLOAD: u64 = 180 * 1024 * 1024;
const MAX_EXTRACTED: u64 = 384 * 1024 * 1024;
const MAX_EXPORT_TIME: Duration = Duration::from_secs(180);

#[derive(Clone, Default)]
pub(crate) struct BedrockStore(Arc<Mutex<Option<BedrockTile>>>);

struct BedrockTile {
    server_id: String,
    world: PathBuf,
    output: PathBuf,
}

impl Drop for BedrockTile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.output);
    }
}

pub(super) async fn artifact(
    state: &WorldsRoutesState,
    server: &ConfigServer,
    query: &ArtifactQuery,
    content_type: &'static str,
) -> Response {
    if query.dimension != "minecraft:overworld" {
        return error_response(
            StatusCode::CONFLICT,
            "dimension_unavailable",
            "Only saved Bedrock Overworld terrain is available in this map.",
        );
    }
    if !matches!(
        query.path.as_str(),
        "terrain.vtile" | "terrain.vtexarr" | "viewer-world.json"
    ) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_artifact",
            "The requested Bedrock terrain artifact is not supported.",
        );
    }
    let server_dir = PathBuf::from(&server.server_dir);
    let Some(level_name) =
        msc_application::worlds::read_configured_level_name(&StdFileSystem, &server_dir)
            .filter(|name| is_safe_level_folder(name))
    else {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active server has no safe configured Bedrock world folder.",
        );
    };
    let worlds = server_dir.join("worlds");
    let world = worlds.join(level_name);
    if !is_direct_child(&server_dir, &worlds)
        || !is_direct_child(&worlds, &world)
        || !world.join("level.dat").is_file()
        || !world.join("db").is_dir()
    {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active Bedrock world has no safely accessible saved terrain.",
        );
    }
    let store = state.bedrock_map.clone();
    let lifecycle = state.lifecycle.clone();
    let server_id = server.id.clone();
    let artifact = query.path.clone();
    match tokio::task::spawn_blocking(move || {
        store.read(&lifecycle, &server_dir, &server_id, &world, &artifact)
    })
    .await
    {
        Ok(Ok(bytes)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            Body::from(bytes),
        )
            .into_response(),
        Ok(Err(error)) => {
            error_response(StatusCode::BAD_GATEWAY, "bedrock_map_unavailable", &error)
        }
        Err(_) => error_response(
            StatusCode::BAD_GATEWAY,
            "bedrock_map_unavailable",
            "The Bedrock terrain exporter stopped unexpectedly.",
        ),
    }
}

impl BedrockStore {
    fn read(
        &self,
        lifecycle: &crate::routes::lifecycle::LifecycleRoutesState,
        server_dir: &Path,
        server_id: &str,
        world: &Path,
        artifact: &str,
    ) -> Result<Vec<u8>, String> {
        let mut current = self
            .0
            .lock()
            .map_err(|_| "The Bedrock map cache is unavailable.")?;
        let reuse = current
            .as_ref()
            .is_some_and(|tile| tile.server_id == server_id && tile.world == world);
        if !reuse {
            *current = None;
            let pack = resource_pack()?;
            let binary = std::env::var_os("MSC2_BEDROCK_MAP_BIN")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::current_exe().ok()?.parent().map(|path| {
                        path.join(if cfg!(windows) {
                            "bedrock-map.exe"
                        } else {
                            "bedrock-map"
                        })
                    })
                })
                .ok_or("The Bedrock terrain exporter could not be located.")?;
            if !binary.is_file() {
                return Err("The Bedrock terrain exporter is missing beside the MSC agent.".into());
            }
            let output = std::env::temp_dir().join(format!("msc-bedrock-tile-{}", Uuid::new_v4()));
            fs::create_dir(&output)
                .map_err(|error| format!("Could not prepare the Bedrock map: {error}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&output, fs::Permissions::from_mode(0o700))
                    .map_err(|error| format!("Could not protect the Bedrock map: {error}"))?;
            }
            let snapshot = if lifecycle.status_snapshot().running {
                match crate::backup_operations::snapshot_bedrock_world(
                    lifecycle.clone(),
                    server_dir,
                    || false,
                ) {
                    Ok(snapshot) => Some(snapshot),
                    Err(error) => {
                        let _ = fs::remove_dir_all(&output);
                        return Err(error);
                    }
                }
            } else {
                None
            };
            let source = snapshot
                .as_ref()
                .map_or(world, |snapshot| snapshot.path.as_path());
            let rendered = Command::new(binary)
                .arg(source)
                .arg(&pack)
                .arg(&output)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .and_then(|mut child| {
                    let started = Instant::now();
                    loop {
                        if let Some(status) = child.try_wait()? {
                            break Ok(status.success());
                        }
                        if started.elapsed() >= MAX_EXPORT_TIME {
                            let _ = child.kill();
                            let _ = child.wait();
                            break Ok(false);
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                });
            if let Some(snapshot) = snapshot
                && let Some(parent) = snapshot.path.parent()
            {
                let _ = fs::remove_dir_all(parent);
            }
            if !rendered.is_ok_and(|success| success) {
                let _ = fs::remove_dir_all(&output);
                return Err("The saved Bedrock tile could not be exported. Check the world and resource pack.".into());
            }
            *current = Some(BedrockTile {
                server_id: server_id.to_owned(),
                world: world.to_owned(),
                output,
            });
        }
        let path = current
            .as_ref()
            .ok_or("The Bedrock tile is unavailable.")?
            .output
            .join(artifact);
        let metadata =
            fs::metadata(&path).map_err(|_| "The Bedrock tile artifact is unavailable.")?;
        if metadata.len() > MAX_ARTIFACT as u64 {
            return Err("The Bedrock tile artifact exceeds the map size limit.".into());
        }
        fs::read(path).map_err(|error| format!("Could not read the Bedrock tile: {error}"))
    }
}

fn resource_pack() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MSC2_BEDROCK_RESOURCE_PACK") {
        let path = PathBuf::from(path);
        if path.join("textures/terrain_texture.json").is_file() {
            return Ok(path);
        }
        return Err(
            "MSC2_BEDROCK_RESOURCE_PACK does not contain textures/terrain_texture.json.".into(),
        );
    }
    let base = if let Some(path) = std::env::var_os("MSC2_DATA_DIR") {
        PathBuf::from(path)
    } else {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(
                std::env::var_os("HOME")
                    .ok_or("HOME is unavailable for Bedrock texture storage.")?,
            )
            .join("Library/Application Support/MSC2")
        }
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(
                std::env::var_os("LOCALAPPDATA")
                    .ok_or("LOCALAPPDATA is unavailable for Bedrock texture storage.")?,
            )
            .join("MSC2")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from(
                std::env::var_os("HOME")
                    .ok_or("HOME is unavailable for Bedrock texture storage.")?,
            )
            .join(".local/share/msc2")
        }
    };
    let root = base.join("bedrock-map-assets");
    let pack = root.join("v1.26.50.4");
    if pack.join("textures/terrain_texture.json").is_file() {
        return Ok(pack);
    }
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not prepare Bedrock textures: {error}"))?;
    let staging = root.join(format!("download-{}", Uuid::new_v4()));
    fs::create_dir(&staging)
        .map_err(|error| format!("Could not prepare Bedrock textures: {error}"))?;
    let result = download_resource_pack(&staging);
    let installed = result.and_then(|()| {
        fs::rename(staging.join("pack"), &pack)
            .map_err(|error| format!("Could not install verified Bedrock textures: {error}"))
    });
    let _ = fs::remove_dir_all(&staging);
    installed?;
    if pack.join("textures/terrain_texture.json").is_file() {
        Ok(pack)
    } else {
        Err("The verified Bedrock texture pack could not be installed.".into())
    }
}

fn download_resource_pack(staging: &Path) -> Result<(), String> {
    let client: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let response = client
        .get(SAMPLES_URL)
        .call()
        .map_err(|error| format!("Could not download official Bedrock sample textures: {error}"))?;
    let archive_path = staging.join("samples.zip");
    let mut archive_file = File::create(&archive_path).map_err(|error| error.to_string())?;
    let copied = std::io::copy(
        &mut response.into_body().as_reader().take(MAX_DOWNLOAD + 1),
        &mut archive_file,
    )
    .map_err(|error| format!("Bedrock texture download failed: {error}"))?;
    if copied > MAX_DOWNLOAD {
        return Err("Bedrock texture download exceeded its size limit.".into());
    }
    archive_file.flush().map_err(|error| error.to_string())?;
    let mut checksum_file = File::open(&archive_path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut checksum_file, &mut hasher).map_err(|error| error.to_string())?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != SAMPLES_SHA256 {
        return Err("The Bedrock texture download failed its checksum check.".into());
    }
    let mut archive =
        zip::ZipArchive::new(File::open(&archive_path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("Could not open verified Bedrock textures: {error}"))?;
    let pack = staging.join("pack");
    let mut extracted = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name();
        let Some(relative) = name
            .strip_prefix("resource_pack/")
            .or_else(|| name.split_once("/resource_pack/").map(|(_, rest)| rest))
        else {
            continue;
        };
        if !relative.starts_with("textures/")
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            continue;
        }
        let relative = Path::new(relative);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            continue;
        }
        extracted = extracted
            .checked_add(entry.size())
            .ok_or("Bedrock texture archive size overflow")?;
        if extracted > MAX_EXTRACTED {
            return Err("Bedrock textures exceed the extraction size limit.".into());
        }
        let destination = pack.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(destination).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }
    if !pack.join("textures/terrain_texture.json").is_file() {
        return Err("The verified Bedrock archive did not include terrain textures.".into());
    }
    Ok(())
}
