//! Player access and named-token access surfaces.
//!
//! Allowlist mutations use the existing Bedrock route. Named-token data is
//! intentionally decoded locally because the agent keeps those route structs
//! private; the TUI still sends the same documented JSON shapes and does not
//! persist or print a newly issued bearer token.

use crossterm::event::KeyCode;
use serde::Deserialize;

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AccessSurface {
    #[default]
    Allowlist,
    Users,
    Me,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccessIdentity {
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub is_named_token: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistEntry {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AllowlistResponse {
    #[serde(default)]
    pub entries: Vec<AllowlistEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamedUser {
    pub id: String,
    pub label: String,
    pub role: String,
    pub is_expired: bool,
    #[serde(default)]
    pub expires_at_iso8601: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserListResponse {
    #[serde(default)]
    pub users: Vec<NamedUser>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessMutation {
    AllowlistAdd { name: String },
    AllowlistRemove { name: String },
    RevokeUser { user_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessIntent {
    Confirm(AccessMutation),
}

#[derive(Debug, Clone, Default)]
pub struct AccessState {
    pub surface: AccessSurface,
    pub identity: Option<AccessIdentity>,
    pub allowlist: Vec<AllowlistEntry>,
    pub users: Vec<NamedUser>,
    pub selected: usize,
    pub input: Option<String>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl AccessState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let identity = client.get_json("/v1/me").await?;
        let allowlist = client
            .get_json::<AllowlistResponse>("/v1/allowlist")
            .await
            .map(|response| response.entries)
            .unwrap_or_default();
        let users = client
            .get_json::<UserListResponse>("/v1/users")
            .await
            .map(|response| response.users)
            .unwrap_or_default();
        Ok(Self {
            identity: Some(identity),
            allowlist,
            users,
            loaded: true,
            ..Self::default()
        })
    }

    pub fn selected_allowlist(&self) -> Option<&AllowlistEntry> {
        (self.surface == AccessSurface::Allowlist)
            .then(|| self.allowlist.get(self.selected))
            .flatten()
    }

    pub fn selected_user(&self) -> Option<&NamedUser> {
        (self.surface == AccessSurface::Users)
            .then(|| self.users.get(self.selected))
            .flatten()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<AccessIntent> {
        if let Some(mut value) = self.input.take() {
            return match key {
                KeyCode::Esc => None,
                KeyCode::Backspace => {
                    value.pop();
                    self.input = Some(value);
                    None
                }
                KeyCode::Char(character) => {
                    value.push(character);
                    self.input = Some(value);
                    None
                }
                KeyCode::Enter if !value.trim().is_empty() => {
                    Some(AccessIntent::Confirm(AccessMutation::AllowlistAdd {
                        name: value.trim().to_string(),
                    }))
                }
                _ => {
                    self.input = Some(value);
                    None
                }
            };
        }
        match key {
            KeyCode::Char('1') => {
                self.surface = AccessSurface::Allowlist;
                self.selected = 0;
            }
            KeyCode::Char('2') => {
                self.surface = AccessSurface::Users;
                self.selected = 0;
            }
            KeyCode::Char('3') => {
                self.surface = AccessSurface::Me;
                self.selected = 0;
            }
            KeyCode::Char('a') if self.surface == AccessSurface::Allowlist => {
                self.input = Some(String::new());
            }
            KeyCode::Char('x') if self.surface == AccessSurface::Allowlist => {
                let entry = self.selected_allowlist()?.name.clone();
                return Some(AccessIntent::Confirm(AccessMutation::AllowlistRemove {
                    name: entry,
                }));
            }
            KeyCode::Char('x') if self.surface == AccessSurface::Users => {
                let user_id = self.selected_user()?.id.clone();
                return Some(AccessIntent::Confirm(AccessMutation::RevokeUser {
                    user_id,
                }));
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn move_selection(&mut self, offset: isize) {
        let count = match self.surface {
            AccessSurface::Allowlist => self.allowlist.len(),
            AccessSurface::Users => self.users.len(),
            AccessSurface::Me => 1,
        };
        if count > 0 {
            self.selected = (self.selected as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    pub async fn mutate_allowlist(
        client: &SharedClient,
        add: bool,
        name: String,
    ) -> Result<serde_json::Value, CliError> {
        client
            .post_json(
                "/v1/allowlist",
                &serde_json::json!({"action": if add { "add" } else { "remove" }, "name": name}),
            )
            .await
    }

    pub async fn revoke_user(
        client: &SharedClient,
        user_id: String,
    ) -> Result<serde_json::Value, CliError> {
        client
            .post_json("/v1/users/revoke", &serde_json::json!({"userId": user_id}))
            .await
    }
}
