//! Transfer UI delegates archive handling to the existing local CLI workflow.
use super::{box_outline, paint};
use crossterm::{event::KeyCode, style::Color};
use std::io::{self, Stdout};
#[derive(Default)]
pub(super) struct TransferUi {
    pub open: bool,
    selected: usize,
    export: bool,
    chosen: bool,
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
        if self.editing {
            let field = if self.selected == 2 {
                &mut self.backup
            } else {
                &mut self.path
            };
            match key {
                KeyCode::Enter | KeyCode::Esc => self.editing = false,
                KeyCode::Backspace => {
                    field.pop();
                }
                KeyCode::Char(c) if !c.is_control() => field.push(c),
                _ => {}
            }
            return;
        }
        if !self.chosen {
            match key {
                KeyCode::Esc => self.open = false,
                KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                    self.selected = 1 - self.selected.min(1)
                }
                KeyCode::Char('e') => self.choose(true),
                KeyCode::Char('i') => self.choose(false),
                KeyCode::Enter => self.choose(self.selected == 0),
                _ => {}
            }
            return;
        }
        let fields: &[usize] = if self.export {
            &[0, 3]
        } else if self.replace {
            &[0, 1, 2, 3]
        } else {
            &[0, 1, 3]
        };
        let index = fields
            .iter()
            .position(|field| *field == self.selected)
            .unwrap_or(0);
        match key {
            KeyCode::Esc => {
                self.chosen = false;
                self.selected = 0;
                self.message.clear();
            }
            KeyCode::Up | KeyCode::BackTab => self.selected = fields[index.saturating_sub(1)],
            KeyCode::Down | KeyCode::Tab => self.selected = fields[(index + 1) % fields.len()],
            KeyCode::Enter => match self.selected {
                0 | 2 => self.editing = true,
                1 => self.replace = !self.replace,
                _ => {
                    if self.path.trim().is_empty() {
                        self.message = "Enter the transfer file path first.".into();
                    } else if !self.export && self.replace && self.backup.trim().is_empty() {
                        self.message =
                            "Enter a new file path for the backup of your current servers.".into();
                    } else {
                        self.confirm = true;
                    }
                }
            },
            _ => {}
        }
    }
    fn choose(&mut self, export: bool) {
        self.chosen = true;
        self.export = export;
        self.selected = 0;
        self.path.clear();
        self.backup.clear();
        self.replace = false;
        self.message.clear();
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
        let marker = |field| if self.selected == field { ">" } else { " " };
        let mut lines = vec![(1, "SERVER TRANSFER".to_owned())];
        if !self.chosen {
            lines.extend([
                (3, "Move servers between MSC installations".into()),
                (
                    4,
                    "Bundles settings, worlds, backups, mods, plugins and configuration.".into(),
                ),
                (5, "Machine-specific credentials stay behind.".into()),
                (8, format!("{} Export transfer file [e]", marker(0))),
                (10, format!("{} Import transfer file [i]", marker(1))),
            ]);
        } else if self.export {
            lines.extend([
                (3, "EXPORT TRANSFER FILE".into()),
                (
                    5,
                    "Save all configured servers into one .msctransfer file.".into(),
                ),
                (
                    7,
                    "Enter a new file's full path, including its filename.".into(),
                ),
                (
                    8,
                    "Example on macOS: /Users/yourname/Desktop/servers.msctransfer".into(),
                ),
                (10, format!("{} Save to: {}", marker(0), self.path)),
                (14, format!("{} Export", marker(3))),
            ]);
        } else {
            lines.extend([
                (3, "IMPORT TRANSFER FILE".into()),
                (
                    5,
                    "Enter the full path of an existing .msctransfer file.".into(),
                ),
                (
                    6,
                    "Example on macOS: /Users/yourname/Downloads/servers.msctransfer".into(),
                ),
                (8, format!("{} Import from: {}", marker(0), self.path)),
                (
                    10,
                    format!(
                        "{} Mode: {}",
                        marker(1),
                        if self.replace { "Replace all" } else { "Merge" }
                    ),
                ),
                (
                    11,
                    if self.replace {
                        "Replaces your current server set after saving a backup."
                    } else {
                        "Adds servers to this installation; keeps the existing server set."
                    }
                    .into(),
                ),
                (16, format!("{} Import", marker(3))),
            ]);
            if self.replace {
                lines.extend([
                (13, format!("{} Back up current servers to: {}",marker(2),self.backup)),
                (14, "Enter a new backup filename, e.g. /Users/yourname/Desktop/before-import.msctransfer".into()),
            ]);
            }
        }
        for (row, text) in lines {
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
                "↑↓/Tab Select · Enter Edit/Continue · Esc Back"
            },
            Color::Green,
        )
    }
}
