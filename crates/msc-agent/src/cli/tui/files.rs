//! Read-only server-file browsing for the authenticated TUI.
//!
//! The agent remains the only authority over which paths may be read. This
//! state stores only the relative path returned by the scoped files routes;
//! it never scans the local machine or accepts an arbitrary filesystem path.

use crossterm::event::KeyCode;
use serde::Deserialize;

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub modified_at: Option<String>,
    #[serde(default)]
    pub file_extension: Option<String>,
    #[serde(default)]
    pub is_previewable: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FilesResponse {
    #[serde(default)]
    pub server_name: Option<String>,
    pub path: String,
    #[serde(default)]
    pub parent_path: Option<String>,
    #[serde(default)]
    pub items: Vec<FileItem>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesIntent {
    Navigate(Option<String>),
    Preview(String),
    ReportPath(String),
}

#[derive(Debug, Clone, Default)]
pub struct FilesState {
    pub response: Option<FilesResponse>,
    pub preview: Option<FilePreview>,
    pub requested_path: Option<String>,
    pub loaded: bool,
    pub error: Option<String>,
    pub selected: usize,
    pub detail_open: bool,
    pub status: Option<String>,
}

impl FilesState {
    pub async fn load(
        client: &SharedClient,
        requested_path: Option<String>,
    ) -> Result<Self, CliError> {
        let path = requested_path
            .as_deref()
            .map_or_else(|| "/v1/files".to_string(), files_path);
        let response: FilesResponse = client.get_json(&path).await?;
        let mut state = Self {
            response: Some(response),
            requested_path,
            loaded: true,
            ..Self::default()
        };
        state.normalize_selection();
        Ok(state)
    }

    pub async fn preview(
        &mut self,
        client: &SharedClient,
        requested_path: &str,
    ) -> Result<(), CliError> {
        let preview: FilePreview = client
            .get_json(&format!(
                "/v1/files/read?path={}",
                encode_uri_component(requested_path)
            ))
            .await?;
        if !preview.success {
            return Err(CliError::usage(preview.message));
        }
        self.preview = Some(preview);
        self.detail_open = true;
        self.error = None;
        self.status = None;
        Ok(())
    }

    pub fn set_requested_path(&mut self, path: Option<String>) {
        self.requested_path = path;
        self.response = None;
        self.preview = None;
        self.loaded = false;
        self.error = None;
        self.selected = 0;
        self.detail_open = false;
        self.status = None;
    }

    pub fn items(&self) -> &[FileItem] {
        self.response
            .as_ref()
            .map(|response| response.items.as_slice())
            .unwrap_or_default()
    }

    pub fn selected_item(&self) -> Option<&FileItem> {
        self.items().get(self.selected)
    }

    pub fn report_path(&self) -> String {
        self.response
            .as_ref()
            .map(|response| display_path(&response.path))
            .unwrap_or_else(|| "Server Root".to_string())
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<FilesIntent> {
        if self.detail_open {
            return match key {
                KeyCode::Esc | KeyCode::Char('b') => {
                    self.detail_open = false;
                    self.preview = None;
                    None
                }
                KeyCode::Char('y') => Some(FilesIntent::ReportPath(
                    self.preview
                        .as_ref()
                        .and_then(|preview| preview.path.as_deref())
                        .map(display_path)
                        .unwrap_or_else(|| "Server Root".to_string()),
                )),
                _ => None,
            };
        }

        match key {
            KeyCode::Esc | KeyCode::Char('b') => self
                .response
                .as_ref()
                .and_then(|response| response.parent_path.clone())
                .map(|path| FilesIntent::Navigate(Some(path))),
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
                None
            }
            KeyCode::Enter => match self.selected_item() {
                Some(item) if item.is_directory => {
                    Some(FilesIntent::Navigate(Some(item.path.clone())))
                }
                Some(item) if item.is_previewable => Some(FilesIntent::Preview(item.path.clone())),
                Some(_) => {
                    self.status = Some("That file type is not previewable".to_string());
                    None
                }
                None => None,
            },
            KeyCode::Char('y') => Some(FilesIntent::ReportPath(self.report_path())),
            KeyCode::Char('r') => {
                self.loaded = false;
                None
            }
            _ => None,
        }
    }

    fn move_selection(&mut self, offset: isize) {
        let count = self.items().len();
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn normalize_selection(&mut self) {
        self.selected = self.selected.min(self.items().len().saturating_sub(1));
    }
}

fn files_path(path: &str) -> String {
    format!("/v1/files?path={}", encode_uri_component(path))
}

fn display_path(path: &str) -> String {
    if path.is_empty() {
        "Server Root".to_string()
    } else {
        format!("Server Root / {path}")
    }
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
