//! Keyboard-first state for the Components section.
//!
//! The agent owns component discovery, capability decisions, and mutation
//! safety. This module only keeps the selected list/detail surface and turns
//! explicit terminal actions into requests against the existing API.

use std::path::PathBuf;

use crossterm::event::KeyCode;
use msc_api::dto::{
    AddonItemDto, AddonRemoveRequestDto, AddonRemoveResultDto, AddonUpdateResultDto,
    AddonsResponseDto, CatalogInstallRequestDto, CatalogInstallResultDto, CatalogSearchResponseDto,
    ComponentStatusDto, ComponentUpdateRequestDto, ComponentsStatusDto, ModpackImportRequestDto,
    ModpackImportResultDto, ModpackInspectionRequestDto, ModpackInspectionResultDto,
    ResourcePackActivateRequestDto, ResourcePackItemDto, ResourcePackMutationResultDto,
    ResourcePackRemoveRequestDto, ResourcePackSetUrlRequestDto, StagedUploadBeginRequestDto,
    StagedUploadCompleteResultDto, StagedUploadPurposeDto, VersionChangeRequestDto,
    VersionChangeResultDto, VersionEntryDto, VersionsResponseDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentSurface {
    Versions,
    Addons,
    System,
    Catalog,
    ResourcePacks,
    Modpacks,
}

impl ComponentSurface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Versions => "SERVER JAR / VERSION",
            Self::Addons => "INSTALLED ADD-ONS",
            Self::System => "SYSTEM COMPONENTS",
            Self::Catalog => "CATALOG",
            Self::ResourcePacks => "RESOURCE PACKS",
            Self::Modpacks => "MODPACKS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentInputKind {
    CatalogSearch,
    ModpackPath,
    ResourcePackUrl,
}

impl ComponentInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::CatalogSearch => "Catalog search",
            Self::ModpackPath => "inspect|import|replace | local .mrpack/.zip path",
            Self::ResourcePackUrl => "Resource-pack URL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentMutation {
    ChangeVersion {
        version_id: String,
    },
    UpdateAddon {
        jar_stem: String,
    },
    UpdateAllAddons,
    SetAddonEnabled {
        jar_stem: String,
        enabled: bool,
    },
    RemoveAddon {
        jar_stem: String,
    },
    UpdateSystem {
        component: String,
    },
    InstallCatalog {
        project_id: String,
        slug: String,
        title: String,
    },
    ActivateResourcePack {
        pack_id: String,
        require: bool,
    },
    ClearResourcePack,
    RemoveResourcePack {
        pack_id: String,
        pack_kind: String,
    },
    SetResourcePackUrl {
        url: String,
    },
    Modpack {
        path: PathBuf,
        action: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentIntent {
    Search(String),
    Confirm(ComponentMutation),
}

#[derive(Debug, Clone, Default)]
pub struct ComponentsState {
    pub versions: Option<VersionsResponseDto>,
    pub addons: Option<AddonsResponseDto>,
    pub system: Option<ComponentsStatusDto>,
    pub catalog: Option<CatalogSearchResponseDto>,
    pub resource_packs: Option<msc_api::dto::ResourcePacksResponseDto>,
    pub loaded: bool,
    pub error: Option<String>,
    pub surface: ComponentSurface,
    pub selected: usize,
    pub detail_open: bool,
    pub action_menu_open: bool,
    pub input: Option<(ComponentInputKind, String)>,
    pub status: Option<String>,
}

impl Default for ComponentSurface {
    fn default() -> Self {
        Self::Versions
    }
}

impl ComponentsState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let mut state = Self {
            versions: client.get_json("/v1/versions").await.ok(),
            addons: Some(client.get_json("/v1/addons").await?),
            system: client.get_json("/v1/components").await.ok(),
            resource_packs: client.get_json("/v1/resourcepacks").await.ok(),
            loaded: true,
            ..Self::default()
        };
        state.normalize_selection();
        Ok(state)
    }

    pub fn item_count(&self) -> usize {
        match self.surface {
            ComponentSurface::Versions => self.versions.as_ref().map_or(0, |v| v.versions.len()),
            ComponentSurface::Addons => self.addons.as_ref().map_or(0, |a| a.addons.len()),
            ComponentSurface::System => self.system.as_ref().map_or(0, |s| s.components.len()),
            ComponentSurface::Catalog => self.catalog.as_ref().map_or(0, |c| c.results.len()),
            ComponentSurface::ResourcePacks => self
                .resource_packs
                .as_ref()
                .map_or(0, |packs| packs.packs.len() + packs.geyser_packs.len()),
            ComponentSurface::Modpacks => 0,
        }
    }

    pub fn selected_version(&self) -> Option<&VersionEntryDto> {
        match self.surface {
            ComponentSurface::Versions => self.versions.as_ref()?.versions.get(self.selected),
            _ => None,
        }
    }

    pub fn selected_addon(&self) -> Option<&AddonItemDto> {
        match self.surface {
            ComponentSurface::Addons => self.addons.as_ref()?.addons.get(self.selected),
            _ => None,
        }
    }

    pub fn selected_system(&self) -> Option<&ComponentStatusDto> {
        match self.surface {
            ComponentSurface::System => self.system.as_ref()?.components.get(self.selected),
            _ => None,
        }
    }

    pub fn selected_catalog(&self) -> Option<&msc_api::dto::CatalogItemDto> {
        match self.surface {
            ComponentSurface::Catalog => self.catalog.as_ref()?.results.get(self.selected),
            _ => None,
        }
    }

    pub fn selected_resource_pack(&self) -> Option<&ResourcePackItemDto> {
        let packs = self.resource_packs.as_ref()?;
        match self.selected.checked_sub(packs.packs.len()) {
            None => packs.packs.get(self.selected),
            Some(index) => packs.geyser_packs.get(index),
        }
    }

    pub fn select_surface(&mut self, surface: ComponentSurface) {
        self.surface = surface;
        self.selected = 0;
        self.detail_open = false;
        self.action_menu_open = false;
    }

    pub async fn search(&mut self, client: &SharedClient, query: &str) -> Result<(), CliError> {
        let response: CatalogSearchResponseDto = client.get_json(&catalog_path(query)).await?;
        self.catalog = Some(response);
        self.surface = ComponentSurface::Catalog;
        self.selected = 0;
        self.detail_open = false;
        self.action_menu_open = false;
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<ComponentIntent> {
        if let Some((kind, value)) = self.input.take() {
            return self.handle_input(kind, value, key);
        }
        if self.detail_open {
            return self.handle_detail_key(key);
        }
        match key {
            KeyCode::Char('1') => self.select_surface(ComponentSurface::Versions),
            KeyCode::Char('2') => self.select_surface(ComponentSurface::Addons),
            KeyCode::Char('3') => self.select_surface(ComponentSurface::System),
            KeyCode::Char('4') => self.select_surface(ComponentSurface::Catalog),
            KeyCode::Char('5') => self.select_surface(ComponentSurface::ResourcePacks),
            KeyCode::Char('6') => self.select_surface(ComponentSurface::Modpacks),
            KeyCode::Char('/') if self.surface == ComponentSurface::Catalog => {
                self.input = Some((ComponentInputKind::CatalogSearch, String::new()));
            }
            KeyCode::Char('i') if self.surface == ComponentSurface::Modpacks => {
                self.input = Some((ComponentInputKind::ModpackPath, String::new()));
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Enter => self.detail_open = self.item_count() > 0,
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn handle_detail_key(&mut self, key: KeyCode) -> Option<ComponentIntent> {
        if self.action_menu_open {
            return self.handle_action_key(key);
        }
        match key {
            KeyCode::Esc => self.detail_open = false,
            KeyCode::Char('a') => self.action_menu_open = true,
            _ => {}
        }
        None
    }

    fn handle_action_key(&mut self, key: KeyCode) -> Option<ComponentIntent> {
        let mutation = match (self.surface, key) {
            (ComponentSurface::Versions, KeyCode::Char('v')) => {
                Some(ComponentMutation::ChangeVersion {
                    version_id: self.selected_version()?.id.clone(),
                })
            }
            (ComponentSurface::Addons, KeyCode::Char('u')) => {
                Some(ComponentMutation::UpdateAddon {
                    jar_stem: self.selected_addon()?.jar_stem.clone(),
                })
            }
            (ComponentSurface::Addons, KeyCode::Char('U')) => {
                Some(ComponentMutation::UpdateAllAddons)
            }
            (ComponentSurface::Addons, KeyCode::Char('e')) => {
                Some(ComponentMutation::SetAddonEnabled {
                    jar_stem: self.selected_addon()?.jar_stem.clone(),
                    enabled: true,
                })
            }
            (ComponentSurface::Addons, KeyCode::Char('d')) => {
                Some(ComponentMutation::SetAddonEnabled {
                    jar_stem: self.selected_addon()?.jar_stem.clone(),
                    enabled: false,
                })
            }
            (ComponentSurface::Addons, KeyCode::Char('x')) => {
                Some(ComponentMutation::RemoveAddon {
                    jar_stem: self.selected_addon()?.jar_stem.clone(),
                })
            }
            (ComponentSurface::Catalog, KeyCode::Char('i')) => {
                let item = self.selected_catalog()?;
                Some(ComponentMutation::InstallCatalog {
                    project_id: item.project_id.clone(),
                    slug: item.slug.clone(),
                    title: item.title.clone(),
                })
            }
            (ComponentSurface::System, KeyCode::Char('u')) => {
                Some(ComponentMutation::UpdateSystem {
                    component: self.selected_system()?.name.clone(),
                })
            }
            (ComponentSurface::ResourcePacks, KeyCode::Char('a')) => {
                let pack = self.selected_resource_pack()?;
                Some(ComponentMutation::ActivateResourcePack {
                    pack_id: pack.id.clone(),
                    require: false,
                })
            }
            (ComponentSurface::ResourcePacks, KeyCode::Char('c')) => {
                Some(ComponentMutation::ClearResourcePack)
            }
            (ComponentSurface::ResourcePacks, KeyCode::Char('x')) => {
                let pack = self.selected_resource_pack()?;
                Some(ComponentMutation::RemoveResourcePack {
                    pack_id: pack.id.clone(),
                    pack_kind: pack.pack_kind.clone(),
                })
            }
            (ComponentSurface::ResourcePacks, KeyCode::Char('u')) => {
                self.action_menu_open = false;
                self.input = Some((ComponentInputKind::ResourcePackUrl, String::new()));
                None
            }
            (ComponentSurface::Modpacks, KeyCode::Char('i')) => {
                self.action_menu_open = false;
                self.input = Some((ComponentInputKind::ModpackPath, String::new()));
                None
            }
            (_, KeyCode::Esc) => {
                self.action_menu_open = false;
                None
            }
            _ => None,
        }?;
        Some(ComponentIntent::Confirm(mutation))
    }

    fn handle_input(
        &mut self,
        kind: ComponentInputKind,
        mut value: String,
        key: KeyCode,
    ) -> Option<ComponentIntent> {
        match key {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.input = Some((kind, value));
            }
            KeyCode::Enter => return self.finish_input(kind, value),
            KeyCode::Char(character) => {
                value.push(character);
                self.input = Some((kind, value));
            }
            _ => self.input = Some((kind, value)),
        }
        None
    }

    fn finish_input(&mut self, kind: ComponentInputKind, value: String) -> Option<ComponentIntent> {
        let value = value.trim().to_string();
        match kind {
            ComponentInputKind::CatalogSearch if !value.is_empty() => {
                Some(ComponentIntent::Search(value))
            }
            ComponentInputKind::ResourcePackUrl if !value.is_empty() => Some(
                ComponentIntent::Confirm(ComponentMutation::SetResourcePackUrl { url: value }),
            ),
            ComponentInputKind::ModpackPath => {
                let (action, path) = value.split_once('|')?;
                let action = action.trim();
                if !matches!(action, "inspect" | "import" | "replace") || path.trim().is_empty() {
                    return None;
                }
                Some(ComponentIntent::Confirm(ComponentMutation::Modpack {
                    path: PathBuf::from(path.trim()),
                    action: action.to_string(),
                }))
            }
            _ => None,
        }
    }

    fn move_selection(&mut self, offset: isize) {
        let count = self.item_count();
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn normalize_selection(&mut self) {
        self.selected = self.selected.min(self.item_count().saturating_sub(1));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentMutationResult {
    pub message: String,
    pub operation_id: Option<String>,
}

pub async fn execute(
    client: &SharedClient,
    mutation: ComponentMutation,
) -> Result<ComponentMutationResult, CliError> {
    match mutation {
        ComponentMutation::ChangeVersion { version_id } => {
            let result: VersionChangeResultDto = client
                .post_json(
                    "/v1/components/version",
                    &VersionChangeRequestDto {
                        version_id,
                        loader_version: None,
                    },
                )
                .await?;
            mutation_result(result.success, result.message, result.operation_id)
        }
        ComponentMutation::UpdateAddon { jar_stem } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        jar_stem: Some(jar_stem),
                        ..Default::default()
                    },
                )
                .await?;
            mutation_result(true, result.result, result.operation_id)
        }
        ComponentMutation::UpdateAllAddons => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        update_all: Some(true),
                        ..Default::default()
                    },
                )
                .await?;
            mutation_result(true, result.result, result.operation_id)
        }
        ComponentMutation::SetAddonEnabled { jar_stem, enabled } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        jar_stem: Some(jar_stem),
                        enabled: Some(enabled),
                        ..Default::default()
                    },
                )
                .await?;
            mutation_result(true, result.result, result.operation_id)
        }
        ComponentMutation::RemoveAddon { jar_stem } => {
            let result: AddonRemoveResultDto = client
                .post_json("/v1/components/remove", &AddonRemoveRequestDto { jar_stem })
                .await?;
            mutation_result(result.success, result.message, None)
        }
        ComponentMutation::UpdateSystem { component } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: Some(component),
                        ..Default::default()
                    },
                )
                .await?;
            mutation_result(true, result.result, result.operation_id)
        }
        ComponentMutation::InstallCatalog {
            project_id,
            slug,
            title,
        } => {
            let result: CatalogInstallResultDto = client
                .post_json(
                    "/v1/components/install",
                    &CatalogInstallRequestDto {
                        project_id: Some(project_id),
                        slug: Some(slug),
                        title: Some(title),
                        staged_upload_id: None,
                        version_id: None,
                    },
                )
                .await?;
            mutation_result(result.success, result.message, result.operation_id)
        }
        ComponentMutation::ActivateResourcePack { pack_id, require } => {
            let result: ResourcePackMutationResultDto = client
                .post_json(
                    "/v1/resourcepacks/activate",
                    &ResourcePackActivateRequestDto {
                        pack_id: Some(pack_id),
                        require: Some(require),
                    },
                )
                .await?;
            mutation_result(result.success, result.message, None)
        }
        ComponentMutation::ClearResourcePack => {
            let result: ResourcePackMutationResultDto = client
                .post_json(
                    "/v1/resourcepacks/activate",
                    &ResourcePackActivateRequestDto {
                        pack_id: None,
                        require: Some(false),
                    },
                )
                .await?;
            mutation_result(result.success, result.message, None)
        }
        ComponentMutation::RemoveResourcePack { pack_id, pack_kind } => {
            let result: ResourcePackMutationResultDto = client
                .post_json(
                    "/v1/resourcepacks/remove",
                    &ResourcePackRemoveRequestDto { pack_id, pack_kind },
                )
                .await?;
            mutation_result(result.success, result.message, None)
        }
        ComponentMutation::SetResourcePackUrl { url } => {
            let result: ResourcePackMutationResultDto = client
                .post_json(
                    "/v1/resourcepacks/seturl",
                    &ResourcePackSetUrlRequestDto {
                        url,
                        sha1: None,
                        require: Some(false),
                    },
                )
                .await?;
            mutation_result(result.success, result.message, None)
        }
        ComponentMutation::Modpack { path, action } => {
            execute_modpack(client, &path, &action).await
        }
    }
}

async fn execute_modpack(
    client: &SharedClient,
    path: &PathBuf,
    action: &str,
) -> Result<ComponentMutationResult, CliError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|error| CliError::usage(format!("failed to read {}: {error}", path.display())))?;
    let staged: msc_api::dto::StagedUploadBeginResultDto = client
        .post_json(
            "/v1/staged-uploads",
            &StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::ModpackArchive,
                content_type: Some("application/zip".to_string()),
                operation_id: None,
                file_id: None,
            },
        )
        .await?;
    let _: StagedUploadCompleteResultDto = client
        .put_bytes(&staged.upload_path, "application/zip", bytes)
        .await?;

    if action == "inspect" {
        let result: ModpackInspectionResultDto = client
            .post_json(
                "/v1/modpacks/inspect",
                &ModpackInspectionRequestDto {
                    staged_upload_id: staged.staged_upload_id,
                },
            )
            .await?;
        return mutation_result(result.success, result.message, None);
    }

    let result: ModpackImportResultDto = client
        .post_json(
            "/v1/modpacks/import",
            &ModpackImportRequestDto {
                staged_upload_id: staged.staged_upload_id,
                action: action.to_string(),
            },
        )
        .await?;
    let message = if result.pending_manual_files.is_empty() {
        result.message
    } else {
        format!(
            "{} {} manual file(s) still need a staged upload.",
            result.message,
            result.pending_manual_files.len()
        )
    };
    mutation_result(result.success, message, Some(result.operation_id))
}

fn mutation_result(
    success: bool,
    message: String,
    operation_id: Option<String>,
) -> Result<ComponentMutationResult, CliError> {
    if success {
        Ok(ComponentMutationResult {
            message,
            operation_id,
        })
    } else {
        Err(CliError::usage(message))
    }
}

pub fn catalog_path(query: &str) -> String {
    format!(
        "/v1/catalog/search?q={}&offset=0",
        encode_uri_component(query)
    )
}

fn encode_uri_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            byte => format!("%{byte:02X}"),
        })
        .collect()
}
