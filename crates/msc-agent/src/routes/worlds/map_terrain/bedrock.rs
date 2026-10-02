//! Saved Bedrock tiles exported from a consistent BDS copy.
use std::collections::{BTreeMap, BTreeSet};
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
const MAX_EXPORT_TIME: Duration = Duration::from_secs(600);

#[derive(Clone, Default)]
pub(crate) struct BedrockStore(Arc<Mutex<BedrockState>>);

#[derive(Default)]
struct BedrockState {
    tiles: BTreeMap<String, BedrockTile>,
    snapshot: Option<BedrockSnapshot>,
}

struct BedrockSnapshot {
    server_id: String,
    world: PathBuf,
    snapshot: crate::backup_operations::WorldMapSnapshot,
}

impl Drop for BedrockSnapshot {
    fn drop(&mut self) {
        if let Some(parent) = self.snapshot.path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

struct BedrockTile {
    output: PathBuf,
    tiles: BTreeSet<String>,
}

pub(crate) struct RefreshStats {
    pub reused_tiles: usize,
    pub changed_tiles: usize,
    pub removed_tiles: usize,
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
    if !matches!(
        query.dimension.as_str(),
        "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
    ) {
        return error_response(
            StatusCode::CONFLICT,
            "dimension_unavailable",
            "This Bedrock dimension is not supported by the saved terrain map.",
        );
    }
    if !matches!(query.path.as_str(), "manifest.json" | "terrain.vtexarr")
        && !matches!(query.path.strip_prefix("tiles/t."), Some(name) if {
            let parts: Vec<_> = name.split('.').collect();
            matches!(parts.as_slice(), [x, z, "vtile"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok())
        })
        && !matches!(query.path.strip_prefix("overview/t."), Some(name) if {
            let parts: Vec<_> = name.split('.').collect();
            matches!(parts.as_slice(), [x, z, "vlr"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok())
        })
    {
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
    let dimension = query.dimension.clone();
    match tokio::task::spawn_blocking(move || {
        store.read(
            &lifecycle,
            &server_dir,
            &server_id,
            &world,
            &dimension,
            &artifact,
        )
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
    pub(crate) fn use_snapshot(
        &self,
        server_id: String,
        world: PathBuf,
        snapshot: crate::backup_operations::WorldMapSnapshot,
    ) -> Result<RefreshStats, String> {
        let pending = BedrockSnapshot {
            server_id,
            world,
            snapshot,
        };
        let mut current = self
            .0
            .lock()
            .map_err(|_| "The Bedrock map cache is unavailable.")?;
        let mut next_tiles = BTreeMap::new();
        let mut stats = RefreshStats {
            reused_tiles: 0,
            changed_tiles: 0,
            removed_tiles: 0,
        };
        if let Some(previous) = current
            .snapshot
            .as_ref()
            .filter(|old| old.server_id == pending.server_id && old.world == pending.world)
            && !current.tiles.is_empty()
        {
            let binary = exporter_binary()?;
            let pack = resource_pack()?;
            for (dimension, old) in &current.tiles {
                let output =
                    std::env::temp_dir().join(format!("msc-bedrock-tile-{}", Uuid::new_v4()));
                fs::create_dir(&output).map_err(|error| {
                    format!("Could not prepare refreshed Bedrock tiles: {error}")
                })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&output, fs::Permissions::from_mode(0o700)).map_err(
                        |error| format!("Could not protect refreshed Bedrock tiles: {error}"),
                    )?;
                }
                let mut next = BedrockTile {
                    output,
                    tiles: BTreeSet::new(),
                };
                if !run_exporter(
                    Command::new(&binary)
                        .arg("catalog")
                        .arg(&pending.snapshot.path)
                        .arg(&pack)
                        .arg(&next.output)
                        .arg(dimension),
                ) {
                    return Err(format!(
                        "The refreshed {dimension} catalog could not be exported."
                    ));
                }
                let manifest: serde_json::Value = serde_json::from_slice(
                    &fs::read(next.output.join("manifest.json"))
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("The refreshed Bedrock catalog is invalid: {error}"))?;
                let tiles = manifest["tiles"]
                    .as_array()
                    .ok_or("The refreshed Bedrock catalog has no tile list.")?
                    .iter()
                    .map(|tile| {
                        tile["path"]
                            .as_str()
                            .filter(|path| valid_tile_path(path))
                            .map(str::to_owned)
                            .ok_or("The refreshed Bedrock catalog has an invalid tile path.")
                    })
                    .collect::<Result<BTreeSet<_>, _>>()?;
                let diff_path = next.output.join("diff.json");
                if !run_exporter(
                    Command::new(&binary)
                        .arg("diff-catalogs")
                        .arg(&previous.snapshot.path)
                        .arg(&pending.snapshot.path)
                        .arg(dimension)
                        .arg(old.output.join("manifest.json"))
                        .arg(next.output.join("manifest.json"))
                        .arg(&diff_path),
                ) {
                    return Err(format!(
                        "The refreshed {dimension} chunks could not be compared."
                    ));
                }
                let diff: serde_json::Value = serde_json::from_slice(
                    &fs::read(&diff_path).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("The Bedrock tile comparison is invalid: {error}"))?;
                let unchanged = diff["unchangedTiles"]
                    .as_array()
                    .ok_or("The Bedrock tile comparison has no unchanged tile list.")?;
                for path in unchanged {
                    let path = path
                        .as_str()
                        .filter(|path| {
                            valid_tile_path(path)
                                && old.tiles.contains(*path)
                                && tiles.contains(*path)
                        })
                        .ok_or("The Bedrock tile comparison named an invalid tile.")?;
                    let old_file = old.output.join(path);
                    if old_file.is_file() {
                        fs::copy(old_file, next.output.join(path)).map_err(|error| {
                            format!("Could not keep an unchanged Bedrock tile: {error}")
                        })?;
                        stats.reused_tiles += 1;
                    }
                }
                for name in ["terrain.vtexarr", "texture-index.json"] {
                    fs::copy(old.output.join(name), next.output.join(name))
                        .map_err(|error| format!("Could not keep Bedrock textures: {error}"))?;
                }
                stats.changed_tiles += diff["changedTiles"]
                    .as_u64()
                    .ok_or("The Bedrock tile comparison has no changed count.")?
                    as usize;
                stats.removed_tiles += diff["removedTiles"]
                    .as_u64()
                    .ok_or("The Bedrock tile comparison has no removed count.")?
                    as usize;
                let _ = fs::remove_file(diff_path);
                next.tiles = tiles;
                next_tiles.insert(dimension.to_owned(), next);
            }
        }
        current.tiles = next_tiles;
        current.snapshot = Some(pending);
        Ok(stats)
    }

    fn read(
        &self,
        lifecycle: &crate::routes::lifecycle::LifecycleRoutesState,
        server_dir: &Path,
        server_id: &str,
        world: &Path,
        dimension: &str,
        artifact: &str,
    ) -> Result<Vec<u8>, String> {
        let mut current = self
            .0
            .lock()
            .map_err(|_| "The Bedrock map cache is unavailable.")?;
        if current
            .snapshot
            .as_ref()
            .is_some_and(|saved| saved.server_id != server_id || saved.world != world)
        {
            current.tiles.clear();
            current.snapshot = None;
        }
        if !current.tiles.contains_key(dimension) {
            let pack = resource_pack()?;
            let binary = exporter_binary()?;
            let output = std::env::temp_dir().join(format!("msc-bedrock-tile-{}", Uuid::new_v4()));
            fs::create_dir(&output)
                .map_err(|error| format!("Could not prepare the Bedrock map: {error}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&output, fs::Permissions::from_mode(0o700))
                    .map_err(|error| format!("Could not protect the Bedrock map: {error}"))?;
            }
            if current.snapshot.is_none() {
                let copied = if lifecycle.status_snapshot().running {
                    crate::backup_operations::snapshot_bedrock_world(
                        lifecycle.clone(),
                        server_dir,
                        || false,
                    )
                } else {
                    crate::backup_operations::snapshot_stopped_bedrock_world(server_dir)
                };
                match copied {
                    Ok(snapshot) => {
                        current.snapshot = Some(BedrockSnapshot {
                            server_id: server_id.to_owned(),
                            world: world.to_owned(),
                            snapshot,
                        })
                    }
                    Err(error) => {
                        let _ = fs::remove_dir_all(&output);
                        return Err(error);
                    }
                }
            }
            let source = current
                .snapshot
                .as_ref()
                .map(|saved| saved.snapshot.path.as_path())
                .unwrap_or(world);
            let rendered = Command::new(binary)
                .arg("catalog")
                .arg(source)
                .arg(&pack)
                .arg(&output)
                .arg(dimension)
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
            if !rendered.is_ok_and(|success| success) {
                let _ = fs::remove_dir_all(&output);
                return Err("The saved Bedrock catalog could not be exported. This dimension may have no generated chunks; also check the world and resource pack.".into());
            }
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(output.join("manifest.json")).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("The Bedrock tile catalog is invalid: {error}"))?;
            let tiles = manifest["tiles"]
                .as_array()
                .ok_or("The Bedrock tile catalog has no tile list.")?
                .iter()
                .filter_map(|tile| tile["path"].as_str().map(str::to_owned))
                .collect();
            current
                .tiles
                .insert(dimension.to_owned(), BedrockTile { output, tiles });
        }
        let tile = current
            .tiles
            .get(dimension)
            .ok_or("The Bedrock tile is unavailable.")?;
        if tile.tiles.is_empty() {
            return Err(format!(
                "No saved terrain has been generated in {dimension}."
            ));
        }
        let path = tile.output.join(artifact);
        if artifact.starts_with("tiles/") {
            if !tile.tiles.contains(artifact) {
                return Err("This Bedrock tile has no saved chunks.".into());
            }
            if !path.is_file() {
                let name = artifact
                    .strip_prefix("tiles/t.")
                    .ok_or("Invalid Bedrock tile path")?;
                let parts: Vec<_> = name.split('.').collect();
                let [x, z, "vtile"] = parts.as_slice() else {
                    return Err("Invalid Bedrock tile path".into());
                };
                let x = x
                    .parse::<i32>()
                    .map_err(|_| "Invalid Bedrock tile x")?
                    .checked_mul(4)
                    .ok_or("Bedrock tile x exceeds world bounds")?;
                let z = z
                    .parse::<i32>()
                    .map_err(|_| "Invalid Bedrock tile z")?
                    .checked_mul(4)
                    .ok_or("Bedrock tile z exceeds world bounds")?;
                let source = current
                    .snapshot
                    .as_ref()
                    .map_or(world, |saved| saved.snapshot.path.as_path());
                let binary = exporter_binary()?;
                let pack = resource_pack()?;
                let started = Instant::now();
                eprintln!("[world-map] Rendering Bedrock {dimension} tile {x},{z}");
                let rendered = run_exporter(
                    Command::new(binary)
                        .arg("tile")
                        .arg(source)
                        .arg(pack)
                        .arg(&tile.output)
                        .arg(dimension)
                        .arg(x.to_string())
                        .arg(z.to_string()),
                );
                eprintln!(
                    "[world-map] Bedrock {dimension} tile {x},{z}: {} after {:.2}s",
                    if rendered { "ready" } else { "failed" },
                    started.elapsed().as_secs_f64()
                );
                if !rendered {
                    return Err("The saved Bedrock tile could not be exported. See the agent log for the exporter error.".into());
                }
            }
        }
        let metadata =
            fs::metadata(&path).map_err(|_| "The Bedrock tile artifact is unavailable.")?;
        if metadata.len() > MAX_ARTIFACT as u64 {
            return Err("The Bedrock tile artifact exceeds the map size limit.".into());
        }
        fs::read(path).map_err(|error| format!("Could not read the Bedrock tile: {error}"))
    }
}

fn valid_tile_path(path: &str) -> bool {
    let Some(name) = path
        .strip_prefix("tiles/t.")
        .and_then(|name| name.strip_suffix(".vtile"))
    else {
        return false;
    };
    let Some((x, z)) = name.split_once('.') else {
        return false;
    };
    x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok()
}

fn exporter_binary() -> Result<PathBuf, String> {
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
    Ok(binary)
}

fn run_exporter(command: &mut Command) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Keep exporter failures in the agent log rather than hiding them
        // behind a generic HTTP error and the viewer's retry loop.
        .stderr(Stdio::inherit())
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
        })
        .unwrap_or(false)
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
    let pack = root.join("v1.26.50.4-models");
    if pack.join("textures/terrain_texture.json").is_file() && pack.join("blocks.json").is_file() {
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
    if pack.join("textures/terrain_texture.json").is_file() && pack.join("blocks.json").is_file() {
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
        if !(relative.starts_with("textures/") || relative == "blocks.json")
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
    if !pack.join("textures/terrain_texture.json").is_file() || !pack.join("blocks.json").is_file()
    {
        return Err("The verified Bedrock archive did not include terrain textures.".into());
    }
    Ok(())
}
