//! Host-wide settings for the local TUI client.

use std::io::{self, Stdout};
use std::process::Stdio;

use crossterm::event::KeyCode;
use crossterm::style::Color;
use msc_api::dto::{CurseForgeApiKeyStatusDto, PlayitSetupAcceptedDto, PlayitStatusDto};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;

use super::{box_outline, paint};
use crate::cli::transport::SharedClient;

#[derive(Default)]
pub(super) struct AppSettingsUi {
    open: bool,
    storage: super::storage::StorageUi,
    tab: usize,
    selected: usize,
    input: Option<Input>,
    draft: String,
    email: String,
    password: String,
    playit_field: usize,
    curseforge: Option<CurseForgeApiKeyStatusDto>,
    playit: Option<PlayitStatusDto>,
    playit_operation: Option<String>,
    update: Option<UpdateSummary>,
    uninstall_preview: Option<String>,
    uninstall_blocked: bool,
    confirm: Option<Confirm>,
    message: String,
    quit_after_action: bool,
}

#[derive(Clone, Copy)]
enum Input {
    CurseForge,
    Playit,
    ConfirmReset(&'static str),
    ConfirmUninstall,
}

#[derive(Clone)]
enum Confirm {
    ClearCurseForge,
    InstallUpdate(String),
    ClearPlayit,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSummary {
    state: String,
    release_id: Option<String>,
    release_notes: String,
    install_mode: Option<String>,
    detail: String,
}

impl AppSettingsUi {
    pub(super) fn is_open(&self) -> bool {
        self.open
    }

    pub(super) fn should_quit(&self) -> bool {
        self.quit_after_action
    }

    pub(super) async fn open(&mut self) {
        self.open = true;
        self.message.clear();
        self.refresh_services().await;
    }

    pub(super) async fn handle(&mut self, key: KeyCode) -> bool {
        if !self.open {
            return false;
        }
        if self.tab == 2
            && (self.storage.is_confirming()
                || !matches!(
                    key,
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::Char('1' | '2' | '3')
                ))
        {
            if !self.storage.handle(key).await {
                self.open = false;
            }
            return true;
        }
        if let Some(confirm) = self.confirm.take() {
            match (confirm, key) {
                (Confirm::ClearCurseForge, KeyCode::Char('y' | 'Y')) => {
                    self.save_curseforge("").await
                }
                (Confirm::InstallUpdate(id), KeyCode::Char('y' | 'Y')) => {
                    self.install_update(&id).await
                }
                (Confirm::ClearPlayit, KeyCode::Char('y' | 'Y')) => self.clear_playit().await,
                (_, KeyCode::Esc | KeyCode::Char('n' | 'N')) => self.message = "Cancelled.".into(),
                (confirm, _) => self.confirm = Some(confirm),
            }
            return true;
        }
        if let Some(input) = self.input {
            match input {
                Input::CurseForge => match key {
                    KeyCode::Esc => {
                        self.input = None;
                        self.draft.clear();
                    }
                    KeyCode::Backspace => {
                        self.draft.pop();
                    }
                    KeyCode::Enter => {
                        if self.draft.trim().is_empty() {
                            self.message = "Enter a CurseForge API key, or press Esc and use c to clear the saved key.".into();
                            return true;
                        }
                        let value = std::mem::take(&mut self.draft);
                        self.input = None;
                        self.save_curseforge(&value).await;
                    }
                    KeyCode::Char(c) if !c.is_control() => self.draft.push(c),
                    _ => {}
                },
                Input::Playit => match key {
                    KeyCode::Esc => {
                        self.input = None;
                        self.email.clear();
                        self.password.clear();
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        self.playit_field = (self.playit_field + 1).min(2)
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        self.playit_field = self.playit_field.saturating_sub(1)
                    }
                    KeyCode::Backspace => {
                        if self.playit_field == 0 {
                            self.email.pop();
                        } else if self.playit_field == 1 {
                            self.password.pop();
                        }
                    }
                    KeyCode::Char(c) if !c.is_control() => {
                        if self.playit_field == 0 {
                            self.email.push(c);
                        } else if self.playit_field == 1 {
                            self.password.push(c);
                        }
                    }
                    KeyCode::Enter if self.playit_field < 2 => self.playit_field += 1,
                    KeyCode::Enter => self.submit_playit_setup().await,
                    _ => {}
                },
                Input::ConfirmReset(mode) => match key {
                    KeyCode::Esc => self.input = None,
                    KeyCode::Char(c) if !c.is_control() => self.draft.push(c),
                    KeyCode::Backspace => {
                        self.draft.pop();
                    }
                    KeyCode::Enter => {
                        if self.draft == "RESET AGENT" {
                            self.input = None;
                            self.reset_host(mode).await;
                        } else {
                            self.message = "Type RESET AGENT exactly to confirm.".into();
                        }
                    }
                    _ => {}
                },
                Input::ConfirmUninstall => match key {
                    KeyCode::Esc => self.input = None,
                    KeyCode::Char(c) if !c.is_control() => self.draft.push(c),
                    KeyCode::Backspace => {
                        self.draft.pop();
                    }
                    KeyCode::Enter => {
                        if self.draft == "UNINSTALL MSC 2" {
                            self.input = None;
                            self.uninstall().await;
                        } else {
                            self.message = "Type UNINSTALL MSC 2 exactly to confirm.".into();
                        }
                    }
                    _ => {}
                },
            }
            return true;
        }
        match key {
            KeyCode::Esc => self.open = false,
            KeyCode::Tab | KeyCode::Right => {
                self.tab = (self.tab + 1).min(2);
                self.selected = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = self.tab.saturating_sub(1);
                self.selected = 0;
            }
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => {
                self.tab = 2;
                self.storage.load().await;
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(if self.tab == 0 { 4 } else { 2 })
            }
            KeyCode::Enter => {
                if self.tab == 0 {
                    match self.selected {
                        0 => self.check_updates().await,
                        1 => {
                            if let Some(id) = self
                                .update
                                .as_ref()
                                .and_then(|update| update.release_id.clone())
                            {
                                if self
                                    .update
                                    .as_ref()
                                    .is_some_and(|update| update.state == "staged")
                                {
                                    self.confirm = Some(Confirm::InstallUpdate(id));
                                } else {
                                    self.message = "Check for updates first.".into();
                                }
                            } else {
                                self.message = "No staged update is available.".into();
                            }
                        }
                        2 => {
                            self.draft.clear();
                            self.input = Some(Input::ConfirmReset("configuration"));
                            self.message = "This clears host settings and credentials while preserving server data. Type RESET AGENT.".into();
                        }
                        3 => {
                            self.draft.clear();
                            self.input = Some(Input::ConfirmReset("everything"));
                            self.message = "This also deletes all managed servers, worlds, and backups. Type RESET AGENT.".into();
                        }
                        _ if self.uninstall_preview.is_none() => {
                            self.preview_uninstall().await;
                        }
                        _ if self.uninstall_blocked => {
                            self.message = "The uninstall preview is blocked. Resolve its findings and preview again.".into();
                        }
                        _ => {
                            self.draft.clear();
                            self.input = Some(Input::ConfirmUninstall);
                            self.message = "Type UNINSTALL MSC 2 exactly to permanently remove the listed installation.".into();
                        }
                    }
                } else {
                    match self.selected {
                        0 => {
                            self.draft.clear();
                            self.input = Some(Input::CurseForge);
                        }
                        1 => {
                            self.input = Some(Input::Playit);
                            self.playit_field = 0;
                        }
                        _ => self.refresh_services().await,
                    }
                }
            }
            KeyCode::Char('e') if self.tab == 1 && self.selected == 0 => {
                self.draft.clear();
                self.input = Some(Input::CurseForge);
            }
            KeyCode::Char('c')
                if self.tab == 1
                    && self.selected == 0
                    && self
                        .curseforge
                        .as_ref()
                        .is_some_and(|status| status.configured) =>
            {
                self.confirm = Some(Confirm::ClearCurseForge)
            }
            KeyCode::Char('p') if self.tab == 1 => {
                self.input = Some(Input::Playit);
                self.playit_field = 0;
            }
            KeyCode::Char('x')
                if self.tab == 1
                    && self
                        .playit
                        .as_ref()
                        .is_some_and(|status| status.has_secret_key) =>
            {
                self.confirm = Some(Confirm::ClearPlayit)
            }
            KeyCode::Char('r') => self.poll_playit().await,
            _ => {}
        }
        if self.tab == 2 {
            self.storage.load().await;
        }
        true
    }

    async fn refresh_services(&mut self) {
        let Ok(client) = SharedClient::connect_local().await else {
            self.message = "Agent is unavailable; service settings could not be loaded.".into();
            return;
        };
        let (curseforge, playit) = tokio::join!(
            client.get_json::<CurseForgeApiKeyStatusDto>("/v1/config/curseforge"),
            client.get_json::<PlayitStatusDto>("/v1/playit"),
        );
        self.curseforge = curseforge.ok();
        self.playit = playit.ok();
    }

    async fn save_curseforge(&mut self, key: &str) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .post_json::<_, CurseForgeApiKeyStatusDto>(
                    "/v1/config/curseforge",
                    &json!({"apiKey": key.trim()}),
                )
                .await
        }
        .await;
        match result {
            Ok(status) => {
                self.curseforge = Some(status.clone());
                self.message = if status.configured {
                    "CurseForge API key saved on this agent."
                } else {
                    "CurseForge API key cleared."
                }
                .into();
            }
            Err(error) => self.message = error.to_string(),
        }
        self.draft.clear();
    }

    async fn submit_playit_setup(&mut self) {
        if self.email.trim().is_empty() || self.password.is_empty() {
            self.message = "Enter both Playit email and password.".into();
            return;
        }
        let body =
            json!({"email": self.email.trim(), "password": self.password, "accountOnly": true});
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .post_json::<_, PlayitSetupAcceptedDto>("/v1/playit/setup", &body)
                .await
        }
        .await;
        match result {
            Ok(accepted) => {
                self.playit_operation = Some(accepted.operation_id);
                self.message =
                    "Playit account setup started. Press r to refresh its status.".into();
            }
            Err(error) => self.message = error.to_string(),
        }
        self.password.clear();
        self.input = None;
    }

    async fn clear_playit(&mut self) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .post_json::<_, Value>("/v1/playit/reset", &json!({}))
                .await
        }
        .await;
        match result {
            Ok(_) => {
                self.message = "Playit account and saved addresses cleared on this agent.".into();
                self.refresh_services().await;
            }
            Err(error) => self.message = error.to_string(),
        }
    }

    async fn poll_playit(&mut self) {
        let Some(id) = self.playit_operation.clone() else {
            self.refresh_services().await;
            return;
        };
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .get_json::<Value>(&format!("/v1/operations/{id}"))
                .await
        }
        .await;
        match result {
            Ok(operation) => {
                let state = operation["state"].as_str().unwrap_or("running");
                self.message = operation["statusLine"]
                    .as_str()
                    .unwrap_or("Playit setup is running.")
                    .to_owned();
                if matches!(state, "succeeded" | "failed" | "cancelled") {
                    if state == "failed" {
                        self.message = operation["error"]["message"]
                            .as_str()
                            .unwrap_or("Playit setup failed.")
                            .to_owned();
                    }
                    self.playit_operation = None;
                    self.refresh_services().await;
                }
            }
            Err(error) => self.message = error.to_string(),
        }
    }

    pub(super) async fn poll_services(&mut self) {
        if self.tab == 2 {
            self.storage.poll().await;
        }
        if self.playit_operation.is_some() {
            self.poll_playit().await;
        }
    }

    async fn check_updates(&mut self) {
        self.message = "Checking for a signed MSC 2 release…".into();
        let result = async {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let output = Command::new(executable)
                .args(["update", "check", "--json"])
                .stdin(Stdio::null())
                .output()
                .await
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
            }
            serde_json::from_slice::<UpdateSummary>(&output.stdout)
                .map_err(|error| error.to_string())
        }
        .await;
        match result {
            Ok(update) => {
                self.message = update.detail.clone();
                self.update = Some(update);
            }
            Err(error) => self.message = format!("Update check failed: {error}"),
        }
    }

    async fn install_update(&mut self, release_id: &str) {
        self.message = format!("Installing verified release {release_id}…");
        let result = async {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let output = Command::new(executable)
                .args([
                    "update",
                    "install",
                    "--release-id",
                    release_id,
                    "--yes",
                    "--json",
                ])
                .stdin(Stdio::null())
                .output()
                .await
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
            }
            serde_json::from_slice::<Value>(&output.stdout).map_err(|error| error.to_string())
        }
        .await;
        match result {
            Ok(result) => {
                self.message = result["detail"]
                    .as_str()
                    .unwrap_or("Update installation finished.")
                    .to_owned();
                self.quit_after_action = true;
            }
            Err(error) => self.message = format!("Update installation failed: {error}"),
        }
    }

    async fn reset_host(&mut self, mode: &str) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            client
                .post_json::<_, Value>(
                    "/v1/host/reset",
                    &json!({"mode": mode, "confirmation": "RESET AGENT"}),
                )
                .await
        }
        .await;
        match result {
            Ok(result) => {
                self.message = result["message"]
                    .as_str()
                    .unwrap_or("Host reset started.")
                    .to_owned();
                self.quit_after_action = true;
            }
            Err(error) => self.message = error.to_string(),
        }
    }

    async fn uninstall(&mut self) {
        if self.draft != "UNINSTALL MSC 2" {
            self.input = Some(Input::ConfirmUninstall);
            self.message = "Type UNINSTALL MSC 2 and press Enter. This permanently removes local servers and backups.".into();
            return;
        }
        let result = async {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let output = Command::new(executable)
                .args([
                    "uninstall",
                    "--danger",
                    "--confirm",
                    "UNINSTALL MSC 2",
                    "--json",
                ])
                .stdin(Stdio::null())
                .output()
                .await
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
            }
            serde_json::from_slice::<Value>(&output.stdout).map_err(|error| error.to_string())
        }
        .await;
        match result {
            Ok(result) => {
                self.message = result["detail"]
                    .as_str()
                    .unwrap_or("Uninstall was scheduled.")
                    .to_owned();
                self.quit_after_action = true;
            }
            Err(error) => self.message = format!("Uninstall did not start: {error}"),
        }
        self.draft.clear();
    }

    async fn preview_uninstall(&mut self) {
        self.message = "Reviewing the local uninstall inventory…".into();
        let result = async {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let output = Command::new(executable)
                .args(["uninstall", "--dry-run", "--json"])
                .stdin(Stdio::null())
                .output()
                .await
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
            }
            serde_json::from_slice::<Value>(&output.stdout).map_err(|error| error.to_string())
        }
        .await;
        match result {
            Ok(inventory) => {
                let entries = inventory["entries"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice);
                let present = entries
                    .iter()
                    .filter(|entry| entry["state"] == "present")
                    .count();
                let blocked = entries
                    .iter()
                    .filter(|entry| entry["state"] == "blocked")
                    .count();
                self.uninstall_preview = Some(if blocked == 0 {
                    format!(
                        "Preview: {present} local items are present for removal. Select Uninstall again to confirm."
                    )
                } else {
                    format!(
                        "Preview found {blocked} blocked items. Resolve them before uninstalling."
                    )
                });
                self.uninstall_blocked = blocked > 0;
                self.message = self.uninstall_preview.clone().unwrap_or_default();
            }
            Err(error) => self.message = format!("Could not preview uninstall: {error}"),
        }
    }

    pub(super) fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let w = width.saturating_sub(28).clamp(60, 116);
        let h = height.saturating_sub(8).clamp(24, 34);
        let x = width.saturating_sub(w) / 2;
        let y = height.saturating_sub(h) / 2;
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(out, x + 2, y + 1, w - 4, "MSC SETTINGS", Color::White)?;
        paint(
            out,
            x + 2,
            y + 3,
            w - 4,
            "1 General     2 Services     3 Storage",
            Color::DarkGrey,
        )?;
        let (active_tab_x, active_tab_label, active_tab_width) = match self.tab {
            0 => (x + 2, "1 General", 9),
            1 => (x + 16, "2 Services", 10),
            _ => (x + 31, "3 Storage", 9),
        };
        paint(
            out,
            active_tab_x,
            y + 3,
            active_tab_width,
            active_tab_label,
            Color::Green,
        )?;
        if self.tab == 2 {
            self.storage.draw(out, x + 2, y + 5, w - 4, h - 7)?;
            return Ok(());
        }
        if self.tab == 0 {
            paint(out, x + 2, y + 5, w - 4, "UPDATES", Color::DarkGrey)?;
            paint(
                out,
                x + 2,
                y + 7,
                w - 4,
                &format!("MSC 2 · installed bundle v{}", env!("CARGO_PKG_VERSION")),
                Color::White,
            )?;
            let update_state = self
                .update
                .as_ref()
                .map_or("Check for a signed release", |update| {
                    update.detail.as_str()
                });
            paint(out, x + 2, y + 8, w - 4, update_state, Color::DarkGrey)?;
            draw_row(
                out,
                x + 2,
                y + 10,
                w - 4,
                self.selected == 0,
                "Check for updates",
            )?;
            let install = self
                .update
                .as_ref()
                .and_then(|update| update.release_id.as_deref())
                .filter(|_| {
                    self.update
                        .as_ref()
                        .is_some_and(|update| update.state == "staged")
                })
                .map_or("Install available update", |id| id);
            draw_row(out, x + 2, y + 11, w - 4, self.selected == 1, install)?;
            paint(
                out,
                x + 2,
                y + 14,
                w - 4,
                "RESET AND UNINSTALL",
                Color::DarkGrey,
            )?;
            draw_row(
                out,
                x + 2,
                y + 16,
                w - 4,
                self.selected == 2,
                "Reset host configuration…",
            )?;
            draw_row(
                out,
                x + 2,
                y + 17,
                w - 4,
                self.selected == 3,
                "Reset host and all managed data…",
            )?;
            draw_row(
                out,
                x + 2,
                y + 18,
                w - 4,
                self.selected == 4,
                "Uninstall MSC 2…",
            )?;
            paint(
                out,
                x + 2,
                y + 20,
                w - 4,
                "Uninstall permanently removes the MSC installation, servers, worlds, and backups.",
                Color::DarkGrey,
            )?;
            if let Some(preview) = &self.uninstall_preview {
                paint(out, x + 2, y + 21, w - 4, preview, Color::Yellow)?;
            }
            if let Some(update) = &self.update {
                let detail = update.install_mode.as_ref().map_or_else(
                    || update.detail.clone(),
                    |mode| format!("{} · {mode}", update.detail),
                );
                paint(out, x + 2, y + 12, w - 4, &detail, Color::DarkGrey)?;
                if !update.release_notes.is_empty() {
                    paint(
                        out,
                        x + 2,
                        y + 13,
                        w - 4,
                        update.release_notes.lines().next().unwrap_or(""),
                        Color::DarkGrey,
                    )?;
                }
            }
        } else {
            paint(out, x + 2, y + 5, w - 4, "MODPACK IMPORTS", Color::DarkGrey)?;
            paint(
                out,
                x + 2,
                y + 7,
                w - 4,
                "CurseForge modpacks need an API key to resolve their files.",
                Color::White,
            )?;
            paint(
                out,
                x + 2,
                y + 8,
                w - 4,
                "Create one at console.curseforge.com; the key is stored on this agent and never shown again.",
                Color::DarkGrey,
            )?;
            let configured = self
                .curseforge
                .as_ref()
                .is_some_and(|status| status.configured);
            draw_row(
                out,
                x + 2,
                y + 10,
                w - 4,
                self.selected == 0,
                if configured {
                    "CurseForge API key · Configured · Enter replace / c clear"
                } else {
                    "CurseForge API key · Not configured · Enter to set"
                },
            )?;
            paint(out, x + 2, y + 13, w - 4, "PLAYIT", Color::DarkGrey)?;
            let playit = self.playit.as_ref();
            let account = if playit.is_some_and(|status| status.has_secret_key) {
                "Configured"
            } else {
                "Setup required"
            };
            draw_row(
                out,
                x + 2,
                y + 15,
                w - 4,
                self.selected == 1,
                &format!("Playit account · {account} · Enter manage setup / x clear"),
            )?;
            draw_row(
                out,
                x + 2,
                y + 16,
                w - 4,
                self.selected == 2,
                "Refresh service status",
            )?;
            for (index, label, value) in [
                (
                    0,
                    "Agent",
                    playit.and_then(|status| status.agent_id.as_deref()),
                ),
                (
                    1,
                    "Java tunnel",
                    playit.and_then(|status| status.java_address.as_deref()),
                ),
                (
                    2,
                    "Bedrock tunnel",
                    playit.and_then(|status| status.bedrock_address.as_deref()),
                ),
                (
                    3,
                    "Voice tunnel",
                    playit.and_then(|status| status.voice_address.as_deref()),
                ),
            ] {
                if let Some(value) = value {
                    paint(
                        out,
                        x + 4,
                        y + 17 + index as u16,
                        w - 8,
                        &format!("{label:<14}{value}"),
                        Color::DarkGrey,
                    )?;
                }
            }
            if self.playit_operation.is_some() {
                paint(
                    out,
                    x + 2,
                    y + 21,
                    w - 4,
                    "Playit setup is active · press r to refresh",
                    Color::Yellow,
                )?;
            }
            paint(
                out,
                x + 2,
                y + 23,
                w - 4,
                "Playit account is shared by this agent. Tunnel participation is per server in Manage → Services.",
                Color::DarkGrey,
            )?;
        }
        if let Some(confirm) = &self.confirm {
            let text = match confirm {
                Confirm::ClearCurseForge => {
                    "Clear the saved CurseForge key? y confirm · Esc cancel"
                }
                Confirm::InstallUpdate(_) => "Install the signed release? y confirm · Esc cancel",
                Confirm::ClearPlayit => {
                    "Clear Playit account and saved tunnel addresses? y confirm · Esc cancel"
                }
            };
            paint(out, x + 2, y + h - 4, w - 4, text, Color::Yellow)?;
        }
        if let Some(input) = self.input {
            if matches!(input, Input::Playit) {
                paint(out, x + 2, y + 5, w - 4, "SET UP PLAYIT", Color::White)?;
                paint(
                    out,
                    x + 2,
                    y + 6,
                    w - 4,
                    "Sign in on this host to configure the shared Playit agent.",
                    Color::DarkGrey,
                )?;
                paint(
                    out,
                    x + 2,
                    y + 8,
                    w - 4,
                    &format!(
                        "{} Email     {}",
                        if self.playit_field == 0 { ">" } else { " " },
                        self.email
                    ),
                    if self.playit_field == 0 {
                        Color::Green
                    } else {
                        Color::White
                    },
                )?;
                paint(
                    out,
                    x + 2,
                    y + 9,
                    w - 4,
                    &format!(
                        "{} Password  {}",
                        if self.playit_field == 1 { ">" } else { " " },
                        "•".repeat(self.password.chars().count())
                    ),
                    if self.playit_field == 1 {
                        Color::Green
                    } else {
                        Color::White
                    },
                )?;
                paint(
                    out,
                    x + 2,
                    y + 11,
                    w - 4,
                    "Accounts requiring a second factor need browser setup instead.",
                    Color::Yellow,
                )?;
            }
            let prompt = match input {
                Input::CurseForge => format!("API key: {}", "•".repeat(self.draft.chars().count())),
                Input::Playit => "Tab/↑↓ Select field   Enter Next/Set up   Esc Cancel".into(),
                Input::ConfirmReset(_) => format!("Type RESET AGENT: {}", self.draft),
                Input::ConfirmUninstall => format!("Type UNINSTALL MSC 2: {}", self.draft),
            };
            paint(out, x + 2, y + h - 6, w - 4, &prompt, Color::Green)?;
        }
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "↑↓ Select   Enter Act   Tab/1–2 Sections   r Refresh   Esc Close",
            Color::DarkGrey,
        )?;
        if !self.message.is_empty() {
            paint(out, x + 2, y + h - 3, w - 4, &self.message, Color::DarkGrey)?;
        }
        Ok(())
    }
}

fn draw_row(
    out: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    selected: bool,
    label: &str,
) -> io::Result<()> {
    paint(
        out,
        x,
        y,
        width,
        &format!("{} {label}", if selected { ">" } else { " " }),
        if selected { Color::Green } else { Color::White },
    )
}
