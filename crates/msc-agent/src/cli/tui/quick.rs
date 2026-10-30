//! Sidebar shortcuts use the same command/profile owners as the desktop.
use super::{Snapshot, box_outline, paint};
use crate::cli::transport::SharedClient;
use crossterm::{event::KeyCode, style::Color};
use serde_json::{Value, json};
use std::io::{self, Stdout};
#[derive(Default)]
pub(super) struct Quick {
    pub open: bool,
    selected: usize,
    difficulty: usize,
    gamemode: usize,
    force: bool,
    force_available: bool,
    server: String,
    slot: Option<String>,
    bedrock: bool,
    caps: Value,
    pending: bool,
    safety: Option<String>,
    message: String,
}
const DIFFICULTIES: &[&str] = &["peaceful", "easy", "normal", "hard"];
const MODES: &[&str] = &["survival", "creative", "adventure", "spectator"];
impl Quick {
    pub async fn open(&mut self, snapshot: &Snapshot) {
        *self = Self::default();
        self.open = true;
        let Some(server) = snapshot.servers.as_ref().and_then(|servers| {
            servers
                .iter()
                .find(|s| Some(&s.id) == snapshot.active_server_id.as_ref())
        }) else {
            self.message = "Select a server first.".into();
            return;
        };
        self.server = server.id.clone();
        self.bedrock = server.server_type.eq_ignore_ascii_case("bedrock");
        if let Some(slot) = snapshot
            .worlds
            .as_ref()
            .and_then(|worlds| worlds.slots.iter().find(|s| s.is_active))
        {
            self.slot = Some(slot.id.clone());
            let profile = match SharedClient::connect_local().await {
                Ok(client) => match client
                    .get_json::<Value>(&format!("/v1/worlds/{}/profile", slot.id))
                    .await
                {
                    Ok(value) => value["profile"].clone(),
                    Err(error) => {
                        self.message = error.to_string();
                        self.slot = None;
                        Value::Null
                    }
                },
                Err(error) => {
                    self.message = error.to_string();
                    self.slot = None;
                    Value::Null
                }
            };
            self.difficulty = DIFFICULTIES
                .iter()
                .position(|s| Some(*s) == profile["gameplay"]["difficulty"].as_str())
                .unwrap_or(2);
            self.gamemode = MODES
                .iter()
                .position(|s| Some(*s) == profile["gameplay"]["defaultGameMode"].as_str())
                .unwrap_or(0);
        }
        if let Some(field) = snapshot.settings.as_ref().and_then(|s| {
            s.sections
                .iter()
                .flat_map(|s| &s.fields)
                .find(|f| f.key == "force-gamemode")
        }) {
            self.force_available = true;
            self.force = field.value == "true";
        }
        if let Ok(c) = SharedClient::connect_local().await {
            match c.get_json::<Value>("/v1/capabilities").await {
                Ok(v) => self.caps = v,
                Err(e) => self.message = e.to_string(),
            }
        }
    }
    fn labels(&self) -> Vec<String> {
        let mut rows = vec![
            "Dawn (same day)".into(),
            "Dusk (same day)".into(),
            "Night (same day)".into(),
            "Weather: Clear".into(),
            "Weather: Rain".into(),
            "Weather: Storm".into(),
            format!("Difficulty: {}", DIFFICULTIES[self.difficulty]),
            format!("Default gamemode: {}", MODES[self.gamemode]),
            format!(
                "Enforce gamemode: {}",
                if self.force { "On" } else { "Off" }
            ),
            "Save all".into(),
        ];
        if !self.bedrock {
            rows.push("Reload".into());
        }
        rows
    }
    pub async fn handle(&mut self, key: KeyCode) {
        if self.pending {
            match key {
                KeyCode::Esc => {
                    self.pending = false;
                    self.safety = None;
                }
                KeyCode::Char('y') => {
                    self.pending = false;
                    self.apply().await;
                }
                _ => {}
            }
            return;
        }
        match key {
            KeyCode::Esc => self.open = false,
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(self.labels().len() - 1),
            KeyCode::Left | KeyCode::Right => {
                let forward = key == KeyCode::Right;
                match self.selected {
                    6 => self.difficulty = (self.difficulty + if forward { 1 } else { 3 }) % 4,
                    7 => self.gamemode = (self.gamemode + if forward { 1 } else { 3 }) % 4,
                    8 => self.force = !self.force,
                    _ => {}
                }
            }
            KeyCode::Enter => {
                self.safety = None;
                self.message = format!(
                    "Apply {}? y Confirm · Esc Cancel",
                    self.labels()[self.selected]
                );
                self.pending = true;
            }
            _ => {}
        }
    }
    async fn apply(&mut self) {
        let result=async {
 let c=SharedClient::connect_local().await?;let status=c.get_json::<Value>("/v1/status").await?;if status["activeServerId"].as_str()!=Some(self.server.as_str()){return Err(crate::cli::CliError::usage("Selected server changed. Reopen Quick Commands."));}
 if self.selected!=8 && status["running"]!=true{return Err(crate::cli::CliError::usage("Start the server before using this shortcut."));}
 let (path,mut body)=match self.selected{
 0..=2=>{let preset=["dawn","dusk","night"][self.selected];if self.caps["worldSettings"]["relativeTime"]["available"]!=true||!self.caps["worldSettings"]["relativeTime"]["presets"].as_array().is_some_and(|a|a.iter().any(|p|p==preset)){return Err(crate::cli::CliError::usage("This server does not advertise this same-day time preset."));}("/v1/time/relative".into(),json!({"preset":preset}))},
 3..=5=>("/v1/command".into(),json!({"command":format!("weather {}",["clear","rain","thunder"][self.selected-3]),"expectedActiveServerId":self.server})),
 6|7=>{let slot=self.slot.as_ref().ok_or_else(||crate::cli::CliError::usage("Select an active world before changing its settings."))?;let (key,value)=if self.selected==6{("gameplay.difficulty",DIFFICULTIES[self.difficulty])}else{("gameplay.default-game-mode",MODES[self.gamemode])};(format!("/v1/worlds/{slot}/profile"),json!({"changes":{key:value}}))},
 8=>{if !self.force_available{return Err(crate::cli::CliError::usage("Enforce gamemode is unavailable on this server."));}("/v1/settings".into(),json!({"changes":{"force-gamemode":self.force.to_string()}}))},
 9 if self.bedrock=>{c.post_json::<_,Value>("/v1/command",&json!({"command":"save hold","expectedActiveServerId":self.server})).await?;("/v1/command".into(),json!({"command":"save resume","expectedActiveServerId":self.server}))},
 _=>("/v1/command".into(),json!({"command":if self.selected==9{"save-all"}else{"reload"},"expectedActiveServerId":self.server}))};
 if let Some(token)=&self.safety{body["confirmation"]=json!(token);}c.post_json::<_,Value>(&path,&body).await
 }.await;
        match result {
            Ok(value) => {
                self.safety = None;
                self.message = if value["success"] == false {
                    format!("Settings were not applied: {value}")
                } else {
                    format!(
                        "Requested {}. {}",
                        self.labels()[self.selected],
                        value["status"]
                            .as_str()
                            .unwrap_or("Check the console for the result.")
                    )
                };
            }
            Err(e) => {
                if let Some((token, explanation)) = e.settings_confirmation() {
                    self.safety = Some(token);
                    self.pending = true;
                    self.message = format!("{explanation} · y Confirm · Esc Cancel");
                } else {
                    self.message = e.to_string();
                    self.safety = None;
                }
            }
        }
    }
    pub fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let w = width.saturating_sub(34);
        let h = height.saturating_sub(8);
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(out, x + 2, y + 1, w - 4, "QUICK COMMANDS", Color::White)?;
        paint(
            out,
            x + 2,
            y + 3,
            w - 4,
            "Time/weather affect this session. Difficulty/default gamemode save to the active world.",
            Color::DarkGrey,
        )?;
        for (i, label) in self
            .labels()
            .iter()
            .enumerate()
            .take(h.saturating_sub(10) as usize)
        {
            paint(
                out,
                x + 2,
                y + 5 + i as u16,
                w - 4,
                &format!("{} {label}", if i == self.selected { ">" } else { " " }),
                if i == self.selected {
                    Color::Green
                } else {
                    Color::White
                },
            )?;
        }
        paint(out, x + 2, y + h - 3, w - 4, &self.message, Color::Yellow)?;
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "↑↓ Select · ←→ Change value · Enter Review · y Confirm · Esc Close/cancel",
            Color::DarkGrey,
        )
    }
}
