//! The native Playit account API boundary.
//!
//! Playit's web API is intentionally kept behind this small, synchronous
//! transport.  The application layer owns the setup workflow; this module
//! only knows how to make the provider calls and turn provider responses into
//! stable, secret-free outcomes.  Tests inject a fake transport, so they never
//! contact playit.gg.

use crate::addon_provider::{AddonTransport, TransportError};
use msc_domain::networking::PlayitTunnelKind;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitTunnel {
    pub name: String,
    pub active: bool,
    pub tunnel_type: Option<String>,
    pub protocol_type: Option<String>,
    pub port_type: Option<String>,
    pub origin_type: Option<String>,
    pub agent_id: Option<String>,
    pub local_ip: Option<String>,
    pub local_port: Option<u16>,
    pub assigned_domain: Option<String>,
    pub port_start: Option<u16>,
    pub static_ip4: Option<String>,
}

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

/// Thin typed wrapper over the account, claim, and tunnel endpoints used by
/// MSC 1's native setup flow. Provider-specific response details stay here so
/// the application layer can make inventory decisions without handling JSON.
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

    /// Lists all tunnels visible to the host's read-only agent key. The null
    /// agent filter is intentional: the saved key is the authority, while the
    /// application layer still verifies each named tunnel belongs to the
    /// saved agent before reusing it.
    pub fn list_tunnels(&self, secret: &str) -> Result<Vec<PlayitTunnel>, PlayitApiError> {
        let authorization = format!("Agent-Key {secret}");
        let response = self.request(
            "/tunnels/list",
            json!({"agent_id": Value::Null}),
            Some(&authorization),
        )?;
        ensure_success(&response)?;
        let tunnels = response
            .value
            .get("data")
            .and_then(|data| data.get("tunnels"))
            .and_then(Value::as_array)
            .ok_or(PlayitApiError::InvalidResponse)?;
        tunnels.iter().map(parse_tunnel).collect()
    }

    /// Creates one missing tunnel with the short-lived web session. Java and
    /// Bedrock use the legacy Minecraft shape; voice uses Playit's raw UDP
    /// shape, matching the provider payload MSC 1 sends.
    pub fn create_tunnel(
        &self,
        agent_id: &str,
        kind: PlayitTunnelKind,
        local_port: u16,
        session: &PlayitSession,
    ) -> Result<(), PlayitApiError> {
        let (path, body) = match kind {
            PlayitTunnelKind::Java | PlayitTunnelKind::Bedrock => (
                "/tunnels/create",
                json!({
                    "name": kind.name(),
                    "tunnel_type": kind.tunnel_type(),
                    "port_type": kind.port_type(),
                    "port_count": 1,
                    "enabled": true,
                    "origin": {
                        "type": "agent",
                        "data": {
                            "agent_id": agent_id,
                            "local_ip": "127.0.0.1",
                            "local_port": local_port,
                        }
                    },
                    "alloc": {
                        "type": "region",
                        "details": {"region": "global"}
                    }
                }),
            ),
            PlayitTunnelKind::Voice => (
                "/v1/tunnels/create",
                json!({
                    "name": kind.name(),
                    "enabled": true,
                    "protocol": {
                        "type": "raw-ports",
                        "details": {
                            "port_type": "udp",
                            "port_count": 1,
                            "software_description": "simple voice chat"
                        }
                    },
                    "endpoint": {
                        "type": "region",
                        "details": {"region": "global", "port": Value::Null}
                    },
                    "origin": {
                        "type": "agent",
                        "data": {
                            "agent_id": agent_id,
                            "config": {
                                "fields": [
                                    {"name": "local_ip", "value": "127.0.0.1"},
                                    {"name": "local_port", "value": local_port.to_string()}
                                ]
                            }
                        }
                    }
                }),
            ),
        };
        let response = self.request_with_session(path, body, session)?;
        ensure_success(&response)
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
    } else if text.contains("agentnotfound")
        || text.contains("agentversiontooold")
        || text.contains("agentoffline")
    {
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

fn parse_tunnel(value: &Value) -> Result<PlayitTunnel, PlayitApiError> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or(PlayitApiError::InvalidResponse)?
        .to_owned();
    let origin = value.get("origin");
    let origin_data = origin.and_then(|origin| origin.get("data"));
    let protocol = value.get("protocol");
    let protocol_details = protocol.and_then(|protocol| protocol.get("details"));
    let alloc = value.get("alloc");
    let alloc_data = alloc
        .and_then(|alloc| alloc.get("data"))
        .or_else(|| alloc.and_then(|alloc| alloc.get("details")))
        .or(alloc);
    let fields = origin_data
        .and_then(|data| data.get("config"))
        .and_then(|config| config.get("fields"))
        .and_then(Value::as_array);

    Ok(PlayitTunnel {
        name,
        active: value
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        tunnel_type: value.get("tunnel_type").and_then(value_string),
        protocol_type: value
            .get("protocol_type")
            .and_then(value_string)
            .or_else(|| {
                protocol
                    .and_then(|protocol| protocol.get("type"))
                    .and_then(value_string)
            }),
        port_type: value.get("port_type").and_then(value_string).or_else(|| {
            protocol_details
                .and_then(|details| details.get("port_type"))
                .and_then(value_string)
        }),
        origin_type: origin
            .and_then(|origin| origin.get("type"))
            .and_then(value_string),
        agent_id: origin_data
            .and_then(|data| data.get("agent_id"))
            .and_then(value_string),
        local_ip: origin_data
            .and_then(|data| data.get("local_ip"))
            .and_then(value_string)
            .or_else(|| field_value(fields, "local_ip")),
        local_port: origin_data
            .and_then(|data| data.get("local_port"))
            .and_then(value_u16)
            .or_else(|| field_value(fields, "local_port").and_then(|value| value.parse().ok())),
        assigned_domain: alloc_data
            .and_then(|alloc| alloc.get("assigned_domain"))
            .and_then(value_string),
        port_start: alloc_data
            .and_then(|alloc| alloc.get("port_start"))
            .and_then(value_u16),
        static_ip4: alloc_data
            .and_then(|alloc| alloc.get("static_ip4"))
            .and_then(value_string),
    })
}

fn value_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn value_u16(value: &Value) -> Option<u16> {
    value
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .or_else(|| value.as_str().and_then(|value| value.trim().parse().ok()))
}

fn field_value(fields: Option<&Vec<Value>>, name: &str) -> Option<String> {
    fields?
        .iter()
        .find(|field| field.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|field| field.get("value"))
        .and_then(value_string)
}
