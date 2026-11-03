//! Edits the original server's agent-owned notes record.
use super::{Snapshot, box_outline, paint};
use crate::cli::transport::SharedClient;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::Color,
};
use serde_json::{Value, json};
use std::io::{self, Stdout};
pub(super) struct Editor {
    id: String,
    name: String,
    text: String,
    message: String,
}
impl Editor {
    pub fn new(server: &msc_api::dto::ServerDto) -> Self {
        Self {
            id: server.id.clone(),
            name: server.name.clone(),
            text: server.notes.clone(),
            message: String::new(),
        }
    }
    pub async fn handle(&mut self, key: KeyEvent, snapshot: &mut Snapshot) -> bool {
        match key.code {
            KeyCode::Esc => return true,
            KeyCode::F(2) | KeyCode::Char('s')
                if key.code == KeyCode::F(2) || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let result = async {
                    let client = SharedClient::connect_local().await?;
                    client
                        .post_json::<_, Value>(
                            "/v1/servers/notes",
                            &json!({"serverId":self.id,"notes":self.text}),
                        )
                        .await
                }
                .await;
                match result {
                    Ok(_) => {
                        if let Some(server) = snapshot
                            .servers
                            .as_mut()
                            .and_then(|servers| servers.iter_mut().find(|s| s.id == self.id))
                        {
                            server.notes = self.text.clone();
                        }
                        return true;
                    }
                    Err(e) => self.message = e.to_string(),
                }
            }
            KeyCode::Enter => self.text.push('\n'),
            KeyCode::Backspace => {
                self.text.pop();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && !c.is_control() =>
            {
                self.text.push(c)
            }
            _ => {}
        }
        false
    }
    pub fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        let w = width.saturating_sub(20);
        let h = height.saturating_sub(8);
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(
            out,
            x + 2,
            y + 1,
            w - 4,
            &format!("NOTES — {}", self.name),
            Color::White,
        )?;
        let columns = w.saturating_sub(4).max(1) as usize;
        let mut lines = Vec::new();
        for line in self.text.split('\n') {
            let chars: Vec<_> = line.chars().collect();
            if chars.is_empty() {
                lines.push(String::new());
            } else {
                lines.extend(chars.chunks(columns).map(|c| c.iter().collect::<String>()));
            }
        }
        let visible = h.saturating_sub(7) as usize;
        let start = lines.len().saturating_sub(visible);
        for (i, line) in lines.iter().skip(start).enumerate() {
            paint(out, x + 2, y + 3 + i as u16, w - 4, line, Color::White)?;
        }
        paint(out, x + 2, y + h - 3, w - 4, &self.message, Color::Yellow)?;
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            "Type to edit · Enter New line · Backspace Delete · Ctrl+S/F2 Save · Esc Cancel",
            Color::Green,
        )
    }
}
