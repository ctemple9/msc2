//! Shared player-profile routes for the active Java or Bedrock server.
//!
//! The profile response is kept route-local because these DTOs are reserved
//! by the frozen contract but are not yet shared by another Rust client.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::UNIX_EPOCH;

use axum::Json;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use msc_application::bedrock_players::{self, BedrockPlayerRecord};
use msc_application::output_reducer::JavaOutputReducer;
use msc_application::player_profiles::{self, JavaPlayerProfile};
use msc_domain::identity::ServerType;
use msc_domain::player_nbt::{InventoryItem, ItemEnchantment, PlayerStats};
use msc_infrastructure::fs::StdFileSystem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

pub fn router(state: LifecycleRoutesState) -> Router {
    Router::new()
        .route("/players", get(players))
        .route("/players/profiles", get(profiles))
        .route("/players/hidden", post(mutate_hidden))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerProfilesResponseDto {
    profiles: Vec<PlayerProfileDto>,
    is_loading_stats: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerProfileDto {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    image_identifier: String,
    is_online: bool,
    is_op: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen: Option<String>,
    is_bedrock_player: bool,
    is_hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    skin_override_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_skin_file_override: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<PlayerStatsDto>,
    inventory: Vec<InventoryItemDto>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerStatsDto {
    health: f32,
    max_health: f32,
    food_level: i32,
    xp_level: i32,
    xp_total: i32,
    game_mode: i32,
    game_mode_display: String,
    pos_x: f64,
    pos_y: f64,
    pos_z: f64,
    dimension_display: String,
    score: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryItemDto {
    slot: i32,
    #[serde(rename = "itemID")]
    item_id: String,
    icon_name: String,
    count: i32,
    display_name: String,
    enchantments: Vec<ItemEnchantmentDto>,
    damage: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemEnchantmentDto {
    id: String,
    level: i32,
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenProfileMutationRequestDto {
    profile_id: Option<String>,
    hidden: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HiddenProfileMutationResultDto {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerDto {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayersResponse {
    players: Vec<PlayerDto>,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

pub async fn players(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return Json(PlayersResponse {
            players: Vec::new(),
            count: 0,
            note: Some("no_active_server".to_owned()),
        })
        .into_response();
    };
    if server.server_type != ServerType::Bedrock {
        return Json(PlayersResponse {
            players: Vec::new(),
            count: 0,
            note: Some("not_bedrock".to_owned()),
        })
        .into_response();
    }

    match discover_bedrock(&server.server_dir) {
        Ok(players) => Json(PlayersResponse {
            count: players.len(),
            players: players
                .into_iter()
                .map(|player| PlayerDto {
                    name: player.name,
                    uuid: Some(player.xuid),
                })
                .collect(),
            note: None,
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "player_data_unavailable",
            &error,
        ),
    }
}

pub async fn profiles(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return Json(PlayerProfilesResponseDto {
            profiles: Vec::new(),
            is_loading_stats: false,
        })
        .into_response();
    };

    match profiles_for_server(&server.server_dir, server.server_type) {
        Ok(profiles) => Json(PlayerProfilesResponseDto {
            profiles,
            is_loading_stats: false,
        })
        .into_response(),
        Err(error) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", &error),
    }
}

pub async fn mutate_hidden(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<HiddenProfileMutationRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) =
        require_permission(&credential, msc_api::dto::PermissionCategoryDto::Players)
    {
        return response;
    }
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "No server is currently active.",
        );
    };
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(profile_id) = body
        .profile_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return invalid_body("missing_profile_id", "profileId is required.");
    };
    let Some(hidden) = body.hidden else {
        return invalid_body("invalid_body", "hidden is required.");
    };

    let profiles = match profiles_for_server(&server.server_dir, server.server_type) {
        Ok(profiles) => profiles,
        Err(error) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", &error);
        }
    };
    if !profiles.iter().any(|profile| profile.id == profile_id) {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Player profile was not found.",
        );
    }

    let result = if let Some(xuid) = profile_id.strip_prefix("xuid_") {
        bedrock_players::set_hidden(&StdFileSystem, Path::new(&server.server_dir), xuid, hidden)
            .map_err(|error| error.to_string())
    } else {
        let Ok(uuid) = Uuid::parse_str(&profile_id) else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Player profile was not found.",
            );
        };
        if hidden {
            player_profiles::hide(&StdFileSystem, Path::new(&server.server_dir), &uuid)
        } else {
            player_profiles::unhide(&StdFileSystem, Path::new(&server.server_dir), &uuid)
        }
        .map_err(|error| error.to_string())
    };

    match result {
        Ok(()) => Json(HiddenProfileMutationResultDto {
            success: true,
            message: if hidden { "hidden" } else { "visible" }.to_owned(),
            profile_id: Some(profile_id),
            is_hidden: Some(hidden),
        })
        .into_response(),
        Err(error) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", &error),
    }
}

fn profiles_for_server(
    server_dir: &str,
    server_type: ServerType,
) -> Result<Vec<PlayerProfileDto>, String> {
    match server_type {
        ServerType::Java => {
            let reducer = JavaOutputReducer::new();
            player_profiles::load_player_profiles(&StdFileSystem, Path::new(server_dir), &reducer)
                .map(|profiles| profiles.into_iter().map(java_profile_to_dto).collect())
                .map_err(|error| error.to_string())
        }
        ServerType::Bedrock => discover_bedrock(server_dir).map(|players| {
            let hidden = bedrock_players::load_hidden(&StdFileSystem, Path::new(server_dir));
            players
                .into_iter()
                .map(|player| bedrock_profile_to_dto(player, &hidden))
                .collect()
        }),
    }
}

fn discover_bedrock(server_dir: &str) -> Result<Vec<BedrockPlayerRecord>, String> {
    let server_dir = Path::new(server_dir);
    let settings = msc_application::bedrock_settings::load(&StdFileSystem, server_dir);
    let cache = bedrock_players::load_name_cache(&StdFileSystem, server_dir);
    bedrock_players::discover_players(server_dir, &settings.model.level_name, &cache)
        .map_err(|error| error.to_string())
}

fn java_profile_to_dto(profile: JavaPlayerProfile) -> PlayerProfileDto {
    PlayerProfileDto {
        id: profile.uuid.to_string(),
        username: profile.username,
        image_identifier: profile.uuid.simple().to_string(),
        is_online: profile.is_online,
        is_op: profile.is_op,
        last_seen: profile
            .last_modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| super::system_time_to_iso8601(duration.as_secs())),
        is_bedrock_player: false,
        is_hidden: profile.is_hidden,
        skin_override_identifier: None,
        has_skin_file_override: None,
        stats: profile.stats.as_ref().map(player_stats_to_dto),
        inventory: profile
            .inventory
            .iter()
            .map(inventory_item_to_dto)
            .collect(),
    }
}

fn bedrock_profile_to_dto(
    profile: BedrockPlayerRecord,
    hidden: &BTreeSet<String>,
) -> PlayerProfileDto {
    let image_identifier = if profile.name.starts_with('.') {
        profile.name.clone()
    } else {
        format!(".{}", profile.name)
    };
    PlayerProfileDto {
        id: format!("xuid_{}", profile.xuid),
        username: Some(profile.name),
        image_identifier,
        is_online: false,
        is_op: false,
        last_seen: None,
        is_bedrock_player: true,
        is_hidden: hidden.contains(&profile.xuid),
        skin_override_identifier: None,
        has_skin_file_override: None,
        stats: None,
        inventory: Vec::new(),
    }
}

fn player_stats_to_dto(stats: &PlayerStats) -> PlayerStatsDto {
    PlayerStatsDto {
        health: stats.health,
        max_health: stats.max_health,
        food_level: stats.food_level,
        xp_level: stats.xp_level,
        xp_total: stats.xp_total,
        game_mode: stats.game_mode,
        game_mode_display: stats.game_mode_display(),
        pos_x: stats.pos_x,
        pos_y: stats.pos_y,
        pos_z: stats.pos_z,
        dimension_display: stats.dimension_display(),
        score: stats.score,
    }
}

fn inventory_item_to_dto(item: &InventoryItem) -> InventoryItemDto {
    InventoryItemDto {
        slot: item.slot,
        item_id: item.item_id.clone(),
        icon_name: item.icon_name(),
        count: item.count,
        display_name: item.display_name(),
        enchantments: item.enchantments.iter().map(enchantment_to_dto).collect(),
        damage: item.damage,
    }
}

fn enchantment_to_dto(enchantment: &ItemEnchantment) -> ItemEnchantmentDto {
    ItemEnchantmentDto {
        id: enchantment.id.clone(),
        level: enchantment.level,
        display_name: enchantment.display_name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn java_profile_mapping_preserves_contract_field_names_and_derived_values() {
        let profile = JavaPlayerProfile {
            uuid: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            username: Some("Alex".to_owned()),
            dat_file_path: std::path::PathBuf::from("/server/world/playerdata/player.dat"),
            last_modified: SystemTime::UNIX_EPOCH,
            is_online: true,
            is_op: true,
            is_hidden: false,
            stats: Some(PlayerStats {
                health: 18.0,
                max_health: 20.0,
                food_level: 17,
                xp_level: 4,
                xp_total: 42,
                game_mode: 0,
                pos_x: 1.5,
                pos_y: 64.0,
                pos_z: -2.0,
                dimension: "minecraft:the_nether".to_owned(),
                score: 3,
            }),
            inventory: vec![InventoryItem {
                slot: 0,
                item_id: "minecraft:diamond_sword".to_owned(),
                count: 1,
                enchantments: vec![ItemEnchantment {
                    id: "minecraft:sharpness".to_owned(),
                    level: 2,
                }],
                custom_name: None,
                damage: 7,
            }],
        };

        let json = serde_json::to_value(java_profile_to_dto(profile)).unwrap();
        assert_eq!(json["id"], "11111111-1111-4111-8111-111111111111");
        assert_eq!(json["imageIdentifier"], "11111111111141118111111111111111");
        assert_eq!(json["lastSeen"], "1970-01-01T00:00:00Z");
        assert_eq!(json["stats"]["dimensionDisplay"], "Nether");
        assert_eq!(json["inventory"][0]["itemID"], "minecraft:diamond_sword");
        assert_eq!(json["inventory"][0]["iconName"], "diamond_sword");
        assert_eq!(
            json["inventory"][0]["enchantments"][0]["displayName"],
            "Sharpness II"
        );
        assert!(json.get("skinOverrideIdentifier").is_none());
        assert!(json.get("hasSkinFileOverride").is_none());
    }

    #[test]
    fn bedrock_profile_mapping_uses_xuid_identity_and_always_empty_inventory() {
        let profile = bedrock_profile_to_dto(
            BedrockPlayerRecord {
                xuid: "2535416361514257".to_owned(),
                name: "Builder".to_owned(),
                has_stats: true,
                inventory_items: 4,
            },
            &BTreeSet::from(["2535416361514257".to_owned()]),
        );
        let json = serde_json::to_value(profile).unwrap();
        assert_eq!(json["id"], "xuid_2535416361514257");
        assert_eq!(json["imageIdentifier"], ".Builder");
        assert_eq!(json["isHidden"], true);
        assert_eq!(json["inventory"], serde_json::json!([]));
        assert!(json.get("stats").is_none());
    }
}
