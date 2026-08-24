//! Remote desktop-pairing exchange. The Tauri Rust backend is the only
//! caller: it stores the returned bearer credential before any webview code
//! can observe it.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Extension, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthState, DesktopPairingError};
use crate::routes::lifecycle::{error_response, invalid_body};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopPairingExchangeRequest {
    pairing_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopCredentialResult {
    agent_host_id: String,
    credential_id: String,
    token: String,
    expires_at: Option<String>,
}

pub async fn exchange_desktop_pairing(
    Extension(auth): Extension<AuthState>,
    body: Result<Json<DesktopPairingExchangeRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be valid JSON."),
    };
    let credential = match auth.exchange_desktop_pairing(&request.pairing_code) {
        Ok(credential) => credential,
        Err(DesktopPairingError::Consumed) => {
            return error_response(
                StatusCode::CONFLICT,
                "pairing_consumed",
                "This pairing code was already used.",
            );
        }
        Err(DesktopPairingError::Expired) => {
            return error_response(
                StatusCode::GONE,
                "pairing_expired",
                "This pairing code has expired.",
            );
        }
        Err(DesktopPairingError::Store(message)) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &message,
            );
        }
        Err(DesktopPairingError::Unauthorized) => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "The pairing code is invalid.",
            );
        }
    };
    Json(DesktopCredentialResult {
        agent_host_id: credential.agent_host_id,
        credential_id: credential.issued.credential_id,
        token: credential.issued.token,
        expires_at: credential.expires_at.map(unix_timestamp),
    })
    .into_response()
}

fn unix_timestamp(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
