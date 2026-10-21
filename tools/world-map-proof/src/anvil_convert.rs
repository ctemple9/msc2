//! Durable Bedrock-to-Anvil preparation for the shared Vantage renderer.
use crate::anvil_probe::{Findings, encode_chunk, encode_region};
use bedrock_world::{
    BedrockWorld, BiomeDataRequirement, ChunkDataRequest, ChunkLoadOptions, Dimension, OpenOptions,
    WorldScanOptions,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    converter: String,
    source_version: Vec<u64>,
    biome_mapping: String,
    regions: BTreeMap<String, RegionFingerprint>,
}

#[derive(Serialize, Deserialize)]
struct RegionFingerprint {
    converted_content_sha256: String,
    region_file_sha256: String,
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
    let world = BedrockWorld::open_blocking(&source, OpenOptions::default())?;
    let level = world.read_level_dat_blocking()?;
    let NbtTag::Compound(root) = level.root else {
        return Err("level.dat root is not a compound".into());
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
    let mut biome_names = BTreeMap::new();
    let exact_biome_registry = args.get(4).is_some();
    let registry_sha256 = if let Some(registry_path) = args.get(4) {
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
        format: 1,
        converter: "bedrock-anvil-v2".into(),
        source_version: version.clone(),
        biome_mapping: registry_sha256.clone(),
        regions: BTreeMap::new(),
    });
    if manifest.format != 1
        || manifest.converter != "bedrock-anvil-v2"
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
    eprintln!(
        "Bedrock prepare: indexed {} saved regions in {:.1}s",
        regions.len(),
        scan.elapsed().as_secs_f64()
    );
    let total = regions.len();
    let mut completed = 0;
    println!("MSC_PROGRESS {completed} {total}");
    let mut reused = 0;
    let mut findings = Findings::default();
    let mut current_regions = std::collections::BTreeSet::new();
    for ((dimension, rx, rz), positions) in regions {
        let key = format!("{}:{rx}:{rz}", dim_name(dimension));
        current_regions.insert(key.clone());
        let mut opts = ChunkLoadOptions::for_data_request(
            ChunkDataRequest::new()
                .full_3d_indices()
                .biome(BiomeDataRequirement::All),
        );
        opts.threading = bedrock_world::WorldThreadingOptions::Fixed(
            std::thread::available_parallelism()
                .map_or(1, usize::from)
                .clamp(1, 4),
        );
        let mut contents = Vec::new();
        let mut region_chunks = Vec::new();
        for batch in positions.chunks(32) {
            let chunks =
                world.query_chunk_data_many_blocking(batch.iter().copied(), opts.clone())?;
            for chunk in chunks {
                if !chunk.is_loaded {
                    return Err(format!("saved chunk {},{} in {:?} has no supported render data; refusing to publish an incomplete region", chunk.pos.x, chunk.pos.z, chunk.pos.dimension).into());
                }
                let nbt = encode_chunk(
                    &chunk,
                    &mut findings,
                    exact_biome_registry.then_some(&biome_names),
                    true,
                )?;
                contents.extend_from_slice(&chunk.pos.x.to_le_bytes());
                contents.extend_from_slice(&chunk.pos.z.to_le_bytes());
                contents.extend_from_slice(&nbt);
                region_chunks.push((chunk.pos.x, chunk.pos.z, nbt));
            }
        }
        let digest = hex(&Sha256::digest(&contents));
        let target = region_path(&output, dimension, rx, rz);
        let existing_region_hash = target
            .is_file()
            .then(|| fs::read(&target).map(|bytes| hex(&Sha256::digest(bytes))))
            .transpose()?;
        let reused_region = manifest.regions.get(&key).is_some_and(|fingerprint| {
            fingerprint.converted_content_sha256 == digest
                && existing_region_hash.as_ref() == Some(&fingerprint.region_file_sha256)
        });
        if reused_region {
            reused += 1;
        } else {
            let bytes = encode_region(region_chunks)?;
            const DISK_RESERVE_BYTES: u64 = 512 * 1024 * 1024;
            let required_space = u64::try_from(bytes.len())?
                .checked_add(DISK_RESERVE_BYTES)
                .ok_or("Bedrock region size overflow")?;
            if fs2::available_space(&output)? < required_space {
                return Err(format!(
                    "insufficient disk space to publish region {key} while preserving a 512 MiB reserve"
                ).into());
            }
            fs::create_dir_all(target.parent().ok_or("region has no parent")?)?;
            let temp = target.with_extension(format!("mca.tmp-{}", std::process::id()));
            fs::write(&temp, bytes)?;
            fs::File::open(&temp)?.sync_all()?;
            fs::rename(&temp, &target)?;
            if let Some(parent) = target.parent() {
                fs::File::open(parent)?.sync_all()?;
            }
            let region_file_sha256 = hex(&Sha256::digest(fs::read(&target)?));
            manifest.regions.insert(
                key,
                RegionFingerprint {
                    converted_content_sha256: digest,
                    region_file_sha256,
                },
            );
            write_manifest(&output, &manifest)?;
        }
        completed += 1;
        if completed % 10 == 0 || completed == total {
            println!("MSC_PROGRESS {completed} {total}");
            eprintln!(
                "Bedrock prepare: {completed}/{total} regions converted or reused ({reused} reused)"
            );
        }
    }
    let stale = manifest
        .regions
        .keys()
        .filter(|key| !current_regions.contains(*key))
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
    fs::copy(source.join("level.dat"), output.join("level.dat"))?;
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
