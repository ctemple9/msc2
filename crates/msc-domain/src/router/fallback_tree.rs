//! Port of `RouterPortForwardFallbackDecisionTree.swift` (610 lines, the
//! largest of the five router files) — two loosely-coupled pieces, both
//! ported here since D-026 names "the fallback resolver" and "the...
//! decision tree" as executable behavior together:
//!
//! 1. **The decision tree** (`make_tree`, `unknown_router_bullets`): a fixed
//!    8-node navigation graph plus one genuinely conditional function
//!    (`unknownRouterBullets` inserts a gateway-IP bullet only when one was
//!    detected and is non-blank). `resolve` below never calls `make_tree` at
//!    all — it only returns bare `NodeId` values as "go here next" — so
//!    `DecisionNode`/`DecisionChoice` carry only the fields that describe
//!    *navigation* (`id`, `kind`, each choice's `id`/`next_node_id`/
//!    `implied_network_type`). `title`/`body`/`bullets`/`suggestedSearchTerms`
//!    on tree nodes are UI prose nothing here computes over, excluded per
//!    the port plan's deletion test — unlike `unknown_router_bullets`'s
//!    output, which *is* the tested behavior (whether a bullet appears and
//!    where), not decoration, so it's ported with its literal text intact.
//!    Because tree-node prose is excluded, `make_tree` needs no
//!    `detected_gateway_ip_address` parameter — Swift's version only takes
//!    one to build the `unknownRouterHelp` node's `bullets` field.
//!
//! 2. **The resolution engine** (`resolve`, `RouterPortForwardFallbackRouter`
//!    in Swift): the actual "fallback resolver" — routes a
//!    [`FallbackState`] plus a search query through [`matcher`] and the
//!    guide catalog to one of `ResolutionKind`'s seven outcomes. This is the
//!    bulk of the file's real logic and this step's main fixture target.
//!    `explanationBullets`/`suggestedSearchTerms` here are genuinely
//!    computed (conditional string-building, not static prose) and are
//!    ported in full.
//!
//! `RouterPortForwardGuideRuntimeContext` (used only for its
//! `detectedGatewayIPAddress: String?` field by this file) isn't ported here
//! — P1.14 owns that type. `resolve`/`generic_mesh_resolution` instead take
//! `detected_gateway_ip_address: Option<&str>` directly, the one field this
//! file actually reads.
//!
//! Two branches of `resolve` — the "top candidate exists but doesn't match
//! directly, while a *different*, already-seeded family was still inferred"
//! path (Swift's `familyGuide` return at line 346-359) and the matching
//! "inferred family is troubleshooting but wasn't the top candidate's own
//! family" path (line 336-344) — are not exercised by this domain's
//! fixtures. Both require a guide to out-score every guide whose own family
//! got inferred, but inferring a family always hands that family's guide a
//! flat +70 in `scoreCandidate`, which this sample catalog's data (built
//! from MSC 1's real keyword/alias lists, which overlap heavily by design)
//! makes very hard to out-score with an unrelated guide without an
//! artificial, unrealistic query. Real source behavior, not a gap in this
//! port; noted rather than silently skipped.

use crate::router::matcher::{self, Guide, GuideFamily};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeId {
    Start,
    IspProviderChoice,
    OwnRouterBrandChoice,
    MeshBrandChoice,
    IspVsRouterClarifier,
    UnknownRouterHelp,
    OptionalSearch,
    AdvancedTroubleshooting,
}

impl NodeId {
    /// Swift declares no explicit `rawValue` strings for this enum, so its
    /// implicit raw value is the case name verbatim — camelCase, unlike
    /// `GuideFamily`/`GuideCategory`'s explicit snake_case in `matcher.rs`.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::IspProviderChoice => "ispProviderChoice",
            Self::OwnRouterBrandChoice => "ownRouterBrandChoice",
            Self::MeshBrandChoice => "meshBrandChoice",
            Self::IspVsRouterClarifier => "ispVsRouterClarifier",
            Self::UnknownRouterHelp => "unknownRouterHelp",
            Self::OptionalSearch => "optionalSearch",
            Self::AdvancedTroubleshooting => "advancedTroubleshooting",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "start" => Self::Start,
            "ispProviderChoice" => Self::IspProviderChoice,
            "ownRouterBrandChoice" => Self::OwnRouterBrandChoice,
            "meshBrandChoice" => Self::MeshBrandChoice,
            "ispVsRouterClarifier" => Self::IspVsRouterClarifier,
            "unknownRouterHelp" => Self::UnknownRouterHelp,
            "optionalSearch" => Self::OptionalSearch,
            "advancedTroubleshooting" => Self::AdvancedTroubleshooting,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    SingleChoice,
    FreeTextSearch,
    Info,
}

impl NodeKind {
    /// Also an implicit (camelCase) raw value — see `NodeId::raw_value`.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::SingleChoice => "singleChoice",
            Self::FreeTextSearch => "freeTextSearch",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    IspGateway,
    OwnRouter,
    MeshSystem,
    NotSure,
}

impl NetworkType {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::IspGateway => "isp_gateway",
            Self::OwnRouter => "own_router",
            Self::MeshSystem => "mesh_system",
            Self::NotSure => "not_sure",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "isp_gateway" => Self::IspGateway,
            "own_router" => Self::OwnRouter,
            "mesh_system" => Self::MeshSystem,
            "not_sure" => Self::NotSure,
            _ => return None,
        })
    }
}

/// See the module doc comment for which `RouterPortForwardDecisionChoice`
/// fields are excluded (`title`, `suggestedSearchTerms`).
#[derive(Debug, Clone)]
pub struct DecisionChoice {
    pub id: &'static str,
    pub next_node_id: Option<NodeId>,
    pub implied_network_type: Option<NetworkType>,
}

/// See the module doc comment for which `RouterPortForwardDecisionNode`
/// fields are excluded (`title`, `body`, `bullets`).
#[derive(Debug, Clone)]
pub struct DecisionNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub choices: Vec<DecisionChoice>,
}

pub fn make_tree() -> Vec<DecisionNode> {
    vec![
        DecisionNode {
            id: NodeId::Start,
            kind: NodeKind::SingleChoice,
            choices: vec![
                DecisionChoice {
                    id: "start-isp",
                    next_node_id: Some(NodeId::IspProviderChoice),
                    implied_network_type: Some(NetworkType::IspGateway),
                },
                DecisionChoice {
                    id: "start-own",
                    next_node_id: Some(NodeId::OwnRouterBrandChoice),
                    implied_network_type: Some(NetworkType::OwnRouter),
                },
                DecisionChoice {
                    id: "start-mesh",
                    next_node_id: Some(NodeId::MeshBrandChoice),
                    implied_network_type: Some(NetworkType::MeshSystem),
                },
                DecisionChoice {
                    id: "start-unsure",
                    next_node_id: Some(NodeId::UnknownRouterHelp),
                    implied_network_type: Some(NetworkType::NotSure),
                },
            ],
        },
        DecisionNode {
            id: NodeId::IspProviderChoice,
            kind: NodeKind::FreeTextSearch,
            choices: vec![],
        },
        DecisionNode {
            id: NodeId::OwnRouterBrandChoice,
            kind: NodeKind::FreeTextSearch,
            choices: vec![],
        },
        DecisionNode {
            id: NodeId::MeshBrandChoice,
            kind: NodeKind::FreeTextSearch,
            choices: vec![],
        },
        DecisionNode {
            id: NodeId::IspVsRouterClarifier,
            kind: NodeKind::Info,
            choices: vec![DecisionChoice {
                id: "clarifier-search",
                next_node_id: Some(NodeId::OptionalSearch),
                implied_network_type: None,
            }],
        },
        DecisionNode {
            id: NodeId::UnknownRouterHelp,
            kind: NodeKind::Info,
            choices: vec![
                DecisionChoice {
                    id: "unknown-search",
                    next_node_id: Some(NodeId::OptionalSearch),
                    implied_network_type: None,
                },
                DecisionChoice {
                    id: "unknown-advanced",
                    next_node_id: Some(NodeId::AdvancedTroubleshooting),
                    implied_network_type: None,
                },
            ],
        },
        DecisionNode {
            id: NodeId::OptionalSearch,
            kind: NodeKind::FreeTextSearch,
            choices: vec![],
        },
        DecisionNode {
            id: NodeId::AdvancedTroubleshooting,
            kind: NodeKind::Info,
            choices: vec![],
        },
    ]
}

pub fn unknown_router_bullets(detected_gateway_ip_address: Option<&str>) -> Vec<String> {
    let mut bullets: Vec<String> = vec![
        "Look for the model name on the sticker on the router, gateway, or mesh node.".to_string(),
        "Check which router or mesh apps are already installed on your phone or Mac.".to_string(),
        "Identify whether the ISP device or your own router is actually doing routing.".to_string(),
        "Common gateway addresses include 192.168.1.1, 192.168.0.1, and 10.0.0.1.".to_string(),
        "Port forwarding may also be labeled NAT Forwarding, Virtual Server, Applications & Gaming, Port Rules, Firewall Rules, or Advanced NAT."
            .to_string(),
    ];

    if let Some(ip) = detected_gateway_ip_address
        && !ip.trim().is_empty()
    {
        bullets.insert(3, format!("Detected gateway IP from the app: {ip}"));
    }

    bullets
}

#[derive(Debug, Clone, Default)]
pub struct FallbackState {
    pub network_type: Option<NetworkType>,
    pub search_query: String,
    pub only_knows_isp: bool,
    pub only_knows_mesh_system: bool,
    pub unsure_whether_isp_or_own_router: bool,
    pub wants_advanced_troubleshooting: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    ExactGuide,
    FamilyGuide,
    GenericRouterGuide,
    GenericMeshGuide,
    TroubleshootingGuide,
    UnknownRouterHelp,
    NeedsMoreInfo,
}

impl ResolutionKind {
    /// Implicit (camelCase) raw value, same as `NodeId`. Coincidentally
    /// shares the literal string "unknownRouterHelp" with `NodeId::UnknownRouterHelp`
    /// — two unrelated Swift enums, no relationship beyond the spelling.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::ExactGuide => "exactGuide",
            Self::FamilyGuide => "familyGuide",
            Self::GenericRouterGuide => "genericRouterGuide",
            Self::GenericMeshGuide => "genericMeshGuide",
            Self::TroubleshootingGuide => "troubleshootingGuide",
            Self::UnknownRouterHelp => "unknownRouterHelp",
            Self::NeedsMoreInfo => "needsMoreInfo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideAvailability {
    ExactMatch,
    FamilyMatch,
    FallbackUsed,
    DesiredGuideNotSeededYet,
}

impl GuideAvailability {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::ExactMatch => "exactMatch",
            Self::FamilyMatch => "familyMatch",
            Self::FallbackUsed => "fallbackUsed",
            Self::DesiredGuideNotSeededYet => "desiredGuideNotSeededYet",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FallbackResolution<'a> {
    pub kind: ResolutionKind,
    pub availability: GuideAvailability,
    pub matched_guide: Option<&'a Guide>,
    pub fallback_guide: Option<&'a Guide>,
    pub desired_family: Option<GuideFamily>,
    pub inferred_families: Vec<GuideFamily>,
    pub explanation_bullets: Vec<String>,
    pub recommended_next_node_id: Option<NodeId>,
    pub suggested_search_terms: &'static [&'static str],
    pub matched_query: Option<String>,
}

fn is_mesh_family(family: GuideFamily) -> bool {
    // Swift lists `.tpLink` explicitly as `false`, identical to its own
    // `default: return false` right below — redundant in the source, kept
    // as an equivalent match here rather than silently collapsed.
    matches!(
        family,
        GuideFamily::GenericMesh | GuideFamily::Eero | GuideFamily::GoogleNest
    )
}

fn best_available_generic_router_guide(catalog: &[Guide]) -> Option<&Guide> {
    catalog
        .iter()
        .find(|g| g.family == GuideFamily::GenericRouter)
        .or_else(|| catalog.first())
}

fn best_available_generic_mesh_or_router_guide(catalog: &[Guide]) -> Option<&Guide> {
    catalog
        .iter()
        .find(|g| g.family == GuideFamily::GenericMesh)
        .or_else(|| best_available_generic_router_guide(catalog))
}

fn explanation_bullets_for_match(
    state: &FallbackState,
    match_result: &matcher::MatchResult,
    guide: &Guide,
) -> Vec<String> {
    let mut bullets = Vec::new();

    if let Some(top_candidate) = match_result.candidates.first()
        && !top_candidate.reasons.is_empty()
    {
        bullets.push(format!(
            "Matched this guide because: {}.",
            top_candidate.reasons.join(", ")
        ));
    }

    if state.unsure_whether_isp_or_own_router {
        bullets.push(
            "Verify that this is the device doing routing before you make changes.".to_string(),
        );
    }

    if let (Some(provider), Some(device)) =
        (&guide.provider_display_name, &guide.device_display_name)
        && provider.to_lowercase() != device.to_lowercase()
    {
        bullets.push(format!(
            "Provider: {provider}. Device family: {device}. Do not treat those as interchangeable."
        ));
    }

    bullets
}

fn troubleshooting_resolution<'a>(
    catalog: &'a [Guide],
    inferred_families: Vec<GuideFamily>,
    query: &str,
    mut explanation_bullets: Vec<String>,
) -> FallbackResolution<'a> {
    let guide = catalog
        .iter()
        .find(|g| g.family == GuideFamily::AdvancedTroubleshooting);
    explanation_bullets.push(
        "Use the advanced troubleshooting path for double NAT, CGNAT, wrong-router configuration, firewall blocks, or ISP restrictions.".to_string(),
    );

    FallbackResolution {
        kind: ResolutionKind::TroubleshootingGuide,
        availability: if guide.is_none() {
            GuideAvailability::DesiredGuideNotSeededYet
        } else {
            GuideAvailability::FamilyMatch
        },
        matched_guide: guide,
        fallback_guide: None,
        desired_family: Some(GuideFamily::AdvancedTroubleshooting),
        inferred_families,
        explanation_bullets,
        recommended_next_node_id: Some(NodeId::AdvancedTroubleshooting),
        suggested_search_terms: &["double NAT", "CGNAT", "wrong router", "firewall"],
        matched_query: if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        },
    }
}

fn recognized_family_fallback_resolution<'a>(
    catalog: &'a [Guide],
    desired_family: GuideFamily,
    inferred_families: Vec<GuideFamily>,
    query: &str,
    state: &FallbackState,
) -> FallbackResolution<'a> {
    if desired_family == GuideFamily::AdvancedTroubleshooting {
        return troubleshooting_resolution(catalog, inferred_families, query, Vec::new());
    }

    let mesh = is_mesh_family(desired_family);
    let kind = if mesh {
        ResolutionKind::GenericMeshGuide
    } else {
        ResolutionKind::GenericRouterGuide
    };
    let fallback_guide = if mesh {
        best_available_generic_mesh_or_router_guide(catalog)
    } else {
        best_available_generic_router_guide(catalog)
    };

    let mut bullets = vec![
        format!(
            "The matcher recognized the family {}, but that family guide is not seeded in the current project yet.",
            desired_family.raw_value()
        ),
        "Use the fallback guide for the general flow, then continue to advanced troubleshooting if the router labels differ too much."
            .to_string(),
    ];
    if state.unsure_whether_isp_or_own_router {
        bullets.push("Before changing settings, confirm whether the ISP hardware or your own router is actually doing routing.".to_string());
    }

    FallbackResolution {
        kind,
        availability: GuideAvailability::DesiredGuideNotSeededYet,
        matched_guide: None,
        fallback_guide,
        desired_family: Some(desired_family),
        inferred_families,
        explanation_bullets: bullets,
        recommended_next_node_id: None,
        suggested_search_terms: &[],
        matched_query: if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        },
    }
}

fn generic_router_resolution<'a>(catalog: &'a [Guide], query: &str) -> FallbackResolution<'a> {
    let guide = best_available_generic_router_guide(catalog);
    FallbackResolution {
        kind: ResolutionKind::GenericRouterGuide,
        availability: GuideAvailability::FallbackUsed,
        matched_guide: guide,
        fallback_guide: None,
        desired_family: Some(GuideFamily::GenericRouter),
        inferred_families: Vec::new(),
        explanation_bullets: vec![
            "No exact family match was found, so the generic router guide is the safest fallback.".to_string(),
            "Use menu aliases such as NAT Forwarding, Virtual Server, Applications & Gaming, Port Rules, Firewall Rules, or Advanced NAT while you search the router interface.".to_string(),
        ],
        recommended_next_node_id: None,
        suggested_search_terms: &[],
        matched_query: if query.is_empty() { None } else { Some(query.to_string()) },
    }
}

fn generic_mesh_resolution<'a>(
    catalog: &'a [Guide],
    query: &str,
    detected_gateway_ip_address: Option<&str>,
) -> FallbackResolution<'a> {
    if let Some(generic_mesh) = catalog
        .iter()
        .find(|g| g.family == GuideFamily::GenericMesh)
    {
        return FallbackResolution {
            kind: ResolutionKind::GenericMeshGuide,
            availability: GuideAvailability::FamilyMatch,
            matched_guide: Some(generic_mesh),
            fallback_guide: None,
            desired_family: Some(GuideFamily::GenericMesh),
            inferred_families: vec![GuideFamily::GenericMesh],
            explanation_bullets: vec!["A mesh-specific fallback guide is available for app-managed or mesh-first systems.".to_string()],
            recommended_next_node_id: None,
            suggested_search_terms: &[],
            matched_query: if query.is_empty() { None } else { Some(query.to_string()) },
        };
    }

    let bridge_bullet = match detected_gateway_ip_address {
        None => "If the mesh is in bridge mode, forwarding may need to happen on another router."
            .to_string(),
        Some(ip) => format!(
            "If the mesh is in bridge mode, forwarding may need to happen on the gateway at {ip}."
        ),
    };

    FallbackResolution {
        kind: ResolutionKind::GenericMeshGuide,
        availability: GuideAvailability::DesiredGuideNotSeededYet,
        matched_guide: None,
        fallback_guide: best_available_generic_router_guide(catalog),
        desired_family: Some(GuideFamily::GenericMesh),
        inferred_families: vec![GuideFamily::GenericMesh],
        explanation_bullets: vec![
            "The current seed catalog does not include a dedicated generic mesh guide yet.".to_string(),
            "Use the generic router guide as the fallback, but expect more app-based wording and bridge-mode edge cases for mesh systems."
                .to_string(),
            bridge_bullet,
        ],
        recommended_next_node_id: None,
        suggested_search_terms: &["eero", "Deco", "Nest"],
        matched_query: if query.is_empty() { None } else { Some(query.to_string()) },
    }
}

fn unknown_router_explanation_bullets(detected_gateway_ip_address: Option<&str>) -> Vec<String> {
    let mut bullets = vec![
        "Check the sticker on the router, gateway, or mesh node for the brand and model.".to_string(),
        "Check your phone or Mac for router apps that may already identify the brand.".to_string(),
        "Separate the provider name from the device doing routing before you continue.".to_string(),
        "Common local gateway addresses are 192.168.1.1, 192.168.0.1, and 10.0.0.1.".to_string(),
        "Port forwarding may also be labeled NAT Forwarding, Virtual Server, Applications & Gaming, Port Rules, Firewall Rules, or Advanced NAT."
            .to_string(),
    ];

    if let Some(ip) = detected_gateway_ip_address
        && !ip.trim().is_empty()
    {
        bullets.insert(3, format!("The app detected a gateway IP of {ip}."));
    }

    bullets
}

pub fn resolve<'a>(
    state: &FallbackState,
    catalog: &'a [Guide],
    detected_gateway_ip_address: Option<&str>,
) -> FallbackResolution<'a> {
    let query = state.search_query.trim();

    if state.wants_advanced_troubleshooting {
        return troubleshooting_resolution(
            catalog,
            vec![GuideFamily::AdvancedTroubleshooting],
            query,
            vec!["Advanced troubleshooting is the right path when forwarding still fails or when the network may be blocked by double NAT, CGNAT, or ISP restrictions.".to_string()],
        );
    }

    if !query.is_empty() {
        let m = matcher::match_query(query, catalog);

        if let Some(top_candidate) = m.candidates.first() {
            if top_candidate.guide.family == GuideFamily::AdvancedTroubleshooting {
                return troubleshooting_resolution(
                    catalog,
                    m.inferred_families,
                    query,
                    vec!["Your search looks like a networking failure or restriction rather than a router identification request.".to_string()],
                );
            }

            if m.matched_direct_guide {
                let exact = top_candidate.reasons.contains(&"exact keyword");
                let kind = if exact {
                    ResolutionKind::ExactGuide
                } else {
                    ResolutionKind::FamilyGuide
                };
                let availability = if exact {
                    GuideAvailability::ExactMatch
                } else {
                    GuideAvailability::FamilyMatch
                };
                let guide = top_candidate.guide;
                return FallbackResolution {
                    kind,
                    availability,
                    matched_guide: Some(guide),
                    fallback_guide: None,
                    desired_family: Some(guide.family),
                    inferred_families: m.inferred_families.clone(),
                    explanation_bullets: explanation_bullets_for_match(state, &m, guide),
                    recommended_next_node_id: None,
                    suggested_search_terms: &[],
                    matched_query: Some(query.to_string()),
                };
            }
        }

        if let Some(&first_recognized_family) = m.inferred_families.first() {
            if first_recognized_family == GuideFamily::AdvancedTroubleshooting {
                return troubleshooting_resolution(
                    catalog,
                    m.inferred_families,
                    query,
                    vec!["The matcher recognized an advanced networking problem rather than a supported router family.".to_string()],
                );
            }

            if let Some(direct_family_guide) =
                catalog.iter().find(|g| g.family == first_recognized_family)
            {
                return FallbackResolution {
                    kind: ResolutionKind::FamilyGuide,
                    availability: GuideAvailability::FamilyMatch,
                    matched_guide: Some(direct_family_guide),
                    fallback_guide: None,
                    desired_family: Some(first_recognized_family),
                    inferred_families: m.inferred_families.clone(),
                    explanation_bullets: explanation_bullets_for_match(
                        state,
                        &m,
                        direct_family_guide,
                    ),
                    recommended_next_node_id: None,
                    suggested_search_terms: &[],
                    matched_query: Some(query.to_string()),
                };
            }

            return recognized_family_fallback_resolution(
                catalog,
                first_recognized_family,
                m.inferred_families,
                query,
                state,
            );
        }
    }

    match state.network_type {
        Some(NetworkType::MeshSystem) => {
            return generic_mesh_resolution(catalog, query, detected_gateway_ip_address);
        }
        Some(NetworkType::IspGateway) => {
            return FallbackResolution {
                kind: ResolutionKind::NeedsMoreInfo,
                availability: GuideAvailability::FallbackUsed,
                matched_guide: None,
                fallback_guide: None,
                desired_family: None,
                inferred_families: Vec::new(),
                explanation_bullets: vec![
                    "Choose the provider gateway family next, or search by gateway line such as XB7 or BGW320.".to_string(),
                    "If your own router is behind the provider modem, use the router path instead.".to_string(),
                ],
                recommended_next_node_id: Some(if state.unsure_whether_isp_or_own_router {
                    NodeId::IspVsRouterClarifier
                } else {
                    NodeId::IspProviderChoice
                }),
                suggested_search_terms: &["Xfinity", "Spectrum", "AT&T", "Fios", "XB7", "BGW320"],
                matched_query: None,
            };
        }
        Some(NetworkType::OwnRouter) => {
            return FallbackResolution {
                kind: ResolutionKind::NeedsMoreInfo,
                availability: GuideAvailability::FallbackUsed,
                matched_guide: None,
                fallback_guide: None,
                desired_family: None,
                inferred_families: Vec::new(),
                explanation_bullets: vec![
                    "Search by router brand or product line next.".to_string(),
                    "Common examples include ASUS, TP-Link, Netgear, Linksys, Nighthawk, and Archer.".to_string(),
                ],
                recommended_next_node_id: Some(NodeId::OwnRouterBrandChoice),
                suggested_search_terms: &["ASUS", "TP-Link", "Netgear", "Linksys", "Nighthawk", "Archer"],
                matched_query: None,
            };
        }
        Some(NetworkType::NotSure) | None => {}
    }

    if state.only_knows_mesh_system {
        return generic_mesh_resolution(catalog, query, detected_gateway_ip_address);
    }

    if state.only_knows_isp
        || state.unsure_whether_isp_or_own_router
        || state.network_type == Some(NetworkType::NotSure)
        || query.is_empty()
    {
        return FallbackResolution {
            kind: ResolutionKind::UnknownRouterHelp,
            availability: GuideAvailability::FallbackUsed,
            matched_guide: None,
            fallback_guide: best_available_generic_router_guide(catalog),
            desired_family: Some(GuideFamily::UnknownRouter),
            inferred_families: Vec::new(),
            explanation_bullets: unknown_router_explanation_bullets(detected_gateway_ip_address),
            recommended_next_node_id: Some(if state.unsure_whether_isp_or_own_router {
                NodeId::IspVsRouterClarifier
            } else {
                NodeId::UnknownRouterHelp
            }),
            suggested_search_terms: &["Xfinity", "Spectrum", "ASUS", "eero", "Deco", "Nighthawk"],
            matched_query: None,
        };
    }

    generic_router_resolution(catalog, query)
}
