//! Port of `RouterPortForwardGuideMatcher.swift`'s `match(_:)` — "scores guide
//! candidates against user input and returns ranked results with confidence
//! metadata" (the file's own doc comment). No MSC 1 test file exists for this
//! engine; `fixtures/router-matcher/` characterizes it fresh by running the
//! literal Swift source (matcher plus a 6-guide sample catalog) through
//! `swift`, not by hand-deriving expected scores — the scoring arithmetic
//! stacks enough simultaneous bonuses that hand-derivation produced three
//! wrong predictions during this port's own preparation (see the fixture
//! `notes` fields), the same lesson P1.9's off-by-one taught for this
//! technique in general.
//!
//! `Guide` here only carries the fields `scoreCandidate`/`suggestedFallbackGuide`
//! touch (`id`, `displayName`, `category`, `family`, `searchKeywords`,
//! `adminSurface`, `providerDisplayName`, `deviceDisplayName`). MSC 1's
//! `RouterPortForwardGuide` also has `adminAddresses`, `menuPath`,
//! `alternateMenuNames`, `steps`, `notes`, `troubleshooting`, `sharedSections`,
//! and `review` — read only by the composer/troubleshooting engines P1.12/P1.13
//! port later, so per the port plan's deletion test they're left out of this
//! step's `Guide` type. `fixtures/router-sample-catalog.json` (this step's
//! other deliverable) still carries the full guide shape, since P1.11-P1.13
//! reuse that same file.
//!
//! Two deliberate gaps from a byte-for-byte port, both because nothing in
//! this domain's fixtures exercises them:
//! - `normalize` skips Swift's `.folding(options: [.diacriticInsensitive, ...])`
//!   Unicode NFKD fold. `msc-domain` has no Unicode-normalization dependency
//!   (only `regex`), and every guide/keyword/query in the sample catalog and
//!   fixtures is plain ASCII. Genuine diacritic-insensitive matching (e.g. a
//!   router brand with an accented name) would need a normalization crate
//!   added to `Cargo.toml` first — out of scope for this step.
//! - the tie-break comparison uses `str::to_lowercase` instead of Swift's
//!   locale-aware `localizedCaseInsensitiveCompare` — equivalent for the
//!   ASCII display names in this domain, not necessarily for all locales.
//!
//! One faithfully-preserved source quirk, not a bug this port introduces:
//! `inferredIntent`'s unknown-intent branch checks
//! `normalizedQuery.contains("don't know")`, but `normalize` strips every
//! apostrophe before this check ever runs, so that specific literal can never
//! match — dead code in MSC 1 itself. Ported as-is rather than silently
//! dropped, since deleting it would be an undocumented behavior change (in
//! this case a no-op one, but the principle carries to less obviously-dead
//! branches).

use std::collections::HashSet;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideCategory {
    IspGateway,
    RetailRouter,
    MeshSystem,
    GenericFallback,
    AdvancedNetworking,
}

impl GuideCategory {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::IspGateway => "isp_gateway",
            Self::RetailRouter => "retail_router",
            Self::MeshSystem => "mesh_system",
            Self::GenericFallback => "generic_fallback",
            Self::AdvancedNetworking => "advanced_networking",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "isp_gateway" => Self::IspGateway,
            "retail_router" => Self::RetailRouter,
            "mesh_system" => Self::MeshSystem,
            "generic_fallback" => Self::GenericFallback,
            "advanced_networking" => Self::AdvancedNetworking,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideFamily {
    GenericRouter,
    GenericMesh,
    UnknownRouter,
    XfinityGateway,
    SpectrumGateway,
    AttGateway,
    FiosRouter,
    CoxGateway,
    Asus,
    TpLink,
    Netgear,
    Linksys,
    Eero,
    GoogleNest,
    AdvancedTroubleshooting,
}

impl GuideFamily {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::GenericRouter => "generic_router",
            Self::GenericMesh => "generic_mesh",
            Self::UnknownRouter => "unknown_router",
            Self::XfinityGateway => "xfinity_gateway",
            Self::SpectrumGateway => "spectrum_gateway",
            Self::AttGateway => "att_gateway",
            Self::FiosRouter => "fios_router",
            Self::CoxGateway => "cox_gateway",
            Self::Asus => "asus",
            Self::TpLink => "tp_link",
            Self::Netgear => "netgear",
            Self::Linksys => "linksys",
            Self::Eero => "eero",
            Self::GoogleNest => "google_nest",
            Self::AdvancedTroubleshooting => "advanced_troubleshooting",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "generic_router" => Self::GenericRouter,
            "generic_mesh" => Self::GenericMesh,
            "unknown_router" => Self::UnknownRouter,
            "xfinity_gateway" => Self::XfinityGateway,
            "spectrum_gateway" => Self::SpectrumGateway,
            "att_gateway" => Self::AttGateway,
            "fios_router" => Self::FiosRouter,
            "cox_gateway" => Self::CoxGateway,
            "asus" => Self::Asus,
            "tp_link" => Self::TpLink,
            "netgear" => Self::Netgear,
            "linksys" => Self::Linksys,
            "eero" => Self::Eero,
            "google_nest" => Self::GoogleNest,
            "advanced_troubleshooting" => Self::AdvancedTroubleshooting,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminSurface {
    WebBrowser,
    MobileApp,
    Either,
}

impl AdminSurface {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::WebBrowser => "web_browser",
            Self::MobileApp => "mobile_app",
            Self::Either => "either",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "web_browser" => Self::WebBrowser,
            "mobile_app" => Self::MobileApp,
            "either" => Self::Either,
            _ => return None,
        })
    }
}

/// See the module doc comment for which `RouterPortForwardGuide` fields this
/// step's `Guide` deliberately leaves out.
#[derive(Debug, Clone)]
pub struct Guide {
    pub id: String,
    pub display_name: String,
    pub category: GuideCategory,
    pub family: GuideFamily,
    pub search_keywords: Vec<String>,
    pub admin_surface: AdminSurface,
    pub provider_display_name: Option<String>,
    pub device_display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchCandidate<'a> {
    pub guide: &'a Guide,
    pub score: i32,
    pub reasons: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct MatchResult<'a> {
    pub normalized_query: String,
    pub normalized_tokens: Vec<String>,
    pub inferred_families: Vec<GuideFamily>,
    pub candidates: Vec<MatchCandidate<'a>>,
    pub suggested_fallback_guide: Option<&'a Guide>,
    pub is_ambiguous: bool,
    pub matched_direct_guide: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchIntent {
    GuideLookup,
    Troubleshooting,
    Mesh,
    Unknown,
}

struct FamilyAliasRule {
    family: GuideFamily,
    aliases: &'static [&'static str],
}

const FAMILY_ALIAS_RULES: &[FamilyAliasRule] = &[
    FamilyAliasRule {
        family: GuideFamily::XfinityGateway,
        aliases: &[
            "xfinity",
            "comcast",
            "comcast xfinity",
            "xfi",
            "xb6",
            "xb7",
            "xb8",
            "xfinity gateway",
            "comcast gateway",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::SpectrumGateway,
        aliases: &[
            "spectrum",
            "charter",
            "charter spectrum",
            "spectrum router",
            "spectrum gateway",
            "sax1v1k",
            "rac2v1k",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::AttGateway,
        aliases: &[
            "att",
            "at&t",
            "at and t",
            "uverse",
            "u verse",
            "bgw210",
            "bgw320",
            "5268ac",
            "att gateway",
            "at&t gateway",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::FiosRouter,
        aliases: &[
            "fios",
            "verizon",
            "verizon fios",
            "verizon router",
            "fios router",
            "g3100",
            "cr1000a",
            "cr1000b",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::CoxGateway,
        aliases: &[
            "cox",
            "panoramic wifi",
            "cox panoramic",
            "cox gateway",
            "pw3",
            "pw6",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::Asus,
        aliases: &[
            "asus",
            "asus router",
            "rt ax",
            "rt ac",
            "rog router",
            "zenwifi",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::TpLink,
        aliases: &["tp link", "tplink", "deco", "archer", "omada"],
    },
    FamilyAliasRule {
        family: GuideFamily::Netgear,
        aliases: &["netgear", "nighthawk", "orbi"],
    },
    FamilyAliasRule {
        family: GuideFamily::Linksys,
        aliases: &["linksys", "velop"],
    },
    FamilyAliasRule {
        family: GuideFamily::Eero,
        aliases: &["eero", "eero pro", "eero mesh"],
    },
    FamilyAliasRule {
        family: GuideFamily::GoogleNest,
        aliases: &["google wifi", "google nest", "nest wifi", "nest pro"],
    },
    FamilyAliasRule {
        family: GuideFamily::AdvancedTroubleshooting,
        aliases: &[
            "double nat",
            "cgnat",
            "wrong router",
            "firewall",
            "port forwarding not working",
        ],
    },
    FamilyAliasRule {
        family: GuideFamily::GenericMesh,
        aliases: &["mesh", "mesh wifi", "app managed router", "app managed"],
    },
    FamilyAliasRule {
        family: GuideFamily::UnknownRouter,
        aliases: &[
            "unknown router",
            "dont know my router",
            "don't know my router",
            "not sure what router",
        ],
    },
];

const TROUBLESHOOTING_TERMS: &[&str] = &[
    "double",
    "nat",
    "cgnat",
    "firewall",
    "blocked",
    "wrong",
    "router",
    "device",
    "reboot",
    "passthrough",
    "bridge",
    "stuck",
    "failing",
    "failed",
    "working",
];

const STOP_WORDS: &[&str] = &["the", "a", "an", "my", "i", "have", "wifi"];

fn normalize(raw: &str) -> String {
    let folded = raw.to_lowercase().replace('&', " and ").replace('\'', "");
    let non_alnum = Regex::new(r"[^a-z0-9]+").expect("static regex");
    let collapsed = non_alnum.replace_all(&folded, " ");
    let whitespace = Regex::new(r"\s+").expect("static regex");
    whitespace.replace_all(&collapsed, " ").trim().to_string()
}

fn tokens_from_normalized(normalized: &str) -> Vec<String> {
    if normalized.is_empty() {
        return Vec::new();
    }
    normalized
        .split(' ')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && !STOP_WORDS.contains(&s.as_str()))
        .collect()
}

fn infer_families(normalized_query: &str, tokens: &[String]) -> Vec<GuideFamily> {
    let token_set: HashSet<&str> = tokens.iter().map(String::as_str).collect();
    let mut inferred = Vec::new();

    for rule in FAMILY_ALIAS_RULES {
        let matched = rule.aliases.iter().any(|alias| {
            let normalized_alias = normalize(alias);
            if normalized_alias.is_empty() {
                return false;
            }
            if normalized_query == normalized_alias {
                return true;
            }
            if normalized_query.contains(&normalized_alias) {
                return true;
            }
            let alias_tokens = tokens_from_normalized(&normalized_alias);
            !alias_tokens.is_empty() && alias_tokens.iter().all(|t| token_set.contains(t.as_str()))
        });

        if matched && !inferred.contains(&rule.family) {
            inferred.push(rule.family);
        }
    }

    inferred
}

fn inferred_intent(
    normalized_query: &str,
    tokens: &[String],
    inferred_families: &[GuideFamily],
) -> MatchIntent {
    let token_set: HashSet<&str> = tokens.iter().map(String::as_str).collect();

    if token_set.contains("mesh")
        || token_set.contains("deco")
        || token_set.contains("eero")
        || token_set.contains("nest")
    {
        return MatchIntent::Mesh;
    }

    let has_troubleshooting_token = TROUBLESHOOTING_TERMS.iter().any(|t| token_set.contains(t));
    if has_troubleshooting_token
        && (normalized_query.contains("not working")
            || normalized_query.contains("trouble")
            || normalized_query.contains("fail")
            || inferred_families.contains(&GuideFamily::AdvancedTroubleshooting))
    {
        return MatchIntent::Troubleshooting;
    }

    // `.contains("don't know")` can never match: `normalize` strips every
    // apostrophe before this line runs. See the module doc comment.
    if normalized_query.is_empty()
        || normalized_query == "router"
        || normalized_query == "generic router"
        || normalized_query.contains("dont know")
        || normalized_query.contains("don't know")
        || normalized_query.contains("unknown")
    {
        return MatchIntent::Unknown;
    }

    MatchIntent::GuideLookup
}

fn score_candidate<'a>(
    guide: &'a Guide,
    normalized_query: &str,
    normalized_tokens: &[String],
    inferred_families: &[GuideFamily],
    intent: MatchIntent,
) -> MatchCandidate<'a> {
    let mut score: i32 = 0;
    let mut reasons: Vec<&'static str> = Vec::new();

    let guide_keywords: Vec<String> = guide.search_keywords.iter().map(|k| normalize(k)).collect();
    let provider_name = normalize(guide.provider_display_name.as_deref().unwrap_or(""));
    let device_name = normalize(guide.device_display_name.as_deref().unwrap_or(""));
    let display_name = normalize(&guide.display_name);

    if !normalized_query.is_empty() {
        if guide_keywords
            .iter()
            .any(|k| k.as_str() == normalized_query)
            || provider_name == normalized_query
            || device_name == normalized_query
            || display_name == normalized_query
        {
            score += 120;
            reasons.push("exact keyword");
        }

        if guide_keywords
            .iter()
            .any(|k| k.starts_with(normalized_query) && k.as_str() != normalized_query)
        {
            score += 48;
            reasons.push("keyword prefix");
        }

        if guide_keywords
            .iter()
            .any(|k| k.contains(normalized_query) && k.as_str() != normalized_query)
        {
            score += 28;
            reasons.push("keyword substring");
        }
    }

    let mut guide_token_set: HashSet<String> = HashSet::new();
    for text in guide
        .search_keywords
        .iter()
        .chain(std::iter::once(&guide.display_name))
        .chain(guide.provider_display_name.iter())
        .chain(guide.device_display_name.iter())
    {
        guide_token_set.extend(tokens_from_normalized(&normalize(text)));
    }

    let query_token_set: HashSet<&str> = normalized_tokens.iter().map(String::as_str).collect();
    let overlap_count = query_token_set
        .iter()
        .filter(|t| guide_token_set.contains(**t))
        .count();
    if overlap_count > 0 {
        score += (overlap_count as i32 * 14).min(40);
        reasons.push("token overlap");
    }

    if inferred_families.contains(&guide.family) {
        score += 70;
        reasons.push("family alias");
    }

    match intent {
        MatchIntent::Troubleshooting => {
            if guide.family == GuideFamily::AdvancedTroubleshooting {
                score += 55;
                reasons.push("troubleshooting intent");
            }
        }
        MatchIntent::Mesh => {
            if score > 0 && guide.category == GuideCategory::MeshSystem {
                score += 22;
                reasons.push("mesh intent");
            }
        }
        MatchIntent::Unknown => {
            if guide.family == GuideFamily::GenericRouter
                || guide.family == GuideFamily::UnknownRouter
            {
                score += 20;
                reasons.push("unknown router fallback");
            }
        }
        MatchIntent::GuideLookup => {}
    }

    if score > 0
        && (normalized_query.contains("isp")
            || normalized_query.contains("gateway")
            || normalized_query.contains("modem"))
        && guide.category == GuideCategory::IspGateway
    {
        score += 12;
        reasons.push("isp/gateway hint");
    }

    if score > 0
        && (normalized_query.contains("app") || normalized_query.contains("mesh"))
        && (guide.admin_surface == AdminSurface::MobileApp
            || guide.admin_surface == AdminSurface::Either)
    {
        score += 10;
        reasons.push("app-managed hint");
    }

    score = score.min(250);

    MatchCandidate {
        guide,
        score,
        reasons,
    }
}

/// Swift guards each branch with `availableFamilies.contains(family)` before
/// looking the family up in the repository; skipped here since a family with
/// no guide in `catalog` already makes the lookup return `None`, the same
/// outcome — one fewer redundant existence check, no behavior change.
fn suggested_fallback_guide<'a>(
    catalog: &'a [Guide],
    inferred_families: &[GuideFamily],
    intent: MatchIntent,
    excluded_guide_id: Option<&str>,
) -> Option<&'a Guide> {
    let by_family = |family: GuideFamily| catalog.iter().find(|g| g.family == family);
    let not_excluded = |g: &&'a Guide| Some(g.id.as_str()) != excluded_guide_id;

    if intent == MatchIntent::Troubleshooting
        && let Some(guide) = by_family(GuideFamily::AdvancedTroubleshooting).filter(not_excluded)
    {
        return Some(guide);
    }

    for &family in inferred_families {
        if let Some(guide) = by_family(family).filter(not_excluded) {
            return Some(guide);
        }
    }

    if intent == MatchIntent::Mesh
        && let Some(guide) = by_family(GuideFamily::GenericMesh).filter(not_excluded)
    {
        return Some(guide);
    }

    if let Some(guide) = by_family(GuideFamily::GenericRouter).filter(not_excluded) {
        return Some(guide);
    }

    catalog.iter().find(|g| not_excluded(g))
}

pub fn match_query<'a>(raw_query: &str, catalog: &'a [Guide]) -> MatchResult<'a> {
    let normalized_query = normalize(raw_query);
    let normalized_tokens = tokens_from_normalized(&normalized_query);
    let inferred_families = infer_families(&normalized_query, &normalized_tokens);
    let intent = inferred_intent(&normalized_query, &normalized_tokens, &inferred_families);

    let mut ranked_candidates: Vec<MatchCandidate<'a>> = catalog
        .iter()
        .map(|guide| {
            score_candidate(
                guide,
                &normalized_query,
                &normalized_tokens,
                &inferred_families,
                intent,
            )
        })
        .filter(|c| c.score > 0)
        .collect();

    ranked_candidates.sort_by(|lhs, rhs| {
        if lhs.score != rhs.score {
            rhs.score.cmp(&lhs.score)
        } else {
            lhs.guide
                .display_name
                .to_lowercase()
                .cmp(&rhs.guide.display_name.to_lowercase())
        }
    });

    let excluded_guide_id = ranked_candidates.first().map(|c| c.guide.id.as_str());
    let suggested_fallback_guide =
        suggested_fallback_guide(catalog, &inferred_families, intent, excluded_guide_id);

    let is_ambiguous = if ranked_candidates.len() >= 2 {
        let first = ranked_candidates[0].score;
        let second = ranked_candidates[1].score;
        first > 0 && (first - second) <= 12
    } else {
        false
    };

    let matched_direct_guide = ranked_candidates.first().is_some_and(|candidate| {
        inferred_families.contains(&candidate.guide.family)
            || candidate.reasons.contains(&"exact keyword")
    });

    MatchResult {
        normalized_query,
        normalized_tokens,
        inferred_families,
        candidates: ranked_candidates,
        suggested_fallback_guide,
        is_ambiguous,
        matched_direct_guide,
    }
}
