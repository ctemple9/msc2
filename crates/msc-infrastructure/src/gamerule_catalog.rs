//! Bundled release catalogs extracted from the actual server registrations.
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub default_value: String,
    pub choices: Vec<String>,
    pub experimental: bool,
    pub minimum: Option<i32>,
    pub maximum: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub server_type: String,
    pub minecraft_version: String,
    pub complete: bool,
    pub source: String,
    pub rules: Vec<Rule>,
}

pub fn catalog(edition: &str, version: Option<&str>) -> Option<&'static Catalog> {
    static CATALOGS: OnceLock<Vec<Catalog>> = OnceLock::new();
    let version = version?;
    // Fail closed on a malformed bundle; never substitute another release.
    CATALOGS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../data/gamerules/catalogs.json"))
                .unwrap_or_default()
        })
        .iter()
        .find(|catalog| catalog.server_type == edition && catalog.minecraft_version == version)
}

pub fn validate_values(
    edition: &str,
    version: Option<&str>,
    values: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let Some(catalog) = catalog(edition, version) else {
        return Ok(());
    };
    if edition == "bedrock" {
        let find = |id: &str| {
            values
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(id))
                .map(|(_, value)| value.as_str())
        };
        if let (Some(bar), Some(waypoints)) = (find("locatorbar"), find("playerwaypoints"))
            && ((bar == "true" && waypoints == "off")
                || (bar == "false" && waypoints == "everyone"))
        {
            return Err(
                "Locator Bar and Player Waypoints control the same setting; choose one override."
                    .into(),
            );
        }
    }
    for (name, value) in values {
        let rule = catalog.rules.iter().find(|rule| {
            if edition == "bedrock" {
                rule.id.eq_ignore_ascii_case(name)
            } else {
                rule.id == *name || rule.id.strip_prefix("minecraft:") == Some(name.as_str())
            }
        });
        // Extensions and old saved names remain preserved; the UI flags unlisted names.
        let Some(rule) = rule else {
            continue;
        };
        let valid = match rule.value_type.as_str() {
            "boolean" => value == "true" || value == "false",
            "integer" => value.parse::<i32>().is_ok_and(|value| {
                rule.minimum.is_none_or(|min| value >= min)
                    && rule.maximum.is_none_or(|max| value <= max)
            }),
            "choice" => rule.choices.contains(value),
            _ => false,
        };
        if !valid {
            return Err(format!(
                "Invalid value for {} on Minecraft {}",
                rule.id, catalog.minecraft_version
            ));
        }
    }
    Ok(())
}
