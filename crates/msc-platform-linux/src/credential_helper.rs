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

#[cfg(target_os = "linux")]
const SYSTEMD_LISTEN_FDS_START: i32 = 3;

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
        // render_service_unit hardens the helper with PrivateTmp=yes and
        // ProtectHome=yes. Both are intentional and must not be relaxed --
        // but that means a binary placed under /tmp, /var/tmp, /home,
        // /root, or /run/user is invisible to the unit's own ExecStart:
        // the service can't see its own executable and fails 203/EXEC at
        // startup. Reject that up front instead of installing a unit that
        // silently can't start.
        const HIDDEN_BY_OWN_HARDENING: &[&str] =
            &["/tmp", "/var/tmp", "/home", "/root", "/run/user"];
        if let Some(hidden_root) = HIDDEN_BY_OWN_HARDENING
            .iter()
            .find(|root| self.binary_path.starts_with(root))
        {
            return Err(format!(
                "credential-helper binary path {} is under {hidden_root}, which the helper's own PrivateTmp=yes/ProtectHome=yes hardening hides from it at runtime",
                self.binary_path.display()
            ));
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
            "[Unit]\nDescription=MSC 2 credential helper\nRequires={SOCKET_UNIT_NAME}\nAfter=network.target\n\n[Service]\nType=simple\nUser=root\nGroup=root\nExecStart={} credential-helper serve --allowed-uid {} --store-dir {}\nNoNewPrivileges=yes\nPrivateTmp=yes\nProtectSystem=strict\nProtectHome=yes\nReadWritePaths={}\nRuntimeDirectory=msc2\n\n[Install]\nWantedBy=multi-user.target\n",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[cfg(target_os = "linux")]
pub struct HelperClient {
    socket_path: PathBuf,
}

#[cfg(target_os = "linux")]
impl HelperClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub fn default_socket() -> Self {
        Self::new(DEFAULT_SOCKET_PATH)
    }

    pub fn ping(&self) -> Result<(), String> {
        self.request(&HelperRequest {
            version: PROTOCOL_VERSION,
            op: HelperOperation::Ping,
            key: None,
            value: None,
        })
        .map(|_| ())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let response = self.request(&HelperRequest {
            version: PROTOCOL_VERSION,
            op: HelperOperation::Get,
            key: Some(key.to_string()),
            value: None,
        })?;
        Ok(response.value)
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.request(&HelperRequest {
            version: PROTOCOL_VERSION,
            op: HelperOperation::Set,
            key: Some(key.to_string()),
            value: Some(value.to_string()),
        })
        .map(|_| ())
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        self.request(&HelperRequest {
            version: PROTOCOL_VERSION,
            op: HelperOperation::Delete,
            key: Some(key.to_string()),
            value: None,
        })
        .map(|_| ())
    }

    fn request(&self, request: &HelperRequest) -> Result<HelperResponse, String> {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;

        validate_request(request)?;
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|err| format!("connecting {}: {err}", self.socket_path.display()))?;
        let line = serde_json::to_string(request)
            .map_err(|err| format!("serializing helper request: {err}"))?;
        stream
            .write_all(line.as_bytes())
            .and_then(|_| stream.write_all(b"\n"))
            .map_err(|err| format!("writing helper request: {err}"))?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .map_err(|err| format!("reading helper response: {err}"))?;
        if response.len() > MAX_REQUEST_BYTES {
            return Err(format!("helper response exceeds {MAX_REQUEST_BYTES} bytes"));
        }
        let response: HelperResponse = serde_json::from_str(response.trim_end_matches('\n'))
            .map_err(|err| format!("decoding helper response: {err}"))?;
        if response.ok {
            Ok(response)
        } else {
            let message = response
                .error
                .as_ref()
                .map(|err| format!("{}: {}", err.code, err.message))
                .unwrap_or_else(|| "helper returned an unspecified error".to_string());
            Err(message)
        }
    }
}

#[cfg(target_os = "linux")]
pub trait HelperBackend {
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

#[cfg(target_os = "linux")]
pub struct SystemdCredsStore {
    store_dir: PathBuf,
    systemd_creds_path: PathBuf,
}

#[cfg(target_os = "linux")]
impl SystemdCredsStore {
    pub fn new(store_dir: impl Into<PathBuf>) -> Self {
        Self {
            store_dir: store_dir.into(),
            systemd_creds_path: PathBuf::from("systemd-creds"),
        }
    }

    pub fn systemd_creds_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.systemd_creds_path = path.into();
        self
    }

    fn path_for_key(&self, key: &str) -> Result<PathBuf, String> {
        validate_key(key)?;
        Ok(self.store_dir.join(format!("{key}.cred")))
    }

    fn ensure_store_dir(&self) -> Result<(), String> {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fs::create_dir_all(&self.store_dir)
            .map_err(|err| format!("creating {}: {err}", self.store_dir.display()))?;
        fs::set_permissions(&self.store_dir, fs::Permissions::from_mode(0o700))
            .map_err(|err| format!("setting mode on {}: {err}", self.store_dir.display()))
    }
}

#[cfg(target_os = "linux")]
impl HelperBackend for SystemdCredsStore {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        use std::fs;
        use std::io::ErrorKind;
        use std::process::Command;

        let path = self.path_for_key(key)?;
        match fs::metadata(&path) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(format!("reading {}: {err}", path.display())),
        }

        let output = Command::new(&self.systemd_creds_path)
            .arg("decrypt")
            .arg(format!("--name={key}"))
            .arg(&path)
            .arg("-")
            .output()
            .map_err(|err| format!("running systemd-creds decrypt: {err}"))?;
        if !output.status.success() {
            return Err(command_error("systemd-creds decrypt", &output.stderr));
        }
        String::from_utf8(output.stdout)
            .map(Some)
            .map_err(|err| format!("decrypted credential is not UTF-8: {err}"))
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        use std::fs;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};
        use std::time::{SystemTime, UNIX_EPOCH};

        let path = self.path_for_key(key)?;
        self.ensure_store_dir()?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| format!("system clock before epoch: {err}"))?
            .as_nanos();
        let tmp_path = self
            .store_dir
            .join(format!(".{key}.{}.{}.tmp", std::process::id(), nonce));

        let mut child = Command::new(&self.systemd_creds_path)
            .arg("encrypt")
            .arg(format!("--name={key}"))
            .arg("-")
            .arg(&tmp_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("running systemd-creds encrypt: {err}"))?;
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "systemd-creds stdin was not captured".to_string())?;
        stdin
            .write_all(value.as_bytes())
            .map_err(|err| format!("writing plaintext to systemd-creds: {err}"))?;
        let output = child
            .wait_with_output()
            .map_err(|err| format!("waiting for systemd-creds encrypt: {err}"))?;
        if !output.status.success() {
            let _ = fs::remove_file(&tmp_path);
            return Err(command_error("systemd-creds encrypt", &output.stderr));
        }

        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
            .map_err(|err| format!("setting mode on {}: {err}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path).map_err(|err| {
            let _ = fs::remove_file(&tmp_path);
            format!(
                "renaming {} to {}: {err}",
                tmp_path.display(),
                path.display()
            )
        })
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        use std::fs;
        use std::io::ErrorKind;

        let path = self.path_for_key(key)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("deleting {}: {err}", path.display())),
        }
    }
}

#[cfg(target_os = "linux")]
fn command_error(command: &str, stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {stderr}")
    }
}

#[cfg(target_os = "linux")]
pub fn run_helper_service(
    allowed_uid: u32,
    store_dir: impl Into<PathBuf>,
    socket_path: Option<PathBuf>,
) -> Result<(), String> {
    let backend = SystemdCredsStore::new(store_dir);
    if let Some(socket_path) = socket_path {
        return serve_bound_socket(allowed_uid, &backend, &socket_path);
    }
    if let Some(listener) = inherited_systemd_listener()? {
        return serve_listener(allowed_uid, &backend, listener);
    }
    serve_stdio_connection(allowed_uid, &backend)
}

#[cfg(target_os = "linux")]
fn serve_bound_socket<B: HelperBackend>(
    allowed_uid: u32,
    backend: &B,
    socket_path: &Path,
) -> Result<(), String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("creating {}: {err}", parent.display()))?;
    }
    if socket_path.exists() {
        return Err(format!(
            "socket path already exists: {}",
            socket_path.display()
        ));
    }
    let listener = UnixListener::bind(socket_path)
        .map_err(|err| format!("binding {}: {err}", socket_path.display()))?;
    chown_socket_to_allowed_uid(socket_path, allowed_uid)?;
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))
        .map_err(|err| format!("setting mode on {}: {err}", socket_path.display()))?;
    serve_listener(allowed_uid, backend, listener)
}

#[cfg(target_os = "linux")]
fn chown_socket_to_allowed_uid(socket_path: &Path, allowed_uid: u32) -> Result<(), String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(socket_path.as_os_str().as_bytes())
        .map_err(|_| format!("socket path contains NUL: {}", socket_path.display()))?;
    // SAFETY: path is a valid NUL-terminated C string. Passing gid -1
    // asks chown to leave the current group unchanged.
    let rc = unsafe { libc::chown(path.as_ptr(), allowed_uid, u32::MAX) };
    if rc == -1 {
        return Err(format!(
            "changing owner of {}: {}",
            socket_path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inherited_systemd_listener() -> Result<Option<std::os::unix::net::UnixListener>, String> {
    use std::os::fd::FromRawFd;
    use std::os::unix::net::UnixListener;

    let listen_pid = std::env::var("LISTEN_PID").ok();
    let listen_fds = std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    if listen_fds < 1 {
        return Ok(None);
    }
    if listen_pid.as_deref() != Some(&std::process::id().to_string()) {
        return Ok(None);
    }

    // SAFETY: systemd socket activation passes the first listening socket
    // at fd 3 and transfers ownership to the service process.
    let listener = unsafe { UnixListener::from_raw_fd(SYSTEMD_LISTEN_FDS_START) };
    Ok(Some(listener))
}

#[cfg(target_os = "linux")]
fn serve_listener<B: HelperBackend>(
    allowed_uid: u32,
    backend: &B,
    listener: std::os::unix::net::UnixListener,
) -> Result<(), String> {
    for stream in listener.incoming() {
        let stream = stream.map_err(|err| format!("accepting helper connection: {err}"))?;
        handle_stream(allowed_uid, backend, stream)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn handle_stream<B: HelperBackend>(
    allowed_uid: u32,
    backend: &B,
    mut stream: std::os::unix::net::UnixStream,
) -> Result<(), String> {
    use std::io::BufReader;
    use std::os::fd::AsRawFd;

    let uid = peer_uid(stream.as_raw_fd())?;
    if uid != allowed_uid {
        write_response(
            &mut stream,
            &HelperResponse::error(
                "forbidden_uid",
                format!("peer uid {uid} is not allowed to use this helper"),
            ),
        )?;
        return Ok(());
    }

    let reader_stream = stream
        .try_clone()
        .map_err(|err| format!("cloning helper stream: {err}"))?;
    let response = handle_connection(BufReader::new(reader_stream), backend);
    write_response(&mut stream, &response)
}

#[cfg(target_os = "linux")]
fn serve_stdio_connection<B: HelperBackend>(allowed_uid: u32, backend: &B) -> Result<(), String> {
    use std::io::{BufReader, stdin, stdout};
    use std::os::fd::AsRawFd;

    let stdin = stdin();
    let uid = peer_uid(stdin.as_raw_fd())?;
    let mut stdout = stdout();
    if uid != allowed_uid {
        return write_response(
            &mut stdout,
            &HelperResponse::error(
                "forbidden_uid",
                format!("peer uid {uid} is not allowed to use this helper"),
            ),
        );
    }
    let response = handle_connection(BufReader::new(stdin.lock()), backend);
    write_response(&mut stdout, &response)
}

#[cfg(target_os = "linux")]
fn peer_uid(fd: std::os::fd::RawFd) -> Result<u32, String> {
    let mut cred = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `cred` points to valid writable storage for `ucred`, and
    // `len` carries the correct buffer size for SO_PEERCRED.
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            cred.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if rc == -1 {
        return Err(format!(
            "reading peer credentials: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: getsockopt returned success, so the kernel initialized cred.
    Ok(unsafe { cred.assume_init().uid })
}

#[cfg(target_os = "linux")]
pub fn handle_connection<R: std::io::BufRead, B: HelperBackend>(
    mut reader: R,
    backend: &B,
) -> HelperResponse {
    let mut line = Vec::new();
    match reader.read_until(b'\n', &mut line) {
        Ok(0) => {
            return HelperResponse::error("empty_request", "helper request was empty");
        }
        Ok(_) => {}
        Err(err) => {
            return HelperResponse::error("read_failed", format!("reading helper request: {err}"));
        }
    }
    if line.len() > MAX_REQUEST_BYTES {
        return HelperResponse::error(
            "request_too_large",
            format!("request exceeds {MAX_REQUEST_BYTES} bytes"),
        );
    }
    let line = match std::str::from_utf8(&line) {
        Ok(line) => line.trim_end_matches('\n'),
        Err(err) => {
            return HelperResponse::error("invalid_utf8", format!("request is not UTF-8: {err}"));
        }
    };
    let request = match parse_request_line(line) {
        Ok(request) => request,
        Err(err) => return HelperResponse::error("invalid_request", err),
    };
    handle_request(request, backend)
}

#[cfg(target_os = "linux")]
fn handle_request<B: HelperBackend>(request: HelperRequest, backend: &B) -> HelperResponse {
    match request.op {
        HelperOperation::Ping => HelperResponse::ok(),
        HelperOperation::Get => match backend.get(request.key.as_deref().unwrap_or_default()) {
            Ok(value) => HelperResponse::ok_with_value(value),
            Err(err) => HelperResponse::error("secret_store", err),
        },
        HelperOperation::Set => match backend.set(
            request.key.as_deref().unwrap_or_default(),
            request.value.as_deref().unwrap_or_default(),
        ) {
            Ok(()) => HelperResponse::ok(),
            Err(err) => HelperResponse::error("secret_store", err),
        },
        HelperOperation::Delete => match backend.delete(request.key.as_deref().unwrap_or_default())
        {
            Ok(()) => HelperResponse::ok(),
            Err(err) => HelperResponse::error("secret_store", err),
        },
    }
}

#[cfg(target_os = "linux")]
fn write_response<W: std::io::Write>(
    writer: &mut W,
    response: &HelperResponse,
) -> Result<(), String> {
    let line = serialize_response(response)?;
    writer
        .write_all(line.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|err| format!("writing helper response: {err}"))
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryBackend {
        values: Mutex<HashMap<String, String>>,
    }

    impl HelperBackend for MemoryBackend {
        fn get(&self, key: &str) -> Result<Option<String>, String> {
            Ok(self.values.lock().unwrap().get(key).cloned())
        }

        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            self.values
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn delete(&self, key: &str) -> Result<(), String> {
            self.values.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn helper_connection_sets_gets_overwrites_and_deletes() {
        let backend = MemoryBackend::default();
        let set = handle_connection(
            br#"{"version":1,"op":"set","key":"remote-api.token.abc","value":"first"}
"#
            .as_slice(),
            &backend,
        );
        assert!(set.ok);

        let get = handle_connection(
            br#"{"version":1,"op":"get","key":"remote-api.token.abc"}
"#
            .as_slice(),
            &backend,
        );
        assert_eq!(get.value.as_deref(), Some("first"));

        let overwrite = handle_connection(
            br#"{"version":1,"op":"set","key":"remote-api.token.abc","value":"second"}
"#
            .as_slice(),
            &backend,
        );
        assert!(overwrite.ok);

        let get = handle_connection(
            br#"{"version":1,"op":"get","key":"remote-api.token.abc"}
"#
            .as_slice(),
            &backend,
        );
        assert_eq!(get.value.as_deref(), Some("second"));

        let delete = handle_connection(
            br#"{"version":1,"op":"delete","key":"remote-api.token.abc"}
"#
            .as_slice(),
            &backend,
        );
        assert!(delete.ok);

        let get = handle_connection(
            br#"{"version":1,"op":"get","key":"remote-api.token.abc"}
"#
            .as_slice(),
            &backend,
        );
        assert_eq!(get.value, None);
    }

    #[test]
    fn helper_connection_rejects_bad_keys_before_backend() {
        let backend = MemoryBackend::default();
        let response = handle_connection(
            br#"{"version":1,"op":"set","key":"../bad","value":"secret"}
"#
            .as_slice(),
            &backend,
        );
        assert!(!response.ok);
        assert_eq!(
            response.error.as_ref().map(|err| err.code.as_str()),
            Some("invalid_request")
        );
    }

    #[test]
    fn helper_response_for_unset_key_deserializes_without_value_field() {
        // `serialize_response` omits `value` entirely (skip_serializing_if)
        // when a key is unset, so the wire form is `{"ok":true}` with no
        // `value` key at all -- not `{"ok":true,"value":null}`. A caller on
        // the other end of the socket (HelperClient::get, used in production
        // by LinuxCredentialHelperSecretStore) must still deserialize that
        // into `value: None` rather than erroring on a missing field.
        let response: HelperResponse = serde_json::from_str(r#"{"ok":true}"#).unwrap();
        assert_eq!(response, HelperResponse::ok());
    }
}
