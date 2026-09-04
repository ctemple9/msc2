//! Server identity: which platform (Java/Bedrock) a server runs, and for
//! Java servers, which specific server software (flavor) it runs and the
//! category, provisioning, and add-on rules that follow from that choice.
//!
//! Ported from `AppConfig.swift` (`ServerType`) and `JavaServerFlavor.swift`
//! (`JavaServerCategory`, `ServerProvisioningKind`, `AddOnKind`,
//! `JavaServerFlavor`). Neither MSC 1 file has a test file of its own —
//! `fixtures/server-identity/` characterizes them fresh by reading the
//! source's closed, deterministic logic directly, per `rolling-plan.md`
//! P1.8's own instructions. `displayName`, `shortDescription`, and
//! `iconName` are client-rendering (port plan §1's deletion test) and are
//! not ported. `ServerType` itself has no computed property beyond the
//! excluded `displayName` — nothing to characterize beyond its two cases,
//! so it carries no fixture of its own.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    Java,
    Bedrock,
}

impl ServerType {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Bedrock => "bedrock",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "java" => Some(Self::Java),
            "bedrock" => Some(Self::Bedrock),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaServerCategory {
    Standard,
    Modded,
}

impl JavaServerCategory {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Modded => "modded",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "standard" => Some(Self::Standard),
            "modded" => Some(Self::Modded),
            _ => None,
        }
    }
}

/// How a flavor is provisioned. MSC 1's `ServerProvisioningKind` isn't
/// `: String` and has no `rawValue` of its own — `download_and_go` /
/// `install_step` are wire tokens invented for this port's fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerProvisioningKind {
    DownloadAndGo,
    InstallStep,
}

impl ServerProvisioningKind {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::DownloadAndGo => "download_and_go",
            Self::InstallStep => "install_step",
        }
    }
}

/// What kind of add-on a flavor accepts. MSC 1's `AddOnKind` isn't
/// `: String` either — `plugin` / `mod` wire tokens match its case names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOnKind {
    Plugin,
    Mod,
}

impl AddOnKind {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Mod => "mod",
        }
    }

    /// The add-on directory `createNewServer` creates inside a new
    /// server's folder (`AppViewModel+ServerCreation.swift:314`).
    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Plugin => "plugins",
            Self::Mod => "mods",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaServerFlavor {
    Paper,
    Purpur,
    Pufferfish,
    Vanilla,
    Fabric,
    NeoForge,
    Spigot,
    Forge,
    Quilt,
}

impl JavaServerFlavor {
    /// Declaration order from `JavaServerFlavor.swift`'s `CaseIterable`
    /// `allCases` — `create_flow_choices`'s stable sort depends on this
    /// order matching the source exactly.
    pub const ALL: [Self; 9] = [
        Self::Paper,
        Self::Purpur,
        Self::Pufferfish,
        Self::Vanilla,
        Self::Fabric,
        Self::NeoForge,
        Self::Spigot,
        Self::Forge,
        Self::Quilt,
    ];

    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::Purpur => "purpur",
            Self::Pufferfish => "pufferfish",
            Self::Vanilla => "vanilla",
            Self::Fabric => "fabric",
            Self::NeoForge => "neoforge",
            Self::Spigot => "spigot",
            Self::Forge => "forge",
            Self::Quilt => "quilt",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "paper" => Some(Self::Paper),
            "purpur" => Some(Self::Purpur),
            "pufferfish" => Some(Self::Pufferfish),
            "vanilla" => Some(Self::Vanilla),
            "fabric" => Some(Self::Fabric),
            "neoforge" => Some(Self::NeoForge),
            "spigot" => Some(Self::Spigot),
            "forge" => Some(Self::Forge),
            "quilt" => Some(Self::Quilt),
            _ => None,
        }
    }

    pub fn category(self) -> JavaServerCategory {
        match self {
            Self::Paper | Self::Purpur | Self::Pufferfish | Self::Vanilla | Self::Spigot => {
                JavaServerCategory::Standard
            }
            Self::Fabric | Self::NeoForge | Self::Forge | Self::Quilt => JavaServerCategory::Modded,
        }
    }

    /// Forge/NeoForge run Fabric mods through Sinytra Connector, which
    /// changes which Modrinth project a Fabric-side dependency maps to.
    /// Used by `slug::canonical_slug`'s `forge_family` flag.
    pub fn is_forge_family(self) -> bool {
        matches!(self, Self::Forge | Self::NeoForge)
    }

    /// Vanilla has no plugin/mod API (datapacks only), so it returns
    /// `None` — the add-on browser is hidden for it.
    pub fn add_on_kind(self) -> Option<AddOnKind> {
        match self {
            Self::Vanilla => None,
            _ => Some(if self.category() == JavaServerCategory::Standard {
                AddOnKind::Plugin
            } else {
                AddOnKind::Mod
            }),
        }
    }

    /// Spigot looks like a download to the user but actually needs a
    /// local BuildTools compile, so it's an install step.
    pub fn provisioning_kind(self) -> ServerProvisioningKind {
        match self {
            Self::NeoForge | Self::Forge | Self::Spigot => ServerProvisioningKind::InstallStep,
            _ => ServerProvisioningKind::DownloadAndGo,
        }
    }

    /// Modrinth `project_type` facet. Derived from `category` alone, not
    /// `add_on_kind` — Vanilla has no add-on catalog but still reports
    /// `"plugin"` here, matching MSC 1's source exactly (preserved as-is,
    /// not a bug this port introduces).
    pub fn modrinth_project_type(self) -> &'static str {
        if self.category() == JavaServerCategory::Standard {
            "plugin"
        } else {
            "mod"
        }
    }

    /// Modrinth `loaders` facet values used when searching add-ons for
    /// this flavor. Empty when the flavor has no add-on catalog (Vanilla).
    pub fn modrinth_loader_facets(self) -> &'static [&'static str] {
        match self {
            Self::Paper | Self::Purpur | Self::Pufferfish | Self::Spigot => {
                &["paper", "spigot", "bukkit"]
            }
            Self::Fabric => &["fabric"],
            Self::Quilt => &["quilt", "fabric"],
            Self::NeoForge => &["neoforge"],
            Self::Forge => &["forge"],
            Self::Vanilla => &[],
        }
    }

    /// Flavor-specific console command MSC can send to request a TPS
    /// sample. `None` for Vanilla/Fabric/Quilt, which have no stable
    /// built-in TPS command — MSC skips the poll rather than spamming
    /// "Unknown or incomplete command".
    pub fn auto_tps_command(self) -> Option<&'static str> {
        match self {
            Self::Paper | Self::Purpur | Self::Pufferfish | Self::Spigot => Some("tps"),
            Self::Forge => Some("forge tps"),
            Self::NeoForge => Some("neoforge tps"),
            Self::Vanilla | Self::Fabric | Self::Quilt => None,
        }
    }

    /// The console command MSC should poll for a live TPS sample, given
    /// the server's Minecraft version. Loader-native commands are
    /// version-independent and come straight from `auto_tps_command`.
    /// Vanilla/Fabric/Quilt have no loader command, but Minecraft 1.20.3+
    /// ships `/tick query`; older or unknown versions return `None`.
    pub fn tps_poll_command(self, minecraft_version: Option<&str>) -> Option<&'static str> {
        if let Some(native) = self.auto_tps_command() {
            return Some(native);
        }
        match self {
            Self::Vanilla | Self::Fabric | Self::Quilt => {
                if supports_vanilla_tick_query(minecraft_version) {
                    Some("tick query")
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Selects the automatic TPS command when the server has the spark mod.
    /// Native loader commands stay highest priority; spark fills the gap for
    /// older Fabric/Quilt/Vanilla servers before `/tick query` is available.
    pub fn tps_poll_command_with_spark(
        self,
        minecraft_version: Option<&str>,
        has_spark: bool,
    ) -> Option<&'static str> {
        if self.auto_tps_command().is_none() && has_spark {
            Some("spark tps")
        } else {
            self.tps_poll_command(minecraft_version)
        }
    }

    /// Highlighted as the recommended default within its category.
    pub fn is_recommended(self) -> bool {
        matches!(self, Self::Paper | Self::Fabric)
    }

    /// Whether this flavor is offered in the Create Server flow today.
    pub fn is_available_in_create_flow(self) -> bool {
        !matches!(self, Self::Spigot | Self::Quilt | Self::Pufferfish)
    }

    /// Flavors offered for a given category in the Create flow,
    /// recommended first. Relies on a *stable* sort (Swift's `sorted(by:)`
    /// is stable) so ties keep `ALL`'s declaration order.
    pub fn create_flow_choices(category: JavaServerCategory) -> Vec<Self> {
        let mut choices: Vec<Self> = Self::ALL
            .into_iter()
            .filter(|f| f.category() == category && f.is_available_in_create_flow())
            .collect();
        choices.sort_by_key(|f| if f.is_recommended() { 0 } else { 1 });
        choices
    }
}

/// Whether the running server exposes the vanilla `/tick query` command,
/// added in Minecraft 1.20.3. Uses a numeric dotted-integer compare so
/// multi-digit components order correctly (e.g. "1.20.10" > "1.20.3").
/// Unknown/empty versions are treated as unsupported to avoid console
/// spam. Swift trims `.whitespaces` (spaces/tabs) before checking
/// emptiness; Rust's `trim()` also strips other Unicode whitespace
/// (e.g. newlines) — a wider trim than Swift's, unexercised by any
/// fixture here since none uses embedded whitespace.
pub fn supports_vanilla_tick_query(minecraft_version: Option<&str>) -> bool {
    let Some(v) = minecraft_version.map(str::trim) else {
        return false;
    };
    if v.is_empty() {
        return false;
    }
    match (
        crate::version::parse_components(v),
        crate::version::parse_components("1.20.3"),
    ) {
        (Some(a), Some(b)) => {
            crate::version::compare_components(&a, &b) != std::cmp::Ordering::Less
        }
        _ => false,
    }
}
