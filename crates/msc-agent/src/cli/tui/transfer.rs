//! Transfer UI delegates archive handling to the existing local CLI workflow.
use super::{box_outline, paint};
use crossterm::{event::KeyCode, style::Color};
use std::io::{self, Stdout};
#[derive(Default)]
pub(super) struct TransferUi {
    pub open: bool,
    selected: usize,
    export: bool,
    editing: bool,
    confirm: bool,
    path: String,
    backup: String,
    replace: bool,
    message: String,
}
impl TransferUi {
    pub async fn handle(&mut self, key: KeyCode) {
        if self.confirm {
            match key {
                KeyCode::Esc => self.confirm = false,
                KeyCode::Char('y') => {
                    self.confirm = false;
                    self.run().await;
                }
                _ => {}
            }
            return;
        }
        match key {
            KeyCode::Esc => {
                if self.editing {
                    self.editing = false;
                } else {
                    self.open = false;
                }
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => self.selected = (self.selected + 1) % 4,
            KeyCode::Backspace if self.editing => {
                if self.selected == 2 {
                    self.backup.pop();
                } else {
                    self.path.pop();
                }
            }
            KeyCode::Char(c) if self.editing => {
                if self.selected == 2 {
                    self.backup.push(c);
                } else {
                    self.path.push(c);
                }
            }
            KeyCode::Char('e') => {
                self.export = true;
                self.path = "MinecraftServers.msctransfer".into();
                self.editing = true;
                self.selected = 0;
            }
            KeyCode::Char('i') => {
                self.export = false;
                self.path.clear();
                self.editing = true;
                self.selected = 0;
            }
            KeyCode::Enter if self.editing => self.editing = false,
            KeyCode::Enter => match self.selected {
                0 => self.editing = true,
                1 => self.replace = !self.replace,
                2 => self.editing = true,
                _ => {
                    if self.path.trim().is_empty() {
                        self.message = "Enter an archive path first.".into();
                    } else if !self.export && self.replace && self.backup.trim().is_empty() {
                        self.message = "Replace requires a backup path.".into();
                    } else {
                        self.confirm = true;
                    }
                }
            },
            _ => {}
        }
    }
    async fn run(&mut self) {
        let result = async {
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let mut command = tokio::process::Command::new(executable);
            command.arg("server");
            if self.export {
                command.args(["export", "--output", self.path.trim()]);
            } else {
                command.args([
                    "import",
                    self.path.trim(),
                    "--kind",
                    "transfer",
                    "--transfer-mode",
                    if self.replace { "replaceAll" } else { "merge" },
                ]);
                if self.replace {
                    command.args(["--backup-path", self.backup.trim()]);
                }
            }
            let output = command
                .arg("--json")
                .stdin(std::process::Stdio::null())
                .output()
                .await
                .map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        }
        .await;
        self.message = match result {
            Ok(s) => s,
            Err(s) => format!("Transfer failed: {s}"),
        };
    }
    pub fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let w = width.saturating_sub(24);
        let h = 24.min(height.saturating_sub(4));
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        for (row, text) in [
            (1, "SERVER TRANSFER".into()),
            (3, "Move servers between MSC installations".into()),
            (
                4,
                "Bundles settings, worlds, backups, mods, plugins and configuration.".into(),
            ),
            (5, "Machine-specific credentials stay behind.".into()),
            (7, "e Export transfer file    i Import transfer file".into()),
            (
                9,
                format!(
                    "{} Path: {}",
                    if self.selected == 0 { ">" } else { " " },
                    self.path
                ),
            ),
            (
                11,
                format!(
                    "{} Import mode: {}",
                    if self.selected == 1 { ">" } else { " " },
                    if self.replace {
                        "Replace all (backup required)"
                    } else {
                        "Merge"
                    }
                ),
            ),
            (
                13,
                format!(
                    "{} Backup path: {}",
                    if self.selected == 2 { ">" } else { " " },
                    self.backup
                ),
            ),
            (
                15,
                format!(
                    "{} {}",
                    if self.selected == 3 { ">" } else { " " },
                    if self.export { "Export" } else { "Import" }
                ),
            ),
        ] {
            paint(out, x + 2, y + row, w - 4, &text, Color::White)?;
        }
        paint(
            out,
            x + 2,
            y + h - 5,
            w - 4,
            &self.message.replace('\n', " "),
            Color::Yellow,
        )?;
        paint(
            out,
            x + 2,
            y + h - 3,
            w - 4,
            if self.confirm {
                "Confirm transfer: y Continue · Esc Cancel"
            } else if self.editing {
                "Type path · Enter Finish editing · Esc Cancel"
            } else {
                "↑↓ Select · Enter Edit/Act · e Export · i Import · Esc Close"
            },
            Color::Green,
        )
    }
}
