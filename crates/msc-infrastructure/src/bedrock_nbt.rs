//! Bounded Bedrock player NBT decoding and world-settings updates.
//!
//! BDS stores player records as little-endian NBT values inside LevelDB.  The
//! parser is intentionally independent of the LevelDB reader so callers can
//! test or inspect one record without opening a live database.  Counts,
//! strings, nesting, and decompressed payload size are bounded before memory
//! is allocated.

use std::collections::BTreeMap;
use std::fmt;

pub const MAX_NBT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_NBT_DEPTH: usize = 64;
pub const MAX_NBT_ITEMS: usize = 100_000;
pub const MAX_NBT_STRING_BYTES: usize = 1024 * 1024;

/// Updates only the named world settings, preserving all unrelated NBT bytes.
/// Compounds are merged so changing experiments retains unknown flags.
pub fn update_level_dat(
    raw: &[u8],
    updates: &BTreeMap<String, msc_domain::nbt::NbtValue>,
) -> Result<Vec<u8>, NbtError> {
    if raw.len() > MAX_NBT_BYTES || raw.len() < 12 || raw[8] != 10 {
        return Err(NbtError::Corrupt("Bedrock level.dat root"));
    }
    let mut cursor = 9;
    let name_len = take_u16_le(raw, &mut cursor)? as usize;
    skip_bytes(raw, &mut cursor, name_len)?;
    let mut output = raw[..cursor].to_vec();
    output.extend(patch_compound(&raw[cursor..], updates)?);
    let size =
        u32::try_from(output.len() - 8).map_err(|_| NbtError::LimitExceeded("level.dat bytes"))?;
    if output.len() > MAX_NBT_BYTES {
        return Err(NbtError::LimitExceeded("level.dat bytes"));
    }
    output[4..8].copy_from_slice(&size.to_le_bytes());
    Ok(output)
}

pub fn set_level_dat_coordinates(raw: &[u8], enabled: bool) -> Result<Vec<u8>, NbtError> {
    update_level_dat(
        raw,
        &BTreeMap::from([(
            "showcoordinates".into(),
            msc_domain::nbt::NbtValue::Byte(i8::from(enabled)),
        )]),
    )
}

/// A new BDS world starts with world metadata and no generated chunks or players.
pub fn new_level_dat(
    settings: &BTreeMap<String, msc_domain::nbt::NbtValue>,
) -> Result<Vec<u8>, NbtError> {
    update_level_dat(&[10, 0, 0, 0, 4, 0, 0, 0, 10, 0, 0, 0], settings)
}

fn patch_compound(
    raw: &[u8],
    updates: &BTreeMap<String, msc_domain::nbt::NbtValue>,
) -> Result<Vec<u8>, NbtError> {
    use msc_domain::nbt::NbtValue;
    let mut output = Vec::new();
    let mut cursor = 0;
    let mut pending = updates.clone();
    let mut entries = 0;
    loop {
        let start = cursor;
        let tag = take_u8(raw, &mut cursor)?;
        if tag == 0 {
            break;
        }
        entries += 1;
        if entries > MAX_NBT_ITEMS {
            return Err(NbtError::LimitExceeded("level.dat tags"));
        }
        let length = take_u16_le(raw, &mut cursor)? as usize;
        let name_start = cursor;
        skip_bytes(raw, &mut cursor, length)?;
        let name = std::str::from_utf8(&raw[name_start..cursor])
            .map_err(|_| NbtError::Corrupt("level.dat tag name"))?;
        let payload_start = cursor;
        skip_nbt_payload(raw, &mut cursor, tag, 0)?;
        if let Some(value) = pending.remove(name) {
            if let NbtValue::Compound(children) = &value
                && tag == 10
            {
                output.extend_from_slice(&raw[start..payload_start]);
                output.extend(patch_compound(&raw[payload_start..cursor], children)?);
                continue;
            }
            encode_setting(&mut output, name, &value)?;
        } else {
            output.extend_from_slice(&raw[start..cursor]);
        }
    }
    for (name, value) in pending {
        encode_setting(&mut output, &name, &value)?;
    }
    output.push(0);
    Ok(output)
}

fn encode_setting(
    output: &mut Vec<u8>,
    name: &str,
    value: &msc_domain::nbt::NbtValue,
) -> Result<(), NbtError> {
    use msc_domain::nbt::NbtValue;
    let tag = match value {
        NbtValue::Byte(_) => 1,
        NbtValue::Int(_) => 3,
        NbtValue::Long(_) => 4,
        NbtValue::String(_) => 8,
        NbtValue::Compound(_) => 10,
        _ => return Err(NbtError::Unsupported("world setting type")),
    };
    output.push(tag);
    let length = u16::try_from(name.len()).map_err(|_| NbtError::LimitExceeded("tag name"))?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(name.as_bytes());
    match value {
        NbtValue::Byte(value) => output.push(*value as u8),
        NbtValue::Int(value) => output.extend_from_slice(&value.to_le_bytes()),
        NbtValue::Long(value) => output.extend_from_slice(&value.to_le_bytes()),
        NbtValue::String(value) => {
            let length =
                u16::try_from(value.len()).map_err(|_| NbtError::LimitExceeded("string"))?;
            output.extend_from_slice(&length.to_le_bytes());
            output.extend_from_slice(value.as_bytes());
        }
        NbtValue::Compound(children) => {
            for (name, value) in children {
                encode_setting(output, name, value)?;
            }
            output.push(0);
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn skip_nbt_payload(raw: &[u8], cursor: &mut usize, tag: u8, depth: usize) -> Result<(), NbtError> {
    if depth > 64 {
        return Err(NbtError::LimitExceeded("level.dat nesting"));
    }
    match tag {
        1 => skip_bytes(raw, cursor, 1),
        2 => skip_bytes(raw, cursor, 2),
        3 | 5 => skip_bytes(raw, cursor, 4),
        4 | 6 => skip_bytes(raw, cursor, 8),
        7 => {
            let count = take_i32_le(raw, cursor)?;
            skip_counted_bytes(raw, cursor, count, 1)
        }
        8 => {
            let count = take_u16_le(raw, cursor)? as usize;
            if count > MAX_NBT_STRING_BYTES {
                return Err(NbtError::LimitExceeded("level.dat string bytes"));
            }
            skip_bytes(raw, cursor, count)
        }
        9 => {
            let element_type = take_u8(raw, cursor)?;
            if element_type > 12 {
                return Err(NbtError::Unsupported("level.dat list tag type"));
            }
            let count = take_i32_le(raw, cursor)?;
            let count = checked_count(count)?;
            if count > 100_000 {
                return Err(NbtError::LimitExceeded("level.dat list items"));
            }
            for _ in 0..count {
                skip_nbt_payload(raw, cursor, element_type, depth + 1)?;
            }
            Ok(())
        }
        10 => {
            let mut count = 0;
            loop {
                let child_type = take_u8(raw, cursor)?;
                if child_type == 0 {
                    return Ok(());
                }
                count += 1;
                if count > 100_000 {
                    return Err(NbtError::LimitExceeded("level.dat compound tags"));
                }
                let name_len = take_u16_le(raw, cursor)? as usize;
                skip_bytes(raw, cursor, name_len)?;
                skip_nbt_payload(raw, cursor, child_type, depth + 1)?;
            }
        }
        11 => {
            let count = take_i32_le(raw, cursor)?;
            skip_counted_bytes(raw, cursor, count, 4)
        }
        12 => {
            let count = take_i32_le(raw, cursor)?;
            skip_counted_bytes(raw, cursor, count, 8)
        }
        _ => Err(NbtError::Unsupported("level.dat tag type")),
    }
}

fn checked_count(count: i32) -> Result<usize, NbtError> {
    usize::try_from(count).map_err(|_| NbtError::Corrupt("negative level.dat count"))
}

fn skip_counted_bytes(
    raw: &[u8],
    cursor: &mut usize,
    count: i32,
    item_size: usize,
) -> Result<(), NbtError> {
    let count = checked_count(count)?;
    if count > MAX_NBT_ITEMS {
        return Err(NbtError::LimitExceeded("level.dat array items"));
    }
    let bytes = count
        .checked_mul(item_size)
        .ok_or(NbtError::Corrupt("level.dat array size"))?;
    skip_bytes(raw, cursor, bytes)
}

fn take_u8(raw: &[u8], cursor: &mut usize) -> Result<u8, NbtError> {
    let byte = *raw
        .get(*cursor)
        .ok_or(NbtError::Corrupt("truncated level.dat"))?;
    *cursor += 1;
    Ok(byte)
}

fn take_u16_le(raw: &[u8], cursor: &mut usize) -> Result<u16, NbtError> {
    let end = cursor
        .checked_add(2)
        .ok_or(NbtError::Corrupt("level.dat offset"))?;
    let bytes = raw
        .get(*cursor..end)
        .ok_or(NbtError::Corrupt("truncated level.dat"))?;
    *cursor = end;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn take_i32_le(raw: &[u8], cursor: &mut usize) -> Result<i32, NbtError> {
    let end = cursor
        .checked_add(4)
        .ok_or(NbtError::Corrupt("level.dat offset"))?;
    let bytes = raw
        .get(*cursor..end)
        .ok_or(NbtError::Corrupt("truncated level.dat"))?;
    *cursor = end;
    Ok(i32::from_le_bytes(bytes.try_into().unwrap()))
}

fn skip_bytes(raw: &[u8], cursor: &mut usize, count: usize) -> Result<(), NbtError> {
    let end = cursor
        .checked_add(count)
        .ok_or(NbtError::Corrupt("level.dat offset"))?;
    if end > raw.len() {
        return Err(NbtError::Corrupt("truncated level.dat"));
    }
    *cursor = end;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NbtError {
    Unavailable,
    Corrupt(&'static str),
    Unsupported(&'static str),
    LimitExceeded(&'static str),
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => write!(f, "player NBT is unavailable"),
            Self::Corrupt(reason) => write!(f, "corrupt player NBT: {reason}"),
            Self::Unsupported(reason) => write!(f, "unsupported player NBT: {reason}"),
            Self::LimitExceeded(reason) => write!(f, "player NBT limit exceeded: {reason}"),
        }
    }
}

impl std::error::Error for NbtError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub game_mode: i32,
    pub position: [f64; 3],
    pub dimension: String,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEnchantment {
    pub id: String,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub slot: i32,
    pub item_id: String,
    pub count: i32,
    pub damage: i32,
    pub enchantments: Vec<ItemEnchantment>,
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockPlayerNbt {
    pub stats: Option<PlayerStats>,
    pub inventory: Vec<InventoryItem>,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Value>),
    Compound(BTreeMap<String, Value>),
}

struct Reader<'a> {
    data: &'a [u8],
    cursor: usize,
    depth: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Result<Self, NbtError> {
        if data.len() > MAX_NBT_BYTES {
            return Err(NbtError::LimitExceeded("record bytes"));
        }
        Ok(Self {
            data,
            cursor: 0,
            depth: 0,
        })
    }

    fn read_root(&mut self) -> Result<Value, NbtError> {
        let tag = self.u8()?;
        if tag != 10 {
            return Err(NbtError::Corrupt("root is not a compound"));
        }
        self.string()?;
        self.payload(tag)
    }

    fn payload(&mut self, tag: u8) -> Result<Value, NbtError> {
        if self.depth >= MAX_NBT_DEPTH {
            return Err(NbtError::LimitExceeded("nesting depth"));
        }
        self.depth += 1;
        let result = match tag {
            0 => Ok(Value::Compound(BTreeMap::new())),
            1 => Ok(Value::Byte(self.u8()? as i8)),
            2 => Ok(Value::Short(self.i16()?)),
            3 => Ok(Value::Int(self.i32()?)),
            4 => Ok(Value::Long(self.i64()?)),
            5 => Ok(Value::Float(f32::from_bits(self.i32()? as u32))),
            6 => Ok(Value::Double(f64::from_bits(self.i64()? as u64))),
            7 => {
                let count = self.i32()?;
                if count < 0 {
                    return Err(NbtError::Corrupt("negative byte-array length"));
                }
                self.bytes(count as usize)?;
                Ok(Value::String(String::new()))
            }
            8 => Ok(Value::String(self.string()?)),
            9 => {
                let element_tag = self.u8()?;
                if element_tag > 12 {
                    return Err(NbtError::Corrupt("unknown list element tag"));
                }
                let count = self.i32()?;
                if count < 0 {
                    return Err(NbtError::Corrupt("negative list length"));
                }
                let count = count as usize;
                if count > MAX_NBT_ITEMS {
                    return Err(NbtError::LimitExceeded("list items"));
                }
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.payload(element_tag)?);
                }
                Ok(Value::List(values))
            }
            10 => {
                let mut values = BTreeMap::new();
                loop {
                    let child_tag = self.u8()?;
                    if child_tag == 0 {
                        break;
                    }
                    if child_tag > 12 {
                        return Err(NbtError::Unsupported("unknown tag"));
                    }
                    let name = self.string()?;
                    values.insert(name, self.payload(child_tag)?);
                    if values.len() > MAX_NBT_ITEMS {
                        return Err(NbtError::LimitExceeded("compound items"));
                    }
                }
                Ok(Value::Compound(values))
            }
            11 => {
                let count = self.i32()?;
                if count < 0 {
                    return Err(NbtError::Corrupt("negative int-array length"));
                }
                let count = count as usize;
                if count > MAX_NBT_ITEMS {
                    return Err(NbtError::LimitExceeded("int-array items"));
                }
                for _ in 0..count {
                    self.i32()?;
                }
                Ok(Value::String(String::new()))
            }
            12 => {
                let count = self.i32()?;
                if count < 0 {
                    return Err(NbtError::Corrupt("negative long-array length"));
                }
                let count = count as usize;
                if count > MAX_NBT_ITEMS {
                    return Err(NbtError::LimitExceeded("long-array items"));
                }
                for _ in 0..count {
                    self.i64()?;
                }
                Ok(Value::String(String::new()))
            }
            _ => Err(NbtError::Unsupported("unknown tag")),
        };
        self.depth -= 1;
        result
    }

    fn bytes(&mut self, count: usize) -> Result<&'a [u8], NbtError> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(NbtError::Corrupt("offset overflow"))?;
        if end > self.data.len() {
            return Err(NbtError::Corrupt("truncated payload"));
        }
        let result = &self.data[self.cursor..end];
        self.cursor = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, NbtError> {
        Ok(*self
            .bytes(1)?
            .first()
            .ok_or(NbtError::Corrupt("truncated byte"))?)
    }

    fn i16(&mut self) -> Result<i16, NbtError> {
        Ok(i16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, NbtError> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, NbtError> {
        Ok(i64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }

    fn string(&mut self) -> Result<String, NbtError> {
        let length = self.i16()?;
        if length < 0 {
            return Err(NbtError::Corrupt("negative string length"));
        }
        let length = length as usize;
        if length > MAX_NBT_STRING_BYTES {
            return Err(NbtError::LimitExceeded("string bytes"));
        }
        let bytes = self.bytes(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| NbtError::Corrupt("invalid UTF-8"))
    }
}

pub fn read_player_nbt(data: &[u8]) -> Result<BedrockPlayerNbt, NbtError> {
    let mut reader = Reader::new(data)?;
    let root = reader.read_root()?;
    Ok(BedrockPlayerNbt {
        stats: extract_stats(&root),
        inventory: extract_inventory(&root),
    })
}

fn compound(value: &Value) -> Option<&BTreeMap<String, Value>> {
    match value {
        Value::Compound(value) => Some(value),
        _ => None,
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Byte(value) => Some(f64::from(*value)),
        Value::Short(value) => Some(f64::from(*value)),
        Value::Int(value) => Some(f64::from(*value)),
        Value::Long(value) => Some(*value as f64),
        Value::Float(value) => Some(f64::from(*value)),
        Value::Double(value) => Some(*value),
        _ => None,
    }
}

fn int(value: Option<&Value>) -> Option<i32> {
    number(value).map(|value| value as i32)
}

fn extract_stats(root: &Value) -> Option<PlayerStats> {
    let root = compound(root)?;
    let health = number(root.get("Health")).unwrap_or(20.0) as f32;
    let mut max_health = 20.0;
    if let Some(Value::List(attributes)) = root.get("Attributes") {
        for attribute in attributes {
            let Some(attribute) = compound(attribute) else {
                continue;
            };
            let Some(Value::String(name)) = attribute.get("Name") else {
                continue;
            };
            if matches!(
                name.as_str(),
                "minecraft:health" | "minecraft:generic.max_health" | "generic.maxHealth"
            ) && let Some(value) = number(attribute.get("Base"))
            {
                max_health = value as f32;
            }
        }
    }
    let xp_level = int(root.get("PlayerLevel")).unwrap_or(0);
    let base = if xp_level < 17 {
        xp_level * xp_level + 6 * xp_level
    } else if xp_level < 32 {
        (2.5 * f64::from(xp_level * xp_level)) as i32 - 40 * xp_level + 360
    } else {
        (4.5 * f64::from(xp_level * xp_level)) as i32 - 162 * xp_level + 2220
    };
    let xp_per_level = if xp_level < 16 {
        2 * xp_level + 7
    } else if xp_level < 31 {
        5 * xp_level - 38
    } else {
        9 * xp_level - 158
    };
    let progress = number(root.get("PlayerLevelProgress")).unwrap_or(0.0);
    let xp_total = base + (f64::from(xp_per_level) * progress) as i32;
    let mut position = [0.0; 3];
    if let Some(Value::List(values)) = root.get("Pos")
        && values.len() >= 3
    {
        for (index, value) in values.iter().take(3).enumerate() {
            if let Some(number) = number(Some(value)) {
                position[index] = number;
            }
        }
    }
    let dimension = match root.get("DimensionId") {
        Some(value) => match int(Some(value)) {
            Some(1) => "minecraft:the_nether".to_owned(),
            Some(2) => "minecraft:the_end".to_owned(),
            _ => "minecraft:overworld".to_owned(),
        },
        None => match root.get("Dimension") {
            Some(Value::String(value)) => value.clone(),
            _ => "minecraft:overworld".to_owned(),
        },
    };
    Some(PlayerStats {
        health,
        max_health,
        food_level: int(root.get("FoodLevel")).unwrap_or(20),
        xp_level,
        xp_total,
        game_mode: int(root.get("playerGameType")).unwrap_or(0),
        position,
        dimension,
        score: int(root.get("Score")).unwrap_or(0),
    })
}

fn extract_inventory(root: &Value) -> Vec<InventoryItem> {
    let Some(root) = compound(root) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    if let Some(Value::List(inventory)) = root.get("Inventory") {
        items.extend(inventory.iter().filter_map(|item| parse_item(item, None)));
    }
    let armor_slots = [103, 102, 101, 100];
    if let Some(Value::List(armor)) = root.get("Armor") {
        for (index, item) in armor.iter().take(4).enumerate() {
            if let Some(item) = parse_item(item, Some(armor_slots[index])) {
                items.push(item);
            }
        }
    }
    if let Some(Value::List(offhand)) = root.get("Offhand")
        && let Some(item) = offhand
            .first()
            .and_then(|item| parse_item(item, Some(-106)))
    {
        items.push(item);
    }
    items
}

fn parse_item(value: &Value, forced_slot: Option<i32>) -> Option<InventoryItem> {
    let entry = compound(value)?;
    let Value::String(item_id) = entry.get("Name")? else {
        return None;
    };
    if item_id.is_empty() || item_id == "minecraft:air" {
        return None;
    }
    let slot = forced_slot.or_else(|| int(entry.get("Slot")))?;
    let count = int(entry.get("Count")).unwrap_or(1).max(1);
    let damage = int(entry.get("Damage")).unwrap_or(0);
    let mut enchantments = Vec::new();
    let mut custom_name = None;
    if let Some(tag) = entry.get("tag").and_then(compound) {
        let key = if tag.contains_key("ench") {
            "ench"
        } else {
            "StoredEnchantments"
        };
        if let Some(Value::List(list)) = tag.get(key) {
            for enchantment in list {
                let Some(enchantment) = compound(enchantment) else {
                    continue;
                };
                let Some(id) = int(enchantment.get("id")) else {
                    continue;
                };
                enchantments.push(ItemEnchantment {
                    id: enchantment_id(id),
                    level: int(enchantment.get("lvl")).unwrap_or(1),
                });
            }
        }
        if let Some(display) = tag.get("display").and_then(compound)
            && let Some(Value::String(raw)) = display.get("Name")
            && !raw.is_empty()
        {
            custom_name = serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|value| {
                    value
                        .get("text")
                        .and_then(|text| text.as_str())
                        .map(str::to_owned)
                })
                .or_else(|| Some(raw.clone()));
        }
    }
    Some(InventoryItem {
        slot,
        item_id: item_id.clone(),
        count,
        damage,
        enchantments,
        custom_name,
    })
}

fn enchantment_id(id: i32) -> String {
    const NAMES: [&str; 38] = [
        "protection",
        "fire_protection",
        "feather_falling",
        "blast_protection",
        "projectile_protection",
        "thorns",
        "respiration",
        "depth_strider",
        "aqua_affinity",
        "sharpness",
        "smite",
        "bane_of_arthropods",
        "knockback",
        "fire_aspect",
        "looting",
        "efficiency",
        "silk_touch",
        "unbreaking",
        "fortune",
        "power",
        "punch",
        "flame",
        "infinity",
        "luck_of_the_sea",
        "lure",
        "frost_walker",
        "mending",
        "binding_curse",
        "vanishing_curse",
        "impaling",
        "riptide",
        "loyalty",
        "channeling",
        "multishot",
        "piercing",
        "quick_charge",
        "soul_speed",
        "swift_sneak",
    ];
    NAMES
        .get(id as usize)
        .map(|name| format!("minecraft:{name}"))
        .unwrap_or_else(|| format!("minecraft:unknown_{id}"))
}
