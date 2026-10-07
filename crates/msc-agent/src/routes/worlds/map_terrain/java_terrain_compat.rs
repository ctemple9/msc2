//! Private, read-only compatibility copy for Java's 26.x dimension layout.
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fastnbt::Value;
use flate2::{Compression, read::GzDecoder, read::ZlibDecoder, write::ZlibEncoder};

const SECTOR: usize = 4096;
const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn staging_estimate(world: &Path, dimension: &str) -> io::Result<u64> {
    let relative = match dimension {
        "minecraft:overworld" => "dimensions/minecraft/overworld/region",
        "minecraft:the_nether" => "dimensions/minecraft/the_nether/region",
        "minecraft:the_end" => "dimensions/minecraft/the_end/region",
        _ => return Ok(0),
    };
    let mut source = world.to_path_buf();
    for segment in Path::new(relative).components() {
        source.push(segment);
        match fs::symlink_metadata(&source) {
            Ok(meta) if meta.file_type().is_dir() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
            _ => return Err(invalid("Java dimension path is not a regular directory")),
        }
    }
    // Reserve one working copy. Recompression growth is checked using the
    // actual normalized size before every write, rather than doubling all
    // regions and refusing worlds that already fit on disk.
    crate::map_staging::estimate_tree(&source)
}

pub(super) fn prepare(
    world: &Path,
    dimension: &str,
    cache: &Path,
    progress: &impl Fn(usize, usize),
) -> io::Result<PathBuf> {
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
    let estimate = crate::map_staging::estimate_tree(&source)?;
    crate::map_staging::ensure_space(cache, estimate.saturating_add(64 * 1024 * 1024))?;
    let staged = cache.join("world");
    let target = staged.join(legacy);
    fs::create_dir_all(&target)?;
    fs::copy(level, staged.join("level.dat"))?;
    let entries = fs::read_dir(&source)?.collect::<io::Result<Vec<_>>>()?;
    let mut regions = Vec::new();
    for entry in entries {
        if entry.file_name().to_string_lossy().ends_with(".mca") && entry.file_type()?.is_file() {
            regions.push(entry);
        }
    }
    let total = regions.len();
    progress(0, total);
    for (index, entry) in regions.into_iter().enumerate() {
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
        crate::map_staging::ensure_space(cache, normalized.len() as u64 + 64 * 1024 * 1024)?;
        fs::write(target.join(name), normalized)?;
        progress(index + 1, total);
    }
    Ok(staged)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn normalize_region(region: &[u8]) -> io::Result<Vec<u8>> {
    if region.len() < 2 * SECTOR {
        return Err(invalid("Java region header is incomplete"));
    }
    let mut output = vec![0u8; 2 * SECTOR];
    output[SECTOR..2 * SECTOR].copy_from_slice(&region[SECTOR..2 * SECTOR]);
    for index in 0..1024 {
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
        let mut chunk: Value =
            fastnbt::from_bytes(&decoded).map_err(|_| invalid("Java chunk NBT is invalid"))?;
        if normalize_chunk(&mut chunk) {
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
            let mut legacy = HashMap::from([("Name".to_string(), Value::String(name.clone()))]);
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
    let block = name.strip_prefix("minecraft:")?;
    let (key, value) = if ["_log", "_wood", "_stem", "_hyphae"]
        .iter()
        .any(|suffix| block.ends_with(suffix))
        || block == "deepslate"
    {
        ("axis", "y")
    } else if block == "grass_block" {
        ("snowy", "false")
    } else if matches!(block, "tall_grass" | "tall_seagrass") {
        ("half", "lower")
    } else if block.ends_with("_amethyst_bud") {
        ("facing", "up")
    } else {
        return None;
    };
    Some(Value::Compound(HashMap::from([(
        key.to_string(),
        Value::String(value.to_string()),
    )])))
}
