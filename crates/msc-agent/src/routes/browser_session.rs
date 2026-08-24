//! Browser pairing and session endpoints from the P11.21 public contract.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Extension, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use msc_api::dto::PermissionCategoryDto;
use serde::{Deserialize, Serialize};

use crate::auth::{
    AuthState, AuthenticatedCredential, BrowserSessionAuthentication, BrowserSessionError,
    CreateBrowserPairing, CredentialRole, cleared_session_cookie, forbidden,
    request_has_exact_origin, request_uses_https, session_cookie,
};
use crate::routes::lifecycle::{error_response, invalid_body, require_permission};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingCreateRequest {
    client_kind: String,
    label: String,
    role: String,
    permissions: Vec<PermissionCategoryDto>,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserSessionExchangeRequest {
    pairing_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PairingCreateResult {
    pairing_code: String,
    agent_host_id: String,
    client_kind: &'static str,
    expires_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CsrfTokenResponse {
    csrf_token: String,
    expires_at: String,
}

pub async fn create_pairing(
    Extension(auth): Extension<AuthState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<PairingCreateRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be valid JSON."),
    };
    if request.client_kind != "browser" {
        return invalid_body(
            "invalid_client_kind",
            "This endpoint creates browser pairing codes only.",
        );
    }
    let Some(role) = parse_role(&request.role) else {
        return invalid_body("invalid_role", "The pairing role is not recognized.");
    };
    let label = request.label.trim();
    if label.is_empty() {
        return invalid_body("label_empty", "The pairing label must not be blank.");
    }
    if auth.browser_pairing_creation_is_rate_limited(&credential.credential_id) {
        auth.record_browser_audit(
            &credential.label,
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
        );
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Try again later.",
        );
    }
    // The frozen contract exposes ISO-8601 expiry for a later desktop grant.
    // Browser pairings in this step deliberately use the session's bounded
    // lifetime rather than accepting a date string with an ad-hoc parser.
    if request.expires_at.is_some() {
        return invalid_body(
            "invalid_expiry",
            "Browser pairing expiry is managed by the agent session policy.",
        );
    }
    let created = match auth.create_browser_pairing(CreateBrowserPairing {
        label: label.to_string(),
        role,
        permissions: request.permissions,
        expires_at: None,
    }) {
        Ok(created) => created,
        Err(BrowserSessionError::Store(message)) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &message,
            );
        }
        Err(_) => {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Try again later.",
            );
        }
    };
    auth.record_browser_audit(
        &credential.label,
        StatusCode::CREATED,
        "browser_pairing_created",
    );
    Json(PairingCreateResult {
        pairing_code: created.pairing_code,
        // The durable installation identity required by remote desktop pairing
        // is P11.23 work. Browser pairing has no host-store consumer, so this
        // value is intentionally not derived from an address or URL.
        agent_host_id: "browser-session-agent".to_string(),
        client_kind: "browser",
        expires_at: unix_timestamp(created.expires_at),
    })
    .into_response()
}

pub async fn exchange_browser_session(
    Extension(auth): Extension<AuthState>,
    headers: HeaderMap,
    body: Result<Json<BrowserSessionExchangeRequest>, JsonRejection>,
) -> Response {
    if !request_has_exact_origin(&headers) {
        auth.record_browser_audit("anonymous", StatusCode::FORBIDDEN, "wrong_origin");
        return forbidden(
            "wrong_origin",
            "Browser pairing must use this agent origin.",
        );
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be valid JSON."),
    };
    let session = match auth.exchange_browser_pairing(&request.pairing_code) {
        Ok(session) => session,
        Err(BrowserSessionError::Consumed) => {
            auth.record_browser_audit("anonymous", StatusCode::CONFLICT, "pairing_consumed");
            return error_response(
                StatusCode::CONFLICT,
                "pairing_consumed",
                "This pairing code was already used.",
            );
        }
        Err(BrowserSessionError::Expired) => {
            auth.record_browser_audit("anonymous", StatusCode::GONE, "pairing_expired");
            return error_response(
                StatusCode::GONE,
                "pairing_expired",
                "This pairing code has expired.",
            );
        }
        Err(BrowserSessionError::Store(message)) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &message,
            );
        }
        Err(BrowserSessionError::Unauthorized) => {
            if auth.browser_failure_is_rate_limited("browser-pairing") {
                auth.record_browser_audit(
                    "anonymous",
                    StatusCode::TOO_MANY_REQUESTS,
                    "rate_limited",
                );
                return error_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    "rate_limited",
                    "Try again later.",
                );
            }
            auth.record_browser_audit("anonymous", StatusCode::UNAUTHORIZED, "unauthorized");
            return error_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "The pairing code is invalid.",
            );
        }
    };
    auth.record_browser_audit(
        &session.credential_id,
        StatusCode::NO_CONTENT,
        "browser_session_created",
    );
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&session, request_uses_https(&headers)))
            .expect("generated session cookie is a header value"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub async fn csrf_token(
    Extension(auth): Extension<AuthState>,
    Extension(session): Extension<BrowserSessionAuthentication>,
) -> Response {
    match auth.csrf_for_browser_session(&session) {
        Ok((csrf_token, expires_at)) => no_store(Json(CsrfTokenResponse {
            csrf_token,
            expires_at: unix_timestamp(expires_at),
        })),
        Err(BrowserSessionError::Store(message)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &message,
        ),
        Err(_) => error_response(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "The browser session is not valid.",
        ),
    }
}

pub async fn logout_browser_session(
    Extension(auth): Extension<AuthState>,
    Extension(session): Extension<BrowserSessionAuthentication>,
    headers: HeaderMap,
) -> Response {
    if let Err(BrowserSessionError::Store(message)) =
        auth.revoke_browser_session(&session.session_id)
    {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &message,
        );
    }
    auth.record_browser_audit(
        "browser-session",
        StatusCode::NO_CONTENT,
        "browser_session_revoked",
    );
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cleared_session_cookie(request_uses_https(&headers)))
            .expect("generated session cookie is a header value"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn parse_role(role: &str) -> Option<CredentialRole> {
    match role {
        "admin" => Some(CredentialRole::Admin),
        "guest" => Some(CredentialRole::Guest),
        "named" => Some(CredentialRole::Named),
        _ => None,
    }
}

fn unix_timestamp(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn no_store(body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
