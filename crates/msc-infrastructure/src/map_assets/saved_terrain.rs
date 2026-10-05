//! Reads only saved Anvil chunks in the requested bounds, without generating terrain.
use super::*;
use fastnbt::Value;
use flate2::read::{GzDecoder, ZlibDecoder};
use msc_domain::map_assets::Area;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;
use zip::ZipArchive;
const MAX_CHUNK: u64 = 8 * 1024 * 1024;
#[derive(Clone)]
pub struct BlockSample {
    pub id: String,
    pub state: BTreeMap<String, String>,
    pub position: [i32; 3],
    pub entity: bool,
}
pub struct Terrain {
    pub blocks: Vec<BlockSample>,
    pub missing: Vec<[i32; 3]>,
    pub snapshot_id: String,
    pub chunks: u64,
}
#[derive(Clone)]
pub enum WorldSource {
    Directory(PathBuf),
    Archive(PathBuf),
}
impl WorldSource {
    pub fn recorded_game_version(&self) -> io::Result<Option<String>> {
        let Some(compressed) = self.bytes("level.dat", MAX_JSON)? else {
            return Err(error("missing_level_dat"));
        };
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(MAX_CHUNK + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > MAX_CHUNK {
            return Err(error("level_dat_decoded_limit"));
        }
        validate_nbt(&raw)?;
        let value: Value = fastnbt::from_bytes(&raw).map_err(|_| error("invalid_level_dat"))?;
        let version = compound(&value)
            .and_then(|root| root.get("Data"))
            .and_then(compound)
            .and_then(|data| data.get("Version"))
            .and_then(compound)
            .and_then(|version| version.get("Name"))
            .and_then(string);
        Ok(version.map(str::to_string))
    }
    fn bytes(&self, path: &str, max: u64) -> io::Result<Option<Vec<u8>>> {
        match self {
            Self::Directory(root) => {
                let path = root.join(path);
                match read(&path, max) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
                    Err(error) => Err(error),
                }
            }
            Self::Archive(archive_path) => {
                let mut archive = ZipArchive::new(open(archive_path)?)
                    .map_err(|_| error("invalid_world_archive"))?;
                if archive.len() > 200_000 {
                    return Err(error("archive_entry_limit"));
                }
                let mut prefix = None;
                let mut names = BTreeSet::new();
                for i in 0..archive.len() {
                    let entry = archive
                        .by_index(i)
                        .map_err(|_| error("invalid_world_archive"))?;
                    let name = entry.name();
                    if !safe_member(name)
                        || !names.insert(name.trim_end_matches('/').to_ascii_lowercase())
                    {
                        return Err(error("unsafe_or_duplicate_archive_path"));
                    }
                    if entry
                        .unix_mode()
                        .is_some_and(|m| ![0, 0o100000, 0o040000].contains(&(m & 0o170000)))
                    {
                        return Err(error("archive_link_or_special_file"));
                    }
                    if name == "level.dat" || name.ends_with("/level.dat") {
                        if prefix.is_some() {
                            return Err(error("ambiguous_world_archive"));
                        }
                        prefix = Some(name.trim_end_matches("level.dat").to_string());
                    }
                }
                let prefix = prefix.ok_or_else(|| error("missing_level_dat"))?;
                let name = format!("{prefix}{path}");
                let mut entry = match archive.by_name(&name) {
                    Ok(entry) => entry,
                    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
                    Err(_) => return Err(error("invalid_world_archive")),
                };
                if entry.size() > max {
                    return Err(error("world_entry_byte_limit"));
                }
                let mut raw = Vec::new();
                entry.by_ref().take(max + 1).read_to_end(&mut raw)?;
                if raw.len() as u64 > max || raw.len() as u64 != entry.size() {
                    return Err(error("world_entry_byte_limit"));
                }
                Ok(Some(raw))
            }
        }
    }
    pub fn inspect(
        &self,
        dimension: &str,
        area: Area,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<Terrain> {
        area.validate().map_err(error)?;
        if !msc_domain::map_assets::valid_resource_id(dimension) || !dimension.contains(':') {
            return Err(error("invalid_dimension"));
        }
        let (namespace, path) = dimension
            .split_once(':')
            .ok_or_else(|| error("invalid_dimension"))?;
        let modern = format!("dimensions/{namespace}/{path}/region");
        let legacy = match dimension {
            "minecraft:overworld" => Some("region"),
            "minecraft:the_nether" => Some("DIM-1/region"),
            "minecraft:the_end" => Some("DIM1/region"),
            _ => None,
        };
        let level = self
            .bytes("level.dat", MAX_JSON)?
            .ok_or_else(|| error("missing_level_dat"))?;
        let level_id = hash(&level);
        let mut receipts = BTreeMap::from([("level.dat".to_string(), level_id.clone())]);
        let mut blocks = Vec::new();
        let mut missing = Vec::new();
        let mut chunks = 0;
        let mut regions: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
        let mut region_bytes = 0u64;
        for x in area.min[0].div_euclid(16)..=area.max[0].div_euclid(16) {
            for z in area.min[2].div_euclid(16)..=area.max[2].div_euclid(16) {
                poll(cancel)?;
                let file = format!("r.{}.{}.mca", x.div_euclid(32), z.div_euclid(32));
                let preferred = format!("{modern}/{file}");
                if !regions.contains_key(&preferred) {
                    let mut raw = self.bytes(&preferred, MAX_ENTRY)?;
                    if raw.is_none()
                        && let Some(legacy) = legacy
                    {
                        raw = self.bytes(&format!("{legacy}/{file}"), MAX_ENTRY)?;
                    }
                    region_bytes += raw.as_ref().map(|r| r.len() as u64).unwrap_or(0);
                    if region_bytes > 512 * 1024 * 1024 {
                        return Err(error("inspection_region_budget"));
                    }
                    regions.insert(preferred.clone(), raw);
                }
                let raw = regions[&preferred]
                    .as_deref()
                    .map(|region| chunk(region, x, z))
                    .transpose()?
                    .flatten();
                let Some(raw) = raw else {
                    missing.push([x * 16, area.min[1], z * 16]);
                    continue;
                };
                receipts.insert(format!("{x},{z}"), hash(&raw));
                validate_nbt(&raw)?;
                let value: Value =
                    fastnbt::from_bytes(&raw).map_err(|_| error("invalid_chunk_nbt"))?;
                let root = compound(&value).ok_or_else(|| error("invalid_chunk_nbt"))?;
                if root.get("xPos").and_then(integer) != Some(x)
                    || root.get("zPos").and_then(integer) != Some(z)
                {
                    return Err(error("chunk_coordinate_mismatch"));
                }
                if !root
                    .get("Status")
                    .and_then(string)
                    .is_some_and(|s| s == "full" || s == "minecraft:full")
                {
                    missing.push([x * 16, area.min[1], z * 16]);
                    continue;
                }
                let sections = root
                    .get("sections")
                    .and_then(list)
                    .ok_or_else(|| error("unsupported_saved_chunk_format"))?;
                let mut section_map = BTreeMap::new();
                for section in sections {
                    let s = compound(section).ok_or_else(|| error("invalid_chunk_section"))?;
                    let y = s
                        .get("Y")
                        .and_then(integer)
                        .ok_or_else(|| error("invalid_section_y"))?;
                    if section_map.insert(y, s).is_some() {
                        return Err(error("duplicate_chunk_section"));
                    }
                }
                let mut entities = BTreeSet::new();
                if let Some(values) = root.get("block_entities").and_then(list) {
                    for entity in values {
                        if let Some(e) = compound(entity)
                            && let (Some(ex), Some(ey), Some(ez)) = (
                                e.get("x").and_then(integer),
                                e.get("y").and_then(integer),
                                e.get("z").and_then(integer),
                            )
                        {
                            entities.insert([ex, ey, ez]);
                        }
                    }
                }
                for y in area.min[1]..=area.max[1] {
                    poll(cancel)?;
                    for bz in (z * 16).max(area.min[2])..=(z * 16 + 15).min(area.max[2]) {
                        for bx in (x * 16).max(area.min[0])..=(x * 16 + 15).min(area.max[0]) {
                            let position = [bx, y, bz];
                            let palette_state = match section_map.get(&y.div_euclid(16)) {
                                Some(section) => palette_at(section, bx, y, bz)?,
                                None => None,
                            };
                            let (id, state) = if let Some(palette) = palette_state {
                                let fields = compound(palette);
                                let id = string(palette)
                                    .or_else(|| {
                                        fields
                                            .and_then(|f| {
                                                f.get("Name")
                                                    .or_else(|| f.get("id"))
                                                    .or_else(|| f.get(""))
                                            })
                                            .and_then(string)
                                    })
                                    .ok_or_else(|| error("invalid_palette_id"))?;
                                if !msc_domain::map_assets::valid_resource_id(id) {
                                    return Err(error("invalid_palette_id"));
                                }
                                let mut state = BTreeMap::new();
                                if let Some(props) = fields
                                    .and_then(|f| {
                                        f.get("Properties").or_else(|| f.get("properties"))
                                    })
                                    .and_then(compound)
                                {
                                    for (k, v) in props {
                                        state.insert(
                                            k.clone(),
                                            string(v)
                                                .ok_or_else(|| error("invalid_palette_property"))?
                                                .into(),
                                        );
                                    }
                                }
                                if !fields.is_some_and(|f| f.contains_key("Name"))
                                    && state.is_empty()
                                {
                                    state = modern_default_state(id);
                                }
                                (id.into(), state)
                            } else {
                                ("minecraft:air".into(), BTreeMap::new())
                            };
                            blocks.push(BlockSample {
                                id,
                                state,
                                position,
                                entity: entities.contains(&position),
                            });
                        }
                    }
                }
                chunks += 1;
            }
        }
        if hash(
            &self
                .bytes("level.dat", MAX_JSON)?
                .ok_or_else(|| error("input_changed"))?,
        ) != level_id
        {
            return Err(error("input_changed"));
        }
        // Compare complete region bytes, including timestamps and chunks outside the area.
        for (preferred, before) in regions {
            poll(cancel)?;
            let mut after = self.bytes(&preferred, MAX_ENTRY)?;
            if after.is_none()
                && let Some(legacy) = legacy
            {
                after = self.bytes(
                    &format!("{legacy}/{}", preferred.rsplit('/').next().unwrap_or("")),
                    MAX_ENTRY,
                )?;
            }
            if before.as_deref().map(hash) != after.as_deref().map(hash) {
                return Err(error("input_changed"));
            }
        }
        Ok(Terrain {
            blocks,
            missing,
            snapshot_id: hash_json(&(dimension, area, receipts))?,
            chunks,
        })
    }
}
fn compound(value: &Value) -> Option<&HashMap<String, Value>> {
    if let Value::Compound(v) = value {
        Some(v)
    } else {
        None
    }
}
fn list(value: &Value) -> Option<&Vec<Value>> {
    if let Value::List(v) = value {
        Some(v)
    } else {
        None
    }
}
fn string(value: &Value) -> Option<&str> {
    if let Value::String(v) = value {
        Some(v)
    } else {
        None
    }
}
fn integer(value: &Value) -> Option<i32> {
    match value {
        Value::Byte(v) => Some(i32::from(*v)),
        Value::Short(v) => Some(i32::from(*v)),
        Value::Int(v) => Some(*v),
        _ => None,
    }
}
/// Defaults encoded implicitly by modern vanilla palettes; do not invent mod properties.
pub fn modern_default_state(name: &str) -> BTreeMap<String, String> {
    let Some(block) = name.strip_prefix("minecraft:") else {
        return BTreeMap::new();
    };
    let (key, value) = if ["_log", "_wood", "_stem", "_hyphae"]
        .iter()
        .any(|s| block.ends_with(s))
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
        return BTreeMap::new();
    };
    BTreeMap::from([(key.into(), value.into())])
}
pub fn palette_at(
    section: &HashMap<String, Value>,
    x: i32,
    y: i32,
    z: i32,
) -> io::Result<Option<&Value>> {
    let Some(states) = section.get("block_states").and_then(compound) else {
        return Ok(None);
    };
    let palette = states
        .get("palette")
        .and_then(list)
        .ok_or_else(|| error("missing_chunk_palette"))?;
    if palette.is_empty() || palette.len() > 4096 {
        return Err(error("invalid_chunk_palette"));
    }
    let index = if palette.len() == 1 {
        0
    } else {
        let bits = (usize::BITS - (palette.len() - 1).leading_zeros()).max(4) as usize;
        let per = 64 / bits;
        let Value::LongArray(data) = states
            .get("data")
            .ok_or_else(|| error("missing_palette_data"))?
        else {
            return Err(error("invalid_palette_data"));
        };
        if data.len() != 4096usize.div_ceil(per) {
            return Err(error("invalid_palette_data"));
        }
        let index = (y.rem_euclid(16) * 256 + z.rem_euclid(16) * 16 + x.rem_euclid(16)) as usize;
        ((data[index / per] as u64 >> (index % per * bits)) & ((1u64 << bits) - 1)) as usize
    };
    palette
        .get(index)
        .map(Some)
        .ok_or_else(|| error("palette_index_out_of_bounds"))
}
fn chunk(region: &[u8], x: i32, z: i32) -> io::Result<Option<Vec<u8>>> {
    if region.len() < 8192 {
        return Err(error("incomplete_region_header"));
    }
    let index = (x.rem_euclid(32) + z.rem_euclid(32) * 32) as usize * 4;
    let offset = (usize::from(region[index]) << 16)
        | (usize::from(region[index + 1]) << 8)
        | usize::from(region[index + 2]);
    let sectors = usize::from(region[index + 3]);
    if offset == 0 && sectors == 0 {
        return Ok(None);
    }
    if offset < 2 || sectors == 0 {
        return Err(error("invalid_region_location"));
    }
    let start = offset * 4096;
    let header = region
        .get(start..start + 5)
        .ok_or_else(|| error("invalid_region_location"))?;
    let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
    if length < 1 || length + 4 > sectors * 4096 || start + sectors * 4096 > region.len() {
        return Err(error("invalid_region_record"));
    }
    let payload = region
        .get(start + 5..start + 4 + length)
        .ok_or_else(|| error("invalid_region_record"))?;
    let mut raw = Vec::new();
    match header[4] {
        1 => {
            GzDecoder::new(payload)
                .take(MAX_CHUNK + 1)
                .read_to_end(&mut raw)?;
        }
        2 => {
            ZlibDecoder::new(payload)
                .take(MAX_CHUNK + 1)
                .read_to_end(&mut raw)?;
        }
        3 => raw.extend_from_slice(payload),
        _ => return Err(error("unsupported_chunk_compression_or_external_data")),
    }
    if raw.len() as u64 > MAX_CHUNK {
        return Err(error("chunk_decoded_byte_limit"));
    }
    Ok(Some(raw))
}
pub fn validate_nbt(raw: &[u8]) -> io::Result<()> {
    let mut cursor = Cursor::new(raw);
    if byte(&mut cursor)? != 10 {
        return Err(error("invalid_chunk_nbt"));
    }
    skip_string(&mut cursor)?;
    let mut nodes = 0;
    skip(&mut cursor, 10, 0, &mut nodes)?;
    if cursor.position() != raw.len() as u64 {
        return Err(error("invalid_chunk_nbt"));
    }
    Ok(())
}
fn take(cursor: &mut Cursor<&[u8]>, count: u64) -> io::Result<()> {
    let end = cursor
        .position()
        .checked_add(count)
        .ok_or_else(|| error("invalid_chunk_nbt"))?;
    if end > cursor.get_ref().len() as u64 {
        return Err(error("invalid_chunk_nbt"));
    }
    cursor.seek(SeekFrom::Start(end))?;
    Ok(())
}
fn byte(cursor: &mut Cursor<&[u8]>) -> io::Result<u8> {
    let mut b = [0];
    cursor.read_exact(&mut b)?;
    Ok(b[0])
}
fn count(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut b = [0; 4];
    cursor.read_exact(&mut b)?;
    let n = i32::from_be_bytes(b);
    if n < 0 {
        return Err(error("invalid_chunk_nbt"));
    }
    Ok(n as u64)
}
fn skip_string(cursor: &mut Cursor<&[u8]>) -> io::Result<()> {
    let mut b = [0; 2];
    cursor.read_exact(&mut b)?;
    take(cursor, u64::from(u16::from_be_bytes(b)))
}
fn skip(cursor: &mut Cursor<&[u8]>, tag: u8, depth: u32, nodes: &mut u64) -> io::Result<()> {
    *nodes += 1;
    if depth > 64 || *nodes > 100_000 {
        return Err(error("chunk_nbt_structure_limit"));
    }
    match tag {
        1 => take(cursor, 1)?,
        2 => take(cursor, 2)?,
        3 | 5 => take(cursor, 4)?,
        4 | 6 => take(cursor, 8)?,
        7 | 11 | 12 => {
            let n = count(cursor)?;
            take(
                cursor,
                n * match tag {
                    11 => 4,
                    12 => 8,
                    _ => 1,
                },
            )?;
        }
        8 => skip_string(cursor)?,
        9 => {
            let tag = byte(cursor)?;
            let n = count(cursor)?;
            if n > 100_000 || !(1..=12).contains(&tag) && n != 0 {
                return Err(error("chunk_nbt_structure_limit"));
            }
            for _ in 0..n {
                skip(cursor, tag, depth + 1, nodes)?;
            }
        }
        10 => loop {
            let tag = byte(cursor)?;
            if tag == 0 {
                break;
            }
            skip_string(cursor)?;
            skip(cursor, tag, depth + 1, nodes)?;
        },
        _ => return Err(error("invalid_chunk_nbt")),
    }
    Ok(())
}
