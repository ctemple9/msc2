//! Content compiled into the agent so every MSC client reads one validated
//! handbook instead of carrying its own copy of the prose.

use std::collections::BTreeMap;
use std::net::UdpSocket;
use std::path::Path;
use std::sync::Arc;

use include_dir::{Dir, DirEntry, include_dir};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::ServerType;
use msc_domain::router::runtime_resolver::RuntimeContext;
use msc_domain::router_guides::{RouterGuide, RouterSymptom, RouterTroubleshootingTopic};
use serde::Serialize;

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../content");

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpTopic {
    pub help_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub analogy: Option<String>,
    pub body: String,
    pub category: String,
    pub related_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<HelpContentBlock>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HelpContentBlock {
    Body {
        markdown: String,
    },
    BulletList {
        items: Vec<String>,
    },
    Callout {
        style: CalloutStyle,
        text: String,
    },
    InApp {
        items: Vec<String>,
    },
    Advanced {
        markdown: String,
    },
    Checklist {
        phase: String,
        steps: Vec<ChecklistStepBlock>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CalloutStyle {
    Tip,
    Warning,
    Pitfall,
    Note,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistStepBlock {
    pub number: u32,
    pub title: String,
    pub detail: String,
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

pub type RouterRuntimeProvider = Arc<dyn Fn() -> Option<RuntimeContext> + Send + Sync>;

#[derive(Clone, Default)]
pub struct HelpContent {
    topics: BTreeMap<String, HelpTopic>,
    router_runtime_provider: Option<RouterRuntimeProvider>,
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
        Ok(Self {
            topics,
            router_runtime_provider: None,
        })
    }

    pub fn with_router_runtime_provider(mut self, provider: RouterRuntimeProvider) -> Self {
        self.router_runtime_provider = Some(provider);
        self
    }

    pub fn router_runtime_context(&self) -> Option<RuntimeContext> {
        self.router_runtime_provider
            .as_ref()
            .and_then(|provider| provider())
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

    pub fn onboarding(&self) -> Result<serde_json::Value, String> {
        json_file("guides/onboarding.json")
    }

    pub fn router_catalog(&self) -> Result<serde_json::Value, String> {
        let guides: Vec<RouterGuide> =
            msc_domain::router_guides::embedded_catalog().map_err(|error| error.to_string())?;
        let troubleshooting: Vec<RouterTroubleshootingTopic> =
            msc_domain::router_guides::embedded_troubleshooting_topics()
                .map_err(|error| error.to_string())?;
        let symptoms: Vec<RouterSymptom> =
            msc_domain::router_guides::embedded_symptoms().map_err(|error| error.to_string())?;
        Ok(serde_json::json!({
            "guides": guides,
            "troubleshooting": troubleshooting,
            "symptoms": symptoms,
        }))
    }
}

/// Builds the runtime values used by router-guide placeholders from the
/// currently selected MSC server. Local-IP discovery is best effort because a
/// guide must remain useful on an offline host; the resolver supplies its
/// documented fallback text when discovery is unavailable.
#[allow(dead_code)]
pub fn router_runtime_context_for_server(server: &ConfigServer) -> RuntimeContext {
    let java_port = (server.server_type == ServerType::Java).then(|| {
        read_server_port(
            Path::new(&server.server_dir).join("server.properties"),
            25565,
        )
    });
    let bedrock_port = server
        .bedrock_port
        .map(|port| port as i32)
        .or_else(|| (server.server_type == ServerType::Bedrock).then_some(19132));
    let bedrock_enabled = server.server_type == ServerType::Bedrock
        || server.bedrock_enabled
        || bedrock_port.is_some();
    RuntimeContext {
        selected_server_id: Some(server.id.clone()),
        selected_server_name: Some(server.display_name.clone()),
        detected_local_ip_address: detect_local_ip(),
        detected_gateway_ip_address: None,
        java_port,
        bedrock_port,
        recommended_protocol: Some(
            msc_domain::router::runtime_resolver::make_recommended_protocol(
                java_port,
                bedrock_port,
                bedrock_enabled,
            ),
        ),
        bedrock_enabled: Some(bedrock_enabled),
    }
}

#[allow(dead_code)]
fn read_server_port(path: std::path::PathBuf, default: i32) -> i32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                line.strip_prefix("server-port=")
                    .and_then(|port| port.trim().parse::<i32>().ok())
            })
        })
        .unwrap_or(default)
}

/// Resolve the LAN address the host would use for an outbound connection.
///
/// UDP connect does not send application data here; it asks the operating
/// system which local interface and address would route to the destination.
pub(crate) fn detect_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback()).then(|| ip.to_string())
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
    // Git checkouts on Windows may materialize embedded Markdown with CRLF
    // endings. Normalize before parsing so valid front matter is portable.
    let normalized = text.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(format!("{source}: missing YAML front matter"));
    };
    let Some((front_matter, body)) = rest.split_once("\n---\n") else {
        return Err(format!("{source}: unterminated YAML front matter"));
    };
    let fields = front_matter
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| format!("{source}: malformed front-matter line '{line}'"))?;
            Ok((
                key.trim().to_string(),
                parse_front_matter_value(value.trim(), &source)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let required = |key: &str| {
        fields
            .get(key)
            .cloned()
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
    let (body, sections) = parse_content_blocks(body, &source)?;
    Ok(HelpTopic {
        help_id: required("id")?,
        title: required("title")?,
        subtitle: fields.get("subtitle").cloned(),
        analogy: fields.get("analogy").cloned(),
        body,
        category: required("category")?,
        related_ids,
        sections,
    })
}

fn parse_front_matter_value(value: &str, source: &str) -> Result<String, String> {
    if value.starts_with('"') {
        serde_json::from_str(value)
            .map_err(|error| format!("{source}: invalid quoted front-matter value: {error}"))
    } else {
        Ok(value.to_string())
    }
}

fn parse_content_blocks(
    body: &str,
    source: &str,
) -> Result<(String, Vec<HelpContentBlock>), String> {
    if !body.lines().any(|line| line.starts_with("### ")) {
        return Ok((body.trim().to_string(), Vec::new()));
    }

    let mut preamble = Vec::new();
    let mut blocks: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut current_marker: Option<&str> = None;
    let mut current_lines = Vec::new();

    for line in body.lines() {
        if let Some(marker) = line.strip_prefix("### ") {
            if let Some(previous_marker) = current_marker.take() {
                blocks.push((previous_marker, current_lines));
                current_lines = Vec::new();
            }
            current_marker = Some(marker.trim());
        } else if current_marker.is_some() {
            current_lines.push(line);
        } else {
            preamble.push(line);
        }
    }
    if let Some(marker) = current_marker {
        blocks.push((marker, current_lines));
    }

    let sections = blocks
        .into_iter()
        .map(|(marker, lines)| parse_content_block(marker, &lines, source))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((preamble.join("\n").trim().to_string(), sections))
}

fn parse_content_block(
    marker: &str,
    lines: &[&str],
    source: &str,
) -> Result<HelpContentBlock, String> {
    let text = lines.join("\n").trim().to_string();
    match marker {
        "Body" => non_empty_block(HelpContentBlock::Body { markdown: text }, marker, source),
        "Bullet List" => Ok(HelpContentBlock::BulletList {
            items: parse_markdown_list(lines, marker, source)?,
        }),
        "In This App" => Ok(HelpContentBlock::InApp {
            items: parse_markdown_list(lines, marker, source)?,
        }),
        "Advanced Details" => non_empty_block(
            HelpContentBlock::Advanced { markdown: text },
            marker,
            source,
        ),
        "Table" => Ok(parse_table(lines, source)?),
        marker if marker.starts_with("Callout: ") => {
            let style = parse_callout_style(&marker[9..], source)?;
            non_empty_block(HelpContentBlock::Callout { style, text }, marker, source)
        }
        marker if marker.starts_with("Checklist: ") => Ok(HelpContentBlock::Checklist {
            phase: marker[11..].trim().to_string(),
            steps: parse_checklist(lines, marker, source)?,
        }),
        _ => Err(format!(
            "{source}: unrecognized content heading '### {marker}'"
        )),
    }
}

fn non_empty_block(
    block: HelpContentBlock,
    marker: &str,
    source: &str,
) -> Result<HelpContentBlock, String> {
    let empty = match &block {
        HelpContentBlock::Body { markdown } | HelpContentBlock::Advanced { markdown } => {
            markdown.is_empty()
        }
        HelpContentBlock::Callout { text, .. } => text.is_empty(),
        _ => false,
    };
    if empty {
        Err(format!(
            "{source}: content heading '### {marker}' has no content"
        ))
    } else {
        Ok(block)
    }
}

fn parse_markdown_list(lines: &[&str], marker: &str, source: &str) -> Result<Vec<String>, String> {
    let items = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("- ")
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{source}: '### {marker}' contains a non-list line"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if items.is_empty() {
        return Err(format!(
            "{source}: list heading '### {marker}' has no items"
        ));
    }
    Ok(items)
}

fn parse_callout_style(value: &str, source: &str) -> Result<CalloutStyle, String> {
    match value.trim() {
        "tip" => Ok(CalloutStyle::Tip),
        "warning" => Ok(CalloutStyle::Warning),
        "pitfall" => Ok(CalloutStyle::Pitfall),
        "note" => Ok(CalloutStyle::Note),
        style => Err(format!("{source}: unknown callout style '{style}'")),
    }
}

fn parse_checklist(
    lines: &[&str],
    marker: &str,
    source: &str,
) -> Result<Vec<ChecklistStepBlock>, String> {
    let mut steps = Vec::new();
    for line in lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
    {
        let Some((number, rest)) = line.split_once(". **") else {
            return Err(format!(
                "{source}: malformed checklist line in '### {marker}'"
            ));
        };
        let number = number
            .parse::<u32>()
            .map_err(|_| format!("{source}: invalid checklist number '{number}'"))?;
        let Some((title, detail)) = rest.split_once("** — ") else {
            return Err(format!(
                "{source}: malformed checklist line in '### {marker}'"
            ));
        };
        if title.is_empty() || detail.trim().is_empty() {
            return Err(format!(
                "{source}: incomplete checklist line in '### {marker}'"
            ));
        }
        steps.push(ChecklistStepBlock {
            number,
            title: title.to_string(),
            detail: detail.trim().to_string(),
        });
    }
    if steps.is_empty() {
        return Err(format!(
            "{source}: checklist heading '### {marker}' has no steps"
        ));
    }
    Ok(steps)
}

fn parse_table(lines: &[&str], source: &str) -> Result<HelpContentBlock, String> {
    let rows = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if rows.len() < 3 {
        return Err(format!("{source}: table needs a header, divider, and row"));
    }
    let headers = parse_pipe_row(rows[0], source)?;
    let divider = parse_pipe_row(rows[1], source)?;
    if divider.len() != headers.len()
        || divider
            .iter()
            .any(|cell| cell.is_empty() || cell.chars().any(|character| character != '-'))
    {
        return Err(format!("{source}: malformed table divider"));
    }
    let data = rows[2..]
        .iter()
        .map(|row| {
            let values = parse_pipe_row(row, source)?;
            if values.len() != headers.len() {
                return Err(format!("{source}: table row has the wrong number of cells"));
            }
            Ok(values)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(HelpContentBlock::Table {
        headers,
        rows: data,
    })
}

fn parse_pipe_row(line: &str, source: &str) -> Result<Vec<String>, String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Err(format!("{source}: malformed table row"));
    }
    Ok(trimmed[1..trimmed.len() - 1]
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect())
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

    #[test]
    fn parses_every_content_block_variant() {
        let topic = parse_topic(
            "---\nid: handbook.fixture\ntitle: Fixture\ncategory: test\nsubtitle: \"A subtitle: with punctuation\"\nanalogy: \"An analogy\"\nrelatedIds: []\n---\nPreamble.\n\n### Body\nA second paragraph.\n\n### Bullet List\n- First\n- Second\n\n### Callout: warning\nBe careful.\n\n### In This App\n- One setting\n\n### Advanced Details\nMore detail.\n\n### Checklist: Phase 1 — Setup\n1. **Install Java** — Use Java 21.\n\n### Table\n| Players | Min RAM | Max RAM |\n|---|---|---|\n| 1–2 players | 2 GB | 4 GB |\n",
            "inline fixture".into(),
        )
        .unwrap();

        assert_eq!(
            topic.subtitle.as_deref(),
            Some("A subtitle: with punctuation")
        );
        assert_eq!(topic.body, "Preamble.");
        assert!(matches!(
            &topic.sections[0],
            HelpContentBlock::Body { markdown } if markdown == "A second paragraph."
        ));
        assert!(matches!(
            &topic.sections[1],
            HelpContentBlock::BulletList { items } if items == &["First", "Second"]
        ));
        assert!(matches!(
            &topic.sections[2],
            HelpContentBlock::Callout { style: CalloutStyle::Warning, text } if text == "Be careful."
        ));
        assert!(matches!(
            &topic.sections[3],
            HelpContentBlock::InApp { items } if items == &["One setting"]
        ));
        assert!(matches!(
            &topic.sections[4],
            HelpContentBlock::Advanced { markdown } if markdown == "More detail."
        ));
        assert!(matches!(
            &topic.sections[5],
            HelpContentBlock::Checklist { phase, steps }
                if phase == "Phase 1 — Setup"
                    && steps == &[ChecklistStepBlock {
                        number: 1,
                        title: "Install Java".into(),
                        detail: "Use Java 21.".into(),
                    }]
        ));
        assert!(matches!(
            &topic.sections[6],
            HelpContentBlock::Table { headers, rows }
                if headers == &["Players", "Min RAM", "Max RAM"]
                    && rows == &[["1–2 players", "2 GB", "4 GB"].map(String::from).to_vec()]
        ));
    }

    #[test]
    fn rejects_an_unrecognized_content_heading() {
        let error = parse_topic(
            "---\nid: handbook.fixture\ntitle: Fixture\ncategory: test\n---\n### Unknown\nText.",
            "inline fixture".into(),
        )
        .unwrap_err();
        assert!(error.contains("unrecognized content heading"));
    }

    #[test]
    fn every_handbook_topic_has_rich_content_and_flat_topics_stay_compatible() {
        let content = HelpContent::embedded().unwrap();
        let handbook = content
            .topics
            .values()
            .filter(|topic| topic.help_id.starts_with("handbook."))
            .collect::<Vec<_>>();
        assert_eq!(handbook.len(), 31);
        assert!(
            handbook
                .iter()
                .all(|topic| topic.subtitle.is_some() && !topic.sections.is_empty())
        );

        let flat = content.topic("bedrock.runtime-unavailable").unwrap();
        assert!(flat.subtitle.is_none());
        assert!(flat.sections.is_empty());
    }
}
