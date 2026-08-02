//! Linux privileged credential-helper definitions for Phase 4.
//!
//! The helper runs as a root-owned `systemd` service with socket
//! activation. The agent itself stays unprivileged and talks to the socket
//! as the installing user; peer-credential checks make that user id the
//! actual authority.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SOCKET_UNIT_NAME: &str = "msc2-credential-helper.socket";
pub const SERVICE_UNIT_NAME: &str = "msc2-credential-helper.service";
pub const DEFAULT_SOCKET_PATH: &str = "/run/msc2/credential-helper.sock";
pub const DEFAULT_STORE_DIR: &str = "/var/lib/msc2/credentials";
pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_VALUE_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialHelperInstall {
    pub binary_path: PathBuf,
    pub allowed_uid: u32,
    pub socket_user: String,
    pub socket_group: String,
    pub socket_path: PathBuf,
    pub store_dir: PathBuf,
}

impl CredentialHelperInstall {
    pub fn new(
        binary_path: impl Into<PathBuf>,
        allowed_uid: u32,
        socket_user: impl Into<String>,
        socket_group: impl Into<String>,
    ) -> Self {
        Self {
            binary_path: binary_path.into(),
            allowed_uid,
            socket_user: socket_user.into(),
            socket_group: socket_group.into(),
            socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
            store_dir: PathBuf::from(DEFAULT_STORE_DIR),
        }
    }

    pub fn socket_path(mut self, socket_path: impl Into<PathBuf>) -> Self {
        self.socket_path = socket_path.into();
        self
    }

    pub fn store_dir(mut self, store_dir: impl Into<PathBuf>) -> Self {
        self.store_dir = store_dir.into();
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.binary_path.is_absolute() {
            return Err(format!(
                "credential-helper binary path must be absolute: {}",
                self.binary_path.display()
            ));
        }
        if !self.socket_path.is_absolute() {
            return Err(format!(
                "credential-helper socket path must be absolute: {}",
                self.socket_path.display()
            ));
        }
        if !self.store_dir.is_absolute() {
            return Err(format!(
                "credential-helper store dir must be absolute: {}",
                self.store_dir.display()
            ));
        }
        if self.socket_user.trim().is_empty() {
            return Err("credential-helper socket user cannot be empty".to_string());
        }
        if self.socket_group.trim().is_empty() {
            return Err("credential-helper socket group cannot be empty".to_string());
        }
        Ok(())
    }

    pub fn render_socket_unit(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!(
            "[Unit]\nDescription=MSC 2 credential helper socket\n\n[Socket]\nListenStream={}\nSocketUser={}\nSocketGroup={}\nSocketMode=0600\nRemoveOnStop=yes\n\n[Install]\nWantedBy=sockets.target\n",
            self.socket_path.display(),
            self.socket_user,
            self.socket_group,
        ))
    }

    pub fn render_service_unit(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!(
            "[Unit]\nDescription=MSC 2 credential helper\nRequires={SOCKET_UNIT_NAME}\nAfter=network.target\n\n[Service]\nType=simple\nUser=root\nGroup=root\nExecStart={} credential-helper serve --allowed-uid {} --store-dir {}\nStandardInput=socket\nNoNewPrivileges=yes\nPrivateTmp=yes\nProtectSystem=strict\nProtectHome=yes\nReadWritePaths={}\nRuntimeDirectory=msc2\n\n[Install]\nWantedBy=multi-user.target\n",
            shell_quote_path(&self.binary_path),
            self.allowed_uid,
            shell_quote_path(&self.store_dir),
            shell_quote_path(&self.store_dir),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HelperOperation {
    Get,
    Set,
    Delete,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperRequest {
    pub version: u8,
    pub op: HelperOperation,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HelperError>,
}

impl HelperResponse {
    pub fn ok() -> Self {
        Self {
            ok: true,
            value: None,
            error: None,
        }
    }

    pub fn ok_with_value(value: Option<String>) -> Self {
        Self {
            ok: true,
            value,
            error: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            value: None,
            error: Some(HelperError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

pub fn parse_request_line(line: &str) -> Result<HelperRequest, String> {
    if line.len() > MAX_REQUEST_BYTES {
        return Err(format!("request exceeds {MAX_REQUEST_BYTES} bytes"));
    }

    let request: HelperRequest =
        serde_json::from_str(line).map_err(|err| format!("invalid helper JSON: {err}"))?;
    validate_request(&request)?;
    Ok(request)
}

pub fn serialize_response(response: &HelperResponse) -> Result<String, String> {
    serde_json::to_string(response).map_err(|err| format!("serializing helper response: {err}"))
}

pub fn validate_request(request: &HelperRequest) -> Result<(), String> {
    if request.version != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported helper protocol version {}; expected {PROTOCOL_VERSION}",
            request.version
        ));
    }

    match request.op {
        HelperOperation::Ping => {
            if request.key.is_some() || request.value.is_some() {
                return Err("ping does not accept key or value".to_string());
            }
        }
        HelperOperation::Get | HelperOperation::Delete => {
            let key = request
                .key
                .as_deref()
                .ok_or_else(|| "helper request is missing key".to_string())?;
            validate_key(key)?;
            if request.value.is_some() {
                return Err("get/delete do not accept value".to_string());
            }
        }
        HelperOperation::Set => {
            let key = request
                .key
                .as_deref()
                .ok_or_else(|| "helper request is missing key".to_string())?;
            validate_key(key)?;
            let value = request
                .value
                .as_deref()
                .ok_or_else(|| "set request is missing value".to_string())?;
            if value.len() > MAX_VALUE_BYTES {
                return Err(format!("value exceeds {MAX_VALUE_BYTES} bytes"));
            }
        }
    }

    Ok(())
}

pub fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("credential key is not allowed".to_string());
    }
    if key.len() > 192 {
        return Err("credential key is not allowed".to_string());
    }
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err("credential key is not allowed".to_string());
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err("credential key is not allowed".to_string());
    }
    for ch in chars {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '-') {
            return Err("credential key is not allowed".to_string());
        }
    }
    if key.contains("..")
        || key.contains('/')
        || key.contains('\\')
        || key.contains(char::is_whitespace)
    {
        return Err("credential key is not allowed".to_string());
    }
    Ok(())
}

fn shell_quote_path(path: &Path) -> String {
    let escaped = path.display().to_string().replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}
