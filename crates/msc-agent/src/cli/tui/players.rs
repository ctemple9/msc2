//! Keyboard-first Players state for the authenticated TUI.
//!
//! The response types live here because the agent keeps the player-profile
//! DTOs route-local. They are deliberately read-only observations; every
//! mutation below is converted into an existing agent request by `app.rs`.

use crossterm::event::KeyCode;
use msc_infrastructure::console_buffer::ConsoleLine;
use serde::Deserialize;

use super::transport::SharedClient;
use crate::cli::CliError;

const MAX_SESSION_EVENTS: usize = 200;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnlinePlayer {
    pub name: String,
    #[serde(default)]
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayersResponse {
    #[serde(default)]
    pub players: Vec<OnlinePlayer>,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub image_identifier: String,
    #[serde(default)]
    pub is_online: bool,
    #[serde(default)]
    pub is_op: bool,
    #[serde(default)]
    pub last_seen: Option<String>,
    #[serde(default)]
    pub is_bedrock_player: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub skin_override_identifier: Option<String>,
    #[serde(default)]
    pub stats: Option<PlayerStats>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfilesResponse {
    #[serde(default)]
    pub profiles: Vec<PlayerProfile>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub game_mode_display: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub dimension_display: String,
    pub score: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub player_name: String,
    pub event_type: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SessionLogResponse {
    #[serde(default)]
    events: Vec<SessionEvent>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AllowlistResponse {
    #[serde(default)]
    entries: Vec<AllowlistEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AllowlistEntry {
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSort {
    LastSeen,
    Name,
}

impl ProfileSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::LastSeen => "last seen",
            Self::Name => "name A–Z",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::LastSeen => Self::Name,
            Self::Name => Self::LastSeen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerInputKind {
    ProfileSearch,
    SessionFilter,
    AllowlistAdd,
    Identify,
    CustomUuid,
    SkinOverride,
}

impl PlayerInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::ProfileSearch => "Search profiles",
            Self::SessionFilter => "Filter session log",
            Self::AllowlistAdd => "Add Bedrock player",
            Self::Identify => "Bedrock gamertag",
            Self::CustomUuid => "Target UUID",
            Self::SkinOverride => "Skin lookup identifier (blank clears)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerMutation {
    ClearSessionLog,
    ToggleHidden {
        profile_id: String,
        hidden: bool,
    },
    Delete {
        profile_id: String,
    },
    Duplicate {
        profile_id: String,
    },
    MigrateOffline {
        profile_id: String,
    },
    MigrateCustom {
        profile_id: String,
        target_uuid: String,
    },
    Identify {
        profile_id: String,
        gamertag: String,
    },
    SkinOverride {
        profile_id: String,
        lookup_identifier: Option<String>,
    },
    AllowlistAdd {
        name: String,
    },
    AllowlistRemove {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerIntent {
    Confirm(PlayerMutation),
    ClearSessionLog,
    ClearLocalSession,
}

#[derive(Debug, Clone)]
pub struct PlayersState {
    pub online: PlayersResponse,
    pub profiles: Vec<PlayerProfile>,
    pub session_events: Vec<SessionEvent>,
    pub allowlist: Vec<String>,
    pub allowlist_selected: usize,
    pub is_bedrock: bool,
    pub loaded: bool,
    pub error: Option<String>,
    pub show_hidden: bool,
    pub sort: ProfileSort,
    pub profile_query: String,
    pub session_query: String,
    pub detail_open: bool,
    pub selected_profile: usize,
    pub input: Option<(PlayerInputKind, String)>,
    pub status: Option<String>,
}

impl Default for PlayersState {
    fn default() -> Self {
        Self {
            online: PlayersResponse::default(),
            profiles: Vec::new(),
            session_events: Vec::new(),
            allowlist: Vec::new(),
            allowlist_selected: 0,
            is_bedrock: false,
            loaded: false,
            error: None,
            show_hidden: false,
            sort: ProfileSort::LastSeen,
            profile_query: String::new(),
            session_query: String::new(),
            detail_open: false,
            selected_profile: 0,
            input: None,
            status: None,
        }
    }
}

impl PlayersState {
    pub async fn load(client: &SharedClient, is_bedrock: bool) -> Result<Self, CliError> {
        let online: PlayersResponse = client.get_json("/v1/players").await?;
        let profiles: PlayerProfilesResponse = client.get_json("/v1/players/profiles").await?;

        let (session_events, status) = if is_bedrock {
            let lines: Vec<ConsoleLine> = client
                .get_json("/v1/console/tail?n=200")
                .await
                .unwrap_or_default();
            (session_events_from_console(&lines), None)
        } else {
            match client
                .get_json::<SessionLogResponse>("/v1/session-log")
                .await
            {
                Ok(log) => (log.events, None),
                Err(error) => (Vec::new(), Some(error.to_string())),
            }
        };

        let (allowlist, allowlist_status) = if is_bedrock {
            match client.get_json::<AllowlistResponse>("/v1/allowlist").await {
                Ok(response) => (
                    response
                        .entries
                        .into_iter()
                        .map(|entry| entry.name)
                        .collect(),
                    None,
                ),
                Err(error) => (Vec::new(), Some(error.to_string())),
            }
        } else {
            (Vec::new(), None)
        };

        let mut state = Self {
            online,
            profiles: profiles.profiles,
            session_events,
            allowlist,
            allowlist_selected: 0,
            is_bedrock,
            loaded: true,
            error: None,
            show_hidden: false,
            sort: ProfileSort::LastSeen,
            profile_query: String::new(),
            session_query: String::new(),
            detail_open: false,
            selected_profile: 0,
            input: None,
            status: status.or(allowlist_status),
        };
        state.normalize_selection();
        Ok(state)
    }

    pub fn display_name(profile: &PlayerProfile) -> String {
        profile
            .username
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| short_id(&profile.id))
    }

    pub fn filtered_profiles(&self) -> Vec<&PlayerProfile> {
        let needle = self.profile_query.trim().to_ascii_lowercase();
        let mut profiles = self
            .profiles
            .iter()
            .filter(|profile| self.show_hidden || !profile.is_hidden)
            .filter(|profile| {
                needle.is_empty()
                    || Self::display_name(profile)
                        .to_ascii_lowercase()
                        .contains(&needle)
                    || profile.id.to_ascii_lowercase().contains(&needle)
            })
            .collect::<Vec<_>>();
        match self.sort {
            ProfileSort::LastSeen => profiles.sort_by(|left, right| {
                right
                    .last_seen
                    .cmp(&left.last_seen)
                    .then_with(|| Self::display_name(left).cmp(&Self::display_name(right)))
            }),
            ProfileSort::Name => profiles
                .sort_by(|left, right| Self::display_name(left).cmp(&Self::display_name(right))),
        }
        profiles
    }

    pub fn filtered_session_events(&self) -> Vec<&SessionEvent> {
        let needle = self.session_query.trim().to_ascii_lowercase();
        self.session_events
            .iter()
            .filter(|event| {
                needle.is_empty() || event.player_name.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn selected_profile(&self) -> Option<&PlayerProfile> {
        self.filtered_profiles().get(self.selected_profile).copied()
    }

    pub fn online_summary(&self) -> String {
        let seen = self
            .session_events
            .iter()
            .map(|event| event.player_name.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        format!(
            "{} online · {} seen this session",
            self.online.count.max(self.online.players.len()),
            seen
        )
    }

    pub fn clear_local_session(&mut self) {
        self.session_events.clear();
        self.status = Some("Session history cleared locally".to_string());
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<PlayerIntent> {
        if self.input.is_some() {
            return self.handle_input(key);
        }
        if self.detail_open {
            return self.handle_detail_key(key);
        }
        match key {
            KeyCode::Char('/') => self.begin_input(PlayerInputKind::ProfileSearch),
            KeyCode::Char('f') => self.begin_input(PlayerInputKind::SessionFilter),
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.normalize_selection();
            }
            KeyCode::Char('r') => self.loaded = false,
            KeyCode::Char('H') => {
                self.show_hidden = !self.show_hidden;
                self.normalize_selection();
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_profile(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_profile(-1),
            KeyCode::Enter => self.detail_open = self.selected_profile().is_some(),
            KeyCode::Char('l') => {
                return Some(if self.is_bedrock {
                    PlayerIntent::ClearLocalSession
                } else {
                    PlayerIntent::ClearSessionLog
                });
            }
            KeyCode::Char('a') if self.is_bedrock => {
                self.begin_input(PlayerInputKind::AllowlistAdd)
            }
            KeyCode::Char('x') if self.is_bedrock => {
                let name = self.allowlist.get(self.allowlist_selected)?.clone();
                return Some(PlayerIntent::Confirm(PlayerMutation::AllowlistRemove {
                    name,
                }));
            }
            _ => {}
        }
        None
    }

    fn handle_detail_key(&mut self, key: KeyCode) -> Option<PlayerIntent> {
        let profile = self.selected_profile()?.clone();
        match key {
            KeyCode::Esc => self.detail_open = false,
            KeyCode::Char('h') => {
                return Some(PlayerIntent::Confirm(PlayerMutation::ToggleHidden {
                    profile_id: profile.id,
                    hidden: !profile.is_hidden,
                }));
            }
            KeyCode::Char('d') => {
                return Some(PlayerIntent::Confirm(PlayerMutation::Delete {
                    profile_id: profile.id,
                }));
            }
            KeyCode::Char('u') => {
                return Some(PlayerIntent::Confirm(PlayerMutation::Duplicate {
                    profile_id: profile.id,
                }));
            }
            KeyCode::Char('m') => {
                return Some(PlayerIntent::Confirm(PlayerMutation::MigrateOffline {
                    profile_id: profile.id,
                }));
            }
            KeyCode::Char('i') => self.begin_input(PlayerInputKind::Identify),
            KeyCode::Char('t') => self.begin_input(PlayerInputKind::CustomUuid),
            KeyCode::Char('o') => self.begin_input(PlayerInputKind::SkinOverride),
            _ => {}
        }
        None
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<PlayerIntent> {
        let (kind, mut value) = self.input.take()?;
        match key {
            KeyCode::Esc => self.input = None,
            KeyCode::Backspace => {
                value.pop();
                self.input = Some((kind, value));
            }
            KeyCode::Enter => {
                return self.finish_input(kind, value);
            }
            KeyCode::Char(character) => {
                value.push(character);
                self.input = Some((kind, value));
            }
            _ => self.input = Some((kind, value)),
        }
        None
    }

    fn finish_input(&mut self, kind: PlayerInputKind, value: String) -> Option<PlayerIntent> {
        let value = value.trim().to_string();
        match kind {
            PlayerInputKind::ProfileSearch => self.profile_query = value,
            PlayerInputKind::SessionFilter => self.session_query = value,
            PlayerInputKind::AllowlistAdd if !value.is_empty() => {
                return Some(PlayerIntent::Confirm(PlayerMutation::AllowlistAdd {
                    name: value,
                }));
            }
            PlayerInputKind::Identify if !value.is_empty() => {
                let profile_id = self.selected_profile()?.id.clone();
                return Some(PlayerIntent::Confirm(PlayerMutation::Identify {
                    profile_id,
                    gamertag: value,
                }));
            }
            PlayerInputKind::CustomUuid if !value.is_empty() => {
                let profile_id = self.selected_profile()?.id.clone();
                return Some(PlayerIntent::Confirm(PlayerMutation::MigrateCustom {
                    profile_id,
                    target_uuid: value,
                }));
            }
            PlayerInputKind::SkinOverride => {
                let profile_id = self.selected_profile()?.id.clone();
                return Some(PlayerIntent::Confirm(PlayerMutation::SkinOverride {
                    profile_id,
                    lookup_identifier: (!value.is_empty()).then_some(value),
                }));
            }
            _ => {}
        }
        self.normalize_selection();
        None
    }

    fn begin_input(&mut self, kind: PlayerInputKind) {
        self.input = Some((kind, String::new()));
    }

    fn move_profile(&mut self, offset: isize) {
        let count = self.filtered_profiles().len();
        if count == 0 {
            self.selected_profile = 0;
            return;
        }
        self.selected_profile =
            (self.selected_profile as isize + offset).rem_euclid(count as isize) as usize;
    }

    fn normalize_selection(&mut self) {
        self.selected_profile = self
            .selected_profile
            .min(self.filtered_profiles().len().saturating_sub(1));
        self.allowlist_selected = self
            .allowlist_selected
            .min(self.allowlist.len().saturating_sub(1));
    }
}

pub fn session_events_from_console(lines: &[ConsoleLine]) -> Vec<SessionEvent> {
    lines
        .iter()
        .filter_map(|line| {
            let (player_name, event_type) = line
                .text
                .strip_suffix(" joined the game")
                .map(|name| (name.trim(), "joined"))
                .or_else(|| {
                    line.text
                        .strip_suffix(" left the game")
                        .map(|name| (name.trim(), "left"))
                })?;
            if player_name.is_empty() {
                return None;
            }
            Some(SessionEvent {
                player_name: player_name.to_string(),
                event_type: event_type.to_string(),
                timestamp: line.ts.clone(),
            })
        })
        .rev()
        .take(MAX_SESSION_EVENTS)
        .collect()
}

pub fn profile_status(profile: &PlayerProfile) -> &'static str {
    if profile.is_online {
        "ONLINE"
    } else {
        "OFFLINE"
    }
}

pub fn profile_edition(profile: &PlayerProfile) -> &'static str {
    if profile.is_bedrock_player {
        "Bedrock"
    } else {
        "Java"
    }
}

fn short_id(id: &str) -> String {
    id.get(..8)
        .map_or_else(|| id.to_string(), |prefix| format!("{prefix}…"))
}
