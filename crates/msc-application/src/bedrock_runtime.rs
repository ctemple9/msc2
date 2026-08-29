//! The platform-neutral Bedrock runtime boundary and the macOS sidecar wire.
//!
//! Native runtimes and the sidecar all report the same lifecycle vocabulary.
//! This module intentionally does not know how a process, VM, or OS gathers
//! metrics; those details belong to each backend adapter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use msc_infrastructure::bedrock_distribution::{
    self, BedrockPlatform, InstalledBedrockDistribution,
};
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::process::ProcessId;

pub const SIDECAR_SHARED_DIRECTORY_TAG: &str = "world";
pub const SIDECAR_GUEST_MOUNT: &str = "/mnt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BedrockRuntimeBackend {
    Native,
    Sidecar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockRuntimeEligibilityState {
    Available,
    ProvisioningRequired,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockHost {
    Linux,
    Windows,
    MacosIntel,
    MacosAppleSilicon,
    Other,
}

impl BedrockHost {
    pub const fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            return Self::Linux;
        }
        #[cfg(target_os = "windows")]
        {
            return Self::Windows;
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            return Self::MacosIntel;
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            return Self::MacosAppleSilicon;
        }
        #[allow(unreachable_code)]
        Self::Other
    }

    pub const fn host_os(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::MacosIntel | Self::MacosAppleSilicon => "macos",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockSidecarResources {
    pub executable: PathBuf,
    pub kernel: PathBuf,
    pub initramfs: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockRuntimePaths {
    pub server_dir: PathBuf,
    pub sidecar: Option<BedrockSidecarResources>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockRuntimeEligibility {
    pub host: BedrockHost,
    pub backend: Option<BedrockRuntimeBackend>,
    pub state: BedrockRuntimeEligibilityState,
    pub reason_code: Option<String>,
    pub message: String,
}

impl BedrockRuntimeEligibility {
    /// Detect the real host and inspect only files owned by the agent. The
    /// compatibility CSV is deliberately not read here: it is published
    /// evidence, not a runtime switch.
    pub fn detect(fs: &dyn FileSystem, paths: &BedrockRuntimePaths) -> Self {
        Self::for_host(fs, BedrockHost::current(), paths)
    }

    /// The host parameter is injectable so fixture tests can exercise all
    /// platform branches on one machine. Production composition uses
    /// [`Self::detect`] and therefore cannot claim another host by config.
    pub fn for_host(fs: &dyn FileSystem, host: BedrockHost, paths: &BedrockRuntimePaths) -> Self {
        match host {
            BedrockHost::Linux => Self::native(fs, host, paths, BedrockPlatform::Linux),
            BedrockHost::Windows => Self::native(fs, host, paths, BedrockPlatform::Windows),
            BedrockHost::MacosIntel => Self::sidecar(fs, host, paths),
            BedrockHost::MacosAppleSilicon => Self {
                host,
                backend: None,
                state: BedrockRuntimeEligibilityState::Unavailable,
                reason_code: Some("no_test_hardware".to_owned()),
                message: "Bedrock is unavailable on Apple Silicon under D-028.".to_owned(),
            },
            BedrockHost::Other => Self {
                host,
                backend: None,
                state: BedrockRuntimeEligibilityState::Unavailable,
                reason_code: Some("unsupported_host".to_owned()),
                message: "Bedrock has no runtime backend for this host.".to_owned(),
            },
        }
    }

    /// Synthetic runtime eligibility used by the existing backend unit tests.
    /// It is intentionally separate from [`Self::detect`], which always
    /// checks the real filesystem and host.
    pub fn synthetic_available(host: BedrockHost, backend: BedrockRuntimeBackend) -> Self {
        Self {
            host,
            backend: Some(backend),
            state: BedrockRuntimeEligibilityState::Available,
            reason_code: None,
            message: "Bedrock runtime is available.".to_owned(),
        }
    }

    pub fn capabilities_for(&self, backend: BedrockRuntimeBackend) -> BedrockRuntimeCapabilities {
        if self.backend == Some(backend) && self.state == BedrockRuntimeEligibilityState::Available
        {
            BedrockRuntimeCapabilities::supported(backend)
        } else {
            BedrockRuntimeCapabilities::unavailable(
                backend,
                self.reason_code
                    .clone()
                    .unwrap_or_else(|| "bedrock-provisioning-required".to_owned()),
            )
        }
    }

    fn native(
        fs: &dyn FileSystem,
        host: BedrockHost,
        paths: &BedrockRuntimePaths,
        platform: BedrockPlatform,
    ) -> Self {
        Self::from_distribution(
            host,
            BedrockRuntimeBackend::Native,
            bedrock_distribution::inspect_installed_distribution(fs, &paths.server_dir, platform),
        )
    }

    fn sidecar(fs: &dyn FileSystem, host: BedrockHost, paths: &BedrockRuntimePaths) -> Self {
        let Some(resources) = paths.sidecar.as_ref() else {
            return Self::provisioning(
                host,
                BedrockRuntimeBackend::Sidecar,
                "sidecar_resources_required",
            );
        };
        if !is_file(fs, &resources.executable) || !is_executable(fs, &resources.executable) {
            return Self::provisioning(
                host,
                BedrockRuntimeBackend::Sidecar,
                "sidecar_executable_required",
            );
        }
        if !is_file(fs, &resources.kernel) || !is_file(fs, &resources.initramfs) {
            return Self::provisioning(
                host,
                BedrockRuntimeBackend::Sidecar,
                "sidecar_appliance_required",
            );
        }

        // The macOS VM runs the Linux BDS binary. Keep the Macos inspection as
        // a read-only compatibility fallback for installations made before
        // the sidecar's guest-package platform was made explicit.
        let distribution = bedrock_distribution::inspect_installed_distribution(
            fs,
            &paths.server_dir,
            BedrockPlatform::Linux,
        );
        let distribution = if matches!(&distribution, InstalledBedrockDistribution::Verified(_)) {
            distribution
        } else {
            let legacy_distribution = bedrock_distribution::inspect_installed_distribution(
                fs,
                &paths.server_dir,
                BedrockPlatform::Macos,
            );
            if matches!(
                &legacy_distribution,
                InstalledBedrockDistribution::Verified(_)
            ) {
                legacy_distribution
            } else {
                distribution
            }
        };
        Self::from_distribution(host, BedrockRuntimeBackend::Sidecar, distribution)
    }

    fn native_or_sidecar_message(reason_code: &str) -> String {
        match reason_code {
            "bds_distribution_unverified" => {
                "A verified Bedrock distribution is required before start.".to_owned()
            }
            _ => "A verified Bedrock distribution must be provisioned before start.".to_owned(),
        }
    }

    fn from_distribution(
        host: BedrockHost,
        backend: BedrockRuntimeBackend,
        distribution: InstalledBedrockDistribution,
    ) -> Self {
        match distribution {
            InstalledBedrockDistribution::Verified(_) => Self {
                host,
                backend: Some(backend),
                state: BedrockRuntimeEligibilityState::Available,
                reason_code: None,
                message: "Bedrock runtime is available.".to_owned(),
            },
            InstalledBedrockDistribution::Missing => {
                Self::provisioning(host, backend, "bds_distribution_required")
            }
            InstalledBedrockDistribution::Unverified => {
                Self::provisioning(host, backend, "bds_distribution_unverified")
            }
        }
    }

    fn provisioning(host: BedrockHost, backend: BedrockRuntimeBackend, reason_code: &str) -> Self {
        Self {
            host,
            backend: Some(backend),
            state: BedrockRuntimeEligibilityState::ProvisioningRequired,
            reason_code: Some(reason_code.to_owned()),
            message: Self::native_or_sidecar_message(reason_code),
        }
    }
}

fn is_file(fs: &dyn FileSystem, path: &Path) -> bool {
    fs.stat(path).is_ok_and(|metadata| metadata.is_file)
}

fn is_executable(fs: &dyn FileSystem, path: &Path) -> bool {
    fs.stat(path)
        .is_ok_and(|metadata| metadata.is_file && metadata.executable)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BedrockRuntimeCapability {
    Provision,
    Start,
    Readiness,
    Console,
    Command,
    GracefulStop,
    ForceStop,
    Metrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BedrockRuntimeCapabilities {
    pub backend: BedrockRuntimeBackend,
    pub supported: bool,
    pub capabilities: BTreeSet<BedrockRuntimeCapability>,
    pub unavailable_reason: Option<String>,
}

impl BedrockRuntimeCapabilities {
    pub fn supported(backend: BedrockRuntimeBackend) -> Self {
        Self {
            backend,
            supported: true,
            capabilities: BTreeSet::from([
                BedrockRuntimeCapability::Provision,
                BedrockRuntimeCapability::Start,
                BedrockRuntimeCapability::Readiness,
                BedrockRuntimeCapability::Console,
                BedrockRuntimeCapability::Command,
                BedrockRuntimeCapability::GracefulStop,
                BedrockRuntimeCapability::ForceStop,
                BedrockRuntimeCapability::Metrics,
            ]),
            unavailable_reason: None,
        }
    }

    pub fn unavailable(backend: BedrockRuntimeBackend, reason: impl Into<String>) -> Self {
        Self {
            backend,
            supported: false,
            capabilities: BTreeSet::new(),
            unavailable_reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockRuntimeState {
    New,
    Provisioned,
    Starting,
    Running,
    Stopping,
    Stopped,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockProvisionRequest {
    pub server_dir: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BedrockStartRequest {
    pub memory_gb: u32,
    pub bedrock_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockRuntimeMetrics {
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BedrockRuntimeEvent {
    Ready { address: Option<String>, port: u16 },
    ConsoleLine(String),
    Metrics(BedrockRuntimeMetrics),
    Terminated { reason: BedrockTerminationReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BedrockTerminationReason {
    Clean,
    GuestError(String),
    StartFailed(String),
}

impl BedrockTerminationReason {
    fn wire_value(&self) -> String {
        match self {
            Self::Clean => "clean".to_owned(),
            Self::GuestError(message) => format!("guest-error:{message}"),
            Self::StartFailed(message) => format!("start-failed:{message}"),
        }
    }

    fn from_wire(value: &str) -> Result<Self, String> {
        if value == "clean" {
            return Ok(Self::Clean);
        }
        if let Some(message) = value.strip_prefix("guest-error:")
            && !message.is_empty()
        {
            return Ok(Self::GuestError(message.to_owned()));
        }
        if let Some(message) = value.strip_prefix("start-failed:")
            && !message.is_empty()
        {
            return Ok(Self::StartFailed(message.to_owned()));
        }
        Err(format!("invalid termination reason {value:?}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BedrockRuntimeError {
    InvalidState {
        operation: &'static str,
        state: BedrockRuntimeState,
    },
    Protocol(String),
    Remote(String),
    Transport(String),
    Provisioning(String),
    SidecarEof,
}

impl fmt::Display for BedrockRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { operation, state } => {
                write!(f, "cannot {operation} while runtime is {state:?}")
            }
            Self::Protocol(message) => write!(f, "sidecar protocol error: {message}"),
            Self::Remote(message) => write!(f, "sidecar rejected request: {message}"),
            Self::Transport(message) => write!(f, "sidecar transport error: {message}"),
            Self::Provisioning(message) => write!(f, "Bedrock provisioning failed: {message}"),
            Self::SidecarEof => f.write_str("sidecar ended unexpectedly"),
        }
    }
}

impl std::error::Error for BedrockRuntimeError {}

/// Every backend supplies this same lifecycle surface. A backend reports
/// values such as CPU and RAM through [`BedrockRuntimeMetrics`], while the
/// collection mechanism remains outside this trait.
pub trait BedrockRuntime {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities;
    fn state(&self) -> BedrockRuntimeState;
    /// Native runtimes expose their child process for the shared OS metrics
    /// provider. Sidecar runtimes leave this unset because their metrics are
    /// reported by the sidecar boundary instead.
    fn process_id(&self) -> Option<ProcessId> {
        None
    }
    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError>;
    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError>;
    fn stop(&mut self) -> Result<(), BedrockRuntimeError>;
    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError>;
    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError>;
    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError>;
}

/// The transport boundary is deliberately only lines in and lines out. The
/// real adapter can use child-process stdio; tests use an in-memory transport.
pub trait SidecarTransport {
    fn send_line(&mut self, line: &str) -> Result<(), String>;
    fn receive_line(&mut self) -> Result<Option<String>, String>;

    /// A concrete child-process transport can distinguish an idle pipe from
    /// EOF. The in-memory transport used by the protocol tests predates that
    /// distinction, so its default maps `None` to terminal EOF.
    fn receive_status(&mut self) -> Result<SidecarReceive, String> {
        self.receive_line().map(|line| match line {
            Some(line) => SidecarReceive::Line(line),
            None => SidecarReceive::Eof,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarReceive {
    Line(String),
    Pending,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarDirectoryMapping {
    pub server_dir: String,
    pub guest_mount: &'static str,
    pub tag: &'static str,
    pub read_only: bool,
}

impl SidecarDirectoryMapping {
    fn for_server_dir(server_dir: String) -> Self {
        Self {
            server_dir,
            guest_mount: SIDECAR_GUEST_MOUNT,
            tag: SIDECAR_SHARED_DIRECTORY_TAG,
            read_only: false,
        }
    }
}

/// The frozen JSON-lines frames. The Swift sidecar sees the same objects, so
/// no Rust process or macOS VM type appears in the wire model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum SidecarFrame {
    Provision {
        server_dir: String,
        version: String,
    },
    Provisioned {
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Start {
        memory_gb: u32,
        bedrock_port: u16,
    },
    Started {
        accepted: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Ready {
        guest_ip: String,
        port: u16,
        relay_up: bool,
    },
    Stop,
    ForceStop,
    Command {
        command: String,
    },
    CommandResult {
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    ConsoleLine {
        line: String,
    },
    Terminated {
        #[serde(with = "termination_reason_wire")]
        reason: BedrockTerminationReason,
    },
}

mod termination_reason_wire {
    use super::BedrockTerminationReason;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &BedrockTerminationReason, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.wire_value())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BedrockTerminationReason, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        BedrockTerminationReason::from_wire(&value).map_err(serde::de::Error::custom)
    }
}

pub fn encode_frame(frame: &SidecarFrame) -> Result<String, BedrockRuntimeError> {
    let mut line = serde_json::to_string(frame)
        .map_err(|error| BedrockRuntimeError::Protocol(error.to_string()))?;
    line.push('\n');
    Ok(line)
}

pub fn decode_frame(line: &str) -> Result<SidecarFrame, BedrockRuntimeError> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.is_empty() || line.contains(['\r', '\n']) {
        return Err(BedrockRuntimeError::Protocol(
            "a frame must contain exactly one JSON object".to_owned(),
        ));
    }
    serde_json::from_str(line).map_err(|error| BedrockRuntimeError::Protocol(error.to_string()))
}

pub struct SidecarRuntime<T> {
    transport: T,
    capabilities: BedrockRuntimeCapabilities,
    state: BedrockRuntimeState,
    directory_mapping: Option<SidecarDirectoryMapping>,
}

impl<T> SidecarRuntime<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            capabilities: BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Sidecar),
            state: BedrockRuntimeState::New,
            directory_mapping: None,
        }
    }

    pub fn directory_mapping(&self) -> Option<&SidecarDirectoryMapping> {
        self.directory_mapping.as_ref()
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn require_state(
        &self,
        operation: &'static str,
        allowed: &[BedrockRuntimeState],
    ) -> Result<(), BedrockRuntimeError> {
        if allowed.contains(&self.state) {
            Ok(())
        } else {
            Err(BedrockRuntimeError::InvalidState {
                operation,
                state: self.state,
            })
        }
    }

    fn send(&mut self, frame: SidecarFrame) -> Result<(), BedrockRuntimeError>
    where
        T: SidecarTransport,
    {
        let line = encode_frame(&frame)?;
        self.transport
            .send_line(&line)
            .map_err(BedrockRuntimeError::Transport)
    }

    fn receive_response(&mut self) -> Result<SidecarFrame, BedrockRuntimeError>
    where
        T: SidecarTransport,
    {
        loop {
            match self
                .transport
                .receive_status()
                .map_err(BedrockRuntimeError::Transport)?
            {
                SidecarReceive::Line(line) => return decode_frame(&line),
                SidecarReceive::Pending => std::thread::yield_now(),
                SidecarReceive::Eof => {
                    self.state = BedrockRuntimeState::Unavailable;
                    return Err(BedrockRuntimeError::SidecarEof);
                }
            }
        }
    }

    fn remote_result(ok: bool, reason: Option<String>) -> Result<(), BedrockRuntimeError> {
        if ok {
            Ok(())
        } else {
            Err(BedrockRuntimeError::Remote(
                reason.unwrap_or_else(|| "request rejected".to_owned()),
            ))
        }
    }
}

impl<T: SidecarTransport> BedrockRuntime for SidecarRuntime<T> {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        &self.capabilities
    }

    fn state(&self) -> BedrockRuntimeState {
        self.state
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        self.require_state("provision", &[BedrockRuntimeState::New])?;
        self.send(SidecarFrame::Provision {
            server_dir: request.server_dir.clone(),
            version: request.version,
        })?;
        match self.receive_response()? {
            SidecarFrame::Provisioned { ok, reason } => {
                Self::remote_result(ok, reason)?;
                self.directory_mapping =
                    Some(SidecarDirectoryMapping::for_server_dir(request.server_dir));
                self.state = BedrockRuntimeState::Provisioned;
                Ok(())
            }
            frame => Err(BedrockRuntimeError::Protocol(format!(
                "expected provisioned response, received {frame:?}"
            ))),
        }
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        self.require_state("start", &[BedrockRuntimeState::Provisioned])?;
        self.send(SidecarFrame::Start {
            memory_gb: request.memory_gb,
            bedrock_port: request.bedrock_port,
        })?;
        match self.receive_response()? {
            SidecarFrame::Started { accepted, reason } => {
                Self::remote_result(accepted, reason)?;
                self.state = BedrockRuntimeState::Starting;
                Ok(())
            }
            frame => Err(BedrockRuntimeError::Protocol(format!(
                "expected started response, received {frame:?}"
            ))),
        }
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_state(
            "stop",
            &[BedrockRuntimeState::Starting, BedrockRuntimeState::Running],
        )?;
        self.send(SidecarFrame::Stop)?;
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_state(
            "force-stop",
            &[
                BedrockRuntimeState::Starting,
                BedrockRuntimeState::Running,
                BedrockRuntimeState::Stopping,
            ],
        )?;
        self.send(SidecarFrame::ForceStop)?;
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.require_state("command", &[BedrockRuntimeState::Running])?;
        self.send(SidecarFrame::Command {
            command: command.to_owned(),
        })?;
        match self.receive_response()? {
            SidecarFrame::CommandResult { ok, reason } => Self::remote_result(ok, reason),
            frame => Err(BedrockRuntimeError::Protocol(format!(
                "expected command-result response, received {frame:?}"
            ))),
        }
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        let status = self
            .transport
            .receive_status()
            .map_err(BedrockRuntimeError::Transport)?;
        let line = match status {
            SidecarReceive::Line(line) => line,
            SidecarReceive::Pending => return Ok(None),
            SidecarReceive::Eof => {
                self.state = BedrockRuntimeState::Unavailable;
                return Err(BedrockRuntimeError::SidecarEof);
            }
        };
        let frame = decode_frame(&line)?;
        match frame {
            SidecarFrame::Ready {
                guest_ip,
                port,
                relay_up,
            } => {
                self.require_state("accept readiness", &[BedrockRuntimeState::Starting])?;
                if !relay_up {
                    return Err(BedrockRuntimeError::Protocol(
                        "ready arrived before the relay was up".to_owned(),
                    ));
                }
                self.state = BedrockRuntimeState::Running;
                Ok(Some(BedrockRuntimeEvent::Ready {
                    address: Some(guest_ip),
                    port,
                }))
            }
            SidecarFrame::ConsoleLine { line } => {
                self.require_state(
                    "accept console output",
                    &[
                        BedrockRuntimeState::Starting,
                        BedrockRuntimeState::Running,
                        BedrockRuntimeState::Stopping,
                    ],
                )?;
                Ok(Some(BedrockRuntimeEvent::ConsoleLine(line)))
            }
            SidecarFrame::Terminated { reason } => {
                self.require_state(
                    "accept termination",
                    &[
                        BedrockRuntimeState::Starting,
                        BedrockRuntimeState::Running,
                        BedrockRuntimeState::Stopping,
                    ],
                )?;
                self.state = BedrockRuntimeState::Stopped;
                Ok(Some(BedrockRuntimeEvent::Terminated { reason }))
            }
            frame => Err(BedrockRuntimeError::Protocol(format!(
                "unexpected sidecar event frame {frame:?}"
            ))),
        }
    }
}
