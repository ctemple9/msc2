//! Port of `RouterPortForwardGuideComposer.swift` (306 lines) — "composes
//! fully ordered logical guide structures from seed data, merging
//! router-specific steps, prerequisites, value summaries, and notes into a
//! renderable section list" (the file's own doc comment).
//!
//! That doc comment's "merging" turns out to mean *assembling/concatenating*,
//! not overriding: reading the source, `composeSections` simply appends up
//! to seven sections in a fixed order, each included or omitted by a boolean
//! flag or an emptiness guard. There is no mechanism anywhere in this file
//! where a router-specific item replaces or takes precedence over a shared
//! one — `rolling-plan.md`'s own P1.12 entry describes "merge precedence
//! when a router-specific step overrides a shared one," but no such
//! override logic exists in the actual source. Ported as the source
//! actually behaves (plain conditional concatenation), flagged here rather
//! than silently reconciled with the plan's description.
//!
//! This is the first router-engine step to need the full guide shape — the
//! composed sections *are* a guide's rendered content (steps' titles/bodies,
//! notes, troubleshooting topics), not decoration around it, so none of it
//! is excludable as client-rendering the way `matcher.rs`/`fallback_tree.rs`
//! excluded prose. `Guide` here is accordingly richer than `matcher::Guide`
//! — it reuses `matcher::GuideCategory`/`matcher::AdminSurface` (identical
//! needs) but adds `admin_addresses`, `menu_path`, `alternate_menu_names`,
//! `steps`, `notes`, `troubleshooting`, `shared_sections`, and `review`,
//! while dropping `family`/`search_keywords`/`display_name` (matcher-only,
//! untouched by composition). `GuideStep`/`GuideNote`/`TroubleshootingTopic`/
//! `RouterGuideToken`/`RouterGuideStepKind`/`RouterGuideConfidence`/
//! `TroubleshootingTopicId` are new to this step — no earlier router module
//! needed step/note/topic content.
//!
//! `SectionOrigin` isn't `String`-backed in Swift (just `Equatable`, no
//! `rawValue` at all) — its wire tokens here (`shared`/`guide_specific`/
//! `mixed`) are invented for this port's fixtures, same as P1.8's
//! `ServerProvisioningKind`/`AddOnKind` for non-`String` Swift enums.
//! `SectionKind` *is* `String`-backed but declares no explicit raw values,
//! so its raw value is the case name verbatim (camelCase) — same pattern as
//! `fallback_tree::NodeId`/`NodeKind`.
//!
//! `composeGuide(id:)` (repository lookup + delegate) is ported as
//! `compose_guide_by_id`. `composeBestMatch(for:matcher:)` — which chains
//! the matcher's `bestMatch` into `composeGuide` — is not ported here: both
//! halves are already independently characterized (P1.10's matcher, this
//! step's `compose_guide`), and wiring them together tests only that they
//! compose, not new logic of this file's own.

use std::collections::HashSet;

pub use crate::router::matcher::{AdminSurface, GuideCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouterGuideToken {
    SelectedServerName,
    DetectedLocalIpAddress,
    DetectedGatewayIpAddress,
    JavaPort,
    BedrockPort,
    RecommendedProtocol,
    BedrockEnabled,
}

impl RouterGuideToken {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::SelectedServerName => "selected_server_name",
            Self::DetectedLocalIpAddress => "detected_local_ip_address",
            Self::DetectedGatewayIpAddress => "detected_gateway_ip_address",
            Self::JavaPort => "java_port",
            Self::BedrockPort => "bedrock_port",
            Self::RecommendedProtocol => "recommended_protocol",
            Self::BedrockEnabled => "bedrock_enabled",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "selected_server_name" => Self::SelectedServerName,
            "detected_local_ip_address" => Self::DetectedLocalIpAddress,
            "detected_gateway_ip_address" => Self::DetectedGatewayIpAddress,
            "java_port" => Self::JavaPort,
            "bedrock_port" => Self::BedrockPort,
            "recommended_protocol" => Self::RecommendedProtocol,
            "bedrock_enabled" => Self::BedrockEnabled,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterGuideStepKind {
    Intro,
    Prerequisite,
    Navigate,
    Input,
    Save,
    Test,
    Warning,
}

impl RouterGuideStepKind {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Prerequisite => "prerequisite",
            Self::Navigate => "navigate",
            Self::Input => "input",
            Self::Save => "save",
            Self::Test => "test",
            Self::Warning => "warning",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "intro" => Self::Intro,
            "prerequisite" => Self::Prerequisite,
            "navigate" => Self::Navigate,
            "input" => Self::Input,
            "save" => Self::Save,
            "test" => Self::Test,
            "warning" => Self::Warning,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterGuideConfidence {
    VerifiedRecently,
    CommonFlow,
    OlderInterfaceMayVary,
    CommunityBased,
}

impl RouterGuideConfidence {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::VerifiedRecently => "verified_recently",
            Self::CommonFlow => "common_flow",
            Self::OlderInterfaceMayVary => "older_interface_may_vary",
            Self::CommunityBased => "community_based",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "verified_recently" => Self::VerifiedRecently,
            "common_flow" => Self::CommonFlow,
            "older_interface_may_vary" => Self::OlderInterfaceMayVary,
            "community_based" => Self::CommunityBased,
            _ => return None,
        })
    }

    /// `RouterGuideConfidence.displayName` — baked directly into the intro
    /// section's "Confidence" paragraph, so unlike most `displayName`
    /// properties elsewhere in this port, this one is *not* excludable as
    /// client-rendering: it's part of `compose_guide`'s own computed output.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::VerifiedRecently => "Verified recently",
            Self::CommonFlow => "Common flow",
            Self::OlderInterfaceMayVary => "Older interface, may vary",
            Self::CommunityBased => "Community-based guidance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TroubleshootingTopicId {
    LocalIpChanged,
    DoubleNat,
    Cgnat,
    WrongRouter,
    WrongDevice,
    FirewallBlocked,
    WrongProtocol,
    RouterRebootRequired,
    NoAdminAccess,
}

impl TroubleshootingTopicId {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::LocalIpChanged => "local_ip_changed",
            Self::DoubleNat => "double_nat",
            Self::Cgnat => "cgnat",
            Self::WrongRouter => "wrong_router",
            Self::WrongDevice => "wrong_device",
            Self::FirewallBlocked => "firewall_blocked",
            Self::WrongProtocol => "wrong_protocol",
            Self::RouterRebootRequired => "router_reboot_required",
            Self::NoAdminAccess => "no_admin_access",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "local_ip_changed" => Self::LocalIpChanged,
            "double_nat" => Self::DoubleNat,
            "cgnat" => Self::Cgnat,
            "wrong_router" => Self::WrongRouter,
            "wrong_device" => Self::WrongDevice,
            "firewall_blocked" => Self::FirewallBlocked,
            "wrong_protocol" => Self::WrongProtocol,
            "router_reboot_required" => Self::RouterRebootRequired,
            "no_admin_access" => Self::NoAdminAccess,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GuideStep {
    pub id: String,
    pub kind: RouterGuideStepKind,
    pub title: String,
    pub body: String,
    pub referenced_tokens: Vec<RouterGuideToken>,
    pub alternate_terms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GuideNote {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct TroubleshootingTopic {
    pub id: TroubleshootingTopicId,
    pub title: String,
    pub summary: String,
    pub suggested_next_actions: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SharedSections {
    pub include_shared_intro: bool,
    pub include_shared_prerequisites: bool,
    pub include_shared_value_summary: bool,
    pub include_shared_troubleshooting_footer: bool,
}

/// Only `sourceConfidence` — `lastReviewed`/`reviewNotes` are read by
/// nothing `composeSections` touches, excluded per the deletion test.
#[derive(Debug, Clone)]
pub struct ReviewMetadata {
    pub source_confidence: RouterGuideConfidence,
}

/// See the module doc comment for how this differs from `matcher::Guide`.
#[derive(Debug, Clone)]
pub struct Guide {
    pub id: String,
    pub category: GuideCategory,
    pub admin_addresses: Vec<String>,
    pub admin_surface: AdminSurface,
    pub menu_path: Vec<String>,
    pub alternate_menu_names: Vec<String>,
    pub steps: Vec<GuideStep>,
    pub notes: Vec<GuideNote>,
    pub troubleshooting: Vec<TroubleshootingTopicId>,
    pub shared_sections: SharedSections,
    pub review: ReviewMetadata,
    pub provider_display_name: Option<String>,
    pub device_display_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Intro,
    Prerequisites,
    ValueSummary,
    MenuPath,
    RouterSpecificSteps,
    Notes,
    TroubleshootingFooter,
}

impl SectionKind {
    /// Implicit (camelCase) raw value — Swift declares no explicit
    /// `= "..."` for this enum. See `fallback_tree::NodeId::raw_value`.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Prerequisites => "prerequisites",
            Self::ValueSummary => "valueSummary",
            Self::MenuPath => "menuPath",
            Self::RouterSpecificSteps => "routerSpecificSteps",
            Self::Notes => "notes",
            Self::TroubleshootingFooter => "troubleshootingFooter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionOrigin {
    Shared,
    GuideSpecific,
    Mixed,
}

impl SectionOrigin {
    /// Invented wire token — see the module doc comment.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::GuideSpecific => "guide_specific",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SectionItem {
    Paragraph {
        title: Option<String>,
        body: String,
        referenced_tokens: Vec<RouterGuideToken>,
    },
    BulletList {
        title: Option<String>,
        bullets: Vec<String>,
        referenced_tokens: Vec<RouterGuideToken>,
    },
    MenuPath {
        title: Option<String>,
        path: Vec<String>,
        alternate_menu_names: Vec<String>,
    },
    Step(GuideStep),
    Note(GuideNote),
    TroubleshootingTopic(TroubleshootingTopic),
}

impl SectionItem {
    pub fn referenced_tokens(&self) -> Vec<RouterGuideToken> {
        match self {
            Self::Paragraph {
                referenced_tokens, ..
            } => referenced_tokens.clone(),
            Self::BulletList {
                referenced_tokens, ..
            } => referenced_tokens.clone(),
            Self::MenuPath { .. } => Vec::new(),
            Self::Step(step) => step.referenced_tokens.clone(),
            Self::Note(_) => Vec::new(),
            Self::TroubleshootingTopic(_) => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComposedSection {
    pub id: String,
    pub kind: SectionKind,
    pub title: &'static str,
    pub items: Vec<SectionItem>,
    pub origin: SectionOrigin,
}

impl ComposedSection {
    pub fn referenced_tokens(&self) -> Vec<RouterGuideToken> {
        let mut seen = HashSet::new();
        self.items
            .iter()
            .flat_map(|item| item.referenced_tokens())
            .filter(|token| seen.insert(*token))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ComposedGuide<'a> {
    pub guide: &'a Guide,
    pub sections: Vec<ComposedSection>,
}

impl<'a> ComposedGuide<'a> {
    pub fn id(&self) -> &str {
        &self.guide.id
    }
}

fn cleaned(value: Option<&str>) -> Option<&str> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn intro_body(guide: &Guide) -> String {
    let surface_description = match guide.admin_surface {
        AdminSurface::WebBrowser => "in your router's browser-based admin page",
        AdminSurface::MobileApp => "in the router's mobile app",
        AdminSurface::Either => "in either the router app or its browser-based admin page",
    };

    if let (Some(provider), Some(device)) = (
        cleaned(guide.provider_display_name.as_deref()),
        cleaned(guide.device_display_name.as_deref()),
    ) && provider.to_lowercase() != device.to_lowercase()
    {
        return format!(
            "This guide helps you create a port-forward rule {surface_description} for {device}. Keep the provider name ({provider}) separate from the actual device where settings are changed."
        );
    }

    if let Some(device) = cleaned(guide.device_display_name.as_deref()) {
        return format!(
            "This guide helps you create a port-forward rule {surface_description} for {device}."
        );
    }

    format!("This guide helps you create a port-forward rule {surface_description}.")
}

fn prerequisites_bullets(guide: &Guide) -> Vec<String> {
    let mut bullets = vec![
        "Confirm the host Mac is the device that will receive the forward.".to_string(),
        "Keep the router login open so you can return to the forwarding page if the firmware hides advanced settings.".to_string(),
        "Have the host Mac's local IP ready: {{detected_local_ip_address}}.".to_string(),
    ];

    if !guide.admin_addresses.is_empty() {
        bullets.push(format!(
            "Common admin addresses for this guide: {}.",
            guide.admin_addresses.join(", ")
        ));
    }

    match guide.category {
        GuideCategory::IspGateway => {
            bullets.push("Verify you are configuring the ISP gateway itself and not a separate downstream router or mesh system.".to_string());
        }
        GuideCategory::MeshSystem => {
            bullets.push("Check whether management happens in the router app first; some mesh systems hide forwarding outside the app.".to_string());
        }
        _ => {}
    }

    bullets
}

fn value_summary_bullets(guide: &Guide) -> Vec<String> {
    let mut bullets = vec![
        "Target device / internal IP: {{detected_local_ip_address}}".to_string(),
        "Java port: {{java_port}} (usually TCP)".to_string(),
        "Recommended protocol guidance: {{recommended_protocol}}".to_string(),
    ];

    let mentions_bedrock = guide.steps.iter().any(|s| {
        s.referenced_tokens.contains(&RouterGuideToken::BedrockPort)
            || s.referenced_tokens
                .contains(&RouterGuideToken::BedrockEnabled)
    });
    if mentions_bedrock {
        bullets.push(
            "Bedrock port: {{bedrock_port}} (usually UDP, when Bedrock/Geyser is enabled)"
                .to_string(),
        );
        bullets.push("Bedrock enabled: {{bedrock_enabled}}".to_string());
    }

    bullets
}

fn shared_intro_section(guide: &Guide) -> ComposedSection {
    ComposedSection {
        id: format!("{}.intro", guide.id),
        kind: SectionKind::Intro,
        title: "What you are doing",
        items: vec![
            SectionItem::Paragraph {
                title: None,
                body: intro_body(guide),
                referenced_tokens: Vec::new(),
            },
            SectionItem::Paragraph {
                title: Some("Confidence".to_string()),
                body: guide.review.source_confidence.display_name().to_string(),
                referenced_tokens: Vec::new(),
            },
        ],
        origin: SectionOrigin::Shared,
    }
}

fn shared_prerequisites_section(guide: &Guide) -> ComposedSection {
    ComposedSection {
        id: format!("{}.prerequisites", guide.id),
        kind: SectionKind::Prerequisites,
        title: "Before you start",
        items: vec![SectionItem::BulletList {
            title: None,
            bullets: prerequisites_bullets(guide),
            referenced_tokens: vec![
                RouterGuideToken::DetectedLocalIpAddress,
                RouterGuideToken::JavaPort,
                RouterGuideToken::BedrockPort,
                RouterGuideToken::BedrockEnabled,
            ],
        }],
        origin: SectionOrigin::Shared,
    }
}

fn shared_value_summary_section(guide: &Guide) -> ComposedSection {
    ComposedSection {
        id: format!("{}.value-summary", guide.id),
        kind: SectionKind::ValueSummary,
        title: "Values you will enter",
        items: vec![SectionItem::BulletList {
            title: None,
            bullets: value_summary_bullets(guide),
            referenced_tokens: vec![
                RouterGuideToken::DetectedLocalIpAddress,
                RouterGuideToken::JavaPort,
                RouterGuideToken::BedrockPort,
                RouterGuideToken::RecommendedProtocol,
                RouterGuideToken::BedrockEnabled,
            ],
        }],
        origin: SectionOrigin::Shared,
    }
}

fn menu_path_section(guide: &Guide) -> Option<ComposedSection> {
    let has_menu_path = !guide.menu_path.is_empty();
    let has_alternates = !guide.alternate_menu_names.is_empty();
    if !has_menu_path && !has_alternates {
        return None;
    }

    Some(ComposedSection {
        id: format!("{}.menu-path", guide.id),
        kind: SectionKind::MenuPath,
        title: "Where to look",
        items: vec![SectionItem::MenuPath {
            title: None,
            path: guide.menu_path.clone(),
            alternate_menu_names: guide.alternate_menu_names.clone(),
        }],
        origin: SectionOrigin::Mixed,
    })
}

fn router_specific_steps_section(guide: &Guide) -> ComposedSection {
    ComposedSection {
        id: format!("{}.steps", guide.id),
        kind: SectionKind::RouterSpecificSteps,
        title: "Steps",
        items: guide.steps.iter().cloned().map(SectionItem::Step).collect(),
        origin: SectionOrigin::GuideSpecific,
    }
}

fn notes_section(guide: &Guide) -> Option<ComposedSection> {
    if guide.notes.is_empty() {
        return None;
    }

    Some(ComposedSection {
        id: format!("{}.notes", guide.id),
        kind: SectionKind::Notes,
        title: "Notes and quirks",
        items: guide.notes.iter().cloned().map(SectionItem::Note).collect(),
        origin: SectionOrigin::GuideSpecific,
    })
}

fn troubleshooting_footer_section(
    guide: &Guide,
    topics: &[TroubleshootingTopic],
) -> Option<ComposedSection> {
    let ids: HashSet<TroubleshootingTopicId> = guide.troubleshooting.iter().copied().collect();
    if ids.is_empty() {
        return None;
    }
    // Matches the catalog's own order, not `guide.troubleshooting`'s declared
    // order — mirrors `RouterPortForwardGuideRepository.troubleshootingTopics(for:)`,
    // which filters the catalog list rather than mapping the guide's own list.
    let matched: Vec<TroubleshootingTopic> = topics
        .iter()
        .filter(|t| ids.contains(&t.id))
        .cloned()
        .collect();
    if matched.is_empty() {
        return None;
    }

    let mut items = vec![SectionItem::Paragraph {
        title: None,
        body: "If the rule still does not work after you save and test from another network, continue with the matching troubleshooting topics below instead of guessing.".to_string(),
        referenced_tokens: Vec::new(),
    }];
    items.extend(matched.into_iter().map(SectionItem::TroubleshootingTopic));

    Some(ComposedSection {
        id: format!("{}.troubleshooting", guide.id),
        kind: SectionKind::TroubleshootingFooter,
        title: "Still not working?",
        items,
        origin: SectionOrigin::Mixed,
    })
}

fn compose_sections(guide: &Guide, topics: &[TroubleshootingTopic]) -> Vec<ComposedSection> {
    let mut sections = Vec::new();

    if guide.shared_sections.include_shared_intro {
        sections.push(shared_intro_section(guide));
    }
    if guide.shared_sections.include_shared_prerequisites {
        sections.push(shared_prerequisites_section(guide));
    }
    if guide.shared_sections.include_shared_value_summary {
        sections.push(shared_value_summary_section(guide));
    }
    if let Some(section) = menu_path_section(guide) {
        sections.push(section);
    }
    sections.push(router_specific_steps_section(guide));
    if let Some(section) = notes_section(guide) {
        sections.push(section);
    }
    if guide.shared_sections.include_shared_troubleshooting_footer
        && let Some(section) = troubleshooting_footer_section(guide, topics)
    {
        sections.push(section);
    }

    sections
}

pub fn compose_guide<'a>(guide: &'a Guide, topics: &[TroubleshootingTopic]) -> ComposedGuide<'a> {
    ComposedGuide {
        guide,
        sections: compose_sections(guide, topics),
    }
}

pub fn compose_guide_by_id<'a>(
    id: &str,
    catalog: &'a [Guide],
    topics: &[TroubleshootingTopic],
) -> Option<ComposedGuide<'a>> {
    let guide = catalog.iter().find(|g| g.id == id)?;
    Some(compose_guide(guide, topics))
}
