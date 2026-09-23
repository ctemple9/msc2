//! Child-process stdio supervision for the macOS Bedrock Swift sidecar.
//!
//! The application crate owns the JSON protocol and lifecycle state. This
//! module only turns the existing process-supervisor events into complete
//! stdout lines and writes encoded frames to the sidecar's stdin.

use crate::process::{
    OutputLineFramer, OutputStream, ProcessError, ProcessEvent, ProcessId, ProcessSpawnRequest,
    ProcessSupervisor,
};
use std::collections::VecDeque;
#[cfg(unix)]
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub const BEDROCK_HELPER_SOCKET_PATH: &str = "/var/run/msc2/bedrock.sock";
const BEDROCK_HELPER_SOCKET_MODE: u32 = 0o600;
pub const MAX_SIDECAR_FRAME_BYTES: usize = 64 * 1024;

pub const SIDECAR_STDOUT: OutputStream = OutputStream::Stdout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarReceive {
    Line(String),
    Pending,
    Eof,
}

#[cfg(unix)]
#[derive(Debug)]
pub enum BedrockSidecarSocketError {
    Missing {
        path: PathBuf,
    },
    NotSocket {
        path: PathBuf,
    },
    WrongOwner {
        path: PathBuf,
        expected: u32,
        actual: u32,
    },
    WrongMode {
        path: PathBuf,
        actual: u32,
    },
    Connect {
        path: PathBuf,
        message: String,
    },
    Configure {
        path: PathBuf,
        message: String,
    },
}

#[cfg(unix)]
impl BedrockSidecarSocketError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "helper_socket_missing",
            Self::NotSocket { .. } => "helper_socket_wrong_file_type",
            Self::WrongOwner { .. } => "helper_socket_wrong_owner",
            Self::WrongMode { .. } => "helper_socket_wrong_mode",
            Self::Connect { .. } => "helper_socket_unavailable",
            Self::Configure { .. } => "helper_socket_configuration_failed",
        }
    }
}

#[cfg(unix)]
impl std::fmt::Display for BedrockSidecarSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { path } => write!(f, "helper socket is missing: {}", path.display()),
            Self::NotSocket { path } => write!(
                f,
                "helper socket path is not a Unix socket: {}",
                path.display()
            ),
            Self::WrongOwner {
                path,
                expected,
                actual,
            } => write!(
                f,
                "helper socket {} is owned by UID {actual}, expected installing UID {expected}",
                path.display()
            ),
            Self::WrongMode { path, actual } => write!(
                f,
                "helper socket {} has mode {actual:04o}, expected 0600",
                path.display()
            ),
            Self::Connect { path, message } => {
                write!(
                    f,
                    "could not connect to helper socket {}: {message}",
                    path.display()
                )
            }
            Self::Configure { path, message } => write!(
                f,
                "could not configure helper socket {}: {message}",
                path.display()
            ),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for BedrockSidecarSocketError {}

/// A nonblocking Unix-domain transport for the installed root-owned helper.
///
/// The helper owns the VM and relays; this object only carries the existing
/// newline-delimited sidecar frames. The first EOF is reported as a rejected
/// peer because the helper closes an unauthorized connection before accepting
/// a request. EOF after a frame is the distinct helper-disconnected case.
#[cfg(unix)]
pub struct BedrockSidecarSocket {
    stream: UnixStream,
    path: PathBuf,
    read_buffer: Vec<u8>,
    pending_lines: VecDeque<String>,
    eof: bool,
    saw_frame: bool,
}

#[cfg(unix)]
impl BedrockSidecarSocket {
    pub fn connect(path: impl Into<PathBuf>) -> Result<Self, BedrockSidecarSocketError> {
        let path = path.into();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                BedrockSidecarSocketError::Missing { path: path.clone() }
            } else {
                BedrockSidecarSocketError::Connect {
                    path: path.clone(),
                    message: error.to_string(),
                }
            }
        })?;
        if !metadata.file_type().is_socket() {
            return Err(BedrockSidecarSocketError::NotSocket { path });
        }

        let expected_uid = unsafe { libc::geteuid() };
        let actual_uid = metadata.uid();
        if actual_uid != expected_uid {
            return Err(BedrockSidecarSocketError::WrongOwner {
                path,
                expected: expected_uid,
                actual: actual_uid,
            });
        }

        let actual_mode = metadata.permissions().mode() & 0o777;
        if actual_mode != BEDROCK_HELPER_SOCKET_MODE {
            return Err(BedrockSidecarSocketError::WrongMode {
                path,
                actual: actual_mode,
            });
        }

        let stream =
            UnixStream::connect(&path).map_err(|error| BedrockSidecarSocketError::Connect {
                path: path.clone(),
                message: error.to_string(),
            })?;
        stream
            .set_nonblocking(true)
            .map_err(|error| BedrockSidecarSocketError::Configure {
                path: path.clone(),
                message: error.to_string(),
            })?;
        Ok(Self {
            stream,
            path,
            read_buffer: Vec::new(),
            pending_lines: VecDeque::new(),
            eof: false,
            saw_frame: false,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.path
    }

    pub fn send_line(&mut self, line: &str) -> Result<(), String> {
        if line.len() > MAX_SIDECAR_FRAME_BYTES {
            return Err(format!(
                "helper protocol request exceeds {MAX_SIDECAR_FRAME_BYTES} bytes"
            ));
        }
        self.stream
            .write_all(line.as_bytes())
            .map_err(|error| format!("helper disconnected while sending a request: {error}"))
    }

    pub fn receive(&mut self) -> Result<SidecarReceive, String> {
        if let Some(line) = self.pending_lines.pop_front() {
            return Ok(SidecarReceive::Line(line));
        }
        if self.eof {
            return Ok(SidecarReceive::Eof);
        }

        let mut bytes = [0_u8; 8192];
        match self.stream.read(&mut bytes) {
            Ok(0) => {
                self.eof = true;
                if self.read_buffer.is_empty() && !self.saw_frame {
                    return Err("helper rejected the agent peer".to_owned());
                }
                if !self.read_buffer.is_empty() {
                    return Err("helper disconnected with a truncated protocol frame".to_owned());
                }
                return Err("helper disconnected and tore down the Bedrock session".to_owned());
            }
            Ok(count) => {
                self.saw_frame = true;
                self.read_buffer.extend_from_slice(&bytes[..count]);
                self.frame_lines()?
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                return Ok(SidecarReceive::Pending);
            }
            Err(error) => {
                return Err(format!(
                    "helper disconnected while receiving a frame: {error}"
                ));
            }
        }

        if let Some(line) = self.pending_lines.pop_front() {
            Ok(SidecarReceive::Line(line))
        } else {
            Ok(SidecarReceive::Pending)
        }
    }

    fn frame_lines(&mut self) -> Result<(), String> {
        while let Some(newline) = self.read_buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.read_buffer.drain(..=newline).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.len() > MAX_SIDECAR_FRAME_BYTES {
                return Err(format!(
                    "incompatible helper protocol: response exceeds {MAX_SIDECAR_FRAME_BYTES} bytes"
                ));
            }
            let line = String::from_utf8(line)
                .map_err(|_| "incompatible helper protocol: response is not UTF-8".to_owned())?;
            self.pending_lines.push_back(line);
        }
        if self.read_buffer.len() > MAX_SIDECAR_FRAME_BYTES {
            return Err(format!(
                "incompatible helper protocol: response exceeds {MAX_SIDECAR_FRAME_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for BedrockSidecarSocket {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

/// A process-supervisor-backed transport for one Swift sidecar instance.
pub struct BedrockSidecarProcess<'supervisor> {
    process_supervisor: &'supervisor dyn ProcessSupervisor,
    process: ProcessId,
    stdout_framer: OutputLineFramer,
    pending_lines: VecDeque<String>,
    eof: bool,
}

impl<'supervisor> BedrockSidecarProcess<'supervisor> {
    pub fn spawn(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        executable_path: impl Into<PathBuf>,
        working_directory: impl Into<PathBuf>,
    ) -> Result<Self, ProcessError> {
        let process =
            process_supervisor.spawn(sidecar_spawn_request(executable_path, working_directory))?;
        Ok(Self::from_process(process_supervisor, process))
    }

    pub fn from_process(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        process: ProcessId,
    ) -> Self {
        Self {
            process_supervisor,
            process,
            stdout_framer: OutputLineFramer::new(),
            pending_lines: VecDeque::new(),
            eof: false,
        }
    }

    pub fn process_id(&self) -> ProcessId {
        self.process
    }

    pub fn send_line(&self, line: &str) -> Result<(), ProcessError> {
        self.process_supervisor
            .write_stdin(self.process, line.as_bytes())
    }

    pub fn receive(&mut self) -> Result<SidecarReceive, ProcessError> {
        if let Some(line) = self.pending_lines.pop_front() {
            return Ok(SidecarReceive::Line(line));
        }
        if self.eof {
            return Ok(SidecarReceive::Eof);
        }

        for event in self.process_supervisor.drain_events(self.process)? {
            match event {
                ProcessEvent::Output { stream, bytes } if stream == SIDECAR_STDOUT => {
                    self.pending_lines.extend(self.stdout_framer.push(&bytes));
                }
                ProcessEvent::Output { .. } => {
                    // Diagnostics on stderr must not be mistaken for a
                    // protocol frame; the sidecar's contract is stdout-only.
                }
                ProcessEvent::Exited(_) => {
                    if let Some(line) = self.stdout_framer.flush() {
                        self.pending_lines.push_back(line);
                    }
                    self.eof = true;
                }
            }
        }

        if let Some(line) = self.pending_lines.pop_front() {
            Ok(SidecarReceive::Line(line))
        } else if self.eof {
            Ok(SidecarReceive::Eof)
        } else {
            Ok(SidecarReceive::Pending)
        }
    }

    pub fn force_terminate(&mut self) -> Result<(), ProcessError> {
        if self.eof {
            return Ok(());
        }
        self.process_supervisor.force_terminate(self.process)?;
        self.eof = true;
        Ok(())
    }
}

impl Drop for BedrockSidecarProcess<'_> {
    fn drop(&mut self) {
        if !self.eof {
            let _ = self.process_supervisor.force_terminate(self.process);
            self.eof = true;
        }
    }
}

pub fn sidecar_spawn_request(
    executable_path: impl Into<PathBuf>,
    working_directory: impl Into<PathBuf>,
) -> ProcessSpawnRequest {
    ProcessSpawnRequest::new(executable_path, working_directory)
}

pub fn sidecar_spawn_request_from_paths(
    executable_path: &Path,
    working_directory: &Path,
) -> ProcessSpawnRequest {
    sidecar_spawn_request(executable_path.to_owned(), working_directory.to_owned())
}
