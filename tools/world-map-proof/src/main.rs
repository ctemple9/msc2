use bedrock_world::{
    BedrockWorld, ChunkPos, Dimension, NbtTag, OpenOptions, SubChunkFormat, WorldScanOptions,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

mod render;

fn validated_biome_registry(world: &BedrockWorld, path: &Path) -> Result<String, Box<dyn Error>> {
    let document = world.read_level_dat_blocking()?;
    let NbtTag::Compound(root) = document.root else {
        return Err("level.dat root is not a compound".into());
    };
    let Some(NbtTag::List(version_tags)) = root.get("lastOpenedWithVersion") else {
        return Err("save has no lastOpenedWithVersion; cannot match biome registry".into());
    };
    let save_version: Vec<u64> = version_tags
        .iter()
        .map(|tag| match tag {
            NbtTag::Int(value) if *value >= 0 => Some(*value as u64),
            _ => None,
        })
        .collect::<Option<_>>()
        .ok_or("save version contains a non-integer component")?;
    let registry: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let registry_version: Vec<u64> = registry["version"]
        .as_array()
        .ok_or("registry version must be an integer array")?
        .iter()
        .map(serde_json::Value::as_u64)
        .collect::<Option<_>>()
        .ok_or("registry version contains a non-integer component")?;
    if registry_version != save_version {
        return Err(format!(
            "biome registry version {registry_version:?} does not match save version {save_version:?}"
        )
        .into());
    }
    let source = registry["source"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or("registry source is missing")?;
    let source_hash = registry["source_sha256"]
        .as_str()
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("registry source_sha256 must be a 64-character hex digest")?;
    let ids = registry["ids"]
        .as_object()
        .ok_or("registry ids must map biome names to integers")?;
    let mut seen = BTreeSet::new();
    for (name, id) in ids {
        let id = id.as_u64().ok_or("registry has a non-integer biome ID")?;
        if name.is_empty() || !seen.insert(id) {
            return Err("registry has an empty name or duplicate biome ID".into());
        }
    }
    for (name, expected_id) in [
        ("birch_forest", 27),
        ("birch_forest_mutated", 155),
        ("frozen_peaks", 183),
        ("grove", 185),
        ("stony_peaks", 189),
    ] {
        if ids.get(name).and_then(serde_json::Value::as_u64) != Some(expected_id) {
            return Err(format!("registry conflicts with current tint assumption: {name}").into());
        }
    }
    Ok(format!(
        "save metadata version {save_version:?} matched registry; {} unique IDs checked against current tint assumptions; source {source}; source SHA-256 {source_hash}",
        seen.len(),
    ))
}

fn family(name: &str) -> &'static str {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    if name.contains("stair") {
        "stairs"
    } else if name.contains("glass") {
        "glass"
    } else if name.contains("water") {
        "water"
    } else if name.contains("leaves")
        || name.contains("grass")
        || name.contains("flower")
        || name.contains("sapling")
    {
        "foliage"
    } else {
        "other"
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).ok_or("usage: msc-world-map-proof <offline world copy> <Bedrock resource pack> <private output directory> [chunk-x,chunk-z] [biome-registry.json]")?;
    let world = BedrockWorld::open_blocking(Path::new(&path), OpenOptions::default())?;
    let positions: BTreeSet<_> = world
        .list_render_chunk_positions_blocking(WorldScanOptions::default())?
        .into_iter()
        .filter(|p| p.dimension == Dimension::Overworld)
        .map(|p| (p.x, p.z))
        .collect();
    let spawn = world
        .read_level_dat_blocking()
        .ok()
        .and_then(|document| {
            let NbtTag::Compound(root) = document.root else {
                return None;
            };
            let (Some(NbtTag::Int(x)), Some(NbtTag::Int(z))) =
                (root.get("SpawnX"), root.get("SpawnZ"))
            else {
                return None;
            };
            Some((x.div_euclid(16), z.div_euclid(16)))
        })
        .unwrap_or((0, 0));
    let anchor = if let Some(value) = args.get(4) {
        let (x, z) = value
            .split_once(',')
            .ok_or("anchor must be chunk-x,chunk-z")?;
        (x.parse::<i32>()?, z.parse::<i32>()?)
    } else {
        positions
            .iter()
            .copied()
            .max_by_key(|&(x, z)| {
                let present = (0..4)
                    .flat_map(|dx| (0..4).map(move |dz| (dx, dz)))
                    .filter(|&(dx, dz)| positions.contains(&(x + dx, z + dz)))
                    .count();
                let distance = (x + 2 - spawn.0).abs() + (z + 2 - spawn.1).abs();
                (present, -distance)
            })
            .ok_or("no Overworld chunks")?
    };
    let mut states = BTreeMap::<String, usize>::new();
    let mut families = BTreeMap::<&str, usize>::new();
    let mut missing = Vec::new();
    let mut chunks = 0;
    for dx in 0..4 {
        for dz in 0..4 {
            let (x, z) = (anchor.0 + dx, anchor.1 + dz);
            if !positions.contains(&(x, z)) {
                missing.push((x, z));
                continue;
            }
            chunks += 1;
            let chunk = world.get_chunk_blocking(ChunkPos {
                x,
                z,
                dimension: Dimension::Overworld,
            })?;
            for y in -4i8..=19i8 {
                let Some(subchunk) = chunk.get_subchunk(y)? else {
                    continue;
                };
                let SubChunkFormat::Paletted { storages, .. } = subchunk.format else {
                    continue;
                };
                for storage in storages {
                    let mut counts = vec![0usize; storage.states.len()];
                    for local_y in 0..16 {
                        for local_z in 0..16 {
                            for local_x in 0..16 {
                                if let Some(index) =
                                    storage.palette_index_at(local_x, local_y, local_z)
                                    && let Some(count) = counts.get_mut(usize::from(index))
                                {
                                    *count += 1;
                                }
                            }
                        }
                    }
                    for (index, state) in storage.states.iter().enumerate() {
                        let count = counts[index];
                        if count == 0 || state.name.ends_with(":air") {
                            continue;
                        }
                        *states.entry(state.name.clone()).or_default() += count;
                        *families.entry(family(&state.name)).or_default() += count;
                    }
                }
            }
        }
    }
    println!(
        "4x4 origin: {}, {} | present chunks: {} | missing chunks: {:?}",
        anchor.0, anchor.1, chunks, missing
    );
    println!("block families: {families:?}");
    println!("distinct block states: {}", states.len());
    for (name, count) in states.iter().filter(|(name, _)| family(name) != "other") {
        println!("{name}: {count}");
    }
    let pack = args.get(2).ok_or("missing Bedrock resource pack path")?;
    let output = args.get(3).ok_or("missing private output directory")?;
    let registry_status = args.get(5).map_or_else(
        || Ok("provisional; no exact-version registry supplied".to_owned()),
        |path| validated_biome_registry(&world, Path::new(path)),
    )?;
    render::render(
        &world,
        anchor,
        Path::new(pack),
        Path::new(output),
        &registry_status,
    )?;
    Ok(())
}
