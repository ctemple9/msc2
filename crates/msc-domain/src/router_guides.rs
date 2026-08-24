//! The data half of MSC's router-help feature.
//!
//! `router::{matcher,fallback_tree,composer,runtime_resolver,troubleshooting}`
//! contains the executable decisions ported from MSC 1. This module owns the
//! embedded catalog that those decisions describe, keeping user-visible guide
//! copy separate from the rules that select and compose it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterGuide {
    pub id: String,
    pub family: String,
    pub category: String,
    pub display_name: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TroubleshootingTopic {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Deserialize)]
struct RouterCatalogFile {
    guides: Vec<RouterGuide>,
}

#[derive(Deserialize)]
struct TroubleshootingFile {
    topics: Vec<TroubleshootingTopic>,
}

/// Parses the checked-in router catalog compiled into the agent binary.
/// Parsing here makes malformed content a startup/test failure, never a
/// partially rendered guide in one client only.
pub fn embedded_catalog() -> Result<Vec<RouterGuide>, serde_json::Error> {
    serde_json::from_str::<RouterCatalogFile>(include_str!(
        "../../../content/guides/router-catalog.json"
    ))
    .map(|catalog| catalog.guides)
}

/// Parses the checked-in troubleshooting topic catalog compiled into the
/// agent binary. The matching and ranking rules remain in `router`.
pub fn embedded_troubleshooting_topics() -> Result<Vec<TroubleshootingTopic>, serde_json::Error> {
    serde_json::from_str::<TroubleshootingFile>(include_str!(
        "../../../content/guides/router-troubleshooting.json"
    ))
    .map(|catalog| catalog.topics)
}
