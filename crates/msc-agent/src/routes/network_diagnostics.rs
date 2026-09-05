//! DuckDNS label storage and the existing connectivity summary route.
use crate::{
    auth::AuthenticatedCredential,
    routes::lifecycle::{LifecycleRoutesState, error_response, invalid_body, require_permission},
};
use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use msc_api::dto::{
    ConnectivityPortDiagnosticDto, ConnectivityPortDiagnosticsDto, ConnectivityResponseDto,
    DuckDnsStatusResponseDto, DuckDnsUpdateRequestDto, DuckDnsUpdateResultDto,
    PermissionCategoryDto,
};
use msc_application::network_diagnostics::connectivity_summary_with_public_ip;
use msc_domain::networking::DiagnosticResult;
use msc_infrastructure::{
    duckdns::normalize_hostname, port_diagnostics::probe_tcp, public_ip::detect as detect_public_ip,
};
use std::{path::Path, time::Duration};

pub async fn duckdns_status(
    State(state): State<LifecycleRoutesState>,
) -> Json<DuckDnsStatusResponseDto> {
    let hostname = state.app_config_snapshot().duckdns_hostname;
    Json(DuckDnsStatusResponseDto {
        is_configured: hostname.is_some(),
        hostname,
    })
}
pub async fn update_duckdns(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<DuckDnsUpdateRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let hostname = match normalize_hostname(body.hostname.as_deref()) {
        Ok(hostname) => hostname,
        Err(_) => {
            return invalid_body(
                "invalid_hostname",
                "Hostname must be a DuckDNS hostname or empty.",
            );
        }
    };
    match state.update_duckdns_hostname(hostname.clone()) {
        Ok(()) => Json(DuckDnsUpdateResultDto {
            success: true,
            hostname,
            message: None,
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}
pub async fn connectivity(
    State(state): State<LifecycleRoutesState>,
) -> Json<ConnectivityResponseDto> {
    let config = state.app_config_snapshot();
    let active = config
        .active_server_id
        .as_ref()
        .and_then(|id| config.servers.iter().find(|server| &server.id == id));
    let Some(server) = active else {
        return Json(no_server_connectivity());
    };
    let port = if server.server_type == msc_domain::identity::ServerType::Bedrock {
        server.bedrock_port.unwrap_or(19132) as u16
    } else {
        java_server_port(Path::new(&server.server_dir)).unwrap_or(25565)
    };
    let running = state.status_snapshot().running;
    let local = if running {
        tokio::task::spawn_blocking(move || probe_tcp("127.0.0.1", port, Duration::from_secs(1)))
            .await
            .unwrap_or(DiagnosticResult::Unavailable)
    } else {
        DiagnosticResult::NotAttempted
    };
    let duckdns_hostname = if server.playit_enabled {
        None
    } else {
        config.duckdns_hostname.as_deref()
    };
    let public_ip = if duckdns_hostname.is_some() {
        None
    } else {
        let configured = server
            .public_host_override
            .as_deref()
            .map(str::trim)
            .filter(|host| !host.is_empty())
            .map(str::to_owned);
        match configured {
            Some(host) => Some(host),
            None => tokio::task::spawn_blocking(|| detect_public_ip(Duration::from_secs(2)))
                .await
                .unwrap_or(None),
        }
    };
    let summary = connectivity_summary_with_public_ip(
        duckdns_hostname,
        public_ip.as_deref(),
        port,
        local,
        if running {
            DiagnosticResult::Unavailable
        } else {
            DiagnosticResult::NotAttempted
        },
    );
    Json(ConnectivityResponseDto {
        server_type: if server.server_type == msc_domain::identity::ServerType::Bedrock {
            "bedrock"
        } else {
            "java"
        }
        .into(),
        server_name: server.display_name.clone(),
        server_running: running,
        status: if running { "unknown" } else { "offline" }.into(),
        severity: if running { "yellow" } else { "gray" }.into(),
        headline: if running {
            "Reachability unknown"
        } else {
            "Server is off"
        }
        .into(),
        detail: None,
        join_address: summary.join_address,
        method: summary.join_address_source.into(),
        join_address_source: summary.join_address_source.into(),
        local_listening: Some(matches!(summary.local, DiagnosticResult::Open)),
        externally_reachable: None,
        port_diagnostics: ConnectivityPortDiagnosticsDto {
            local: diagnostic(summary.local),
            public: diagnostic(summary.public),
        },
        note: None,
        help_id: None,
    })
}

fn java_server_port(server_dir: &Path) -> Option<u16> {
    std::fs::read_to_string(server_dir.join("server.properties"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|value| value.trim().parse().ok())
}
fn diagnostic(result: DiagnosticResult) -> ConnectivityPortDiagnosticDto {
    ConnectivityPortDiagnosticDto {
        outcome: result.api_outcome().into(),
        detail: None,
        help_id: None,
    }
}
fn no_server_connectivity() -> ConnectivityResponseDto {
    ConnectivityResponseDto {
        server_type: "java".into(),
        server_name: String::new(),
        server_running: false,
        status: "unknown".into(),
        severity: "gray".into(),
        headline: "Connectivity unavailable".into(),
        detail: None,
        join_address: None,
        method: "unavailable".into(),
        join_address_source: "unavailable".into(),
        local_listening: None,
        externally_reachable: None,
        port_diagnostics: ConnectivityPortDiagnosticsDto {
            local: diagnostic(DiagnosticResult::NotAttempted),
            public: diagnostic(DiagnosticResult::NotAttempted),
        },
        note: Some("no_active_server".into()),
        help_id: None,
    }
}
