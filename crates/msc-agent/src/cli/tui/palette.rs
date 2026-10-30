//! Guided commands mirror the desktop console registry; sending is explicit.
use super::{Snapshot, box_outline, paint};
use crate::cli::transport::SharedClient;
use crossterm::{event::KeyCode, style::Color};
use serde_json::{Value, json};
use std::io::{self, Stdout};
#[derive(Default)]
pub(super) struct Palette {
    pub open: bool,
    defs: Vec<Value>,
    selected: usize,
    category: usize,
    query: String,
    command: Option<Value>,
    values: Vec<String>,
    field: usize,
    preview: Option<String>,
    preset: Option<String>,
    server: String,
    players: Vec<String>,
    caps: Value,
    message: String,
}
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
const CATEGORIES: &[&str] = &[
    "All",
    "Players",
    "World",
    "Server Admin",
    "Game Rules",
    "Creative",
];
impl Palette {
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
        self.players = snapshot.online_names.clone().unwrap_or_default();
        // Snapshot copied from desktop console/model.ts MINECRAFT_COMMANDS.
        self.defs = serde_json::from_str::<Vec<Value>>(include_str!("command_catalog.json"))
            .unwrap_or_default()
            .into_iter()
            .filter(|d| {
                d[if server.server_type.eq_ignore_ascii_case("bedrock") {
                    "supportsBedrock"
                } else {
                    "supportsJava"
                }] == true
            })
            .collect();
        if let Ok(client) = SharedClient::connect_local().await
            && let Ok(caps) = client.get_json::<Value>("/v1/capabilities").await
        {
            self.caps = caps;
        }
    }
    fn rows(&self) -> Vec<Value> {
        let mut rows = Vec::new();
        for preset in ["dawn", "dusk", "night"] {
            if self.caps["worldSettings"]["relativeTime"]["available"] == true
                && self.caps["worldSettings"]["relativeTime"]["presets"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v == preset))
            {
                rows.push(json!({"name":preset,"description":"Set time within the current Minecraft day","category":"Same-day time","preset":preset}));
            }
        }
        rows.extend(
            self.defs
                .iter()
                .filter(|d| {
                    (self.category == 0 || text(d, "category") == CATEGORIES[self.category])
                        && format!("{} {}", text(d, "name"), text(d, "description"))
                            .to_lowercase()
                            .contains(&self.query.to_lowercase())
                })
                .cloned(),
        );
        rows
    }
    pub async fn handle(&mut self, key: KeyCode) {
        if let Some(preview) = self.preview.clone() {
            match key {
                KeyCode::Esc => {
                    self.preview = None;
                    self.preset = None;
                }
                KeyCode::Enter => {
                    let result=async{let client=SharedClient::connect_local().await?;let (path,body)=if let Some(preset)=&self.preset{("/v1/time/relative",json!({"preset":preset,"expectedActiveServerId":self.server}))}else{("/v1/command",json!({"command":preview.trim_start_matches('/'),"expectedActiveServerId":self.server}))};client.post_json::<_,Value>(path,&body).await}.await;
                    self.message = match result {
                        Ok(_) => {
                            self.preview = None;
                            "Command sent. Check the console for its result.".into()
                        }
                        Err(e) => format!("Command not sent: {e}"),
                    };
                }
                _ => {}
            }
            return;
        }
        if let Some(def) = &self.command {
            let slots = def["argumentSlots"].as_array().cloned().unwrap_or_default();
            let count = slots.len();
            match key {
                KeyCode::Esc => self.command = None,
                KeyCode::Tab | KeyCode::Down => self.field = (self.field + 1) % (count + 1),
                KeyCode::BackTab | KeyCode::Up => self.field = self.field.saturating_sub(1),
                KeyCode::Backspace if self.field < count => {
                    self.values[self.field].pop();
                }
                KeyCode::Char(c) if self.field < count && !c.is_control() => {
                    self.values[self.field].push(c)
                }
                KeyCode::Right if self.field < count => {
                    let slot = &slots[self.field];
                    let options: Vec<String> = if text(slot, "kind") == "player" {
                        self.players.clone()
                    } else {
                        slot["options"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(Value::as_str)
                                    .map(String::from)
                                    .collect()
                            })
                            .unwrap_or_default()
                    };
                    if !options.is_empty() {
                        let next = options
                            .iter()
                            .position(|s| s == &self.values[self.field])
                            .map_or(0, |i| (i + 1) % options.len());
                        self.values[self.field] = options[next].clone();
                    }
                }
                KeyCode::Enter if self.field < count => self.field += 1,
                KeyCode::Enter => {
                    if slots.iter().zip(&self.values).any(|(slot, v)| {
                        !text(slot, "label").contains("optional") && v.trim().is_empty()
                    }) {
                        self.message = "Fill each required argument before previewing.".into();
                    } else {
                        self.preview = Some(format!(
                            "/{} {}",
                            text(def, "name"),
                            self.values
                                .iter()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join(" ")
                        ));
                    }
                }
                _ => {}
            }
            return;
        }
        let rows = self.rows();
        match key {
            KeyCode::Esc => self.open = false,
            KeyCode::Tab => {
                self.category = (self.category + 1) % CATEGORIES.len();
                self.selected = 0;
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(rows.len().saturating_sub(1)),
            KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.query.push(c);
                self.selected = 0;
            }
            KeyCode::Enter => {
                if let Some(def) = rows.get(self.selected) {
                    if let Some(preset) = def["preset"].as_str() {
                        self.preset = Some(preset.into());
                        self.preview = Some(format!("Set {preset} within the current day"));
                    } else {
                        self.values = vec![
                            String::new();
                            def["argumentSlots"].as_array().map_or(0, Vec::len)
                        ];
                        self.field = 0;
                        if self.values.is_empty() {
                            self.preview = Some(format!("/{}", text(def, "name")));
                        } else {
                            self.command = Some(def.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    pub fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let w = width.saturating_sub(24);
        let h = height.saturating_sub(8);
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(out, x + 2, y + 1, w - 4, "COMMAND PALETTE", Color::White)?;
        let mut lines = Vec::new();
        if let Some(preview) = &self.preview {
            lines
                .push("COMMAND PREVIEW — Enter sends to the selected server; Esc goes back".into());
            lines.push(preview.clone());
        } else if let Some(def) = &self.command {
            lines.push(format!(
                "/{} — {}",
                text(def, "name"),
                text(def, "description")
            ));
            if text(def, "name") == "time" {
                lines.push("Numeric time set values are absolute. Same-day shortcuts preserve the Minecraft day.".into());
            }
            for (i, slot) in def["argumentSlots"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                lines.push(format!(
                    "{} {}: {}",
                    if i == self.field { ">" } else { " " },
                    text(slot, "label"),
                    self.values[i]
                ));
                let options = slot["options"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                if !options.is_empty() {
                    lines.push(format!("  Options (Right cycles): {options}"));
                }
            }
            lines.push(format!(
                "{} Preview command",
                if self.field == self.values.len() {
                    ">"
                } else {
                    " "
                }
            ));
        } else {
            lines.push(format!(
                "Search: {} | Category: {} (Tab changes)",
                self.query, CATEGORIES[self.category]
            ));
            for (i, def) in self.rows().iter().enumerate() {
                lines.push(format!(
                    "{} {} /{} — {}",
                    if i == self.selected { ">" } else { " " },
                    text(def, "category"),
                    text(def, "name"),
                    text(def, "description")
                ));
            }
        }
        let visible = h.saturating_sub(8) as usize;
        let start = if self.command.is_none() && self.preview.is_none() {
            self.selected.saturating_sub(visible.saturating_sub(2))
        } else {
            self.field.saturating_sub(visible.saturating_sub(2))
        };
        for (i, line) in lines.iter().skip(start).take(visible).enumerate() {
            paint(out, x + 2, y + 3 + i as u16, w - 4, line, Color::White)?;
        }
        paint(out, x + 2, y + h - 3, w - 4, &self.message, Color::Yellow)?;
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "↑↓ Select · Type Search/Argument · Tab Next/category · Right Suggestion · Enter Continue/Send · Esc Back",
            Color::Green,
        )
    }
}
