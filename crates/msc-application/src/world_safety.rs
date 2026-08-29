//! Shared safety rules for changes that can alter Minecraft world safety.
//!
//! The agent owns the confirmation boundary, so every client gets the same
//! explanation and the same acknowledgement token. A token is deliberately
//! returned by the API instead of making clients guess a phrase: the user
//! still has to take a second, explicit action, while scripts can handle the
//! contract without matching prose.

use msc_domain::identity::ServerType;
use msc_domain::world_profile::WorldProfile;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationKind {
    BedrockAchievements,
    JavaCreative,
    JavaCommands,
    ServerForceGamemode,
}

impl ConfirmationKind {
    pub const fn token(self) -> &'static str {
        match self {
            Self::BedrockAchievements => "bedrock_achievements",
            Self::JavaCreative => "java_creative",
            Self::JavaCommands => "java_commands",
            Self::ServerForceGamemode => "server_force_gamemode",
        }
    }

    pub const fn scope(self) -> &'static str {
        match self {
            Self::ServerForceGamemode => "server",
            Self::BedrockAchievements | Self::JavaCreative | Self::JavaCommands => "world",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::BedrockAchievements => "Bedrock achievements warning",
            Self::JavaCreative => "Java Creative mode confirmation",
            Self::JavaCommands => "Java commands confirmation",
            Self::ServerForceGamemode => "Server-wide gamemode override",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::BedrockAchievements => {
                "Bedrock Creative mode or cheats can permanently disable Xbox achievements for this world, even if cheats are later turned off."
            }
            Self::JavaCreative => {
                "Java Creative mode changes the world's advancement and command semantics. Java does not have Bedrock's permanent Xbox-achievement consequence."
            }
            Self::JavaCommands => {
                "Enabling Java commands changes the world's command and advancement semantics. Java does not have Bedrock's permanent Xbox-achievement consequence."
            }
            Self::ServerForceGamemode => {
                "This server-wide override applies to every world and can supersede each slot's saved default gamemode. It is separate from a world's saved Creative setting."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyConfirmation {
    pub kind: ConfirmationKind,
}

impl SafetyConfirmation {
    pub const fn token(self) -> &'static str {
        self.kind.token()
    }

    pub const fn message(self) -> &'static str {
        self.kind.message()
    }

    pub fn details(self) -> Value {
        json!({
            "confirmation": {
                "kind": self.kind.token(),
                "scope": self.kind.scope(),
                "title": self.kind.title(),
                "message": self.kind.message(),
                "acknowledgement": self.kind.token(),
            }
        })
    }
}

pub fn is_confirmed(required: SafetyConfirmation, supplied: Option<&str>) -> bool {
    supplied.is_some_and(|value| value.trim() == required.token())
}

pub fn confirmation_for_world_profile(
    server_type: ServerType,
    profile: &WorldProfile,
) -> Option<SafetyConfirmation> {
    confirmation_for_world_values(
        server_type,
        profile.gameplay.default_game_mode.as_deref(),
        profile.gameplay.cheats,
        profile.gameplay.commands,
    )
}

pub fn confirmation_for_profile_changes(
    server_type: ServerType,
    changes: &BTreeMap<String, Value>,
) -> Option<SafetyConfirmation> {
    for (key, value) in changes {
        let confirmation = match key.as_str() {
            "gameplay.default-game-mode"
                if value
                    .as_str()
                    .is_some_and(|mode| mode.eq_ignore_ascii_case("creative")) =>
            {
                confirmation_for_world_values(server_type, Some("creative"), None, None)
            }
            "gameplay.cheats" if value.as_bool() == Some(true) => {
                (server_type == ServerType::Bedrock).then_some(SafetyConfirmation {
                    kind: ConfirmationKind::BedrockAchievements,
                })
            }
            "gameplay.commands" if value.as_bool() == Some(true) => {
                (server_type == ServerType::Java).then_some(SafetyConfirmation {
                    kind: ConfirmationKind::JavaCommands,
                })
            }
            _ => None,
        };
        if confirmation.is_some() {
            return confirmation;
        }
    }
    None
}

pub fn confirmation_for_command(
    server_type: ServerType,
    command: &str,
) -> Option<SafetyConfirmation> {
    let tokens: Vec<String> = command
        .trim()
        .trim_start_matches('/')
        .split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect();
    let changes_gamemode = tokens.windows(2).any(|window| {
        matches!(window[0].as_str(), "gamemode" | "defaultgamemode")
            && matches!(window[1].as_str(), "creative" | "1")
    });
    if !changes_gamemode {
        return None;
    }
    confirmation_for_world_values(server_type, Some("creative"), None, None)
}

pub fn confirmation_for_server_setting(key: &str, value: &str) -> Option<SafetyConfirmation> {
    let enabled = matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "on" | "yes"
    );
    (key == "force-gamemode" && enabled).then_some(SafetyConfirmation {
        kind: ConfirmationKind::ServerForceGamemode,
    })
}

pub fn confirmation_for_server_creation(
    server_type: ServerType,
    gamemode: Option<&str>,
) -> Option<SafetyConfirmation> {
    confirmation_for_world_values(server_type, gamemode, None, None)
}

fn confirmation_for_world_values(
    server_type: ServerType,
    gamemode: Option<&str>,
    cheats: Option<bool>,
    commands: Option<bool>,
) -> Option<SafetyConfirmation> {
    if server_type == ServerType::Bedrock
        && (gamemode.is_some_and(|mode| mode.eq_ignore_ascii_case("creative"))
            || cheats == Some(true))
    {
        return Some(SafetyConfirmation {
            kind: ConfirmationKind::BedrockAchievements,
        });
    }
    if server_type == ServerType::Java
        && gamemode.is_some_and(|mode| mode.eq_ignore_ascii_case("creative"))
    {
        return Some(SafetyConfirmation {
            kind: ConfirmationKind::JavaCreative,
        });
    }
    if server_type == ServerType::Java && commands == Some(true) {
        return Some(SafetyConfirmation {
            kind: ConfirmationKind::JavaCommands,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_domain::world_profile::WorldProfile;

    #[test]
    fn bedrock_creative_and_cheats_share_the_permanent_achievement_warning() {
        let creative = confirmation_for_server_creation(ServerType::Bedrock, Some("creative"));
        let mut profile = WorldProfile::new();
        profile.gameplay.cheats = Some(true);
        let cheats = confirmation_for_world_profile(ServerType::Bedrock, &profile);

        assert_eq!(creative, cheats);
        assert_eq!(creative.unwrap().token(), "bedrock_achievements");
    }

    #[test]
    fn server_override_is_separate_from_world_gamemode() {
        let world = confirmation_for_server_creation(ServerType::Java, Some("creative"));
        let server = confirmation_for_server_setting("force-gamemode", "true");

        assert_eq!(world.unwrap().token(), "java_creative");
        assert_eq!(server.unwrap().token(), "server_force_gamemode");
    }

    #[test]
    fn ordinary_commands_and_modes_do_not_require_confirmation() {
        assert!(confirmation_for_command(ServerType::Java, "say hello").is_none());
        assert!(confirmation_for_command(ServerType::Bedrock, "gamemode survival").is_none());
        assert!(!is_confirmed(
            SafetyConfirmation {
                kind: ConfirmationKind::JavaCreative,
            },
            Some("bedrock_achievements")
        ));
    }
}
