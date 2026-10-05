//! Private, read-only compatibility copy for Java's 26.x dimension layout.
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fastnbt::Value;
use flate2::{Compression, read::GzDecoder, read::ZlibDecoder, write::ZlibEncoder};

const SECTOR: usize = 4096;
const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn prepare(world: &Path, dimension: &str, cache: &Path) -> io::Result<PathBuf> {
    let (modern, legacy) = match dimension {
        "minecraft:overworld" => ("dimensions/minecraft/overworld/region", "region"),
        "minecraft:the_nether" => ("dimensions/minecraft/the_nether/region", "DIM-1/region"),
        "minecraft:the_end" => ("dimensions/minecraft/the_end/region", "DIM1/region"),
        _ => return Ok(world.to_path_buf()),
    };
    let mut source = world.to_path_buf();
    for segment in Path::new(modern).components() {
        source.push(segment);
        match fs::symlink_metadata(&source) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(world.to_path_buf());
            }
            Ok(_) => return Err(invalid("Java dimension path is not a regular directory")),
            Err(error) => return Err(error),
        }
    }
    let level = world.join("level.dat");
    if !fs::symlink_metadata(&level)?.file_type().is_file() {
        return Err(invalid("Java level.dat is not a regular file"));
    }
    let staged = cache.join("world");
    let target = staged.join(legacy);
    fs::create_dir_all(&target)?;
    fs::copy(level, staged.join("level.dat"))?;
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if !name.to_string_lossy().ends_with(".mca") || !entry.file_type()?.is_file() {
            continue;
        }
        if entry.metadata()?.len() > MAX_REGION_BYTES {
            return Err(io::Error::other(
                "Java region exceeds the terrain renderer limit",
            ));
        }
        let raw = fs::read(path)?;
        let normalized = normalize_region(&raw)?;
        fs::write(target.join(name), normalized)?;
    }
    Ok(staged)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn normalize_region(region: &[u8]) -> io::Result<Vec<u8>> {
    normalize_region_with(
        region,
        &mut |chunk, _, _| Ok(normalize_chunk(chunk)),
        &|| false,
    )
}
fn normalize_region_with(
    region: &[u8],
    adapt: &mut dyn FnMut(&mut Value, usize, &str) -> io::Result<bool>,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Vec<u8>> {
    if region.len() < 2 * SECTOR {
        return Err(invalid("Java region header is incomplete"));
    }
    let mut output = vec![0u8; 2 * SECTOR];
    output[SECTOR..2 * SECTOR].copy_from_slice(&region[SECTOR..2 * SECTOR]);
    for index in 0..1024 {
        msc_infrastructure::map_assets::poll(cancel)?;
        let entry = index * 4;
        let offset = (usize::from(region[entry]) << 16)
            | (usize::from(region[entry + 1]) << 8)
            | usize::from(region[entry + 2]);
        let sectors = usize::from(region[entry + 3]);
        if offset == 0 || sectors == 0 {
            continue;
        }
        let start = offset
            .checked_mul(SECTOR)
            .ok_or_else(|| invalid("Region offset overflow"))?;
        start
            .checked_add(sectors * SECTOR)
            .ok_or_else(|| invalid("Region length overflow"))?;
        let header = region
            .get(start..start + 5)
            .ok_or_else(|| invalid("Region chunk header is incomplete"))?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        if length < 1 || length + 4 > sectors * SECTOR || start + 4 + length > region.len() {
            return Err(invalid("Region chunk length is invalid"));
        }
        let compression = header[4];
        if compression & 0x80 != 0 {
            return Err(invalid("External Java chunk data is unsupported"));
        }
        let payload = &region[start + 5..start + 4 + length];
        let mut decoded = Vec::new();
        match compression {
            1 => GzDecoder::new(payload)
                .take((MAX_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut decoded)?,
            2 => ZlibDecoder::new(payload)
                .take((MAX_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut decoded)?,
            3 => {
                decoded.extend_from_slice(payload);
                decoded.len()
            }
            _ => return Err(invalid("Unsupported Java chunk compression")),
        };
        if decoded.len() > MAX_CHUNK_BYTES {
            return Err(invalid("Java chunk exceeds the terrain renderer limit"));
        }
        msc_infrastructure::map_assets::saved_terrain::validate_nbt(&decoded)?;
        let mut chunk: Value =
            fastnbt::from_bytes(&decoded).map_err(|_| invalid("Java chunk NBT is invalid"))?;
        if adapt(
            &mut chunk,
            index,
            &msc_infrastructure::map_assets::hash(&decoded),
        )? {
            decoded = fastnbt::to_bytes(&chunk)
                .map_err(|_| invalid("Could not encode Java chunk NBT"))?;
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&decoded)?;
        let compressed = encoder.finish()?;
        let record_len = compressed.len() + 5;
        let needed = record_len.div_ceil(SECTOR);
        if needed > 255 {
            return Err(invalid("Normalized Java chunk exceeds region sector limit"));
        }
        let new_offset = output.len() / SECTOR;
        if new_offset > 0x00ff_ffff {
            return Err(invalid("Normalized Java region exceeds offset limit"));
        }
        output[entry] = (new_offset >> 16) as u8;
        output[entry + 1] = (new_offset >> 8) as u8;
        output[entry + 2] = new_offset as u8;
        output[entry + 3] = needed as u8;
        output.extend_from_slice(&((compressed.len() + 1) as u32).to_be_bytes());
        output.push(2);
        output.extend_from_slice(&compressed);
        output.resize(output.len().div_ceil(SECTOR) * SECTOR, 0);
    }
    Ok(output)
}

fn normalize_chunk(chunk: &mut Value) -> bool {
    let Value::Compound(root) = chunk else {
        return false;
    };
    let Some(Value::List(sections)) = root.get_mut("sections") else {
        return false;
    };
    let mut changed = false;
    for section in sections {
        let Value::Compound(section) = section else {
            continue;
        };
        let Some(Value::Compound(states)) = section.get_mut("block_states") else {
            continue;
        };
        let Some(Value::List(palette)) = states.get_mut("palette") else {
            continue;
        };
        for entry in palette {
            let (name, properties) = match entry {
                Value::String(name) => (name.clone(), None),
                Value::Compound(fields) if !fields.contains_key("Name") => {
                    let name = fields.get("id").or_else(|| fields.get(""));
                    let Some(Value::String(name)) = name else {
                        continue;
                    };
                    (name.clone(), fields.get("properties").cloned())
                }
                _ => continue,
            };
            let mut legacy = if let Value::Compound(fields) = entry {
                fields.clone()
            } else {
                HashMap::new()
            };
            legacy.insert("Name".into(), Value::String(name.clone()));
            if let Some(properties) = properties.or_else(|| default_properties(&name)) {
                legacy.insert("Properties".to_string(), properties);
            }
            *entry = Value::Compound(legacy);
            changed = true;
        }
    }
    changed
}

fn default_properties(name: &str) -> Option<Value> {
    let state = msc_infrastructure::map_assets::saved_terrain::modern_default_state(name);
    if state.is_empty() {
        None
    } else {
        Some(Value::Compound(
            state
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        ))
    }
}

#[derive(serde::Serialize)]
pub(super) struct AdaptedWorld {
    pub path: PathBuf,
    pub renderer_dimension: String,
    pub snapshot_id: String,
    pub palettes:
        std::collections::BTreeMap<String, msc_infrastructure::map_assets::adapter::PaletteResult>,
    pub chunks: std::collections::BTreeMap<String, String>,
}
impl AdaptedWorld {
    /// Include the seam apron; a neighbor's changed parent/texture invalidates
    /// the touching tile even when that tile's own palette did not change.
    pub fn tile_fingerprint(&self, x: i32, z: i32, width: i32) -> io::Result<String> {
        let mut inputs = std::collections::BTreeMap::new();
        for cx in x * width - 1..=(x + 1) * width {
            for cz in z * width - 1..=(z + 1) * width {
                let key = format!("{cx},{cz}");
                inputs.insert(key.clone(), self.chunks.get(&key));
            }
        }
        msc_infrastructure::map_assets::hash_json(&inputs)
    }
}

pub(super) fn prepare_adapted(
    world: &Path,
    dimension: &str,
    cache: &Path,
    inventory: &msc_infrastructure::map_assets::inventory::Inventory,
    cancel: &dyn Fn() -> bool,
) -> io::Result<AdaptedWorld> {
    use msc_infrastructure::map_assets::{adapter::Adapter, hash_json, *};
    use std::collections::BTreeMap;
    if !msc_domain::map_assets::valid_resource_id(dimension) {
        return Err(error("invalid_dimension"));
    }
    let (namespace, path) = dimension
        .split_once(':')
        .ok_or_else(|| error("invalid_dimension"))?;
    let modern = world
        .join("dimensions")
        .join(namespace)
        .join(path)
        .join("region");
    let (legacy, renderer_dimension) = match dimension {
        "minecraft:overworld" => ("region", dimension),
        "minecraft:the_nether" => ("DIM-1/region", dimension),
        "minecraft:the_end" => ("DIM1/region", dimension),
        _ => ("region", "minecraft:overworld"),
    };
    let source = if modern.exists() {
        modern
    } else {
        world.join(legacy)
    };
    if !matches!(
        dimension,
        "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
    ) && !world
        .join("dimensions")
        .join(namespace)
        .join(path)
        .join("region")
        .exists()
    {
        return Err(error("saved_custom_dimension_missing"));
    }
    safe_path(&source)?;
    let staged = cache.join("world");
    let target = staged.join(legacy);
    fs::create_dir_all(&target)?;
    let level = read(&world.join("level.dat"), MAX_JSON)?;
    fs::write(staged.join("level.dat"), &level)?;
    let source_stamp = stamp(&fs::symlink_metadata(&source)?);
    let mut result = AdaptedWorld {
        path: staged,
        renderer_dimension: renderer_dimension.into(),
        snapshot_id: String::new(),
        palettes: BTreeMap::new(),
        chunks: BTreeMap::new(),
    };
    let adapter = Adapter { inventory };
    adapter.materialize(&cache.join("assets/minecraft"), cancel)?;
    let mut total = 0u64;
    let mut snapshot_chunks = BTreeMap::new();
    let paths = msc_infrastructure::map_assets::inventory::source_paths(&source)?;
    for region in paths {
        poll(cancel)?;
        let name = region
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| error("unsafe_region_name"))?;
        if !name.ends_with(".mca") {
            continue;
        }
        let parts = name.split('.').collect::<Vec<_>>();
        if !matches!(parts.as_slice(),["r",x,z,"mca"] if x.parse::<i32>().is_ok()&&z.parse::<i32>().is_ok())
        {
            return Err(error("invalid_region_name"));
        }
        let raw = read(&region, MAX_REGION_BYTES)?;
        total += raw.len() as u64;
        if total > 2 * 1024 * 1024 * 1024 {
            return Err(error("snapshot_region_budget"));
        }
        let rx = parts[1]
            .parse::<i32>()
            .map_err(|_| error("invalid_region_name"))?;
        let rz = parts[2]
            .parse::<i32>()
            .map_err(|_| error("invalid_region_name"))?;
        let normalized = normalize_region_with(
            &raw,
            &mut |chunk, index, source_hash| {
                normalize_chunk(chunk);
                let Value::Compound(root) = chunk else {
                    return Err(error("invalid_chunk_nbt"));
                };
                let (Some(Value::Int(x)), Some(Value::Int(z))) =
                    (root.get("xPos"), root.get("zPos"))
                else {
                    return Err(error("chunk_coordinate_missing"));
                };
                let key = format!("{x},{z}");
                if x.unsigned_abs() > 1_875_000 || z.unsigned_abs() > 1_875_000 {
                    return Err(error("chunk_coordinate_limit"));
                }
                if i64::from(*x) != i64::from(rx) * 32 + (index % 32) as i64
                    || i64::from(*z) != i64::from(rz) * 32 + (index / 32) as i64
                {
                    return Err(error("chunk_coordinate_mismatch"));
                }
                let chunk_x = *x;
                let chunk_z = *z;
                let mut entities = std::collections::BTreeSet::new();
                if let Some(Value::List(values)) = root.get("block_entities")
                    && let Some(Value::List(sections)) = root.get("sections")
                {
                    if values.len() > 4096 || sections.len() > 256 {
                        return Err(error("chunk_context_limit"));
                    }
                    for value in values {
                        let Value::Compound(entity) = value else {
                            continue;
                        };
                        let (Some(Value::Int(x)), Some(Value::Int(y)), Some(Value::Int(z))) =
                            (entity.get("x"), entity.get("y"), entity.get("z"))
                        else {
                            continue;
                        };
                        if x.div_euclid(16) != chunk_x || z.div_euclid(16) != chunk_z {
                            continue;
                        }
                        for section in sections {
                            let Value::Compound(section) = section else {
                                continue;
                            };
                            let sy = match section.get("Y") {
                                Some(Value::Byte(v)) => i32::from(*v),
                                Some(Value::Int(v)) => *v,
                                _ => continue,
                            };
                            if sy == y.div_euclid(16)
                                && let Some(Value::Compound(palette)) =
                                    msc_infrastructure::map_assets::saved_terrain::palette_at(
                                        section, *x, *y, *z,
                                    )?
                                && let Some(Value::String(name)) = palette.get("Name")
                            {
                                entities.insert(name.clone());
                            }
                        }
                    }
                }
                let before = source_hash.to_string();
                let mut used = BTreeMap::new();
                if let Some(Value::List(sections)) = root.get("sections") {
                    for section in sections {
                        if let Value::Compound(section) = section
                            && let Some(Value::Compound(biomes)) = section.get("biomes")
                            && let Some(Value::List(palette)) = biomes.get("palette")
                        {
                            for biome in palette {
                                if let Value::String(id) = biome
                                    && let Some((namespace, path)) = id.split_once(':')
                                {
                                    let name =
                                        format!("data/{namespace}/worldgen/biome/{path}.json");
                                    used.insert(
                                        name.clone(),
                                        hash_json(
                                            &inventory
                                                .resources
                                                .get(&name)
                                                .map(|r| (&r.sha256, r.invalid, r.conflict)),
                                        )?,
                                    );
                                }
                            }
                        }
                    }
                }
                if let Some(Value::List(sections)) = root.get_mut("sections") {
                    for section in sections {
                        let Value::Compound(section) = section else {
                            return Err(error("invalid_chunk_section"));
                        };
                        let Some(Value::Compound(states)) = section.get_mut("block_states") else {
                            continue;
                        };
                        let Some(Value::List(palette)) = states.get_mut("palette") else {
                            return Err(error("invalid_palette_state"));
                        };
                        if palette.len() > 4096 {
                            return Err(error("invalid_chunk_palette"));
                        }
                        for entry in palette {
                            let Value::Compound(fields) = entry else {
                                return Err(error("invalid_palette_state"));
                            };
                            let Some(Value::String(id)) = fields.get("Name") else {
                                return Err(error("invalid_palette_id"));
                            };
                            let id = id.clone();
                            let mut state = BTreeMap::new();
                            if let Some(Value::Compound(props)) = fields.get("Properties") {
                                for (k, v) in props {
                                    let Value::String(v) = v else {
                                        return Err(error("invalid_palette_property"));
                                    };
                                    state.insert(k.clone(), v.clone());
                                }
                            }
                            let palette_key = hash_json(&(&id, &state, entities.contains(&id)))?;
                            if !result.palettes.contains_key(&palette_key) {
                                if result.palettes.len() >= 100_000 {
                                    return Err(error("palette_inventory_limit"));
                                }
                                result.palettes.insert(
                                    palette_key.clone(),
                                    adapter.palette(&id, &state, entities.contains(&id))?,
                                );
                            }
                            let inspected = &result.palettes[&palette_key];
                            fields.insert(
                                "Name".into(),
                                Value::String(inspected.renderer_id.clone()),
                            );
                            used.insert(palette_key, inspected.fingerprint.clone());
                        }
                    }
                }
                snapshot_chunks.insert(key.clone(), before.clone());
                result.chunks.insert(key, hash_json(&(before, used))?);
                Ok(true)
            },
            cancel,
        )?;
        fs::write(target.join(name), normalized)?;
        if hash(&read(&region, MAX_REGION_BYTES)?) != hash(&raw) {
            return Err(error("snapshot_input_changed"));
        }
    }
    if stamp(&fs::symlink_metadata(&source)?) != source_stamp
        || read(&world.join("level.dat"), MAX_JSON)? != level
    {
        return Err(error("snapshot_input_changed"));
    }
    result.snapshot_id = hash_json(&(dimension, hash(&level), &snapshot_chunks))?;
    // Fallbacks are explicit checkerboard inset models, never resolved models or
    // occluders. They retain neighboring real terrain and their diagnostic IDs.
    write_fallbacks(&result, &cache.join("assets/minecraft"))?;
    fs::write(
        cache.join("palette-receipt.json"),
        serde_json::to_vec(&result).map_err(|_| error("serialization_failed"))?,
    )?;
    Ok(result)
}
fn write_fallbacks(world: &AdaptedWorld, assets: &Path) -> io::Result<()> {
    use msc_infrastructure::map_assets::{adapter::PREFIX, error};
    let texture = format!("{PREFIX}/fallback");
    let path = assets.join("textures").join(format!("{texture}.png"));
    fs::create_dir_all(path.parent().ok_or_else(|| error("unsafe_resource_path"))?)?;
    let file = fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, 16, 16);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|_| error("fallback_texture_encoding"))?;
    let mut pixels = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            pixels.extend_from_slice(if (x / 4 + y / 4) % 2 == 0 {
                &[160, 70, 160, 255]
            } else {
                &[35, 35, 35, 255]
            });
        }
    }
    writer
        .write_image_data(&pixels)
        .map_err(|_| error("fallback_texture_encoding"))?;
    for palette in world
        .palettes
        .values()
        .filter(|p| p.renderer_id.contains("/fallback/"))
    {
        let local = palette
            .renderer_id
            .strip_prefix("minecraft:")
            .ok_or_else(|| error("invalid_palette_id"))?;
        let block = assets.join("blockstates").join(format!("{local}.json"));
        fs::create_dir_all(
            block
                .parent()
                .ok_or_else(|| error("unsafe_resource_path"))?,
        )?;
        fs::write(
            block,
            serde_json::to_vec(
                &serde_json::json!({"variants":{"":{"model":format!("{PREFIX}/fallback")}}}),
            )?,
        )?;
    }
    let model = assets
        .join("models")
        .join(format!("{PREFIX}/fallback.json"));
    fs::create_dir_all(
        model
            .parent()
            .ok_or_else(|| error("unsafe_resource_path"))?,
    )?;
    let faces = ["north", "south", "east", "west", "up", "down"]
        .into_iter()
        .map(|face| (face, serde_json::json!({"texture":"#all"})))
        .collect::<BTreeMap<_, _>>();
    fs::write(
        model,
        serde_json::to_vec(
            &serde_json::json!({"textures":{"all":texture},"elements":[{"from":[1,1,1],"to":[15,15,15],"faces":faces}]}),
        )?,
    )?;
    Ok(())
}
