//! Handbook and interactive router guidance backed by the desktop's help APIs.
use super::{box_outline, paint};
use crate::cli::transport::SharedClient;
use crossterm::{event::KeyCode, style::Color};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{self, Stdout},
};
#[derive(Default)]
pub(super) struct Guides {
    pub open: bool,
    router: bool,
    related: bool,
    step_selected: usize,
    intro_expanded: bool,
    notes_expanded: bool,
    catalog: Value,
    routers: Value,
    article: Value,
    analysis: Value,
    search: Option<Value>,
    query: String,
    typing: bool,
    selected: usize,
    scroll: usize,
    reader: bool,
    symptoms: bool,
    checked: BTreeSet<String>,
    done: BTreeSet<String>,
    favorites: BTreeSet<String>,
    history: Vec<Value>,
    message: String,
}
const TOPICS: &[&str] = &[
    "handbook.overview",
    "handbook.networking-basics",
    "handbook.ram-performance",
    "handbook.standard-vs-modded",
    "handbook.paper",
    "handbook.vanilla",
    "handbook.purpur",
    "handbook.jars-java",
    "handbook.eula-online-mode",
    "handbook.plugins-crossplay",
    "handbook.fabric",
    "handbook.neoforge",
    "handbook.forge",
    "handbook.mods-browser",
    "handbook.client-requirements",
    "handbook.bedrock",
    "handbook.how-bedrock-runs",
    "handbook.port-forwarding-duckdns",
    "handbook.playit",
    "handbook.tailscale",
    "handbook.xbox-broadcast",
    "handbook.remote-access",
    "handbook.worlds-backups",
    "handbook.world-conversion",
    "handbook.server-transfer",
    "handbook.server-files",
    "handbook.watchdog",
    "handbook.player-management",
    "handbook.first-server",
    "handbook.first-modded-server",
    "handbook.first-bedrock-server",
];
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
fn array(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
impl Guides {
    pub async fn open(&mut self) {
        self.open = true;
        self.intro_expanded = true;
        if self.catalog.is_null() {
            self.favorites = [
                "handbook.first-server",
                "handbook.first-modded-server",
                "handbook.first-bedrock-server",
            ]
            .into_iter()
            .map(String::from)
            .collect();
        }
        if let Ok(bytes) = msc_infrastructure::storage::read_small(&favorites_path(), 65536)
            && let Ok(saved) = serde_json::from_slice::<BTreeSet<String>>(&bytes)
        {
            self.favorites = saved;
        }
        let result = async {
            let c = SharedClient::connect_local().await?;
            let (a, b) = tokio::join!(
                c.get_json::<Value>("/v1/help/catalog"),
                c.get_json::<Value>("/v1/guides/router-catalog")
            );
            Ok::<_, crate::cli::CliError>((a?, b?))
        }
        .await;
        match result {
            Ok((a, b)) => {
                self.catalog = a;
                self.routers = b;
            }
            Err(e) => self.message = e.to_string(),
        }
    }
    fn rows(&self) -> Vec<Value> {
        if self.router {
            if self.symptoms {
                return array(&self.routers["symptoms"]);
            }
            if let Some(search) = &self.search {
                let mut rows = Vec::new();
                if let Some(f) = search
                    .get("suggestedFallbackGuide")
                    .filter(|v| !v.is_null())
                {
                    rows.push(f.clone());
                }
                for c in array(&search["candidates"]) {
                    if !rows.iter().any(|v| v["id"] == c["guide"]["id"]) {
                        rows.push(c["guide"].clone());
                    }
                }
                return rows;
            }
            let mut rows = array(&self.routers["guides"]);
            rows.sort_by_key(|v| {
                (
                    match text(v, "category") {
                        "generic_fallback" => 0,
                        "isp_gateway" => 1,
                        "retail_router" => 2,
                        "mesh_system" => 3,
                        _ => 4,
                    },
                    text(v, "displayName").to_owned(),
                )
            });
            return rows;
        }
        let mut rows: Vec<_> = array(&self.catalog["topics"])
            .into_iter()
            .filter(|v| {
                text(v, "helpId").starts_with("handbook.")
                    && (!self.related
                        || array(&self.article["relatedIds"])
                            .iter()
                            .any(|id| id.as_str() == Some(text(v, "helpId")))
                        || self
                            .article
                            .to_string()
                            .contains(&format!("help:{}", text(v, "helpId"))))
                    && text(v, "title")
                        .to_lowercase()
                        .contains(&self.query.to_lowercase())
            })
            .collect();
        rows.sort_by_key(|v| {
            (
                !self.favorites.contains(text(v, "helpId")),
                TOPICS
                    .iter()
                    .position(|id| *id == text(v, "helpId"))
                    .unwrap_or(usize::MAX),
                text(v, "title").to_owned(),
            )
        });
        rows
    }
    async fn read(&mut self, id: &str) {
        let path = if self.router {
            format!("/v1/guides/router/{}", encode(id))
        } else {
            format!("/v1/help/{}", encode(id))
        };
        let result = async {
            let c = SharedClient::connect_local().await?;
            c.get_json::<Value>(&path).await
        }
        .await;
        match result {
            Ok(v) => {
                if !self.article.is_null() {
                    self.history.push(self.article.clone());
                }
                self.article = v;
                self.analysis = Value::Null;
                self.scroll = 0;
                self.reader = true;
                self.done.clear();
                self.step_selected = 0;
                self.intro_expanded = true;
                self.notes_expanded = false;
                self.message.clear();
            }
            Err(e) => {
                self.message =
                    format!("Could not load guide: {e}. Router values require a selected server.")
            }
        }
    }
    pub async fn handle(&mut self, key: KeyCode) {
        if self.typing {
            match key {
                KeyCode::Esc => self.typing = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                KeyCode::Enter => {
                    self.typing = false;
                    self.selected = 0;
                    if self.router {
                        let result = async {
                            let c = SharedClient::connect_local().await?;
                            c.get_json::<Value>(&format!(
                                "/v1/guides/router/search?q={}",
                                encode(&self.query)
                            ))
                            .await
                        }
                        .await;
                        match result {
                            Ok(v) => {
                                self.message = if v["isAmbiguous"] == true {
                                    "Closest available guides"
                                } else if v["matchedDirectGuide"] == false {
                                    "Showing closest available fallback"
                                } else {
                                    "Search results"
                                }
                                .into();
                                self.search = Some(v);
                            }
                            Err(e) => self.message = e.to_string(),
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        let rows = self.rows();
        match key {
            KeyCode::Esc => {
                if self.reader {
                    self.reader = false;
                } else if self.symptoms {
                    self.symptoms = false;
                    self.analysis = Value::Null;
                } else if !self.query.is_empty() {
                    self.query.clear();
                    self.search = None;
                    self.selected = 0;
                } else {
                    self.open = false;
                }
            }
            KeyCode::Char('1' | '2') => {
                self.router = key == KeyCode::Char('2');
                self.related = false;
                self.symptoms = false;
                self.reader = false;
                self.selected = 0;
                self.scroll = 0;
                self.article = Value::Null;
                self.analysis = Value::Null;
                self.search = None;
                self.query.clear();
                self.history.clear();
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Left => self.reader = !self.reader,
            KeyCode::Char('/') => {
                self.typing = true;
                self.reader = false;
            }
            KeyCode::Up => {
                if self.reader {
                    self.scroll = self.scroll.saturating_sub(1)
                } else {
                    self.selected = self.selected.saturating_sub(1)
                }
            }
            KeyCode::Down => {
                if self.reader {
                    self.scroll += 1;
                } else {
                    self.selected = (self.selected + 1).min(rows.len().saturating_sub(1));
                }
            }
            KeyCode::PageDown => self.scroll += 12,
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(12),
            KeyCode::Home => self.scroll = 0,
            KeyCode::Char('l') if !self.router => {
                self.related = !self.related;
                self.selected = 0;
                self.reader = false;
            }
            KeyCode::Char('i') if self.router && self.reader => {
                self.intro_expanded = !self.intro_expanded
            }
            KeyCode::Char('n') if self.router && self.reader => {
                self.notes_expanded = !self.notes_expanded
            }
            KeyCode::Char('[' | ']') if !self.router => {
                let id = text(&self.article, "helpId");
                if let Some(index) = TOPICS.iter().position(|s| *s == id) {
                    let next = if key == KeyCode::Char('[') {
                        index.saturating_sub(1)
                    } else {
                        (index + 1).min(TOPICS.len() - 1)
                    };
                    self.read(TOPICS[next]).await;
                }
            }
            KeyCode::Char('b') => {
                if let Some(v) = self.history.pop() {
                    self.article = v;
                    self.scroll = 0;
                    self.reader = true;
                }
            }
            KeyCode::Char('f') if !self.router => {
                if let Some(v) = rows.get(self.selected) {
                    let id = text(v, "helpId");
                    if !self.favorites.remove(id) {
                        self.favorites.insert(id.into());
                    }
                    let path = favorites_path();
                    let saved = (|| -> Result<(), String> {
                        std::fs::create_dir_all(path.parent().ok_or("Missing preferences folder")?)
                            .map_err(|e| e.to_string())?;
                        let bytes =
                            serde_json::to_vec(&self.favorites).map_err(|e| e.to_string())?;
                        msc_infrastructure::atomic_write::atomic_write(
                            &msc_infrastructure::fs::StdFileSystem,
                            &path,
                            &bytes,
                        )
                        .map_err(|e| e.to_string())
                    })();
                    if let Err(error) = saved {
                        self.message = format!("Favorites kept for this session: {error}");
                    }
                }
            }
            KeyCode::Char('t') if self.router => {
                self.symptoms = true;
                self.reader = false;
                self.selected = 0;
                self.scroll = 0;
            }
            KeyCode::Char('r') if self.symptoms => {
                self.checked.clear();
                self.analysis = Value::Null;
                self.scroll = 0;
            }
            KeyCode::Char('a') if self.symptoms => {
                if self.checked.is_empty() {
                    self.message = "Select symptoms with Space first.".into();
                } else {
                    let result = async {
                        let c = SharedClient::connect_local().await?;
                        c.post_json::<_, Value>(
                            "/v1/guides/router/troubleshooting/analyze",
                            &json!({"symptoms":self.checked}),
                        )
                        .await
                    }
                    .await;
                    match result {
                        Ok(v) => {
                            self.analysis = v;
                            self.reader = true;
                            self.scroll = 0;
                        }
                        Err(e) => self.message = e.to_string(),
                    }
                }
            }
            KeyCode::Char('g') if self.router => {
                let id = text(&self.analysis["fallbackResolution"], "matchedGuideId");
                let id = if id.is_empty() {
                    text(&self.analysis["fallbackResolution"], "fallbackGuideId")
                } else {
                    id
                };
                let id = if id.is_empty() {
                    "advanced-troubleshooting"
                } else {
                    id
                }
                .to_owned();
                self.symptoms = false;
                self.read(&id).await;
            }
            KeyCode::Char(' ') if self.symptoms && !self.reader => {
                if let Some(v) = rows.get(self.selected) {
                    let id = text(v, "id");
                    if !self.checked.remove(id) {
                        self.checked.insert(id.into());
                    }
                    self.analysis = Value::Null;
                }
            }
            KeyCode::Char(' ') if self.router && self.reader => {
                let steps = self.steps();
                if let Some(step) = steps.get(self.step_selected) {
                    let id = text(step, "id");
                    if !self.done.remove(id) {
                        self.done.insert(id.into());
                    }
                }
            }
            KeyCode::Char('j') if self.router && self.reader => {
                self.step_selected =
                    (self.step_selected + 1).min(self.steps().len().saturating_sub(1))
            }
            KeyCode::Char('k') if self.router && self.reader => {
                self.step_selected = self.step_selected.saturating_sub(1)
            }
            KeyCode::Char('x') if self.router && self.reader => {
                let steps = self.steps();
                if steps.iter().all(|s| self.done.contains(text(s, "id"))) {
                    self.done.clear();
                } else {
                    self.done = steps.iter().map(|s| text(s, "id").into()).collect();
                }
            }
            KeyCode::Enter => {
                if let Some(v) = rows.get(self.selected) {
                    if self.symptoms {
                        self.message = text(v, "description").into();
                    } else {
                        let id = text(v, if self.router { "id" } else { "helpId" }).to_owned();
                        self.read(&id).await;
                    }
                }
            }
            _ => {}
        }
    }
    fn steps(&self) -> Vec<Value> {
        array(&self.article["sections"])
            .iter()
            .flat_map(|s| array(&s["items"]))
            .filter(|i| text(i, "type") == "step")
            .collect()
    }
    pub fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let x = 3;
        let y = 3;
        let w = width.saturating_sub(6);
        let h = height.saturating_sub(6);
        for row in y..y + h {
            paint(out, x, row, w, &" ".repeat(w as usize), Color::Black)?;
        }
        box_outline(out, x, y, w, h)?;
        paint(
            out,
            x + 2,
            y + 1,
            w - 4,
            "GUIDES   1 Handbook   2 Router Guide",
            Color::White,
        )?;
        paint(
            out,
            x + 2,
            y + 2,
            w - 4,
            &format!(
                "{} | {}",
                if self.router {
                    if self.symptoms {
                        "3/3 Diagnose issues"
                    } else if self.article.is_null() {
                        "1/3 Find your router"
                    } else {
                        "2/3 Follow the steps"
                    }
                } else {
                    "Handbook — topics left, content right"
                },
                if self.reader {
                    "Reader focus"
                } else {
                    "Contents focus"
                }
            ),
            Color::Green,
        )?;
        paint(
            out,
            x + 2,
            y + 3,
            w - 4,
            &format!(
                "Search{}: {}",
                if self.typing { " (typing)" } else { " [/]" },
                self.query
            ),
            Color::White,
        )?;
        let sidebar = (w / 3).clamp(24, 42);
        let visible = h.saturating_sub(10) as usize;
        let rows = self.rows();
        let start = self.selected.saturating_sub(visible.saturating_sub(1));
        for (i, v) in rows.iter().skip(start).take(visible).enumerate() {
            let name = if self.symptoms {
                text(v, "title")
            } else if self.router {
                text(v, "displayName")
            } else {
                text(v, "title")
            };
            let id = text(v, "helpId");
            let mark = if self.symptoms {
                if self.checked.contains(text(v, "id")) {
                    "[x]"
                } else {
                    "[ ]"
                }
            } else if self.favorites.contains(id) {
                "*"
            } else {
                ""
            };
            paint(
                out,
                x + 2,
                y + 5 + i as u16,
                sidebar - 3,
                &format!(
                    "{} {mark} {name}",
                    if start + i == self.selected && !self.reader {
                        ">"
                    } else {
                        " "
                    }
                ),
                Color::White,
            )?;
        }
        let mut lines = Vec::new();
        if self.symptoms && !self.analysis.is_null() {
            render(&self.analysis, &mut lines);
        } else if !self.article.is_null() {
            if self.router {
                lines.push(text(&self.article["guide"], "displayName").into());
                lines.push(format!(
                    "Source confidence: {}",
                    text(&self.article["guide"]["review"], "sourceConfidence")
                ));
                let runtime = &self.article["runtime"];
                lines.push(format!(
                    "Server: {} | IP: {}",
                    text(runtime, "selectedServerName"),
                    text(runtime, "detectedLocalIpAddress")
                ));
                for section in array(&self.article["sections"]) {
                    lines.push(text(&section, "title").into());
                    if (text(&section, "kind") == "intro" && !self.intro_expanded)
                        || (text(&section, "kind") == "notes" && !self.notes_expanded)
                    {
                        lines.push("[Collapsed]".into());
                    } else {
                        render(&section["items"], &mut lines);
                    }
                }
                for token in array(&self.article["unresolvedTokens"]) {
                    lines.push(format!("Value unavailable: {}. Find this value manually before configuring your router.",text(&token,"token")));
                }
                lines.push("Firmware layouts and menu labels may differ.".into());
            } else {
                render(&self.article, &mut lines);
            }
            if self.router {
                lines.push("STEP CHECKLIST — j/k select, Space toggle, x all/reset".into());
                for (i, step) in self.steps().iter().enumerate() {
                    lines.push(format!(
                        "{} [{}] {}",
                        if i == self.step_selected { ">" } else { " " },
                        if self.done.contains(text(step, "id")) {
                            "x"
                        } else {
                            " "
                        },
                        text(step, "title")
                    ));
                }
            }
        } else {
            lines.push(
                if self.router {
                    "Select a router, or choose the generic guide if you don't know it."
                } else {
                    "Start with a favorite (*), search a topic, or select a handbook chapter."
                }
                .into(),
            );
        }
        let cols = w.saturating_sub(sidebar + 5) as usize;
        let wrapped: Vec<String> = lines.iter().flat_map(|s| wrap(s, cols)).collect();
        let offset = self.scroll.min(wrapped.len().saturating_sub(visible));
        for (i, line) in wrapped.iter().skip(offset).take(visible).enumerate() {
            paint(
                out,
                x + sidebar + 1,
                y + 5 + i as u16,
                w - sidebar - 3,
                line,
                Color::White,
            )?;
        }
        paint(out, x + 2, y + h - 4, w - 4, &self.message, Color::Yellow)?;
        paint(
            out,
            x + 2,
            y + h - 3,
            w - 4,
            "↑↓ Move/scroll  Enter Read  Tab Focus  PgUp/PgDn Scroll  / Search  b Back  f Favorite  l Related  [/] Chapter  Esc Back/Close",
            Color::DarkGrey,
        )?;
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            if self.router {
                "t Symptoms  Space Check  a Analyze  r Reset  g Suggested guide  j/k Step  x All/reset  i Intro  n Notes"
            } else {
                "f Favorite  l Related topics  [ Previous chapter  ] Next chapter  2 Router Guide"
            },
            Color::DarkGrey,
        )
    }
}
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if line.chars().count() + word.chars().count() + 1 > width && !line.is_empty() {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push(line);
    out
}
fn render(v: &Value, out: &mut Vec<String>) {
    if let Some(s) = v.as_str() {
        out.extend(s.lines().map(String::from));
        return;
    }
    if let Some(a) = v.as_array() {
        for x in a {
            render(x, out);
        }
        return;
    }
    if !v.is_object() {
        return;
    }
    if text(v, "type") == "table" {
        out.push(
            array(&v["headers"])
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" | "),
        );
        for row in array(&v["rows"]) {
            out.push(
                array(&row)
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
        }
        out.push(String::new());
        return;
    }
    for key in ["title", "displayName", "subtitle", "analogy", "phase"] {
        if let Some(s) = v[key].as_str() {
            out.push(s.into());
        }
    }
    for key in ["summary", "body", "markdown", "text", "description"] {
        if let Some(s) = v[key].as_str() {
            out.extend(s.lines().map(String::from));
        }
    }
    if let Some(guide) = v.get("guide") {
        render(guide, out);
    }
    if let Some(runtime) = v.get("runtime") {
        out.push(format!(
            "Server: {} | Local IP: {} | Java: {} | Bedrock: {}",
            text(runtime, "selectedServerName"),
            text(runtime, "detectedLocalIpAddress"),
            runtime["javaPort"],
            runtime["bedrockPort"]
        ));
    }
    for key in [
        "headers",
        "rows",
        "bullets",
        "items",
        "steps",
        "sections",
        "path",
        "alternateMenuNames",
        "alternateTerms",
        "suggestedNextActions",
        "recommendedActions",
        "escalationBullets",
        "unresolvedTokens",
    ] {
        if let Some(value) = v.get(key) {
            if !value.is_null() {
                render(value, out);
            }
        }
    }
    for cause in array(&v["likelyCauses"]) {
        out.push(format!(
            "{} / {}",
            text(&cause, "severity"),
            text(&cause, "confidence")
        ));
        render(&cause["topic"], out);
    }
    if let Some(f) = v.get("fallbackResolution") {
        render(&f["explanationBullets"], out);
    }
    out.push(String::new());
}

fn favorites_path() -> std::path::PathBuf {
    msc_infrastructure::config_repository::default_app_data_dir()
        .join("tui")
        .join("handbook-favorites.json")
}
