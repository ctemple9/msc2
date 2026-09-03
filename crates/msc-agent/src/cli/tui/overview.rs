//! The first useful TUI data slice: selected host/server identity, lifecycle,
//! connection, health, activity, notes, and the console tail.
//!
//! This is a client-side view model. It deliberately does not duplicate agent
//! state or policy: every value below is read from the existing authenticated
//! API, and lifecycle changes are sent to the same routes as the scriptable
//! CLI.

use msc_api::dto::{
    CapabilitiesResponseDto, ConnectivityResponseDto, HealthResponseDto, PerformanceSnapshotDto,
    PermissionCategoryDto, RemoteApiStatus, ServerDto, WorldSlotsResponseDto,
};
use msc_infrastructure::console_buffer::ConsoleLine;

use super::session::ensure_active_server;
use super::transport::SharedClient;
use crate::cli::CliError;

pub const TAB_NAMES: [&str; 7] = [
    "Overview",
    "Players",
    "Worlds",
    "Performance",
    "Components",
    "Settings",
    "Files",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionVisibility {
    Local,
    Public,
    Hidden,
}

impl ConnectionVisibility {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "LOCAL",
            Self::Public => "PUBLIC",
            Self::Hidden => "HIDDEN",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OverviewState {
    pub servers: Vec<ServerDto>,
    pub selected_server_id: Option<String>,
    pub status: Option<RemoteApiStatus>,
    pub capabilities: Option<CapabilitiesResponseDto>,
    pub connectivity: Option<ConnectivityResponseDto>,
    pub performance: Option<PerformanceSnapshotDto>,
    pub health: Option<HealthResponseDto>,
    pub worlds: Option<WorldSlotsResponseDto>,
    pub console: Vec<ConsoleLine>,
    pub activity: Vec<String>,
    pub loading: bool,
    pub error: Option<String>,
}

impl OverviewState {
    pub fn placeholder() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

    /// Fetch the overview from the agent. The first three responses are the
    /// identity/authentication foundation; secondary cards are best effort so
    /// an older agent can still provide a useful first screen.
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let capabilities: CapabilitiesResponseDto = client.get_json("/v1/capabilities").await?;
        let servers: Vec<ServerDto> = client.get_json("/v1/servers").await?;
        let status: RemoteApiStatus = client.get_json("/v1/status").await?;
        let selected_server_id = status.active_server_id.clone();

        let connectivity: Option<ConnectivityResponseDto> =
            client.get_json("/v1/connectivity").await.ok();
        let performance: Option<PerformanceSnapshotDto> =
            client.get_json("/v1/performance").await.ok();
        let health: Option<HealthResponseDto> = client.get_json("/v1/health").await.ok();
        let worlds: Option<WorldSlotsResponseDto> = client.get_json("/v1/worlds").await.ok();
        let console: Vec<ConsoleLine> = client
            .get_json("/v1/console/tail?n=12")
            .await
            .unwrap_or_default();

        let mut activity = Vec::new();
        if let Some(worlds) = &worlds
            && let Some(active_world) = worlds
                .slots
                .iter()
                .find(|slot| slot.is_active || Some(&slot.id) == worlds.active_slot_id.as_ref())
        {
            activity.push(format!("Active world: {}", active_world.name));
        }
        if let Some(chat) = console.iter().rev().find(|line| line.source == "chat") {
            activity.push(format!("Chat: {}", chat.text));
        }
        if let Some(line) = console.last() {
            activity.push(format!("{}: {}", line.source, line.text));
        }

        Ok(Self {
            servers,
            selected_server_id,
            status: Some(status),
            capabilities: Some(capabilities),
            connectivity,
            performance,
            health,
            worlds,
            console,
            activity,
            loading: false,
            error: None,
        })
    }

    pub fn selected_server(&self) -> Option<&ServerDto> {
        self.selected_server_id
            .as_ref()
            .and_then(|id| self.servers.iter().find(|server| &server.id == id))
    }

    pub fn selected_server_name(&self) -> &str {
        self.selected_server()
            .map(|server| server.name.as_str())
            .unwrap_or("No active server")
    }

    pub fn lifecycle_label(&self) -> &'static str {
        match self.status.as_ref().map(|status| status.running) {
            Some(true) => "RUNNING",
            Some(false) => "STOPPED",
            None if self.loading => "LOADING",
            None => "UNKNOWN",
        }
    }

    pub fn server_type_label(&self) -> &str {
        self.selected_server()
            .map(|server| server.server_type.as_str())
            .or_else(|| {
                self.status
                    .as_ref()
                    .and_then(|status| status.server_type.as_deref())
            })
            .unwrap_or("unknown")
    }

    pub fn edition_label(&self) -> String {
        let Some(server) = self.selected_server() else {
            return "Unknown edition".to_string();
        };
        if server.server_type.eq_ignore_ascii_case("bedrock") {
            "Bedrock".to_string()
        } else if let Some(flavor) = &server.java_flavor {
            format!("Java / {flavor}")
        } else {
            "Java".to_string()
        }
    }

    pub fn connection_visibility(&self) -> ConnectionVisibility {
        let Some(connection) = &self.connectivity else {
            return ConnectionVisibility::Hidden;
        };
        if connection.join_address.is_none() {
            return ConnectionVisibility::Hidden;
        }
        let source = format!(
            "{} {}",
            connection.method.to_ascii_lowercase(),
            connection.join_address_source.to_ascii_lowercase()
        );
        if source.contains("public")
            || source.contains("playit")
            || source.contains("duckdns")
            || connection.externally_reachable == Some(true)
        {
            ConnectionVisibility::Public
        } else {
            ConnectionVisibility::Local
        }
    }

    pub fn connection_detail(&self) -> String {
        let Some(connection) = &self.connectivity else {
            return "Connection information unavailable".to_string();
        };
        let address = connection
            .join_address
            .as_deref()
            .unwrap_or("No join address advertised");
        format!(
            "{} · {} · {}",
            self.connection_visibility().label(),
            connection.server_type,
            address
        )
    }

    pub fn available_tabs(&self) -> Vec<usize> {
        let Some(capabilities) = &self.capabilities else {
            return (0..TAB_NAMES.len()).collect();
        };
        let permissions = &capabilities.base.permissions;
        let has = |permission| permissions.contains(&permission);
        let mut tabs = vec![0];
        if has(PermissionCategoryDto::Players) {
            tabs.push(1);
        }
        if has(PermissionCategoryDto::Worlds) {
            tabs.push(2);
        }
        tabs.push(3);
        if has(PermissionCategoryDto::Addons) {
            tabs.push(4);
        }
        if has(PermissionCategoryDto::Settings) {
            tabs.push(5);
        }
        tabs.push(6);
        tabs
    }

    pub fn tab_is_available(&self, index: usize) -> bool {
        self.available_tabs().contains(&index)
    }

    pub fn health_summary(&self) -> String {
        let Some(health) = &self.health else {
            return "Health information unavailable".to_string();
        };
        let cards = health.cards.len();
        format!(
            "{} · {cards} checks · {}",
            health.overall_severity, health.server_name
        )
    }

    pub fn stats_summary(&self) -> String {
        let Some(performance) = &self.performance else {
            return "Live stats unavailable".to_string();
        };
        let players = performance
            .players_online
            .map_or_else(|| "—".to_string(), |value| value.to_string());
        let tps = performance
            .tps_1m
            .as_ref()
            .map_or_else(|| "—".to_string(), |metric| format!("{:.1}", metric.value));
        format!("Players {players} · TPS (1m) {tps}")
    }

    pub async fn select_server(client: &SharedClient, selector: &str) -> Result<Self, CliError> {
        ensure_active_server(client, Some(selector)).await?;
        Self::load(client).await
    }

    pub async fn request_lifecycle(client: &SharedClient, start: bool) -> Result<Self, CliError> {
        let path = if start { "/v1/start" } else { "/v1/stop" };
        let _: msc_api::dto::SimpleResultDto =
            client.post_json(path, &serde_json::json!({})).await?;
        Self::load(client).await
    }
}
