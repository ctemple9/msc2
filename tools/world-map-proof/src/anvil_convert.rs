//! Durable Bedrock-to-Anvil preparation for the shared Vantage renderer.
use crate::anvil_probe::{Findings, encode_chunk, encode_region};
use bedrock_world::{
    BedrockDbKey, BedrockWorld, BiomeDataRequirement, ChunkDataRequest, ChunkLoadOptions,
    Dimension, OpenOptions, StorageReadOptions, StorageVisitorControl,
};
use flate2::{Compression, write::GzEncoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
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
    #[serde(default)]
    source_sha256: Option<String>,
    converted_content_sha256: String,
    region_file_sha256: String,
}

struct RegionTask {
    dimension: Dimension,
    x: i32,
    z: i32,
    positions: Vec<bedrock_world::ChunkPos>,
    source_sha256: String,
    fingerprint: Option<RegionFingerprint>,
}

#[derive(Default)]
struct RegionIndex {
    positions: BTreeSet<bedrock_world::ChunkPos>,
    record_hashes: [u64; 4],
    record_count: u64,
}

impl RegionIndex {
    fn add(&mut self, key: &[u8], value: &[u8]) {
        let mut record = Sha256::new();
        record.update((key.len() as u64).to_le_bytes());
        record.update(key);
        record.update((value.len() as u64).to_le_bytes());
        record.update(value);
        for (lane, bytes) in self
            .record_hashes
            .iter_mut()
            .zip(record.finalize().chunks_exact(8))
        {
            *lane = lane.wrapping_add(u64::from_le_bytes(bytes.try_into().unwrap()));
        }
        self.record_count = self.record_count.saturating_add(1);
    }

    fn source_sha256(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(b"bedrock-render-records-v1");
        digest.update(self.record_count.to_le_bytes());
        for lane in self.record_hashes {
            digest.update(lane.to_le_bytes());
        }
        hex(&digest.finalize())
    }
}

struct RegionWork {
    dimension: Dimension,
    x: i32,
    z: i32,
    fingerprint: RegionFingerprint,
    reused: bool,
    findings: Findings,
    read_time: Duration,
    nbt_time: Duration,
    encode_time: Duration,
    publish_time: Duration,
}

const DISK_RESERVE_BYTES: u64 = 512 * 1024 * 1024;
const MANIFEST_CHECKPOINT_REGIONS: usize = 16;

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
    let mut index_only = false;
    for arg in args.iter().skip(4) {
        if let Some(value) = arg.strip_prefix("--region=") {
            if !selected.insert(selected_region(value)?) || selected.len() > 8 {
                return Err("choose one to eight distinct saved regions".into());
            }
        } else if arg == "--server-running" {
            server_running = true;
        } else if arg == "--index-only" {
            index_only = true;
        } else if arg.starts_with('-') || registry_path.replace(arg).is_some() {
            return Err("unexpected converter argument".into());
        }
    }
    if index_only && !selected.is_empty() {
        return Err("index-only mode cannot be limited to selected regions".into());
    }
    let mut biome_names = BTreeMap::new();
    let registry_bytes = if let Some(registry_path) = registry_path {
        Some(fs::read(registry_path)?)
    } else if version == [1, 26, 60, 0, 0] {
        // Keep this version-bound: numeric Bedrock IDs can change between saves.
        Some(include_bytes!("../data/bedrock-biome-ids-1.26.60.json").to_vec())
    } else {
        None
    };
    let exact_biome_registry = registry_bytes.is_some();
    let registry_sha256 = if let Some(registry_bytes) = registry_bytes {
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
        "neutral-unmapped-biomes-v2".to_owned()
    };
    let mut manifest = read_manifest(&output)?.unwrap_or(Manifest {
        format: 2,
        converter: "bedrock-anvil-v3".into(),
        source_version: version.clone(),
        biome_mapping: registry_sha256.clone(),
        regions: BTreeMap::new(),
    });
    if manifest.format != 2 || manifest.converter != "bedrock-anvil-v3" {
        return Err("prepared cache format differs; use a new output directory".into());
    }
    if index_only
        && (manifest.source_version != version || manifest.biome_mapping != registry_sha256)
    {
        return Err("cannot index a cache with a different save or biome format".into());
    }
    if manifest.source_version != version || manifest.biome_mapping != registry_sha256 {
        // This output is a private, unpublished generation. A global format or
        // biome change invalidates every region and must not leave seeded MCA
        // files for Vantage to mistake for current terrain.
        for name in ["region", "DIM-1", "DIM1"] {
            let directory = output.join(name);
            if directory.exists() {
                fs::remove_dir_all(directory)?;
            }
        }
        manifest = Manifest {
            format: 2,
            converter: "bedrock-anvil-v3".into(),
            source_version: version.clone(),
            biome_mapping: registry_sha256.clone(),
            regions: BTreeMap::new(),
        };
    }
    let scan = Instant::now();
    let mut indexed_regions = BTreeMap::<(Dimension, i32, i32), RegionIndex>::new();
    let mut scanned_records = 0usize;
    world
        .storage()
        .for_each_entry(StorageReadOptions::default(), &mut |key, value| {
            scanned_records = scanned_records.saturating_add(1);
            if let BedrockDbKey::Chunk(chunk_key) = BedrockDbKey::decode(key)
                && chunk_key.tag.is_render_chunk_record()
            {
                let pos = chunk_key.pos;
                if matches!(pos.dimension, Dimension::Unknown(_)) {
                    return Err(bedrock_world::BedrockWorldError::Validation(format!(
                        "unsupported saved dimension {:?}; refusing to omit its terrain",
                        pos.dimension
                    )));
                }
                let index = indexed_regions
                    .entry((pos.dimension, pos.x.div_euclid(32), pos.z.div_euclid(32)))
                    .or_default();
                index.positions.insert(pos);
                index.add(key, value);
            }
            if scanned_records.is_multiple_of(8192) {
                eprintln!("Bedrock prepare: indexed {scanned_records} saved database records");
            }
            Ok(StorageVisitorControl::Continue)
        })?;
    let discovered = indexed_regions.len();
    let mut regions = indexed_regions
        .into_iter()
        .map(|(key, index)| {
            let source_sha256 = index.source_sha256();
            let mut positions = index.positions.into_iter().collect::<Vec<_>>();
            positions.sort_unstable_by_key(|pos| (pos.z, pos.x));
            (key, (positions, source_sha256))
        })
        .collect::<BTreeMap<_, _>>();
    if !selected.is_empty() {
        regions.retain(|key, _| selected.contains(key));
        if regions.len() != selected.len() {
            return Err("one or more selected regions have no saved chunks".into());
        }
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
    if index_only {
        if manifest.regions.len() != regions.len() {
            return Err("the saved region set differs from the converted cache".into());
        }
        for ((dimension, x, z), (positions, source_sha256)) in regions {
            let key = format!("{}:{x}:{z}", dim_name(dimension));
            let fingerprint = manifest
                .regions
                .get_mut(&key)
                .ok_or("a saved region is missing from the converted cache")?;
            if positions.is_empty() {
                return Err("the saved region index unexpectedly contains no chunks".into());
            }
            let path = region_path(&output, dimension, x, z);
            if !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file()) {
                return Err("a converted region is missing from the active cache".into());
            }
            fingerprint.source_sha256 = Some(source_sha256);
            completed += 1;
            if completed % MANIFEST_CHECKPOINT_REGIONS == 0 || completed == total {
                write_manifest(&output, &manifest)?;
            }
            println!("MSC_PROGRESS {completed} {total}");
        }
        write_manifest(&output, &manifest)?;
        println!(
            "{}",
            serde_json::json!({"totalRegions":total,"reusedRegions":total,"convertedRegions":0,"indexedOnly":true,"output":output,"registrySha256":registry_sha256})
        );
        eprintln!(
            "Bedrock prepare: populated per-region input fingerprints without rewriting regions"
        );
        return Ok(());
    }
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
        (available_disk.saturating_sub(DISK_RESERVE_BYTES) / DISK_RESERVE_BYTES).max(1) as usize;
    let workers = if server_running {
        1
    } else {
        cores.min(memory_workers).min(disk_workers).clamp(1, 8)
    };
    eprintln!(
        "Bedrock prepare: {workers} region workers ({cores} logical cores, {available_memory} bytes available memory, {available_disk} bytes available disk, server running: {server_running})"
    );
    let chunk_read_workers = if workers == 1 && !server_running {
        cores.min(4)
    } else {
        1
    };
    let tasks = regions
        .into_iter()
        .map(|((dimension, x, z), (positions, source_sha256))| {
            let key = format!("{}:{x}:{z}", dim_name(dimension));
            RegionTask {
                dimension,
                x,
                z,
                positions,
                source_sha256,
                fingerprint: manifest.regions.get(&key).cloned(),
            }
        })
        .collect::<Vec<_>>();
    let work_queue = Mutex::new(VecDeque::from(tasks));
    let failed = AtomicBool::new(false);
    let reserved_disk_bytes = AtomicU64::new(0);
    let first_error = Mutex::new(None::<String>);
    let mut read_time = Duration::ZERO;
    let mut nbt_time = Duration::ZERO;
    let mut encode_time = Duration::ZERO;
    let mut publish_time = Duration::ZERO;
    let mut manifest_time = Duration::ZERO;
    let (sender, receiver) = mpsc::channel();
    let worker_count = workers.min(total);
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let sender = sender.clone();
            let work_queue = &work_queue;
            let failed = &failed;
            let reserved_disk_bytes = &reserved_disk_bytes;
            let world = &world;
            let biome_names = &biome_names;
            let output = &output;
            scope.spawn(move || {
                loop {
                    if failed.load(Ordering::Acquire) {
                        break;
                    }
                    let task = match work_queue.lock() {
                        Ok(mut queue) => queue.pop_front(),
                        Err(_) => {
                            failed.store(true, Ordering::Release);
                            let _ = sender.send(Err("Bedrock region queue was poisoned".into()));
                            break;
                        }
                    };
                    let Some(task) = task else {
                        break;
                    };
                    let result = prepare_region(
                        world,
                        output,
                        task,
                        exact_biome_registry.then_some(biome_names),
                        chunk_read_workers,
                        reserved_disk_bytes,
                    );
                    if result.is_err() {
                        failed.store(true, Ordering::Release);
                    }
                    if sender.send(result).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        for result in receiver {
            let work = match result {
                Ok(work) => work,
                Err(error) => {
                    failed.store(true, Ordering::Release);
                    if let Ok(mut first) = first_error.lock()
                        && first.is_none()
                    {
                        *first = Some(error);
                    }
                    continue;
                }
            };
            let key = format!("{}:{}:{}", dim_name(work.dimension), work.x, work.z);
            read_time += work.read_time;
            nbt_time += work.nbt_time;
            encode_time += work.encode_time;
            publish_time += work.publish_time;
            merge_findings(&mut findings, work.findings);
            if work.reused {
                reused += 1;
            }
            manifest.regions.insert(key, work.fingerprint);
            completed += 1;
            println!("MSC_PROGRESS {completed} {total}");
            eprintln!(
                "Bedrock prepare: {completed}/{total} regions converted or reused ({reused} reused)"
            );
            if completed % MANIFEST_CHECKPOINT_REGIONS == 0 {
                let saved = Instant::now();
                if let Err(error) = write_manifest(&output, &manifest) {
                    failed.store(true, Ordering::Release);
                    if let Ok(mut first) = first_error.lock()
                        && first.is_none()
                    {
                        *first = Some(error.to_string());
                    }
                }
                manifest_time += saved.elapsed();
            }
        }
    });
    if let Some(error) = first_error
        .into_inner()
        .map_err(|_| std::io::Error::other("Bedrock conversion error state was poisoned"))?
    {
        return Err(error.into());
    }
    if failed.load(Ordering::Acquire) {
        return Err("Bedrock region conversion stopped before completion".into());
    }
    let saved = Instant::now();
    write_manifest(&output, &manifest)?;
    manifest_time += saved.elapsed();
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
    reserved_disk_bytes: &AtomicU64,
) -> Result<RegionWork, String> {
    let target = region_path(output, task.dimension, task.x, task.z);
    if let Some(mut fingerprint) = task.fingerprint.clone()
        && fingerprint.source_sha256.as_deref() == Some(task.source_sha256.as_str())
        && fs::read(&target)
            .map(|bytes| hex(&Sha256::digest(bytes)) == fingerprint.region_file_sha256)
            .unwrap_or(false)
    {
        fingerprint.source_sha256 = Some(task.source_sha256.clone());
        return Ok(RegionWork {
            dimension: task.dimension,
            x: task.x,
            z: task.z,
            fingerprint,
            reused: true,
            findings: Findings::default(),
            read_time: Duration::ZERO,
            nbt_time: Duration::ZERO,
            encode_time: Duration::ZERO,
            publish_time: Duration::ZERO,
        });
    }

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
    let cached = task.fingerprint.filter(|fingerprint| {
        fingerprint.converted_content_sha256 == digest
            && fs::read(&target)
                .map(|bytes| hex(&Sha256::digest(bytes)) == fingerprint.region_file_sha256)
                .unwrap_or(false)
    });
    let (fingerprint, reused, encode_time, publish_time) = if let Some(mut fingerprint) = cached {
        fingerprint.source_sha256 = Some(task.source_sha256.clone());
        (fingerprint, true, Duration::ZERO, Duration::ZERO)
    } else {
        let started = Instant::now();
        let bytes = encode_region(region_chunks).map_err(|error| error.to_string())?;
        let encode_time = started.elapsed();
        let started = Instant::now();
        let region_file_sha256 = publish_region(output, &target, &bytes, reserved_disk_bytes)?;
        let publish_time = started.elapsed();
        (
            RegionFingerprint {
                source_sha256: Some(task.source_sha256.clone()),
                converted_content_sha256: digest,
                region_file_sha256,
            },
            false,
            encode_time,
            publish_time,
        )
    };
    Ok(RegionWork {
        dimension: task.dimension,
        x: task.x,
        z: task.z,
        fingerprint,
        reused,
        findings,
        read_time,
        nbt_time,
        encode_time,
        publish_time,
    })
}

fn publish_region(
    output: &Path,
    target: &Path,
    bytes: &[u8],
    reserved_disk_bytes: &AtomicU64,
) -> Result<String, String> {
    let length = u64::try_from(bytes.len()).map_err(|_| "Bedrock region size overflow")?;
    let required = length
        .checked_add(DISK_RESERVE_BYTES)
        .ok_or("Bedrock region size overflow")?;
    let reservation = loop {
        let outstanding = reserved_disk_bytes.load(Ordering::Acquire);
        let available = fs2::available_space(output).map_err(|error| error.to_string())?;
        if available.saturating_sub(outstanding) < required {
            return Err(format!(
                "insufficient disk space to publish {} while preserving a 512 MiB reserve",
                target.display()
            ));
        }
        let next = outstanding
            .checked_add(length)
            .ok_or("Bedrock region reservation overflow")?;
        if reserved_disk_bytes
            .compare_exchange(outstanding, next, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break length;
        }
    };
    let result = (|| {
        let region_file_sha256 = hex(&Sha256::digest(bytes));
        fs::create_dir_all(
            target
                .parent()
                .ok_or("Bedrock region has no parent directory")?,
        )
        .map_err(|error| error.to_string())?;
        let name = target
            .file_name()
            .ok_or("Bedrock region filename is missing")?
            .to_string_lossy();
        let temporary = target.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
        let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary, target).map_err(|error| error.to_string())?;
        if let Some(parent) = target.parent() {
            fs::File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|error| error.to_string())?;
        }
        Ok(region_file_sha256)
    })();
    reserved_disk_bytes.fetch_sub(reservation, Ordering::AcqRel);
    result
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
