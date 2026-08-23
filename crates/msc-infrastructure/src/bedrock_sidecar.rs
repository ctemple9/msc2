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
use std::path::{Path, PathBuf};

pub const SIDECAR_STDOUT: OutputStream = OutputStream::Stdout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarReceive {
    Line(String),
    Pending,
    Eof,
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
