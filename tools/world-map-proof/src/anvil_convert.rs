//! Durable Bedrock-to-Anvil preparation for the shared Vantage renderer.
use crate::anvil_probe::{Findings, encode_chunk, encode_region};
use bedrock_world::{
    BedrockWorld, BiomeDataRequirement, ChunkDataRequest, ChunkLoadOptions, Dimension, OpenOptions,
    WorldScanOptions,
};
use flate2::{Compression, write::GzEncoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    converter: String,
    source_version: Vec<u64>,
    biome_mapping: String,
    regions: BTreeMap<String, RegionFingerprint>,
}

#[derive(Clone, Serialize, Deserialize)]
struct RegionFingerprint {
    converted_content_sha256: String,
    region_file_sha256: String,
}

struct RegionTask {
    dimension: Dimension,
    x: i32,
    z: i32,
    positions: Vec<bedrock_world::ChunkPos>,
    fingerprint: Option<RegionFingerprint>,
}

struct RegionWork {
    dimension: Dimension,
    x: i32,
    z: i32,
    digest: String,
    bytes: Option<Vec<u8>>,
    findings: Findings,
    read_time: Duration,
    nbt_time: Duration,
    encode_time: Duration,
}

#[derive(Serialize)]
struct JavaSpawnLevel {
    #[serde(rename = "Data")]
    data: JavaSpawnData,
}

#[derive(Serialize)]
struct JavaSpawnData {
    #[serde(rename = "SpawnX")]
    x: i32,
    #[serde(rename = "SpawnY")]
    y: i32,
    #[serde(rename = "SpawnZ")]
    z: i32,
}

fn write_private_spawn_level(
    source: &Path,
    output: &Path,
    spawn: Option<(i32, i32, i32)>,
) -> Result<(), Box<dyn Error>> {
    let target = output.join("level.dat");
    let temporary = output.join("level.dat.tmp");
    if let Some((x, y, z)) = spawn {
        // Vantage consumes Java NBT. Only the private converted input gets this
        // small compatibility document; the Bedrock save remains untouched.
        let nbt = fastnbt::to_bytes(&JavaSpawnLevel {
            data: JavaSpawnData { x, y, z },
        })?;
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&nbt)?;
        fs::write(&temporary, gzip.finish()?)?;
    } else {
        // An unreadable/missing spawn retains Vantage's populated-bounds fallback.
        fs::copy(source.join("level.dat"), &temporary)?;
    }
    fs::rename(temporary, target)?;
    Ok(())
}

fn selected_region(value: &str) -> Result<(Dimension, i32, i32), Box<dyn Error>> {
    let mut parts = value.split(':');
    let dimension = match parts.next() {
        Some("overworld") => Dimension::Overworld,
        Some("nether") => Dimension::Nether,
        Some("end") => Dimension::End,
        _ => return Err("region dimension must be overworld, nether, or end".into()),
    };
    let x = parts.next().ok_or("region x is missing")?.parse()?;
    let z = parts.next().ok_or("region z is missing")?.parse()?;
    if parts.next().is_some() {
        return Err("region selector must be dimension:x:z".into());
    }
    Ok((dimension, x, z))
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = Path::new(
        args.get(2)
            .ok_or("anvil-convert requires a read-only Bedrock snapshot")?,
    );
    let output = Path::new(
        args.get(3)
            .ok_or("anvil-convert requires a private output directory")?,
    );
    if !source.join("db").is_dir() || !source.join("level.dat").is_file() {
        return Err("source must be an offline Bedrock world snapshot".into());
    }
    fs::create_dir_all(output)?;
    let source = source.canonicalize()?;
    let output = output.canonicalize()?;
    if source.starts_with(&output) || output.starts_with(&source) {
        return Err("converted output must be separate from the source snapshot".into());
    }
    let open_started = Instant::now();
    let world = BedrockWorld::open_blocking(&source, OpenOptions::default())?;
    eprintln!(
        "Bedrock prepare: LevelDB open {:.1}s",
        open_started.elapsed().as_secs_f64()
    );
    let level = world.read_level_dat_blocking()?;
    let NbtTag::Compound(root) = level.root else {
        return Err("level.dat root is not a compound".into());
    };
    let spawn = match (root.get("SpawnX"), root.get("SpawnY"), root.get("SpawnZ")) {
        (Some(NbtTag::Int(x)), Some(NbtTag::Int(y)), Some(NbtTag::Int(z)))
            if (-64..320).contains(y) =>
        {
            Some((*x, *y, *z))
        }
        _ => None,
    };
    let version = root
        .get("lastOpenedWithVersion")
        .and_then(|v| match v {
            NbtTag::List(xs) => Some(xs),
            _ => None,
        })
        .ok_or("save has no lastOpenedWithVersion")?
        .iter()
        .map(|v| match v {
            NbtTag::Int(n) if *n >= 0 => Ok(*n as u64),
            _ => Err("invalid save version"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut selected = BTreeSet::new();
    let mut registry_path = None;
    let mut server_running = false;
    for arg in args.iter().skip(4) {
        if let Some(value) = arg.strip_prefix("--region=") {
            if !selected.insert(selected_region(value)?) || selected.len() > 8 {
                return Err("choose one to eight distinct saved regions".into());
            }
        } else if arg == "--server-running" {
            server_running = true;
        } else if arg.starts_with('-') || registry_path.replace(arg).is_some() {
            return Err("unexpected converter argument".into());
        }
    }
    let mut biome_names = BTreeMap::new();
    let exact_biome_registry = registry_path.is_some();
    let registry_sha256 = if let Some(registry_path) = registry_path {
        let registry_bytes = fs::read(registry_path)?;
        let registry: serde_json::Value = serde_json::from_slice(&registry_bytes)?;
        let registry_version = registry["version"]
            .as_array()
            .ok_or("registry version must be an integer array")?
            .iter()
            .map(|v| v.as_u64().ok_or("invalid registry version component"))
            .collect::<Result<Vec<_>, _>>()?;
        if version != registry_version {
            return Err(format!(
                "biome registry version {registry_version:?} does not match save version {version:?}"
            )
            .into());
        }
        let declared_source_hash = registry["source_sha256"]
            .as_str()
            .ok_or("registry source_sha256 is required")?;
        if declared_source_hash.len() != 64
            || !declared_source_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("registry source_sha256 must be a 64-character hex digest".into());
        }
        let ids = registry["ids"]
            .as_object()
            .ok_or("registry ids must map names to numeric IDs")?;
        for (name, id) in ids {
            let id = u32::try_from(id.as_u64().ok_or("invalid biome ID")?)?;
            let normalized_name = format!(
                "minecraft:{}",
                name.strip_prefix("minecraft:").unwrap_or(name)
            );
            if name.trim().is_empty() || biome_names.insert(id, normalized_name).is_some() {
                return Err("registry contains an empty name or duplicate numeric biome ID".into());
            }
        }
        hex(&Sha256::digest(&registry_bytes))
    } else {
        "neutral-unmapped-biomes-v1".to_owned()
    };
    let mut manifest = read_manifest(&output)?.unwrap_or(Manifest {
        format: 2,
        converter: "bedrock-anvil-v3".into(),
        source_version: version.clone(),
        biome_mapping: registry_sha256.clone(),
        regions: BTreeMap::new(),
    });
    if manifest.format != 2
        || manifest.converter != "bedrock-anvil-v3"
        || manifest.source_version != version
        || manifest.biome_mapping != registry_sha256
    {
        return Err("prepared cache metadata differs; use a new output directory".into());
    }
    let scan = Instant::now();
    let positions = world.list_render_chunk_positions_blocking(WorldScanOptions::default())?;
    let mut regions = BTreeMap::<(Dimension, i32, i32), Vec<_>>::new();
    for pos in positions {
        if matches!(pos.dimension, Dimension::Unknown(_)) {
            return Err(format!(
                "unsupported saved dimension {:?}; refusing to omit its terrain",
                pos.dimension
            )
            .into());
        }
        regions
            .entry((pos.dimension, pos.x.div_euclid(32), pos.z.div_euclid(32)))
            .or_default()
            .push(pos);
    }
    let discovered = regions.len();
    if !selected.is_empty() {
        regions.retain(|key, _| selected.contains(key));
        if regions.len() != selected.len() {
            return Err("one or more selected regions have no saved chunks".into());
        }
    }
    for positions in regions.values_mut() {
        positions.sort_unstable_by_key(|pos| (pos.z, pos.x));
    }
    eprintln!(
        "Bedrock prepare: indexed {} saved regions in {:.1}s",
        discovered,
        scan.elapsed().as_secs_f64()
    );
    if !selected.is_empty() {
        eprintln!(
            "Bedrock prepare: bounded sample of {} saved regions",
            regions.len()
        );
    }
    let total = regions.len();
    let mut completed = 0;
    println!("MSC_PROGRESS {completed} {total}");
    let mut reused = 0;
    let mut findings = Findings::default();
    let current_regions = regions
        .keys()
        .map(|(dim, x, z)| format!("{}:{x}:{z}", dim_name(*dim)))
        .collect::<BTreeSet<_>>();
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let available_memory = system.available_memory();
    let available_disk = fs2::available_space(&output)?;
    let memory_workers = (available_memory / 2 / (768 * 1024 * 1024)).max(1) as usize;
    let disk_workers =
        (available_disk.saturating_sub(512 * 1024 * 1024) / (512 * 1024 * 1024)).max(1) as usize;
    let workers = if server_running {
        1
    } else {
        cores.min(memory_workers).min(disk_workers).clamp(1, 4)
    };
    eprintln!(
        "Bedrock prepare: {workers} region workers ({cores} logical cores, {available_memory} bytes available memory, {available_disk} bytes available disk, server running: {server_running})"
    );
    let tasks = regions
        .into_iter()
        .map(|((dimension, x, z), positions)| {
            let key = format!("{}:{x}:{z}", dim_name(dimension));
            RegionTask {
                dimension,
                x,
                z,
                positions,
                fingerprint: manifest.regions.get(&key).cloned(),
            }
        })
        .collect::<Vec<_>>();
    let mut tasks = tasks.into_iter();
    let mut read_time = Duration::ZERO;
    let mut nbt_time = Duration::ZERO;
    let mut encode_time = Duration::ZERO;
    let mut publish_time = Duration::ZERO;
    let mut manifest_time = Duration::ZERO;
    loop {
        let batch = tasks.by_ref().take(workers).collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        let count = batch.len();
        let (sender, receiver) = mpsc::channel();
        std::thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            for task in batch {
                let sender = sender.clone();
                let world = &world;
                let biome_names = &biome_names;
                let output = &output;
                scope.spawn(move || {
                    let result = prepare_region(
                        world,
                        output,
                        task,
                        exact_biome_registry.then_some(biome_names),
                        if workers == 1 && !server_running {
                            4
                        } else {
                            1
                        },
                    );
                    let _ = sender.send(result);
                });
            }
            drop(sender);
            for _ in 0..count {
                let work = receiver.recv()?.map_err(std::io::Error::other)?;
                let key = format!("{}:{}:{}", dim_name(work.dimension), work.x, work.z);
                let target = region_path(&output, work.dimension, work.x, work.z);
                read_time += work.read_time;
                nbt_time += work.nbt_time;
                encode_time += work.encode_time;
                merge_findings(&mut findings, work.findings);
                if let Some(bytes) = work.bytes {
                    const DISK_RESERVE_BYTES: u64 = 512 * 1024 * 1024;
                    let required_space = u64::try_from(bytes.len())?
                        .checked_add(DISK_RESERVE_BYTES)
                        .ok_or("Bedrock region size overflow")?;
                    if fs2::available_space(&output)? < required_space {
                        return Err(format!(
                            "insufficient disk space to publish region {key} while preserving a 512 MiB reserve"
                        ).into());
                    }
                    let published = Instant::now();
                    fs::create_dir_all(target.parent().ok_or("region has no parent")?)?;
                    let temp = target.with_extension(format!("mca.tmp-{}", std::process::id()));
                    fs::write(&temp, bytes)?;
                    fs::File::open(&temp)?.sync_all()?;
                    fs::rename(&temp, &target)?;
                    if let Some(parent) = target.parent() {
                        fs::File::open(parent)?.sync_all()?;
                    }
                    let region_file_sha256 = hex(&Sha256::digest(fs::read(&target)?));
                    publish_time += published.elapsed();
                    manifest.regions.insert(
                        key,
                        RegionFingerprint {
                            converted_content_sha256: work.digest,
                            region_file_sha256,
                        },
                    );
                    let saved = Instant::now();
                    write_manifest(&output, &manifest)?;
                    manifest_time += saved.elapsed();
                } else {
                    reused += 1;
                }
                completed += 1;
                println!("MSC_PROGRESS {completed} {total}");
                eprintln!(
                    "Bedrock prepare: {completed}/{total} regions converted or reused ({reused} reused)"
                );
            }
            Ok(())
        })?;
    }
    eprintln!(
        "Bedrock prepare: worker-time chunk read {:.1}s, NBT encode {:.1}s, region compression {:.1}s; wall-time publish/hash {:.1}s, manifest sync {:.1}s",
        read_time.as_secs_f64(),
        nbt_time.as_secs_f64(),
        encode_time.as_secs_f64(),
        publish_time.as_secs_f64(),
        manifest_time.as_secs_f64()
    );
    let stale = manifest
        .regions
        .keys()
        .filter(|key| selected.is_empty() && !current_regions.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    for key in stale {
        if let Some((dim, x, z)) = parse_region_key(&key) {
            let path = region_path(&output, dim, x, z);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        manifest.regions.remove(&key);
    }
    write_manifest(&output, &manifest)?;
    write_private_spawn_level(&source, &output, spawn)?;
    println!(
        "{}",
        serde_json::json!({"totalRegions":total,"reusedRegions":reused,"convertedRegions":completed-reused,"output":output,"registrySha256":registry_sha256})
    );
    eprintln!("Bedrock prepare: biome mapping mode: {registry_sha256}");
    fs::write(
        output.join("conversion-report.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "sourceVersion": version,
            "biomeMapping": registry_sha256,
            "blockNamesAndPositionCounts": findings.blocks,
            "renamedBlockNames": findings.renamed,
            "biomeIdsAndPositionCounts": findings.biomes,
            "unsupportedSections": findings.unsupported_sections,
            "sectionsWithSecondaryBlockLayers": findings.layered_sections,
            "sectionYRange": [findings.min_section, findings.max_section],
            "secondaryLayerPolicy": "The Anvil bridge writes the first visible Bedrock layer; sections with more than one layer are counted here.",
            "unmappedBiomes": findings.biomes.iter().filter_map(|(&id, &count)| {
                (!biome_names.contains_key(&id)).then_some(serde_json::json!({"id": id, "blockPositions": count}))
            }).collect::<Vec<_>>()
        }))?,
    )?;
    Ok(())
}

fn prepare_region(
    world: &BedrockWorld,
    output: &Path,
    task: RegionTask,
    biome_names: Option<&BTreeMap<u32, String>>,
    read_workers: usize,
) -> Result<RegionWork, String> {
    let mut opts = ChunkLoadOptions::for_data_request(
        ChunkDataRequest::new()
            .full_3d_indices()
            .biome(BiomeDataRequirement::All),
    );
    opts.threading = bedrock_world::WorldThreadingOptions::Fixed(read_workers);
    let mut findings = Findings::default();
    let mut read_time = Duration::ZERO;
    let mut nbt_time = Duration::ZERO;
    let mut region_chunks = Vec::with_capacity(task.positions.len());
    for batch in task.positions.chunks(32) {
        let started = Instant::now();
        let chunks = world
            .query_chunk_data_many_blocking(batch.iter().copied(), opts.clone())
            .map_err(|error| error.to_string())?;
        read_time += started.elapsed();
        for chunk in chunks {
            if !chunk.is_loaded {
                return Err(format!(
                    "saved chunk {},{} in {:?} has no supported render data; refusing to publish an incomplete region",
                    chunk.pos.x, chunk.pos.z, chunk.pos.dimension
                ));
            }
            let started = Instant::now();
            let nbt = encode_chunk(&chunk, &mut findings, biome_names, true)
                .map_err(|error| error.to_string())?;
            nbt_time += started.elapsed();
            region_chunks.push((chunk.pos.x, chunk.pos.z, nbt));
        }
    }
    region_chunks.sort_unstable_by_key(|(x, z, _)| (*z, *x));
    let mut content_hash = Sha256::new();
    for (x, z, nbt) in &region_chunks {
        content_hash.update(x.to_le_bytes());
        content_hash.update(z.to_le_bytes());
        content_hash.update(nbt);
    }
    let digest = hex(&content_hash.finalize());
    let target = region_path(output, task.dimension, task.x, task.z);
    let reused = task.fingerprint.as_ref().is_some_and(|fingerprint| {
        fingerprint.converted_content_sha256 == digest
            && fs::read(&target)
                .map(|bytes| hex(&Sha256::digest(bytes)) == fingerprint.region_file_sha256)
                .unwrap_or(false)
    });
    let started = Instant::now();
    let bytes = if reused {
        None
    } else {
        Some(encode_region(region_chunks).map_err(|error| error.to_string())?)
    };
    Ok(RegionWork {
        dimension: task.dimension,
        x: task.x,
        z: task.z,
        digest,
        bytes,
        findings,
        read_time,
        nbt_time,
        encode_time: started.elapsed(),
    })
}

fn merge_findings(total: &mut Findings, next: Findings) {
    for (name, count) in next.blocks {
        *total.blocks.entry(name).or_default() += count;
    }
    total.renamed.extend(next.renamed);
    for (id, count) in next.biomes {
        *total.biomes.entry(id).or_default() += count;
    }
    total.unsupported_sections += next.unsupported_sections;
    total.layered_sections += next.layered_sections;
    if let Some(y) = next.min_section {
        total.min_section = Some(total.min_section.map_or(y, |current| current.min(y)));
    }
    if let Some(y) = next.max_section {
        total.max_section = Some(total.max_section.map_or(y, |current| current.max(y)));
    }
}

use bedrock_world::NbtTag;
fn dim_name(dim: Dimension) -> &'static str {
    match dim {
        Dimension::Overworld => "overworld",
        Dimension::Nether => "nether",
        Dimension::End => "end",
        Dimension::Unknown(_) => "unknown",
    }
}
fn parse_region_key(key: &str) -> Option<(Dimension, i32, i32)> {
    let mut parts = key.split(':');
    let dimension = match parts.next()? {
        "overworld" => Dimension::Overworld,
        "nether" => Dimension::Nether,
        "end" => Dimension::End,
        _ => return None,
    };
    Some((
        dimension,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}
fn region_path(root: &Path, dim: Dimension, x: i32, z: i32) -> PathBuf {
    let sub = match dim {
        Dimension::Overworld => PathBuf::new(),
        Dimension::Nether => PathBuf::from("DIM-1"),
        Dimension::End => PathBuf::from("DIM1"),
        Dimension::Unknown(n) => PathBuf::from(format!("DIM{n}")),
    };
    root.join(sub).join("region").join(format!("r.{x}.{z}.mca"))
}
fn read_manifest(root: &Path) -> Result<Option<Manifest>, Box<dyn Error>> {
    let path = root.join("bedrock-conversion.json");
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}
fn write_manifest(root: &Path, value: &Manifest) -> Result<(), Box<dyn Error>> {
    let path = root.join("bedrock-conversion.json");
    let temp = root.join("bedrock-conversion.json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::File::open(&temp)?.sync_all()?;
    fs::rename(temp, path)?;
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
