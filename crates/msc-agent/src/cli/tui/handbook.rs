//! Keyboard-first access to the agent-served Handbook and router guides.
//!
//! The agent remains the content authority. This module only keeps a bounded
//! selection/search state and renders the returned teaching material.

use std::collections::BTreeSet;

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandbookSurface {
    #[default]
    Topics,
    Topic,
    RouterSearch,
    RouterGuide,
    Troubleshooting,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpCatalogEntry {
    pub help_id: String,
    pub title: String,
    pub category: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpCatalog {
    pub topics: Vec<HelpCatalogEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HelpTopic {
    pub help_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub analogy: Option<String>,
    pub body: String,
    pub category: String,
    pub related_ids: Vec<String>,
    #[serde(default)]
    pub sections: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandbookIntent {
    LoadTopic(String),
    SearchRouter(String),
    OpenRouterGuide(String),
    AnalyzeTroubleshooting(Vec<String>),
}

#[derive(Debug, Clone, Default)]
pub struct HandbookState {
    pub surface: HandbookSurface,
    pub catalog: Option<HelpCatalog>,
    pub topic: Option<HelpTopic>,
    pub router_catalog: Option<Value>,
    pub router_search: Option<Value>,
    pub router_guide: Option<Value>,
    pub troubleshooting: Option<Value>,
    pub selected: usize,
    pub router_selected: usize,
    pub search_input: Option<String>,
    pub query: String,
    pub selected_symptoms: BTreeSet<String>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl HandbookState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let catalog = client.get_json("/v1/help/catalog").await?;
        let router_catalog = client.get_json("/v1/guides/router-catalog").await.ok();
        Ok(Self {
            catalog: Some(catalog),
            router_catalog,
            loaded: true,
            ..Self::default()
        })
    }

    pub fn filtered_topics(&self) -> Vec<&HelpCatalogEntry> {
        let query = self.query.trim().to_ascii_lowercase();
        self.catalog
            .as_ref()
            .map(|catalog| {
                catalog
                    .topics
                    .iter()
                    .filter(|topic| {
                        query.is_empty()
                            || topic.title.to_ascii_lowercase().contains(&query)
                            || topic.help_id.to_ascii_lowercase().contains(&query)
                            || topic.category.to_ascii_lowercase().contains(&query)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn router_candidates(&self) -> Vec<(String, String)> {
        self.router_search
            .as_ref()
            .and_then(|value| value.get("candidates"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|candidate| {
                let guide = candidate.get("guide")?;
                Some((
                    guide.get("id")?.as_str()?.to_string(),
                    guide.get("displayName")?.as_str()?.to_string(),
                ))
            })
            .collect()
    }

    pub fn symptoms(&self) -> Vec<(String, String)> {
        self.router_catalog
            .as_ref()
            .and_then(|value| value.get("symptoms"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|symptom| {
                Some((
                    symptom.get("id")?.as_str()?.to_string(),
                    symptom.get("title")?.as_str()?.to_string(),
                ))
            })
            .collect()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<HandbookIntent> {
        if let Some(mut value) = self.search_input.take() {
            return match key {
                KeyCode::Esc => None,
                KeyCode::Backspace => {
                    value.pop();
                    self.search_input = Some(value);
                    None
                }
                KeyCode::Char(character) => {
                    value.push(character);
                    self.search_input = Some(value);
                    None
                }
                KeyCode::Enter if !value.trim().is_empty() => {
                    if self.surface == HandbookSurface::RouterSearch {
                        Some(HandbookIntent::SearchRouter(value.trim().to_string()))
                    } else {
                        self.query = value;
                        self.selected = 0;
                        None
                    }
                }
                _ => {
                    self.search_input = Some(value);
                    None
                }
            };
        }

        match self.surface {
            HandbookSurface::Topics => match key {
                KeyCode::Char('s') | KeyCode::Char('/') => {
                    self.search_input = Some(self.query.clone())
                }
                KeyCode::Char('j') | KeyCode::Down => self.move_topic(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_topic(-1),
                KeyCode::Enter => {
                    return self
                        .filtered_topics()
                        .get(self.selected)
                        .map(|topic| HandbookIntent::LoadTopic(topic.help_id.clone()));
                }
                KeyCode::Char('r') => {
                    self.surface = HandbookSurface::RouterSearch;
                    self.router_selected = 0;
                }
                KeyCode::Char('t') => self.surface = HandbookSurface::Troubleshooting,
                KeyCode::Char('1') => {}
                _ => {}
            },
            HandbookSurface::Topic => match key {
                KeyCode::Char('b') | KeyCode::Esc => {
                    self.surface = HandbookSurface::Topics;
                    self.topic = None;
                }
                KeyCode::Char('j') | KeyCode::Down => self.move_related(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_related(-1),
                KeyCode::Enter => {
                    return self
                        .related_topics()
                        .get(self.selected)
                        .map(|id| HandbookIntent::LoadTopic(id.clone()));
                }
                _ => {}
            },
            HandbookSurface::RouterSearch => match key {
                KeyCode::Char('s') | KeyCode::Enter => self.search_input = Some(self.query.clone()),
                KeyCode::Char('j') | KeyCode::Down => self.move_router(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_router(-1),
                KeyCode::Char('o') => {
                    return self
                        .router_candidates()
                        .get(self.router_selected)
                        .map(|(id, _)| HandbookIntent::OpenRouterGuide(id.clone()));
                }
                KeyCode::Char('t') => self.surface = HandbookSurface::Troubleshooting,
                KeyCode::Esc => self.surface = HandbookSurface::Topics,
                _ => {}
            },
            HandbookSurface::RouterGuide => match key {
                KeyCode::Char('b') | KeyCode::Esc => self.surface = HandbookSurface::RouterSearch,
                KeyCode::Char('t') => self.surface = HandbookSurface::Troubleshooting,
                _ => {}
            },
            HandbookSurface::Troubleshooting => match key {
                KeyCode::Char('j') | KeyCode::Down => self.move_symptom(1),
                KeyCode::Char('k') | KeyCode::Up => self.move_symptom(-1),
                KeyCode::Char(' ') => self.toggle_selected_symptom(),
                KeyCode::Enter => {
                    let symptoms = self.selected_symptoms.iter().cloned().collect();
                    if !self.selected_symptoms.is_empty() {
                        return Some(HandbookIntent::AnalyzeTroubleshooting(symptoms));
                    }
                }
                KeyCode::Esc => self.surface = HandbookSurface::Topics,
                _ => {}
            },
        }
        None
    }

    pub async fn topic(client: &SharedClient, id: String) -> Result<HelpTopic, CliError> {
        client.get_json(&format!("/v1/help/{id}")).await
    }

    pub async fn search_router(client: &SharedClient, query: String) -> Result<Value, CliError> {
        client
            .get_json(&format!(
                "/v1/guides/router/search?q={}",
                percent_encode(&query)
            ))
            .await
    }

    pub async fn router_guide(client: &SharedClient, id: String) -> Result<Value, CliError> {
        client.get_json(&format!("/v1/guides/router/{id}")).await
    }

    pub async fn analyze_troubleshooting(
        client: &SharedClient,
        symptoms: Vec<String>,
    ) -> Result<Value, CliError> {
        client
            .post_json(
                "/v1/guides/router/troubleshooting/analyze",
                &serde_json::json!({ "symptoms": symptoms }),
            )
            .await
    }

    fn move_topic(&mut self, offset: isize) {
        let count = self.filtered_topics().len();
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn related_topics(&self) -> Vec<String> {
        self.topic
            .as_ref()
            .map(|topic| topic.related_ids.clone())
            .unwrap_or_default()
    }

    fn move_related(&mut self, offset: isize) {
        let count = self.related_topics().len();
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn move_router(&mut self, offset: isize) {
        let count = self.router_candidates().len();
        if count > 0 {
            self.router_selected =
                (self.router_selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn move_symptom(&mut self, offset: isize) {
        let count = self.symptoms().len();
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    fn toggle_selected_symptom(&mut self) {
        if let Some((id, _)) = self.symptoms().get(self.selected) {
            if !self.selected_symptoms.insert(id.clone()) {
                self.selected_symptoms.remove(id);
            }
        }
    }

    pub fn related_titles(&self) -> Vec<String> {
        let ids = self.related_topics();
        self.catalog
            .as_ref()
            .map(|catalog| {
                ids.iter()
                    .filter_map(|id| catalog.topics.iter().find(|topic| &topic.help_id == id))
                    .map(|topic| topic.title.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
