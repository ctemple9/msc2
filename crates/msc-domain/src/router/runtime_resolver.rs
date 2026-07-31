//! Port of `RouterPortForwardGuideRuntimeResolver.swift` (291 lines) — a
//! "runtime value injection engine for composed router port-forwarding
//! guides. Resolves dynamic tokens against the selected server's live
//! context" (the file's own doc comment).
//!
//! Two independent pieces, both already adjudicated agent-owned in the
//! symbol ledger (`msc2-symbol-ledger.csv`, two rows for this file:
//! `makeRecommendedProtocol`, and the `resolve`/`resolveGuide`/`resolveBestMatch`/
//! `resolveItem`/`resolveText` family — the latter corrected to agent during
//! Codex's P0.27 review):
//!
//! - `RouterPortForwardGuideRuntimeContext.makeRecommendedProtocol` (ported
//!   as [`make_recommended_protocol`]) — a pure rule (TCP always; UDP only
//!   when Bedrock is enabled *and* a Bedrock port is known) with no
//!   dependency on a composed guide at all.
//! - `resolve`/`resolveItem`/`resolveText` (ported as [`resolve`] and its
//!   private helpers) — walks a [`ComposedGuide`]'s sections and substitutes
//!   `{{raw_value}}` placeholders in `paragraph`/`bulletList`/`step` bodies
//!   against [`RuntimeContext`], recording any token whose placeholder
//!   couldn't be resolved. `menuPath`/`note`/`troubleshootingTopic` items
//!   pass through unchanged — `resolveItem` never calls `resolveText` on
//!   them, confirmed empirically via the characterization harness (see
//!   `menu-path-and-note-and-troubleshooting-topic-items-pass-through-unchanged`).
//!
//! **`resolveGuide(id:...)` and `resolveBestMatch(for:...)` are not
//! ported.** Both are one-line chains of an already-tested lookup
//! (`compose_guide_by_id` from P1.12, or the matcher's `best_match` from
//! P1.10) into this file's own `resolve` — the same "tests only that they
//! compose, not new logic of this file's own" reasoning `composer.rs`
//! recorded for excluding `composeBestMatch`. `RouterPortForwardGuideRuntimeContext`'s
//! `@MainActor extension AppViewModel` (the host glue that reads a live
//! server's actual IP/port/Bedrock state into a context) is host-owned I/O,
//! not domain logic, and stays out of this crate entirely.
//!
//! **Characterized by running the real source, not hand-derived** — a
//! harness combining this file, `RouterPortForwardGuideComposer.swift`,
//! `RouterPortForwardGuideRepository.swift`, `RouterPortForwardGuideCatalogLoader.swift`,
//! and `RouterPortForwardGuidesFoundation.swift` verbatim (its `AppViewModel`
//! extension stripped, the only edit) against `fixtures/router-sample-catalog.json`,
//! run through `swift` and dumping JSON via `JSONSerialization`, the same
//! methodology P1.13 introduced. One finding worth calling out: a step's
//! `referencedTokens` metadata (e.g. `xfinity-step-4`, which declares
//! `[.javaPort, .bedrockPort, .bedrockEnabled]`) does **not** by itself
//! cause substitution — `resolveText` only acts on a token whose literal
//! `{{raw_value}}` placeholder is actually present in the body text, and
//! `xfinity-step-4`'s body contains no placeholder at all despite the
//! metadata, so it passes through unchanged and contributes no
//! `unresolvedTokens` entry. One synthetic fixture (a hand-built
//! `ComposedSection` with a body repeating the same placeholder twice, not
//! literal MSC 1 content) pins that a repeated placeholder is replaced at
//! every occurrence, the same all-occurrences semantics as Swift's
//! `replacingOccurrences(of:with:)`.

use std::collections::HashSet;

use crate::router::composer::{ComposedGuide, GuideNote, RouterGuideStepKind, SectionItem};
pub use crate::router::composer::{RouterGuideToken, SectionKind};

const ALL_TOKENS: [RouterGuideToken; 7] = [
    RouterGuideToken::SelectedServerName,
    RouterGuideToken::DetectedLocalIpAddress,
    RouterGuideToken::DetectedGatewayIpAddress,
    RouterGuideToken::JavaPort,
    RouterGuideToken::BedrockPort,
    RouterGuideToken::RecommendedProtocol,
    RouterGuideToken::BedrockEnabled,
];

#[derive(Debug, Clone, Default)]
pub struct RuntimeContext {
    pub selected_server_id: Option<String>,
    pub selected_server_name: Option<String>,
    pub detected_local_ip_address: Option<String>,
    pub detected_gateway_ip_address: Option<String>,
    pub java_port: Option<i32>,
    pub bedrock_port: Option<i32>,
    pub recommended_protocol: Option<String>,
    pub bedrock_enabled: Option<bool>,
}

impl RuntimeContext {
    pub fn resolved_string(&self, token: RouterGuideToken) -> Option<String> {
        match token {
            RouterGuideToken::SelectedServerName => cleaned(self.selected_server_name.as_deref()),
            RouterGuideToken::DetectedLocalIpAddress => {
                cleaned(self.detected_local_ip_address.as_deref())
            }
            RouterGuideToken::DetectedGatewayIpAddress => {
                cleaned(self.detected_gateway_ip_address.as_deref())
            }
            RouterGuideToken::JavaPort => self.java_port.map(|p| p.to_string()),
            RouterGuideToken::BedrockPort => self.bedrock_port.map(|p| p.to_string()),
            RouterGuideToken::RecommendedProtocol => cleaned(self.recommended_protocol.as_deref()),
            RouterGuideToken::BedrockEnabled => self
                .bedrock_enabled
                .map(|enabled| if enabled { "Yes" } else { "No" }.to_string()),
        }
    }

    pub fn fallback_string(token: RouterGuideToken) -> &'static str {
        match token {
            RouterGuideToken::SelectedServerName => "Current Server",
            RouterGuideToken::DetectedLocalIpAddress => "Unavailable on this Mac right now",
            RouterGuideToken::DetectedGatewayIpAddress => {
                "Look up your router or gateway address manually"
            }
            RouterGuideToken::JavaPort => "Unknown",
            RouterGuideToken::BedrockPort => "Not enabled",
            RouterGuideToken::RecommendedProtocol => {
                "Forward TCP for Java. Add UDP only when Bedrock or Geyser is enabled."
            }
            RouterGuideToken::BedrockEnabled => "Unknown",
        }
    }
}

fn cleaned(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// TCP always; UDP only when Bedrock is enabled *and* a Bedrock port is
/// known — `bedrockEnabled` alone, without a port, falls through to the
/// Java-only (or fully-generic) branches below.
pub fn make_recommended_protocol(
    java_port: Option<i32>,
    bedrock_port: Option<i32>,
    bedrock_enabled: bool,
) -> String {
    if bedrock_enabled && let Some(bedrock_port) = bedrock_port {
        return match java_port {
            Some(java_port) => format!(
                "Forward TCP for the Java server on port {java_port}. Also forward UDP for Bedrock or Geyser on port {bedrock_port}."
            ),
            None => format!("Forward UDP for Bedrock or Geyser on port {bedrock_port}."),
        };
    }

    if let Some(java_port) = java_port {
        return format!("Forward TCP for the Java server on port {java_port}.");
    }

    "Forward TCP for Java. Add UDP only when Bedrock or Geyser is enabled.".to_string()
}

#[derive(Debug, Clone)]
pub enum ResolvedItem {
    Paragraph {
        title: Option<String>,
        body: String,
    },
    BulletList {
        title: Option<String>,
        bullets: Vec<String>,
    },
    MenuPath {
        title: Option<String>,
        path: Vec<String>,
        alternate_menu_names: Vec<String>,
    },
    Step(ResolvedStep),
    Note(GuideNote),
    TroubleshootingTopic(crate::router::composer::TroubleshootingTopic),
}

#[derive(Debug, Clone)]
pub struct ResolvedStep {
    pub id: String,
    pub kind: RouterGuideStepKind,
    pub title: String,
    pub body: String,
    pub alternate_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnresolvedToken {
    pub section_id: String,
    pub token: RouterGuideToken,
}

#[derive(Debug, Clone)]
pub struct ResolvedSection {
    pub id: String,
    pub kind: SectionKind,
    pub title: &'static str,
    pub items: Vec<ResolvedItem>,
    pub origin: crate::router::composer::SectionOrigin,
}

#[derive(Debug, Clone)]
pub struct ResolvedGuide<'a> {
    pub base_guide: ComposedGuide<'a>,
    pub sections: Vec<ResolvedSection>,
    pub unresolved_tokens: Vec<UnresolvedToken>,
}

impl<'a> ResolvedGuide<'a> {
    pub fn id(&self) -> &str {
        self.base_guide.id()
    }
}

pub fn resolve<'a>(
    composed_guide: ComposedGuide<'a>,
    context: &RuntimeContext,
) -> ResolvedGuide<'a> {
    let mut unresolved = HashSet::new();

    let sections: Vec<ResolvedSection> = composed_guide
        .sections
        .iter()
        .map(|section| ResolvedSection {
            id: section.id.clone(),
            kind: section.kind,
            title: section.title,
            items: section
                .items
                .iter()
                .map(|item| resolve_item(item, context, &section.id, &mut unresolved))
                .collect(),
            origin: section.origin,
        })
        .collect();

    let mut unresolved_tokens: Vec<UnresolvedToken> = unresolved.into_iter().collect();
    unresolved_tokens.sort_by(|a, b| {
        if a.section_id != b.section_id {
            return a.section_id.cmp(&b.section_id);
        }
        a.token.raw_value().cmp(b.token.raw_value())
    });

    ResolvedGuide {
        base_guide: composed_guide,
        sections,
        unresolved_tokens,
    }
}

fn resolve_item(
    item: &SectionItem,
    context: &RuntimeContext,
    section_id: &str,
    unresolved: &mut HashSet<UnresolvedToken>,
) -> ResolvedItem {
    match item {
        SectionItem::Paragraph { title, body, .. } => ResolvedItem::Paragraph {
            title: title.clone(),
            body: resolve_text(body, context, section_id, unresolved),
        },
        SectionItem::BulletList { title, bullets, .. } => ResolvedItem::BulletList {
            title: title.clone(),
            bullets: bullets
                .iter()
                .map(|bullet| resolve_text(bullet, context, section_id, unresolved))
                .collect(),
        },
        SectionItem::MenuPath {
            title,
            path,
            alternate_menu_names,
        } => ResolvedItem::MenuPath {
            title: title.clone(),
            path: path.clone(),
            alternate_menu_names: alternate_menu_names.clone(),
        },
        SectionItem::Step(step) => ResolvedItem::Step(ResolvedStep {
            id: step.id.clone(),
            kind: step.kind,
            title: step.title.clone(),
            body: resolve_text(&step.body, context, section_id, unresolved),
            alternate_terms: step.alternate_terms.clone(),
        }),
        SectionItem::Note(note) => ResolvedItem::Note(note.clone()),
        SectionItem::TroubleshootingTopic(topic) => {
            ResolvedItem::TroubleshootingTopic(topic.clone())
        }
    }
}

fn resolve_text(
    text: &str,
    context: &RuntimeContext,
    section_id: &str,
    unresolved: &mut HashSet<UnresolvedToken>,
) -> String {
    let mut resolved = text.to_string();

    for token in ALL_TOKENS {
        let placeholder = format!("{{{{{}}}}}", token.raw_value());
        if !resolved.contains(&placeholder) {
            continue;
        }

        if let Some(replacement) = context.resolved_string(token) {
            resolved = resolved.replace(&placeholder, &replacement);
        } else {
            unresolved.insert(UnresolvedToken {
                section_id: section_id.to_string(),
                token,
            });
            resolved = resolved.replace(&placeholder, RuntimeContext::fallback_string(token));
        }
    }

    resolved
}
