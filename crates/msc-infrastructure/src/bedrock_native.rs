//! Native Bedrock process support.
//!
//! Linux and Windows BDS binaries own the game UDP socket themselves.  This
//! module therefore performs a short bind preflight, builds the child-process
//! request, and provides the real Linux process supervisor used by the first
//! native runtime.  The preflight socket is dropped before spawning so BDS can
//! bind the port it will actually serve.

use crate::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessExitStatus, ProcessId, ProcessSpawnRequest,
    ProcessSupervisor,
};
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub const BEDROCK_EXECUTABLE_NAME: &str = "bedrock_server";
pub const WINDOWS_BEDROCK_EXECUTABLE_NAME: &str = "bedrock_server.exe";
pub const BEDROCK_BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBedrockHost {
    Linux,
    Windows,
    Macos,
    Other,
}

impl NativeBedrockHost {
    pub const fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            return Self::Linux;
        }
        #[cfg(target_os = "windows")]
        {
            return Self::Windows;
        }
        #[cfg(target_os = "macos")]
        {
            return Self::Macos;
        }
        #[allow(unreachable_code)]
        Self::Other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeBedrockError {
    UdpPortInUse {
        address: IpAddr,
        port: u16,
    },
    UdpBind {
        address: IpAddr,
        port: u16,
        message: String,
    },
}

impl fmt::Display for NativeBedrockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UdpPortInUse { address, port } => {
                write!(f, "udp-port-in-use: {address}:{port}")
            }
            Self::UdpBind {
                address,
                port,
                message,
            } => write!(f, "udp-bind-failed: {address}:{port}: {message}"),
        }
    }
}

impl std::error::Error for NativeBedrockError {}

/// Checks that the native BDS UDP endpoint is available without inserting a
/// relay.  The socket is intentionally closed before returning; keeping it
/// open would prevent the child BDS process from becoming the port owner.
pub fn preflight_udp_bind(address: IpAddr, port: u16) -> Result<(), NativeBedrockError> {
    let socket = UdpSocket::bind(SocketAddr::new(address, port)).map_err(|error| {
        if error.kind() == io::ErrorKind::AddrInUse {
            NativeBedrockError::UdpPortInUse { address, port }
        } else {
            NativeBedrockError::UdpBind {
                address,
                port,
                message: error.to_string(),
            }
        }
    })?;
    drop(socket);
    Ok(())
}

pub fn linux_bedrock_spawn_request(server_dir: impl AsRef<Path>) -> ProcessSpawnRequest {
    let server_dir = server_dir.as_ref();
    ProcessSpawnRequest::new(
        server_dir.join(BEDROCK_EXECUTABLE_NAME),
        server_dir.to_path_buf(),
    )
}

pub fn windows_bedrock_spawn_request(server_dir: impl AsRef<Path>) -> ProcessSpawnRequest {
    let server_dir = server_dir.as_ref();
    ProcessSpawnRequest::new(
        server_dir.join(WINDOWS_BEDROCK_EXECUTABLE_NAME),
        server_dir.to_path_buf(),
    )
}

type OutputQueue = Arc<Mutex<VecDeque<ProcessEvent>>>;

struct ManagedLinuxProcess {
    child: Child,
    stdin: ChildStdin,
    output: OutputQueue,
    readers: Vec<JoinHandle<()>>,
    exit_status: Option<ProcessExitStatus>,
}

/// A small real process supervisor for native Linux BDS.
///
/// The shared process trait is synchronous.  Reader threads keep stdout and
/// stderr flowing without blocking the agent, while `drain_events` performs
/// the non-blocking child-exit check and preserves the two stream identities.
#[derive(Default)]
pub struct LinuxBedrockProcessSupervisor {
    processes: Mutex<BTreeMap<ProcessId, ManagedLinuxProcess>>,
}

impl LinuxBedrockProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    fn start_reader<R>(mut reader: R, stream: OutputStream, output: OutputQueue) -> JoinHandle<()>
    where
        R: Read + Send + 'static,
    {
        thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        output
                            .lock()
                            .expect("process output queue poisoned")
                            .push_back(ProcessEvent::Output {
                                stream,
                                bytes: buffer[..count].to_vec(),
                            });
                    }
                    Err(_) => break,
                }
            }
        })
    }

    fn process_mut(
        &self,
        pid: ProcessId,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<ProcessId, ManagedLinuxProcess>>, ProcessError>
    {
        let processes = self.processes.lock().expect("process table poisoned");
        if processes.contains_key(&pid) {
            Ok(processes)
        } else {
            Err(ProcessError::NotFound(pid))
        }
    }
}

impl ProcessSupervisor for LinuxBedrockProcessSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        let mut command = Command::new(&request.executable_path);
        command
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &request.environment {
            command.env(key, value);
        }

        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::Spawn(error.to_string()))?;
        let pid = ProcessId::new(child.id());
        let stdin = child.stdin.take().ok_or_else(|| {
            ProcessError::Spawn("native Bedrock process has no stdin pipe".to_owned())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ProcessError::Spawn("native Bedrock process has no stdout pipe".to_owned())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ProcessError::Spawn("native Bedrock process has no stderr pipe".to_owned())
        })?;
        let output = Arc::new(Mutex::new(VecDeque::new()));
        let readers = vec![
            Self::start_reader(stdout, OutputStream::Stdout, Arc::clone(&output)),
            Self::start_reader(stderr, OutputStream::Stderr, Arc::clone(&output)),
        ];

        self.processes
            .lock()
            .expect("process table poisoned")
            .insert(
                pid,
                ManagedLinuxProcess {
                    child,
                    stdin,
                    output,
                    readers,
                    exit_status: None,
                },
            );
        Ok(pid)
    }

    fn write_stdin(&self, pid: ProcessId, bytes: &[u8]) -> Result<(), ProcessError> {
        let mut processes = self.process_mut(pid)?;
        let process = processes.get_mut(&pid).expect("process checked above");
        process
            .stdin
            .write_all(bytes)
            .and_then(|_| process.stdin.flush())
            .map_err(|error| ProcessError::Stdin(error.to_string()))
    }

    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError> {
        let mut processes = self.process_mut(pid)?;
        let process = processes.get_mut(&pid).expect("process checked above");
        process
            .child
            .kill()
            .map_err(|error| ProcessError::Terminate(error.to_string()))
    }

    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        let mut processes = self.process_mut(pid)?;
        let process = processes.get_mut(&pid).expect("process checked above");
        let child_status = process
            .child
            .try_wait()
            .map_err(|error| ProcessError::Terminate(error.to_string()))?;

        if let Some(status) = child_status
            && process.exit_status.is_none()
        {
            let readers = std::mem::take(&mut process.readers);
            for reader in readers {
                let _ = reader.join();
            }
            process.exit_status = Some(match status.code() {
                Some(code) => ProcessExitStatus::exited(code),
                None => ProcessExitStatus::signaled(-1),
            });
        }

        let mut events = process
            .output
            .lock()
            .expect("process output queue poisoned")
            .drain(..)
            .collect::<Vec<_>>();
        if let Some(status) = process.exit_status.take() {
            events.push(ProcessEvent::Exited(status));
        }
        Ok(events)
    }
}
