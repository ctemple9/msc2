//! Keyboard-first create wizard. Every page fits in the terminal viewport.
use std::collections::BTreeMap;
use std::io::{self, Stdout, Write};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::Color;
use crossterm::{cursor::MoveTo, queue, style::Print};
use msc_api::dto::{
    CapabilitiesResponseDto, JavaRuntimeDto, JavaRuntimeInstallResultDto, JavaRuntimesResponseDto,
    ModpackInspectionResultDto, OperationDto, OperationStateDto, RemoteApiStatus,
    ServerCreateRequestDto, ServerCreateResultDto, ServerCreateWorldSettingsDto,
    ServerImportResultDto, ServerImportScanResponseDto, StagedUploadPurposeDto, VersionEntryDto,
    VersionsResponseDto, WorldGameplayDto, WorldGenerationDto, WorldIdentityDto,
    WorldSettingsCapabilitiesDto,
};
use serde_json::json;

use super::{box_outline, paint};
use crate::cli::transport::SharedClient;
use crate::cli::{CliError, stage_file_upload};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Fresh,
    Route,
    Modpack,
    Import,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Choose,
    ChooseSource,
    Configure,
    Network,
    World,
    Generation,
    Gameplay,
    Addons,
    Upload,
    Review,
    Confirm,
}

const STANDARD_SOFTWARE: &[&str] = &["Paper", "Purpur", "Vanilla"];
const MODDED_SOFTWARE: &[&str] = &["Fabric", "NeoForge", "Forge"];

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Choose => "Choose path",
            Self::ChooseSource => "Choose source",
            Self::Configure => "Configure",
            Self::Network => "Network",
            Self::World => "First world",
            Self::Generation => "World generation",
            Self::Gameplay => "Gameplay rules",
            Self::Addons => "Add-ons",
            Self::Upload => "Source",
            Self::Review => "Import review",
            Self::Confirm => "Confirm",
        }
    }
}

#[derive(Clone)]
enum FieldKind {
    Text,
    Toggle,
    Choice(&'static [&'static str]),
    Action,
}
struct Field {
    key: &'static str,
    label: &'static str,
    kind: FieldKind,
    hint: &'static str,
}
fn field(key: &'static str, label: &'static str, kind: FieldKind, hint: &'static str) -> Field {
    Field {
        key,
        label,
        kind,
        hint,
    }
}

struct ChoiceCard<'a> {
    title: &'a str,
    description: &'a str,
}

fn draw_choice_card(
    out: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    card: ChoiceCard<'_>,
    selected: bool,
) -> io::Result<()> {
    let color = if selected {
        Color::Green
    } else {
        Color::DarkGrey
    };
    let rule = "─".repeat(width.saturating_sub(2) as usize);
    paint(out, x, y, width, &format!("┌{rule}┐"), color)?;
    for row in y + 1..y + height - 1 {
        paint(out, x, row, 1, "│", color)?;
        paint(out, x + width - 1, row, 1, "│", color)?;
    }
    paint(out, x, y + height - 1, width, &format!("└{rule}┘"), color)?;
    paint(
        out,
        x + 2,
        y + 1,
        width - 4,
        &format!("{} {}", if selected { ">" } else { " " }, card.title),
        if selected { Color::Green } else { Color::White },
    )?;
    for (offset, line) in card.description.lines().enumerate() {
        paint(
            out,
            x + 2,
            y + 3 + offset as u16,
            width - 4,
            line,
            Color::DarkGrey,
        )?;
    }
    Ok(())
}

pub(super) enum Outcome {
    Stay,
    Close,
    Created(String),
}

pub(super) struct Wizard {
    path: PathKind,
    page_index: usize,
    configure_page_index: usize,
    network_page_index: usize,
    world_page_index: usize,
    gamerules_text_saved: bool,
    version_picker_active: bool,
    java_install_page: bool,
    java_install_major: i64,
    cursor: usize,
    values: BTreeMap<&'static str, String>,
    editing: Option<String>,
    message: String,
    scan: Option<ServerImportScanResponseDto>,
    inspection: Option<ModpackInspectionResultDto>,
    staged_modpack: Option<String>,
    submitting: bool,
    confirmation: Option<String>,
    pending_operation_id: Option<String>,
    versions: Vec<VersionEntryDto>,
    runtimes: Vec<JavaRuntimeDto>,
    world_capabilities: Option<WorldSettingsCapabilitiesDto>,
}

impl Wizard {
    pub(super) fn new() -> Self {
        let values = BTreeMap::from([
            ("path", "Import or Create from Modpack".into()),
            ("source_kind", "Import existing".into()),
            ("edition", "Java".into()),
            ("java_category", "Standard".into()),
            ("name", String::new()),
            ("software", "Paper".into()),
            ("source_mode", "Download latest".into()),
            ("version", "LATEST".into()),
            ("java", "Automatic".into()),
            ("port", "25565".into()),
            ("bedrock_port", "19132".into()),
            ("connection", "Port forwarding".into()),
            ("world_source", "New world".into()),
            ("world_name", String::new()),
            ("difficulty", "Normal".into()),
            ("gamemode", "Survival".into()),
            ("world_type", "Default".into()),
            ("structures", "Yes".into()),
            ("crossplay", "No".into()),
            ("broadcast", "No".into()),
            ("hardcore", "No".into()),
            ("commands", "No".into()),
            ("cheats", "No".into()),
            ("coordinates", "Yes".into()),
            ("starting_map", "No".into()),
            ("source", String::new()),
            ("import_world", String::new()),
            ("accept_eula", "No".into()),
            ("max_players", "10".into()),
            ("voice_chat", "No".into()),
            ("bedrock_version", "LATEST".into()),
            ("curseforge_key", String::new()),
        ]);
        Self {
            path: PathKind::Route,
            page_index: 0,
            configure_page_index: 0,
            network_page_index: 0,
            world_page_index: 0,
            gamerules_text_saved: false,
            version_picker_active: false,
            java_install_page: false,
            java_install_major: 25,
            cursor: 0,
            values,
            editing: None,
            message: String::new(),
            scan: None,
            inspection: None,
            staged_modpack: None,
            submitting: false,
            confirmation: None,
            pending_operation_id: None,
            versions: Vec::new(),
            runtimes: Vec::new(),
            world_capabilities: None,
        }
    }

    fn pages(&self) -> Vec<Page> {
        let mut pages = match self.path {
            PathKind::Route => vec![Page::Choose, Page::ChooseSource],
            PathKind::Fresh => {
                let mut pages = vec![
                    Page::Choose,
                    Page::Configure,
                    Page::Network,
                    Page::World,
                    Page::Generation,
                    Page::Gameplay,
                ];
                if self.java() && self.get("software") != "Vanilla" {
                    pages.push(Page::Addons);
                }
                pages.push(Page::Confirm);
                pages
            }
            PathKind::Modpack => vec![
                Page::Choose,
                Page::Upload,
                Page::Network,
                Page::World,
                Page::Generation,
                Page::Gameplay,
                Page::Confirm,
            ],
            PathKind::Import => vec![
                Page::Choose,
                Page::Upload,
                Page::Review,
                Page::Network,
                Page::Confirm,
            ],
        };
        if self.get("world_source") == "Backup .zip" {
            pages.retain(|page| !matches!(page, Page::Generation | Page::Gameplay));
        }
        pages
    }
    fn page(&self) -> Page {
        self.pages()[self.page_index]
    }
    fn step_labels(&self) -> Vec<&'static str> {
        match self.path {
            PathKind::Fresh => {
                let mut labels = vec!["Choose path", "Configure", "Network", "World"];
                if self.java() && self.get("software") != "Vanilla" {
                    labels.push("Add-ons");
                }
                labels.push("Confirm");
                labels
            }
            PathKind::Route => vec![
                "Choose path",
                "Configure",
                "Network",
                "World",
                "Add-ons",
                "Confirm",
            ],
            PathKind::Modpack => {
                vec!["Choose path", "Upload", "Network", "World", "Confirm"]
            }
            PathKind::Import => vec!["Choose path", "Upload", "Review", "Network", "Confirm"],
        }
    }
    fn step_number(&self) -> usize {
        match self.page() {
            Page::Choose | Page::ChooseSource => 1,
            Page::Configure | Page::Upload => 2,
            Page::Review => 3,
            Page::Network if self.path == PathKind::Import => 4,
            Page::Network => 3,
            Page::World | Page::Generation | Page::Gameplay => 4,
            Page::Addons => 5,
            Page::Confirm => self.step_labels().len(),
        }
    }
    fn configure_page_count(&self) -> usize {
        let base = if !self.java() {
            4
        } else if self.yes("crossplay") && self.crossplay_available() {
            7
        } else {
            6
        };
        if self.java() { base + 1 } else { base }
    }
    fn world_page_count(&self) -> usize {
        if self.get("world_source") == "Backup .zip" {
            2
        } else {
            7
        }
    }
    fn java_runtime_page_index(&self) -> usize {
        if self.yes("crossplay") && self.crossplay_available() {
            7
        } else {
            6
        }
    }
    fn is_java_runtime_page(&self) -> bool {
        self.java() && self.configure_page_index == self.java_runtime_page_index()
    }
    fn crossplay_available(&self) -> bool {
        self.java() && self.get("java_category") == "Standard" && self.get("software") != "Vanilla"
    }
    fn configure_fields(&self) -> Vec<Field> {
        use FieldKind::{Choice, Text, Toggle};
        if !self.java() {
            return match self.configure_page_index {
                0 => vec![field(
                    "edition",
                    "Server type",
                    Choice(&["Java", "Bedrock"]),
                    "PC · Console · Mobile",
                )],
                1 => vec![field("name", "Server name", Text, "Enter server name.")],
                2 => vec![
                    field("bedrock_version", "Bedrock version", Text, "LATEST"),
                    field("max_players", "Max players", Text, "10"),
                ],
                _ => vec![field(
                    "broadcast",
                    "Xbox broadcast",
                    Toggle,
                    "Lets console, mobile, and PC players discover the server in the Xbox Friends tab.",
                )],
            };
        }
        if self.is_java_runtime_page() {
            return vec![field(
                "java",
                "Java runtime",
                Choice(&["Select a compatible runtime"]),
                "Use ↑/↓ to select a compatible runtime, or press i to install Java.",
            )];
        }
        match self.configure_page_index {
            0 => vec![field(
                "edition",
                "Server type",
                Choice(&["Java", "Bedrock"]),
                "Java supports PC; Bedrock supports PC, console, and mobile.",
            )],
            1 => vec![field("name", "Server name", Text, "Enter server name.")],
            2 => vec![field(
                "java_category",
                "Server software",
                Choice(&["Standard", "Modded"]),
                "Standard servers add plugins; modded servers add content and require matching player mods.",
            )],
            3 => vec![field(
                "software",
                "Server software",
                Choice(if self.get("java_category") == "Modded" {
                    MODDED_SOFTWARE
                } else {
                    STANDARD_SOFTWARE
                }),
                "Choose the server software for this edition.",
            )],
            4 => {
                vec![field(
                    "source_mode",
                    "Source",
                    Choice(&["Download latest", "Choose version…"]),
                    if self.get("source_mode") == "Download latest" {
                        "MSC will use the latest compatible release for this software."
                    } else {
                        "Choose a Minecraft version available from this agent."
                    },
                )]
            }
            5 => vec![field(
                "crossplay",
                "Enable Bedrock Cross-play",
                Toggle,
                if self.crossplay_available() {
                    "Geyser and Floodgate let Bedrock players join a Java server."
                } else if self.get("java_category") == "Modded" {
                    "Cross-play is unavailable with modded Java."
                } else {
                    "Vanilla has no plugin API for Geyser."
                },
            )],
            _ => vec![field(
                "broadcast",
                "Enable Xbox Broadcast",
                Toggle,
                "Lets console, mobile, and PC players discover the server in the Xbox Friends tab.",
            )],
        }
    }
    fn get(&self, key: &'static str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or("")
    }
    fn set(&mut self, key: &'static str, value: impl Into<String>) {
        self.values.insert(key, value.into());
    }
    fn yes(&self, key: &'static str) -> bool {
        self.get(key) == "Yes"
    }
    fn version_options(&self) -> Vec<(String, String)> {
        std::iter::once(("LATEST".to_string(), "Latest".to_string()))
            .chain(self.versions.iter().map(|version| {
                let label = version
                    .build_label
                    .as_deref()
                    .map(|build| format!("{} · {build}", version.display_label))
                    .unwrap_or_else(|| version.display_label.clone());
                (version.id.clone(), label)
            }))
            .collect()
    }
    fn version_label(&self) -> String {
        self.version_label_for("version")
    }
    fn version_label_for(&self, key: &'static str) -> String {
        self.version_options()
            .into_iter()
            .find(|(id, _)| id == self.get(key))
            .map(|(_, label)| label)
            .unwrap_or_else(|| "Latest".into())
    }
    fn java_label(&self) -> String {
        self.runtimes
            .iter()
            .find(|runtime| runtime.executable_path == self.get("java"))
            .map(|runtime| {
                format!(
                    "{} · Java {}",
                    runtime.name,
                    runtime.major_version.unwrap_or_default()
                )
            })
            .unwrap_or_else(|| self.get("java").to_string())
    }
    fn java_options(&self) -> Vec<(String, String)> {
        let required = self.required_java_major();
        self.runtimes
            .iter()
            .filter(|runtime| runtime.major_version.is_some_and(|major| major >= required))
            .map(|runtime| {
                (
                    runtime.executable_path.clone(),
                    format!(
                        "Java {} · {}",
                        runtime.major_version.unwrap_or_default(),
                        runtime.name
                    ),
                )
            })
            .collect()
    }
    fn move_java_selection(&mut self, forward: bool) {
        let options = self.java_options();
        if options.is_empty() {
            return;
        }
        let current = options
            .iter()
            .position(|(path, _)| path == self.get("java"))
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % options.len()
        } else {
            (current + options.len() - 1) % options.len()
        };
        self.set("java", options[next].0.clone());
        self.message.clear();
    }
    fn move_java_install_major(&mut self, forward: bool) {
        const MAJORS: [i64; 4] = [25, 21, 17, 8];
        let current = MAJORS
            .iter()
            .position(|major| *major == self.java_install_major)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % MAJORS.len()
        } else {
            (current + MAJORS.len() - 1) % MAJORS.len()
        };
        self.java_install_major = MAJORS[next];
    }
    fn required_java_major(&self) -> i64 {
        let minecraft = if self.get("version") == "LATEST" {
            self.versions
                .iter()
                .find(|entry| entry.is_latest)
                .or_else(|| self.versions.first())
                .map(|entry| entry.mc_version.as_str())
        } else {
            self.versions
                .iter()
                .find(|entry| entry.id == self.get("version"))
                .map(|entry| entry.mc_version.as_str())
        };
        msc_domain::java_runtime::required_java_major(minecraft)
    }
    fn java(&self) -> bool {
        self.path == PathKind::Modpack || self.get("edition") == "Java"
    }
    fn world_field_available(&self, key: &str, fallback: bool) -> bool {
        self.world_capabilities
            .as_ref()
            .map_or(fallback, |capabilities| {
                capabilities
                    .fields
                    .get(key)
                    .is_some_and(|field| field.available)
            })
    }

    fn fields(&self) -> Vec<Field> {
        use FieldKind::{Action, Choice, Text, Toggle};
        match self.page() {
            Page::Choose => vec![field(
                "path",
                "How do you want to add this server?",
                Choice(&["Import or Create from Modpack", "Start Fresh"]),
                "Fresh downloads a new server; archive and folder paths use files on this host.",
            )],
            Page::ChooseSource => vec![field(
                "source_kind",
                "What do you want to bring in?",
                Choice(&["Import existing", "Modpack archive"]),
                "Choose a server folder or ZIP, or use a modpack .mrpack/.zip archive.",
            )],
            Page::Configure => self.configure_fields(),
            Page::Upload => {
                let mut rows = vec![field(
                    "source",
                    if self.path == PathKind::Modpack {
                        "Modpack .mrpack or .zip path"
                    } else {
                        "Server folder or .zip path"
                    },
                    Text,
                    "Type an absolute path on this computer. Press e to edit.",
                )];
                if self.path == PathKind::Modpack
                    && self.inspection.as_ref().is_some_and(|i| {
                        i.format == "curseforge" && i.curseforge_lookup_available == Some(false)
                    })
                {
                    rows.push(field(
                        "curseforge_key",
                        "CurseForge API key",
                        Text,
                        "Saved securely by the local agent, then used to inspect this archive.",
                    ));
                }
                rows
            }
            Page::Review => vec![
                field("name", "Display name", Text, "The name shown in MSC."),
                field(
                    "import_world",
                    "Active world",
                    Text,
                    "Use a detected world name; blank keeps the scanned default.",
                ),
                field(
                    "max_players",
                    "Max players",
                    Text,
                    "Use a positive player limit.",
                ),
                field(
                    "accept_eula",
                    "Accept Minecraft EULA",
                    Toggle,
                    "Required to run a Java server.",
                ),
            ],
            Page::Network => {
                if self.network_page_index == 0 {
                    vec![field(
                        "connection",
                        "How will friends connect?",
                        if self.path == PathKind::Import {
                            Choice(&["Port forwarding"])
                        } else {
                            Choice(&["Port forwarding", "Tunnel (playit.gg)"])
                        },
                        if self.path == PathKind::Import {
                            "Imported servers can enable a tunnel after import."
                        } else {
                            "Port forwarding uses your router; tunnel does not require router access."
                        },
                    )]
                } else {
                    let mut rows = Vec::new();
                    if self.path == PathKind::Modpack {
                        rows.push(field("name", "Server name", Text, "The name shown in MSC."));
                    }
                    let tunnel = self.get("connection") == "Tunnel (playit.gg)";
                    rows.push(field(
                        "port",
                        match (self.java(), tunnel) {
                            (true, true) => "Local port (TCP)",
                            (true, false) => "Java port (TCP)",
                            (false, true) => "Local port (UDP)",
                            (false, false) => "Bedrock port (UDP)",
                        },
                        Text,
                        "Choose a free port from 1 to 65535.",
                    ));
                    if !self.java() {
                        rows.push(field(
                            "max_players",
                            "Max players",
                            Text,
                            "Bedrock player limit; defaults to 20.",
                        ));
                    }
                    if self.java() && self.yes("crossplay") {
                        rows.push(field(
                            "bedrock_port",
                            if tunnel {
                                "Bedrock / Geyser local port (UDP)"
                            } else {
                                "Bedrock / Geyser port (UDP)"
                            },
                            Text,
                            "Bedrock clients use this port.",
                        ));
                    }
                    rows
                }
            }
            Page::World => match self.world_page_index {
                0 => vec![field(
                    "world_source",
                    "World source",
                    Choice(&["New world", "Backup .zip"]),
                    "Start with a new world or bring one in from a backup.",
                )],
                _ if self.get("world_source") == "Backup .zip" => vec![field(
                    "backup",
                    "Backup .zip path",
                    Text,
                    "Choose an existing .zip backup on this computer.",
                )],
                _ => vec![
                    field(
                        "world_name",
                        "World name",
                        Text,
                        "Display name for this world slot.",
                    ),
                    field("seed", "Seed", Text, "Optional; random if blank."),
                    field(
                        "difficulty",
                        "Difficulty",
                        Choice(&["Peaceful", "Easy", "Normal", "Hard"]),
                        "First world difficulty.",
                    ),
                    field(
                        "gamemode",
                        "Default game mode",
                        Choice(&["Survival", "Creative", "Adventure", "Spectator"]),
                        "Default mode for new players.",
                    ),
                ],
            },
            Page::Generation => {
                let mut rows = Vec::new();
                if self.world_page_index == 2
                    && self.world_field_available("identity.level-name", true)
                {
                    rows.push(field(
                        "level_name",
                        "Minecraft folder name",
                        Text,
                        "Blank uses the default folder name.",
                    ));
                }
                if self.world_page_index == 2
                    && self.world_field_available("generation.world-type", true)
                {
                    rows.push(field(
                        "world_type",
                        "World type",
                        if self.java() {
                            Choice(&[
                                "Default",
                                "Flat",
                                "Large biomes",
                                "Amplified",
                                "Single biome",
                            ])
                        } else {
                            Choice(&["Default", "Flat"])
                        },
                        "Terrain generation.",
                    ));
                }
                for (key, label, hint) in [
                    (
                        "flat_preset",
                        "Flat preset JSON",
                        "Optional, for Flat worlds.",
                    ),
                    ("biome_source", "Biome source", "Optional biome ID or JSON."),
                    (
                        "structures",
                        "Generate structures",
                        "Villages and other generated structures.",
                    ),
                    (
                        "bonus_chest",
                        "Bonus chest",
                        "Only affects a newly generated world.",
                    ),
                    (
                        "generator_options",
                        "Generator options JSON",
                        "Optional advanced generation settings.",
                    ),
                ] {
                    if (self.world_page_index == 2
                        && !matches!(key, "flat_preset" | "biome_source"))
                        || (self.world_page_index == 3
                            && matches!(key, "flat_preset" | "biome_source"))
                    {
                        continue;
                    }
                    let (capability, fallback) = match key {
                        "flat_preset" => ("generation.flat-preset", self.java()),
                        "biome_source" => ("generation.biome-source", self.java()),
                        "structures" => ("generation.structures", self.java()),
                        "bonus_chest" => ("generation.bonus-chest", true),
                        "generator_options" => ("generation.generator-options", self.java()),
                        _ => unreachable!(),
                    };
                    if self.world_field_available(capability, fallback) {
                        let kind = if matches!(key, "structures" | "bonus_chest" | "starting_map") {
                            Toggle
                        } else {
                            Text
                        };
                        rows.push(field(key, label, kind, hint));
                    }
                }
                if self.world_page_index == 3
                    && self.java()
                    && self.world_field_available("generation.data-packs", true)
                {
                    rows.push(field(
                        "datapacks",
                        "Data pack IDs",
                        Text,
                        "Optional comma-separated data pack IDs.",
                    ));
                }
                rows
            }
            Page::Gameplay if !self.java() => {
                if self.world_page_index == 5 {
                    return vec![
                        field(
                            "gamerules",
                            if self.gamerules_text_saved || self.editing.is_some() {
                                "Game rules"
                            } else {
                                "Edit rules as text"
                            },
                            Text,
                            "Comma-separated rule=value pairs.",
                        ),
                        field(
                            "skip_rule_editor",
                            "Skip rule editor",
                            Action,
                            "Continue without changing game rules.",
                        ),
                    ];
                }
                if self.world_page_index == 6 {
                    return vec![
                        field(
                            "world_pack",
                            "Behavior pack path",
                            Text,
                            "Optional local .mcpack or .zip installed in the first world.",
                        ),
                        field(
                            "skip_world_pack",
                            "Skip behavior pack",
                            Action,
                            "Continue without importing a behavior pack.",
                        ),
                    ];
                }
                let mut rows = Vec::new();
                for (key, label, hint, capability) in [
                    (
                        "cheats",
                        "Allow cheats",
                        "Bedrock worlds use this to enable commands.",
                        "gameplay.cheats",
                    ),
                    (
                        "coordinates",
                        "Show coordinates",
                        "Show player coordinates in game.",
                        "gameplay.coordinates",
                    ),
                    (
                        "experiments",
                        "Experiments",
                        "Optional name=true values, separated by commas.",
                        "gameplay.experiments",
                    ),
                    (
                        "gamerules",
                        "Game rules",
                        "Optional comma-separated name=value pairs.",
                        "gameplay.gamerules",
                    ),
                    (
                        "starting_map",
                        "Starting map",
                        "Give Bedrock players a map when they spawn.",
                        "gameplay.starting-map",
                    ),
                ] {
                    if self.world_field_available(capability, true) {
                        rows.push(field(
                            key,
                            label,
                            if matches!(key, "experiments" | "gamerules") {
                                Text
                            } else {
                                Toggle
                            },
                            hint,
                        ));
                    }
                }
                rows
            }
            Page::Gameplay => {
                if self.world_page_index == 5 {
                    return vec![
                        field(
                            "gamerules",
                            if self.gamerules_text_saved || self.editing.is_some() {
                                "Game rules"
                            } else {
                                "Edit rules as text"
                            },
                            Text,
                            "Comma-separated rule=value pairs.",
                        ),
                        field(
                            "skip_rule_editor",
                            "Skip rule editor",
                            Action,
                            "Continue without changing game rules.",
                        ),
                    ];
                }
                if self.world_page_index == 6 {
                    return vec![
                        field(
                            "world_pack",
                            "Datapack ZIP path",
                            Text,
                            "Optional local .zip installed in the first world.",
                        ),
                        field(
                            "skip_world_pack",
                            "Skip datapack",
                            Action,
                            "Continue without importing a datapack.",
                        ),
                    ];
                }
                let mut rows = Vec::new();
                for (key, label, hint, capability) in [
                    (
                        "hardcore",
                        "Hardcore",
                        "One life; changes the first world.",
                        "gameplay.hardcore",
                    ),
                    (
                        "commands",
                        "Allow commands",
                        "Let operators use commands.",
                        "gameplay.commands",
                    ),
                    (
                        "gamerules",
                        "Game rules",
                        "Optional comma-separated name=value pairs.",
                        "gameplay.gamerules",
                    ),
                ] {
                    if self.world_field_available(capability, true) {
                        rows.push(field(
                            key,
                            label,
                            if key == "gamerules" { Text } else { Toggle },
                            hint,
                        ));
                    }
                }
                rows
            }
            Page::Addons => vec![
                field(
                    "addon",
                    "Local plugin or mod path",
                    Text,
                    "Optional .jar path on this computer. Modrinth browsing is left out of this terminal flow.",
                ),
                field(
                    "skip_addon",
                    "Skip plugins",
                    Action,
                    "Continue without adding a plugin or mod.",
                ),
            ],
            Page::Confirm => Vec::new(),
        }
    }

    pub(super) async fn handle(&mut self, key: KeyEvent, out: &mut Stdout) -> Outcome {
        if self.pending_operation_id.is_some() && key.code != KeyCode::Enter {
            return Outcome::Stay;
        }
        if let Some(buffer) = self.editing.as_mut() {
            match key.code {
                KeyCode::Esc => self.editing = None,
                KeyCode::Enter => {
                    let value = self.editing.take().unwrap_or_default();
                    if let Some(field) = self.fields().get(self.cursor) {
                        self.set(field.key, value);
                    }
                    if self.page() == Page::Gameplay && self.world_page_index == 5 {
                        self.gamerules_text_saved = true;
                        self.message = "Rules saved. Press Enter to continue to datapacks.".into();
                    }
                }
                KeyCode::Backspace => {
                    buffer.pop();
                }
                KeyCode::Char(c) => buffer.push(c),
                _ => {}
            }
            return Outcome::Stay;
        }
        let fields = self.fields();
        match key.code {
            KeyCode::Esc => {
                if self.java_install_page {
                    self.java_install_page = false;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.version_picker_active {
                    self.version_picker_active = false;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.page() == Page::Network && self.network_page_index > 0 {
                    self.network_page_index -= 1;
                    self.cursor = 0;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.page() == Page::Configure && self.configure_page_index > 0 {
                    self.configure_page_index -= 1;
                    self.cursor = 0;
                    self.message.clear();
                    self.confirmation = None;
                    return Outcome::Stay;
                }
                if matches!(self.page(), Page::World | Page::Generation | Page::Gameplay)
                    && self.world_page_index > 0
                {
                    match self.world_page_index {
                        2 => self.page_index = self.page_index.saturating_sub(1),
                        4 => self.page_index = self.page_index.saturating_sub(1),
                        _ => {}
                    }
                    if self.world_page_index == 5 {
                        self.gamerules_text_saved = false;
                    }
                    self.world_page_index -= 1;
                    self.cursor = 0;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.page_index == 0 {
                    return Outcome::Close;
                }
                self.page_index -= 1;
                self.cursor = 0;
                self.message.clear();
                self.confirmation = None;
            }
            KeyCode::Up => {
                if self.page() == Page::Configure
                    && self.is_java_runtime_page()
                    && !self.java_install_page
                {
                    self.move_java_selection(false);
                } else if self.java_install_page {
                    self.move_java_install_major(false);
                } else {
                    self.cursor = self.cursor.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if self.page() == Page::Configure
                    && self.is_java_runtime_page()
                    && !self.java_install_page
                {
                    self.move_java_selection(true);
                } else if self.java_install_page {
                    self.move_java_install_major(true);
                } else {
                    self.cursor = (self.cursor + 1).min(fields.len().saturating_sub(1));
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                if let Some(field) = fields.get(self.cursor) {
                    if field.key == "java" && self.is_java_runtime_page() {
                        return Outcome::Stay;
                    }
                    if field.key == "source_mode" && self.version_picker_active {
                        match key.code {
                            KeyCode::Left | KeyCode::Right => {
                                let options = self.version_options();
                                let current = options
                                    .iter()
                                    .position(|(id, _)| id == self.get("version"))
                                    .unwrap_or(0);
                                let next = if key.code == KeyCode::Left {
                                    (current + options.len() - 1) % options.len()
                                } else {
                                    (current + 1) % options.len()
                                };
                                self.set("version", options[next].0.clone());
                                self.message.clear();
                            }
                            _ => {}
                        }
                        return Outcome::Stay;
                    }
                    if matches!(field.key, "version" | "bedrock_version")
                        && !self.versions.is_empty()
                    {
                        let options = self.version_options();
                        let current = options
                            .iter()
                            .position(|(id, _)| id == self.get(field.key))
                            .unwrap_or(0);
                        let next = if key.code == KeyCode::Left {
                            (current + options.len() - 1) % options.len()
                        } else {
                            (current + 1) % options.len()
                        };
                        self.set(field.key, options[next].0.clone());
                        return Outcome::Stay;
                    }
                    if field.key == "import_world"
                        && let Some(scan) = &self.scan
                        && !scan.worlds.is_empty()
                    {
                        let current = scan
                            .worlds
                            .iter()
                            .position(|world| world.name == self.get("import_world"))
                            .unwrap_or(0);
                        let next = if key.code == KeyCode::Left {
                            (current + scan.worlds.len() - 1) % scan.worlds.len()
                        } else {
                            (current + 1) % scan.worlds.len()
                        };
                        let world_name = scan.worlds[next].name.clone();
                        self.set("import_world", world_name);
                        return Outcome::Stay;
                    }
                    match &field.kind {
                        FieldKind::Choice(choices) => {
                            let current = choices
                                .iter()
                                .position(|v| *v == self.get(field.key))
                                .unwrap_or(0);
                            let next = if key.code == KeyCode::Left {
                                (current + choices.len() - 1) % choices.len()
                            } else {
                                (current + 1) % choices.len()
                            };
                            self.set(field.key, choices[next]);
                            if field.key == "path" {
                                self.path = match choices[next] {
                                    "Start Fresh" => PathKind::Fresh,
                                    _ => PathKind::Route,
                                };
                            }
                            if field.key == "edition" {
                                self.set("port", if next == 0 { "25565" } else { "19132" });
                                if next == 1 {
                                    self.set("crossplay", "No");
                                }
                            }
                            if field.key == "java_category" {
                                self.set("software", if next == 0 { "Paper" } else { "Fabric" });
                                self.set("crossplay", "No");
                                self.set("broadcast", "No");
                            }
                            if field.key == "software" && choices[next] == "Vanilla" {
                                self.set("crossplay", "No");
                                self.set("broadcast", "No");
                            }
                            if field.key == "source_mode" {
                                self.set("version", "LATEST");
                                self.version_picker_active = false;
                            }
                        }
                        FieldKind::Toggle => {
                            if field.key != "crossplay" || self.crossplay_available() {
                                self.set(field.key, if self.yes(field.key) { "No" } else { "Yes" })
                            }
                        }
                        FieldKind::Text | FieldKind::Action => {}
                    }
                }
            }
            KeyCode::Char('e') => {
                if self.page() == Page::Gameplay
                    && self.world_page_index == 4
                    && fields
                        .get(self.cursor)
                        .is_some_and(|field| field.key == "gamerules")
                {
                    self.world_page_index = 5;
                    self.gamerules_text_saved = false;
                    self.cursor = 0;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if let Some(field) = fields.get(self.cursor)
                    && !matches!(field.key, "version" | "bedrock_version")
                    && matches!(field.kind, FieldKind::Text)
                {
                    if self.page() == Page::Gameplay && self.world_page_index == 5 {
                        self.gamerules_text_saved = false;
                        self.message.clear();
                    }
                    self.editing = Some(self.get(field.key).to_string());
                }
            }
            KeyCode::Char('a')
                if self.page() == Page::Gameplay
                    && self.world_page_index == 4
                    && fields
                        .get(self.cursor)
                        .is_some_and(|field| field.key == "gamerules") =>
            {
                let rules = self.get("gamerules");
                self.editing = Some(if rules.is_empty() {
                    String::new()
                } else {
                    format!("{rules},")
                });
            }
            KeyCode::Char('i') if self.page() == Page::Configure && self.is_java_runtime_page() => {
                self.java_install_major = self.required_java_major();
                self.java_install_page = true;
                self.message.clear();
            }
            KeyCode::Enter => {
                if let Some(field) = fields.get(self.cursor) {
                    match field.key {
                        "skip_rule_editor" => {
                            self.world_page_index = 6;
                            self.cursor = 0;
                            self.message.clear();
                            return Outcome::Stay;
                        }
                        "skip_world_pack" | "skip_addon" => {
                            if field.key == "skip_world_pack" {
                                self.set("world_pack", "");
                            } else {
                                self.set("addon", "");
                            }
                            if let Err(error) = self.advance(out).await {
                                self.message = error.to_string();
                            }
                            return Outcome::Stay;
                        }
                        _ => {}
                    }
                }
                if self.java_install_page {
                    match self.install_java(out, self.java_install_major).await {
                        Ok(path) => {
                            self.java_install_page = false;
                            if self.java_install_major >= self.required_java_major() {
                                self.set("java", path);
                                self.message =
                                    "Java installed and selected for this server.".into();
                            } else {
                                if !self
                                    .java_options()
                                    .iter()
                                    .any(|(runtime_path, _)| runtime_path == self.get("java"))
                                {
                                    self.set("java", "");
                                }
                                self.message = format!(
                                    "Java {} installed, but this server needs Java {} or newer.",
                                    self.java_install_major,
                                    self.required_java_major()
                                );
                            }
                        }
                        Err(error) => self.message = error.to_string(),
                    }
                    return Outcome::Stay;
                }
                if self.page() == Page::Configure
                    && self.is_java_runtime_page()
                    && self.get("java").is_empty()
                {
                    self.message = format!(
                        "No compatible Java runtime is selected. Press i to install Java {}.",
                        self.required_java_major()
                    );
                    return Outcome::Stay;
                }
                if self.page() == Page::Configure
                    && self.java()
                    && self.configure_page_index == 4
                    && self.get("source_mode") == "Choose version…"
                {
                    if !self.version_picker_active {
                        self.version_picker_active = true;
                        self.message.clear();
                        return Outcome::Stay;
                    }
                    self.version_picker_active = false;
                }
                if self.page() == Page::ChooseSource {
                    self.path = if self.get("source_kind") == "Modpack archive" {
                        PathKind::Modpack
                    } else {
                        self.set("connection", "Port forwarding");
                        self.set("max_players", "20");
                        PathKind::Import
                    };
                    self.cursor = 0;
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.page() == Page::Gameplay
                    && self.world_page_index == 5
                    && self.editing.is_none()
                    && !self.gamerules_text_saved
                    && fields
                        .get(self.cursor)
                        .is_some_and(|field| field.key == "gamerules")
                {
                    self.editing = Some(self.get("gamerules").to_string());
                    self.message.clear();
                    return Outcome::Stay;
                }
                if self.page() == Page::Confirm {
                    if self.submitting {
                        return Outcome::Stay;
                    }
                    self.submitting = true;
                    let result = self.submit(out).await;
                    self.submitting = false;
                    return match result {
                        Ok(note) => Outcome::Created(note),
                        Err(error) => {
                            self.confirmation = error
                                .json_message
                                .as_deref()
                                .and_then(|body| {
                                    serde_json::from_str::<serde_json::Value>(body).ok()
                                })
                                .and_then(|body| {
                                    body["details"]["confirmation"]["acknowledgement"]
                                        .as_str()
                                        .map(str::to_string)
                                });
                            self.message = if self.pending_operation_id.is_some() {
                                format!(
                                    "{error} · Press Enter to check the submitted operation again."
                                )
                            } else if self.confirmation.is_some() {
                                "This world disables Bedrock achievements. Press Enter again to confirm; Esc goes back.".into()
                            } else {
                                error.to_string()
                            };
                            Outcome::Stay
                        }
                    };
                }
                if let Err(error) = self.advance(out).await {
                    self.message = error.to_string();
                }
            }
            _ => {}
        }
        Outcome::Stay
    }

    async fn advance(&mut self, out: &mut Stdout) -> Result<(), CliError> {
        self.message.clear();
        if self.page() == Page::Configure {
            if self.configure_page_index == 1 && self.get("name").trim().is_empty() {
                return Err(CliError::usage("Enter a server name."));
            }
            if self.is_java_runtime_page() && self.get("java").is_empty() {
                return Err(CliError::usage(format!(
                    "No compatible Java runtime is selected. Press i to install Java {} before continuing.",
                    self.required_java_major()
                )));
            }
            if self.configure_page_index + 1 < self.configure_page_count() {
                self.configure_page_index += 1;
                self.cursor = 0;
                if (self.java() && self.configure_page_index == 4)
                    || (!self.java() && self.configure_page_index == 2)
                {
                    self.progress(
                        out,
                        if self.java() {
                            "Loading Minecraft versions and Java runtimes…"
                        } else {
                            "Loading Bedrock versions…"
                        },
                    )?;
                    self.load_configure_options().await;
                }
                if self.is_java_runtime_page() {
                    let options = self.java_options();
                    if !options.iter().any(|(path, _)| path == self.get("java")) {
                        self.set(
                            "java",
                            options
                                .first()
                                .map(|(path, _)| path.clone())
                                .unwrap_or_default(),
                        );
                    }
                }
                return Ok(());
            }
        }
        match self.page() {
            Page::Choose | Page::ChooseSource => {}
            Page::Configure => {}
            Page::Upload => {
                if self.path == PathKind::Modpack && self.staged_modpack.is_some() {
                    if self.inspection.as_ref().is_some_and(|i| {
                        i.format == "curseforge" && i.curseforge_lookup_available == Some(false)
                    }) {
                        let key = self.get("curseforge_key").trim();
                        if key.is_empty() {
                            return Err(CliError::usage(
                                "This CurseForge archive needs an API key. Select the key field, press e, enter the key, then press Enter.",
                            ));
                        }
                        let client = SharedClient::connect_local().await?;
                        let status: serde_json::Value = client
                            .post_json("/v1/config/curseforge", &json!({"apiKey":key}))
                            .await?;
                        if status["configured"] != true {
                            return Err(CliError::usage(
                                "The agent did not save the CurseForge API key.",
                            ));
                        }
                        let id = self.staged_modpack.as_deref().unwrap_or_default();
                        let inspection: ModpackInspectionResultDto = client
                            .post_json("/v1/modpacks/inspect", &json!({"stagedUploadId":id}))
                            .await?;
                        if !inspection.success
                            || inspection.curseforge_lookup_available == Some(false)
                        {
                            return Err(CliError::usage(
                                "CurseForge files could not be resolved with this key.",
                            ));
                        }
                        self.inspection = Some(inspection);
                        self.set("curseforge_key", "");
                    }
                    self.page_index += 1;
                    self.cursor = 0;
                    return Ok(());
                }
                let source = self.get("source").trim().to_string();
                if source.is_empty() {
                    return Err(CliError::usage("Enter a source path before continuing."));
                }
                let path = Path::new(&source);
                if !path.is_absolute() || !path.exists() {
                    return Err(CliError::usage(
                        "Source must be an existing absolute path on this computer.",
                    ));
                }
                let client = SharedClient::connect_local().await?;
                if self.path == PathKind::Import {
                    let kind = if path.is_dir() { "folder" } else { "zip" };
                    let scan: ServerImportScanResponseDto = client
                        .post_json(
                            "/v1/servers/import",
                            &json!({"action":"scan","sourcePath":source,"importKind":kind}),
                        )
                        .await?;
                    if !scan.success {
                        return Err(CliError::usage(scan.message));
                    }
                    self.set(
                        "edition",
                        if scan.server_type.as_deref() == Some("bedrock") {
                            "Bedrock"
                        } else {
                            "Java"
                        },
                    );
                    if let Some(flavor) = &scan.java_flavor {
                        self.set("software", flavor.clone());
                    }
                    let default_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Imported server")
                        .replace('_', " ");
                    self.set("name", default_name);
                    if let Some(port) = scan.port {
                        self.set("port", port.to_string());
                    } else if scan.server_type.as_deref() == Some("bedrock") {
                        self.set("port", "19132");
                    }
                    if let Some(limit) = scan.max_players {
                        self.set("max_players", limit.to_string());
                    }
                    if let Some(world) = &scan.default_world_name {
                        self.set("import_world", world.clone());
                    }
                    self.set(
                        "accept_eula",
                        if scan.eula_accepted == Some(true) {
                            "Yes"
                        } else {
                            "No"
                        },
                    );
                    self.scan = Some(scan);
                } else {
                    let archive = path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| {
                            matches!(extension.to_ascii_lowercase().as_str(), "mrpack" | "zip")
                        });
                    if !path.is_file() || !archive {
                        return Err(CliError::usage(
                            "A modpack source must be a .mrpack or .zip file.",
                        ));
                    }
                    self.progress(out, "Uploading modpack archive")?;
                    let id = stage_file_upload(
                        &client,
                        path,
                        StagedUploadPurposeDto::ModpackArchive,
                        None,
                        None,
                        None,
                    )
                    .await?;
                    self.progress(out, "Inspecting modpack")?;
                    let inspection: ModpackInspectionResultDto = client
                        .post_json("/v1/modpacks/inspect", &json!({"stagedUploadId":id}))
                        .await?;
                    if !inspection.success {
                        return Err(CliError::usage(inspection.message));
                    }
                    self.set(
                        "name",
                        inspection
                            .pack_name
                            .as_deref()
                            .unwrap_or("Modpack server")
                            .to_string(),
                    );
                    if let Some(loader) = &inspection.loader_name {
                        let loader = loader.to_ascii_lowercase();
                        let flavor = if loader.contains("neoforge") {
                            "neoforge"
                        } else if loader.contains("forge") {
                            "forge"
                        } else if loader.contains("fabric") {
                            "fabric"
                        } else if loader.contains("purpur") {
                            "purpur"
                        } else if loader.contains("vanilla") {
                            "vanilla"
                        } else {
                            "paper"
                        };
                        self.set("software", flavor);
                    }
                    self.staged_modpack = Some(id);
                    self.inspection = Some(inspection);
                    if self.inspection.as_ref().is_some_and(|i| {
                        i.format == "curseforge" && i.curseforge_lookup_available == Some(false)
                    }) {
                        return Err(CliError::usage(
                            "CurseForge requires an API key. Enter it below to continue.",
                        ));
                    }
                }
            }
            Page::Review => {
                if self.get("name").trim().is_empty() {
                    return Err(CliError::usage("Enter a display name."));
                }
                if self
                    .get("max_players")
                    .parse::<u32>()
                    .ok()
                    .filter(|value| (1..=10_000).contains(value))
                    .is_none()
                {
                    return Err(CliError::usage("Max players must be from 1 to 10,000."));
                }
            }
            Page::Network => {
                if self.network_page_index == 0 {
                    self.network_page_index = 1;
                    self.cursor = 0;
                    return Ok(());
                }
                if self.path == PathKind::Modpack && self.get("name").trim().is_empty() {
                    return Err(CliError::usage("Enter a server name."));
                }
                for key in ["port", "bedrock_port"] {
                    if key == "bedrock_port" && !self.yes("crossplay") {
                        continue;
                    }
                    if self
                        .get(key)
                        .parse::<u16>()
                        .ok()
                        .filter(|v| *v > 0)
                        .is_none()
                    {
                        return Err(CliError::usage(format!(
                            "{key} must be a port from 1 to 65535."
                        )));
                    }
                }
                if !self.java()
                    && self
                        .get("max_players")
                        .parse::<u32>()
                        .ok()
                        .filter(|n| (1..=10_000).contains(n))
                        .is_none()
                {
                    return Err(CliError::usage("Max players must be from 1 to 10,000."));
                }
            }
            Page::World => {
                if self.world_page_index == 0 {
                    self.world_page_index = 1;
                    self.cursor = 0;
                    return Ok(());
                }
                if self.get("world_source") == "Backup .zip" {
                    let path = Path::new(self.get("backup"));
                    let is_zip = path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
                    if !path.is_absolute() || !path.is_file() || !is_zip {
                        return Err(CliError::usage(
                            "Enter an existing absolute .zip backup path.",
                        ));
                    }
                }
                if self.get("world_source") != "Backup .zip" {
                    self.page_index += 1;
                    self.world_page_index = 2;
                    self.cursor = 0;
                    return Ok(());
                }
            }
            Page::Generation => {
                if self.world_page_index == 2 {
                    self.world_page_index = 3;
                    self.cursor = 0;
                    return Ok(());
                }
                self.page_index += 1;
                self.world_page_index = 4;
                self.cursor = 0;
                return Ok(());
            }
            Page::Gameplay => {
                if self.world_page_index == 4 {
                    self.world_page_index = 5;
                    self.cursor = 0;
                    return Ok(());
                }
                if self.world_page_index == 5 {
                    self.world_page_index = 6;
                    self.cursor = 0;
                    return Ok(());
                }
                if self.world_page_index == 6 {
                    let pack_path = Path::new(self.get("world_pack"));
                    let pack_extension = pack_path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .unwrap_or_default();
                    let valid_pack = if self.java() {
                        pack_extension.eq_ignore_ascii_case("zip")
                    } else {
                        pack_extension.eq_ignore_ascii_case("mcpack")
                            || pack_extension.eq_ignore_ascii_case("zip")
                    };
                    if !self.get("world_pack").is_empty() && (!pack_path.is_file() || !valid_pack) {
                        return Err(CliError::usage(
                            "Use a .zip world pack for Java, or a .mcpack/.zip behavior pack for Bedrock.",
                        ));
                    }
                }
                for (key, label, boolean) in [
                    ("gamerules", "Game rules", false),
                    ("experiments", "Experiments", true),
                ] {
                    for pair in self
                        .get(key)
                        .split(',')
                        .map(str::trim)
                        .filter(|pair| !pair.is_empty())
                    {
                        let (name, value) = pair.split_once('=').ok_or_else(|| {
                            CliError::usage(format!(
                                "{label} must be comma-separated name=value pairs."
                            ))
                        })?;
                        if name.trim().is_empty() || value.trim().is_empty() {
                            return Err(CliError::usage(format!(
                                "Each {label} entry needs a name and value."
                            )));
                        }
                        if boolean && value.trim().parse::<bool>().is_err() {
                            return Err(CliError::usage(
                                "Experiment values must be true or false.",
                            ));
                        }
                    }
                }
                self.page_index += 1;
                self.cursor = 0;
                return Ok(());
            }
            Page::Addons
                if !self.get("addon").is_empty()
                    && (!Path::new(self.get("addon")).is_file()
                        || !Path::new(self.get("addon"))
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))) =>
            {
                return Err(CliError::usage(
                    "Local add-on must be a .jar file on this computer.",
                ));
            }
            _ => {}
        }
        self.page_index += 1;
        self.cursor = 0;
        if self.page() == Page::World {
            self.load_world_capabilities().await;
        }
        Ok(())
    }

    async fn load_configure_options(&mut self) {
        let edition = if self.java() { "java" } else { "bedrock" };
        let flavor = self.get("software").to_ascii_lowercase();
        let path = if edition == "java" {
            format!("/v1/versions/create?serverType=java&javaFlavor={flavor}")
        } else {
            "/v1/versions/create?serverType=bedrock".into()
        };
        let client = match SharedClient::connect_local().await {
            Ok(client) => client,
            Err(error) => {
                self.message = format!("Could not connect to the agent to load versions: {error}");
                return;
            }
        };
        // A cold provider lookup can take longer than the local CLI's default read window.
        let response = match client
            .get_json_with_timeout::<VersionsResponseDto>(&path, Duration::from_secs(30))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.message = format!("Could not load Minecraft versions: {error}");
                return;
            }
        };
        self.versions = response.versions;
        self.set("version", "LATEST");
        if !self.java() && !self.versions.is_empty() {
            self.set("bedrock_version", "LATEST");
        }
        if self.java() {
            match client
                .get_json::<JavaRuntimesResponseDto>("/v1/java-runtimes")
                .await
            {
                Ok(runtime_response) => self.runtimes = runtime_response.runtimes,
                Err(error) => {
                    self.message = format!("Could not check installed Java runtimes: {error}");
                }
            }
        }
    }

    async fn load_world_capabilities(&mut self) {
        self.world_capabilities = None;
        let client = match SharedClient::connect_local().await {
            Ok(client) => client,
            Err(_) => {
                self.message =
                    "Could not check version-specific world settings; using edition defaults."
                        .into();
                return;
            }
        };
        let server_type = if self.java() { "java" } else { "bedrock" };
        let inspection = self.inspection.as_ref();
        let selected = if self.path == PathKind::Modpack {
            inspection.and_then(|info| info.minecraft_version.as_deref())
        } else if self.java() {
            let id = self.get("version");
            self.versions
                .iter()
                .find(|version| version.id == id)
                .map(|version| version.mc_version.as_str())
                .or_else(|| {
                    self.versions
                        .iter()
                        .find(|version| version.is_latest)
                        .or_else(|| self.versions.first())
                        .map(|version| version.mc_version.as_str())
                })
        } else {
            let id = self.get("bedrock_version");
            self.versions
                .iter()
                .find(|version| version.id == id || version.mc_version == id)
                .map(|version| version.mc_version.as_str())
                .or_else(|| {
                    self.versions
                        .iter()
                        .find(|version| version.is_latest)
                        .or_else(|| self.versions.first())
                        .map(|version| version.mc_version.as_str())
                })
        };
        let mut query = format!("serverType={server_type}");
        if let Some(version) = selected {
            query.push_str("&minecraftVersion=");
            query.push_str(&encode_query_component(version));
        }
        if self.java() {
            query.push_str("&javaFlavor=");
            query.push_str(&encode_query_component(
                &self.get("software").to_ascii_lowercase(),
            ));
        }
        if let Some(loader) = inspection.and_then(|info| info.loader_version.as_deref()) {
            query.push_str("&loaderVersion=");
            query.push_str(&encode_query_component(loader));
        }
        if let Ok(response) = client
            .get_json::<CapabilitiesResponseDto>(&format!("/v1/capabilities?{query}"))
            .await
        {
            self.world_capabilities = response.world_settings;
        } else {
            self.message =
                "Could not check version-specific world settings; using edition defaults.".into();
        }
    }

    pub(super) fn draw(&self, out: &mut Stdout, width: u16, height: u16) -> io::Result<()> {
        let w = (width - 8).min(116);
        let h = (height - 4).min(30);
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        for row in y..y + h {
            queue!(out, MoveTo(x, row), Print(" ".repeat(w as usize)))?;
        }
        box_outline(out, x, y, w, h)?;
        let step_labels = self.step_labels();
        let page = self.page();
        paint(out, x + 2, y + 1, w - 24, "Add Server", Color::White)?;
        paint(
            out,
            x + w - 22,
            y + 1,
            20,
            &format!("Step {} of {}", self.step_number(), step_labels.len()),
            Color::DarkGrey,
        )?;
        paint(
            out,
            x + 2,
            y + 3,
            w - 4,
            &step_labels
                .iter()
                .enumerate()
                .map(|(i, label)| format!("{} {}", i + 1, label))
                .collect::<Vec<_>>()
                .join("  ·  "),
            Color::DarkGrey,
        )?;
        if page != Page::Configure && page != Page::Network {
            let world_page = matches!(page, Page::World | Page::Generation | Page::Gameplay);
            let title = self
                .world_capabilities
                .as_ref()
                .filter(|_| world_page)
                .map(|capabilities| {
                    let context = &capabilities.context;
                    let mut detail = context
                        .minecraft_version
                        .as_deref()
                        .unwrap_or("version unknown")
                        .to_string();
                    if let Some(flavor) = context.java_flavor.as_deref() {
                        detail.push_str(" · ");
                        detail.push_str(flavor);
                    }
                    format!("{} · {}", page.title(), detail)
                })
                .unwrap_or_else(|| page.title().to_string());
            if page == Page::Addons {
                paint(out, x + 2, y + 5, w - 4, "Page 1 / 1", Color::DarkGrey)?;
                paint(out, x + 2, y + 6, w - 4, "Add plugins?", Color::White)?;
                paint(
                    out,
                    x + 2,
                    y + 7,
                    w - 4,
                    "Add local files now, or skip and add them after the server is created. Modrinth browsing is omitted from this TUI flow.",
                    Color::DarkGrey,
                )?;
            } else if world_page {
                let page_number = self.world_page_index + 1;
                let page_count = self.world_page_count();
                let (heading, subtitle) = match self.world_page_index {
                    0 => (
                        "What should the first world be?",
                        "Start with a new world or bring one in from a backup.",
                    ),
                    1 => (
                        if self.get("world_source") == "Backup .zip" {
                            "Choose a world backup"
                        } else {
                            "What should the first world be?"
                        },
                        if self.get("world_source") == "Backup .zip" {
                            "Enter the absolute path to an existing .zip backup."
                        } else {
                            "Choose its name, seed, difficulty, and default game mode."
                        },
                    ),
                    2 | 3 => (
                        "What should the first world be?",
                        "Terrain, folder, and first-creation options.",
                    ),
                    4 => (
                        "What should the first world be?",
                        "Commands, gamerules, cheats, and edition-specific options.",
                    ),
                    5 => (
                        if self.editing.is_some() {
                            "Editing game rules"
                        } else if self.gamerules_text_saved {
                            "Game rules saved"
                        } else {
                            "Edit game rules"
                        },
                        if self.editing.is_some() {
                            "Enter saves the rule list. Esc cancels editing."
                        } else if self.gamerules_text_saved {
                            "Press Enter to continue to datapacks, or e to change these rules."
                        } else {
                            "Press Enter to edit rules, or choose Skip rule editor."
                        },
                    ),
                    _ if !self.java() => (
                        "Behavior pack",
                        "This installs in the first world after the server is created.",
                    ),
                    _ => (
                        "Datapacks",
                        "These install in the first world after the server is created.",
                    ),
                };
                paint(
                    out,
                    x + 2,
                    y + 5,
                    w - 4,
                    &format!("Page {page_number} / {page_count}"),
                    Color::DarkGrey,
                )?;
                paint(out, x + 2, y + 6, w - 4, heading, Color::White)?;
                paint(out, x + 2, y + 7, w - 4, subtitle, Color::DarkGrey)?;
            } else {
                paint(out, x + 2, y + 5, w - 4, &title, Color::White)?;
            }
        }
        let mut row = if page == Page::Configure {
            y + 10
        } else if page == Page::Addons {
            y + 9
        } else if matches!(page, Page::World | Page::Generation | Page::Gameplay) {
            y + 10
        } else {
            y + 7
        };
        if page == Page::Review
            && let Some(scan) = &self.scan
        {
            for line in [
                format!(
                    "Detected: {} · {}",
                    scan.server_type.as_deref().unwrap_or("Unknown"),
                    scan.java_flavor.as_deref().unwrap_or("")
                ),
                format!(
                    "Minecraft: {} · {} world(s)",
                    scan.detected_mc_version.as_deref().unwrap_or("Unknown"),
                    scan.worlds.len()
                ),
                format!(
                    "Worlds: {}",
                    scan.worlds
                        .iter()
                        .map(|world| world.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ] {
                paint(out, x + 3, row, w - 6, &line, Color::DarkGrey)?;
                row += 1;
            }
        }
        if page == Page::Network
            && self.path == PathKind::Modpack
            && let Some(info) = &self.inspection
        {
            paint(
                out,
                x + 3,
                row,
                w - 6,
                &format!(
                    "{} · {} · {} files",
                    info.pack_name.as_deref().unwrap_or("Modpack"),
                    info.minecraft_version
                        .as_deref()
                        .unwrap_or("Unknown version"),
                    info.file_count
                ),
                Color::DarkGrey,
            )?;
            row += 1;
        }
        if page == Page::Configure {
            self.draw_configure_page(out, x, y, w, h)?;
        } else if page == Page::Network {
            self.draw_network_page(out, x, y, w, h)?;
        } else if page == Page::World && self.world_page_index == 0 {
            let choices = [
                ("New World", "Generate a new world\nfor this server."),
                (
                    "From Backup (.zip)",
                    "Use an existing\nworld backup (.zip).",
                ),
            ];
            let selected = usize::from(self.get("world_source") == "Backup .zip");
            let card_width = (w.saturating_sub(12)) / 2;
            draw_choice_card(
                out,
                x + 2,
                y + 10,
                card_width,
                6,
                ChoiceCard {
                    title: choices[0].0,
                    description: choices[0].1,
                },
                selected == 0,
            )?;
            draw_choice_card(
                out,
                x + 8 + card_width,
                y + 10,
                card_width,
                6,
                ChoiceCard {
                    title: choices[1].0,
                    description: choices[1].1,
                },
                selected == 1,
            )?;
        } else if matches!(page, Page::Choose | Page::ChooseSource) {
            let choices = if page == Page::Choose {
                [
                    (
                        "Import or Create from Modpack",
                        "Use a local server folder or .zip,\nor create from a modpack archive.",
                    ),
                    (
                        "Start Fresh",
                        "MSC downloads and sets up a\nbrand new server from scratch.",
                    ),
                ]
            } else {
                [
                    (
                        "Import existing",
                        "Bring in a server folder or\nserver .zip from this computer.",
                    ),
                    (
                        "Modpack archive",
                        "Create a server from a local\n.mrpack or .zip archive.",
                    ),
                ]
            };
            let selected = if page == Page::Choose {
                self.get("path")
            } else {
                self.get("source_kind")
            };
            let selected_index = choices
                .iter()
                .position(|choice| choice.0 == selected)
                .unwrap_or(0);
            let card_width = (w.saturating_sub(12)) / 2;
            draw_choice_card(
                out,
                x + 2,
                y + 8,
                card_width,
                6,
                ChoiceCard {
                    title: choices[0].0,
                    description: choices[0].1,
                },
                selected_index == 0,
            )?;
            draw_choice_card(
                out,
                x + 8 + card_width,
                y + 8,
                card_width,
                6,
                ChoiceCard {
                    title: choices[1].0,
                    description: choices[1].1,
                },
                selected_index == 1,
            )?;
        } else if page == Page::Confirm {
            let edition = if self.java() { "Java" } else { "Bedrock" };
            let software = if self.path == PathKind::Modpack {
                self.inspection
                    .as_ref()
                    .and_then(|i| i.loader_name.as_deref())
                    .unwrap_or("Modpack")
            } else if self.java() {
                self.get("software")
            } else {
                "Bedrock"
            };
            let source = if self.path == PathKind::Fresh {
                "Fresh download"
            } else {
                self.get("source")
            };
            let world = if self.path == PathKind::Import {
                self.get("import_world")
            } else if self.get("world_name").is_empty() {
                self.get("name")
            } else {
                self.get("world_name")
            };
            let version = if self.path == PathKind::Modpack {
                self.inspection
                    .as_ref()
                    .and_then(|i| i.minecraft_version.clone())
                    .unwrap_or_else(|| "From modpack".into())
            } else if self.java() {
                self.version_label()
            } else {
                self.version_label_for("bedrock_version")
            };
            let eula = if self.path == PathKind::Import {
                if self.yes("accept_eula") {
                    "Accepted"
                } else {
                    "Accept later"
                }
            } else {
                "Accept later in Settings"
            };
            let left = [
                format!("Name: {}", self.get("name")),
                format!("Type: {edition} · {software}"),
                format!("Source: {source}"),
                format!("Port: {}", self.get("port")),
                format!("Version: {version}"),
                format!(
                    "Java: {}",
                    if self.java() {
                        self.java_label()
                    } else {
                        "Not used".into()
                    }
                ),
                format!(
                    "Max players: {}",
                    if self.java() {
                        "Default"
                    } else {
                        self.get("max_players")
                    }
                ),
                format!("EULA: {eula}"),
            ];
            let right = [
                format!("Connection: {}", self.get("connection")),
                format!("First world: {world}"),
                format!(
                    "World source: {}",
                    if self.path == PathKind::Import {
                        "Existing"
                    } else {
                        self.get("world_source")
                    }
                ),
                format!(
                    "Rules: {} / {}",
                    self.get("difficulty"),
                    self.get("gamemode")
                ),
                format!(
                    "Seed: {}",
                    nonempty(self.get("seed")).unwrap_or_else(|| "Random".into())
                ),
                format!(
                    "Cross-play: {} · Geyser port {}",
                    if self.yes("crossplay") { "On" } else { "Off" },
                    self.get("bedrock_port")
                ),
                format!(
                    "Xbox: {} · Voice chat: {}",
                    if self.yes("broadcast") { "On" } else { "Off" },
                    if self.yes("voice_chat") { "On" } else { "Off" }
                ),
                format!(
                    "Local add-on: {}",
                    if self.get("addon").is_empty() {
                        "None"
                    } else {
                        Path::new(self.get("addon"))
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("Selected")
                    }
                ),
            ];
            for index in 0..left.len() {
                paint(out, x + 3, row, w / 2 - 5, &left[index], Color::White)?;
                paint(out, x + w / 2, row, w / 2 - 4, &right[index], Color::White)?;
                row += 1;
            }
            paint(
                out,
                x + 3,
                row + 1,
                w - 6,
                "Enter creates or imports this server. Esc goes back to edit.",
                Color::Green,
            )?;
        } else {
            if matches!(page, Page::World | Page::Generation | Page::Gameplay) {
                let section = match self.world_page_index {
                    0 => "WORLD SOURCE",
                    1 => "ESSENTIALS",
                    2 | 3 => "WORLD GENERATION · TERRAIN, FOLDER, AND FIRST-CREATION OPTIONS",
                    4 => "GAMEPLAY RULES · COMMANDS, GAMERULES, AND EDITION-SPECIFIC OPTIONS",
                    5 if self.editing.is_some() => {
                        "EDIT RULES AS TEXT · COMMA-SEPARATED RULE=VALUE PAIRS"
                    }
                    5 => "EDIT RULES AS TEXT",
                    _ => "DATAPACKS",
                };
                paint(out, x + 2, y + 9, w - 4, section, Color::DarkGrey)?;
            } else if page == Page::Addons {
                paint(out, x + 2, y + 8, w - 4, "LOCAL FILES", Color::DarkGrey)?;
            }
            for (index, field) in self.fields().iter().enumerate() {
                if row >= y + h - 5 {
                    break;
                }
                if (page == Page::Configure && field.key == "software" && !self.java())
                    || (page == Page::World
                        && field.key == "backup"
                        && self.get("world_source") != "Backup .zip")
                    || (page == Page::Generation && self.get("world_source") == "Backup .zip")
                {
                    continue;
                }
                let focused = self.cursor == index;
                let value = if matches!(field.key, "version" | "bedrock_version") {
                    self.version_label_for(field.key)
                } else if field.key == "import_world" && self.scan.is_some() {
                    self.scan
                        .as_ref()
                        .and_then(|scan| {
                            scan.worlds
                                .iter()
                                .find(|world| world.name == self.get("import_world"))
                        })
                        .map(|world| format!("{} · {}", world.name, world.dimensions_label))
                        .unwrap_or_else(|| self.get(field.key).to_string())
                } else if field.key == "java" && self.editing.is_none() {
                    self.java_label()
                } else if field.key == "curseforge_key" {
                    "•".repeat(self.get(field.key).chars().count())
                } else if focused {
                    self.editing
                        .as_deref()
                        .unwrap_or(self.get(field.key))
                        .to_string()
                } else {
                    self.get(field.key).to_string()
                };
                paint(
                    out,
                    x + 3,
                    row,
                    2,
                    if focused { ">" } else { " " },
                    if focused {
                        Color::Green
                    } else {
                        Color::DarkGrey
                    },
                )?;
                paint(
                    out,
                    x + 5,
                    row,
                    27,
                    field.label,
                    if focused {
                        Color::White
                    } else {
                        Color::DarkGrey
                    },
                )?;
                paint(
                    out,
                    x + 33,
                    row,
                    w - 38,
                    &format!(
                        "{}{}",
                        value,
                        if focused && self.editing.is_some() {
                            "_"
                        } else {
                            ""
                        }
                    ),
                    if focused { Color::Green } else { Color::White },
                )?;
                row += 1;
            }
            if page == Page::Gameplay && self.world_page_index == 4 {
                paint(
                    out,
                    x + 3,
                    row + 1,
                    w - 6,
                    "Add Rule [a]    Edit as Text [e]",
                    Color::White,
                )?;
            }
            if page == Page::Addons {
                paint(
                    out,
                    x + 3,
                    row + 1,
                    w - 6,
                    "Add your own .jar",
                    Color::White,
                )?;
                paint(
                    out,
                    x + 3,
                    row + 2,
                    w - 6,
                    "Enter the local path with e. File chooser and multi-file staging are not connected.",
                    Color::DarkGrey,
                )?;
            } else if matches!(page, Page::World | Page::Generation | Page::Gameplay)
                && self.world_page_index == 6
            {
                paint(
                    out,
                    x + 3,
                    row + 1,
                    w - 6,
                    if self.java() {
                        "Import local datapack ZIP"
                    } else {
                        "Import local behavior pack"
                    },
                    Color::White,
                )?;
                paint(
                    out,
                    x + 3,
                    row + 2,
                    w - 6,
                    "Enter the local path with e. File chooser is not connected.",
                    Color::DarkGrey,
                )?;
            }
        }
        let configure_source_page =
            page == Page::Configure && self.java() && self.configure_page_index == 4;
        let status_row = if configure_source_page {
            y + h - 3
        } else {
            y + h - 4
        };
        if !self.message.is_empty() {
            paint(out, x + 2, status_row, w - 4, &self.message, Color::Yellow)?;
        } else if page != Page::Confirm
            && page != Page::Network
            && !configure_source_page
            && !self.is_java_runtime_page()
            && let Some(field) = self.fields().get(self.cursor)
        {
            paint(out, x + 2, status_row, w - 4, field.hint, Color::DarkGrey)?;
        }
        let footer = if self.pending_operation_id.is_some() {
            "Enter Check submitted operation   Please wait"
        } else if self.editing.is_some() && page == Page::Gameplay && self.world_page_index == 5 {
            "Type rule=value pairs   Enter Save   Esc Cancel"
        } else if self.editing.is_some() {
            "Type value   Backspace Delete   Enter Save   Esc Cancel"
        } else if page == Page::Confirm {
            "Enter Create server   Esc Back"
        } else if page == Page::Configure
            && self.java()
            && self.configure_page_index == 4
            && self.get("source_mode") == "Choose version…"
            && self.version_picker_active
        {
            "←→ Choose version   Enter Continue   Esc Cancel version choice"
        } else if page == Page::Configure
            && self.java()
            && self.configure_page_index == 4
            && self.get("source_mode") == "Choose version…"
        {
            "←→ Choose source   Enter Choose version   Esc Back"
        } else if page == Page::Configure && self.is_java_runtime_page() && self.java_install_page {
            "↑↓ Choose Java version   Enter Install   Esc Back to runtimes"
        } else if page == Page::Configure && self.is_java_runtime_page() {
            "↑↓ Choose runtime   i Install Java…   Enter Use selected Java   Esc Back"
        } else if page == Page::Network && self.network_page_index == 0 {
            "←→ Choose connection   Enter Continue   Esc Back"
        } else if page == Page::Network {
            "↑↓ Move   e Edit field   Enter Continue   Esc Back"
        } else if page == Page::World && self.world_page_index == 0 {
            "←→ Choose source   Enter Continue   Esc Back"
        } else if page == Page::Gameplay && self.world_page_index == 4 {
            "↑↓ Move   Space Toggle   a Add Rule   e Edit as Text   Enter Continue   Esc Back"
        } else if page == Page::Gameplay && self.world_page_index == 5 {
            if self.gamerules_text_saved {
                "↑↓ Choose   Enter Continue or Skip   e Edit selected rules   Esc Back"
            } else {
                "↑↓ Choose   Enter Edit or Skip   Esc Back"
            }
        } else if (page == Page::Gameplay && self.world_page_index == 6) || page == Page::Addons {
            "↑↓ Choose   e Edit selected path   Enter Continue or Skip   Esc Back"
        } else if matches!(page, Page::World | Page::Generation | Page::Gameplay) {
            "↑↓ Move   ←→ Choose   Space Toggle   e Edit   Enter Continue   Esc Back"
        } else if page == Page::Configure && self.java() && self.configure_page_index == 4 {
            "←→ Choose source   Enter Continue   Esc Back"
        } else if page == Page::Configure && self.configure_page_index == 1 {
            "e Edit name   Enter Continue   Esc Back"
        } else if page == Page::Configure {
            "↑↓ Move   ←→ Choose   Space Toggle   Enter Continue   Esc Back"
        } else if page == Page::Review
            && self
                .fields()
                .get(self.cursor)
                .is_some_and(|field| field.key == "import_world")
        {
            "↑↓ Move   ←→ Choose world   Space Toggle   Enter Continue   Esc Back"
        } else if matches!(page, Page::Choose | Page::ChooseSource) {
            "←→ Choose card   Enter Continue   Esc Back"
        } else {
            "↑↓ Move   ←→ Choice   Space Toggle   e Edit   Enter Continue   Esc Back"
        };
        paint(out, x + 2, y + h - 2, w - 4, footer, Color::DarkGrey)
    }

    fn draw_configure_page(
        &self,
        out: &mut Stdout,
        x: u16,
        y: u16,
        width: u16,
        _height: u16,
    ) -> io::Result<()> {
        let fields = self.fields();
        let field = fields.first();
        paint(
            out,
            x + 2,
            y + 5,
            width - 4,
            &if self.is_java_runtime_page() {
                format!(
                    "{} · step {} of {}",
                    if self.java_install_page {
                        "Install Java"
                    } else {
                        "Java selection"
                    },
                    self.step_number(),
                    self.step_labels().len()
                )
            } else {
                format!(
                    "Page {} / {} · ↑↓ move between pages",
                    self.configure_page_index + 1,
                    self.configure_page_count()
                )
            },
            Color::DarkGrey,
        )?;
        paint(
            out,
            x + 2,
            y + 6,
            width - 4,
            if self.is_java_runtime_page() && self.java_install_page {
                "Install Java"
            } else if self.is_java_runtime_page() {
                "Choose Java for this server"
            } else {
                "Configure your server"
            },
            Color::White,
        )?;
        paint(
            out,
            x + 2,
            y + 7,
            width - 4,
            if self.is_java_runtime_page() && self.java_install_page {
                "Pick the Java version that matches this Minecraft release. MSC installs it for this host."
            } else if self.is_java_runtime_page() {
                "MSC checks this host for a compatible runtime. This choice applies to this server only."
            } else {
                "Choose its edition and software. Network and world settings come next."
            },
            Color::DarkGrey,
        )?;
        if let Some(field) = field {
            let field_label = if self.is_java_runtime_page() && self.java_install_page {
                "JAVA VERSION TO INSTALL".to_string()
            } else if self.is_java_runtime_page() {
                "COMPATIBLE RUNTIMES".to_string()
            } else {
                field.label.to_ascii_uppercase()
            };
            paint(out, x + 2, y + 9, width - 4, &field_label, Color::DarkGrey)?;
        }
        let card_width = (width.saturating_sub(12)) / 2;
        match self.configure_page_index {
            0 => {
                draw_choice_card(
                    out,
                    x + 2,
                    y + 10,
                    card_width,
                    6,
                    ChoiceCard {
                        title: "Java",
                        description: "PC · Cross-play optional",
                    },
                    self.java(),
                )?;
                draw_choice_card(
                    out,
                    x + 8 + card_width,
                    y + 10,
                    card_width,
                    6,
                    ChoiceCard {
                        title: "Bedrock",
                        description: "PC · Console · Mobile",
                    },
                    !self.java(),
                )?;
            }
            1 => {
                box_outline(out, x + 2, y + 10, width - 4, 4)?;
                let value = if self.editing.is_some() {
                    self.editing.as_deref().unwrap_or_default()
                } else {
                    self.get("name")
                };
                paint(
                    out,
                    x + 4,
                    y + 12,
                    width - 8,
                    if value.is_empty() {
                        "Enter server name"
                    } else {
                        value
                    },
                    if value.is_empty() {
                        Color::DarkGrey
                    } else {
                        Color::White
                    },
                )?;
            }
            2 if !self.java() => {
                for (index, field) in fields.iter().enumerate() {
                    let row = y + 11 + index as u16 * 3;
                    paint(
                        out,
                        x + 3,
                        row,
                        26,
                        field.label,
                        if self.cursor == index {
                            Color::White
                        } else {
                            Color::DarkGrey
                        },
                    )?;
                    let value = if field.key == "bedrock_version" {
                        self.version_label_for(field.key)
                    } else {
                        self.get(field.key).to_string()
                    };
                    paint(
                        out,
                        x + 31,
                        row,
                        width - 34,
                        &value,
                        if self.cursor == index {
                            Color::Green
                        } else {
                            Color::White
                        },
                    )?;
                }
            }
            2 if self.java() => {
                draw_choice_card(
                    out,
                    x + 2,
                    y + 10,
                    card_width,
                    6,
                    ChoiceCard {
                        title: "Standard",
                        description: "Players join normally ·\nadd plugins",
                    },
                    self.get("java_category") == "Standard",
                )?;
                draw_choice_card(
                    out,
                    x + 8 + card_width,
                    y + 10,
                    card_width,
                    6,
                    ChoiceCard {
                        title: "Modded",
                        description: "Adds new content ·
players need the mods",
                    },
                    self.get("java_category") == "Modded",
                )?;
            }
            3 if self.java() => {
                let choices = if self.get("java_category") == "Modded" {
                    [
                        ("Fabric", "Lightweight mod loader"),
                        ("NeoForge", "For larger modpacks"),
                        ("Forge", "Broad mod support"),
                    ]
                } else {
                    [
                        ("Paper", "Recommended · performance and bug fixes"),
                        ("Purpur", "Paper plus gameplay configuration"),
                        ("Vanilla", "Mojang's unmodified server"),
                    ]
                };
                for (index, (name, detail)) in choices.iter().enumerate() {
                    let row = y + 10 + index as u16 * 2;
                    let selected = self.get("software") == *name;
                    paint(
                        out,
                        x + 3,
                        row,
                        2,
                        if selected { ">" } else { " " },
                        if selected {
                            Color::Green
                        } else {
                            Color::DarkGrey
                        },
                    )?;
                    paint(
                        out,
                        x + 5,
                        row,
                        20,
                        name,
                        if selected {
                            Color::White
                        } else {
                            Color::DarkGrey
                        },
                    )?;
                    paint(out, x + 27, row, width - 30, detail, Color::DarkGrey)?;
                }
            }
            4 if self.java() => {
                let selected = if self.get("source_mode") == "Download latest" {
                    0
                } else {
                    1
                };
                draw_choice_card(
                    out,
                    x + 2,
                    y + 10,
                    card_width,
                    5,
                    ChoiceCard {
                        title: "Download latest",
                        description: "Latest compatible release",
                    },
                    selected == 0,
                )?;
                draw_choice_card(
                    out,
                    x + 8 + card_width,
                    y + 10,
                    card_width,
                    5,
                    ChoiceCard {
                        title: "Choose version…",
                        description: if self.version_picker_active {
                            "←→ Choose a version · Enter Continue"
                        } else {
                            "Press Enter to choose a version"
                        },
                    },
                    selected == 1,
                )?;
                if self.get("source_mode") == "Choose version…" && self.version_picker_active {
                    let value = self.version_label();
                    paint(
                        out,
                        x + 3,
                        y + 15,
                        width - 6,
                        "MINECRAFT VERSION",
                        Color::DarkGrey,
                    )?;
                    paint(out, x + 3, y + 16, width - 6, &value, Color::Green)?;
                } else if self.get("source_mode") == "Download latest" {
                    paint(
                        out,
                        x + 3,
                        y + 16,
                        width - 6,
                        "MSC will use the latest compatible release for this software.",
                        Color::DarkGrey,
                    )?;
                }
            }
            5 if self.java() => {
                let state = if self.crossplay_available() {
                    if self.yes("crossplay") {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                } else {
                    "Unavailable"
                };
                paint(
                    out,
                    x + 3,
                    y + 11,
                    width - 6,
                    &format!("Enable Bedrock Cross-play      [{state}]"),
                    if self.crossplay_available() {
                        Color::White
                    } else {
                        Color::DarkGrey
                    },
                )?;
                if let Some(field) = field {
                    paint(out, x + 3, y + 13, width - 6, field.hint, Color::DarkGrey)?;
                }
            }
            index if self.is_java_runtime_page() && index == self.configure_page_index => {
                self.draw_java_runtime_page(out, x, y, width, _height)?;
            }
            _ => {
                let state = if self.yes("broadcast") {
                    "Enabled"
                } else {
                    "Disabled"
                };
                paint(
                    out,
                    x + 3,
                    y + 11,
                    width - 6,
                    &format!("Enable Xbox Broadcast      [{state}]"),
                    Color::White,
                )?;
                if let Some(field) = field {
                    paint(out, x + 3, y + 13, width - 6, field.hint, Color::DarkGrey)?;
                }
            }
        }
        Ok(())
    }

    fn draw_network_page(
        &self,
        out: &mut Stdout,
        x: u16,
        y: u16,
        width: u16,
        _height: u16,
    ) -> io::Result<()> {
        paint(
            out,
            x + 2,
            y + 5,
            width - 4,
            &format!("Page {} / 2", self.network_page_index + 1),
            Color::DarkGrey,
        )?;
        paint(
            out,
            x + 2,
            y + 6,
            width - 4,
            "How will friends connect?",
            Color::White,
        )?;
        paint(
            out,
            x + 2,
            y + 7,
            width - 4,
            "Choose how players outside your local network will join the server.",
            Color::DarkGrey,
        )?;

        if self.network_page_index == 0 {
            paint(
                out,
                x + 2,
                y + 9,
                width - 4,
                "CONNECTION METHOD",
                Color::DarkGrey,
            )?;
            let card_width = (width.saturating_sub(12)) / 2;
            draw_choice_card(
                out,
                x + 2,
                y + 10,
                card_width,
                6,
                ChoiceCard {
                    title: "Port Forwarding",
                    description: "Open a port on your router.\nDirect connection, no relay.",
                },
                self.get("connection") == "Port forwarding",
            )?;
            draw_choice_card(
                out,
                x + 8 + card_width,
                y + 10,
                card_width,
                6,
                ChoiceCard {
                    title: "Tunnel (playit.gg)",
                    description: "No router access needed. Free relay;\nadds about 10–50 ms.",
                },
                self.get("connection") == "Tunnel (playit.gg)",
            )?;
            return Ok(());
        }

        paint(out, x + 2, y + 9, width - 4, "PORTS", Color::DarkGrey)?;
        let tunnel = self.get("connection") == "Tunnel (playit.gg)";
        let mut rows: Vec<(&str, &str)> = if self.java() {
            let mut rows = vec![(
                if tunnel {
                    "Local Port (TCP)"
                } else {
                    "Java Port (TCP)"
                },
                "port",
            )];
            if self.yes("crossplay") {
                rows.push((
                    if tunnel {
                        "Bedrock / Geyser Local Port (UDP)"
                    } else {
                        "Bedrock / Geyser Port (UDP)"
                    },
                    "bedrock_port",
                ));
            }
            rows
        } else {
            vec![(
                if tunnel {
                    "Local Port (UDP)"
                } else {
                    "Bedrock Port (UDP)"
                },
                "port",
            )]
        };

        if self.path == PathKind::Modpack {
            rows.insert(0, ("Server Name", "name"));
        }
        for (index, (label, key)) in rows.iter().enumerate() {
            let row = y + 11 + index as u16 * 3;
            let focused = self.cursor == index;
            paint(
                out,
                x + 3,
                row,
                38,
                label,
                if focused {
                    Color::White
                } else {
                    Color::DarkGrey
                },
            )?;
            let value = if focused {
                self.editing.as_deref().unwrap_or(self.get(key))
            } else {
                self.get(key)
            };
            box_outline(out, x + 42, row.saturating_sub(1), width - 46, 3)?;
            paint(
                out,
                x + 44,
                row,
                width - 50,
                &format!(
                    "{value}{}",
                    if focused && self.editing.is_some() {
                        "_"
                    } else {
                        ""
                    }
                ),
                if focused { Color::Green } else { Color::White },
            )?;
        }

        let hint = if tunnel {
            "The tunnel assigns a public address when you first start the server. These are local listening ports."
        } else {
            "Forward these ports on your router so players outside your network can connect."
        };
        let hint_row = y + 11 + rows.len() as u16 * 3;
        if hint_row < y + _height.saturating_sub(2) {
            paint(out, x + 2, hint_row, width - 4, hint, Color::DarkGrey)?;
        }
        Ok(())
    }

    fn draw_java_runtime_page(
        &self,
        out: &mut Stdout,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        let required = self.required_java_major();
        if self.java_install_page {
            const INSTALL_OPTIONS: [(i64, &str); 4] = [
                (25, "Minecraft 26.1 and newer"),
                (21, "Minecraft 1.20.5–1.21.x"),
                (17, "Minecraft 1.17–1.20.4"),
                (8, "Minecraft 1.16.5 and older"),
            ];
            paint(
                out,
                x + 3,
                y + 10,
                width - 6,
                &format!("Choose the Java version to install. Required: Java {required} or newer."),
                Color::White,
            )?;
            for (index, (major, minecraft_range)) in INSTALL_OPTIONS.iter().enumerate() {
                let selected = *major == self.java_install_major;
                let row = y + 12 + index as u16;
                paint(
                    out,
                    x + 3,
                    row,
                    2,
                    if selected { ">" } else { " " },
                    if selected {
                        Color::Green
                    } else {
                        Color::DarkGrey
                    },
                )?;
                paint(
                    out,
                    x + 5,
                    row,
                    width - 8,
                    &format!(
                        "Java {major} · {minecraft_range}{}",
                        if *major == required {
                            " · Required for this server"
                        } else {
                            ""
                        }
                    ),
                    if selected {
                        Color::White
                    } else {
                        Color::DarkGrey
                    },
                )?;
            }
            return Ok(());
        }

        paint(
            out,
            x + 3,
            y + 10,
            width - 6,
            &format!(
                "Minecraft {} uses Java {required}. Choose a runtime MSC should use for this server.",
                self.version_label()
            ),
            Color::White,
        )?;

        let visible_rows = (height.saturating_sub(16) as usize).clamp(1, 7);
        if self.runtimes.is_empty() {
            paint(
                out,
                x + 3,
                y + 12,
                width - 6,
                &format!("No Java runtimes detected. Press i to install Java {required}."),
                Color::Yellow,
            )?;
            return Ok(());
        }

        let selected_index = self
            .runtimes
            .iter()
            .position(|runtime| runtime.executable_path == self.get("java"))
            .unwrap_or(0);
        let start = selected_index
            .saturating_add(1)
            .saturating_sub(visible_rows);
        let label_width = width / 3;
        let status_width = 22;
        let path_width = width.saturating_sub(label_width + status_width + 13);
        for (index, runtime) in self
            .runtimes
            .iter()
            .skip(start)
            .take(visible_rows)
            .enumerate()
        {
            let row = y + 12 + index as u16;
            let version = runtime
                .major_version
                .map(|major| format!("Java {major}"))
                .unwrap_or_else(|| "Java version unknown".into());
            let can_run = runtime.major_version.is_some_and(|major| major >= required);
            let selected = can_run && runtime.executable_path == self.get("java");
            paint(
                out,
                x + 3,
                row,
                2,
                if selected { ">" } else { " " },
                if selected {
                    Color::Green
                } else {
                    Color::DarkGrey
                },
            )?;
            paint(
                out,
                x + 5,
                row,
                label_width,
                &format!("{version} · {}", runtime.name),
                if can_run {
                    Color::White
                } else {
                    Color::DarkGrey
                },
            )?;
            paint(
                out,
                x + 5 + label_width + 2,
                row,
                path_width,
                &runtime.executable_path,
                Color::DarkGrey,
            )?;
            paint(
                out,
                x + width.saturating_sub(status_width + 2),
                row,
                status_width,
                if can_run {
                    if selected { "Selected" } else { "Select" }
                } else {
                    "Needs newer Java"
                },
                if can_run {
                    Color::DarkGrey
                } else {
                    Color::Yellow
                },
            )?;
        }
        if self.java_options().is_empty() {
            paint(
                out,
                x + 3,
                y + height.saturating_sub(4),
                width - 6,
                &format!("No compatible runtime detected. Press i to install Java {required}."),
                Color::Yellow,
            )?;
        }
        Ok(())
    }

    fn progress(&self, out: &mut Stdout, line: &str) -> Result<(), CliError> {
        let (w, h) = crossterm::terminal::size().map_err(|e| CliError::internal(e.to_string()))?;
        paint(
            out,
            w.saturating_sub(80) / 2,
            h / 2,
            w.min(78),
            line,
            Color::Green,
        )
        .map_err(|e| CliError::internal(e.to_string()))?;
        out.flush().map_err(|e| CliError::internal(e.to_string()))
    }

    async fn wait_operation(
        &self,
        out: &mut Stdout,
        client: &SharedClient,
        id: Option<String>,
    ) -> Result<(), CliError> {
        let Some(id) = id else {
            return Ok(());
        };
        loop {
            let operation: OperationDto = client.get_json(&format!("/v1/operations/{id}")).await?;
            let line = operation.status_line.as_deref().unwrap_or("Working…");
            self.progress(out, line)?;
            match operation.state {
                OperationStateDto::Succeeded => return Ok(()),
                OperationStateDto::Failed | OperationStateDto::Cancelled => {
                    return Err(CliError::usage(format!(
                        "Operation {}: {}",
                        id,
                        operation
                            .error
                            .map_or_else(|| "failed".into(), |e| e.message)
                    )));
                }
                _ => tokio::time::sleep(Duration::from_millis(400)).await,
            }
        }
    }

    async fn wait_creation_operation(
        &mut self,
        out: &mut Stdout,
        client: &SharedClient,
        id: String,
    ) -> Result<(), CliError> {
        loop {
            let operation: OperationDto = client.get_json(&format!("/v1/operations/{id}")).await?;
            let line = operation.status_line.as_deref().unwrap_or("Working…");
            self.progress(out, line)?;
            match operation.state {
                OperationStateDto::Succeeded => {
                    self.pending_operation_id = None;
                    return Ok(());
                }
                OperationStateDto::Failed | OperationStateDto::Cancelled => {
                    self.pending_operation_id = None;
                    return Err(CliError::usage(format!(
                        "Operation {}: {}",
                        id,
                        operation
                            .error
                            .map_or_else(|| "failed".into(), |error| error.message)
                    )));
                }
                _ => tokio::time::sleep(Duration::from_millis(400)).await,
            }
        }
    }

    async fn install_java(&mut self, out: &mut Stdout, major: i64) -> Result<String, CliError> {
        let client = SharedClient::connect_local().await?;
        let result: JavaRuntimeInstallResultDto = client
            .post_json("/v1/java-runtimes/install", &json!({"major":major}))
            .await?;
        self.wait_operation(out, &client, Some(result.operation_id))
            .await?;
        let detected: JavaRuntimesResponseDto = client.get_json("/v1/java-runtimes").await?;
        self.runtimes = detected.runtimes;
        self.runtimes
            .iter()
            .find(|runtime| runtime.major_version == Some(major))
            .map(|runtime| runtime.executable_path.clone())
            .ok_or_else(|| {
                CliError::usage("Java installation finished, but the new runtime was not detected.")
            })
    }

    async fn submit(&mut self, out: &mut Stdout) -> Result<String, CliError> {
        let client = SharedClient::connect_local().await?;
        self.progress(
            out,
            if self.pending_operation_id.is_some() {
                "Checking submitted server operation…"
            } else {
                "Sending server request…"
            },
        )?;
        if self.path == PathKind::Import {
            if self.pending_operation_id.is_none() {
                let source = self.get("source");
                let kind = if Path::new(source).is_dir() {
                    "folder"
                } else {
                    "zip"
                };
                let result: ServerImportResultDto = client.post_json("/v1/servers/import",&json!({
                    "action":"importExisting", "sourcePath":source, "importKind":kind,
                    "displayName":self.get("name"), "serverType":self.scan.as_ref().and_then(|s|s.server_type.as_deref()),
                    "activeWorldName":if self.get("import_world").is_empty() { None } else { Some(self.get("import_world")) },
                    "port":self.get("port").parse::<u16>().ok(), "maxPlayers":self.get("max_players").parse::<u32>().ok(), "acceptEula":self.yes("accept_eula")
                })).await?;
                if !result.success {
                    return Err(CliError::usage(result.message));
                }
                self.pending_operation_id = result.operation_id;
                if self.pending_operation_id.is_none() {
                    return Ok(format!("Imported server: {}", self.get("name")));
                }
            }
            let operation_id = self.pending_operation_id.clone().ok_or_else(|| {
                CliError::internal("import operation ID disappeared before polling")
            })?;
            self.wait_creation_operation(out, &client, operation_id)
                .await?;
            return Ok(format!("Imported server: {}", self.get("name")));
        }
        let java = self.java();
        let mut world = ServerCreateWorldSettingsDto {
            identity: WorldIdentityDto {
                name: Some(if self.get("world_name").is_empty() {
                    self.get("name").into()
                } else {
                    self.get("world_name").into()
                }),
                level_name: self
                    .world_field_available("identity.level-name", true)
                    .then(|| nonempty(self.get("level_name")))
                    .flatten(),
                seed: self
                    .world_field_available("identity.seed", true)
                    .then(|| nonempty(self.get("seed")))
                    .flatten(),
            },
            generation: WorldGenerationDto {
                world_type: self
                    .world_field_available("generation.world-type", true)
                    .then(|| match self.get("world_type") {
                        "Large biomes" => "large_biomes".into(),
                        "Single biome" => "single_biome_surface".into(),
                        other => other.to_lowercase(),
                    }),
                flat_preset: self
                    .world_field_available("generation.flat-preset", self.java())
                    .then(|| nonempty(self.get("flat_preset")))
                    .flatten(),
                structures: self
                    .world_field_available("generation.structures", self.java())
                    .then(|| self.yes("structures")),
                biome_source: self
                    .world_field_available("generation.biome-source", self.java())
                    .then(|| nonempty(self.get("biome_source")))
                    .flatten(),
                generator_options: self
                    .world_field_available("generation.generator-options", self.java())
                    .then(|| nonempty(self.get("generator_options")))
                    .flatten(),
                bonus_chest: self
                    .world_field_available("generation.bonus-chest", true)
                    .then(|| self.yes("bonus_chest")),
                data_packs: if self.world_field_available("generation.data-packs", self.java()) {
                    self.get("datapacks")
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                } else {
                    Vec::new()
                },
            },
            gameplay: WorldGameplayDto {
                difficulty: self
                    .world_field_available("gameplay.difficulty", true)
                    .then(|| self.get("difficulty").to_lowercase()),
                default_game_mode: self
                    .world_field_available("gameplay.default-game-mode", true)
                    .then(|| self.get("gamemode").to_lowercase()),
                hardcore: self
                    .world_field_available("gameplay.hardcore", self.java())
                    .then(|| self.yes("hardcore")),
                commands: self
                    .world_field_available("gameplay.commands", self.java())
                    .then(|| self.yes("commands")),
                cheats: self
                    .world_field_available("gameplay.cheats", !self.java())
                    .then(|| self.yes("cheats")),
                coordinates: self
                    .world_field_available("gameplay.coordinates", !self.java())
                    .then(|| self.yes("coordinates")),
                starting_map: self
                    .world_field_available("gameplay.starting-map", !self.java())
                    .then(|| self.yes("starting_map")),
                ..Default::default()
            },
        };
        if self.world_field_available("gameplay.gamerules", true) {
            for pair in self
                .get("gamerules")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let (key, value) = pair.split_once('=').ok_or_else(|| {
                    CliError::usage("Game rules must be comma-separated name=value pairs.")
                })?;
                world
                    .gameplay
                    .gamerules
                    .insert(key.trim().into(), value.trim().into());
            }
        }
        if self.world_field_available("gameplay.experiments", !self.java()) {
            for pair in self
                .get("experiments")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let (key, value) = pair.split_once('=').ok_or_else(|| {
                    CliError::usage(
                        "Experiments must be comma-separated name=true or name=false pairs.",
                    )
                })?;
                let enabled = value
                    .trim()
                    .parse::<bool>()
                    .map_err(|_| CliError::usage("Experiment values must be true or false."))?;
                world
                    .gameplay
                    .experiments
                    .insert(key.trim().into(), enabled);
            }
        }
        let request = ServerCreateRequestDto {
            name: self.get("name").into(),
            server_type: Some(if java { "java" } else { "bedrock" }.into()),
            java_flavor: if java {
                Some(self.get("software").to_lowercase())
            } else {
                None
            },
            port: self.get("port").parse().ok(),
            max_players: if java {
                None
            } else {
                self.get("max_players").parse().ok()
            },
            enable_cross_play: Some(self.yes("crossplay")),
            cross_play_bedrock_port: self.get("bedrock_port").parse().ok(),
            enable_playit: Some(self.get("connection") == "Tunnel (playit.gg)"),
            enable_xbox_broadcast: Some(self.yes("broadcast")),
            enable_voice_chat: Some(java && self.yes("voice_chat")),
            difficulty: Some(self.get("difficulty").to_lowercase()),
            gamemode: Some(self.get("gamemode").to_lowercase()),
            world_name: Some(if self.get("world_name").is_empty() {
                self.get("name").into()
            } else {
                self.get("world_name").into()
            }),
            world_seed: nonempty(self.get("seed")),
            version_id: if self.get("version") == "LATEST" {
                None
            } else {
                nonempty(self.get("version"))
            },
            minecraft_version: if self.path == PathKind::Modpack {
                self.inspection
                    .as_ref()
                    .and_then(|i| i.minecraft_version.clone())
            } else {
                None
            },
            loader_version: if self.path == PathKind::Modpack {
                self.inspection
                    .as_ref()
                    .and_then(|i| i.loader_version.clone())
            } else {
                None
            },
            java_path: if self.get("java") == "Automatic" {
                None
            } else {
                nonempty(self.get("java"))
            },
            staged_modpack_upload_id: self.staged_modpack.clone(),
            accept_eula: None,
            bedrock_version: if java {
                None
            } else {
                Some(self.get("bedrock_version").into())
            },
            world_settings: Some(world),
            ..Default::default()
        };
        let mut body =
            serde_json::to_value(request).map_err(|e| CliError::internal(e.to_string()))?;
        if let Some(token) = &self.confirmation {
            body["confirmation"] = json!(token);
        }
        if self.pending_operation_id.is_none() {
            let result: ServerCreateResultDto =
                client.post_json("/v1/servers/create", &body).await?;
            if !result.success {
                return Err(CliError::usage(result.message));
            }
            self.pending_operation_id = result.operation_id;
            if self.pending_operation_id.is_none() {
                return Ok(format!("Created server: {}", self.get("name")));
            }
        }
        let operation_id = self
            .pending_operation_id
            .clone()
            .ok_or_else(|| CliError::internal("create operation ID disappeared before polling"))?;
        self.wait_creation_operation(out, &client, operation_id)
            .await?;
        let needs_follow_up = self.get("world_source") == "Backup .zip"
            || !self.get("world_pack").is_empty()
            || !self.get("addon").is_empty();
        let created_server_id = if needs_follow_up {
            client
                .get_json::<RemoteApiStatus>("/v1/status")
                .await
                .ok()
                .and_then(|status| status.active_server_id)
        } else {
            None
        };
        // The server now exists. A later world or add-on failure must not invite
        // another press of Enter, which would create a duplicate server.
        let additions: Result<(), CliError> = async {
        if self.get("world_source") == "Backup .zip" {
            let server_id = created_server_id.as_deref().ok_or_else(|| {
                CliError::usage("Server created, but no server ID was returned for world import.")
            })?;
            let _: serde_json::Value = client
                .post_json("/v1/active-server", &json!({"serverId":server_id}))
                .await?;
            self.progress(out, "Uploading first world backup…")?;
            let id = stage_file_upload(
                &client,
                Path::new(self.get("backup")),
                StagedUploadPurposeDto::WorldImport,
                None,
                None,
                Some(server_id.into()),
            )
            .await?;
            let response: serde_json::Value = client
                .post_json(
                    "/v1/worlds/import",
                    &json!({"name":Path::new(self.get("backup")).file_stem().and_then(|s|s.to_str()).unwrap_or("Imported World"),"stagedUploadId":id}),
                )
                .await?;
            self.wait_operation(
                out,
                &client,
                response["operationId"].as_str().map(str::to_string),
            )
            .await?;
            let active = response["updated"]["activeSlotId"].as_str();
            let slot = response["updated"]["slots"].as_array()
                .and_then(|slots| slots.iter().filter_map(|slot| slot["id"].as_str()).find(|id| Some(*id) != active))
                .ok_or_else(|| CliError::usage("Imported world slot was not returned by the agent."))?;
            let activated: serde_json::Value = client
                .post_json("/v1/worlds/activate", &json!({"slotId":slot})).await?;
            self.wait_operation(out, &client, activated["operationId"].as_str().map(str::to_string)).await?;
        }
        if !self.get("world_pack").is_empty() {
            let server_id = created_server_id.as_deref().ok_or_else(|| CliError::usage("Server created, but no active server ID was returned for world pack install."))?;
            let _: serde_json::Value = client.post_json("/v1/active-server", &json!({"serverId":server_id})).await?;
            let worlds: serde_json::Value = client.get_json("/v1/worlds").await?;
            let slot = worlds["activeSlotId"].as_str().ok_or_else(|| CliError::usage("No active world slot was returned by the agent."))?;
            self.progress(out, "Uploading local world pack…")?;
            let id = stage_file_upload(&client, Path::new(self.get("world_pack")), StagedUploadPurposeDto::AddonLocalFile, None, None, Some(server_id.into())).await?;
            let kind = if java { "datapacks" } else { "behaviorpacks" };
            let response: serde_json::Value = client.post_json(&format!("/v1/worlds/{slot}/{kind}/install"), &json!({"stagedUploadId":id})).await?;
            self.wait_operation(out, &client, response["operationId"].as_str().map(str::to_string)).await?;
        }
        if !self.get("addon").is_empty() {
            let server_id = created_server_id.as_deref().ok_or_else(|| {
                CliError::usage("Server created, but no server ID was returned for add-on install.")
            })?;
            let _: serde_json::Value = client
                .post_json("/v1/active-server", &json!({"serverId":server_id}))
                .await?;
            self.progress(out, "Uploading local add-on…")?;
            let id = stage_file_upload(
                &client,
                Path::new(self.get("addon")),
                StagedUploadPurposeDto::AddonLocalFile,
                None,
                None,
                Some(server_id.into()),
            )
            .await?;
            let response: serde_json::Value = client
                .post_json("/v1/components/install", &json!({"stagedUploadId":id}))
                .await?;
            self.wait_operation(
                out,
                &client,
                response["operationId"].as_str().map(str::to_string),
            )
            .await?;
        }
        Ok(())
        }.await;
        Ok(match additions {
            Ok(()) => format!("Created server: {}", self.get("name")),
            Err(error) => format!(
                "Created server: {} · follow-up setup needs attention: {error}",
                self.get("name")
            ),
        })
    }
}
fn nonempty(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.trim().into())
    }
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}
