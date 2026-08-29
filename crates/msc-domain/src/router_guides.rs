//! The data half of MSC's router-help feature.
//!
//! `router::{matcher,fallback_tree,composer,runtime_resolver,troubleshooting}`
//! contains the executable decisions ported from MSC 1. This module owns the
//! complete, serializable seed records those decisions describe, plus the
//! checked conversion from source-shaped records into each engine's narrower
//! input model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuide {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub family: String,
    pub search_keywords: Vec<String>,
    pub admin_addresses: Vec<String>,
    pub admin_surface: String,
    pub menu_path: Vec<String>,
    pub alternate_menu_names: Vec<String>,
    pub steps: Vec<RouterGuideStep>,
    pub notes: Vec<RouterGuideNote>,
    pub troubleshooting: Vec<String>,
    pub shared_sections: RouterGuideSharedSections,
    pub review: RouterGuideReviewMetadata,
    pub provider_display_name: Option<String>,
    pub device_display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuideStep {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub referenced_tokens: Vec<String>,
    pub alternate_terms: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuideNote {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuideSharedSections {
    pub include_shared_intro: bool,
    pub include_shared_prerequisites: bool,
    pub include_shared_value_summary: bool,
    pub include_shared_troubleshooting_footer: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuideReviewMetadata {
    pub source_confidence: String,
    pub last_reviewed: Option<String>,
    pub review_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterTroubleshootingTopic {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub suggested_next_actions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RouterSymptom {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Deserialize)]
struct RouterCatalogFile {
    guides: Vec<RouterGuide>,
}

#[derive(Deserialize)]
struct TroubleshootingFile {
    topics: Vec<RouterTroubleshootingTopic>,
    symptoms: Vec<RouterSymptom>,
}

/// Parses the complete router catalog compiled into the agent binary.
/// Parsing here makes malformed content a startup/test failure, never a
/// partially rendered guide in one client only.
pub fn embedded_catalog() -> Result<Vec<RouterGuide>, serde_json::Error> {
    serde_json::from_str::<RouterCatalogFile>(include_str!(
        "../../../content/guides/router-catalog.json"
    ))
    .map(|catalog| catalog.guides)
}

/// Parses the complete troubleshooting topic catalog compiled into the agent
/// binary. The symptom checklist is returned separately because it is
/// client-owned reference copy, not input to the rule engine.
pub fn embedded_troubleshooting_topics()
-> Result<Vec<RouterTroubleshootingTopic>, serde_json::Error> {
    serde_json::from_str::<TroubleshootingFile>(include_str!(
        "../../../content/guides/router-troubleshooting.json"
    ))
    .map(|catalog| catalog.topics)
}

pub fn embedded_symptoms() -> Result<Vec<RouterSymptom>, serde_json::Error> {
    serde_json::from_str::<TroubleshootingFile>(include_str!(
        "../../../content/guides/router-troubleshooting.json"
    ))
    .map(|catalog| catalog.symptoms)
}

impl RouterGuide {
    pub fn to_matcher_guide(&self) -> Result<crate::router::matcher::Guide, String> {
        Ok(crate::router::matcher::Guide {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            category: parse_category(&self.category, &self.id)?,
            family: parse_family(&self.family, &self.id)?,
            search_keywords: self.search_keywords.clone(),
            admin_surface: parse_admin_surface(&self.admin_surface, &self.id)?,
            provider_display_name: self.provider_display_name.clone(),
            device_display_name: self.device_display_name.clone(),
        })
    }

    pub fn to_composer_guide(&self) -> Result<crate::router::composer::Guide, String> {
        use crate::router::composer::{
            Guide, GuideNote, GuideStep, ReviewMetadata, SharedSections, TroubleshootingTopicId,
        };

        let steps = self
            .steps
            .iter()
            .map(|step| {
                Ok(GuideStep {
                    id: step.id.clone(),
                    kind: parse_step_kind(&step.kind, &self.id, &step.id)?,
                    title: step.title.clone(),
                    body: step.body.clone(),
                    referenced_tokens: step
                        .referenced_tokens
                        .iter()
                        .map(|token| parse_token(token, &self.id, &step.id))
                        .collect::<Result<_, _>>()?,
                    alternate_terms: step.alternate_terms.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let notes = self
            .notes
            .iter()
            .map(|note| GuideNote {
                id: note.id.clone(),
                title: note.title.clone(),
                body: note.body.clone(),
            })
            .collect();
        let troubleshooting = self
            .troubleshooting
            .iter()
            .map(|id| {
                TroubleshootingTopicId::from_raw_value(id)
                    .ok_or_else(|| format!("{}: unknown troubleshooting topic '{id}'", self.id))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Guide {
            id: self.id.clone(),
            category: parse_category(&self.category, &self.id)?,
            admin_addresses: self.admin_addresses.clone(),
            admin_surface: parse_admin_surface(&self.admin_surface, &self.id)?,
            menu_path: self.menu_path.clone(),
            alternate_menu_names: self.alternate_menu_names.clone(),
            steps,
            notes,
            troubleshooting,
            shared_sections: SharedSections {
                include_shared_intro: self.shared_sections.include_shared_intro,
                include_shared_prerequisites: self.shared_sections.include_shared_prerequisites,
                include_shared_value_summary: self.shared_sections.include_shared_value_summary,
                include_shared_troubleshooting_footer: self
                    .shared_sections
                    .include_shared_troubleshooting_footer,
            },
            review: ReviewMetadata {
                source_confidence: parse_confidence(&self.review.source_confidence, &self.id)?,
            },
            provider_display_name: self.provider_display_name.clone(),
            device_display_name: self.device_display_name.clone(),
        })
    }
}

impl RouterTroubleshootingTopic {
    pub fn to_engine_topic(&self) -> Result<crate::router::composer::TroubleshootingTopic, String> {
        let id = crate::router::composer::TroubleshootingTopicId::from_raw_value(&self.id)
            .ok_or_else(|| format!("unknown troubleshooting topic '{}'", self.id))?;
        Ok(crate::router::composer::TroubleshootingTopic {
            id,
            title: self.title.clone(),
            summary: self.summary.clone(),
            suggested_next_actions: self.suggested_next_actions.clone(),
        })
    }
}

fn parse_category(
    raw: &str,
    guide_id: &str,
) -> Result<crate::router::matcher::GuideCategory, String> {
    crate::router::matcher::GuideCategory::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}: unknown guide category '{raw}'"))
}

fn parse_family(raw: &str, guide_id: &str) -> Result<crate::router::matcher::GuideFamily, String> {
    crate::router::matcher::GuideFamily::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}: unknown guide family '{raw}'"))
}

fn parse_admin_surface(
    raw: &str,
    guide_id: &str,
) -> Result<crate::router::matcher::AdminSurface, String> {
    crate::router::matcher::AdminSurface::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}: unknown admin surface '{raw}'"))
}

fn parse_step_kind(
    raw: &str,
    guide_id: &str,
    step_id: &str,
) -> Result<crate::router::composer::RouterGuideStepKind, String> {
    crate::router::composer::RouterGuideStepKind::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}/{step_id}: unknown step kind '{raw}'"))
}

fn parse_token(
    raw: &str,
    guide_id: &str,
    step_id: &str,
) -> Result<crate::router::composer::RouterGuideToken, String> {
    crate::router::composer::RouterGuideToken::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}/{step_id}: unknown guide token '{raw}'"))
}

fn parse_confidence(
    raw: &str,
    guide_id: &str,
) -> Result<crate::router::composer::RouterGuideConfidence, String> {
    crate::router::composer::RouterGuideConfidence::from_raw_value(raw)
        .ok_or_else(|| format!("{guide_id}: unknown review confidence '{raw}'"))
}
