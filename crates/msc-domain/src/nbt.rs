//! Minimal NBT reader: enough of Java's gzip-compressed big-endian and
//! Bedrock's headered little-endian `level.dat` format to recover a
//! world's seed, difficulty, gamemode, and cumulative day-time for
//! imported/legacy worlds.
//!
//! Ported from `WorldSlotManager.swift`'s private `NBTReader`/`NBTValue`
//! engine and its `extractSeedString`/`extractDifficultyString`/
//! `extractGamemodeString`/`extractDayTime`/`findInteger`/`nbtInteger`
//! helpers (P6.7's fixtures, `fixtures/world-nbt/`).
//!
//! Gzip decompression is pure, in-memory decompression (`flate2`), not
//! filesystem/process I/O — unlike source, which shells out to
//! `/usr/bin/gunzip` via a temp file, this crate decompresses the bytes it
//! is given directly, so it stays in this crate rather than
//! `msc-infrastructure`. Locating a `level.dat` member inside a zip
//! archive and reading its bytes off disk stay out of this crate; see
//! [`first_level_dat_path`] for the one part of that pipeline
//! (`firstLevelDatPath`'s *selection* rule) that is itself pure — it
//! operates on an already-obtained member listing, not a zip file.

use crate::identity::ServerType;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Endianness {
    Big,
    Little,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NbtValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    ByteArray(Vec<u8>),
    List(Vec<NbtValue>),
    /// Swift's `[String: NBTValue]` has no defined iteration order;
    /// `findInteger`'s recursive fallback (source line 1462-1484) can
    /// therefore itself be order-dependent on a compound with multiple
    /// same-named matches at different depths — a latent nondeterminism in
    /// the oracle, not introduced here. `BTreeMap` gives this port a
    /// deterministic (if arbitrarily different) iteration order instead of
    /// reproducing that nondeterminism.
    Compound(BTreeMap<String, NbtValue>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NbtParseError;

struct NbtReader<'a> {
    data: &'a [u8],
    endianness: Endianness,
    offset: usize,
}

impl<'a> NbtReader<'a> {
    fn new(data: &'a [u8], endianness: Endianness) -> Self {
        Self {
            data,
            endianness,
            offset: 0,
        }
    }

    /// `readRootCompound` (source line 1134-1140).
    fn read_root_compound(&mut self) -> Result<NbtValue, NbtParseError> {
        let tag = self.read_u8()?;
        if tag != 10 {
            return Err(NbtParseError);
        }
        let _name = self.read_string()?;
        self.read_payload(10)
    }

    /// `readPayload(type:)` (source line 1142-1203).
    fn read_payload(&mut self, tag: u8) -> Result<NbtValue, NbtParseError> {
        match tag {
            0 => Ok(NbtValue::Compound(BTreeMap::new())),
            1 => Ok(NbtValue::Byte(self.read_u8()? as i8)),
            2 => Ok(NbtValue::Short(self.read_i16()?)),
            3 => Ok(NbtValue::Int(self.read_i32()?)),
            4 => Ok(NbtValue::Long(self.read_i64()?)),
            5 => Ok(NbtValue::Float(f32::from_bits(self.read_i32()? as u32))),
            6 => Ok(NbtValue::Double(f64::from_bits(self.read_i64()? as u64))),
            7 => {
                // `readData(count:)` guards `count >= 0` (source line
                // 1245) — unlike list/intArray/longArray below, a negative
                // declared count is a hard failure here, not clamped.
                let count = self.read_i32()?;
                if count < 0 {
                    return Err(NbtParseError);
                }
                let bytes = self.read_bytes(count as usize)?;
                Ok(NbtValue::ByteArray(bytes.to_vec()))
            }
            8 => Ok(NbtValue::String(self.read_string()?)),
            9 => {
                let element_type = self.read_u8()?;
                if element_type > 12 {
                    return Err(NbtParseError);
                }
                let count = self.read_i32()?;
                let n = count.max(0) as usize;
                let mut values = Vec::with_capacity(n);
                for _ in 0..n {
                    values.push(self.read_payload(element_type)?);
                }
                Ok(NbtValue::List(values))
            }
            10 => {
                let mut map = BTreeMap::new();
                loop {
                    let raw_type = self.read_u8()?;
                    if raw_type == 0 {
                        break;
                    }
                    if raw_type > 12 {
                        return Err(NbtParseError);
                    }
                    let name = self.read_string()?;
                    let value = self.read_payload(raw_type)?;
                    map.insert(name, value);
                }
                Ok(NbtValue::Compound(map))
            }
            11 => {
                let count = self.read_i32()?;
                let n = count.max(0) as usize;
                let mut values = Vec::with_capacity(n);
                for _ in 0..n {
                    values.push(self.read_i32()?);
                }
                Ok(NbtValue::IntArray(values))
            }
            12 => {
                let count = self.read_i32()?;
                let n = count.max(0) as usize;
                let mut values = Vec::with_capacity(n);
                for _ in 0..n {
                    values.push(self.read_i64()?);
                }
                Ok(NbtValue::LongArray(values))
            }
            _ => Err(NbtParseError),
        }
    }

    fn read_u8(&mut self) -> Result<u8, NbtParseError> {
        let v = *self.data.get(self.offset).ok_or(NbtParseError)?;
        self.offset += 1;
        Ok(v)
    }

    fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], NbtParseError> {
        let end = self.offset.checked_add(count).ok_or(NbtParseError)?;
        if end > self.data.len() {
            return Err(NbtParseError);
        }
        let slice = &self.data[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn read_unsigned(&mut self, byte_count: usize) -> Result<u64, NbtParseError> {
        let chunk = self.read_bytes(byte_count)?;
        let mut acc: u64 = 0;
        match self.endianness {
            Endianness::Big => {
                for &b in chunk {
                    acc = (acc << 8) | b as u64;
                }
            }
            Endianness::Little => {
                for (i, &b) in chunk.iter().enumerate() {
                    acc |= (b as u64) << (i * 8);
                }
            }
        }
        Ok(acc)
    }

    fn read_i16(&mut self) -> Result<i16, NbtParseError> {
        Ok(self.read_unsigned(2)? as u16 as i16)
    }

    fn read_i32(&mut self) -> Result<i32, NbtParseError> {
        Ok(self.read_unsigned(4)? as u32 as i32)
    }

    fn read_i64(&mut self) -> Result<i64, NbtParseError> {
        Ok(self.read_unsigned(8)? as i64)
    }

    /// `readString` (source line 1253-1257): a negative declared length
    /// (from a `readInt16` whose high bit is set) clamps to 0 rather than
    /// failing, matching source's `max(0, length)`.
    fn read_string(&mut self) -> Result<String, NbtParseError> {
        let len = self.read_i16()?;
        let n = len.max(0) as usize;
        let bytes = self.read_bytes(n)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

pub(crate) fn parse_nbt_root(data: &[u8], endianness: Endianness) -> Option<NbtValue> {
    NbtReader::new(data, endianness).read_root_compound().ok()
}

/// `gunzipData` (source line 1352-1363), replaced with in-memory
/// decompression — see the module doc for why that's still a pure
/// function.
pub(crate) fn gunzip(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).ok()?;
    Some(out)
}

/// `ImportedWorldMetadata` (source line 1120-1127).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportedWorldMetadata {
    /// True only when the level.dat payload was successfully parsed. A false
    /// value keeps callers from presenting an absent field as a verified
    /// Minecraft default.
    pub parsed: bool,
    pub seed: Option<String>,
    pub difficulty: Option<String>,
    pub gamemode: Option<String>,
    pub world_type: Option<String>,
    pub flat_preset: Option<String>,
    pub structures: Option<bool>,
    pub biome_source: Option<String>,
    pub generator_options: Option<String>,
    pub bonus_chest: Option<bool>,
    pub data_packs: Vec<String>,
    pub hardcore: Option<bool>,
    pub commands: Option<bool>,
    pub gamerules: BTreeMap<String, String>,
    pub cheats: Option<bool>,
    pub experiments: BTreeMap<String, bool>,
    pub coordinates: Option<bool>,
    pub starting_map: Option<bool>,
    pub supported_toggles: BTreeMap<String, bool>,
    /// Cumulative world day-time in ticks.
    pub day_time: Option<i64>,
}

/// `importedWorldMetadata(fromLevelDatData:serverType:)` (source line
/// 1281-1304): parses a raw `level.dat` buffer and extracts what this
/// reader understands. Returns [`ImportedWorldMetadata::default`] — not an
/// error — for gzip failure, malformed NBT, or a non-compound root,
/// matching source's `try?`/guard-based silent-empty contract throughout.
pub fn imported_world_metadata_from_level_dat(
    raw_level_dat: &[u8],
    server_type: ServerType,
) -> ImportedWorldMetadata {
    let root = match server_type {
        ServerType::Java => match gunzip(raw_level_dat) {
            Some(nbt_data) => parse_nbt_root(&nbt_data, Endianness::Big),
            None => None,
        },
        ServerType::Bedrock => {
            // Source line 1289-1293: the 8-byte Bedrock header (version +
            // payload-length, both little-endian) is skipped only when
            // byte 8 looks like a valid compound tag start; otherwise the
            // whole buffer is handed to the parser unchanged.
            let payload = if raw_level_dat.len() > 8 && raw_level_dat[8] == 10 {
                &raw_level_dat[8..]
            } else {
                raw_level_dat
            };
            parse_nbt_root(payload, Endianness::Little)
        }
    };

    let Some(root) = root else {
        return ImportedWorldMetadata::default();
    };

    let prefer_java_paths = server_type == ServerType::Java;
    ImportedWorldMetadata {
        parsed: true,
        seed: extract_seed_string(&root, prefer_java_paths),
        difficulty: extract_difficulty_string(&root),
        gamemode: extract_gamemode_string(&root),
        world_type: extract_string(
            &root,
            if prefer_java_paths {
                &[
                    &["Data", "generatorName"],
                    &["Data", "WorldGenSettings", "type"],
                ]
            } else {
                &[&["WorldType"], &["Data", "WorldType"]]
            },
        ),
        flat_preset: extract_string(
            &root,
            &[
                &["Data", "generator", "options"],
                &["Data", "flat_world_generator_options"],
            ],
        ),
        structures: extract_bool(
            &root,
            &[
                &["Data", "MapFeatures"],
                &["Data", "WorldGenSettings", "generate_features"],
            ],
        ),
        biome_source: extract_string(&root, &[&["Data", "WorldGenSettings", "biome_source"]]),
        generator_options: extract_string(&root, &[&["Data", "WorldGenSettings", "generator"]]),
        bonus_chest: extract_bool(&root, &[&["Data", "bonusChest"], &["BonusChestEnabled"]]),
        data_packs: extract_string_list(&root, &[&["Data", "DataPacks"]]),
        hardcore: extract_bool(&root, &[&["Data", "hardcore"]]),
        commands: extract_bool(&root, &[&["Data", "allowCommands"], &["commandsEnabled"]]),
        gamerules: extract_string_map(&root, &[&["Data", "GameRules"], &["GameRules"]]),
        cheats: extract_bool(&root, &[&["CheatsEnabled"], &["Data", "CheatsEnabled"]]),
        experiments: extract_bool_map(&root, &[&["experiments"], &["Data", "experiments"]]),
        coordinates: extract_bool(&root, &[&["showCoordinates"], &["Data", "showCoordinates"]]),
        starting_map: extract_bool(&root, &[&["startingMap"], &["Data", "startingMap"]]),
        supported_toggles: extract_bool_map(&root, &[&["GameRules"], &["Data", "GameRules"]]),
        day_time: extract_day_time(&root, prefer_java_paths),
    }
}

fn value_at_path<'a>(value: &'a NbtValue, path: &[&str]) -> Option<&'a NbtValue> {
    path.iter().try_fold(value, |current, key| match current {
        NbtValue::Compound(map) => map.get(*key),
        _ => None,
    })
}

fn value_string(value: &NbtValue) -> Option<String> {
    match value {
        NbtValue::String(value) => Some(value.clone()),
        NbtValue::Byte(value) => Some(value.to_string()),
        NbtValue::Short(value) => Some(value.to_string()),
        NbtValue::Int(value) => Some(value.to_string()),
        NbtValue::Long(value) => Some(value.to_string()),
        NbtValue::Float(value) => Some(value.to_string()),
        NbtValue::Double(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_bool(value: &NbtValue) -> Option<bool> {
    match value {
        NbtValue::Byte(value) => Some(*value != 0),
        NbtValue::Short(value) => Some(*value != 0),
        NbtValue::Int(value) => Some(*value != 0),
        NbtValue::Long(value) => Some(*value != 0),
        NbtValue::String(value) => match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn extract_string(root: &NbtValue, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| value_at_path(root, path).and_then(value_string))
}

fn extract_bool(root: &NbtValue, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| value_at_path(root, path).and_then(value_bool))
}

fn extract_string_list(root: &NbtValue, paths: &[&[&str]]) -> Vec<String> {
    paths
        .iter()
        .find_map(|path| match value_at_path(root, path) {
            Some(NbtValue::List(values)) => {
                Some(values.iter().filter_map(value_string).collect::<Vec<_>>())
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn extract_string_map(root: &NbtValue, paths: &[&[&str]]) -> BTreeMap<String, String> {
    paths
        .iter()
        .find_map(|path| match value_at_path(root, path) {
            Some(NbtValue::Compound(values)) => Some(
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value_string(value).map(|value| (key.clone(), value))
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn extract_bool_map(root: &NbtValue, paths: &[&[&str]]) -> BTreeMap<String, bool> {
    paths
        .iter()
        .find_map(|path| match value_at_path(root, path) {
            Some(NbtValue::Compound(values)) => Some(
                values
                    .iter()
                    .filter_map(|(key, value)| value_bool(value).map(|value| (key.clone(), value)))
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// `nbtInteger(atPath:in:)` (source line 1442-1460).
fn nbt_integer_at_path(path: &[&str], value: &NbtValue) -> Option<i64> {
    if path.is_empty() {
        return match value {
            NbtValue::Long(n) => Some(*n),
            NbtValue::Int(n) => Some(*n as i64),
            NbtValue::Short(n) => Some(*n as i64),
            NbtValue::Byte(n) => Some(*n as i64),
            _ => None,
        };
    }
    match value {
        NbtValue::Compound(map) => {
            let next = map.get(path[0])?;
            nbt_integer_at_path(&path[1..], next)
        }
        _ => None,
    }
}

/// `findInteger(named:in:)` (source line 1462-1484): recurses into every
/// compound/list value, depth-first, until a key named exactly `key` whose
/// own value is integer-typed is found.
fn find_integer(key: &str, value: &NbtValue) -> Option<i64> {
    match value {
        NbtValue::Compound(map) => {
            if let Some(direct) = map.get(key)
                && let Some(matched) = nbt_integer_at_path(&[], direct)
            {
                return Some(matched);
            }
            map.values().find_map(|nested| find_integer(key, nested))
        }
        NbtValue::List(values) => values.iter().find_map(|nested| find_integer(key, nested)),
        _ => None,
    }
}

/// `extractSeedString(fromNBT:preferJavaPaths:)` (source line 1395-1406).
fn extract_seed_string(root: &NbtValue, prefer_java_paths: bool) -> Option<String> {
    if prefer_java_paths {
        if let Some(v) = nbt_integer_at_path(&["Data", "WorldGenSettings", "seed"], root) {
            return Some(v.to_string());
        }
        if let Some(v) = nbt_integer_at_path(&["Data", "RandomSeed"], root) {
            return Some(v.to_string());
        }
    }
    nbt_integer_at_path(&["RandomSeed"], root)
        .or_else(|| nbt_integer_at_path(&["WorldGenSettings", "seed"], root))
        .or_else(|| find_integer("RandomSeed", root))
        .or_else(|| find_integer("seed", root))
        .map(|v| v.to_string())
}

/// `extractDifficultyString(fromNBT:)` (source line 1408-1423).
fn extract_difficulty_string(root: &NbtValue) -> Option<String> {
    let raw = nbt_integer_at_path(&["Data", "Difficulty"], root)
        .or_else(|| nbt_integer_at_path(&["Difficulty"], root))
        .or_else(|| find_integer("Difficulty", root))?;
    match raw {
        0 => Some("peaceful".to_string()),
        1 => Some("easy".to_string()),
        2 => Some("normal".to_string()),
        3 => Some("hard".to_string()),
        _ => None,
    }
}

/// `extractGamemodeString(fromNBT:)` (source line 1425-1440).
fn extract_gamemode_string(root: &NbtValue) -> Option<String> {
    let raw = nbt_integer_at_path(&["Data", "GameType"], root)
        .or_else(|| nbt_integer_at_path(&["GameType"], root))
        .or_else(|| find_integer("GameType", root))?;
    match raw {
        0 => Some("survival".to_string()),
        1 => Some("creative".to_string()),
        2 => Some("adventure".to_string()),
        3 => Some("spectator".to_string()),
        _ => None,
    }
}

/// `extractDayTime(fromNBT:preferJavaPaths:)` (source line 1306-1316).
fn extract_day_time(root: &NbtValue, prefer_java_paths: bool) -> Option<i64> {
    if prefer_java_paths {
        if let Some(v) = nbt_integer_at_path(&["Data", "DayTime"], root) {
            return Some(v);
        }
        if let Some(v) = nbt_integer_at_path(&["Data", "Time"], root) {
            return Some(v);
        }
    }
    nbt_integer_at_path(&["DayTime"], root)
        .or_else(|| nbt_integer_at_path(&["Time"], root))
        .or_else(|| find_integer("DayTime", root))
        .or_else(|| find_integer("Time", root))
}

/// `firstLevelDatPath(inZIP:)`'s *selection* rule (source line 1333-1346):
/// given an already-obtained `unzip -Z -1` member listing, picks the FIRST
/// entry (in the zip's own internal order, not sorted) named `level.dat`
/// or ending in `/level.dat`, after dropping blank lines and
/// `__MACOSX/`-prefixed AppleDouble metadata entries. Obtaining that
/// listing is `msc-infrastructure`'s job; this is the pure decision over
/// it.
pub fn first_level_dat_path(listing: &[&str]) -> Option<String> {
    listing
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("__MACOSX/"))
        .find(|s| *s == "level.dat" || s.ends_with("/level.dat"))
        .map(|s| s.to_string())
}

/// The trim-and-empty-becomes-`None` half of `readAdjacentBackupMetadata`
/// (source line 1325-1327), applied to a sidecar's `worldSeed` field
/// value once the caller has already read and JSON-decoded the sidecar.
pub fn trimmed_sidecar_seed(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `importedWorldMetadata(fromZIP:serverType:)`'s merge step (source line
/// 1260-1269): a non-`None` sidecar seed always wins over a parsed
/// `level.dat` seed. Preserves a real source quirk exactly rather than
/// "fixing" it: this specific merge only ever touches `seed`/`difficulty`/
/// `gamemode` (lines 1265-1267) — a parsed `day_time` is silently dropped
/// here even though the same NBT parse computed one; `day_time` only
/// survives through [`imported_world_metadata_from_level_dat`]'s other,
/// non-sidecar caller (`importedWorldMetadata(fromFolder:serverType:)`,
/// source line 1271-1275).
pub fn merge_sidecar_metadata(
    sidecar_seed: Option<String>,
    parsed: ImportedWorldMetadata,
) -> ImportedWorldMetadata {
    ImportedWorldMetadata {
        parsed: parsed.parsed,
        seed: sidecar_seed.or(parsed.seed),
        difficulty: parsed.difficulty,
        gamemode: parsed.gamemode,
        world_type: parsed.world_type,
        flat_preset: parsed.flat_preset,
        structures: parsed.structures,
        biome_source: parsed.biome_source,
        generator_options: parsed.generator_options,
        bonus_chest: parsed.bonus_chest,
        data_packs: parsed.data_packs,
        hardcore: parsed.hardcore,
        commands: parsed.commands,
        gamerules: parsed.gamerules,
        cheats: parsed.cheats,
        experiments: parsed.experiments,
        coordinates: parsed.coordinates,
        starting_map: parsed.starting_map,
        supported_toggles: parsed.supported_toggles,
        day_time: None,
    }
}
