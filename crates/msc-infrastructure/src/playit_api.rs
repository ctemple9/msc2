//! The native Playit account API boundary.
//!
//! Playit's web API is intentionally kept behind this small, synchronous
//! transport.  The application layer owns the setup workflow; this module
//! only knows how to make the provider calls and turn provider responses into
//! stable, secret-free outcomes.  Tests inject a fake transport, so they never
//! contact playit.gg.

use crate::addon_provider::{AddonTransport, TransportError};
use serde_json::{Value, json};
use std::fmt;

pub const PLAYIT_API_BASE_URL: &str = "https://api.playit.gg";
pub const PLAYIT_AGENT_VERSION: &str = "playit 1.0.10";
pub const PLAYIT_AGENT_NAME: &str = "MSC Agent";
pub const PLAYIT_API_MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

/// The only provider transport behavior the account workflow needs.  The
/// authorization value is borrowed and is never retained by the real
/// transport, which keeps the temporary session inside one setup call.
pub trait PlayitHttpTransport: Send + Sync {
    fn post_json(
        &self,
        path: &str,
        body: &Value,
        authorization: Option<&str>,
    ) -> Result<PlayitHttpResponse, PlayitTransportError>;
}

#[derive(Debug)]
pub struct PlayitHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayitTransportError {
    Network,
    Timeout,
    ResponseTooLarge,
}

impl fmt::Display for PlayitTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network => write!(f, "could not reach the Playit service"),
            Self::Timeout => write!(f, "the Playit service did not respond in time"),
            Self::ResponseTooLarge => {
                write!(f, "the Playit service returned an oversized response")
            }
        }
    }
}

impl std::error::Error for PlayitTransportError {}

/// Reuse the existing bounded `ureq` implementation without giving the
/// account client access to any other provider's response semantics.
impl PlayitHttpTransport for crate::addon_provider::HttpTransport {
    fn post_json(
        &self,
        path: &str,
        body: &Value,
        authorization: Option<&str>,
    ) -> Result<PlayitHttpResponse, PlayitTransportError> {
        let url = format!("{PLAYIT_API_BASE_URL}{path}");
        let headers = authorization
            .map(|value| vec![("Authorization", value)])
            .unwrap_or_default();
        let response = AddonTransport::post_json(
            self,
            &url,
            path,
            body,
            &headers,
            PLAYIT_API_MAX_RESPONSE_BYTES,
        )
        .map_err(map_transport_error)?;
        Ok(PlayitHttpResponse {
            status: response.status,
            body: response.body,
        })
    }
}

fn map_transport_error(error: TransportError) -> PlayitTransportError {
    match error {
        TransportError::Network(_) => PlayitTransportError::Network,
        TransportError::Timeout(_) => PlayitTransportError::Timeout,
        TransportError::ResponseTooLarge { .. } => PlayitTransportError::ResponseTooLarge,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayitApiError {
    IncorrectCredentials,
    AccountBanned,
    TwoFactorRequired,
    RateLimited,
    AgentNotFound,
    ApiFailure,
    TransportFailure,
    InvalidResponse,
}

impl PlayitApiError {
    pub fn stable_code(self) -> &'static str {
        match self {
            Self::IncorrectCredentials => "incorrect_credentials",
            Self::AccountBanned => "account_banned",
            Self::TwoFactorRequired => "two_factor_required",
            Self::RateLimited => "rate_limited",
            Self::AgentNotFound => "agent_not_found",
            Self::ApiFailure | Self::TransportFailure | Self::InvalidResponse => "playit_api_error",
        }
    }
}

impl fmt::Display for PlayitApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::IncorrectCredentials => "Playit rejected the supplied sign-in details.",
            Self::AccountBanned => "The Playit account has been banned.",
            Self::TwoFactorRequired => {
                "This Playit account requires two-factor authentication, which MSC cannot complete yet."
            }
            Self::RateLimited => "Playit temporarily rate-limited this request.",
            Self::AgentNotFound => "Playit could not find the tunnel agent.",
            Self::ApiFailure => "Playit returned an error while setting up the tunnel agent.",
            Self::TransportFailure => {
                "MSC could not reach Playit while setting up the tunnel agent."
            }
            Self::InvalidResponse => "Playit returned an unexpected response.",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for PlayitApiError {}

/// A session key is intentionally opaque.  It has no `Debug`, `Display`, or
/// serialization implementation, so an operation record cannot accidentally
/// include it.  Clearing the owned string on drop makes the lifetime explicit;
/// the provider itself remains responsible for expiring the server-side session.
pub struct PlayitSession(String);

impl PlayitSession {
    fn new(value: String) -> Self {
        Self(value)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl Drop for PlayitSession {
    fn drop(&mut self) {
        self.0.clear();
    }
}

/// The permanent host-scoped agent key is returned only to the application
/// service so that it can be written to `SecretStore`; it is never placed in
/// an operation result or response DTO.
pub struct PlayitSecret(String);

impl PlayitSecret {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Drop for PlayitSecret {
    fn drop(&mut self) {
        self.0.clear();
    }
}

/// Thin typed wrapper over the account and claim endpoints used by MSC 1's
/// native `setupPlayitViaSignin` flow.  Tunnel inventory and creation are
/// deliberately left to the following Playit step.
pub struct PlayitApi<'transport> {
    transport: &'transport dyn PlayitHttpTransport,
}

impl<'transport> PlayitApi<'transport> {
    pub fn new(transport: &'transport dyn PlayitHttpTransport) -> Self {
        Self { transport }
    }

    pub fn sign_in(&self, email: &str, password: &str) -> Result<PlayitSession, PlayitApiError> {
        let response = self.request(
            "/login/signin",
            json!({
                "email": email,
                "password": password,
            }),
            None,
        )?;
        ensure_success(&response)?;
        let session_key = response
            .value
            .get("data")
            .and_then(|data| data.get("session_key"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(PlayitApiError::InvalidResponse)?;
        Ok(PlayitSession::new(session_key.to_owned()))
    }

    pub fn claim_setup(&self, code: &str) -> Result<(), PlayitApiError> {
        let response = self.request(
            "/claim/setup",
            json!({
                "code": code,
                "agent_type": "assignable",
                "version": PLAYIT_AGENT_VERSION,
            }),
            None,
        )?;
        ensure_success(&response)
    }

    pub fn claim_details(&self, code: &str, session: &PlayitSession) -> Result<(), PlayitApiError> {
        let response =
            self.request_with_session("/claim/details", json!({"code": code}), session)?;
        ensure_success(&response)
    }

    pub fn claim_accept(
        &self,
        code: &str,
        name: &str,
        session: &PlayitSession,
    ) -> Result<String, PlayitApiError> {
        let response = self.request_with_session(
            "/claim/accept",
            json!({
                "code": code,
                "name": name,
                "agent_type": "assignable",
            }),
            session,
        )?;
        ensure_success(&response)?;
        response
            .value
            .get("data")
            .and_then(|data| data.get("agent_id"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or(PlayitApiError::InvalidResponse)
    }

    /// Exchanges an accepted claim for the permanent key.  A provider-side
    /// pending response is normal while the claim propagates, so it is
    /// represented as `Ok(None)` rather than an error.
    pub fn claim_exchange(&self, code: &str) -> Result<Option<PlayitSecret>, PlayitApiError> {
        let response = self.request("/claim/exchange", json!({"code": code}), None)?;
        if !is_success(&response) {
            if is_pending_claim(&response.value) {
                return Ok(None);
            }
            return Err(classify_failure(response.status, &response.value));
        }
        let key = response
            .value
            .get("data")
            .and_then(|data| data.get("secret_key"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(PlayitApiError::InvalidResponse)?;
        Ok(Some(PlayitSecret(key.to_owned())))
    }

    fn request(
        &self,
        path: &str,
        body: Value,
        authorization: Option<&str>,
    ) -> Result<PlayitResponse, PlayitApiError> {
        let response = self
            .transport
            .post_json(path, &body, authorization)
            .map_err(|_| PlayitApiError::TransportFailure)?;
        let value =
            serde_json::from_slice(&response.body).map_err(|_| PlayitApiError::InvalidResponse)?;
        Ok(PlayitResponse {
            status: response.status,
            value,
        })
    }

    /// MSC 1 observed four authorization header schemes in the provider API.
    /// Keep that compatibility behavior inside the agent and never expose the
    /// temporary session to the caller or to diagnostics.
    fn request_with_session(
        &self,
        path: &str,
        body: Value,
        session: &PlayitSession,
    ) -> Result<PlayitResponse, PlayitApiError> {
        let authorization_values = [
            format!("session {}", session.as_str()),
            format!("agent-key {}", session.as_str()),
            format!("Bearer {}", session.as_str()),
            session.as_str().to_owned(),
        ];
        for authorization in authorization_values {
            let response = self.request(path, body.clone(), Some(&authorization))?;
            if is_auth_failure(&response.value) {
                continue;
            }
            return Ok(response);
        }
        Err(PlayitApiError::ApiFailure)
    }
}

struct PlayitResponse {
    status: u16,
    value: Value,
}

fn is_success(response: &PlayitResponse) -> bool {
    (200..300).contains(&response.status)
        && response.value.get("status").and_then(Value::as_str) == Some("success")
}

fn ensure_success(response: &PlayitResponse) -> Result<(), PlayitApiError> {
    if is_success(response) {
        Ok(())
    } else {
        Err(classify_failure(response.status, &response.value))
    }
}

fn classify_failure(status: u16, value: &Value) -> PlayitApiError {
    if status == 429 {
        return PlayitApiError::RateLimited;
    }
    let text = provider_error_text(value);
    let text = text.to_ascii_lowercase().replace(['_', '-', ' '], "");
    if text.contains("incorrectcredentials")
        || text.contains("invalidcredentials")
        || text.contains("invalidemailorpassword")
    {
        PlayitApiError::IncorrectCredentials
    } else if text.contains("accountbanned") || text.contains("banned") {
        PlayitApiError::AccountBanned
    } else if text.contains("totprequired")
        || text.contains("2farequired")
        || text.contains("twofactorrequired")
    {
        PlayitApiError::TwoFactorRequired
    } else if text.contains("ratelimit") || text.contains("toomanyrequests") {
        PlayitApiError::RateLimited
    } else if text.contains("agentnotfound") {
        PlayitApiError::AgentNotFound
    } else {
        PlayitApiError::ApiFailure
    }
}

fn provider_error_text(value: &Value) -> String {
    let mut values = Vec::new();
    collect_provider_text(value, &mut values);
    values.join(" ")
}

fn collect_provider_text(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(value) => output.push(value.clone()),
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect_provider_text(value, output)),
        Value::Object(values) => values
            .iter()
            .filter(|(key, _)| {
                matches!(
                    key.as_str(),
                    "status" | "data" | "type" | "message" | "code" | "error"
                )
            })
            .for_each(|(_, value)| collect_provider_text(value, output)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn is_auth_failure(value: &Value) -> bool {
    value
        .get("data")
        .and_then(|data| data.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("auth"))
}

fn is_pending_claim(value: &Value) -> bool {
    let text = provider_error_text(value)
        .to_ascii_lowercase()
        .replace(['_', '-', ' '], "");
    text.contains("pending")
        || text.contains("notready")
        || text.contains("waitingfor")
        || text.contains("notaccepted")
}
