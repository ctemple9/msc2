//! Host-local file browsing and text editing for the selected server.

use std::io::{self, Stdout};
use std::path::Path;
use std::process::Command;

use crossterm::event::KeyCode;
use crossterm::style::Color;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use serde::Deserialize;

use super::{Snapshot, paint};
use crate::cli::transport::{SharedClient, encode_query_component};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileItem {
    name: String,
    path: String,
    is_directory: bool,
    size_bytes: Option<u64>,
    is_previewable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileListing {
    path: String,
    parent_path: Option<String>,
    items: Vec<FileItem>,
    note: Option<String>,
}

#[derive(Default)]
pub(super) struct FilesUi {
    listing: Option<FileListing>,
    selected: usize,
    scroll: usize,
    server_id: Option<String>,
}

impl FilesUi {
    pub(super) async fn open(&mut self, snapshot: &Snapshot, message: &mut String) {
        if self.server_id != snapshot.active_server_id {
            self.server_id = snapshot.active_server_id.clone();
            self.listing = None;
            self.selected = 0;
            self.scroll = 0;
        }
        let path = self
            .listing
            .as_ref()
            .map_or("", |listing| listing.path.as_str())
            .to_owned();
        self.load(&path, message).await;
    }

    async fn load(&mut self, path: &str, message: &mut String) {
        let Ok(client) = SharedClient::connect_local().await else {
            *message = "Agent unavailable; cannot browse server files.".into();
            return;
        };
        let route = format!("/v1/files?path={}", encode_query_component(path));
        match client.get_json::<FileListing>(&route).await {
            Ok(listing) if listing.note.is_none() => {
                self.listing = Some(listing);
                self.selected = 0;
                self.scroll = 0;
                message.clear();
            }
            Ok(listing) => {
                *message = format!(
                    "Could not open folder: {}",
                    listing.note.unwrap_or_default()
                )
            }
            Err(error) => *message = error.to_string(),
        }
    }

    pub(super) async fn handle(
        &mut self,
        key: KeyCode,
        snapshot: &Snapshot,
        out: &mut Stdout,
        message: &mut String,
    ) -> bool {
        if self.server_id != snapshot.active_server_id {
            self.open(snapshot, message).await;
        }
        let count = self
            .listing
            .as_ref()
            .map_or(0, |listing| listing.items.len());
        match key {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(count.saturating_sub(1)),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(10),
            KeyCode::PageDown => self.selected = (self.selected + 10).min(count.saturating_sub(1)),
            KeyCode::Backspace | KeyCode::Left => {
                if let Some(parent) = self
                    .listing
                    .as_ref()
                    .and_then(|listing| listing.parent_path.clone())
                {
                    self.load(&parent, message).await;
                }
            }
            KeyCode::Char('r') => {
                let path = self
                    .listing
                    .as_ref()
                    .map_or("", |listing| listing.path.as_str())
                    .to_owned();
                self.load(&path, message).await;
            }
            KeyCode::Enter | KeyCode::Right => {
                if let Some(item) = self
                    .listing
                    .as_ref()
                    .and_then(|listing| listing.items.get(self.selected))
                    .cloned()
                {
                    if item.is_directory {
                        self.load(&item.path, message).await;
                    } else if item.is_previewable {
                        self.edit(&item, snapshot, out, message);
                    } else {
                        *message = format!("{} is not a text file.", item.name);
                    }
                }
            }
            _ => return false,
        }
        true
    }

    fn edit(&self, item: &FileItem, snapshot: &Snapshot, out: &mut Stdout, message: &mut String) {
        let Some(server) = snapshot.active_server_id.as_ref().and_then(|id| {
            snapshot
                .servers
                .as_ref()?
                .iter()
                .find(|server| &server.id == id)
        }) else {
            *message = "Select a server before editing files.".into();
            return;
        };
        let root = match Path::new(&server.directory).canonicalize() {
            Ok(root) => root,
            Err(error) => {
                *message = format!("Could not open server directory: {error}");
                return;
            }
        };
        let target = match root.join(&item.path).canonicalize() {
            Ok(target) if target.starts_with(&root) && target.is_file() => target,
            _ => {
                *message = "File is outside the server directory or no longer exists.".into();
                return;
            }
        };
        // Terminal editors need the normal screen. Restore the TUI even if launch fails.
        let _ = execute!(out, cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        #[cfg(windows)]
        let (editor, result) = (
            "Microsoft Edit",
            Command::new("edit.exe").arg(&target).status(),
        );
        #[cfg(not(windows))]
        let (editor, result) = ("nano", Command::new("nano").arg("--").arg(&target).status());
        let raw = terminal::enable_raw_mode();
        let screen = execute!(out, EnterAlternateScreen, cursor::Hide);
        *message = match (result, raw, screen) {
            (Ok(status), Ok(()), Ok(())) if status.success() => {
                format!("Returned from {editor}: {}", item.name)
            }
            (Ok(status), Ok(()), Ok(())) => format!("{editor} exited with {status}"),
            #[cfg(windows)]
            (Err(error), _, _) if error.kind() == io::ErrorKind::NotFound => {
                "Microsoft Edit is not installed. Run: winget install Microsoft.Edit".into()
            }
            (Err(error), _, _) => format!("Could not launch {editor}: {error}"),
            (_, Err(error), _) | (_, _, Err(error)) => {
                format!("Could not restore terminal: {error}")
            }
        };
    }

    pub(super) fn draw(
        &self,
        out: &mut Stdout,
        snapshot: &Snapshot,
        x: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        if snapshot.active_server_id.is_none() {
            return paint(
                out,
                x,
                7,
                width,
                "Select a server to browse its files.",
                Color::DarkGrey,
            );
        }
        let Some(listing) = &self.listing else {
            return paint(
                out,
                x,
                7,
                width,
                "File listing unavailable. Press r to retry.",
                Color::DarkGrey,
            );
        };
        paint(
            out,
            x,
            6,
            width,
            &format!("SERVER ROOT /{}", listing.path),
            Color::White,
        )?;
        paint(out, x, 8, width, "FOLDERS AND FILES", Color::DarkGrey)?;
        let available = height.saturating_sub(18) as usize;
        let scroll = self
            .scroll
            .max(self.selected.saturating_sub(available.saturating_sub(1)))
            .min(self.selected);
        for (line, item) in listing
            .items
            .iter()
            .skip(scroll)
            .take(available)
            .enumerate()
        {
            let selected = scroll + line == self.selected;
            let label = if item.is_directory {
                format!("{}/", item.name)
            } else {
                format!("{}  ·  {} B", item.name, item.size_bytes.unwrap_or(0))
            };
            paint(
                out,
                x,
                10 + line as u16,
                width,
                &format!("{} {label}", if selected { ">" } else { " " }),
                if selected {
                    Color::Green
                } else if item.is_previewable || item.is_directory {
                    Color::White
                } else {
                    Color::DarkGrey
                },
            )?;
        }
        if listing.items.is_empty() {
            paint(out, x, 10, width, "This folder is empty.", Color::DarkGrey)?;
        }
        paint(
            out,
            x,
            height - 7,
            width,
            if cfg!(windows) {
                "↑↓ Select  Enter Open/Edit in Microsoft Edit  Backspace Parent  r Refresh"
            } else {
                "↑↓ Select  Enter Open/Edit in nano  Backspace Parent  r Refresh"
            },
            Color::DarkGrey,
        )?;
        Ok(())
    }
}
