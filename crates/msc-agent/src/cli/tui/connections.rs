//! Focused connection and management-service state.
//!
//! This module only presents the existing diagnostics and service routes. It
//! never decides whether a tunnel is allowed or reachable; those answers come
//! from the agent response and the app's existing permission gate.

use crossterm::event::KeyCode;
use msc_api::dto::{
    BroadcastAutoStartDto, BroadcastCredentialsDto, BroadcastCredentialsStatusDto,
    BroadcastSimpleResultDto, BroadcastStatusDto, ConnectivityResponseDto,
    DuckDnsStatusResponseDto, DuckDnsUpdateRequestDto, DuckDnsUpdateResultDto,
    PlayitActionResultDto, PlayitStatusDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectionSurface {
    #[default]
    Connection,
    Services,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionInputKind {
    DuckDns,
    BroadcastCredentials,
}

impl ConnectionInputKind {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::DuckDns => "DuckDNS hostname (blank clears)",
            Self::BroadcastCredentials => "email|password|gamertag",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionMutation {
    Playit {
        start: bool,
    },
    Broadcast {
        start: bool,
    },
    BroadcastAutostart {
        enabled: bool,
    },
    BroadcastCredentials {
        email: String,
        password: String,
        gamertag: String,
    },
    DuckDns {
        hostname: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionIntent {
    Confirm(ConnectionMutation),
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionsState {
    pub surface: ConnectionSurface,
    pub connectivity: Option<ConnectivityResponseDto>,
    pub playit: Option<PlayitStatusDto>,
    pub broadcast: Option<BroadcastStatusDto>,
    pub broadcast_autostart: Option<BroadcastAutoStartDto>,
    pub broadcast_credentials: Option<BroadcastCredentialsStatusDto>,
    pub duckdns: Option<DuckDnsStatusResponseDto>,
    pub input: Option<(ConnectionInputKind, String)>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl ConnectionsState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        Ok(Self {
            connectivity: client.get_json("/v1/connectivity").await.ok(),
            playit: client.get_json("/v1/playit").await.ok(),
            broadcast: client.get_json("/v1/broadcast/status").await.ok(),
            broadcast_autostart: client.get_json("/v1/broadcast/autostart").await.ok(),
            broadcast_credentials: client.get_json("/v1/broadcast/credentials").await.ok(),
            duckdns: client.get_json("/v1/duckdns").await.ok(),
            loaded: true,
            ..Self::default()
        })
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<ConnectionIntent> {
        if let Some((kind, value)) = self.input.take() {
            return self.handle_input(kind, value, key);
        }
        match key {
            KeyCode::Char('1') => self.surface = ConnectionSurface::Connection,
            KeyCode::Char('2') => self.surface = ConnectionSurface::Services,
            KeyCode::Char('d') if self.surface == ConnectionSurface::Connection => {
                self.input = Some((ConnectionInputKind::DuckDns, String::new()));
            }
            KeyCode::Char('p') if self.surface == ConnectionSurface::Services => {
                let start = !self.playit.as_ref().is_some_and(|status| status.is_running);
                return Some(ConnectionIntent::Confirm(ConnectionMutation::Playit {
                    start,
                }));
            }
            KeyCode::Char('x') if self.surface == ConnectionSurface::Services => {
                let start = !self
                    .broadcast
                    .as_ref()
                    .is_some_and(|status| status.xbox_broadcast_running);
                return Some(ConnectionIntent::Confirm(ConnectionMutation::Broadcast {
                    start,
                }));
            }
            KeyCode::Char('a') if self.surface == ConnectionSurface::Services => {
                let enabled = !self
                    .broadcast_autostart
                    .as_ref()
                    .is_some_and(|status| status.enabled);
                return Some(ConnectionIntent::Confirm(
                    ConnectionMutation::BroadcastAutostart { enabled },
                ));
            }
            KeyCode::Char('e') if self.surface == ConnectionSurface::Services => {
                self.input = Some((ConnectionInputKind::BroadcastCredentials, String::new()));
            }
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn handle_input(
        &mut self,
        kind: ConnectionInputKind,
        mut value: String,
        key: KeyCode,
    ) -> Option<ConnectionIntent> {
        match key {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.input = Some((kind, value));
            }
            KeyCode::Char(character) => {
                value.push(character);
                self.input = Some((kind, value));
            }
            KeyCode::Enter => match kind {
                ConnectionInputKind::DuckDns => {
                    let hostname = (!value.trim().is_empty()).then(|| value.trim().to_string());
                    return Some(ConnectionIntent::Confirm(ConnectionMutation::DuckDns {
                        hostname,
                    }));
                }
                ConnectionInputKind::BroadcastCredentials => {
                    let mut parts = value.splitn(3, '|');
                    let email = parts.next().unwrap_or_default().trim().to_string();
                    let password = parts.next().unwrap_or_default().to_string();
                    let gamertag = parts.next().unwrap_or_default().trim().to_string();
                    if !email.is_empty() && !password.is_empty() && !gamertag.is_empty() {
                        return Some(ConnectionIntent::Confirm(
                            ConnectionMutation::BroadcastCredentials {
                                email,
                                password,
                                gamertag,
                            },
                        ));
                    }
                    self.status = Some("Credentials require email|password|gamertag".to_string());
                }
            },
            _ => self.input = Some((kind, value)),
        }
        None
    }

    pub async fn playit(
        client: &SharedClient,
        start: bool,
    ) -> Result<PlayitActionResultDto, CliError> {
        client
            .post_json(
                if start {
                    "/v1/playit/start"
                } else {
                    "/v1/playit/stop"
                },
                &serde_json::json!({}),
            )
            .await
    }

    pub async fn broadcast(
        client: &SharedClient,
        start: bool,
    ) -> Result<BroadcastSimpleResultDto, CliError> {
        client
            .post_json(
                if start {
                    "/v1/broadcast/start"
                } else {
                    "/v1/broadcast/stop"
                },
                &serde_json::json!({}),
            )
            .await
    }

    pub async fn set_autostart(
        client: &SharedClient,
        enabled: bool,
    ) -> Result<BroadcastAutoStartDto, CliError> {
        client
            .post_json(
                "/v1/broadcast/autostart",
                &BroadcastAutoStartDto { enabled },
            )
            .await
    }

    pub async fn set_credentials(
        client: &SharedClient,
        email: String,
        password: String,
        gamertag: String,
    ) -> Result<BroadcastSimpleResultDto, CliError> {
        client
            .post_json(
                "/v1/broadcast/credentials",
                &BroadcastCredentialsDto {
                    email,
                    password,
                    gamertag,
                },
            )
            .await
    }

    pub async fn set_duckdns(
        client: &SharedClient,
        hostname: Option<String>,
    ) -> Result<DuckDnsUpdateResultDto, CliError> {
        client
            .post_json("/v1/duckdns", &DuckDnsUpdateRequestDto { hostname })
            .await
    }
}
