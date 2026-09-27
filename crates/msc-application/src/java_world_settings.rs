//! Java world choices projected before generation and through console commands.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use msc_domain::world_profile::WorldProfile;
use msc_infrastructure::fs::FileSystem;
use serde_json::{Value, json};

const GENERATION_PACK: &str = "msc-world-generation";

fn valid_pack_id(value: &str) -> bool {
    if let Some(file) = value.strip_prefix("file/") {
        !file.is_empty()
            && !file.contains("..")
            && !file.contains(['/', '\\', ':', ',', '"'])
            && file.bytes().all(|byte| (32..=126).contains(&byte))
    } else {
        resource_id(value)
    }
}

pub fn resource_id(value: &str) -> bool {
    let value = value.strip_prefix("file/").unwrap_or(value);
    !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || "_-.:/".contains(ch))
        && value.matches(':').count() <= 1
        && !value.starts_with('/')
}

pub fn level_type(profile: &WorldProfile) -> String {
    let raw = profile
        .generation
        .world_type
        .as_deref()
        .unwrap_or("default")
        .replace("\\:", ":");
    match raw.as_str() {
        "default" | "normal" => "minecraft:normal".into(),
        "largebiomes" => "minecraft:large_biomes".into(),
        value if value.contains(':') => value.into(),
        value => format!("minecraft:{value}"),
    }
}

fn json_object(raw: &str, label: &str) -> io::Result<Value> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| io::Error::other(format!("{label} must be a JSON object: {error}")))?;
    if !value.is_object() {
        return Err(io::Error::other(format!("{label} must be a JSON object")));
    }
    Ok(value)
}

fn flat_settings(profile: &WorldProfile) -> io::Result<Value> {
    let preset = profile.generation.flat_preset.as_deref();
    let options = profile.generation.generator_options.as_deref();
    if preset.is_some() && options.is_some() {
        return Err(io::Error::other(
            "Use Flat Preset or Generator Options, not both",
        ));
    }
    let Some(raw) = preset.or(options) else {
        return Ok(json!({}));
    };
    let value = json_object(raw, "Flat generator settings")?;
    if let Some(layers) = value.get("layers") {
        let layers = layers
            .as_array()
            .filter(|layers| !layers.is_empty())
            .ok_or_else(|| io::Error::other("Flat layers must be a nonempty array"))?;
        let mut height = 0u64;
        for layer in layers {
            let count = layer
                .get("height")
                .and_then(Value::as_u64)
                .filter(|height| *height > 0)
                .ok_or_else(|| io::Error::other("Every flat layer needs a positive height"))?;
            let block = layer
                .get("block")
                .and_then(Value::as_str)
                .filter(|block| resource_id(block))
                .ok_or_else(|| io::Error::other("Every flat layer needs a block ID"))?;
            let _ = block;
            height = height.saturating_add(count);
        }
        if height > 384 {
            return Err(io::Error::other("Flat layers cannot exceed 384 blocks"));
        }
    }
    if value
        .get("biome")
        .is_some_and(|biome| !biome.as_str().is_some_and(resource_id))
    {
        return Err(io::Error::other("Flat biome must be a biome ID"));
    }
    Ok(value)
}

fn custom_generator(profile: &WorldProfile) -> io::Result<Option<Value>> {
    let kind = level_type(profile);
    if kind == "minecraft:flat" {
        return Ok(None);
    }
    if profile.generation.flat_preset.is_some() {
        return Err(io::Error::other("Flat Preset requires the Flat world type"));
    }
    let biome = profile.generation.biome_source.as_deref();
    let options = profile.generation.generator_options.as_deref();
    if biome.is_none() && options.is_none() {
        return Ok(None);
    }
    if ![
        "minecraft:normal",
        "minecraft:amplified",
        "minecraft:large_biomes",
        "minecraft:single_biome_surface",
    ]
    .contains(&kind.as_str())
    {
        return Err(io::Error::other(
            "A custom world preset owns its generator; leave Biome Source and Generator Options empty",
        ));
    }
    let settings = match kind.as_str() {
        "minecraft:amplified" => "minecraft:amplified",
        "minecraft:large_biomes" => "minecraft:large_biomes",
        _ => "minecraft:overworld",
    };
    let mut generator = if let Some(raw) = options {
        let value = json_object(raw, "Generator Options")?;
        if value.get("type").and_then(Value::as_str).is_none() {
            return Err(io::Error::other(
                "Generator Options needs a generator type (for example minecraft:noise)",
            ));
        }
        value
    } else {
        json!({"type":"minecraft:noise", "settings":settings,
            "biome_source":{"type":"minecraft:multi_noise", "preset":"minecraft:overworld"}})
    };
    if let Some(raw) = biome {
        let source = if raw.starts_with('{') {
            let value = json_object(raw, "Biome Source")?;
            if value.get("type").and_then(Value::as_str).is_none() {
                return Err(io::Error::other("Biome Source JSON needs a type"));
            }
            value
        } else if resource_id(raw) {
            json!({"type":"minecraft:fixed", "biome":raw})
        } else {
            return Err(io::Error::other(
                "Biome Source must be a biome ID or JSON object",
            ));
        };
        if generator.get("type").and_then(Value::as_str) != Some("minecraft:noise") {
            return Err(io::Error::other(
                "Biome Source requires a minecraft:noise generator",
            ));
        }
        generator["biome_source"] = source;
    }
    Ok(Some(generator))
}

pub fn validate(profile: &WorldProfile) -> io::Result<()> {
    if profile
        .gameplay
        .difficulty
        .as_deref()
        .is_some_and(|value| !["peaceful", "easy", "normal", "hard"].contains(&value))
    {
        return Err(io::Error::other("Invalid Java difficulty"));
    }
    if profile
        .gameplay
        .default_game_mode
        .as_deref()
        .is_some_and(|value| !["survival", "creative", "adventure", "spectator"].contains(&value))
    {
        return Err(io::Error::other("Invalid Java default game mode"));
    }
    if !resource_id(&level_type(profile)) {
        return Err(io::Error::other("World type must be a preset ID"));
    }
    if level_type(profile) == "minecraft:flat" {
        flat_settings(profile)?;
        if profile.generation.biome_source.is_some() {
            return Err(io::Error::other(
                "For Flat worlds, choose the biome inside the flat settings JSON",
            ));
        }
    } else {
        custom_generator(profile)?;
    }
    for pack in &profile.generation.data_packs {
        if !valid_pack_id(pack) {
            return Err(io::Error::other(
                "Data pack names must be pack IDs such as vanilla or file/example.zip",
            ));
        }
    }
    for (name, value) in &profile.gameplay.gamerules {
        if name.is_empty()
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "_:.".contains(ch))
            || !["true", "false"].contains(&value.as_str()) && value.parse::<i32>().is_err()
        {
            return Err(io::Error::other(format!(
                "Gamerule {name} must have a valid name and a true, false, or integer value"
            )));
        }
    }
    Ok(())
}

/// The generated preset remains in the slot's world archive because saved
/// world generation can keep a registry reference to it after first startup.
pub fn generation_properties(
    fs: &dyn FileSystem,
    server_dir: &Path,
    level_name: &str,
    profile: &WorldProfile,
) -> io::Result<BTreeMap<String, String>> {
    validate(profile)?;
    let mut properties = BTreeMap::new();
    let mut packs = selected_data_packs(profile);
    let kind = level_type(profile);
    properties.insert("level-type".into(), kind.clone());
    properties.insert(
        "generator-settings".into(),
        if kind == "minecraft:flat" {
            flat_settings(profile)?.to_string()
        } else {
            "{}".into()
        },
    );
    if let Some(generator) = custom_generator(profile)? {
        let pack = server_dir
            .join(level_name)
            .join("datapacks")
            .join(GENERATION_PACK);
        let preset_dir = pack.join("data/msc/worldgen/world_preset");
        fs.create_dir_all(&preset_dir)?;
        let metadata = json!({"pack":{"description":"MSC world generation", "pack_format":15,
            "supported_formats":[15,2147483647], "min_format":15, "max_format":2147483647}});
        fs.write(&pack.join("pack.mcmeta"), metadata.to_string().as_bytes())?;
        let preset = json!({"dimensions":{
            "minecraft:overworld":{"type":"minecraft:overworld", "generator":generator},
            "minecraft:the_nether":{"type":"minecraft:the_nether", "generator":{
                "type":"minecraft:noise", "settings":"minecraft:nether",
                "biome_source":{"type":"minecraft:multi_noise", "preset":"minecraft:nether"}}},
            "minecraft:the_end":{"type":"minecraft:the_end", "generator":{
                "type":"minecraft:noise", "settings":"minecraft:end", "biome_source":{"type":"minecraft:the_end"}}}
        }});
        fs.write(
            &preset_dir.join("world_profile.json"),
            preset.to_string().as_bytes(),
        )?;
        properties.insert("level-type".into(), "msc:world_profile".into());
        packs.push(format!("file/{GENERATION_PACK}"));
    }
    validate_pack_files(fs, server_dir, level_name, &packs)?;
    properties.insert("initial-enabled-packs".into(), packs.join(","));
    properties.insert("initial-disabled-packs".into(), String::new());
    Ok(properties)
}

pub fn runtime_commands(
    profile: &WorldProfile,
    minecraft_version: Option<&str>,
) -> io::Result<Vec<String>> {
    validate(profile)?;
    let mut commands = Vec::new();
    if let Some(value) = &profile.gameplay.difficulty {
        commands.push(format!("difficulty {value}"));
    }
    if let Some(value) = &profile.gameplay.default_game_mode {
        commands.push(format!("defaultgamemode {value}"));
    }
    for (name, value) in &profile.gameplay.gamerules {
        if name.strip_prefix("minecraft:").unwrap_or(name) == "allowFireTicksAwayFromPlayer"
            && profile
                .gameplay
                .gamerules
                .get("doFireTick")
                .is_some_and(|value| value == "false")
        {
            continue;
        }
        let (mapped, mut value) = gamerule(name, value, minecraft_version);
        if mapped == "minecraft:fire_spread_radius_around_player"
            && name.strip_prefix("minecraft:").unwrap_or(name) == "doFireTick"
            && value != "0"
            && profile
                .gameplay
                .gamerules
                .get("allowFireTicksAwayFromPlayer")
                .is_some_and(|value| value == "true")
        {
            value = "-1".into();
        }
        commands.push(format!("gamerule {mapped} {value}"));
    }
    Ok(commands)
}

fn gamerule(name: &str, value: &str, version: Option<&str>) -> (String, String) {
    let modern = version.is_some_and(|version| {
        let mut parts = version
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0));
        let major = parts.next().unwrap_or(0);
        let minor = parts.next().unwrap_or(0);
        let patch = parts.next().unwrap_or(0);
        major > 1 || major == 1 && (minor > 21 || minor == 21 && patch >= 11)
    });
    if !modern || name.contains(':') && !name.starts_with("minecraft:") {
        return (name.into(), value.into());
    }
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    let (renamed, inverted) = match name {
        "announceAdvancements" => ("show_advancement_messages", false),
        "command_modification_block_limit" => ("max_block_modifications", false),
        "commandBlocksEnabled" => ("command_blocks_work", false),
        "disableElytraMovementCheck" => ("elytra_movement_check", true),
        "disablePlayerMovementCheck" => ("player_movement_check", true),
        "disableRaids" => ("raids", true),
        "doDaylightCycle" => ("advance_time", false),
        "doEntityDrops" => ("entity_drops", false),
        "doImmediateRespawn" => ("immediate_respawn", false),
        "doInsomnia" => ("spawn_phantoms", false),
        "doLimitedCrafting" => ("limited_crafting", false),
        "doMobLoot" => ("mob_drops", false),
        "doMobSpawning" => ("spawn_mobs", false),
        "doPatrolSpawning" => ("spawn_patrols", false),
        "doTileDrops" => ("block_drops", false),
        "doTraderSpawning" => ("spawn_wandering_traders", false),
        "doVinesSpread" => ("spread_vines", false),
        "doWardenSpawning" => ("spawn_wardens", false),
        "doWeatherCycle" => ("advance_weather", false),
        "maxCommandChainLength" => ("max_command_sequence_length", false),
        "maxCommandForkCount" => ("max_command_forks", false),
        "naturalRegeneration" => ("natural_health_regeneration", false),
        "snowAccumulationHeight" => ("max_snow_accumulation_height", false),
        "spawnRadius" => ("respawn_radius", false),
        "spawnerBlocksEnabled" => ("spawner_blocks_work", false),
        "doFireTick" => {
            return (
                "minecraft:fire_spread_radius_around_player".into(),
                if value == "false" { "0" } else { "128" }.into(),
            );
        }
        "allowFireTicksAwayFromPlayer" => {
            return (
                "minecraft:fire_spread_radius_around_player".into(),
                if value == "true" { "-1" } else { "128" }.into(),
            );
        }
        _ => ("", false),
    };
    let mapped = if renamed.is_empty() {
        name.chars()
            .enumerate()
            .fold(String::new(), |mut out, (index, ch)| {
                if ch.is_ascii_uppercase() && index > 0 {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                out
            })
    } else {
        renamed.into()
    };
    (
        format!("minecraft:{mapped}"),
        if inverted {
            (value == "false").to_string()
        } else {
            value.into()
        },
    )
}

pub fn selected_data_packs(profile: &WorldProfile) -> Vec<String> {
    let mut packs = profile.generation.data_packs.clone();
    for pack in profile
        .packs
        .iter()
        .filter(|pack| pack.enabled && pack.kind == "java_datapack")
    {
        for path in &pack.files {
            if let Some(relative) = path.split_once("datapacks/").map(|(_, path)| path)
                && let Some(name) = relative.split('/').next()
            {
                let id = format!("file/{name}");
                if !packs.contains(&id) {
                    packs.push(id);
                }
            }
        }
    }
    if !packs.iter().any(|pack| pack == "vanilla") {
        packs.insert(0, "vanilla".into());
    }
    packs
}

pub fn apply_existing_world(
    fs: &dyn FileSystem,
    server_dir: &Path,
    level_name: &str,
    profile: &WorldProfile,
) -> io::Result<()> {
    use msc_domain::nbt::{NbtValue, imported_world_metadata_from_level_dat};
    let path = server_dir.join(level_name).join("level.dat");
    let raw = match fs.read(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut fields = BTreeMap::new();
    if let Some(value) = profile.gameplay.difficulty.as_deref() {
        let difficulty = match value {
            "peaceful" => 0,
            "easy" => 1,
            "normal" => 2,
            "hard" => 3,
            _ => return Err(io::Error::other("Invalid Java difficulty")),
        };
        fields.insert("Difficulty".into(), NbtValue::Byte(difficulty));
        fields.insert("DifficultyLocked".into(), NbtValue::Byte(0));
    }
    if let Some(value) = profile.gameplay.default_game_mode.as_deref() {
        let mode = match value {
            "survival" => 0,
            "creative" => 1,
            "adventure" => 2,
            "spectator" => 3,
            _ => return Err(io::Error::other("Invalid Java game mode")),
        };
        fields.insert("GameType".into(), NbtValue::Int(mode));
    }
    let detected =
        imported_world_metadata_from_level_dat(&raw, msc_domain::identity::ServerType::Java);
    if !detected.parsed {
        return Err(io::Error::other(
            "The active Java world's level.dat could not be read",
        ));
    }
    let mut enabled = selected_data_packs(profile);
    // Loader-owned built-in packs are server requirements; never disable them.
    for pack in detected
        .data_packs
        .iter()
        .filter(|pack| !pack.starts_with("file/") || pack.as_str() == "file/msc-world-generation")
    {
        if !enabled.contains(pack) {
            enabled.push(pack.clone());
        }
    }
    // Custom generator registries must remain available when this saved world loads.
    if fs
        .stat(
            &server_dir
                .join(level_name)
                .join("datapacks")
                .join(GENERATION_PACK),
        )
        .is_ok()
        && !enabled
            .iter()
            .any(|name| name == "file/msc-world-generation")
    {
        enabled.push("file/msc-world-generation".into());
    }
    validate_pack_files(fs, server_dir, level_name, &enabled)?;
    let (_, mut excluded) = msc_domain::nbt::java_runtime_metadata(&raw);
    excluded.extend(detected.data_packs.iter().cloned());
    excluded.sort();
    excluded.dedup();
    let disabled: Vec<_> = excluded
        .iter()
        .filter(|pack| !enabled.contains(pack))
        .cloned()
        .map(NbtValue::String)
        .collect();
    fields.insert(
        "DataPacks".into(),
        NbtValue::Compound(BTreeMap::from([
            (
                "Enabled".into(),
                NbtValue::List(enabled.into_iter().map(NbtValue::String).collect()),
            ),
            ("Disabled".into(), NbtValue::List(disabled)),
        ])),
    );
    let bytes = msc_infrastructure::java_nbt::update_level_dat(&raw, &fields)?;
    msc_infrastructure::atomic_write::atomic_write(fs, &path, &bytes)
        .map_err(|error| io::Error::other(error.to_string()))?;
    if fs.read(&path)? != bytes {
        return Err(io::Error::other("Java world settings readback mismatch"));
    }
    Ok(())
}

fn validate_pack_files(
    fs: &dyn FileSystem,
    dir: &Path,
    level_name: &str,
    packs: &[String],
) -> io::Result<()> {
    for pack in packs {
        if !valid_pack_id(pack) {
            return Err(io::Error::other("Invalid data pack ID"));
        }
        if let Some(name) = pack.strip_prefix("file/")
            && fs
                .stat(&dir.join(level_name).join("datapacks").join(name))
                .is_err()
        {
            return Err(io::Error::other(format!(
                "Data pack {pack} is not installed in this world. Install it through Browse Packs first."
            )));
        }
    }
    Ok(())
}
