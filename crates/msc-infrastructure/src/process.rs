//! Process supervision boundary for Java lifecycle work.
//!
//! The real platform implementations land in P4.10. This module defines the
//! cross-platform contract and a fake harness so application lifecycle tests
//! can exercise process output, stdin, and exits without starting Java.

use crate::console_buffer::ConsoleLineFramer;

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(u32);

impl ProcessId {
    pub fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpawnRequest {
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: Vec<(String, String)>,
}

impl ProcessSpawnRequest {
    pub fn new(executable_path: impl Into<PathBuf>, working_directory: impl Into<PathBuf>) -> Self {
        Self {
            executable_path: executable_path.into(),
            arguments: Vec::new(),
            working_directory: working_directory.into(),
            environment: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.arguments.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.arguments.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExitStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl ProcessExitStatus {
    pub fn exited(code: i32) -> Self {
        Self {
            code: Some(code),
            signal: None,
        }
    }

    pub fn signaled(signal: i32) -> Self {
        Self {
            code: None,
            signal: Some(signal),
        }
    }

    pub fn success(self) -> bool {
        self.code == Some(0) && self.signal.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    Output {
        stream: OutputStream,
        bytes: Vec<u8>,
    },
    Exited(ProcessExitStatus),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    NotFound(ProcessId),
    NotRunning(ProcessId),
    Spawn(String),
    Stdin(String),
    Terminate(String),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(pid) => write!(f, "process not found: {}", pid.raw()),
            Self::NotRunning(pid) => write!(f, "process is not running: {}", pid.raw()),
            Self::Spawn(message) | Self::Stdin(message) | Self::Terminate(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for ProcessError {}

pub trait ProcessSupervisor: Send + Sync {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError>;
    fn write_stdin(&self, pid: ProcessId, bytes: &[u8]) -> Result<(), ProcessError>;

    fn request_graceful_stop(&self, pid: ProcessId) -> Result<(), ProcessError> {
        self.write_stdin(pid, b"stop\n")
    }

    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError>;
    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError>;
}

#[derive(Debug, Default)]
pub struct OutputLineFramer {
    inner: ConsoleLineFramer,
}

impl OutputLineFramer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.inner.push_bytes(bytes)
    }

    pub fn flush(&mut self) -> Option<String> {
        self.inner.flush()
    }

    pub fn push_event(&mut self, event: &ProcessEvent) -> Vec<String> {
        match event {
            ProcessEvent::Output { bytes, .. } => self.push(bytes),
            ProcessEvent::Exited(_) => self.flush().into_iter().collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct FakeProcessSupervisor {
    state: Mutex<FakeState>,
}

#[derive(Debug)]
struct FakeState {
    next_pid: u32,
    processes: BTreeMap<ProcessId, FakeProcess>,
    spawned: Vec<(ProcessId, ProcessSpawnRequest)>,
    graceful_stops: Vec<ProcessId>,
    force_terminations: Vec<ProcessId>,
    fail_next_spawn: Option<String>,
    fail_next_stdin: Option<String>,
}

impl Default for FakeState {
    fn default() -> Self {
        Self {
            next_pid: 1000,
            processes: BTreeMap::new(),
            spawned: Vec::new(),
            graceful_stops: Vec::new(),
            force_terminations: Vec::new(),
            fail_next_spawn: None,
            fail_next_stdin: None,
        }
    }
}

#[derive(Debug)]
struct FakeProcess {
    running: bool,
    stdin_writes: Vec<Vec<u8>>,
    events: Vec<ProcessEvent>,
}

impl FakeProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawned_requests(&self) -> Vec<(ProcessId, ProcessSpawnRequest)> {
        self.state.lock().unwrap().spawned.clone()
    }

    pub fn fail_next_spawn(&self, message: impl Into<String>) {
        self.state.lock().unwrap().fail_next_spawn = Some(message.into());
    }

    pub fn fail_next_stdin(&self, message: impl Into<String>) {
        self.state.lock().unwrap().fail_next_stdin = Some(message.into());
    }

    pub fn stdin_writes(&self, pid: ProcessId) -> Result<Vec<Vec<u8>>, ProcessError> {
        let state = self.state.lock().unwrap();
        let process = state
            .processes
            .get(&pid)
            .ok_or(ProcessError::NotFound(pid))?;
        Ok(process.stdin_writes.clone())
    }

    pub fn graceful_stops(&self) -> Vec<ProcessId> {
        self.state.lock().unwrap().graceful_stops.clone()
    }

    pub fn force_terminations(&self) -> Vec<ProcessId> {
        self.state.lock().unwrap().force_terminations.clone()
    }

    pub fn emit_stdout(
        &self,
        pid: ProcessId,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), ProcessError> {
        self.emit_output(pid, OutputStream::Stdout, bytes)
    }

    pub fn emit_stderr(
        &self,
        pid: ProcessId,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), ProcessError> {
        self.emit_output(pid, OutputStream::Stderr, bytes)
    }

    pub fn exit_normally(&self, pid: ProcessId) -> Result<(), ProcessError> {
        self.exit(pid, ProcessExitStatus::exited(0))
    }

    pub fn crash(&self, pid: ProcessId, code: i32) -> Result<(), ProcessError> {
        self.exit(pid, ProcessExitStatus::exited(code))
    }

    fn emit_output(
        &self,
        pid: ProcessId,
        stream: OutputStream,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), ProcessError> {
        let mut state = self.state.lock().unwrap();
        let process = state
            .processes
            .get_mut(&pid)
            .ok_or(ProcessError::NotFound(pid))?;
        if !process.running {
            return Err(ProcessError::NotRunning(pid));
        }
        process.events.push(ProcessEvent::Output {
            stream,
            bytes: bytes.into(),
        });
        Ok(())
    }

    fn exit(&self, pid: ProcessId, status: ProcessExitStatus) -> Result<(), ProcessError> {
        let mut state = self.state.lock().unwrap();
        let process = state
            .processes
            .get_mut(&pid)
            .ok_or(ProcessError::NotFound(pid))?;
        if !process.running {
            return Err(ProcessError::NotRunning(pid));
        }
        process.running = false;
        process.events.push(ProcessEvent::Exited(status));
        Ok(())
    }
}

impl ProcessSupervisor for FakeProcessSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        let mut state = self.state.lock().unwrap();
        if let Some(message) = state.fail_next_spawn.take() {
            return Err(ProcessError::Spawn(message));
        }
        let pid = ProcessId::new(state.next_pid);
        state.next_pid += 1;
        state.spawned.push((pid, request));
        state.processes.insert(
            pid,
            FakeProcess {
                running: true,
                stdin_writes: Vec::new(),
                events: Vec::new(),
            },
        );
        Ok(pid)
    }

    fn write_stdin(&self, pid: ProcessId, bytes: &[u8]) -> Result<(), ProcessError> {
        let mut state = self.state.lock().unwrap();
        if let Some(message) = state.fail_next_stdin.take() {
            return Err(ProcessError::Stdin(message));
        }
        let process = state
            .processes
            .get_mut(&pid)
            .ok_or(ProcessError::NotFound(pid))?;
        if !process.running {
            return Err(ProcessError::NotRunning(pid));
        }
        process.stdin_writes.push(bytes.to_vec());
        Ok(())
    }

    fn request_graceful_stop(&self, pid: ProcessId) -> Result<(), ProcessError> {
        self.write_stdin(pid, b"stop\n")?;
        self.state.lock().unwrap().graceful_stops.push(pid);
        Ok(())
    }

    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError> {
        {
            let mut state = self.state.lock().unwrap();
            state.force_terminations.push(pid);
        }
        self.exit(pid, ProcessExitStatus::signaled(15))
    }

    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        let mut state = self.state.lock().unwrap();
        let process = state
            .processes
            .get_mut(&pid)
            .ok_or(ProcessError::NotFound(pid))?;
        Ok(std::mem::take(&mut process.events))
    }
}
