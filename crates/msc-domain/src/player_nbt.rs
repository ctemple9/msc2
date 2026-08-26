//! Player-specific extraction from Java Edition player `.dat` files.
//!
//! Gzip decompression and generic NBT decoding live in [`crate::nbt`]. This
//! module only contains the player fields and item-shape rules; callers that
//! need to read a file from disk do that outside the domain crate.

use crate::nbt::{self, Endianness};
use std::collections::BTreeMap;

pub use crate::nbt::NbtValue;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub game_mode: i32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub dimension: String,
    pub score: i32,
}

impl PlayerStats {
    pub fn game_mode_display(&self) -> String {
        match self.game_mode {
            0 => "Survival".to_owned(),
            1 => "Creative".to_owned(),
            2 => "Adventure".to_owned(),
            3 => "Spectator".to_owned(),
            mode => format!("Unknown ({mode})"),
        }
    }

    pub fn dimension_display(&self) -> String {
        match self.dimension.as_str() {
            "minecraft:overworld" => "Overworld".to_owned(),
            "minecraft:the_nether" => "Nether".to_owned(),
            "minecraft:the_end" => "The End".to_owned(),
            dimension => title_case_words(
                dimension
                    .rsplit(':')
                    .next()
                    .unwrap_or(dimension)
                    .replace('_', " ")
                    .as_str(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemEnchantment {
    pub id: String,
    pub level: i32,
}

impl ItemEnchantment {
    pub fn display_name(&self) -> String {
        let name = prettify_component(&self.id);
        let level = match self.level {
            1 => "I".to_owned(),
            2 => "II".to_owned(),
            3 => "III".to_owned(),
            4 => "IV".to_owned(),
            5 => "V".to_owned(),
            level => level.to_string(),
        };
        format!("{name} {level}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryItem {
    pub slot: i32,
    pub item_id: String,
    pub count: i32,
    pub enchantments: Vec<ItemEnchantment>,
    pub custom_name: Option<String>,
    pub damage: i32,
}

impl InventoryItem {
    pub fn display_name(&self) -> String {
        match self.custom_name.as_deref() {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => prettify_component(&self.item_id),
        }
    }

    pub fn icon_name(&self) -> String {
        self.item_id
            .rsplit(':')
            .next()
            .unwrap_or(&self.item_id)
            .to_owned()
    }
}

pub fn extract_stats(root: &NbtValue) -> Option<PlayerStats> {
    let NbtValue::Compound(dict) = root else {
        return None;
    };

    let health = match dict.get("Health") {
        Some(NbtValue::Float(value)) => *value,
        _ => 20.0,
    };

    let max_health = dict
        .get("Attributes")
        .and_then(|value| match value {
            NbtValue::List(attributes) => Some(attributes),
            _ => None,
        })
        .and_then(|attributes| {
            attributes.iter().find_map(|attribute| {
                let NbtValue::Compound(attribute) = attribute else {
                    return None;
                };
                let Some(NbtValue::String(name)) = attribute.get("Name") else {
                    return None;
                };
                if name != "minecraft:generic.max_health" && name != "generic.maxHealth" {
                    return None;
                }
                match attribute.get("Base") {
                    Some(NbtValue::Double(value)) => Some(*value as f32),
                    _ => None,
                }
            })
        })
        .unwrap_or(20.0);

    let food_level = int_field(dict, "FoodLevel").unwrap_or(20);
    let xp_level = int_field(dict, "XpLevel").unwrap_or(0);
    let xp_total = int_field(dict, "XpTotal").unwrap_or(0);
    let game_mode = int_field(dict, "playerGameType").unwrap_or(0);
    let score = int_field(dict, "Score").unwrap_or(0);

    let (pos_x, pos_y, pos_z) = match dict.get("Pos") {
        Some(NbtValue::List(position)) if position.len() >= 3 => (
            double_value(position.first()).unwrap_or(0.0),
            double_value(position.get(1)).unwrap_or(0.0),
            double_value(position.get(2)).unwrap_or(0.0),
        ),
        _ => (0.0, 0.0, 0.0),
    };

    let dimension = match dict.get("Dimension") {
        Some(NbtValue::String(value)) => value.clone(),
        Some(NbtValue::Int(-1)) => "minecraft:the_nether".to_owned(),
        Some(NbtValue::Int(1)) => "minecraft:the_end".to_owned(),
        Some(NbtValue::Int(_)) => "minecraft:overworld".to_owned(),
        _ => "minecraft:overworld".to_owned(),
    };

    Some(PlayerStats {
        health,
        max_health,
        food_level,
        xp_level,
        xp_total,
        game_mode,
        pos_x,
        pos_y,
        pos_z,
        dimension,
        score,
    })
}

pub fn extract_inventory(root: &NbtValue) -> Vec<InventoryItem> {
    let NbtValue::Compound(root) = root else {
        return Vec::new();
    };
    let Some(NbtValue::List(items)) = root.get("Inventory") else {
        return Vec::new();
    };

    items.iter().filter_map(parse_inventory_item).collect()
}

pub fn read_all(gzip_bytes: &[u8]) -> (Option<PlayerStats>, Vec<InventoryItem>) {
    let Some(nbt_bytes) = nbt::gunzip(gzip_bytes) else {
        return (None, Vec::new());
    };
    let Some(root) = nbt::parse_nbt_root(&nbt_bytes, Endianness::Big) else {
        return (None, Vec::new());
    };
    (extract_stats(&root), extract_inventory(&root))
}

fn int_field(dict: &BTreeMap<String, NbtValue>, key: &str) -> Option<i32> {
    match dict.get(key) {
        Some(NbtValue::Int(value)) => Some(*value),
        _ => None,
    }
}

fn double_value(value: Option<&NbtValue>) -> Option<f64> {
    match value {
        Some(NbtValue::Double(value)) => Some(*value),
        _ => None,
    }
}

fn parse_inventory_item(value: &NbtValue) -> Option<InventoryItem> {
    let NbtValue::Compound(entry) = value else {
        return None;
    };

    let slot = match entry.get("Slot") {
        Some(NbtValue::Byte(value)) => i32::from(*value),
        Some(NbtValue::Int(value)) => *value,
        _ => return None,
    };
    let NbtValue::String(item_id) = entry.get("id")? else {
        return None;
    };

    let count = match entry.get("Count") {
        Some(NbtValue::Byte(value)) => i32::from(*value).max(1),
        Some(NbtValue::Int(value)) => (*value).max(1),
        _ => match entry.get("count") {
            Some(NbtValue::Int(value)) => (*value).max(1),
            _ => 1,
        },
    };

    let (enchantments, custom_name, damage) =
        if let Some(NbtValue::Compound(tag)) = entry.get("tag") {
            parse_legacy_item_tag(tag)
        } else if let Some(NbtValue::Compound(components)) = entry.get("components") {
            parse_modern_item_components(components)
        } else {
            (Vec::new(), None, 0)
        };

    Some(InventoryItem {
        slot,
        item_id: item_id.clone(),
        count,
        enchantments,
        custom_name,
        damage,
    })
}

fn parse_legacy_item_tag(
    tag: &BTreeMap<String, NbtValue>,
) -> (Vec<ItemEnchantment>, Option<String>, i32) {
    let enchantment_values: &[NbtValue] = match tag.get("Enchantments") {
        Some(NbtValue::List(values)) => values,
        _ => match tag.get("StoredEnchantments") {
            Some(NbtValue::List(values)) => values,
            _ => &[],
        },
    };
    let enchantments = enchantment_values
        .iter()
        .filter_map(parse_legacy_enchantment)
        .collect();
    let custom_name = match tag.get("display") {
        Some(NbtValue::Compound(display)) => match display.get("Name") {
            Some(NbtValue::String(raw)) => parse_json_text_component(raw),
            _ => None,
        },
        _ => None,
    };
    let damage = match tag.get("Damage") {
        Some(NbtValue::Int(value)) => *value,
        _ => 0,
    };
    (enchantments, custom_name, damage)
}

fn parse_legacy_enchantment(value: &NbtValue) -> Option<ItemEnchantment> {
    let NbtValue::Compound(enchantment) = value else {
        return None;
    };
    let NbtValue::String(id) = enchantment.get("id")? else {
        return None;
    };
    let level = match enchantment.get("lvl") {
        Some(NbtValue::Short(value)) => i32::from(*value),
        Some(NbtValue::Int(value)) => *value,
        _ => 1,
    };
    Some(ItemEnchantment {
        id: id.clone(),
        level,
    })
}

fn parse_modern_item_components(
    components: &BTreeMap<String, NbtValue>,
) -> (Vec<ItemEnchantment>, Option<String>, i32) {
    let mut enchantments = Vec::new();
    for key in ["minecraft:enchantments", "minecraft:stored_enchantments"] {
        let Some(NbtValue::Compound(enchantment)) = components.get(key) else {
            continue;
        };
        let Some(NbtValue::Compound(levels)) = enchantment.get("levels") else {
            continue;
        };
        for (id, value) in levels {
            let level = match value {
                NbtValue::Int(value) => *value,
                _ => 1,
            };
            enchantments.push(ItemEnchantment {
                id: id.clone(),
                level,
            });
        }
    }

    let custom_name = match components.get("minecraft:custom_name") {
        Some(NbtValue::String(raw)) => parse_json_text_component(raw),
        _ => None,
    };
    let damage = match components.get("minecraft:damage") {
        Some(NbtValue::Int(value)) => *value,
        _ => 0,
    };
    (enchantments, custom_name, damage)
}

fn parse_json_text_component(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw)
        && let Some(text) = value.get("text").and_then(serde_json::Value::as_str)
    {
        return (!text.is_empty()).then(|| text.to_owned());
    }
    Some(raw.to_owned())
}

fn prettify_component(value: &str) -> String {
    let component = value.rsplit(':').next().unwrap_or(value).replace('_', " ");
    title_case_words(&component)
}

fn title_case_words(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut at_word_start = true;
    for character in value.chars() {
        if character.is_whitespace() {
            result.push(character);
            at_word_start = true;
        } else if at_word_start {
            result.extend(character.to_uppercase());
            at_word_start = false;
        } else {
            result.extend(character.to_lowercase());
        }
    }
    result
}
