//! The focused server editor used from Manage Servers.
//!
//! Server paths and measurements belong to the agent host. The TUI only edits
//! the path string or displays bounded values returned by the existing routes;
//! it never opens a local picker or reads a remote directory itself.

use std::collections::HashMap;

use crossterm::event::KeyCode;
use msc_api::dto::{
    BroadcastSimpleResultDto, BroadcastStatusDto, JavaConfigResponseDto, JavaConfigSetRequestDto,
    JavaRuntimeDto, JavaRuntimesResponseDto, PlayitActionResultDto, PlayitStatusDto,
    RamConfigResponseDto, RamConfigUpdateRequestDto, RamConfigUpdateResultDto,
    ServerDirectoryRequestDto, ServerDirectoryResultDto, ServerDirectorySizeResponseDto, ServerDto,
    ServerRenameRequestDto, ServerRenameResultDto, SettingsResponseDto, SettingsUpdateRequestDto,
    SettingsUpdateResultDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditorSurface {
    #[default]
    General,
    Services,
    Java,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorInputKind {
    DisplayName,
    Directory,
    RamPair,
    Port,
    JavaPath,
    JavaArguments,
}

impl EditorInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::DisplayName => "Display name",
            Self::Directory => "Agent-host server directory",
            Self::RamPair => "RAM min|max in GB (blank side keeps current)",
            Self::Port => "Game port",
            Self::JavaPath => "Java executable path on agent host",
            Self::JavaArguments => "Extra Java arguments",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMutation {
    Rename {
        server_id: String,
        name: String,
    },
    SetDirectory {
        server_id: String,
        directory: String,
    },
    UpdateRam {
        min: Option<String>,
        max: Option<String>,
    },
    UpdatePort {
        port: i64,
    },
    SetJavaPath {
        path: String,
    },
    SetJavaArguments {
        arguments: String,
    },
    Playit {
        start: bool,
    },
    XboxBroadcast {
        start: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorIntent {
    Back,
    Confirm(EditorMutation),
}

#[derive(Debug, Clone, Default)]
pub struct ServerEditorState {
    pub server: Option<ServerDto>,
    pub surface: EditorSurface,
    pub input: Option<(EditorInputKind, String)>,
    pub ram: Option<RamConfigResponseDto>,
    pub settings: Option<SettingsResponseDto>,
    pub java_config: Option<JavaConfigResponseDto>,
    pub java_runtimes: Vec<JavaRuntimeDto>,
    pub selected_runtime: usize,
    pub playit: Option<PlayitStatusDto>,
    pub broadcast: Option<BroadcastStatusDto>,
    pub storage_bytes: Option<u64>,
    pub playit_available: bool,
    pub broadcast_available: bool,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl ServerEditorState {
    pub async fn load(client: &SharedClient, server: ServerDto) -> Result<Self, CliError> {
        let ram = client.get_json("/v1/config/ram").await.ok();
        let settings = client.get_json("/v1/settings").await.ok();
        let java_config = client.get_json("/v1/config/java-runtime").await.ok();
        let java_runtimes = client
            .get_json::<JavaRuntimesResponseDto>("/v1/java-runtimes")
            .await
            .map(|response| response.runtimes)
            .unwrap_or_default();
        let playit = client.get_json("/v1/playit").await.ok();
        let broadcast = client.get_json("/v1/broadcast/status").await.ok();
        let storage_bytes = client
            .get_json::<ServerDirectorySizeResponseDto>(&format!(
                "/v1/servers/size?serverId={}",
                server.id
            ))
            .await
            .ok()
            .and_then(|response| response.size_bytes);

        Ok(Self {
            server: Some(server),
            ram,
            settings,
            java_config,
            java_runtimes,
            playit,
            broadcast,
            storage_bytes,
            loaded: true,
            ..Self::default()
        })
    }

    pub fn server_id(&self) -> Option<&str> {
        self.server.as_ref().map(|server| server.id.as_str())
    }

    pub fn server_name(&self) -> &str {
        self.server
            .as_ref()
            .map(|server| server.name.as_str())
            .unwrap_or("No server selected")
    }

    pub fn set_capabilities(&mut self, playit_available: bool, broadcast_available: bool) {
        self.playit_available = playit_available;
        self.broadcast_available = broadcast_available;
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<EditorIntent> {
        if let Some((kind, value)) = self.input.take() {
            return self.handle_input(kind, value, key);
        }
        match key {
            KeyCode::Esc => return Some(EditorIntent::Back),
            KeyCode::Char('1') => {
                self.surface = EditorSurface::General;
            }
            KeyCode::Char('2') => {
                self.surface = EditorSurface::Services;
            }
            KeyCode::Char('3') => {
                self.surface = EditorSurface::Java;
            }
            KeyCode::Char('r') => {
                self.loaded = false;
            }
            _ => return self.handle_surface_key(key),
        }
        None
    }

    fn handle_surface_key(&mut self, key: KeyCode) -> Option<EditorIntent> {
        match self.surface {
            EditorSurface::General => match key {
                KeyCode::Char('n') => {
                    self.input = Some((EditorInputKind::DisplayName, String::new()))
                }
                KeyCode::Char('p') => {
                    self.input = Some((EditorInputKind::Directory, String::new()))
                }
                KeyCode::Char('m') => self.input = Some((EditorInputKind::RamPair, String::new())),
                KeyCode::Char('o') => self.input = Some((EditorInputKind::Port, String::new())),
                _ => {}
            },
            EditorSurface::Services => match key {
                KeyCode::Char('p') if self.playit_available => {
                    let start = !self.playit.as_ref().is_some_and(|status| status.is_running);
                    return Some(EditorIntent::Confirm(EditorMutation::Playit { start }));
                }
                KeyCode::Char('x') if self.broadcast_available => {
                    let start = !self
                        .broadcast
                        .as_ref()
                        .is_some_and(|status| status.xbox_broadcast_running);
                    return Some(EditorIntent::Confirm(EditorMutation::XboxBroadcast {
                        start,
                    }));
                }
                _ => {}
            },
            EditorSurface::Java => match key {
                KeyCode::Char('d') => self.detect_java(),
                KeyCode::Char('p') => self.input = Some((EditorInputKind::JavaPath, String::new())),
                KeyCode::Char('a') => {
                    self.input = Some((EditorInputKind::JavaArguments, String::new()))
                }
                KeyCode::Char('j') | KeyCode::Down => self.move_runtime(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_runtime(-1),
                KeyCode::Enter => {
                    let runtime = self.java_runtimes.get(self.selected_runtime)?;
                    return Some(EditorIntent::Confirm(EditorMutation::SetJavaPath {
                        path: runtime.executable_path.clone(),
                    }));
                }
                _ => {}
            },
        }
        None
    }

    fn handle_input(
        &mut self,
        kind: EditorInputKind,
        mut value: String,
        key: KeyCode,
    ) -> Option<EditorIntent> {
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

    fn finish_input(&mut self, kind: EditorInputKind, value: String) -> Option<EditorIntent> {
        let value = value.trim().to_string();
        let server_id = self.server_id()?.to_string();
        match kind {
            EditorInputKind::DisplayName if !value.is_empty() => {
                Some(EditorIntent::Confirm(EditorMutation::Rename {
                    server_id,
                    name: value,
                }))
            }
            EditorInputKind::Directory if !value.is_empty() => {
                Some(EditorIntent::Confirm(EditorMutation::SetDirectory {
                    server_id,
                    directory: value,
                }))
            }
            EditorInputKind::RamPair => {
                let mut values = value.split('|').map(str::trim);
                let min = parse_optional_value(values.next()?)?;
                let max = parse_optional_value(values.next()?)?;
                if min.is_none() && max.is_none() {
                    return None;
                }
                Some(EditorIntent::Confirm(EditorMutation::UpdateRam {
                    min,
                    max,
                }))
            }
            EditorInputKind::Port => Some(EditorIntent::Confirm(EditorMutation::UpdatePort {
                port: value.parse().ok()?,
            })),
            EditorInputKind::JavaPath if !value.is_empty() => {
                Some(EditorIntent::Confirm(EditorMutation::SetJavaPath {
                    path: value,
                }))
            }
            EditorInputKind::JavaArguments => {
                Some(EditorIntent::Confirm(EditorMutation::SetJavaArguments {
                    arguments: value,
                }))
            }
            _ => None,
        }
    }

    fn detect_java(&mut self) {
        if let Some(runtime) = self.java_runtimes.get(self.selected_runtime) {
            self.status = Some(format!(
                "Detected {}{} at {}",
                runtime.name,
                runtime
                    .major_version
                    .map(|major| format!(" (Java {major})"))
                    .unwrap_or_default(),
                runtime.executable_path
            ));
        } else {
            self.status = Some("No Java runtimes were detected by the agent".to_string());
        }
    }

    fn move_runtime(&mut self, offset: isize) {
        if !self.java_runtimes.is_empty() {
            self.selected_runtime = (self.selected_runtime as isize + offset)
                .rem_euclid(self.java_runtimes.len() as isize)
                as usize;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMutationResult {
    pub message: String,
    pub operation_id: Option<String>,
}

pub async fn execute(
    client: &SharedClient,
    mutation: EditorMutation,
) -> Result<EditorMutationResult, CliError> {
    match mutation {
        EditorMutation::Rename { server_id, name } => {
            let result: ServerRenameResultDto = client
                .post_json(
                    "/v1/servers/rename",
                    &ServerRenameRequestDto { server_id, name },
                )
                .await?;
            result_message(result.message)
        }
        EditorMutation::SetDirectory {
            server_id,
            directory,
        } => {
            let result: ServerDirectoryResultDto = client
                .post_json(
                    "/v1/servers/directory",
                    &ServerDirectoryRequestDto {
                        server_id,
                        directory,
                    },
                )
                .await?;
            result_message(result.message)
        }
        EditorMutation::UpdateRam { min, max } => {
            let result: RamConfigUpdateResultDto = client
                .post_json(
                    "/v1/config/ram",
                    &RamConfigUpdateRequestDto {
                        min_ram_gb: parse_ram(min)?,
                        max_ram_gb: parse_ram(max)?,
                    },
                )
                .await?;
            result_message(
                result
                    .message
                    .unwrap_or_else(|| "RAM allocation updated.".to_string()),
            )
        }
        EditorMutation::UpdatePort { port } => {
            let mut changes = HashMap::new();
            changes.insert("server-port".to_string(), port.to_string());
            let result: SettingsUpdateResultDto = client
                .post_json("/v1/settings", &SettingsUpdateRequestDto { changes })
                .await?;
            result_message(result.message)
        }
        EditorMutation::SetJavaPath { path } => {
            let result: JavaConfigResponseDto = client
                .post_json(
                    "/v1/config/java-runtime",
                    &JavaConfigSetRequestDto {
                        executable_path: Some(path),
                        extra_flags: None,
                    },
                )
                .await?;
            result_message(format!(
                "Java executable set to {}.",
                result
                    .executable_path
                    .unwrap_or_else(|| "default".to_string())
            ))
        }
        EditorMutation::SetJavaArguments { arguments } => {
            let result: JavaConfigResponseDto = client
                .post_json(
                    "/v1/config/java-runtime",
                    &JavaConfigSetRequestDto {
                        executable_path: None,
                        extra_flags: Some(arguments),
                    },
                )
                .await?;
            result_message(format!(
                "Java arguments set to {}.",
                result.extra_flags.unwrap_or_default()
            ))
        }
        EditorMutation::Playit { start } => {
            let path = if start {
                "/v1/playit/start"
            } else {
                "/v1/playit/stop"
            };
            let result: PlayitActionResultDto =
                client.post_json(path, &serde_json::json!({})).await?;
            Ok(EditorMutationResult {
                message: result.message.unwrap_or_else(|| {
                    format!("Playit {} requested.", if start { "start" } else { "stop" })
                }),
                operation_id: result.operation_id,
            })
        }
        EditorMutation::XboxBroadcast { start } => {
            let path = if start {
                "/v1/broadcast/start"
            } else {
                "/v1/broadcast/stop"
            };
            let result: BroadcastSimpleResultDto =
                client.post_json(path, &serde_json::json!({})).await?;
            Ok(EditorMutationResult {
                message: format!(
                    "Xbox Broadcast {} requested.",
                    if start { "start" } else { "stop" }
                ),
                operation_id: result.operation_id,
            })
        }
    }
}

fn result_message(message: String) -> Result<EditorMutationResult, CliError> {
    Ok(EditorMutationResult {
        message,
        operation_id: None,
    })
}

fn parse_optional_value(value: &str) -> Option<Option<String>> {
    if value.is_empty() {
        Some(None)
    } else {
        Some(Some(value.to_string()))
    }
}

fn parse_ram(value: Option<String>) -> Result<Option<f64>, CliError> {
    value
        .map(|value| {
            value
                .parse()
                .map_err(|_| CliError::usage("RAM values must be decimal GB numbers"))
        })
        .transpose()
}
