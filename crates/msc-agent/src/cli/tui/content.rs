//! Host-local content workflows use the same reviewed API as desktop.
//! Provider data is rendered as text, never interpreted as terminal control.
use super::{box_outline, paint};
use crate::cli::{
    CliError, encode_uri_component as enc, stage_file_upload, transport::SharedClient,
};
use crossterm::{event::KeyCode, style::Color};
use msc_api::dto::StagedUploadPurposeDto;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, Stdout},
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Debug)]
pub(super) enum Selection {
    Individual {
        provider: String,
        project: Value,
        version: Value,
        incompatible: bool,
    },
    Pack {
        upload: String,
        inspection: Value,
        current_setup: bool,
    },
    Remove(usize),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Catalog,
    Versions,
    Server,
    Players,
    Sources,
    Updates,
    Retained,
    Review,
    History,
    Publication,
    Worlds,
    WorldPacks,
    Form,
    Confirm,
    PackSetup,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum FormKind {
    Search,
    PlayerFile,
    PlayerOptions,
    Manual,
    Recovery,
    Pack,
    Publication,
    Export,
    Key,
    Source,
}
#[derive(Clone)]
enum FieldKind {
    Text,
    Secret,
    Choice(Vec<&'static str>),
}
#[derive(Clone)]
struct Field {
    key: String,
    label: String,
    value: String,
    kind: FieldKind,
}
#[derive(Clone)]
struct Frame {
    page: Page,
    title: String,
    data: Value,
    context: Value,
    fields: Vec<Field>,
    choices: BTreeMap<String, bool>,
    selected: usize,
}
impl Frame {
    fn new(page: Page, title: impl Into<String>, data: Value, context: Value) -> Self {
        Self {
            page,
            title: title.into(),
            data,
            context,
            fields: vec![],
            choices: BTreeMap::new(),
            selected: 0,
        }
    }
    fn field(&mut self, key: &str, label: &str, value: &str, kind: FieldKind) {
        self.fields.push(Field {
            key: key.into(),
            label: label.into(),
            value: value.into(),
            kind,
        });
    }
    fn value(&self, key: &str) -> String {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.value.clone())
            .unwrap_or_default()
    }
    fn yes(&self, key: &str) -> bool {
        self.value(key) == "Yes"
    }
}
#[derive(Clone)]
enum Action {
    Menu(&'static str),
    Form(FormKind, Value),
    Edit(usize),
    Submit(FormKind),
    Project(Value),
    Release(Value),
    Navigate(Page, String, Value),
    Player(Value),
    Apply(String, String),
    Confirm(Box<Action>),
    Toggle(String),
    RefreshReview,
    Manual(Value),
    Source(Value, Value),
    Update(Value),
    Publication(String),
    Back,
    Home,
    SelectPack(bool),
    RemovePick(usize),
    None,
}
#[derive(Clone)]
struct Row {
    label: String,
    action: Action,
}
fn row(label: impl Into<String>, action: Action) -> Row {
    Row {
        label: label.into(),
        action,
    }
}
fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").into()
}
fn array(value: &Value, key: &str) -> Vec<Value> {
    value[key].as_array().cloned().unwrap_or_default()
}
fn objects(value: &Value, key: &str) -> Vec<Value> {
    value[key]
        .as_object()
        .map(|map| map.values().cloned().collect())
        .unwrap_or_default()
}
fn clean(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}
fn version(value: &Value) -> &Value {
    value
        .get("version")
        .filter(|value| value.is_object())
        .unwrap_or(value)
}
fn issue_lines(value: &Value) -> Vec<String> {
    array(value, "issues")
        .iter()
        .map(|item| format!("{}: {}", text(item, "path"), text(item, "reason")))
        .chain(
            array(value, "warnings")
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned)),
        )
        .collect()
}
fn root(context: &Value) -> String {
    format!("/v1/servers/{}/content", enc(&text(context, "serverId")))
}
fn selected_keys(frame: &Frame, prefix: &str) -> Vec<String> {
    frame
        .choices
        .iter()
        .filter(|(_, checked)| **checked)
        .filter_map(|(key, _)| key.strip_prefix(prefix).map(str::to_owned))
        .collect()
}
#[derive(Default)]
struct Progress {
    line: String,
    operation: Option<String>,
}
type SharedProgress = Arc<Mutex<Progress>>;
struct Work {
    task: tokio::task::JoinHandle<Result<Frame, CliError>>,
    progress: SharedProgress,
}
#[derive(Default)]
pub(super) struct ContentUi {
    pub open: bool,
    frame: Option<Frame>,
    history: Vec<Frame>,
    editing: Option<usize>,
    message: String,
    work: Option<Work>,
    selection: Option<Selection>,
    creation: bool,
    refresh: bool,
}
async fn guard(client: &SharedClient, context: &Value) -> Result<(), CliError> {
    let id = text(context, "serverId");
    if id.is_empty() {
        return Ok(());
    }
    let status: Value = client.get_json("/v1/status").await?;
    if text(&status, "activeServerId") != id {
        return Err(CliError::usage(
            "Selected server changed. Return to the server workspace and reopen content.",
        ));
    }
    Ok(())
}
async fn operation(
    client: &SharedClient,
    path: &str,
    body: &Value,
    progress: &SharedProgress,
) -> Result<Value, CliError> {
    let accepted: Value = client.post_json(path, body).await?;
    let Some(id) = accepted["operationId"].as_str() else {
        return Ok(accepted);
    };
    progress.lock().unwrap().operation = Some(id.into());
    loop {
        let status: Value = client
            .get_json(&format!("/v1/operations/{}", enc(id)))
            .await?;
        progress.lock().unwrap().line = text(&status, "statusLine");
        match status["state"].as_str() {
            Some("succeeded") => {
                progress.lock().unwrap().operation = None;
                return Ok(status["result"].clone());
            }
            Some("failed" | "cancelled") => {
                return Err(CliError::usage(
                    status["error"]["message"]
                        .as_str()
                        .unwrap_or("Operation cancelled or failed"),
                ));
            }
            _ => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
}
impl ContentUi {
    pub(super) fn open_server(&mut self, id: String, name: String, flavor: String) {
        *self = Self {
            open: true,
            ..Default::default()
        };
        let context = json!({"serverId":id,"serverName":name,"flavor":flavor,"provider":"modrinth","kind":if ["paper","purpur"].contains(&flavor.as_str()){"plugin"}else{"mod"}});
        self.set(Frame::new(
            Page::Home,
            "Content and packs",
            Value::Null,
            context.clone(),
        ));
        self.load_home(context);
    }
    pub(super) fn open_creation(
        &mut self,
        flavor: String,
        game: String,
        loader: String,
        picks: Vec<Selection>,
    ) {
        *self = Self {
            open: true,
            creation: true,
            ..Default::default()
        };
        let context = json!({"serverId":"","serverName":"Server draft","flavor":flavor,"game":game,"loader":loader,"provider":"modrinth","kind":if ["paper","purpur"].contains(&flavor.as_str()){"plugin"}else{"mod"},"creation":true});
        let mut data = json!({"picks":[]});
        data["picks"]=Value::Array(picks.iter().map(|pick|match pick {Selection::Individual{provider,project,version,..}=>json!({"title":format!("{} · {} · {}",text(project,"title"),provider,text(version,"name"))}),_=>json!({"title":"Base pack"})}).collect());
        self.set(Frame::new(
            Page::Home,
            "Content for server draft",
            data,
            context,
        ));
    }
    pub(super) fn take_selection(&mut self) -> Option<Selection> {
        self.selection.take()
    }
    pub(super) fn take_refresh(&mut self) -> bool {
        std::mem::take(&mut self.refresh)
    }
    fn set(&mut self, frame: Frame) {
        self.frame = Some(frame);
        self.editing = None;
    }
    fn push(&mut self, frame: Frame) {
        if let Some(current) = self.frame.take() {
            self.history.push(current);
        }
        self.set(frame);
    }
    fn spawn<F>(&mut self, description: &str, build: F)
    where
        F: FnOnce(
                SharedProgress,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<Frame, CliError>> + Send>,
            > + Send
            + 'static,
    {
        if self.work.is_some() {
            return;
        }
        let progress = Arc::new(Mutex::new(Progress {
            line: description.into(),
            operation: None,
        }));
        let copy = progress.clone();
        self.work = Some(Work {
            task: tokio::spawn(async move { build(copy).await }),
            progress,
        });
        self.message.clear();
    }
    fn load_home(&mut self, context: Value) {
        self.spawn("Reading content capabilities", move |_| {
            Box::pin(async move {
                let client = SharedClient::connect_local().await?;
                guard(&client, &context).await?;
                let caps: Value = client.get_json("/v1/capabilities").await?;
                if !caps["contentCatalog"]["playerContentManagement"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    return Err(CliError::usage("Update this agent for content management."));
                }
                let mut context = context;
                let versions: Value = client.get_json("/v1/versions").await.unwrap_or(Value::Null);
                context["game"] = versions["currentVersion"].clone();
                let data: Value = match client.get_json(&format!("{}/collection", root(&context))).await {
                    Ok(data) => data,
                    Err(error) => json!({"notice":format!("Current content unavailable: {error}. Frozen publication history remains available.")}),
                };
                Ok(Frame::new(Page::Home, "Content and packs", data, context))
            })
        });
    }
    pub(super) async fn poll(&mut self) {
        if !self
            .work
            .as_ref()
            .is_some_and(|work| work.task.is_finished())
        {
            return;
        }
        let work = self.work.take().unwrap();
        match work.task.await {
            Ok(Ok(frame)) => self.set(frame),
            Ok(Err(error)) => {
                self.message = error.to_string();
                let details = error
                    .json_message
                    .as_deref()
                    .and_then(|body| serde_json::from_str::<Value>(body).ok());
                if let Some(fingerprint) = details
                    .as_ref()
                    .and_then(|body| body["details"]["restorationFingerprint"].as_str())
                    && let Some(mut frame) = self.frame.clone()
                {
                    frame.context["restorationFingerprint"] = json!(fingerprint);
                    frame.context["restoreDatapack"] = json!(true);
                    if frame.page == Page::Form {
                        frame.field(
                            "restore",
                            "Restore the upstream datapack, replacing reviewed owner edits",
                            "No",
                            choice(vec!["No", "Yes"]),
                        );
                    } else {
                        frame.page = Page::Confirm;
                        frame.title =
                            "Restore upstream datapack and replace reviewed owner edits?".into();
                    }
                    self.set(frame);
                }
                if let Some(frame) = &mut self.frame
                    && frame.page == Page::Form
                    && frame.title.contains("key")
                {
                    for field in &mut frame.fields {
                        if matches!(field.kind, FieldKind::Secret) {
                            field.value.clear();
                        }
                    }
                }
            }
            Err(_) => {
                self.message =
                    "Content worker ended without a confirmed result. Refresh before retrying."
                        .into()
            }
        }
    }
    fn navigate(&mut self, page: Page, path: String, context: Value) {
        if let Some(frame) = self.frame.clone() {
            self.history.push(frame);
        }
        self.spawn("Loading content", move |_| {
            Box::pin(async move {
                let client = SharedClient::connect_local().await?;
                guard(&client, &context).await?;
                let data: Value = client.get_json(&path).await?;
                Ok(Frame::new(page, title(page), data, context))
            })
        });
    }
    fn compatible(&self, v: &Value) -> bool {
        let Some(frame) = &self.frame else {
            return false;
        };
        let mc = text(&frame.context, "game");
        let flavor = text(&frame.context, "flavor");
        let kind = text(&frame.context, "kind");
        !mc.is_empty()
            && array(v, "gameVersions")
                .iter()
                .any(|value| value.as_str() == Some(&mc))
            && (kind == "plugin"
                || kind == "datapack"
                || array(v, "loaders")
                    .iter()
                    .any(|value| value.as_str() == Some(&flavor)))
    }
    fn rows(&self) -> Vec<Row> {
        let Some(frame) = &self.frame else {
            return vec![];
        };
        let context = &frame.context;
        let base = root(context);
        match frame.page {
            Page::Home => {
                let mut rows = vec![row(
                    "Browse mods, plugins or packs",
                    Action::Form(FormKind::Search, context.clone()),
                )];
                if let Some(notice) = frame.data["notice"].as_str() {
                    rows.push(row(notice, Action::None));
                }
                if self.creation {
                    rows.extend(array(&frame.data, "picks").iter().enumerate().map(
                        |(index, item)| {
                            row(
                                format!("Remove staged {}", text(item, "title")),
                                Action::RemovePick(index),
                            )
                        },
                    ));
                    return rows;
                }
                rows.extend([
                    row(
                        "Server content and player requirements",
                        Action::Navigate(
                            Page::Server,
                            format!("{base}/collection"),
                            context.clone(),
                        ),
                    ),
                    row(
                        "Player collection",
                        Action::Navigate(
                            Page::Players,
                            format!("{base}/collection"),
                            context.clone(),
                        ),
                    ),
                    row(
                        "Identify exact provider sources",
                        Action::Navigate(
                            Page::Sources,
                            format!("{base}/identify"),
                            context.clone(),
                        ),
                    ),
                    row(
                        "Check individual content updates",
                        Action::Navigate(Page::Updates, format!("{base}/updates"), context.clone()),
                    ),
                    row(
                        "Recover original player content",
                        Action::Form(FormKind::Recovery, context.clone()),
                    ),
                    row(
                        "Update or replace the base pack",
                        Action::Form(FormKind::Pack, context.clone()),
                    ),
                    row(
                        "Publish current content",
                        Action::Form(FormKind::Publication, context.clone()),
                    ),
                    row(
                        "Dated publications and export",
                        Action::Navigate(
                            Page::History,
                            format!("{base}/publications"),
                            context.clone(),
                        ),
                    ),
                    row(
                        "World datapacks",
                        Action::Navigate(Page::Worlds, "/v1/worlds".into(), context.clone()),
                    ),
                ]);
                rows
            }
            Page::Catalog => {
                let mut rows = vec![row(
                    "Change provider, category or query",
                    Action::Form(FormKind::Search, context.clone()),
                )];
                rows.extend(array(&frame.data, "results").into_iter().map(|item| {
                    row(
                        format!("{} — {}", text(&item, "title"), text(&item, "description")),
                        Action::Project(item),
                    )
                }));
                let offset = frame.data["offset"].as_u64().unwrap_or(0);
                let total = frame.data["totalCount"].as_u64().unwrap_or(0);
                if offset > 0 {
                    let mut next = context.clone();
                    next["offset"] = json!(offset.saturating_sub(50));
                    rows.push(row(
                        "Previous results",
                        Action::Form(FormKind::Search, next),
                    ));
                }
                if offset + (array(&frame.data, "results").len() as u64) < total {
                    let mut next = context.clone();
                    next["offset"] = json!(offset + 50);
                    rows.push(row("Next results", Action::Form(FormKind::Search, next)));
                }
                rows
            }
            Page::Versions => {
                let mut rows = Vec::new();
                let detail = frame.data["detail"]
                    .get("project")
                    .unwrap_or(&frame.data["detail"]);
                rows.push(row(text(detail, "description"), Action::None));
                rows.push(row(text(detail, "body"), Action::None));
                rows.push(row(
                    format!(
                        "[{}] Stable releases only",
                        if frame.choices.get("stable") == Some(&true) {
                            "x"
                        } else {
                            " "
                        }
                    ),
                    Action::Toggle("stable".into()),
                ));
                let mut versions = array(&frame.data, "versions");
                versions.sort_by_key(|item| !self.compatible(version(item)));
                for item in versions {
                    let v = version(&item);
                    if frame.choices.get("stable") == Some(&true)
                        && text(v, "versionType") != "release"
                    {
                        continue;
                    }
                    let available = item["available"].as_bool().unwrap_or(true);
                    rows.push(row(
                        format!(
                            "{} · {} · {}{}",
                            text(v, "name"),
                            text(v, "versionType"),
                            if self.compatible(v) {
                                "compatible"
                            } else {
                                "other setup"
                            },
                            if available { "" } else { " · unavailable" }
                        ),
                        if available {
                            Action::Release(item)
                        } else {
                            Action::None
                        },
                    ));
                }
                let offset = context["releaseOffset"].as_u64().unwrap_or(0);
                let total = frame.data["totalCount"].as_u64().unwrap_or(0);
                let length = array(&frame.data, "versions").len() as u64;
                for (label, next) in [
                    ("Previous releases", offset.checked_sub(50)),
                    (
                        "Next releases",
                        (offset + length < total && length > 0).then_some(offset + length),
                    ),
                ] {
                    if let Some(next) = next {
                        let mut context = context.clone();
                        context["releaseOffset"] = json!(next);
                        rows.push(row(label,Action::Navigate(Page::Versions,format!("/v1/catalog/curseforge/projects/{}/versions?kind={}&offset={next}",enc(&text(&context["project"],"projectId")),enc(&text(&context,"kind"))),context)));
                    }
                }
                rows
            }
            Page::Server => {
                let mut rows = vec![];
                for item in array(&frame.data, "requirements") {
                    rows.push(row(
                        format!("{} — {}", text(&item, "path"), text(&item, "reason")),
                        Action::None,
                    ));
                }
                for entry in objects(&frame.data["collection"], "currentFiles") {
                    rows.push(row(
                        format!("{} · {}", text(&entry, "path"), text(&entry, "audience")),
                        Action::Player(entry),
                    ));
                }
                rows
            }
            Page::Players => {
                let mut rows = vec![row(
                    "Add an independent player file",
                    Action::Form(FormKind::PlayerFile, context.clone()),
                )];
                rows.extend(
                    objects(&frame.data["collection"], "playerFiles")
                        .into_iter()
                        .map(|entry| {
                            row(
                                format!(
                                    "{} · {}",
                                    text(&entry, "path"),
                                    if entry["optional"].as_bool() == Some(true) {
                                        "optional"
                                    } else {
                                        "required"
                                    }
                                ),
                                Action::Player(entry),
                            )
                        }),
                );
                rows
            }
            Page::Sources => {
                let mut rows = vec![];
                for item in array(&frame.data, "items") {
                    rows.push(row(
                        format!(
                            "{} — {} {}",
                            text(&item, "path"),
                            text(&item, "status"),
                            text(&item, "note")
                        ),
                        Action::None,
                    ));
                    let mut source_context = context.clone();
                    source_context["sourceEntry"] = item.clone();
                    rows.push(row(
                        "  Review project hint / remove source",
                        Action::Form(FormKind::Source, source_context),
                    ));
                    for candidate in array(&item, "candidates")
                        .into_iter()
                        .filter(|candidate| candidate["provider"] == "curseforge")
                    {
                        rows.push(row(
                            format!(
                                "  Link exact CurseForge project {} / file {}",
                                text(&candidate, "projectId"),
                                text(&candidate, "versionId")
                            ),
                            Action::Source(item.clone(), candidate),
                        ));
                    }
                }
                rows
            }
            Page::Updates => array(&frame.data, "items")
                .into_iter()
                .map(|item| {
                    row(
                        format!(
                            "{} · {} {}",
                            text(&item, "path"),
                            text(&item, "label"),
                            text(&item, "note")
                        ),
                        if item["availableVersionId"].as_str().is_some() {
                            Action::Update(item)
                        } else {
                            Action::None
                        },
                    )
                })
                .collect(),
            Page::Form => {
                let kind = form_kind(&frame.title);
                let mut rows = frame
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        row(
                            format!(
                                "{}: {}",
                                field.label,
                                if matches!(field.kind, FieldKind::Secret) {
                                    "•".repeat(field.value.chars().count())
                                } else {
                                    field.value.clone()
                                }
                            ),
                            Action::Edit(index),
                        )
                    })
                    .collect::<Vec<_>>();
                if kind == FormKind::Key {
                    rows.push(row(
                        "Open CurseForge developer console",
                        Action::Menu("key_console"),
                    ));
                }
                if let Some(notice) = frame.data["notice"].as_str() {
                    rows.push(row(notice, Action::None));
                }
                rows.push(row("Continue with these choices", Action::Submit(kind)));
                rows
            }
            Page::Confirm => vec![
                row("Keep current content / cancel", Action::Back),
                row(
                    "Confirm the selected action",
                    Action::Apply(text(context, "applyPath"), text(context, "reviewId")),
                ),
            ],
            Page::PackSetup => vec![
                row(
                    "Use pack's exact Minecraft and loader setup",
                    Action::SelectPack(false),
                ),
                row(
                    "Try current setup anyway (compatibility is not guaranteed)",
                    Action::SelectPack(true),
                ),
            ],
            _ => self.workflow_rows(frame),
        }
    }
    pub(super) fn handle(&mut self, key: KeyCode) {
        if let Some(work) = &self.work {
            if key == KeyCode::Esc {
                let progress = work.progress.clone();
                let id = progress.lock().unwrap().operation.clone();
                if let Some(id) = id {
                    tokio::spawn(async move {
                        if let Ok(client) = SharedClient::connect_local().await {
                            let _: Result<Value, _> = client
                                .post_json(
                                    &format!("/v1/operations/{}/cancel", enc(&id)),
                                    &json!({}),
                                )
                                .await;
                        }
                    });
                    self.message =
                        "Cancellation requested; waiting for the operation to finish cleanup."
                            .into();
                } else {
                    self.message="Reading or transferring a file; waiting for its admission/result before cancellation.".into();
                }
            }
            return;
        }
        if let Some(index) = self.editing {
            let field = &mut self.frame.as_mut().unwrap().fields[index];
            match key {
                KeyCode::Esc | KeyCode::Enter => self.editing = None,
                KeyCode::Backspace => {
                    field.value.pop();
                }
                KeyCode::Char(ch) if !ch.is_control() && field.value.len() < 2048 => {
                    field.value.push(ch)
                }
                _ => {}
            }
            return;
        }
        let count = self.rows().len();
        let Some(frame) = self.frame.as_mut() else {
            return;
        };
        match key {
            KeyCode::Esc => self.back(),
            KeyCode::Up => frame.selected = frame.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => {
                frame.selected = (frame.selected + 1).min(count.saturating_sub(1))
            }
            KeyCode::Enter => {
                let selected = frame.selected;
                let action = self.rows().get(selected).map(|row| row.action.clone());
                if let Some(action) = action {
                    self.act(action);
                }
            }
            _ => {}
        }
    }
    fn back(&mut self) {
        if let Some(frame) = self.history.pop() {
            self.set(frame);
        } else {
            self.open = false;
        }
        self.message.clear();
    }
    fn act(&mut self, action: Action) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        match action {
            Action::Back => self.back(),
            Action::Home => {
                self.history.clear();
                self.load_home(frame.context);
            }
            Action::Navigate(page, path, context) => self.navigate(page, path, context),
            Action::Form(kind, context) => {
                let form = make_form(kind, context, &frame.data);
                self.push(form);
            }
            Action::Edit(index) => {
                let item = &mut self.frame.as_mut().unwrap().fields[index];
                if let FieldKind::Choice(options) = &item.kind {
                    let old = options
                        .iter()
                        .position(|option| *option == item.value)
                        .unwrap_or(0);
                    item.value = options[(old + 1) % options.len()].into();
                } else {
                    self.editing = Some(index);
                }
            }
            Action::Submit(kind) => self.submit(kind, frame),
            Action::Project(item) => self.project(item, frame.context),
            Action::Release(item) => {
                let incompatible = !self.compatible(version(&item));
                if text(&frame.context, "kind") == "modpack" {
                    self.install(item, true);
                } else if incompatible {
                    let mut confirm =
                        make_form(FormKind::Search, frame.context.clone(), &Value::Null);
                    confirm.title = "Install other setup".into();
                    confirm.page = Page::Confirm;
                    confirm.context["release"] = item;
                    confirm.context["mismatch"] = json!(true);
                    self.push(confirm);
                } else {
                    self.install(item, false);
                }
            }
            Action::Player(entry) => {
                let mut context = frame.context;
                context["entry"] = entry;
                context["serverEntry"] = json!(frame.page == Page::Server);
                self.push(make_form(FormKind::PlayerOptions, context, &frame.data));
            }
            Action::Toggle(key) => {
                let frame = self.frame.as_mut().unwrap();
                let old = frame.choices.get(&key).copied().unwrap_or(false);
                if key != "incomplete" {
                    frame.context["dirty"] = json!(true);
                }
                frame.choices.insert(key, !old);
            }
            Action::RemovePick(index) => {
                self.selection = Some(Selection::Remove(index));
                self.open = false;
            }
            other => self.workflow_action(other, frame),
        }
    }
    fn project(&mut self, item: Value, mut context: Value) {
        context["project"] = item.clone();
        self.history.push(self.frame.clone().unwrap());
        self.spawn("Reading project and releases", move |_| {
            Box::pin(async move {
                let client = SharedClient::connect_local().await?;
                guard(&client, &context).await?;
                let id = enc(&text(&item, "projectId"));
                let cf = text(&context, "provider") == "curseforge";
                let detail: Value = client
                    .get_json(&format!(
                        "/v1/catalog/{}/{}",
                        if cf {
                            "curseforge/projects"
                        } else {
                            "projects"
                        },
                        id
                    ))
                    .await?;
                let versions: Value = client
                    .get_json(&format!(
                        "/v1/catalog/{}/{id}/versions{}",
                        if cf {
                            "curseforge/projects"
                        } else {
                            "projects"
                        },
                        if cf {
                            format!(
                                "?kind={}&offset={}",
                                enc(&text(&context, "kind")),
                                context["releaseOffset"].as_u64().unwrap_or(0)
                            )
                        } else {
                            String::new()
                        }
                    ))
                    .await?;
                let mut data = versions;
                data["detail"] = detail;
                Ok(Frame::new(
                    Page::Versions,
                    text(&item, "title"),
                    data,
                    context,
                ))
            })
        });
    }
    fn install(&mut self, item: Value, mismatch: bool) {
        let mut frame = self.frame.clone().unwrap();
        frame.context["release"] = item.clone();
        if let Some(current) = &mut self.frame {
            current.context["release"] = item.clone();
        }
        if self.creation && text(&frame.context, "kind") != "modpack" {
            self.selection = Some(Selection::Individual {
                provider: text(&frame.context, "provider"),
                project: frame.context["project"].clone(),
                version: version(&item).clone(),
                incompatible: mismatch,
            });
            self.open = false;
            return;
        }
        self.spawn("Resolving exact content", move |progress| {
            Box::pin(async move { install_selected(frame, mismatch, progress).await })
        });
    }
    pub(super) fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        if !self.open {
            return Ok(());
        }
        let x = 2;
        let y = 2;
        let w = width.saturating_sub(4);
        let h = height.saturating_sub(4);
        for line in y..y + h {
            paint(out, x, line, w, &" ".repeat(w as usize), Color::White)?;
        }
        box_outline(out, x, y, w, h)?;
        let Some(frame) = &self.frame else {
            return Ok(());
        };
        paint(out, x + 2, y + 1, w - 4, &clean(&frame.title), Color::White)?;
        paint(
            out,
            x + 2,
            y + 2,
            w - 4,
            &format!(
                "{} · {} {}",
                text(&frame.context, "serverName"),
                text(&frame.context, "game"),
                text(&frame.context, "flavor")
            ),
            Color::DarkGrey,
        )?;
        let rows = self.rows();
        let visible = h.saturating_sub(9) as usize;
        let start = frame.selected.saturating_sub(visible.saturating_sub(1));
        for (index, row) in rows.iter().enumerate().skip(start).take(visible) {
            paint(
                out,
                x + 2,
                y + 4 + (index - start) as u16,
                w - 4,
                &format!(
                    "{} {}",
                    if index == frame.selected { ">" } else { " " },
                    clean(&row.label)
                ),
                if index == frame.selected {
                    Color::White
                } else {
                    Color::DarkGrey
                },
            )?;
        }
        let progress = self
            .work
            .as_ref()
            .map(|work| work.progress.lock().unwrap().line.clone())
            .unwrap_or_default();
        paint(
            out,
            x + 2,
            y + h - 4,
            w - 4,
            &clean(if self.message.is_empty() {
                &progress
            } else {
                &self.message
            }),
            Color::White,
        )?;
        paint(
            out,
            x + 2,
            y + h - 2,
            w - 4,
            if self.editing.is_some() {
                "Type value · Enter save field · Esc finish editing"
            } else if self.work.is_some() {
                "Working in background · Esc request cancellation"
            } else {
                "↑↓ select · Enter choose/edit · Esc back · paths and choices stay scoped to this server"
            },
            Color::DarkGrey,
        )
    }
}
fn title(page: Page) -> &'static str {
    match page {
        Page::Players => "Player collection",
        Page::Server => "Server content / player requirements",
        Page::Sources => "Exact provider sources",
        Page::Updates => "Individual content updates",
        Page::Retained => "Retained original / missing files",
        Page::Review => "Review content changes",
        Page::History => "Dated publications",
        Page::Publication => "Frozen content revision",
        Page::Worlds => "Choose world slot",
        _ => "Content",
    }
}
fn form_name(kind: FormKind) -> &'static str {
    match kind {
        FormKind::Search => "Browse content",
        FormKind::PlayerFile => "Add player file",
        FormKind::PlayerOptions => "Player and server choices",
        FormKind::Manual => "Choose exact manual file",
        FormKind::Recovery => "Recover player content",
        FormKind::Pack => "Review pack update or replacement",
        FormKind::Publication => "Prepare dated publication",
        FormKind::Export => "Export frozen revision",
        FormKind::Key => "Save missing CurseForge key",
        FormKind::Source => "Review source hint",
    }
}
fn form_kind(name: &str) -> FormKind {
    [
        FormKind::Search,
        FormKind::PlayerFile,
        FormKind::PlayerOptions,
        FormKind::Manual,
        FormKind::Recovery,
        FormKind::Pack,
        FormKind::Publication,
        FormKind::Export,
        FormKind::Key,
        FormKind::Source,
    ]
    .into_iter()
    .find(|kind| form_name(*kind) == name)
    .unwrap_or(FormKind::Search)
}
fn choice(values: Vec<&'static str>) -> FieldKind {
    FieldKind::Choice(values)
}
fn make_form(kind: FormKind, context: Value, data: &Value) -> Frame {
    let mut frame = Frame::new(Page::Form, form_name(kind), data.clone(), context.clone());
    match kind {
        FormKind::Search => {
            frame.field(
                "provider",
                "Provider",
                if text(&context, "provider") == "curseforge" {
                    "curseforge"
                } else {
                    "modrinth"
                },
                choice(vec!["modrinth", "curseforge"]),
            );
            frame.field(
                "kind",
                "Content",
                &text(&context, "kind"),
                choice(if context["creation"] == true {
                    vec!["mod", "plugin", "modpack"]
                } else {
                    vec!["mod", "plugin", "modpack", "datapack"]
                }),
            );
            frame.field("query", "Search", &text(&context, "query"), FieldKind::Text);
        }
        FormKind::PlayerFile => {
            frame.field("path", "Local file path", "", FieldKind::Text);
            frame.field(
                "target",
                "Minecraft relative path (e.g. mods/example.jar)",
                "",
                FieldKind::Text,
            );
            frame.field(
                "optional",
                "Optional for players",
                "No",
                choice(vec!["No", "Yes"]),
            );
            frame.field(
                "evidence",
                "License/redistribution evidence (optional)",
                "",
                FieldKind::Text,
            );
        }
        FormKind::PlayerOptions if context.get("candidate").is_some() => {
            frame.field(
                "evidence",
                "License/redistribution evidence for these exact bytes",
                &text(&context["candidate"], "redistributionEvidence"),
                FieldKind::Text,
            );
            frame.field(
                "configuration",
                "I reviewed this configuration for secrets",
                "No",
                choice(vec!["No", "Yes"]),
            );
        }
        FormKind::PlayerOptions if context.get("packClassification").is_some() => {
            frame.field(
                "audience",
                "Content audience",
                "both",
                choice(vec!["both", "server", "players"]),
            );
        }
        FormKind::PlayerOptions => {
            let entry = &context["entry"];
            if context["serverEntry"].as_bool() == Some(true) {
                frame.field(
                    "action",
                    "Server classification",
                    "Players need this",
                    choice(vec![
                        "Players need this",
                        "Server only",
                        "Optional for players",
                    ]),
                );
            } else {
                frame.field(
                    "action",
                    "Action",
                    "Change options",
                    choice(vec![
                        "Change options",
                        "Remove player file",
                        "Restore upstream",
                    ]),
                );
                frame.field(
                    "optional",
                    "Optional",
                    if entry["optional"] == true {
                        "Yes"
                    } else {
                        "No"
                    },
                    choice(vec!["No", "Yes"]),
                );
                frame.field(
                    "enabled",
                    "Selected",
                    if entry["enabled"] == false {
                        "No"
                    } else {
                        "Yes"
                    },
                    choice(vec!["No", "Yes"]),
                );
                frame.field(
                    "evidence",
                    "License/redistribution evidence",
                    &text(entry, "redistributionEvidence"),
                    FieldKind::Text,
                );
            }
        }
        FormKind::Manual => frame.field("path", "Exact downloaded file path", "", FieldKind::Text),
        FormKind::Recovery => {
            frame.field(
                "source",
                "Source",
                "Retained original",
                choice(vec![
                    "Retained original",
                    "Inspect retained files",
                    "Local original client archive",
                    "Browse original client pack",
                    "Selected server file",
                ]),
            );
            frame.field(
                "path",
                "Local original .mrpack/.zip path",
                "",
                FieldKind::Text,
            );
            frame.field(
                "server_path",
                "Server content path (selected-file recovery)",
                "",
                FieldKind::Text,
            );
        }
        FormKind::Pack => {
            frame.field(
                "action",
                "Action",
                "Update",
                choice(vec!["Update", "Replace"]),
            );
            frame.field(
                "source",
                "Pack source",
                "Local archive",
                choice(vec!["Local archive", "Browse packs"]),
            );
            frame.field("path", "Exact .mrpack/.zip path", "", FieldKind::Text);
            frame.field(
                "origin",
                "I reviewed the identity of an unverified local archive",
                "No",
                choice(vec!["No", "Yes"]),
            );
            frame.field(
                "worlds",
                "I understand replacement can make existing worlds incompatible",
                "No",
                choice(vec!["No", "Yes"]),
            );
            for entry in objects(&data["collection"], "currentFiles") {
                let path = text(&entry, "path");
                if data["collection"]["upstreamFiles"].get(&path).is_none() {
                    frame.field(
                        &format!("preserve:{path}"),
                        &format!("Confirmed compatible owner addition: {path}"),
                        "No",
                        choice(vec!["No", "Yes"]),
                    );
                }
            }
        }
        FormKind::Publication => {
            frame.field(
                "slot",
                "Optional world slot ID (shown in World datapacks)",
                "",
                FieldKind::Text,
            );
            frame.field(
                "datapacks",
                "Exact datapack paths to include (semicolon separated)",
                "",
                FieldKind::Text,
            );
            frame.field(
                "datapack_evidence",
                "Redistribution evidence for the selected datapacks",
                "",
                FieldKind::Text,
            );
            frame.field(
                "audience",
                "Audience",
                "players",
                choice(vec!["players", "server"]),
            );
            frame.field("name", "Optional version name", "", FieldKind::Text);
            frame.field(
                "loader",
                "Player loader",
                "Infer",
                choice(vec![
                    "Infer", "vanilla", "fabric", "forge", "neoforge", "quilt",
                ]),
            );
            frame.field(
                "loader_version",
                "Exact player loader version (if explicit)",
                "",
                FieldKind::Text,
            );
            frame.field(
                "server_path",
                "Additional exact server config/script path (optional)",
                "",
                FieldKind::Text,
            );
        }
        FormKind::Export => {
            frame.field(
                "audience",
                "Audience",
                "players",
                choice(vec!["players", "server"]),
            );
            frame.field(
                "destination",
                "Destination",
                "New local file",
                choice(vec!["New local file", "Server exports folder"]),
            );
            frame.field(
                "path",
                "New local path or host-relative .zip filename",
                "",
                FieldKind::Text,
            );
            frame.field(
                "mrpack",
                "Include .mrpack inside ZIP",
                "No",
                choice(vec!["No", "Yes"]),
            );
            frame.field(
                "incomplete",
                "Export explicitly incomplete with all warnings",
                "No",
                choice(vec!["No", "Yes"]),
            );
            frame.field(
                "replace",
                "Replace the reviewed existing host file",
                "No",
                choice(vec!["No", "Yes"]),
            );
        }
        FormKind::Source => {
            frame.field(
                "action",
                "Action",
                "Save project hint",
                choice(vec!["Save project hint", "Remove source"]),
            );
            frame.field(
                "project",
                "CurseForge project ID (hint only; never an exact match)",
                "",
                FieldKind::Text,
            );
        }
        FormKind::Key => frame.field("key", "CurseForge API key", "", FieldKind::Secret),
    }
    frame
}
impl ContentUi {
    fn workflow_rows(&self, frame: &Frame) -> Vec<Row> {
        let base = root(&frame.context);
        let mut rows = vec![
            row("Back / keep current content", Action::Back),
            row("Content home", Action::Home),
        ];
        if let Some(notice) = frame.data["notice"].as_str() {
            rows.push(row(notice, Action::None));
        }
        match frame.page {
            Page::Retained => {
                rows.extend([
                    row(
                        format!(
                            "{} · {}",
                            text(&frame.data, "name"),
                            text(&frame.data, "version")
                        ),
                        Action::None,
                    ),
                    row(
                        "Retry missing provider downloads",
                        Action::Menu("retry_retained"),
                    ),
                    row(
                        "Review recovery from retained original",
                        Action::Menu("recover_retained"),
                    ),
                ]);
                for file in array(&frame.data, "missing") {
                    rows.push(row(
                        format!("{} — {}", text(&file, "path"), text(&file, "reason")),
                        if file["manualRecoveryAvailable"] == true {
                            Action::Manual(file)
                        } else {
                            Action::None
                        },
                    ));
                }
            }
            Page::Review => {
                for warning in issue_lines(&frame.data) {
                    rows.push(row(warning, Action::None));
                }
                let kind = text(&frame.context, "reviewKind");
                let review = text(&frame.data, "reviewId");
                if kind == "publication" {
                    for candidate in array(&frame.data, "candidates") {
                        let key = format!(
                            "exclude:{}:{}",
                            text(&candidate, "audience"),
                            text(&candidate, "path")
                        );
                        rows.push(row(
                            format!(
                                "[{}] Include {}{}",
                                if frame.choices.get(&key) == Some(&true) {
                                    " "
                                } else {
                                    "x"
                                },
                                text(&candidate, "path"),
                                if candidate["optional"] == true {
                                    " (optional)"
                                } else {
                                    ""
                                }
                            ),
                            Action::Toggle(key),
                        ));
                        let mut ctx = frame.context.clone();
                        ctx["candidate"] = candidate.clone();
                        rows.push(row(
                            format!(
                                "  Review permission/configuration: {}",
                                text(&candidate, "path")
                            ),
                            Action::Form(FormKind::PlayerOptions, ctx),
                        ));
                    }
                    rows.push(row(
                        format!(
                            "[{}] Explicitly publish incomplete warnings",
                            if frame.choices.get("incomplete") == Some(&true) {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        Action::Toggle("incomplete".into()),
                    ));
                    rows.push(row(
                        "Refresh publication review with these choices",
                        Action::RefreshReview,
                    ));
                    rows.push(row(
                        "Publish dated immutable revision",
                        if frame.context["dirty"] == true
                            || (frame.data["complete"] != true
                                && frame.choices.get("incomplete") != Some(&true))
                        {
                            Action::None
                        } else {
                            Action::Confirm(Box::new(Action::Apply(
                                format!("{base}/publications"),
                                review,
                            )))
                        },
                    ));
                } else if kind.starts_with("pack-") {
                    for entry in objects(&frame.data["collection"], "currentFiles") {
                        if !array(&entry, "unresolvedReasons").is_empty()
                            || array(&entry, "classificationEvidence").is_empty()
                        {
                            let mut context = frame.context.clone();
                            context["packClassification"] = entry["path"].clone();
                            rows.push(row(
                                format!("Review audience: {}", text(&entry, "path")),
                                Action::Form(FormKind::PlayerOptions, context),
                            ));
                        }
                    }
                    for (side, key) in [("server", "serverChanges"), ("player", "playerChanges")] {
                        for item in array(&frame.data, key) {
                            let choice = format!("restore-{side}:{}", text(&item, "path"));
                            rows.push(row(
                                format!(
                                    "[{}] Restore {} upstream: {} · {} — {}",
                                    if frame.choices.get(&choice) == Some(&true) {
                                        "x"
                                    } else {
                                        " "
                                    },
                                    side,
                                    text(&item, "path"),
                                    text(&item, "action"),
                                    text(&item, "reason")
                                ),
                                Action::Toggle(choice),
                            ));
                        }
                    }
                    for missing in array(&frame.data, "missing") {
                        rows.push(row(
                            missing.as_str().unwrap_or("Unresolved content"),
                            Action::None,
                        ));
                    }
                    for missing in array(&frame.data, "missingFiles") {
                        if missing["manualRecoveryAvailable"] == true {
                            rows.push(row(
                                format!("Choose exact missing {}", text(&missing, "path")),
                                Action::Manual(missing),
                            ));
                        }
                    }
                    rows.push(row(
                        "Refresh comparison (preserve owner changes by default)",
                        Action::RefreshReview,
                    ));
                    rows.push(row(
                        "Apply this reviewed pack change",
                        if frame.context["dirty"] == true
                            || !array(&frame.data, "missing").is_empty()
                        {
                            Action::None
                        } else {
                            Action::Confirm(Box::new(Action::Apply(
                                format!("{base}/{}", kind),
                                review,
                            )))
                        },
                    ));
                } else {
                    for file in array(&frame.data, "files") {
                        rows.push(row(
                            format!("{} · {}", text(&file, "fileName"), text(&file, "versionId")),
                            if file["manualDownloadRequired"] == true {
                                Action::Manual(file)
                            } else {
                                Action::None
                            },
                        ));
                    }
                    for change in array(&frame.data, "changes") {
                        rows.push(row(
                            format!("{} · {}", text(&change, "path"), text(&change, "action")),
                            Action::None,
                        ));
                    }
                    rows.push(row(
                        if kind == "install" {
                            "Finish exact-file installation"
                        } else {
                            "Apply reviewed player changes"
                        },
                        Action::Confirm(Box::new(Action::Apply(
                            format!(
                                "{base}/{}",
                                if kind == "install" {
                                    "install"
                                } else {
                                    "apply"
                                }
                            ),
                            review,
                        ))),
                    ));
                }
            }
            Page::History => {
                let offset = frame.context["historyOffset"].as_u64().unwrap_or(0);
                let total = frame.data["total"].as_u64().unwrap_or(0);
                let count = array(&frame.data, "items").len() as u64;
                for (label, next) in [
                    ("Previous publications", offset.checked_sub(50)),
                    (
                        "Next publications",
                        (count > 0 && offset + count < total).then_some(offset + count),
                    ),
                ] {
                    if let Some(next) = next {
                        let mut context = frame.context.clone();
                        context["historyOffset"] = json!(next);
                        rows.push(row(
                            label,
                            Action::Navigate(
                                Page::History,
                                format!("{base}/publications?offset={next}&limit=50"),
                                context,
                            ),
                        ));
                    }
                }
                rows.push(row(
                    "Publish current content",
                    Action::Form(FormKind::Publication, frame.context.clone()),
                ));
                for item in array(&frame.data, "items") {
                    rows.push(row(
                        format!(
                            "{} · revision {} · {}{}",
                            text(&item, "createdAt"),
                            item["publicationRevision"],
                            text(&item, "versionName"),
                            if item["complete"] == true {
                                ""
                            } else {
                                " · incomplete"
                            }
                        ),
                        if item["available"] == true {
                            Action::Publication(text(&item, "publicationId"))
                        } else {
                            Action::None
                        },
                    ));
                }
            }
            Page::Publication => {
                rows.push(row(
                    format!(
                        "{} · revision {} · {}",
                        text(&frame.data, "createdAt"),
                        frame.data["publicationRevision"],
                        text(&frame.data, "versionName")
                    ),
                    Action::None,
                ));
                for side in ["playerFiles", "serverFiles"] {
                    for file in objects(&frame.data, side) {
                        rows.push(row(
                            format!(
                                "{}: {}{}",
                                side,
                                text(&file, "path"),
                                if file["optional"] == true {
                                    " · optional"
                                } else {
                                    ""
                                }
                            ),
                            Action::None,
                        ));
                    }
                }
                rows.extend(
                    issue_lines(&frame.data)
                        .into_iter()
                        .map(|line| row(line, Action::None)),
                );
                let mut context = frame.context.clone();
                context["publicationId"] = frame.data["id"].clone();
                rows.push(row(
                    "Export this exact dated revision",
                    Action::Form(FormKind::Export, context),
                ));
            }
            Page::Worlds => {
                for slot in array(&frame.data, "slots") {
                    let mut context = frame.context.clone();
                    context["slotId"] = slot["id"].clone();
                    context["kind"] = json!("datapack");
                    rows.push(row(
                        format!("{} · slot {}", text(&slot, "name"), text(&slot, "id")),
                        Action::Navigate(
                            Page::WorldPacks,
                            format!("/v1/worlds/{}/profile", enc(&text(&slot, "id"))),
                            context,
                        ),
                    ));
                }
            }
            Page::WorldPacks => {
                rows.push(row(
                    "Browse datapacks from either provider",
                    Action::Form(FormKind::Search, frame.context.clone()),
                ));
                for pack in array(&frame.data["profile"], "packs") {
                    rows.push(row(
                        format!(
                            "{} · {}",
                            text(&pack, "name"),
                            text(&pack["source"], "provider")
                        ),
                        Action::None,
                    ));
                }
            }
            _ => {}
        }
        rows
    }
    fn workflow_action(&mut self, action: Action, frame: Frame) {
        match action {
            Action::Apply(path, review) => {
                if frame.context["restoreDatapack"] == true {
                    let mismatch = frame.context["mismatch"] == true;
                    self.spawn("Restoring exact reviewed datapack", move |progress| {
                        Box::pin(async move { install_selected(frame, mismatch, progress).await })
                    });
                    return;
                }
                if frame.context["mismatch"] == true && frame.context.get("release").is_some() {
                    self.install(frame.context["release"].clone(), true);
                    return;
                }
                let context = frame.context.clone();
                let incomplete =
                    frame.choices.get("incomplete") == Some(&true) || context["incomplete"] == true;
                self.spawn("Applying reviewed content", move |progress| {
                    Box::pin(async move {
                        let client = SharedClient::connect_local().await?;
                        guard(&client, &context).await?;
                        let result = operation(
                            &client,
                            &path,
                            &json!({"reviewId":review,"incompleteConsent":incomplete}),
                            &progress,
                        )
                        .await?;
                        if let Some(id) = result["publicationId"].as_str() {
                            let data = client
                                .get_json(&format!("{}/publications/{}", root(&context), enc(id)))
                                .await?;
                            return Ok(Frame::new(
                                Page::Publication,
                                "Dated publication",
                                data,
                                context,
                            ));
                        }
                        let data = client
                            .get_json(&format!("{}/collection", root(&context)))
                            .await?;
                        Ok(Frame::new(Page::Home, "Content saved", data, context))
                    })
                });
                self.refresh = true;
            }
            Action::Confirm(action) => {
                let mut context = frame.context.clone();
                context["confirmedAction"] = json!(true);
                context["applyPath"] = json!(match action.as_ref() {
                    Action::Apply(path, _) => path.as_str(),
                    _ => "",
                });
                context["reviewId"] = json!(match action.as_ref() {
                    Action::Apply(_, id) => id.as_str(),
                    _ => "",
                });
                if frame.choices.get("incomplete") == Some(&true) {
                    context["incomplete"] = json!(true);
                }
                self.push(Frame::new(
                    Page::Confirm,
                    "Confirm the reviewed content action",
                    frame.data,
                    context,
                ));
            }
            Action::Manual(file) => {
                let mut context = frame.context;
                context["manual"] = file;
                self.push(make_form(FormKind::Manual, context, &frame.data));
            }
            Action::Publication(id) => self.navigate(
                Page::Publication,
                format!("{}/publications/{}", root(&frame.context), enc(&id)),
                frame.context,
            ),
            Action::Source(entry, candidate) => {
                let context = frame.context.clone();
                self.spawn("Linking exact source",move |_|Box::pin(async move{let client=SharedClient::connect_local().await?;guard(&client,&context).await?;let _:Value=client.post_json(&format!("{}/source",root(&context)),&json!({"path":entry["path"],"sha512":entry["sha512"],"projectId":text(&candidate,"projectId").parse::<u64>().map_err(|_|CliError::usage("Invalid provider project ID"))?,"versionId":text(&candidate,"versionId").parse::<u64>().map_err(|_|CliError::usage("Invalid provider release ID"))?})).await?;let data=client.get_json(&format!("{}/identify",root(&context))).await?;Ok(Frame::new(Page::Sources,"Provider source saved",data,context))}));
            }
            Action::Update(item) => {
                let mut context = frame.context;
                context["reviewKind"] = json!("install");
                self.spawn("Reviewing exact update",move |_|Box::pin(async move{let client=SharedClient::connect_local().await?;guard(&client,&context).await?;let snapshot:Value=client.get_json(&format!("{}/collection",root(&context))).await?;let path=text(&item,"path");let hash=snapshot["serverBaseline"][&path].as_str().ok_or_else(||CliError::usage("Installed bytes are unavailable; refresh the server collection"))?;let data=client.post_json(&format!("{}/update-review",root(&context)),&json!({"path":path,"sha512":hash,"projectId":text(&item,"projectId").parse::<u64>().map_err(|_|CliError::usage("Invalid project ID"))?,"versionId":text(&item,"availableVersionId").parse::<u64>().map_err(|_|CliError::usage("Invalid release ID"))?})).await?;Ok(Frame::new(Page::Review,"Review exact update",data,context))}));
            }
            Action::RefreshReview => self.refresh_review(frame),
            Action::SelectPack(current) => {
                self.selection = Some(Selection::Pack {
                    upload: text(&frame.context, "uploadId"),
                    inspection: frame.data,
                    current_setup: current,
                });
                self.open = false;
            }
            Action::Menu(name) => self.menu(name, frame),
            Action::None => {}
            _ => {}
        }
    }
}
async fn collection(client: &SharedClient, context: &Value) -> Result<Value, CliError> {
    client
        .get_json(&format!("{}/collection", root(context)))
        .await
}
async fn review_request(
    client: &SharedClient,
    mut context: Value,
    body: Value,
    kind: &str,
    progress: &SharedProgress,
) -> Result<Frame, CliError> {
    guard(client, &context).await?;
    let base = root(&context);
    context["reviewKind"] = json!(kind);
    context["reviewRequest"] = body.clone();
    context["dirty"] = json!(false);
    let (route, read) = match kind {
        "player" => (format!("{base}/review"), None),
        "recovery" => (format!("{base}/recovery-review"), Some("reviews")),
        "publication" => (
            format!("{base}/publication-reviews"),
            Some("publication-reviews"),
        ),
        "pack-update" => (format!("{base}/pack-update-review"), Some("pack-reviews")),
        "pack-replacement" => (
            format!("{base}/pack-replacement-review"),
            Some("pack-reviews"),
        ),
        _ => return Err(CliError::usage("Unknown reviewed content action")),
    };
    let result = operation(client, &route, &body, progress).await?;
    let data = if let Some(read) = read {
        let id = result["reviewId"]
            .as_str()
            .ok_or_else(|| CliError::internal("Agent returned no review identity"))?;
        if let Some(pack) = result["packId"].as_str() {
            context["recoveryPackId"] = json!(pack);
        }
        client
            .get_json(&format!("{base}/{read}/{}", enc(id)))
            .await?
    } else {
        result
    };
    Ok(Frame::new(
        Page::Review,
        "Review exact content choices",
        data,
        context,
    ))
}
async fn catalogue(client: &SharedClient, mut context: Value) -> Result<Frame, CliError> {
    guard(client, &context).await?;
    let provider = text(&context, "provider");
    let kind = text(&context, "kind");
    if !["modrinth", "curseforge"].contains(&provider.as_str())
        || !["mod", "plugin", "modpack", "datapack"].contains(&kind.as_str())
    {
        return Err(CliError::usage("Choose a supported provider and category"));
    }
    let offset = context["offset"].as_u64().unwrap_or(0);
    let path = format!(
        "/v1/catalog/{provider}/search?kind={}&q={}&offset={offset}",
        enc(&kind),
        enc(&text(&context, "query"))
    );
    let data = match client.get_json(&path).await {
        Ok(data) => data,
        Err(error) => {
            let body = error
                .json_message
                .as_deref()
                .and_then(|body| serde_json::from_str::<Value>(body).ok());
            if body
                .as_ref()
                .is_some_and(|body| body["code"] == "missing_curseforge_api_key")
            {
                return Ok(make_form(
                    FormKind::Key,
                    context,
                    &json!({"notice":"This host has no CurseForge API key saved. Save one to continue."}),
                ));
            }
            return Err(error);
        }
    };
    context["offset"] = json!(offset);
    Ok(Frame::new(
        Page::Catalog,
        format!("Browse {provider} {kind}"),
        data,
        context,
    ))
}
async fn install_selected(
    mut frame: Frame,
    mismatch: bool,
    progress: SharedProgress,
) -> Result<Frame, CliError> {
    let client = SharedClient::connect_local().await?;
    guard(&client, &frame.context).await?;
    let provider = text(&frame.context, "provider");
    let project = &frame.context["project"];
    let release = &frame.context["release"];
    let v = version(release);
    let project_id = text(project, "projectId");
    let version_id = text(v, "id");
    let kind = text(&frame.context, "kind");
    let base = root(&frame.context);
    if kind == "modpack" {
        let result = operation(
            &client,
            "/v1/catalog/modpacks/acquire",
            &json!({"provider":provider,"projectId":project_id,"versionId":version_id}),
            &progress,
        )
        .await?;
        let id = result["stagedUploadId"]
            .as_str()
            .ok_or_else(|| CliError::internal("Pack acquisition returned no staged file"))?;
        frame.context["uploadId"] = json!(id);
        if frame.context["creation"] == true {
            let data = client
                .post_json("/v1/modpacks/inspect", &json!({"stagedUploadId":id}))
                .await?;
            return Ok(Frame::new(
                Page::PackSetup,
                "Choose pack setup; review staged additions before creating",
                data,
                frame.context,
            ));
        }
        let snapshot = collection(&client, &frame.context).await?;
        let intent = frame.context["packIntent"]
            .as_str()
            .unwrap_or("recovery")
            .to_owned();
        let mut body = frame.context["packRequest"].clone();
        if !body.is_object() {
            body = json!({});
        }
        body["collectionRevision"] = snapshot["collection"]["revision"].clone();
        body["stagedUploadId"] = json!(id);
        let result = review_request(&client, frame.context, body, &intent, &progress).await;
        let cleanup = client.cancel_staged(id).await;
        match (result, cleanup) {
            (Ok(value), Ok(())) => return Ok(value),
            (Err(error), _) => return Err(error),
            (Ok(_), Err(error)) => {
                return Err(CliError::usage(format!(
                    "Pack review was prepared, but archive cleanup is unconfirmed: {error}"
                )));
            }
        }
    }
    if kind == "datapack" {
        let slot = text(&frame.context, "slotId");
        if slot.is_empty() {
            return Err(CliError::usage(
                "Choose an exact world slot before installing a datapack",
            ));
        }
        if release["manualDownloadRequired"] == true {
            frame.context["manualMode"] = json!("datapack");
            return Ok(make_form(FormKind::Manual, frame.context, &frame.data));
        }
        let mut body = json!({"provider":provider,"projectId":project_id,"versionId":version_id});
        if let Some(fingerprint) = frame.context["restorationFingerprint"].as_str() {
            body["restoreModified"] = json!(true);
            body["restorationFingerprint"] = json!(fingerprint);
        }
        let _: Value = client
            .post_json(
                &format!("/v1/worlds/{}/datapacks/install", enc(&slot)),
                &body,
            )
            .await?;
        let data = client
            .get_json(&format!("/v1/worlds/{}/profile", enc(&slot)))
            .await?;
        return Ok(Frame::new(
            Page::WorldPacks,
            "Datapack installed in selected world",
            data,
            frame.context,
        ));
    }
    if provider == "curseforge" {
        let review:Value=client.post_json(&format!("{base}/install-review"),&json!({"provider":provider,"projectId":project_id.parse::<u64>().map_err(|_|CliError::usage("Invalid project ID"))?,"versionId":version_id.parse::<u64>().map_err(|_|CliError::usage("Invalid release ID"))?,"allowIncompatible":mismatch})).await?;
        if array(&review, "files")
            .iter()
            .any(|file| file["manualDownloadRequired"] == true)
        {
            frame.context["reviewKind"] = json!("install");
            return Ok(Frame::new(
                Page::Review,
                "Author-required manual files",
                review,
                frame.context,
            ));
        }
        operation(
            &client,
            &format!("{base}/install"),
            &json!({"reviewId":review["reviewId"]}),
            &progress,
        )
        .await?;
    } else {
        operation(&client,"/v1/components/install",&json!({"projectId":project_id,"versionId":version_id,"title":project["title"],"slug":project["slug"]}),&progress).await?;
    }
    let data = collection(&client, &frame.context).await?;
    Ok(Frame::new(
        Page::Server,
        "Installed; review player requirements",
        data,
        frame.context,
    ))
}
impl ContentUi {
    fn submit(&mut self, kind: FormKind, frame: Frame) {
        let parent = self.history.last().cloned();
        self.spawn("Processing content choices", move |progress| {
            Box::pin(async move { perform_form(kind, frame, parent, progress).await })
        });
    }
    fn refresh_review(&mut self, frame: Frame) {
        self.spawn("Refreshing pinned comparison", move |progress| {
            Box::pin(async move {
                let client = SharedClient::connect_local().await?;
                let mut body = frame.context["reviewRequest"].clone();
                let kind = text(&frame.context, "reviewKind");
                if kind.starts_with("pack-") {
                    body.as_object_mut()
                        .ok_or_else(|| CliError::usage("Prepare an initial pack review"))?
                        .remove("stagedUploadId");
                    body["previousReviewId"] = frame.data["reviewId"].clone();
                    body["restoreServer"] = json!(selected_keys(&frame, "restore-server:"));
                    body["restorePlayer"] = json!(selected_keys(&frame, "restore-player:"));
                }
                if kind == "publication" {
                    body["excluded"] = json!(
                        selected_keys(&frame, "exclude:")
                            .iter()
                            .filter_map(|key| key
                                .split_once(':')
                                .map(|(audience, path)| json!({"audience":audience,"path":path})))
                            .collect::<Vec<_>>()
                    );
                }
                review_request(&client, frame.context, body, &kind, &progress).await
            })
        });
    }
    fn menu(&mut self, name: &str, frame: Frame) {
        if name == "key_console" {
            self.spawn("Opening CurseForge console", move |_| {
                Box::pin(async move {
                    let note = super::storage::open_folder("https://console.curseforge.com/").await;
                    let mut frame = frame;
                    frame.title = note;
                    Ok(frame)
                })
            });
            return;
        }
        let name = name.to_owned();
        self.spawn("Reading retained pack",move |progress|Box::pin(async move{
            let client=SharedClient::connect_local().await?;let snapshot=collection(&client,&frame.context).await?;let id=snapshot["collection"]["basePackId"].as_str().ok_or_else(||CliError::usage("No retained original; choose the original client archive"))?;let base=root(&frame.context);
            if name=="retry_retained"{operation(&client,&format!("{base}/retained-packs/{}/retry",enc(id)),&json!({}),&progress).await?;}
            if name=="recover_retained"{return review_request(&client,frame.context,json!({"collectionRevision":snapshot["collection"]["revision"],"retainedPackId":id,"selectedPaths":[],"restorePaths":[]}),"recovery",&progress).await;}
            let data=client.get_json(&format!("{base}/retained-packs/{}",enc(id))).await?;Ok(Frame::new(Page::Retained,"Retained original and missing files",data,frame.context))
        }));
    }
}
async fn perform_form(
    kind: FormKind,
    frame: Frame,
    parent: Option<Frame>,
    progress: SharedProgress,
) -> Result<Frame, CliError> {
    let client = SharedClient::connect_local().await?;
    guard(&client, &frame.context).await?;
    let mut context = frame.context.clone();
    let base = root(&context);
    match kind {
        FormKind::Search => {
            let provider = frame.value("provider");
            let category = frame.value("kind");
            let query = frame.value("query");
            if provider != text(&context, "provider")
                || category != text(&context, "kind")
                || query != text(&context, "query")
            {
                context["offset"] = json!(0);
            }
            context["provider"] = json!(provider);
            context["kind"] = json!(category);
            context["query"] = json!(query);
            catalogue(&client, context).await
        }
        FormKind::Key => {
            let _: Value = client
                .post_json(
                    "/v1/config/curseforge",
                    &json!({"apiKey":frame.value("key")}),
                )
                .await?;
            catalogue(&client, context).await
        }
        FormKind::Source => {
            let entry = &context["sourceEntry"];
            let mut body = json!({"path":entry["path"],"sha512":entry["sha512"]});
            if frame.value("action") != "Remove source" {
                let project = frame
                    .value("project")
                    .parse::<u64>()
                    .ok()
                    .filter(|id| *id > 0)
                    .ok_or_else(|| CliError::usage("Enter a positive CurseForge project ID"))?;
                body["projectId"] = json!(project);
            }
            let _: Value = client.post_json(&format!("{base}/source"), &body).await?;
            let data = client.get_json(&format!("{base}/identify")).await?;
            Ok(Frame::new(
                Page::Sources,
                "Source hint saved; only checksum matches establish an exact release",
                data,
                context,
            ))
        }
        FormKind::PlayerFile => {
            let path = frame.value("path");
            let target = frame.value("target");
            if target.trim().is_empty() {
                return Err(CliError::usage("Enter the exact Minecraft content path"));
            }
            let id = stage_file_upload(
                &client,
                Path::new(&path),
                StagedUploadPurposeDto::AddonLocalFile,
                None,
                None,
                Some(text(&context, "serverId")),
            )
            .await?;
            let prepared: Value = client
                .post_json(
                    &format!("{base}/player-files/prepare"),
                    &json!({"stagedUploadId":id,"path":target}),
                )
                .await?;
            let bound = text(&prepared, "stagedUploadId");
            let mut changes = vec![
                json!({"action":"addPlayerFile","path":target,"sha512":prepared["sha512"],"stagedUploadId":bound,"optional":frame.yes("optional"),"redistributionEvidence":if frame.value("evidence").trim().is_empty(){Value::Null}else{json!(frame.value("evidence"))}}),
            ];
            if frame.yes("optional")
                && (target.starts_with("shaderpacks/") || target.starts_with("resourcepacks/"))
            {
                changes.push(json!({"action":"setPlayerOptions","path":target,"sha512":prepared["sha512"],"enabled":false}));
            }
            let snapshot = collection(&client, &context).await?;
            let result = review_request(
                &client,
                context,
                json!({"collectionRevision":snapshot["collection"]["revision"],"changes":changes}),
                "player",
                &progress,
            )
            .await;
            let cleanup = client.cancel_staged(&bound).await;
            match (result, cleanup) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) => Err(error),
                (Ok(_), Err(error)) => Err(CliError::usage(format!(
                    "Review ready, but player upload cleanup is unconfirmed: {error}"
                ))),
            }
        }
        FormKind::PlayerOptions => {
            if context.get("candidate").is_some() {
                let mut original =
                    parent.ok_or_else(|| CliError::usage("Reopen publication review"))?;
                let candidate = &context["candidate"];
                if candidate["restricted"] == true {
                    return Err(CliError::usage(
                        "Known distribution restrictions cannot be overridden",
                    ));
                }
                let evidence = frame.value("evidence");
                if evidence.trim().is_empty() {
                    return Err(CliError::usage(
                        "Provide license/redistribution evidence for these exact bytes",
                    ));
                }
                let approval = json!({"audience":candidate["audience"],"path":candidate["path"],"sha512":candidate["sha512"],"redistributionEvidence":evidence,"reviewedConfiguration":frame.yes("configuration")});
                let body = &mut original.context["reviewRequest"];
                let mut approvals = array(body, "approvals");
                approvals.retain(|item| {
                    item["audience"] != candidate["audience"] || item["path"] != candidate["path"]
                });
                approvals.push(approval);
                body["approvals"] = json!(approvals);
                let request = body.clone();
                return review_request(
                    &client,
                    original.context,
                    request,
                    "publication",
                    &progress,
                )
                .await;
            }
            if context.get("packClassification").is_some() {
                let mut original =
                    parent.ok_or_else(|| CliError::usage("Reopen pack comparison"))?;
                let path = text(&context, "packClassification");
                original.context["reviewRequest"]["audiences"][&path] =
                    json!(frame.value("audience"));
                let mut body = original.context["reviewRequest"].clone();
                body.as_object_mut().unwrap().remove("stagedUploadId");
                body["previousReviewId"] = original.data["reviewId"].clone();
                let kind = text(&original.context, "reviewKind");
                return review_request(&client, original.context, body, &kind, &progress).await;
            }
            let snapshot = collection(&client, &context).await?;
            let entry = &context["entry"];
            let path = text(entry, "path");
            let action = frame.value("action");
            let mut changes = vec![];
            if context["serverEntry"] == true {
                if action == "Server only" {
                    changes.push(json!({"action":"setServerAudience","path":path,"sha512":entry["sha512"],"audience":"server"}));
                } else {
                    changes.push(json!({"action":"copyServerToPlayers","path":path,"sha512":entry["sha512"],"optional":action=="Optional for players"}));
                }
            } else if action == "Restore upstream" {
                let upstream = &snapshot["collection"]["upstreamPlayerFiles"][&path];
                if !upstream.is_object() {
                    return Err(CliError::usage("No retained upstream player file exists"));
                }
                changes.push(json!({"action":"restorePlayerUpstream","path":path,"sha512":upstream["sha512"]}));
            } else {
                changes.push(json!({"action":if action=="Remove player file"{"removePlayerFile"}else{"setPlayerOptions"},"path":path,"sha512":entry["sha512"],"optional":frame.yes("optional"),"enabled":frame.yes("enabled"),"redistributionEvidence":if frame.value("evidence").trim().is_empty(){Value::Null}else{json!(frame.value("evidence"))}}));
            }
            review_request(
                &client,
                context,
                json!({"collectionRevision":snapshot["collection"]["revision"],"changes":changes}),
                "player",
                &progress,
            )
            .await
        }
        FormKind::Recovery | FormKind::Pack => {
            let snapshot = collection(&client, &context).await?;
            let recovery = kind == FormKind::Recovery;
            let source = frame.value("source");
            let intent = if recovery {
                "recovery"
            } else if frame.value("action") == "Replace" {
                "pack-replacement"
            } else {
                "pack-update"
            };
            let mut body = json!({"collectionRevision":snapshot["collection"]["revision"]});
            if recovery {
                body["selectedPaths"] = json!(if source == "Selected server file" {
                    vec![frame.value("server_path")]
                } else {
                    vec![]
                });
                body["restorePaths"] = json!([]);
                if source == "Retained original" || source == "Inspect retained files" {
                    let id = snapshot["collection"]["basePackId"]
                        .as_str()
                        .ok_or_else(|| {
                            CliError::usage(
                                "No original pack retained. Choose a local/client catalog archive",
                            )
                        })?;
                    if source == "Inspect retained files" {
                        let data = client
                            .get_json(&format!("{base}/retained-packs/{}", enc(id)))
                            .await?;
                        return Ok(Frame::new(
                            Page::Retained,
                            "Retained original",
                            data,
                            context,
                        ));
                    }
                    body["retainedPackId"] = json!(id);
                }
            } else {
                if intent == "pack-replacement" && !frame.yes("worlds") {
                    return Err(CliError::usage(
                        "Acknowledge possible world incompatibility before replacement",
                    ));
                }
                let preserved = frame
                    .fields
                    .iter()
                    .filter(|field| field.key.starts_with("preserve:") && field.value == "Yes")
                    .map(|field| {
                        let path = field.key.strip_prefix("preserve:").unwrap();
                        (
                            path.to_owned(),
                            snapshot["collection"]["currentFiles"][path]["sha512"].clone(),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                body["preserveAdditions"] = json!(preserved);
                body["confirmUnverifiedOrigin"] = json!(frame.yes("origin"));
                body["confirmWorldCompatibility"] = json!(frame.yes("worlds"));
                body["restoreServer"] = json!([]);
                body["restorePlayer"] = json!([]);
                body["audiences"] = json!({});
            }
            if source.starts_with("Browse") {
                context["kind"] = json!("modpack");
                context["packIntent"] = json!(intent);
                context["packRequest"] = body;
                return Ok(make_form(FormKind::Search, context, &snapshot));
            }
            let mut staged = None;
            if source.starts_with("Local") {
                let id = stage_file_upload(
                    &client,
                    Path::new(&frame.value("path")),
                    StagedUploadPurposeDto::ModpackArchive,
                    None,
                    None,
                    Some(text(&context, "serverId")),
                )
                .await?;
                body["stagedUploadId"] = json!(id);
                staged = Some(id);
            }
            let result = review_request(&client, context, body, intent, &progress).await;
            if let Some(id) = staged
                && let Err(error) = client.cancel_staged(&id).await
            {
                return Err(CliError::usage(format!(
                    "Pack input cleanup unconfirmed: {error}"
                )));
            }
            result
        }
        FormKind::Manual => manual_file(&client, frame, &progress).await,
        FormKind::Publication => {
            let snapshot = collection(&client, &context).await?;
            let path = frame.value("server_path");
            let mut body = json!({"collectionRevision":snapshot["collection"]["revision"],"audiences":[frame.value("audience")],"versionName":if frame.value("name").is_empty(){Value::Null}else{json!(frame.value("name"))},"approvals":[],"excluded":[],"serverPaths":if path.is_empty(){vec![]}else{vec![path]},"datapacks":[]});
            let paths: Vec<String> = frame
                .value("datapacks")
                .split(';')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect();
            if !paths.is_empty() {
                let slot = frame.value("slot");
                if slot.is_empty() {
                    return Err(CliError::usage("Choose an exact world slot for datapacks"));
                }
                let inventory: Value = client
                    .get_json(&format!("{base}/publication-datapacks/{}", enc(&slot)))
                    .await?;
                let items = array(&inventory, "items");
                let mut selected = Vec::new();
                for path in paths {
                    let file = items
                        .iter()
                        .find(|f| text(f, "path") == path)
                        .ok_or_else(|| {
                            CliError::usage(format!(
                                "Datapack {path} is not present in this slot's inventory"
                            ))
                        })?;
                    selected.push(json!({"slotId":slot,"path":path,"sha512":file["sha512"],"redistributionEvidence":frame.value("datapack_evidence")}));
                }
                body["datapacks"] = json!(selected);
            }
            if frame.value("loader") != "Infer" {
                body["playerSetup"] = json!({"loader":frame.value("loader"),"loaderVersion":if frame.value("loader_version").is_empty(){Value::Null}else{json!(frame.value("loader_version"))}});
            }
            review_request(&client, context, body, "publication", &progress).await
        }
        FormKind::Export => export_file(&client, frame, &progress).await,
    }
}
async fn manual_file(
    client: &SharedClient,
    frame: Frame,
    progress: &SharedProgress,
) -> Result<Frame, CliError> {
    let path = frame.value("path");
    let source = Path::new(&path);
    let mut context = frame.context.clone();
    let base = root(&context);
    if context["manualMode"] == "datapack" {
        if context.get("restorationFingerprint").is_some() && !frame.yes("restore") {
            return Err(CliError::usage(
                "Confirm restoration of these reviewed edits before uploading the replacement",
            ));
        }
        let id = stage_file_upload(
            client,
            source,
            StagedUploadPurposeDto::AddonLocalFile,
            None,
            None,
            Some(text(&context, "serverId")),
        )
        .await?;
        let selected = version(&context["release"]);
        let slot = text(&context, "slotId");
        let mut body = json!({"provider":"curseforge","projectId":context["project"]["projectId"],"versionId":selected["id"],"manualStagedUploadId":id});
        if let Some(fingerprint) = context["restorationFingerprint"].as_str() {
            body["restoreModified"] = json!(true);
            body["restorationFingerprint"] = json!(fingerprint);
        }
        let _: Value = client
            .post_json(
                &format!("/v1/worlds/{}/datapacks/install", enc(&slot)),
                &body,
            )
            .await?;
        let data = client
            .get_json(&format!("/v1/worlds/{}/profile", enc(&slot)))
            .await?;
        return Ok(Frame::new(
            Page::WorldPacks,
            "Exact datapack verified and installed",
            data,
            context,
        ));
    }
    let kind = text(&context, "reviewKind");
    let reviewed = context["manual"].clone();
    let id = text(&frame.data, "reviewId");
    let (prefix, mut body) = if kind == "install" {
        (
            format!("{base}/install-manual"),
            json!({"reviewId":id,"versionId":reviewed["versionId"]}),
        )
    } else if kind.starts_with("pack-") {
        (
            format!("{base}/pack-reviews/{}/manual", enc(&id)),
            json!({"path":reviewed["path"]}),
        )
    } else {
        (
            format!(
                "{base}/retained-packs/{}/manual",
                enc(&text(&frame.data, "packId"))
            ),
            json!({"path":reviewed["path"]}),
        )
    };
    let admission: Value = client.post_json(&format!("{prefix}/admit"), &body).await?;
    let upload = text(&admission, "stagedUploadId");
    if uuid::Uuid::parse_str(&upload).is_err()
        || text(&admission, "uploadPath") != format!("/v1/staged-uploads/{upload}")
    {
        return Err(CliError::internal("Unexpected manual upload admission"));
    }
    let expected = if reviewed["fileName"].is_string() {
        text(&reviewed, "fileName")
    } else {
        text(&reviewed, "path")
            .rsplit('/')
            .next()
            .unwrap_or("")
            .into()
    };
    let result=async{
        let meta=tokio::fs::metadata(source).await.map_err(|error|CliError::usage(error.to_string()))?;
        if source.file_name().and_then(|value|value.to_str())!=Some(expected.as_str()) || admission["maxBytes"].as_u64()!=Some(meta.len()){return Err(CliError::usage("Choose the exact expected filename and byte size; the agent verifies its checksums"));}
        let route=text(&admission,"uploadPath");let limit=admission["maxChunkBytes"].as_u64().unwrap_or(1024*1024).min(8*1024*1024) as usize;
        client.put_file_chunks(&route,source,limit).await?;body["stagedUploadId"]=json!(upload);operation(client,&format!("{prefix}/complete"),&body,progress).await?;Ok::<_,CliError>(())
    }.await;
    if let Err(mut error) = result {
        if client.cancel_staged(&upload).await.is_err() {
            error
                .message
                .push_str(" Upload cleanup could not be confirmed.");
        }
        return Err(error);
    }
    if kind == "install" {
        let mut data = frame.data;
        for file in data["files"].as_array_mut().into_iter().flatten() {
            if file["versionId"] == reviewed["versionId"] {
                file["manualDownloadRequired"] = json!(false);
            }
        }
        return Ok(Frame::new(
            Page::Review,
            "Exact file accepted; finish remaining manual files",
            data,
            context,
        ));
    }
    if kind.starts_with("pack-") {
        let mut request = context["reviewRequest"].clone();
        request
            .as_object_mut()
            .ok_or_else(|| CliError::usage("Reopen the pack comparison"))?
            .remove("stagedUploadId");
        request["previousReviewId"] = json!(id);
        return review_request(client, context, request, &kind, progress).await;
    }
    let data = client
        .get_json(&format!(
            "{base}/retained-packs/{}",
            enc(&text(&frame.data, "packId"))
        ))
        .await?;
    context["notice"] =
        json!("Exact file retained. Prepare a fresh player recovery comparison to use it.");
    Ok(Frame::new(
        Page::Retained,
        "Retained file verified",
        data,
        context,
    ))
}
async fn export_file(
    client: &SharedClient,
    mut frame: Frame,
    progress: &SharedProgress,
) -> Result<Frame, CliError> {
    let base = root(&frame.context);
    let id = text(&frame.context, "publicationId");
    let audience = frame.value("audience");
    let mut body = json!({"publicationId":id,"audience":audience,"includeMrpack":frame.yes("mrpack"),"allowIncomplete":frame.yes("incomplete")});
    if frame.data["issues"]
        .as_array()
        .is_some_and(|issues| !issues.is_empty())
        && !frame.yes("incomplete")
    {
        return Err(CliError::usage(
            "Review the revision's warnings before explicitly allowing an incomplete export",
        ));
    }
    if frame.yes("mrpack") {
        let eligibility: Value = client
            .get_json(&format!(
                "{base}/publications/{}/mrpack-eligibility?audience={}",
                enc(&id),
                enc(&audience)
            ))
            .await?;
        if eligibility["supported"] != true {
            return Err(CliError::usage(format!(
                "This revision cannot include .mrpack: {}",
                array(&eligibility, "reasons")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }
    }
    let target = frame.value("path");
    if target.trim().is_empty() {
        return Err(CliError::usage("Choose the output ZIP path"));
    }
    if frame.value("destination") == "Server exports folder" {
        let inspected: Value = client
            .get_json(&format!("{base}/export-destination?path={}", enc(&target)))
            .await?;
        let hash = text(&inspected, "existingSha512");
        if !hash.is_empty()
            && (!frame.yes("replace") || text(&frame.context, "replaceHash") != hash)
        {
            frame.context["replaceHash"] = json!(hash);
            frame.data["notice"] = json!(format!(
                "Existing host destination: {} / {}. Confirm replacement only after reviewing this file.",
                text(&inspected, "root"),
                target
            ));
            if let Some(field) = frame.fields.iter_mut().find(|field| field.key == "replace") {
                field.value = "No".into();
            }
            return Ok(frame);
        }
        body["hostDestination"] = json!({"relativePath":target,"replaceSha512":if hash.is_empty(){Value::Null}else{json!(hash)}});
        let result = operation(client, &format!("{base}/exports"), &body, progress).await?;
        let mut next = Frame::new(
            Page::Publication,
            "ZIP saved on selected host",
            frame.data,
            frame.context,
        );
        next.data["notice"] = json!(format!("Saved: {}", text(&result, "hostPath")));
        return Ok(next);
    }
    if Path::new(&target).exists() {
        return Err(CliError::usage(
            "Choose a new local filename. Existing files are not overwritten by this terminal action.",
        ));
    }
    let result = operation(client, &format!("{base}/exports"), &body, progress).await?;
    let staged = text(&result, "stagedDownloadId");
    let size = text(&result, "byteSize")
        .parse::<u64>()
        .map_err(|_| CliError::internal("Invalid export byte size"))?;
    let downloaded = client
        .download_file_new(&staged, Path::new(&target), Some(size))
        .await;
    let cleanup = client.cancel_download(&staged).await;
    downloaded?;
    cleanup?;
    let mut next = Frame::new(
        Page::Publication,
        "ZIP saved locally",
        frame.data,
        frame.context,
    );
    next.data["notice"] = json!(format!("Saved {size} bytes to {target}"));
    Ok(next)
}

pub(super) async fn install_creation_pick(
    client: &SharedClient,
    server_id: &str,
    pick: &Selection,
) -> Result<(), CliError> {
    let Selection::Individual {
        provider,
        project,
        version: release,
        incompatible,
    } = pick
    else {
        return Ok(());
    };
    let context = json!({"serverId":server_id});
    guard(client, &context).await?;
    let progress = Arc::new(Mutex::new(Progress {
        line: "Installing staged content".into(),
        operation: None,
    }));
    let project_id = text(project, "projectId");
    let version_id = text(release, "id");
    if text(project, "projectType") == "datapack" {
        return Err(CliError::usage(
            "Server created. Open Content → World datapacks to choose the destination slot for the staged datapack.",
        ));
    }
    if provider == "curseforge" {
        let base = root(&context);
        let review: Value = client.post_json(&format!("{base}/install-review"), &json!({"provider":provider,"projectId":project_id.parse::<u64>().map_err(|_|CliError::usage("Invalid project identity"))?,"versionId":version_id.parse::<u64>().map_err(|_|CliError::usage("Invalid release identity"))?,"allowIncompatible":incompatible})).await?;
        if array(&review, "files")
            .iter()
            .any(|file| file["manualDownloadRequired"] == true)
        {
            return Err(CliError::usage(format!(
                "Server created. {} requires an author-provided manual file; open Content and select its exact release to supply that file.",
                text(project, "title")
            )));
        }
        operation(
            client,
            &format!("{base}/install"),
            &json!({"reviewId":review["reviewId"]}),
            &progress,
        )
        .await?;
    } else {
        operation(client,"/v1/components/install",&json!({"projectId":project_id,"versionId":version_id,"title":project["title"],"slug":project["slug"]}),&progress).await?;
    }
    Ok(())
}
