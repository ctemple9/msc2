//! Content compiled into the agent so every MSC client reads one validated
//! handbook instead of carrying its own copy of the prose.

use std::collections::BTreeMap;

use include_dir::{Dir, DirEntry, include_dir};
use msc_domain::router_guides::{RouterGuide, TroubleshootingTopic};
use serde::{Deserialize, Serialize};

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../content");

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpTopic {
    pub help_id: String,
    pub title: String,
    pub analogy: Option<String>,
    pub body: String,
    pub category: String,
    pub related_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpCatalogEntry {
    pub help_id: String,
    pub title: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelpCatalog {
    pub topics: Vec<HelpCatalogEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct HelpContent {
    topics: BTreeMap<String, HelpTopic>,
}

#[derive(Deserialize)]
struct ConceptGuide {
    id: String,
    pages: serde_json::Value,
}

impl HelpContent {
    pub fn embedded() -> Result<Self, String> {
        let mut topics = BTreeMap::new();
        let help_dir = CONTENT
            .get_dir("help")
            .ok_or_else(|| "embedded content is missing help/".to_string())?;
        collect_markdown(help_dir, &mut topics)?;
        if topics.is_empty() {
            return Err("embedded help corpus contains no Markdown topics".to_string());
        }
        Ok(Self { topics })
    }

    pub fn topic(&self, help_id: &str) -> Option<&HelpTopic> {
        self.topics.get(help_id)
    }

    pub fn catalog(&self) -> HelpCatalog {
        HelpCatalog {
            topics: self
                .topics
                .values()
                .map(|topic| HelpCatalogEntry {
                    help_id: topic.help_id.clone(),
                    title: topic.title.clone(),
                    category: topic.category.clone(),
                })
                .collect(),
        }
    }

    pub fn concept_guide(&self) -> Result<serde_json::Value, String> {
        let text = file_text("guides/concept-guide.json")?;
        let guide: ConceptGuide = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if guide.id != "concept-guide" {
            return Err("concept guide has the wrong id".to_string());
        }
        Ok(serde_json::json!({ "id": guide.id, "pages": guide.pages }))
    }

    pub fn onboarding(&self) -> Result<serde_json::Value, String> {
        json_file("guides/onboarding.json")
    }

    pub fn router_catalog(&self) -> Result<serde_json::Value, String> {
        let guides: Vec<RouterGuide> =
            msc_domain::router_guides::embedded_catalog().map_err(|error| error.to_string())?;
        let troubleshooting: Vec<TroubleshootingTopic> =
            msc_domain::router_guides::embedded_troubleshooting_topics()
                .map_err(|error| error.to_string())?;
        Ok(serde_json::json!({ "guides": guides, "troubleshooting": troubleshooting }))
    }
}

fn collect_markdown(dir: &Dir<'_>, topics: &mut BTreeMap<String, HelpTopic>) -> Result<(), String> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(dir) => collect_markdown(dir, topics)?,
            DirEntry::File(file) if file.path().extension().is_some_and(|ext| ext == "md") => {
                let text = file
                    .contents_utf8()
                    .ok_or_else(|| format!("{} is not UTF-8", file.path().display()))?;
                let topic = parse_topic(text, file.path().display().to_string())?;
                if topics.insert(topic.help_id.clone(), topic).is_some() {
                    return Err(format!("duplicate help id in {}", file.path().display()));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_topic(text: &str, source: String) -> Result<HelpTopic, String> {
    let Some(rest) = text.strip_prefix("---\n") else {
        return Err(format!("{source}: missing YAML front matter"));
    };
    let Some((front_matter, body)) = rest.split_once("\n---\n") else {
        return Err(format!("{source}: unterminated YAML front matter"));
    };
    let fields = front_matter
        .lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(key, value)| (key, value.trim()))
        .collect::<BTreeMap<_, _>>();
    let required = |key: &str| {
        fields
            .get(key)
            .map(|value| (*value).to_string())
            .ok_or_else(|| format!("{source}: missing {key}"))
    };
    let related_ids = fields
        .get("relatedIds")
        .map(|value| {
            value
                .trim_matches(['[', ']'])
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(HelpTopic {
        help_id: required("id")?,
        title: required("title")?,
        analogy: fields.get("analogy").map(|value| (*value).to_string()),
        body: body.trim().to_string(),
        category: required("category")?,
        related_ids,
    })
}

fn file_text(path: &str) -> Result<&'static str, String> {
    CONTENT
        .get_file(path)
        .and_then(|file| file.contents_utf8())
        .ok_or_else(|| format!("embedded content missing {path}"))
}

fn json_file(path: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(file_text(path)?).map_err(|error| format!("{path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_corpus_contains_the_bedrock_unavailable_explanation() {
        let content = HelpContent::embedded().unwrap();
        let topic = content.topic("bedrock.runtime-unavailable").unwrap();
        assert!(topic.body.contains("reason code"));
        assert!(content.catalog().topics.len() >= 60);
    }
}
