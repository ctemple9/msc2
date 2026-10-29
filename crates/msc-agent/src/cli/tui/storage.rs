//! Host-local storage navigation using the existing inventory and review owners.
use super::paint;
use crate::cli::transport::SharedClient;
use crossterm::{event::KeyCode, style::Color};
use msc_api::dto::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, Stdout},
};

#[derive(Default)]
pub(super) struct StorageUi {
    inventory: Option<StorageInventoryDto>,
    path: Vec<String>,
    selected: usize,
    operation: Option<(String, bool)>,
    candidates: Option<StorageCandidatesDto>,
    chosen: Vec<String>,
    review: Option<StorageReviewDto>,
    confirmation: String,
    message: String,
    results: Vec<String>,
}
impl StorageUi {
    pub async fn load(&mut self) {
        let result = async {
            let client = SharedClient::connect_local().await?;
            let mut inventory: StorageInventoryDto =
                client.get_json("/v1/host/storage/inventory").await?;
            while let Some(cursor) = inventory.next_cursor.take() {
                let page: StorageInventoryDto = client
                    .get_json(&format!("/v1/host/storage/inventory?cursor={cursor}"))
                    .await?;
                if page.snapshot.as_ref().map(|s| &s.id)
                    != inventory.snapshot.as_ref().map(|s| &s.id)
                {
                    return Err(crate::cli::CliError::usage(
                        "Inventory changed; refresh again.",
                    ));
                }
                if let (Some(target), Some(source)) = (&mut inventory.snapshot, page.snapshot) {
                    target.items.extend(source.items);
                }
                inventory.next_cursor = page.next_cursor;
            }
            Ok(inventory)
        }
        .await;
        match result {
            Ok(value) => self.inventory = Some(value),
            Err(e) => self.message = e.to_string(),
        }
    }
    fn items(&self) -> Vec<&StorageItemDto> {
        self.inventory
            .as_ref()
            .and_then(|i| i.snapshot.as_ref())
            .map(|s| {
                s.items
                    .iter()
                    .filter(|i| {
                        self.path.get(1).is_none_or(|c| &i.category == c)
                            && self.path.get(2).is_none_or(|server| {
                                i.server_id.as_deref().unwrap_or("unassigned") == server
                            })
                            && self.path.get(3).is_none_or(|g| &i.group == g)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn rows(&self) -> Vec<(String, String, u64)> {
        if self.path.is_empty() {
            return vec![(
                "computer".into(),
                "This computer".into(),
                self.inventory
                    .as_ref()
                    .and_then(|i| i.snapshot.as_ref())
                    .map_or(0, |s| s.logical_bytes),
            )];
        }
        let items = self.items();
        let grouped = self.path.len() == 1
            || (self.path.get(1).is_some_and(|s| s == "servers") && self.path.len() < 4);
        let mut rows = if grouped {
            let mut groups: BTreeMap<String, (String, u64)> = BTreeMap::new();
            for item in items {
                let key = match self.path.len() {
                    1 => item.category.clone(),
                    2 => item.server_id.clone().unwrap_or("unassigned".into()),
                    _ => item.group.clone(),
                };
                let label = if self.path.len() == 2 {
                    item.label
                        .split(" / ")
                        .next()
                        .unwrap_or(&item.label)
                        .to_owned()
                } else {
                    label(&key).into()
                };
                let entry = groups.entry(key).or_insert((label, 0));
                entry.1 = entry.1.saturating_add(item.logical_bytes);
            }
            let mut rows: Vec<_> = groups
                .into_iter()
                .map(|(id, (name, size))| (id, name, size))
                .collect();
            rows.sort_by_key(|row| std::cmp::Reverse(row.2));
            rows
        } else {
            items
                .into_iter()
                .map(|i| (i.id.clone(), i.label.clone(), i.logical_bytes))
                .collect()
        };
        for (key, title) in match self.path.get(1).map(String::as_str) {
            Some("servers") => vec![
                ('t', "Remove a server…"),
                ('w', "Remove a saved world…"),
                ('b', "Manage backups…"),
            ],
            Some("cache") => vec![
                ('m', "Review map cache removal…"),
                ('d', "Review old downloads…"),
            ],
            Some("agent") => vec![('a', "Review obsolete builds…")],
            _ => vec![],
        } {
            rows.push((format!("@{key}"), title.into(), 0));
        }
        rows
    }
    pub async fn poll(&mut self) {
        let Some((id, cleanup)) = self.operation.clone() else {
            return;
        };
        let result = async {
            let c = SharedClient::connect_local().await?;
            c.get_json::<Value>(&format!("/v1/operations/{id}")).await
        }
        .await;
        match result {
            Ok(v) => {
                self.message = v["statusLine"].as_str().unwrap_or("Working…").into();
                if matches!(
                    v["state"].as_str(),
                    Some("succeeded" | "failed" | "cancelled")
                ) {
                    self.operation = None;
                    if cleanup {
                        if let Ok(c) = SharedClient::connect_local().await {
                            match c
                                .get_json::<StorageReviewedResultDto>(&format!(
                                    "/v1/host/storage/cleanups/{id}"
                                ))
                                .await
                            {
                                Ok(result) => {
                                    self.results = result
                                        .items
                                        .iter()
                                        .map(|item| {
                                            format!(
                                                "{}: {} — {}",
                                                item.label, item.state, item.message
                                            )
                                        })
                                        .collect();
                                    self.message = result
                                        .items
                                        .iter()
                                        .map(|i| {
                                            format!("{}: {} — {}", i.label, i.state, i.message)
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" | ")
                                }
                                Err(e) => self.message = e.to_string(),
                            }
                        }
                    }
                    self.load().await;
                }
            }
            Err(e) => self.message = e.to_string(),
        }
    }
    pub async fn handle(&mut self, key: KeyCode) -> bool {
        if let Some(review) = self.review.clone() {
            match key {
                KeyCode::Esc => {
                    self.review = None;
                    self.confirmation.clear();
                }
                KeyCode::Backspace => {
                    self.confirmation.pop();
                }
                KeyCode::Char(c) => self.confirmation.push(c),
                KeyCode::Enter if self.confirmation == review.confirmation => {
                    let result = async { let c = SharedClient::connect_local().await?; c.post_json::<_,Value>("/v1/host/storage/cleanups", &json!({"hostId":review.host_id,"previewToken":review.preview_token,"confirmation":self.confirmation})).await }.await;
                    match result {
                        Ok(v) => {
                            self.operation = v["operationId"].as_str().map(|id| (id.into(), true));
                            self.review = None;
                            self.candidates = None;
                        }
                        Err(e) => {
                            self.message = e.to_string();
                            self.review = None;
                        }
                    }
                }
                _ => {}
            }
            return true;
        }
        if key == KeyCode::Esc {
            if !self.results.is_empty() {
                self.results.clear();
                return true;
            }
            if self.candidates.take().is_some() {
                self.chosen.clear();
                return true;
            }
            if self.path.pop().is_some() {
                self.selected = 0;

                return true;
            }
            return false;
        }
        let rows = self.rows();
        let count = self
            .candidates
            .as_ref()
            .map_or(rows.len(), |c| c.items.len());
        match key {
            KeyCode::Char('o') => {
                if let Some((id, _, _)) = rows.get(self.selected) {
                    if let Some(item) = self.items().into_iter().find(|item| &item.id == id) {
                        if let Some(path) = &item.display_path {
                            let path = std::path::PathBuf::from(path);
                            match std::fs::symlink_metadata(&path) {
                                Ok(metadata) if !metadata.file_type().is_symlink() => {
                                    let folder = if metadata.is_dir() {
                                        path.as_path()
                                    } else {
                                        path.parent().unwrap_or(&path)
                                    };
                                    let program = if cfg!(target_os = "macos") {
                                        "open"
                                    } else if cfg!(windows) {
                                        "explorer"
                                    } else {
                                        "xdg-open"
                                    };
                                    match tokio::process::Command::new(program)
                                        .arg(folder)
                                        .stdin(std::process::Stdio::null())
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn()
                                    {
                                        Ok(mut child) => {
                                            tokio::spawn(async move {
                                                let _ = child.wait().await;
                                            });
                                            self.message =
                                                "Folder open requested on this computer.".into();
                                        }
                                        Err(error) => self.message = error.to_string(),
                                    }
                                }
                                _ => {
                                    self.message =
                                        "Folder unavailable or changed; refresh inventory.".into()
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(count.saturating_sub(1)),
            KeyCode::Char('r') => {
                let result = async {
                    let c = SharedClient::connect_local().await?;
                    c.post_json::<_, Value>("/v1/host/storage/scans", &json!({}))
                        .await
                }
                .await;
                match result {
                    Ok(s) => {
                        self.operation = s["operationId"].as_str().map(|id| (id.to_owned(), false));
                        self.message = "Storage scan started…".into();
                    }
                    Err(e) => self.message = e.to_string(),
                }
            }
            KeyCode::Char('x') => {
                if let Some((id, _)) = &self.operation {
                    if let Ok(c) = SharedClient::connect_local().await {
                        if let Err(e) = c
                            .post_json::<_, Value>(
                                &format!("/v1/operations/{id}/cancel"),
                                &json!({}),
                            )
                            .await
                        {
                            self.message = e.to_string();
                        }
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self
                    .candidates
                    .as_ref()
                    .and_then(|c| c.items.get(self.selected))
                {
                    if item.removable {
                        if self.chosen.contains(&item.id) {
                            self.chosen.retain(|id| id != &item.id)
                        } else {
                            self.chosen.push(item.id.clone());
                        }
                    }
                }
            }
            KeyCode::Enter if self.candidates.is_some() => {
                if let Some(candidates) = &self.candidates {
                    if !self.chosen.is_empty() {
                        let result=async {let c=SharedClient::connect_local().await?;c.post_json::<_,StorageReviewDto>("/v1/host/storage/reviews",&json!({"hostId":candidates.host_id,"category":candidates.category,"action":candidates.action,"itemIds":self.chosen})).await}.await;
                        match result {
                            Ok(r) => {
                                self.review = Some(r);
                                self.confirmation.clear();
                            }
                            Err(e) => self.message = e.to_string(),
                        }
                    }
                }
            }
            KeyCode::Enter => {
                if let Some((id, _, _)) = rows.get(self.selected) {
                    if let Some(action) = id.strip_prefix('@').and_then(|s| s.chars().next()) {
                        return Box::pin(self.handle(KeyCode::Char(action))).await;
                    }
                    if self.path.len() < 2
                        || (self.path.get(1).is_some_and(|s| s == "servers") && self.path.len() < 4)
                    {
                        self.path.push(id.clone());
                        self.selected = 0;
                    } else if let Some(item) = self.items().iter().find(|i| &i.id == id) {
                        self.message = format!(
                            "{} · {} · {}",
                            item.display_path.as_deref().unwrap_or("Path unavailable"),
                            item.protection,
                            if item.partial {
                                "Partial inventory"
                            } else {
                                "Measured"
                            }
                        );
                    }
                }
            }
            KeyCode::Char(c @ ('m' | 'd' | 'w' | 'b' | 'a' | 't')) => {
                let (category, action) = match c {
                    'm' => ("cache", "map-cache"),
                    'd' => ("cache", "old-downloads"),
                    'w' => ("servers", "remove-world"),
                    'b' => ("servers", "delete-backups"),
                    'a' => ("agent", "obsolete-builds"),
                    _ => ("servers", "remove-server"),
                };
                let result = async {
                    let client = SharedClient::connect_local().await?;
                    client
                        .get_json::<StorageCandidatesDto>(&format!(
                            "/v1/host/storage/candidates?category={category}&action={action}"
                        ))
                        .await
                }
                .await;
                match result {
                    Ok(mut c) => {
                        if let Some(server) = self.path.get(2) {
                            c.items.retain(|i| i.server_id.as_ref() == Some(server));
                        }
                        self.candidates = Some(c);
                        self.chosen.clear();
                        self.selected = 0;
                    }
                    Err(e) => self.message = e.to_string(),
                }
            }
            _ => {}
        }
        true
    }
    pub fn draw(&self, out: &mut Stdout, x: u16, y: u16, w: u16, h: u16) -> io::Result<()> {
        paint(
            out,
            x,
            y,
            w,
            &format!(
                "STORAGE / {}",
                self.path
                    .iter()
                    .map(|s| label(s))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
            Color::White,
        )?;
        let summary = self
            .inventory
            .as_ref()
            .and_then(|i| i.snapshot.as_ref())
            .map(|s| {
                format!(
                    "{}{} logical · scanned {} · {} coverage",
                    if s.partial { "At least " } else { "" },
                    size(s.logical_bytes),
                    s.completed_at,
                    if s.partial { "Partial" } else { "Complete" }
                )
            })
            .unwrap_or("Not scanned · r Refresh".into());
        paint(out, x, y + 1, w, &summary, Color::DarkGrey)?;
        let mut lines: Vec<String> = Vec::new();
        if !self.results.is_empty() {
            lines.push("CLEANUP RESULT · Logical bytes removed are not disk space freed".into());
            lines.extend(self.results.clone());
        } else if let Some(review) = &self.review {
            lines.push(format!(
                "REVIEW {} · expires in {} seconds",
                review.action, review.expires_in_seconds
            ));
            for item in &review.items {
                lines.push(item.label.clone());
                lines.extend(item.consequences.clone());
            }
            lines.push(format!("Type exactly: {}", review.confirmation));
            lines.push(self.confirmation.clone());
        } else if let Some(c) = &self.candidates {
            for (n, item) in c.items.iter().enumerate() {
                lines.push(format!(
                    "{} [{}] {} · {}",
                    if n == self.selected { ">" } else { " " },
                    if self.chosen.contains(&item.id) {
                        "x"
                    } else {
                        " "
                    },
                    item.label,
                    item.reason
                ));
            }
        } else {
            for (n, (id, name, bytes)) in self.rows().iter().enumerate() {
                lines.push(format!(
                    "{} {:<32} {}",
                    if n == self.selected { ">" } else { " " },
                    name,
                    if id.starts_with('@') {
                        String::new()
                    } else {
                        size(*bytes)
                    }
                ));
            }
            if self.path.len() == 1 {
                if let Some(s) = self.inventory.as_ref().and_then(|i| i.snapshot.as_ref()) {
                    for v in &s.volumes {
                        lines.push(format!(
                            "{} · {} available",
                            v.label,
                            v.available_bytes.map(size).unwrap_or("Unknown".into())
                        ));
                    }
                    for issue in &s.issues {
                        lines.push(format!("Coverage: {}", issue.reason));
                    }
                }
            }
        }
        let visible = h.saturating_sub(7) as usize;
        let start = self.selected.saturating_sub(visible.saturating_sub(1));
        for (n, line) in lines.iter().skip(start).take(visible).enumerate() {
            paint(out, x, y + 3 + n as u16, w, line, Color::White)?;
        }
        paint(out, x, y + h - 3, w, &self.message, Color::Yellow)?;
        paint(
            out,
            x,
            y + h - 2,
            w,
            "↑↓ Select Enter Open/review Space Select r Scan x Cancel work o Open folder Esc Back",
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            y + h - 1,
            w,
            "Reviews: m Maps d Downloads a Old builds t Server w World b Backups",
            Color::DarkGrey,
        )
    }
}
fn label(key: &str) -> &str {
    match key {
        "computer" => "This computer",
        "servers" => "Servers",
        "desktopApp" => "Desktop app",
        "agent" => "Agent",
        "sharedTools" => "Shared tools & runtimes",
        "cache" => "Cache & temporary files",
        "worlds" => "Worlds",
        "backups" => "Backups",
        "components" => "Mods & server files",
        "other" => "Logs & other files",
        _ => key,
    }
}
fn size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.1} GiB", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.1} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else {
        format!("{bytes} B")
    }
}
