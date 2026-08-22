//! Durable named-token administration: the one place that can issue or
//! revoke bearer credentials is an already-authenticated admin request.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Extension, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use msc_api::dto::PermissionCategoryDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::{
    AuthState, AuthenticatedCredential, CredentialAdminError, CredentialExpiryUpdate,
    CredentialRole, CredentialSummary, all_permissions, role_to_string,
};
use crate::routes::lifecycle::{error_response, invalid_body};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateUserRequest {
    label: String,
    role: String,
    permissions: Option<Vec<String>>,
    expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUserRequest {
    user_id: String,
    label: Option<String>,
    role: Option<String>,
    permissions: Option<Vec<String>>,
    expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevokeUserRequest {
    user_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSummaryResponse {
    id: String,
    label: String,
    role: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    permissions: Vec<PermissionCategoryDto>,
    #[serde(rename = "createdAtISO8601")]
    created_at_iso8601: String,
    #[serde(rename = "expiresAtISO8601", skip_serializing_if = "Option::is_none")]
    expires_at_iso8601: Option<String>,
    is_expired: bool,
}

#[derive(Debug, Serialize)]
struct UserListResponse {
    users: Vec<UserSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct UserCreateResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<UserSummaryResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct UserUpdateResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<UserSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct UserRevokeResponse {
    success: bool,
    message: String,
}

pub async fn list(
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<AuthState>,
) -> Response {
    if let Some(response) = require_admin(&credential) {
        return response;
    }
    let users = auth
        .list_credentials()
        .into_iter()
        .map(UserSummaryResponse::from)
        .collect();
    Json(UserListResponse { users }).into_response()
}

pub async fn create(
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<AuthState>,
    body: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_admin(&credential) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let label = request.label.trim().to_string();
    if label.is_empty() {
        return invalid_body("label_empty", "Token label must not be blank.");
    }
    let role = match parse_role(&request.role) {
        Some(role) => role,
        None => return invalid_body_with_status(StatusCode::UNPROCESSABLE_ENTITY, "invalid_role"),
    };
    let permissions = match parse_permissions(role, request.permissions) {
        Ok(permissions) => permissions,
        Err(()) => {
            return invalid_body_with_status(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_permissions",
            );
        }
    };
    let expires_at = match expiry_from_days(request.expires_in_days) {
        Ok(expiry) => expiry,
        Err(()) => {
            return invalid_body_with_status(StatusCode::UNPROCESSABLE_ENTITY, "invalid_expiry");
        }
    };

    match auth.create_named_credential(label, role, permissions, expires_at, &credential.label) {
        Ok((user, issued)) => Json(UserCreateResponse {
            success: true,
            message: "created".into(),
            user: Some(user.into()),
            token: Some(issued.token),
        })
        .into_response(),
        Err(error) => admin_error_response(error),
    }
}

pub async fn update(
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<AuthState>,
    body: Result<Json<UpdateUserRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_admin(&credential) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let label = match request.label {
        Some(label) => {
            let label = label.trim().to_string();
            if label.is_empty() {
                return invalid_body("label_empty", "Token label must not be blank.");
            }
            Some(label)
        }
        None => None,
    };
    let role = match request.role {
        Some(role) => match parse_role(&role) {
            Some(role) => Some(role),
            None => {
                return invalid_body_with_status(StatusCode::UNPROCESSABLE_ENTITY, "invalid_role");
            }
        },
        None => None,
    };
    let permissions = match request.permissions {
        Some(permissions) => {
            match parse_permissions(role.unwrap_or(CredentialRole::Named), Some(permissions)) {
                Ok(permissions) => Some(permissions),
                Err(()) => {
                    return invalid_body_with_status(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "invalid_permissions",
                    );
                }
            }
        }
        None => None,
    };
    let expiry = match request.expires_in_days {
        None => CredentialExpiryUpdate::Unchanged,
        Some(days) if days < 0 => CredentialExpiryUpdate::Clear,
        Some(days) => match expiry_from_days(Some(days)) {
            Ok(Some(expiry)) => CredentialExpiryUpdate::Set(expiry),
            Ok(None) | Err(()) => {
                return invalid_body_with_status(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_expiry",
                );
            }
        },
    };

    match auth.update_credential(
        &request.user_id,
        label,
        role,
        permissions,
        expiry,
        &credential.label,
    ) {
        Ok(user) => Json(UserUpdateResponse {
            success: true,
            message: "updated".into(),
            user: Some(user.into()),
        })
        .into_response(),
        Err(error) => admin_error_response(error),
    }
}

pub async fn revoke(
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<AuthState>,
    body: Result<Json<RevokeUserRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_admin(&credential) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    match auth.revoke_credential(&request.user_id, &credential.label) {
        Ok(()) => Json(UserRevokeResponse {
            success: true,
            message: "revoked".into(),
        })
        .into_response(),
        Err(error) => admin_error_response(error),
    }
}

fn require_admin(credential: &AuthenticatedCredential) -> Option<Response> {
    if credential.role == CredentialRole::Admin {
        None
    } else {
        Some(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only an admin credential may manage named users.",
        ))
    }
}

fn parse_role(value: &str) -> Option<CredentialRole> {
    serde_json::from_value(Value::String(value.to_string())).ok()
}

fn parse_permissions(
    role: CredentialRole,
    permissions: Option<Vec<String>>,
) -> Result<Vec<PermissionCategoryDto>, ()> {
    match role {
        CredentialRole::Admin => Ok(all_permissions()),
        CredentialRole::Guest => Ok(Vec::new()),
        CredentialRole::Named => permissions
            .unwrap_or_default()
            .into_iter()
            .map(|permission| serde_json::from_value(Value::String(permission)).map_err(|_| ()))
            .collect(),
    }
}

fn expiry_from_days(days: Option<i64>) -> Result<Option<SystemTime>, ()> {
    let Some(days) = days else { return Ok(None) };
    let seconds = u64::try_from(days)
        .ok()
        .and_then(|days| days.checked_mul(86_400))
        .ok_or(())?;
    SystemTime::now()
        .checked_add(Duration::from_secs(seconds))
        .map(Some)
        .ok_or(())
}

fn admin_error_response(error: CredentialAdminError) -> Response {
    match error {
        CredentialAdminError::NotFound => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Credential was not found.",
        ),
        CredentialAdminError::SecretStore(message) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &message,
        ),
    }
}

fn invalid_body_with_status(status: StatusCode, code: &str) -> Response {
    error_response(status, code, "The named-token request is invalid.")
}

impl From<CredentialSummary> for UserSummaryResponse {
    fn from(summary: CredentialSummary) -> Self {
        Self {
            id: summary.credential_id,
            label: summary.label,
            role: role_to_string(summary.role),
            permissions: summary.permissions,
            created_at_iso8601: iso8601(summary.created_at),
            expires_at_iso8601: summary.expires_at.map(iso8601),
            is_expired: summary.is_expired,
        }
    }
}

fn iso8601(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Howard Hinnant's public-domain civil calendar conversion.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + if m <= 2 { 1 } else { 0 }, m as u32, d as u32)
}
