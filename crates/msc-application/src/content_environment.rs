//! A server platform is not always a client loader. In particular Paper-family
//! servers accept ordinary clients; independent client mods need an explicit
//! compatible player loader rather than a fictitious Paper client installation.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerSetup {
    pub loader: String,
    pub loader_version: Option<String>,
}
pub fn exact_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "latest" | "recommended" | "stable"
        )
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}
impl PlayerSetup {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.loader.as_str(),
            "vanilla" | "fabric" | "forge" | "neoforge" | "quilt"
        ) {
            return Err("Choose a supported player loader".into());
        }
        if self.loader == "vanilla" {
            if self.loader_version.is_some() {
                return Err("Vanilla player setup has no mod-loader version".into());
            }
        } else if !self.loader_version.as_deref().is_some_and(exact_version) {
            return Err("Choose an exact player loader version".into());
        }
        Ok(())
    }
}
pub fn infer(
    server_loader: &str,
    server_loader_version: Option<&str>,
    has_player_mods: bool,
) -> Option<PlayerSetup> {
    let setup = match server_loader {
        "fabric" | "forge" | "neoforge" | "quilt" => PlayerSetup {
            loader: server_loader.into(),
            loader_version: server_loader_version.map(str::to_string),
        },
        "vanilla" | "paper" | "purpur" | "pufferfish" | "spigot" if !has_player_mods => {
            PlayerSetup {
                loader: "vanilla".into(),
                loader_version: None,
            }
        }
        _ => return None,
    };
    setup.validate().ok().map(|()| setup)
}
pub fn for_publication(
    publication: &crate::content_publications::Publication,
) -> Option<PlayerSetup> {
    if let Some(setup) = &publication.player_setup {
        return setup.validate().ok().map(|()| setup.clone());
    }
    if publication.player_setup_reviewed {
        return None;
    }
    infer(
        &publication.loader,
        publication.loader_version.as_deref(),
        publication.player_files.values().any(|file| {
            file.path.starts_with("mods/") && file.enabled && !file.path.ends_with(".disabled")
        }),
    )
}

/// Readiness is derived from pinned metadata, including legacy snapshots whose
/// producer did not yet distinguish player setup from the server platform.
pub fn issues(
    publication: &crate::content_publications::Publication,
    audience: &str,
) -> Vec<crate::content_publications::PublicationIssue> {
    use crate::content_publications::PublicationIssue;
    let mut issues: Vec<_> = publication
        .issues
        .iter()
        .filter(|issue| issue.audience == audience || issue.audience == "all")
        .cloned()
        .collect();
    let mut add = |path: Option<String>, reason: &str| {
        if !issues
            .iter()
            .any(|issue| issue.path == path && issue.reason == reason)
        {
            issues.push(PublicationIssue {
                audience: audience.into(),
                path,
                required: true,
                reason: reason.into(),
            });
        }
    };
    if !publication
        .minecraft_version
        .as_deref()
        .is_some_and(exact_version)
    {
        add(None, "Exact Minecraft version is unknown or not pinned");
    }
    if audience == "players" {
        match for_publication(publication) {
            None => add(None, "Exact player loader/version needs review"),
            Some(setup)
                if setup.loader == "vanilla"
                    && publication.player_files.values().any(|file| {
                        file.path.starts_with("mods/")
                            && file.enabled
                            && !file.path.ends_with(".disabled")
                    }) =>
            {
                add(
                    None,
                    "Vanilla player setup cannot load the selected client mod JARs",
                )
            }
            _ => {}
        }
        for file in publication.player_files.values().filter(|file| {
            file.path.starts_with("mods/")
                && !file.optional
                && (!file.enabled || file.path.ends_with(".disabled"))
        }) {
            add(Some(file.path.clone()), "Required player mod is disabled");
        }
    }
    issues
}
