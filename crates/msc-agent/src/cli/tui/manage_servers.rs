//! Keyboard-first Manage Servers state and the existing fleet API requests.
//!
//! The staged create/import drafts are deliberately explicit: a terminal user
//! sees the same important choices as the desktop flow, while the agent stays
//! responsible for validation, permissions, and durable server state.

use std::collections::HashMap;

use crossterm::event::KeyCode;
use msc_api::dto::{
    ActiveServerRequestDto, ServerCreateRequestDto, ServerCreateResultDto, ServerDeleteRequestDto,
    ServerDeleteResultDto, ServerDto, ServerEulaRequestDto, ServerEulaResultDto,
    ServerImportRequestDto, ServerImportResultDto, ServerRenameRequestDto, ServerRenameResultDto,
    SimpleResultDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageSurface {
    List,
    Detail,
    Create,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateStep {
    Name,
    Type,
    Flavor,
    Port,
    World,
    Eula,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportStep {
    Path,
    Type,
    Name,
    World,
    Eula,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageInputKind {
    CreateName,
    CreateType,
    CreateFlavor,
    CreatePort,
    CreateWorld,
    CreateEula,
    ImportPath,
    ImportType,
    ImportName,
    ImportWorld,
    ImportEula,
    Rename,
}

impl ManageInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::CreateName => "New server name",
            Self::CreateType => "Server type (java or bedrock)",
            Self::CreateFlavor => "Java flavor (paper, fabric, forge, or blank)",
            Self::CreatePort => "Game port (blank for agent default)",
            Self::CreateWorld => "World name (blank for default)",
            Self::CreateEula => "Accept EULA? (yes or no)",
            Self::ImportPath => "Existing server path or ZIP",
            Self::ImportType => "Import type (java or bedrock)",
            Self::ImportName => "Imported server name (blank keeps detected name)",
            Self::ImportWorld => "Active world name (blank for detected default)",
            Self::ImportEula => "Accept EULA? (yes or no)",
            Self::Rename => "New server name",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDraft {
    pub name: String,
    pub server_type: Option<String>,
    pub java_flavor: Option<String>,
    pub port: Option<i64>,
    pub world_name: Option<String>,
    pub accept_eula: bool,
}

impl Default for CreateDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            server_type: Some("java".to_string()),
            java_flavor: None,
            port: None,
            world_name: None,
            accept_eula: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDraft {
    pub source_path: String,
    pub import_kind: String,
    pub display_name: Option<String>,
    pub server_type: Option<String>,
    pub active_world_name: Option<String>,
    pub accept_eula: bool,
}

impl Default for ImportDraft {
    fn default() -> Self {
        Self {
            source_path: String::new(),
            import_kind: "auto".to_string(),
            display_name: None,
            server_type: Some("java".to_string()),
            active_world_name: None,
            accept_eula: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManageMutation {
    SetActive { server_id: String },
    Create(CreateDraft),
    Import(ImportDraft),
    Rename { server_id: String, name: String },
    AcceptEula { server_id: String },
    Delete { server_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManageIntent {
    OpenEditor { server_id: String },
    Confirm(ManageMutation),
}

#[derive(Debug, Clone, Default)]
pub struct ManageServersState {
    pub servers: Vec<ServerDto>,
    pub selected: usize,
    pub surface: Option<ManageSurface>,
    pub input: Option<(ManageInputKind, String)>,
    pub create_draft: CreateDraft,
    pub import_draft: ImportDraft,
    create_step: Option<CreateStep>,
    import_step: Option<ImportStep>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl ManageServersState {
    pub fn from_servers(servers: Vec<ServerDto>) -> Self {
        let mut state = Self {
            servers,
            loaded: true,
            ..Self::default()
        };
        state.normalize_selection();
        state
    }

    pub fn is_open(&self) -> bool {
        self.surface.is_some()
    }

    pub fn selected_server(&self) -> Option<&ServerDto> {
        self.servers.get(self.selected)
    }

    pub fn selected_server_id(&self) -> Option<&str> {
        self.selected_server().map(|server| server.id.as_str())
    }

    pub fn create_step_is_review(&self) -> bool {
        self.create_step == Some(CreateStep::Review)
    }

    pub fn import_step_is_review(&self) -> bool {
        self.import_step == Some(ImportStep::Review)
    }

    pub fn open(&mut self) {
        self.surface = Some(ManageSurface::List);
        self.input = None;
        self.status = None;
    }

    pub fn set_servers(&mut self, servers: Vec<ServerDto>) {
        self.servers = servers;
        self.normalize_selection();
        self.loaded = true;
        self.error = None;
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<ManageIntent> {
        if let Some((kind, value)) = self.input.take() {
            return self.handle_input(kind, value, key);
        }
        match self.surface? {
            ManageSurface::List => self.handle_list_key(key),
            ManageSurface::Detail => self.handle_detail_key(key),
            ManageSurface::Create => self.handle_create_key(key),
            ManageSurface::Import => self.handle_import_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyCode) -> Option<ManageIntent> {
        match key {
            KeyCode::Esc => self.surface = None,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Enter => self.surface = self.selected_server().map(|_| ManageSurface::Detail),
            KeyCode::Char('c') => self.begin_create(),
            KeyCode::Char('i') => self.begin_import(),
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn handle_detail_key(&mut self, key: KeyCode) -> Option<ManageIntent> {
        let server = self.selected_server()?.clone();
        match key {
            KeyCode::Esc => self.surface = Some(ManageSurface::List),
            KeyCode::Char('e') => {
                return Some(ManageIntent::OpenEditor {
                    server_id: server.id,
                });
            }
            KeyCode::Char('a') => {
                return Some(ManageIntent::Confirm(ManageMutation::SetActive {
                    server_id: server.id,
                }));
            }
            KeyCode::Char('n') => self.input = Some((ManageInputKind::Rename, String::new())),
            KeyCode::Char('u') => {
                return Some(ManageIntent::Confirm(ManageMutation::AcceptEula {
                    server_id: server.id,
                }));
            }
            KeyCode::Char('d') => {
                return Some(ManageIntent::Confirm(ManageMutation::Delete {
                    server_id: server.id,
                }));
            }
            _ => {}
        }
        None
    }

    fn handle_create_key(&mut self, key: KeyCode) -> Option<ManageIntent> {
        if key == KeyCode::Esc {
            self.surface = Some(ManageSurface::List);
            self.input = None;
        } else if self.create_step == Some(CreateStep::Review) && key == KeyCode::Enter {
            self.surface = Some(ManageSurface::List);
            return Some(ManageIntent::Confirm(ManageMutation::Create(
                self.create_draft.clone(),
            )));
        }
        None
    }

    fn handle_import_key(&mut self, key: KeyCode) -> Option<ManageIntent> {
        if key == KeyCode::Esc {
            self.surface = Some(ManageSurface::List);
            self.input = None;
        } else if self.import_step == Some(ImportStep::Review) && key == KeyCode::Enter {
            self.surface = Some(ManageSurface::List);
            return Some(ManageIntent::Confirm(ManageMutation::Import(
                self.import_draft.clone(),
            )));
        }
        None
    }

    fn handle_input(
        &mut self,
        kind: ManageInputKind,
        mut value: String,
        key: KeyCode,
    ) -> Option<ManageIntent> {
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

    fn finish_input(&mut self, kind: ManageInputKind, value: String) -> Option<ManageIntent> {
        let value = value.trim().to_string();
        match kind {
            ManageInputKind::CreateName if !value.is_empty() => {
                self.create_draft.name = value;
                self.advance_create(CreateStep::Type, ManageInputKind::CreateType);
            }
            ManageInputKind::CreateType => {
                let value = if value.is_empty() {
                    "java"
                } else {
                    value.as_str()
                };
                if !matches!(value, "java" | "bedrock") {
                    self.input = Some((kind, value.to_string()));
                    return None;
                }
                self.create_draft.server_type = Some(value.to_string());
                if value == "java" {
                    self.advance_create(CreateStep::Flavor, ManageInputKind::CreateFlavor);
                } else {
                    self.advance_create(CreateStep::Port, ManageInputKind::CreatePort);
                }
            }
            ManageInputKind::CreateFlavor => {
                self.create_draft.java_flavor = (!value.is_empty()).then_some(value);
                self.advance_create(CreateStep::Port, ManageInputKind::CreatePort);
            }
            ManageInputKind::CreatePort => {
                self.create_draft.port = parse_optional_i64(&value)?;
                self.advance_create(CreateStep::World, ManageInputKind::CreateWorld);
            }
            ManageInputKind::CreateWorld => {
                self.create_draft.world_name = (!value.is_empty()).then_some(value);
                self.advance_create(CreateStep::Eula, ManageInputKind::CreateEula);
            }
            ManageInputKind::CreateEula => {
                self.create_draft.accept_eula = parse_bool(&value)?;
                self.create_step = Some(CreateStep::Review);
                self.input = None;
            }
            ManageInputKind::ImportPath if !value.is_empty() => {
                self.import_draft.source_path = value;
                self.advance_import(ImportStep::Type, ManageInputKind::ImportType);
            }
            ManageInputKind::ImportType => {
                let value = if value.is_empty() {
                    "java"
                } else {
                    value.as_str()
                };
                if !matches!(value, "java" | "bedrock") {
                    self.input = Some((kind, value.to_string()));
                    return None;
                }
                self.import_draft.server_type = Some(value.to_string());
                self.advance_import(ImportStep::Name, ManageInputKind::ImportName);
            }
            ManageInputKind::ImportName => {
                self.import_draft.display_name = (!value.is_empty()).then_some(value);
                self.advance_import(ImportStep::World, ManageInputKind::ImportWorld);
            }
            ManageInputKind::ImportWorld => {
                self.import_draft.active_world_name = (!value.is_empty()).then_some(value);
                self.advance_import(ImportStep::Eula, ManageInputKind::ImportEula);
            }
            ManageInputKind::ImportEula => {
                self.import_draft.accept_eula = parse_bool(&value)?;
                self.import_step = Some(ImportStep::Review);
                self.input = None;
            }
            ManageInputKind::Rename if !value.is_empty() => {
                return Some(ManageIntent::Confirm(ManageMutation::Rename {
                    server_id: self.selected_server_id()?.to_string(),
                    name: value,
                }));
            }
            _ => {}
        }
        None
    }

    fn begin_create(&mut self) {
        self.surface = Some(ManageSurface::Create);
        self.create_draft = CreateDraft::default();
        self.create_step = Some(CreateStep::Name);
        self.input = Some((ManageInputKind::CreateName, String::new()));
    }

    fn begin_import(&mut self) {
        self.surface = Some(ManageSurface::Import);
        self.import_draft = ImportDraft::default();
        self.import_step = Some(ImportStep::Path);
        self.input = Some((ManageInputKind::ImportPath, String::new()));
    }

    fn advance_create(&mut self, step: CreateStep, input: ManageInputKind) {
        self.create_step = Some(step);
        self.input = Some((input, String::new()));
    }

    fn advance_import(&mut self, step: ImportStep, input: ManageInputKind) {
        self.import_step = Some(step);
        self.input = Some((input, String::new()));
    }

    fn move_selection(&mut self, offset: isize) {
        if !self.servers.is_empty() {
            self.selected =
                (self.selected as isize + offset).rem_euclid(self.servers.len() as isize) as usize;
        }
    }

    fn normalize_selection(&mut self) {
        self.selected = self.selected.min(self.servers.len().saturating_sub(1));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageMutationResult {
    pub message: String,
    pub operation_id: Option<String>,
}

pub async fn execute(
    client: &SharedClient,
    mutation: ManageMutation,
) -> Result<ManageMutationResult, CliError> {
    match mutation {
        ManageMutation::SetActive { server_id } => {
            let result: SimpleResultDto = client
                .post_json(
                    "/v1/active-server",
                    &ActiveServerRequestDto {
                        server_id: Some(server_id),
                    },
                )
                .await?;
            Ok(ManageMutationResult {
                message: result.result,
                operation_id: result.operation_id,
            })
        }
        ManageMutation::Create(draft) => {
            let result: ServerCreateResultDto = client
                .post_json(
                    "/v1/servers/create",
                    &ServerCreateRequestDto {
                        name: draft.name,
                        server_type: draft.server_type,
                        java_flavor: draft.java_flavor,
                        port: draft.port,
                        world_name: draft.world_name,
                        accept_eula: Some(draft.accept_eula),
                        ..Default::default()
                    },
                )
                .await?;
            Ok(ManageMutationResult {
                message: result.message,
                operation_id: result.operation_id,
            })
        }
        ManageMutation::Import(draft) => {
            let result: ServerImportResultDto = client
                .post_json(
                    "/v1/servers/import",
                    &ServerImportRequestDto {
                        action: Some(if draft.import_kind == "transfer" {
                            "importTransfer".to_string()
                        } else {
                            "importExisting".to_string()
                        }),
                        source_path: Some(draft.source_path),
                        import_kind: Some(draft.import_kind),
                        display_name: draft.display_name,
                        server_type: draft.server_type,
                        active_world_name: draft.active_world_name,
                        port: None,
                        max_players: None,
                        accept_eula: Some(draft.accept_eula),
                        enable_playit: None,
                        transfer_mode: None,
                        backup_path: None,
                        java_port_overrides: HashMap::new(),
                        bedrock_port_overrides: HashMap::new(),
                    },
                )
                .await?;
            Ok(ManageMutationResult {
                message: result.message,
                operation_id: result.operation_id,
            })
        }
        ManageMutation::Rename { server_id, name } => {
            let result: ServerRenameResultDto = client
                .post_json(
                    "/v1/servers/rename",
                    &ServerRenameRequestDto { server_id, name },
                )
                .await?;
            Ok(ManageMutationResult {
                message: result.message,
                operation_id: None,
            })
        }
        ManageMutation::AcceptEula { server_id } => {
            let result: ServerEulaResultDto = client
                .post_json(
                    "/v1/servers/eula",
                    &ServerEulaRequestDto {
                        server_id: Some(server_id),
                    },
                )
                .await?;
            Ok(ManageMutationResult {
                message: result.message,
                operation_id: None,
            })
        }
        ManageMutation::Delete { server_id } => {
            let result: ServerDeleteResultDto = client
                .post_json("/v1/servers/delete", &ServerDeleteRequestDto { server_id })
                .await?;
            Ok(ManageMutationResult {
                message: result.message,
                operation_id: None,
            })
        }
    }
}

fn parse_optional_i64(value: &str) -> Option<Option<i64>> {
    if value.is_empty() {
        Some(None)
    } else {
        value.parse().ok().map(Some)
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "" | "y" | "yes" | "true" => Some(true),
        "n" | "no" | "false" => Some(false),
        _ => None,
    }
}
