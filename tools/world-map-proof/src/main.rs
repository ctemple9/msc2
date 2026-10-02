use bedrock_world::{
    BedrockWorld, ChunkKey, ChunkPos, ChunkRecordTag, Dimension, NbtTag, OpenOptions,
    SubChunkFormat, WorldScanOptions,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

mod render;

fn map_dimension(id: &str) -> Result<Dimension, Box<dyn Error>> {
    match id {
        "minecraft:overworld" => Ok(Dimension::Overworld),
        "minecraft:the_nether" => Ok(Dimension::Nether),
        "minecraft:the_end" => Ok(Dimension::End),
        _ => Err("unsupported Bedrock dimension".into()),
    }
}

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

fn chunk_terrain_changed(
    before: &BedrockWorld,
    after: &BedrockWorld,
    pos: ChunkPos,
) -> Result<bool, Box<dyn Error>> {
    let tags = [
        ChunkRecordTag::Data3D,
        ChunkRecordTag::Data2D,
        ChunkRecordTag::Data2DLegacy,
        ChunkRecordTag::LegacyTerrain,
        ChunkRecordTag::BlockExtraData,
        ChunkRecordTag::BiomeState,
        ChunkRecordTag::Version,
        ChunkRecordTag::VersionOld,
        ChunkRecordTag::LegacyVersion,
    ];
    for tag in tags {
        let key = ChunkKey::new(pos, tag).encode_inline();
        if before.storage().get(key.as_ref())? != after.storage().get(key.as_ref())? {
            return Ok(true);
        }
    }
    for y in -4i8..=19 {
        let key = ChunkKey::subchunk(pos, y).encode_inline();
        if before.storage().get(key.as_ref())? != after.storage().get(key.as_ref())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn compare_tiles(args: &[String]) -> Result<(), Box<dyn Error>> {
    let before_path = args.get(2).ok_or("compare requires previous world path")?;
    let after_path = args.get(3).ok_or("compare requires current world path")?;
    let origin = args
        .get(4)
        .ok_or("compare requires first tile chunk-x,chunk-z")?;
    let (x, z) = origin.split_once(',').ok_or("tile origin must be x,z")?;
    let (x, z) = (x.parse::<i32>()?, z.parse::<i32>()?);
    let width = args
        .get(5)
        .ok_or("compare requires tile width")?
        .parse::<u32>()?;
    let depth = args
        .get(6)
        .ok_or("compare requires tile depth")?
        .parse::<u32>()?;
    if width == 0 || depth == 0 || width > 8 || depth > 8 {
        return Err("tile grid must be 1..8 by 1..8".into());
    }
    let before = BedrockWorld::open_blocking(Path::new(before_path), OpenOptions::default())?;
    let after = BedrockWorld::open_blocking(Path::new(after_path), OpenOptions::default())?;
    let mut changed = Vec::new();
    let mut checked_chunks = 0usize;
    for tile_x in 0..width {
        for tile_z in 0..depth {
            let start_x = x
                .checked_add(i32::try_from(tile_x)? * 4)
                .ok_or("tile x overflow")?;
            let start_z = z
                .checked_add(i32::try_from(tile_z)? * 4)
                .ok_or("tile z overflow")?;
            let mut dirty = false;
            for dx in 0..4 {
                for dz in 0..4 {
                    let pos = ChunkPos {
                        x: start_x.checked_add(dx).ok_or("chunk x overflow")?,
                        z: start_z.checked_add(dz).ok_or("chunk z overflow")?,
                        dimension: Dimension::Overworld,
                    };
                    checked_chunks += 1;
                    dirty |= chunk_terrain_changed(&before, &after, pos)?;
                }
            }
            if dirty {
                changed.push([start_x, start_z]);
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({ "checkedChunks": checked_chunks, "changedTiles": changed })
    );
    Ok(())
}

fn catalog_paths(path: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let tiles = manifest["tiles"].as_array().ok_or("catalog has no tiles")?;
    tiles
        .iter()
        .map(|tile| {
            let path = tile["path"].as_str().ok_or("catalog tile has no path")?;
            tile_origin(path)?;
            Ok(path.to_owned())
        })
        .collect()
}

fn tile_origin(path: &str) -> Result<(i32, i32), Box<dyn Error>> {
    let name = path
        .strip_prefix("tiles/t.")
        .and_then(|name| name.strip_suffix(".vtile"))
        .ok_or("invalid catalog tile path")?;
    let (x, z) = name
        .split_once('.')
        .ok_or("invalid catalog tile coordinates")?;
    let x = x.parse::<i32>()?.checked_mul(4).ok_or("tile x overflow")?;
    let z = z.parse::<i32>()?.checked_mul(4).ok_or("tile z overflow")?;
    Ok((x, z))
}

fn diff_catalogs(args: &[String]) -> Result<(), Box<dyn Error>> {
    let before = BedrockWorld::open_blocking(
        Path::new(args.get(2).ok_or("diff requires previous world")?),
        OpenOptions::default(),
    )?;
    let after = BedrockWorld::open_blocking(
        Path::new(args.get(3).ok_or("diff requires current world")?),
        OpenOptions::default(),
    )?;
    let dimension = map_dimension(args.get(4).ok_or("diff requires dimension")?)?;
    let old_manifest = Path::new(args.get(5).ok_or("diff requires previous catalog")?);
    let old = catalog_paths(old_manifest)?;
    let new = catalog_paths(Path::new(
        args.get(6).ok_or("diff requires current catalog")?,
    ))?;
    let mut unchanged = Vec::new();
    let mut changed = 0usize;
    for path in &new {
        if !old.contains(path) {
            changed += 1;
            continue;
        }
        if !old_manifest
            .parent()
            .ok_or("previous catalog has no directory")?
            .join(path)
            .is_file()
        {
            continue;
        }
        let (x, z) = tile_origin(path)?;
        let mut dirty = false;
        for dx in 0..4 {
            for dz in 0..4 {
                dirty |= chunk_terrain_changed(
                    &before,
                    &after,
                    ChunkPos {
                        x: x.checked_add(dx).ok_or("chunk x overflow")?,
                        z: z.checked_add(dz).ok_or("chunk z overflow")?,
                        dimension,
                    },
                )?;
            }
        }
        if dirty {
            changed += 1;
        } else {
            unchanged.push(path);
        }
    }
    fs::write(
        Path::new(args.get(7).ok_or("diff requires output path")?),
        serde_json::to_vec(&serde_json::json!({
            "unchangedTiles": unchanged,
            "changedTiles": changed,
            "removedTiles": old.difference(&new).count(),
        }))?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--version") {
        println!("msc-bedrock-map {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.get(1).is_some_and(|arg| arg == "compare") {
        return compare_tiles(&args);
    }
    if args.get(1).is_some_and(|arg| arg == "diff-catalogs") {
        return diff_catalogs(&args);
    }
    if args
        .get(1)
        .is_some_and(|arg| arg == "catalog" || arg == "tile")
    {
        let path = Path::new(args.get(2).ok_or("saved world path required")?);
        let pack = Path::new(args.get(3).ok_or("resource pack path required")?);
        let output = Path::new(args.get(4).ok_or("output directory required")?);
        let dimension_id = args
            .get(5)
            .map(String::as_str)
            .unwrap_or("minecraft:overworld");
        let dimension = map_dimension(dimension_id)?;
        let world = BedrockWorld::open_blocking(path, OpenOptions::default())?;
        if args[1] == "tile" {
            let x = args.get(6).ok_or("tile chunk x required")?.parse::<i32>()?;
            let z = args.get(7).ok_or("tile chunk z required")?.parse::<i32>()?;
            if x.rem_euclid(4) != 0 || z.rem_euclid(4) != 0 {
                return Err("tile origin must align to four chunks".into());
            }
            return render::render_catalog_tile(&world, (x, z), pack, output, dimension);
        }
        let positions: Vec<_> = world
            .list_render_chunk_positions_blocking(WorldScanOptions::default())?
            .into_iter()
            .filter(|p| p.dimension == dimension)
            .collect();
        let anchors: BTreeSet<_> = positions
            .iter()
            .map(|p| (p.x.div_euclid(4) * 4, p.z.div_euclid(4) * 4))
            .collect();
        let spawn = (dimension == Dimension::Overworld)
            .then(|| world.read_level_dat_blocking().ok())
            .flatten()
            .and_then(|document| {
                let NbtTag::Compound(root) = document.root else {
                    return None;
                };
                let (Some(NbtTag::Int(x)), Some(NbtTag::Int(y)), Some(NbtTag::Int(z))) =
                    (root.get("SpawnX"), root.get("SpawnY"), root.get("SpawnZ"))
                else {
                    return None;
                };
                (-64..320).contains(y).then_some((*x, *y, *z))
            });
        return render::create_catalog(
            &world,
            &positions,
            &anchors.into_iter().collect::<Vec<_>>(),
            pack,
            output,
            spawn,
            dimension_id,
        );
    }
    if args.get(1).is_some_and(|arg| arg == "grid") {
        let path = Path::new(args.get(2).ok_or("grid requires saved world path")?);
        let pack = Path::new(args.get(3).ok_or("grid requires resource pack path")?);
        let output = Path::new(args.get(4).ok_or("grid requires output directory")?);
        let world = BedrockWorld::open_blocking(path, OpenOptions::default())?;
        let positions: BTreeSet<_> = world
            .list_render_chunk_positions_blocking(WorldScanOptions::default())?
            .into_iter()
            .filter(|p| p.dimension == Dimension::Overworld)
            .map(|p| (p.x, p.z))
            .collect();
        let spawn = world.read_level_dat_blocking().ok().and_then(|document| {
            let NbtTag::Compound(root) = document.root else {
                return None;
            };
            let (Some(NbtTag::Int(x)), Some(NbtTag::Int(y)), Some(NbtTag::Int(z))) =
                (root.get("SpawnX"), root.get("SpawnY"), root.get("SpawnZ"))
            else {
                return None;
            };
            Some((*x, *y, *z))
        });
        let center = spawn
            .map(|(x, _, z)| (x.div_euclid(64), z.div_euclid(64)))
            .filter(|&(x, z)| {
                (0..4).any(|dx| (0..4).any(|dz| positions.contains(&(x * 4 + dx, z * 4 + dz))))
            })
            .or_else(|| {
                positions
                    .iter()
                    .next()
                    .map(|&(x, z)| (x.div_euclid(4), z.div_euclid(4)))
            })
            .ok_or("no saved Overworld chunks")?;
        let mut anchors = Vec::new();
        for tx in center.0 - 1..=center.0 + 1 {
            for tz in center.1 - 1..=center.1 + 1 {
                let anchor = (tx * 4, tz * 4);
                if (0..4)
                    .any(|dx| (0..4).any(|dz| positions.contains(&(anchor.0 + dx, anchor.1 + dz))))
                {
                    anchors.push(anchor);
                }
            }
        }
        return render::render_grid(&world, &anchors, pack, output, spawn);
    }
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
    let world_spawn = world.read_level_dat_blocking().ok().and_then(|document| {
        let NbtTag::Compound(root) = document.root else {
            return None;
        };
        let (Some(NbtTag::Int(x)), Some(NbtTag::Int(y)), Some(NbtTag::Int(z))) =
            (root.get("SpawnX"), root.get("SpawnY"), root.get("SpawnZ"))
        else {
            return None;
        };
        Some(serde_json::json!({ "x": x, "y": y, "z": z }))
    });
    fs::write(
        Path::new(output).join("viewer-world.json"),
        serde_json::to_vec(&serde_json::json!({
            "chunkOrigin": [anchor.0, anchor.1],
            "spawn": world_spawn,
        }))?,
    )?;
    Ok(())
}
