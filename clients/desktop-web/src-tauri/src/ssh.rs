//! Native, app-owned SSH forwarding for remote MSC hosts.
//!
//! The webview never receives a password, bearer token, or child-process
//! handle. It receives only a small status record and asks this module to
//! start, inspect, retry, or stop a tunnel. OpenSSH remains responsible for
//! the protocol and for private-key/agent authentication; this module owns
//! the process lifecycle and host-key decision boundary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HOST_KEY_SECRET_PREFIX: &str = "msc.desktop.ssh-host-key.";
const SSH_ASKPASS_MARKER: &str = "MSC2_SSH_ASKPASS";
const SSH_ASKPASS_PASSWORD: &str = "MSC2_SSH_ASKPASS_PASSWORD";
const MAX_STDERR_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelRequest {
    pub host_id: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub username: String,
    pub authentication: String,
    pub private_key_path: Option<String>,
    /// Used only during this connection attempt and never written to disk.
    pub password: Option<String>,
    pub local_port: u16,
    pub remote_port: u16,
    /// Sent only when the user explicitly approves a changed SSH identity.
    pub expected_host_key_fingerprint: Option<String>,
    pub remember_host_key: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelStatus {
    pub host_id: String,
    pub state: &'static str,
    pub local_port: u16,
    pub remote_port: u16,
    pub host_key_fingerprint: Option<String>,
    pub stored_host_key_fingerprint: Option<String>,
    pub error_category: Option<&'static str>,
    pub stderr: String,
    pub exit_reason: Option<String>,
    pub recoverable: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshPairingRequest {
    pub ssh_host: String,
    pub ssh_port: u16,
    pub username: String,
    pub authentication: String,
    pub private_key_path: Option<String>,
    /// Used only during this connection attempt and never written to disk.
    pub password: Option<String>,
    pub local_port: u16,
    pub remote_port: u16,
    /// Sent only when the user explicitly approves a changed SSH identity.
    pub expected_host_key_fingerprint: Option<String>,
    pub remember_host_key: bool,
}

#[derive(Debug)]
pub struct RemotePairingBootstrap {
    pub state: &'static str,
    pub agent_host_id: Option<String>,
    pub pairing_code: Option<String>,
    pub host_key_fingerprint: Option<String>,
    pub stored_host_key_fingerprint: Option<String>,
    pub detail: String,
}

pub struct PairingExchangeResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

struct SshSession {
    request: SshTunnelRequest,
    password: Mutex<Option<String>>,
    child: Mutex<Option<Child>>,
    state: Mutex<SshTunnelStatus>,
    stop_requested: Mutex<bool>,
    known_hosts_path: Mutex<Option<PathBuf>>,
}

#[derive(Default)]
pub struct SshTunnelManager {
    sessions: Mutex<HashMap<String, Arc<SshSession>>>,
}

impl SshTunnelManager {
    fn start(&self, request: SshTunnelRequest) -> Result<SshTunnelStatus, String> {
        validate_request(&request)?;

        if let Some(previous) = self.remove_session(&request.host_id) {
            stop_session(&previous);
        }

        let scanned = scan_host_key(&request.ssh_host, request.ssh_port)?;
        let stored = read_host_key(&request.host_id)?;
        if let Some(expected) = request.expected_host_key_fingerprint.as_deref() {
            if expected != scanned.fingerprint {
                return Ok(status_for_key_decision(
                    &request,
                    "host-key-mismatch",
                    &scanned.fingerprint,
                    stored.as_deref(),
                    "The server's SSH identity changed again during setup. Review the change before continuing.",
                ));
            }
        } else if let Some(stored_fingerprint) = stored.as_deref() {
            if stored_fingerprint != scanned.fingerprint {
                return Ok(status_for_key_decision(
                    &request,
                    "host-key-changed",
                    &scanned.fingerprint,
                    stored.as_deref(),
                    "This server's SSH identity changed. Confirm it is still your server before trusting the update.",
                ));
            }
        } else if !request.remember_host_key {
            return Ok(status_for_key_decision(
                &request,
                "awaiting-host-key",
                &scanned.fingerprint,
                None,
                "MSC cannot remember this server's SSH identity for this connection.",
            ));
        }

        if request.remember_host_key {
            write_host_key(&request.host_id, &scanned.fingerprint)?;
        }

        let known_hosts_path = write_known_hosts(&request.host_id, &scanned.known_hosts_line)?;
        let password = request.password.clone();
        let mut session_request = request.clone();
        session_request.password = None;
        let child = match spawn_ssh(&request, &known_hosts_path) {
            Ok(child) => child,
            Err(error) => {
                let _ = fs::remove_file(&known_hosts_path);
                return Err(error);
            }
        };
        let status = SshTunnelStatus {
            host_id: request.host_id.clone(),
            state: "connecting",
            local_port: request.local_port,
            remote_port: request.remote_port,
            host_key_fingerprint: Some(scanned.fingerprint),
            stored_host_key_fingerprint: stored,
            error_category: None,
            stderr: String::new(),
            exit_reason: None,
            recoverable: false,
        };
        let session = Arc::new(SshSession {
            request: session_request,
            password: Mutex::new(password),
            child: Mutex::new(Some(child)),
            state: Mutex::new(status),
            stop_requested: Mutex::new(false),
            known_hosts_path: Mutex::new(Some(known_hosts_path)),
        });
        self.sessions
            .lock()
            .map_err(|_| "The SSH session registry is unavailable.".to_string())?
            .insert(request.host_id.clone(), Arc::clone(&session));
        monitor_session(session);
        self.status(&request.host_id)
    }

    fn status(&self, host_id: &str) -> Result<SshTunnelStatus, String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| "The SSH session registry is unavailable.".to_string())?
            .get(host_id)
            .cloned()
            .ok_or_else(|| "No SSH session exists for the selected host.".to_string())?;
        session
            .state
            .lock()
            .map_err(|_| "The SSH session state is unavailable.".to_string())
            .map(|state| state.clone())
    }

    fn retry(&self, host_id: &str) -> Result<SshTunnelStatus, String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| "The SSH session registry is unavailable.".to_string())?
            .get(host_id)
            .cloned()
            .ok_or_else(|| "No SSH session exists for the selected host.".to_string())?;
        let mut request = session.request.clone();
        request.expected_host_key_fingerprint = None;
        request.remember_host_key = true;
        request.password = session
            .password
            .lock()
            .map_err(|_| "Authentication: The SSH session credential is unavailable.".to_string())?
            .clone();
        if request.authentication == "password" && request.password.is_none() {
            return Err(
                "The password was cleared when the previous SSH session stopped.".to_string(),
            );
        }
        self.start(request)
    }

    fn stop(&self, host_id: &str) -> Result<SshTunnelStatus, String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| "The SSH session registry is unavailable.".to_string())?
            .get(host_id)
            .cloned()
            .ok_or_else(|| "No SSH session exists for the selected host.".to_string())?;
        stop_session(&session);
        session
            .state
            .lock()
            .map_err(|_| "The SSH session state is unavailable.".to_string())
            .map(|state| state.clone())
    }

    fn remove_session(&self, host_id: &str) -> Option<Arc<SshSession>> {
        self.sessions.lock().ok()?.remove(host_id)
    }
}

impl Drop for SshTunnelManager {
    fn drop(&mut self) {
        if let Ok(sessions) = self.sessions.get_mut() {
            for session in sessions.values() {
                stop_session(session);
            }
        }
    }
}

#[tauri::command]
pub fn ssh_tunnel_start(
    manager: tauri::State<'_, SshTunnelManager>,
    request: SshTunnelRequest,
) -> Result<SshTunnelStatus, String> {
    manager.start(request)
}

#[tauri::command]
pub fn ssh_tunnel_status(
    manager: tauri::State<'_, SshTunnelManager>,
    host_id: String,
) -> Result<SshTunnelStatus, String> {
    manager.status(host_id.trim())
}

#[tauri::command]
pub fn ssh_tunnel_retry(
    manager: tauri::State<'_, SshTunnelManager>,
    host_id: String,
) -> Result<SshTunnelStatus, String> {
    manager.retry(host_id.trim())
}

#[tauri::command]
pub fn ssh_tunnel_stop(
    manager: tauri::State<'_, SshTunnelManager>,
    host_id: String,
) -> Result<SshTunnelStatus, String> {
    manager.stop(host_id.trim())
}

/// The SSH askpass helper is the same signed desktop executable, invoked by
/// OpenSSH only when password authentication needs a prompt. The password is
/// inherited in memory for that child process and is never put in a command
/// argument, log, localStorage record, or persistent helper file.
pub fn run_ssh_askpass_helper() -> bool {
    if std::env::var_os(SSH_ASKPASS_MARKER).is_none() {
        return false;
    }
    if let Some(password) = std::env::var_os(SSH_ASKPASS_PASSWORD) {
        let _ = std::io::stdout().write_all(password.to_string_lossy().as_bytes());
    }
    std::process::exit(0);
}

struct ScannedHostKey {
    fingerprint: String,
    known_hosts_line: String,
}

fn validate_request(request: &SshTunnelRequest) -> Result<(), String> {
    validate_connection_values(
        [
            ("host ID", request.host_id.as_str()),
            ("SSH host", request.ssh_host.as_str()),
            ("SSH username", request.username.as_str()),
        ],
        request.ssh_port,
        request.local_port,
        request.remote_port,
        &request.authentication,
        request.password.as_deref(),
        request.private_key_path.as_deref(),
    )
}

fn validate_pairing_request(request: &SshPairingRequest) -> Result<(), String> {
    validate_connection_values(
        [
            ("SSH host", request.ssh_host.as_str()),
            ("SSH username", request.username.as_str()),
        ],
        request.ssh_port,
        request.local_port,
        request.remote_port,
        &request.authentication,
        request.password.as_deref(),
        request.private_key_path.as_deref(),
    )
}

fn validate_connection_values<const N: usize>(
    values: [(&str, &str); N],
    ssh_port: u16,
    local_port: u16,
    remote_port: u16,
    authentication: &str,
    password: Option<&str>,
    private_key_path: Option<&str>,
) -> Result<(), String> {
    for (label, value) in values {
        if value.trim().is_empty() || value.contains('\0') || value.chars().any(char::is_whitespace)
        {
            return Err(format!(
                "SSH: The {label} must be a single non-empty value."
            ));
        }
    }
    if ssh_port == 0 || local_port == 0 || remote_port == 0 {
        return Err(
            "SSH: SSH, local-forward, and remote-management ports must be between 1 and 65535."
                .to_string(),
        );
    }
    match authentication {
        "password" if password.unwrap_or_default().is_empty() => return Err(
            "Authentication: Password authentication needs a password for the connection attempt."
                .to_string(),
        ),
        "private-key" => {
            let path = private_key_path
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    "SSH: Private-key authentication needs a key-file reference.".to_string()
                })?;
            if !Path::new(path).is_file() {
                return Err("SSH: The selected private-key file is not available.".to_string());
            }
            validate_private_key(path)?;
        }
        "agent" => {}
        other => {
            return Err(format!(
                "SSH: Unsupported SSH authentication choice: {other}."
            ))
        }
    }
    Ok(())
}

fn validate_private_key(path: &str) -> Result<(), String> {
    let output = Command::new("ssh-keygen")
        .args(["-y", "-P", ""])
        .arg(path)
        .output()
        .map_err(|error| format!("SSH: Could not inspect the selected private key: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err("SSH: The selected private key is encrypted or uses an unsupported format. Choose an unencrypted key file or use the SSH agent.".to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairingOutput {
    pairing_code: String,
    agent_host_id: String,
    client_kind: String,
    expires_at: String,
}

/// Runs the only remote command this feature permits. The command and every
/// argument are fixed so the setup screen cannot become a general SSH shell.
pub fn create_remote_pairing(
    request: &SshPairingRequest,
) -> Result<RemotePairingBootstrap, String> {
    validate_pairing_request(request)?;
    let scanned = scan_host_key(&request.ssh_host, request.ssh_port)?;
    let pending_host_key_id = pending_host_key_id(request);
    let stored = read_host_key(&pending_host_key_id)?;
    if let Some(expected) = request.expected_host_key_fingerprint.as_deref() {
        if expected != scanned.fingerprint {
            return Ok(RemotePairingBootstrap {
                state: "host-key-mismatch",
                agent_host_id: None,
                pairing_code: None,
                host_key_fingerprint: Some(scanned.fingerprint),
                stored_host_key_fingerprint: stored,
                detail: "The server's SSH identity changed again during setup. Review the change before continuing.".to_string(),
            });
        }
    } else if let Some(stored_fingerprint) = stored.as_deref() {
        if stored_fingerprint != scanned.fingerprint {
            return Ok(RemotePairingBootstrap {
                state: "host-key-changed",
                agent_host_id: None,
                pairing_code: None,
                host_key_fingerprint: Some(scanned.fingerprint),
                stored_host_key_fingerprint: stored,
                detail: "This server's SSH identity changed. Confirm it is still your server before trusting the update.".to_string(),
            });
        }
    } else if !request.remember_host_key {
        return Ok(RemotePairingBootstrap {
            state: "awaiting-host-key",
            agent_host_id: None,
            pairing_code: None,
            host_key_fingerprint: Some(scanned.fingerprint),
            stored_host_key_fingerprint: None,
            detail: "MSC cannot remember this server's SSH identity for this connection."
                .to_string(),
        });
    }

    let known_hosts_path = write_known_hosts(&pending_host_key_id, &scanned.known_hosts_line)?;
    let output = run_remote_pairing_command(request, &known_hosts_path);
    let _ = fs::remove_file(&known_hosts_path);
    let output = output?;
    if !output.status.success() {
        return Err(classify_ssh_failure(&clean_output(&output.stderr)).1);
    }
    let pairing: PairingOutput = serde_json::from_slice(&output.stdout).map_err(|_| {
        "MSC agent: The remote `msc` command is missing from the remote user's PATH, or it returned an invalid pairing response.".to_string()
    })?;
    if pairing.client_kind != "desktop"
        || pairing.agent_host_id.trim().is_empty()
        || !pairing.pairing_code.starts_with("pair_")
        || pairing.expires_at.trim().is_empty()
    {
        return Err(
            "MSC agent: The remote `msc` command returned an invalid desktop challenge."
                .to_string(),
        );
    }
    if request.remember_host_key {
        write_host_key(&pending_host_key_id, &scanned.fingerprint)?;
        write_host_key(&pairing.agent_host_id, &scanned.fingerprint)?;
    }
    Ok(RemotePairingBootstrap {
        state: "paired",
        agent_host_id: Some(pairing.agent_host_id),
        pairing_code: Some(pairing.pairing_code),
        host_key_fingerprint: Some(scanned.fingerprint),
        stored_host_key_fingerprint: stored,
        detail: "The remote host created a one-use desktop authorization.".to_string(),
    })
}

fn run_remote_pairing_command(
    request: &SshPairingRequest,
    known_hosts_path: &Path,
) -> Result<std::process::Output, String> {
    let mut command = ssh_command(
        request.ssh_port,
        &request.username,
        &request.authentication,
        request.private_key_path.as_deref(),
        request.password.as_deref(),
        known_hosts_path,
    )?;
    command
        .arg(&request.ssh_host)
        .args(["msc", "pairing", "create"])
        .args(["--client-kind", "desktop", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .output()
        .map_err(|error| format!("SSH: Could not run the remote MSC pairing command: {error}"))
}

/// Exchanges the captured challenge through a temporary loopback-only SSH
/// forward. The response remains in native Rust so the bearer token never
/// crosses the Tauri boundary.
pub async fn exchange_pairing_through_tunnel(
    request: &SshPairingRequest,
    pairing_code: &str,
    expected_fingerprint: &str,
) -> Result<PairingExchangeResponse, String> {
    let scanned = scan_host_key(&request.ssh_host, request.ssh_port)?;
    if scanned.fingerprint != expected_fingerprint {
        return Err(
            "SSH: The remote host key changed while desktop pairing was in progress.".to_string(),
        );
    }
    let known_hosts_path =
        write_known_hosts(&pending_host_key_id(request), &scanned.known_hosts_line)?;
    let mut command = ssh_command(
        request.ssh_port,
        &request.username,
        &request.authentication,
        request.private_key_path.as_deref(),
        request.password.as_deref(),
        &known_hosts_path,
    )?;
    command
        .args(["-N", "-T"])
        .args(["-o", "ExitOnForwardFailure=yes"])
        .arg("-L")
        .arg(format!(
            "127.0.0.1:{}:127.0.0.1:{}",
            request.local_port, request.remote_port
        ))
        .arg(&request.ssh_host)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("SSH: Could not start the temporary SSH forward: {error}"))?;
    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:{}/v1/auth/desktop-pairings",
        request.local_port
    );
    let mut last_error = "the forwarded management port did not respond".to_string();
    for _ in 0..30 {
        if let Ok(Some(status)) = child.try_wait() {
            last_error = format!("the SSH forward exited with status {status}");
            break;
        }
        match client
            .post(&url)
            .json(&serde_json::json!({ "pairingCode": pairing_code }))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response
                    .bytes()
                    .await
                    .map_err(|error| format!("The pairing response could not be read: {error}"))?
                    .to_vec();
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&known_hosts_path);
                return Ok(PairingExchangeResponse { status, body });
            }
            Err(error) => last_error = error.to_string(),
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let output = child
        .wait_with_output()
        .map_err(|error| format!("The temporary SSH forward could not stop: {error}"))?;
    let _ = fs::remove_file(&known_hosts_path);
    let detail = format!("{last_error}. {}", clean_output(&output.stderr));
    Err(classify_ssh_failure(&detail).1)
}

fn pending_host_key_id(request: &SshPairingRequest) -> String {
    format!(
        "pending:{}@{}:{}",
        request.username, request.ssh_host, request.ssh_port
    )
}

fn scan_host_key(host: &str, port: u16) -> Result<ScannedHostKey, String> {
    let output = Command::new("ssh-keyscan")
        .args([
            "-T",
            "5",
            "-p",
            &port.to_string(),
            "-t",
            "ed25519,ecdsa-sha2-nistp256,rsa-sha2-512,rsa-sha2-256",
            host,
        ])
        .output()
        .map_err(|error| format!("Network: Could not reach the SSH host: {error}"))?;
    let output_text = String::from_utf8_lossy(&output.stdout);
    let mut lines: Vec<&str> = output_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    lines.sort_by_key(|line| host_key_rank(line));
    let known_hosts_line = lines
        .first()
        .copied()
        .ok_or_else(|| {
            format!(
                "Network: ssh-keyscan could not reach the SSH host: {}",
                clean_output(&output.stderr)
            )
        })?
        .to_string();
    let path = temporary_key_path("scan");
    write_restricted_file(&path, format!("{known_hosts_line}\n").as_bytes())?;
    let fingerprint = fingerprint_for_key_file(&path);
    let _ = fs::remove_file(path);
    Ok(ScannedHostKey {
        fingerprint: fingerprint?,
        known_hosts_line,
    })
}

fn host_key_rank(line: &str) -> usize {
    match line.split_whitespace().nth(1) {
        Some("ssh-ed25519") => 0,
        Some("ecdsa-sha2-nistp256") => 1,
        Some("rsa-sha2-512") => 2,
        Some("rsa-sha2-256") => 3,
        _ => 4,
    }
}

fn fingerprint_for_key_file(path: &Path) -> Result<String, String> {
    let output = Command::new("ssh-keygen")
        .args(["-lf"])
        .arg(path)
        .args(["-E", "sha256"])
        .output()
        .map_err(|error| format!("SSH: Could not verify the remote host key: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SSH: ssh-keygen could not verify the host key: {}",
            clean_output(&output.stderr)
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|part| part.starts_with("SHA256:"))
        .map(str::to_string)
        .ok_or_else(|| "SSH: ssh-keygen returned no SHA256 host-key fingerprint.".to_string())
}

fn write_known_hosts(host_id: &str, line: &str) -> Result<PathBuf, String> {
    let path = temporary_key_path(host_id);
    write_restricted_file(&path, format!("{line}\n").as_bytes())?;
    Ok(path)
}

fn write_restricted_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    let mut file = options
        .open(path)
        .map_err(|error| format!("SSH: Could not create temporary SSH host-key file: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| {
                format!("SSH: Could not protect temporary SSH host-key file: {error}")
            })?;
    }
    file.write_all(bytes)
        .map_err(|error| format!("SSH: Could not write temporary SSH host-key file: {error}"))
}

fn temporary_key_path(label: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut digest = Sha256::new();
    digest.update(label.as_bytes());
    digest.update(std::process::id().to_le_bytes());
    digest.update(now.to_le_bytes());
    let suffix = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::env::temp_dir().join(format!("msc2-ssh-known-host-{suffix}"))
}

fn ssh_command(
    ssh_port: u16,
    username: &str,
    authentication: &str,
    private_key_path: Option<&str>,
    password: Option<&str>,
    known_hosts_path: &Path,
) -> Result<Command, String> {
    let executable = std::env::current_exe().map_err(|error| {
        format!("SSH: Could not resolve the desktop executable for SSH askpass: {error}")
    })?;
    let mut command = Command::new("ssh");
    command
        .args(["-p"])
        .arg(ssh_port.to_string())
        .args(["-o", "ConnectTimeout=10"])
        .args(["-o", "BatchMode=no"])
        .args(["-o", "NumberOfPasswordPrompts=1"])
        .args(["-o", "StrictHostKeyChecking=yes"])
        .args(["-o", "GlobalKnownHostsFile=none"])
        .args(["-o", "UserKnownHostsFile"])
        .arg(known_hosts_path)
        .args(["-l", username]);
    if authentication == "private-key" {
        command.args(["-i", private_key_path.unwrap_or_default()]);
        command.args(["-o", "IdentitiesOnly=yes"]);
    }
    if let Some(password) = password {
        command
            .env(SSH_ASKPASS_MARKER, "1")
            .env(SSH_ASKPASS_PASSWORD, password)
            .env("SSH_ASKPASS", executable)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env("DISPLAY", "msc2");
    }
    Ok(command)
}

fn spawn_ssh(request: &SshTunnelRequest, known_hosts_path: &Path) -> Result<Child, String> {
    let mut command = ssh_command(
        request.ssh_port,
        &request.username,
        &request.authentication,
        request.private_key_path.as_deref(),
        request.password.as_deref(),
        known_hosts_path,
    )?;
    command
        .args(["-N", "-T"])
        .args(["-o", "ExitOnForwardFailure=yes"])
        .args(["-L"])
        .arg(format!(
            "127.0.0.1:{}:127.0.0.1:{}",
            request.local_port, request.remote_port
        ))
        .arg(&request.ssh_host)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
        .spawn()
        .map_err(|error| format!("SSH: Could not start the managed SSH tunnel: {error}"))
}

fn monitor_session(session: Arc<SshSession>) {
    thread::spawn(move || {
        let stderr = session
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.as_mut().and_then(|child| child.stderr.take()));
        if let Some(stderr) = stderr {
            let stderr_session = Arc::clone(&session);
            thread::spawn(move || collect_stderr(stderr, stderr_session));
        }

        loop {
            let result = session
                .child
                .lock()
                .ok()
                .and_then(|mut child| child.as_mut().and_then(|child| child.try_wait().ok()));
            match result {
                Some(Some(exit)) => {
                    let stopped = session
                        .stop_requested
                        .lock()
                        .map(|flag| *flag)
                        .unwrap_or(true);
                    if let Ok(mut child) = session.child.lock() {
                        *child = None;
                    }
                    if let Ok(mut state) = session.state.lock() {
                        state.state = if stopped { "stopped" } else { "failed" };
                        state.exit_reason = Some(if exit.success() {
                            "SSH: The SSH process exited.".to_string()
                        } else {
                            format!("SSH: The SSH process exited with status {exit}.")
                        });
                        state.error_category = (!stopped).then_some("ssh");
                        state.recoverable = !stopped;
                    }
                    cleanup_known_hosts(&session);
                    break;
                }
                Some(None) => {
                    if let Ok(mut state) = session.state.lock() {
                        if state.state == "connecting" {
                            state.state = "connected";
                        }
                    }
                }
                None => break,
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}

fn collect_stderr(stderr: impl Read, session: Arc<SshSession>) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok() && !line.is_empty() {
        let password = session.password.lock().ok().and_then(|value| value.clone());
        let clean = redact_output(line.trim_end(), password.as_deref());
        if let Ok(mut state) = session.state.lock() {
            append_bounded(&mut state.stderr, &clean);
            if state.state == "failed" {
                let (category, detail) = classify_ssh_failure(&state.stderr);
                state.error_category = Some(category);
                state.exit_reason = Some(detail);
            }
        }
        line.clear();
    }
}

fn append_bounded(target: &mut String, addition: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(addition);
    if target.len() > MAX_STDERR_BYTES {
        let start = target.len() - MAX_STDERR_BYTES;
        *target = target.get(start..).unwrap_or_default().to_string();
    }
}

fn redact_output(value: &str, password: Option<&str>) -> String {
    let mut clean = value.to_string();
    if let Some(password) = password.filter(|password| !password.is_empty()) {
        clean = clean.replace(password, "[redacted]");
    }
    clean
        .split_whitespace()
        .map(|part| {
            if part.starts_with("msc2_") || part.starts_with("pair_") {
                "[redacted]"
            } else if part.to_ascii_lowercase().starts_with("password=") {
                "password=[redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn classify_ssh_failure(detail: &str) -> (&'static str, String) {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("invalid format") || lower.contains("load key") || lower.contains("encrypted")
    {
        return (
            "ssh",
            "SSH: The selected private key is encrypted or uses an unsupported format.".to_string(),
        );
    }
    if lower.contains("permission denied")
        || lower.contains("authentication failed")
        || lower.contains("incorrect password")
        || lower.contains("password") && lower.contains("failed")
    {
        return (
            "authentication",
            "Authentication: The SSH password or key was refused by the remote computer."
                .to_string(),
        );
    }
    if lower.contains("agent refused")
        || lower.contains("sign_and_send_pubkey")
        || lower.contains("could not open a connection to your authentication agent")
    {
        return (
            "authentication",
            "Authentication: The SSH agent is locked, unavailable, or refused this key."
                .to_string(),
        );
    }
    if lower.contains("address already in use")
        || lower.contains("bind: ")
        || lower.contains("port is already in use")
    {
        return (
            "network",
            "Network: The selected local forwarding port is already occupied. Choose another port."
                .to_string(),
        );
    }
    if lower.contains("command not found")
        || (lower.contains("not found") && lower.contains("msc"))
        || lower.contains("invalid pairing response")
    {
        return (
            "msc-agent",
            "MSC agent: The remote `msc` command is missing from the remote user's PATH."
                .to_string(),
        );
    }
    if lower.contains("agent")
        && (lower.contains("stopped")
            || lower.contains("unavailable")
            || lower.contains("not running")
            || lower.contains("version"))
    {
        return (
            "msc-agent",
            "MSC agent: The remote management service is stopped, unavailable, or below the supported version floor.".to_string(),
        );
    }
    if lower.contains("timed out")
        || lower.contains("connection refused")
        || lower.contains("could not resolve hostname")
        || lower.contains("no route to host")
        || lower.contains("network is unreachable")
    {
        return ("network", format!("Network: {}", detail.trim()));
    }
    ("ssh", format!("SSH: {}", detail.trim()))
}

fn stop_session(session: &SshSession) {
    if let Ok(mut stop_requested) = session.stop_requested.lock() {
        *stop_requested = true;
    }
    if let Ok(mut child) = session.child.lock() {
        if let Some(child) = child.as_mut() {
            let _ = child.kill();
        }
    }
    if let Ok(mut password) = session.password.lock() {
        *password = None;
    }
    cleanup_known_hosts(session);
    if let Ok(mut state) = session.state.lock() {
        state.state = "stopped";
        state.exit_reason = Some("SSH: The SSH tunnel was stopped.".to_string());
        state.error_category = None;
        state.recoverable = false;
    }
}

fn cleanup_known_hosts(session: &SshSession) {
    if let Ok(mut path) = session.known_hosts_path.lock() {
        if let Some(path) = path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn status_for_key_decision(
    request: &SshTunnelRequest,
    state: &'static str,
    observed: &str,
    stored: Option<&str>,
    detail: &str,
) -> SshTunnelStatus {
    SshTunnelStatus {
        host_id: request.host_id.clone(),
        state,
        local_port: request.local_port,
        remote_port: request.remote_port,
        host_key_fingerprint: Some(observed.to_string()),
        stored_host_key_fingerprint: stored.map(str::to_string),
        error_category: Some("ssh"),
        stderr: detail.to_string(),
        exit_reason: None,
        recoverable: true,
    }
}

fn host_key_secret_key(host_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(host_id.as_bytes());
    let suffix = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{HOST_KEY_SECRET_PREFIX}{suffix}")
}

fn read_host_key(host_id: &str) -> Result<Option<String>, String> {
    super::desktop_secret_store()?
        .get(&host_key_secret_key(host_id))
        .map_err(|error| format!("Authentication: Could not read the stored SSH host key: {error}"))
}

fn write_host_key(host_id: &str, fingerprint: &str) -> Result<(), String> {
    super::desktop_secret_store()?
        .set(&host_key_secret_key(host_id), fingerprint)
        .map_err(|error| format!("Authentication: Could not remember the SSH host key: {error}"))
}

fn clean_output(bytes: &[u8]) -> String {
    let value = String::from_utf8_lossy(bytes);
    redact_output(value.trim(), None)
}
