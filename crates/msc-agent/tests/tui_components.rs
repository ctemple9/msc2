mod test_cli {
    pub use crate::cli::{CliError, CommonArgs};

    pub fn resolve_base_url(common: &CommonArgs) -> String {
        common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", common.host, common.port))
    }

    pub fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
        common
            .token
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/mod.rs"]
mod cli;

use cli::tui::components::{
    ComponentIntent, ComponentMutation, ComponentSurface, ComponentsState, catalog_path,
};
use crossterm::event::KeyCode;
use msc_api::dto::{
    AddonItemDto, AddonsResponseDto, CatalogItemDto, CatalogSearchResponseDto, VersionEntryDto,
    VersionsResponseDto,
};

fn state() -> ComponentsState {
    ComponentsState {
        versions: Some(VersionsResponseDto {
            supports_versions: true,
            flavor_name: "Paper".to_string(),
            current_version: Some("1.21.8".to_string()),
            is_bedrock: false,
            versions: vec![VersionEntryDto {
                id: "paper-1.21.8-10".to_string(),
                display_label: "Paper 1.21.8 build 10".to_string(),
                mc_version: "1.21.8".to_string(),
                loader_version: None,
                build_label: Some("build 10".to_string()),
                is_stable: true,
                is_latest: true,
            }],
            note: None,
            runtime: None,
        }),
        addons: Some(AddonsResponseDto {
            addons: vec![AddonItemDto {
                jar_stem: "geyser".to_string(),
                display_name: "Geyser".to_string(),
                is_enabled: true,
                project_id: Some("geysermc".to_string()),
                current_version: Some("2.6".to_string()),
                available_version: Some("2.7".to_string()),
                bucket: "updateAvailable".to_string(),
                icon_url: None,
            }],
            is_resolving: false,
            server_supports_addons: true,
            pack_managed: Some(true),
            pack_name: Some("Create pack".to_string()),
            note: Some("Provider unavailable; installed state is still authoritative.".to_string()),
        }),
        catalog: Some(CatalogSearchResponseDto {
            supports_addons: true,
            addon_kind: Some("plugin".to_string()),
            loader_name: Some("Paper".to_string()),
            game_version: Some("1.21.8".to_string()),
            results: vec![CatalogItemDto {
                project_id: "luckperms".to_string(),
                slug: "luckperms".to_string(),
                title: "LuckPerms".to_string(),
                description: "Permissions for the server.".to_string(),
                author: "Luck".to_string(),
                downloads: 10_000,
                icon_url: None,
                is_client_only: false,
                project_type: "plugin".to_string(),
            }],
            note: None,
        }),
        loaded: true,
        ..ComponentsState::default()
    }
}

#[test]
fn components_keep_version_addon_and_catalog_selection_separate() {
    let mut components = state();
    assert_eq!(components.surface, ComponentSurface::Versions);
    assert_eq!(components.item_count(), 1);

    components.select_surface(ComponentSurface::Addons);
    assert_eq!(components.selected_addon().unwrap().jar_stem, "geyser");
    components.handle_key(KeyCode::Enter);
    components.handle_key(KeyCode::Char('a'));
    assert!(components.action_menu_open);
    assert_eq!(
        components.handle_key(KeyCode::Char('u')),
        Some(ComponentIntent::Confirm(ComponentMutation::UpdateAddon {
            jar_stem: "geyser".to_string(),
        },))
    );

    components.select_surface(ComponentSurface::Catalog);
    components.handle_key(KeyCode::Enter);
    components.handle_key(KeyCode::Char('a'));
    assert_eq!(
        components.handle_key(KeyCode::Char('i')),
        Some(ComponentIntent::Confirm(
            ComponentMutation::InstallCatalog {
                project_id: "luckperms".to_string(),
                slug: "luckperms".to_string(),
                title: "LuckPerms".to_string(),
            },
        ))
    );
}

#[test]
fn provider_and_pack_managed_state_stays_visible_without_inventing_actions() {
    let components = state();
    let addons = components.addons.as_ref().unwrap();
    assert_eq!(addons.pack_managed, Some(true));
    assert_eq!(addons.pack_name.as_deref(), Some("Create pack"));
    assert!(
        addons
            .note
            .as_deref()
            .unwrap()
            .contains("Provider unavailable")
    );
    assert_eq!(
        catalog_path("sodium fabric"),
        "/v1/catalog/search?q=sodium%20fabric&offset=0"
    );
}
