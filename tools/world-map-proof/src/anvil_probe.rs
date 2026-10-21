//! Bounded, private Anvil input for measuring the shared Vantage boundary.
//! This is deliberately a fidelity probe, not a production world converter.

use bedrock_world::{
    BedrockWorld, BiomeDataRequirement, BlockPalette, ChunkData, ChunkDataRequest,
    ChunkLoadOptions, Dimension, OpenOptions, SubChunkFormat, WorldChunkQueryRegion,
    WorldScanOptions,
};
use fastnbt::LongArray;
use flate2::{Compression, write::ZlibEncoder};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Default)]
pub(crate) struct Findings {
    pub(crate) blocks: BTreeMap<String, u64>,
    pub(crate) renamed: BTreeMap<String, String>,
    pub(crate) biomes: BTreeMap<u32, u64>,
    pub(crate) unsupported_sections: u64,
    pub(crate) layered_sections: u64,
    pub(crate) min_section: Option<i8>,
    pub(crate) max_section: Option<i8>,
}

type RegionChunks = BTreeMap<(i32, i32), Vec<(i32, i32, Vec<u8>)>>;

#[derive(Serialize)]
struct AnvilBlock {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Properties", skip_serializing_if = "BTreeMap::is_empty")]
    properties: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct AnvilBlockStates {
    palette: Vec<AnvilBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<LongArray>,
}

#[derive(Serialize)]
struct AnvilBiomes {
    palette: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<LongArray>,
}

#[derive(Serialize)]
struct AnvilSection {
    #[serde(rename = "Y")]
    y: i8,
    block_states: AnvilBlockStates,
    biomes: AnvilBiomes,
}

#[derive(Serialize)]
struct AnvilChunk {
    #[serde(rename = "DataVersion")]
    data_version: i32,
    #[serde(rename = "xPos")]
    x: i32,
    #[serde(rename = "zPos")]
    z: i32,
    sections: Vec<AnvilSection>,
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = Path::new(
        args.get(2)
            .ok_or("anvil-probe requires a private Bedrock snapshot")?,
    );
    let output = Path::new(
        args.get(3)
            .ok_or("anvil-probe requires a new private output directory")?,
    );
    let x = args
        .get(4)
        .ok_or("anvil-probe requires chunk x")?
        .parse::<i32>()?;
    let z = args
        .get(5)
        .ok_or("anvil-probe requires chunk z")?
        .parse::<i32>()?;
    let dimension = match args.get(6).map(String::as_str).unwrap_or("overworld") {
        "overworld" => Dimension::Overworld,
        "nether" => Dimension::Nether,
        "end" => Dimension::End,
        _ => return Err("dimension must be overworld, nether, or end".into()),
    };
    if !source.join("db").is_dir() || !source.join("level.dat").is_file() {
        return Err("source must be an existing offline Bedrock world copy".into());
    }
    if output.exists() {
        return Err(
            "probe output must be a new directory; existing data will not be replaced".into(),
        );
    }
    let source = source.canonicalize()?;
    let output_parent = output
        .parent()
        .ok_or("output needs a parent directory")?
        .canonicalize()?;
    if source.starts_with(&output_parent) || output_parent.starts_with(&source) {
        return Err("probe output must be separate from the Bedrock source".into());
    }
    let started = Instant::now();
    let world = BedrockWorld::open_blocking(&source, OpenOptions::default())?;
    let open_seconds = started.elapsed().as_secs_f64();
    let origin_x = x.div_euclid(4) * 4;
    let origin_z = z.div_euclid(4) * 4;
    let scan_started = Instant::now();
    let positions = world.list_chunk_positions_in_region_blocking(
        WorldChunkQueryRegion {
            dimension,
            min_chunk_x: origin_x,
            min_chunk_z: origin_z,
            max_chunk_x: origin_x + 3,
            max_chunk_z: origin_z + 3,
        },
        WorldScanOptions::default(),
    )?;
    let scan_seconds = scan_started.elapsed().as_secs_f64();
    let read_started = Instant::now();
    let chunks = world.query_chunk_data_many_blocking(
        positions,
        ChunkLoadOptions::for_data_request(
            ChunkDataRequest::new()
                .full_3d_indices()
                .biome(BiomeDataRequirement::All),
        ),
    )?;
    let read_seconds = read_started.elapsed().as_secs_f64();
    let mut findings = Findings::default();
    let mut regions = RegionChunks::new();
    let mut loaded = 0usize;
    for chunk in &chunks {
        if !chunk.is_loaded {
            continue;
        }
        loaded += 1;
        let nbt = encode_chunk(chunk, &mut findings, None, false)?;
        regions
            .entry((chunk.pos.x.div_euclid(32), chunk.pos.z.div_euclid(32)))
            .or_default()
            .push((chunk.pos.x, chunk.pos.z, nbt));
    }
    if loaded == 0 {
        return Err("the bounded 4x4 sample contains no saved chunks".into());
    }
    let encode_seconds =
        started.elapsed().as_secs_f64() - open_seconds - scan_seconds - read_seconds;
    fs::create_dir(output)?;
    let region_dir = output.join("region");
    fs::create_dir(&region_dir)?;
    for ((rx, rz), chunks) in regions {
        let bytes = encode_region(chunks)?;
        fs::write(region_dir.join(format!("r.{rx}.{rz}.mca")), bytes)?;
    }
    let report = serde_json::json!({
        "source": source,
        "sample_origin_chunk": [x.div_euclid(4) * 4, z.div_euclid(4) * 4],
        "dimension": format!("{dimension:?}"),
        "saved_chunks": loaded,
        "requested_chunks": 16,
        "open_seconds": open_seconds,
        "scan_seconds": scan_seconds,
        "read_seconds": read_seconds,
        "encode_seconds": encode_seconds,
        "total_seconds": started.elapsed().as_secs_f64(),
        "block_names": findings.blocks,
        "renamed_blocks": findings.renamed,
        "biome_ids": findings.biomes,
        "unsupported_sections": findings.unsupported_sections,
        "layered_sections": findings.layered_sections,
        "min_section_y": findings.min_section,
        "max_section_y": findings.max_section,
        "fidelity_limit": "The bounded probe assigns plains because it is only a renderer-boundary proof. The durable converter requires an exact-version biome registry and retains block state properties; it selects the visible Bedrock layer and reports layered sections.",
    });
    fs::write(
        output.join("probe.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn mapped_block_name(name: &str) -> &str {
    match name {
        "minecraft:grass" => "minecraft:grass_block",
        "minecraft:stonebrick" => "minecraft:stone_bricks",
        "minecraft:flowing_water" => "minecraft:water",
        "minecraft:flowing_lava" => "minecraft:lava",
        _ => name,
    }
}

fn is_palette_air(name: &str) -> bool {
    // bedrock-world keeps this predicate private; mirror its visible-layer rule here so the
    // optimized path can select layers from numeric palettes instead of resolving each voxel.
    matches!(
        name,
        "air"
            | "cave_air"
            | "void_air"
            | "minecraft:air"
            | "minecraft:cave_air"
            | "minecraft:void_air"
            | "minecraft:structure_void"
            | "minecraft:light_block"
            | "minecraft:light"
    )
}

fn state_signature(name: &str, properties: &BTreeMap<String, String>) -> String {
    let mut signature = name.to_owned();
    for (key, value) in properties {
        signature.push('\0');
        signature.push_str(key);
        signature.push('=');
        signature.push_str(value);
    }
    signature
}

fn intern_state(
    name: String,
    properties: BTreeMap<String, String>,
    states: &mut Vec<(String, BTreeMap<String, String>)>,
    lookup: &mut HashMap<String, usize>,
) -> usize {
    let signature = state_signature(&name, &properties);
    if let Some(&index) = lookup.get(&signature) {
        return index;
    }
    let index = states.len();
    states.push((name, properties));
    lookup.insert(signature, index);
    index
}

fn encode_block_states(storages: &[BlockPalette], findings: &mut Findings) -> AnvilBlockStates {
    let mut canonical_states = Vec::<(String, BTreeMap<String, String>)>::new();
    let mut canonical_lookup = HashMap::<String, usize>::new();
    let air_state = intern_state(
        "minecraft:air".to_owned(),
        BTreeMap::new(),
        &mut canonical_states,
        &mut canonical_lookup,
    );
    let mut source_state_names = Vec::<String>::new();
    let mut source_state_is_air = Vec::<bool>::new();
    let mut source_state_to_canonical = Vec::<Vec<usize>>::with_capacity(storages.len());
    let mut source_state_offsets = Vec::<usize>::with_capacity(storages.len());

    for storage in storages {
        source_state_offsets.push(source_state_names.len());
        let mut palette_map = Vec::with_capacity(storage.states.len());
        for state in &storage.states {
            source_state_names.push(state.name.clone());
            source_state_is_air.push(is_palette_air(&state.name));
            let properties = state
                .states
                .iter()
                .map(|(key, value)| (key.clone(), property_string(key, value)))
                .collect::<BTreeMap<_, _>>();
            palette_map.push(intern_state(
                mapped_block_name(&state.name).to_owned(),
                properties,
                &mut canonical_states,
                &mut canonical_lookup,
            ));
        }
        source_state_to_canonical.push(palette_map);
    }

    let mut output_palette_ids = Vec::<usize>::new();
    let mut output_index_by_state = vec![None; canonical_states.len()];
    let mut source_state_counts = vec![0u64; source_state_names.len()];
    let mut fallback_air_count = 0u64;
    let mut indices = Vec::with_capacity(4096);
    for y in 0u8..16 {
        for z in 0u8..16 {
            for x in 0u8..16 {
                // Bedrock stores Y as the innermost axis; Anvil's packed array is emitted
                // with X innermost below, so the coordinate order stays explicit here.
                let bedrock_index = usize::from(x) * 256 + usize::from(z) * 16 + usize::from(y);
                let mut visible = None;
                for storage_index in (0..storages.len()).rev() {
                    let storage = &storages[storage_index];
                    let palette_index = storage
                        .indices
                        .as_ref()
                        .and_then(|values| values.get(bedrock_index).copied())
                        .or_else(|| storage.palette_index_at(x, y, z));
                    let Some(palette_index) = palette_index.map(usize::from) else {
                        continue;
                    };
                    let Some(&canonical_index) =
                        source_state_to_canonical[storage_index].get(palette_index)
                    else {
                        continue;
                    };
                    let source_state_index = source_state_offsets[storage_index] + palette_index;
                    if !source_state_is_air[source_state_index] {
                        visible = Some((source_state_index, canonical_index));
                        break;
                    }
                }
                let canonical_index = if let Some((source_index, canonical_index)) = visible {
                    source_state_counts[source_index] += 1;
                    canonical_index
                } else {
                    fallback_air_count += 1;
                    air_state
                };
                let output_index = match output_index_by_state[canonical_index] {
                    Some(index) => index,
                    None => {
                        let index = output_palette_ids.len();
                        output_palette_ids.push(canonical_index);
                        output_index_by_state[canonical_index] = Some(index);
                        index
                    }
                };
                indices.push(output_index);
            }
        }
    }

    for (source_index, count) in source_state_counts.into_iter().enumerate() {
        if count == 0 {
            continue;
        }
        let raw_name = &source_state_names[source_index];
        *findings.blocks.entry(raw_name.clone()).or_default() += count;
        let mapped_name = mapped_block_name(raw_name);
        if mapped_name != raw_name {
            findings
                .renamed
                .insert(raw_name.clone(), mapped_name.to_owned());
        }
    }
    if fallback_air_count > 0 {
        *findings
            .blocks
            .entry("minecraft:air".to_owned())
            .or_default() += fallback_air_count;
    }

    let palette = output_palette_ids
        .into_iter()
        .map(|index| {
            let (name, properties) = &canonical_states[index];
            AnvilBlock {
                name: name.clone(),
                properties: properties.clone(),
            }
        })
        .collect::<Vec<_>>();
    AnvilBlockStates {
        data: (palette.len() > 1).then(|| LongArray::new(pack(&indices, 4))),
        palette,
    }
}

pub(crate) fn encode_chunk(
    chunk: &ChunkData,
    findings: &mut Findings,
    biome_names: Option<&BTreeMap<u32, String>>,
    allow_unmapped_biomes: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut sections = Vec::new();
    let mut subchunks = chunk.subchunks.iter().collect::<Vec<_>>();
    subchunks.sort_unstable_by_key(|&(&sy, _)| sy);
    for (&sy, subchunk) in subchunks {
        findings.min_section = Some(findings.min_section.map_or(sy, |value| value.min(sy)));
        findings.max_section = Some(findings.max_section.map_or(sy, |value| value.max(sy)));
        let SubChunkFormat::Paletted { storages, .. } = &subchunk.format else {
            findings.unsupported_sections += 1;
            continue;
        };
        if storages.len() > 1 {
            findings.layered_sections += 1;
        }
        let block_states = encode_block_states(storages, findings);
        let biome = chunk.biome_data.get(&(i32::from(sy)));
        if let Some(biome) = biome {
            for (&id, &count) in biome.palette.iter().zip(&biome.counts) {
                *findings.biomes.entry(id).or_default() += u64::from(count);
            }
        }
        let mut biome_palette = Vec::<String>::new();
        let mut biome_lookup = HashMap::<String, usize>::new();
        let mut biome_indices = Vec::with_capacity(64);
        for by in 0..4 {
            for bz in 0..4 {
                for bx in 0..4 {
                    let id = biome
                        .and_then(|data| data.biome_id_at(bx * 4, by * 4, bz * 4))
                        .unwrap_or(u32::MAX);
                    let name = match biome_names.and_then(|names| names.get(&id)) {
                        Some(name) => name.clone(),
                        None if biome_names.is_some() && !allow_unmapped_biomes => {
                            return Err(format!(
                                "biome ID {id} has no entry in the exact-version registry"
                            )
                            .into());
                        }
                        None if biome_names.is_some() => format!("msc:unmapped_bedrock_biome_{id}"),
                        None => "minecraft:plains".to_owned(),
                    };
                    let index = *biome_lookup.entry(name.clone()).or_insert_with(|| {
                        let index = biome_palette.len();
                        biome_palette.push(name);
                        index
                    });
                    biome_indices.push(index);
                }
            }
        }
        sections.push(AnvilSection {
            y: sy,
            block_states,
            biomes: AnvilBiomes {
                palette: biome_palette,
                data: (biome_lookup.len() > 1).then(|| LongArray::new(pack(&biome_indices, 1))),
            },
        });
    }
    fastnbt::to_bytes(&AnvilChunk {
        data_version: 3700,
        x: chunk.pos.x,
        z: chunk.pos.z,
        sections,
    })
    .map_err(Into::into)
}

fn pack(indices: &[usize], min_bits: usize) -> Vec<i64> {
    let max = indices.iter().copied().max().unwrap_or(0);
    let bits = (usize::BITS as usize - max.leading_zeros() as usize).max(min_bits);
    let per_long = 64 / bits;
    let mut packed = vec![0u64; indices.len().div_ceil(per_long)];
    for (cell, &index) in indices.iter().enumerate() {
        packed[cell / per_long] |= (index as u64) << ((cell % per_long) * bits);
    }
    packed.into_iter().map(|value| value as i64).collect()
}

pub(crate) fn encode_region(chunks: Vec<(i32, i32, Vec<u8>)>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = vec![0u8; 8192];
    for (x, z, nbt) in chunks {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&nbt)?;
        let compressed = encoder.finish()?;
        let length = compressed.len() + 1;
        let sectors = (length + 4).div_ceil(4096);
        if sectors > 255 {
            return Err("probe chunk exceeds Anvil's 255-sector limit".into());
        }
        let offset = bytes.len() / 4096;
        if offset >= 1 << 24 {
            return Err("probe region exceeds Anvil's location-table limit".into());
        }
        let location = ((z.rem_euclid(32) * 32 + x.rem_euclid(32)) * 4) as usize;
        bytes[location] = (offset >> 16) as u8;
        bytes[location + 1] = (offset >> 8) as u8;
        bytes[location + 2] = offset as u8;
        bytes[location + 3] = sectors as u8;
        bytes.extend_from_slice(&(length as u32).to_be_bytes());
        bytes.push(2);
        bytes.extend_from_slice(&compressed);
        bytes.resize(bytes.len().div_ceil(4096) * 4096, 0);
    }
    Ok(bytes)
}

fn property_string(key: &str, value: &bedrock_world::NbtTag) -> String {
    use bedrock_world::NbtTag as N;
    const BOOLEAN_STATES: &[&str] = &[
        "attached",
        "conditional",
        "disarmed",
        "down",
        "east",
        "enabled",
        "eye",
        "hanging",
        "has_record",
        "in_wall",
        "inverted",
        "lit",
        "locked",
        "north",
        "occupied",
        "open",
        "persistent",
        "powered",
        "short",
        "signal_fire",
        "snowy",
        "south",
        "triggered",
        "unstable",
        "up",
        "waterlogged",
        "west",
    ];
    match value {
        N::Byte(v) if BOOLEAN_STATES.contains(&key) => (*v != 0).to_string(),
        N::Byte(v) => v.to_string(),
        N::Short(v) => v.to_string(),
        N::Int(v) => v.to_string(),
        N::Long(v) => v.to_string(),
        N::Float(v) => v.to_string(),
        N::Double(v) => v.to_string(),
        N::String(v) => v.clone(),
        N::End => "0".to_owned(),
        N::ByteArray(_)
        | N::IntArray(_)
        | N::LongArray(_)
        | N::ShortArray(_)
        | N::List(_)
        | N::Compound(_) => format!("{value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedrock_world::{BlockState, NbtTag, block_storage_index};

    #[test]
    fn palette_transcode_preserves_layered_air_and_state_aliases() {
        let mut top_indices = vec![0; 4096];
        top_indices[block_storage_index(1, 0, 0)] = 1;
        let bottom = BlockPalette::with_unpacked_indices(
            vec![BlockState {
                name: "minecraft:grass".to_owned(),
                states: BTreeMap::from([("snowy".to_owned(), NbtTag::Byte(1))]),
                version: None,
            }],
            vec![0; 4096],
            None,
        );
        let top = BlockPalette::with_unpacked_indices(
            vec![
                BlockState {
                    name: "minecraft:air".to_owned(),
                    states: BTreeMap::new(),
                    version: None,
                },
                BlockState {
                    name: "minecraft:stone".to_owned(),
                    states: BTreeMap::new(),
                    version: None,
                },
            ],
            top_indices,
            None,
        );
        let mut findings = Findings::default();

        let encoded = encode_block_states(&[bottom, top], &mut findings);

        assert_eq!(encoded.palette.len(), 2);
        assert_eq!(encoded.palette[0].name, "minecraft:grass_block");
        assert_eq!(encoded.palette[0].properties.get("snowy").unwrap(), "true");
        assert_eq!(encoded.palette[1].name, "minecraft:stone");
        let mut expected_indices = vec![0; 4096];
        expected_indices[1] = 1;
        assert_eq!(
            encoded.data,
            Some(LongArray::new(pack(&expected_indices, 4)))
        );
        assert_eq!(findings.blocks.get("minecraft:grass"), Some(&4095));
        assert_eq!(findings.blocks.get("minecraft:stone"), Some(&1));
        assert!(!findings.blocks.contains_key("minecraft:air"));
        assert_eq!(
            findings.renamed.get("minecraft:grass"),
            Some(&"minecraft:grass_block".to_owned())
        );
    }
}
