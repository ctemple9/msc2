//! Real Java process supervision for Linux.
//!
//! This mirrors the macOS P4.10 supervisor: start the child in a separate
//! POSIX process group, collect stdout/stderr as raw byte events, and emit
//! the exit event only after pipe readers have drained.

use msc_infrastructure::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessExitStatus, ProcessId, ProcessSpawnRequest,
    ProcessSupervisor,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

#[derive(Debug, Default)]
pub struct LinuxJavaProcessSupervisor {
    processes: Mutex<BTreeMap<ProcessId, RealProcess>>,
}

#[derive(Debug)]
struct RealProcess {
    stdin: Mutex<Option<ChildStdin>>,
    events: Arc<Mutex<Vec<ProcessEvent>>>,
    running: Arc<AtomicBool>,
}

impl LinuxJavaProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProcessSupervisor for LinuxJavaProcessSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        let mut command = Command::new(&request.executable_path);
        command
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .envs(request.environment.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // SAFETY: `setpgid(0, 0)` runs in the child after fork and before exec.
        // It only asks the OS to make the child the leader of a new process group.
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }

        let mut child = command
            .spawn()
            .map_err(|e| ProcessError::Spawn(format!("spawning Java process: {e}")))?;
        let pid = ProcessId::new(child.id());
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let events = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(AtomicBool::new(true));

        let stdout_reader = stdout.map(|stream| read_stream(stream, OutputStream::Stdout, &events));
        let stderr_reader = stderr.map(|stream| read_stream(stream, OutputStream::Stderr, &events));
        wait_for_exit(child, stdout_reader, stderr_reader, &events, &running);

        self.processes.lock().unwrap().insert(
            pid,
            RealProcess {
                stdin: Mutex::new(stdin),
                events,
                running,
            },
        );
        Ok(pid)
    }

    fn write_stdin(&self, pid: ProcessId, bytes: &[u8]) -> Result<(), ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;
        if !process.running.load(Ordering::SeqCst) {
            return Err(ProcessError::NotRunning(pid));
        }

        let mut stdin = process.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(ProcessError::NotRunning(pid))?;
        stdin
            .write_all(bytes)
            .and_then(|()| stdin.flush())
            .map_err(|e| {
                ProcessError::Stdin(format!("writing to process {} stdin: {e}", pid.raw()))
            })
    }

    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;
        if !process.running.load(Ordering::SeqCst) {
            return Err(ProcessError::NotRunning(pid));
        }
        let pgid = -(pid.raw() as libc::pid_t);
        // SAFETY: `kill` is called with a process-group id created by `setpgid`
        // in `spawn`; no Rust references cross the FFI boundary.
        let result = unsafe { libc::kill(pgid, libc::SIGTERM) };
        if result == 0 {
            Ok(())
        } else {
            Err(ProcessError::Terminate(format!(
                "terminating process group {}: {}",
                pid.raw(),
                std::io::Error::last_os_error()
            )))
        }
    }

    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;
        Ok(std::mem::take(&mut process.events.lock().unwrap()))
    }
}

fn read_stream<R: Read + Send + 'static>(
    mut stream: R,
    output_stream: OutputStream,
    events: &Arc<Mutex<Vec<ProcessEvent>>>,
) -> thread::JoinHandle<()> {
    let events = Arc::clone(events);
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => events.lock().unwrap().push(ProcessEvent::Output {
                    stream: output_stream,
                    bytes: buffer[..n].to_vec(),
                }),
                Err(_) => break,
            }
        }
    })
}

fn wait_for_exit(
    mut child: std::process::Child,
    stdout_reader: Option<thread::JoinHandle<()>>,
    stderr_reader: Option<thread::JoinHandle<()>>,
    events: &Arc<Mutex<Vec<ProcessEvent>>>,
    running: &Arc<AtomicBool>,
) {
    let events = Arc::clone(events);
    let running = Arc::clone(running);
    thread::spawn(move || {
        let status = child.wait();
        if let Some(reader) = stdout_reader {
            let _ = reader.join();
        }
        if let Some(reader) = stderr_reader {
            let _ = reader.join();
        }
        running.store(false, Ordering::SeqCst);
        let exit = match status {
            Ok(status) => ProcessExitStatus {
                code: status.code(),
                signal: status.signal(),
            },
            Err(_) => ProcessExitStatus {
                code: None,
                signal: None,
            },
        };
        events.lock().unwrap().push(ProcessEvent::Exited(exit));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn request(script: &str) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new("/bin/sh", std::env::temp_dir()).args(["-c", script])
    }

    // 30s, not 5s: same real-process-spawn class as
    // `msc-platform-windows/tests/job_object.rs`'s own `drain_until_exit`,
    // which P7.29's CI runs found missing a 10s budget under heavy
    // concurrent nextest load on GitHub's hosted runners. Raised here too
    // for the same margin, even though this file hadn't been observed
    // failing yet.
    fn drain_until_exit(
        supervisor: &LinuxJavaProcessSupervisor,
        pid: ProcessId,
    ) -> Vec<ProcessEvent> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut events = Vec::new();
        while Instant::now() < deadline {
            events.extend(supervisor.drain_events(pid).unwrap());
            if events
                .iter()
                .any(|event| matches!(event, ProcessEvent::Exited(_)))
            {
                return events;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for process exit; events: {events:?}");
    }

    #[test]
    fn process_supervisor_real_streams_stdin_stdout_stderr_and_exit_code() {
        let supervisor = LinuxJavaProcessSupervisor::new();
        let pid = supervisor
            .spawn(request(
                "printf 'out\\n'; printf 'err\\n' >&2; IFS= read line; printf \"cmd:$line\\n\"; exit 7",
            ))
            .unwrap();

        supervisor.write_stdin(pid, b"say hello\n").unwrap();
        let events = drain_until_exit(&supervisor, pid);

        assert!(events.iter().any(|event| matches!(
            event,
            ProcessEvent::Output {
                stream: OutputStream::Stdout,
                bytes
            } if String::from_utf8_lossy(bytes).contains("out\n")
                || String::from_utf8_lossy(bytes).contains("cmd:say hello\n")
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ProcessEvent::Output {
                stream: OutputStream::Stderr,
                bytes
            } if String::from_utf8_lossy(bytes).contains("err\n")
        )));
        assert!(
            events.iter().any(
                |event| matches!(event, ProcessEvent::Exited(status) if status.code == Some(7))
            )
        );
    }

    #[test]
    fn process_supervisor_real_graceful_stop_writes_stop_newline() {
        let supervisor = LinuxJavaProcessSupervisor::new();
        let pid = supervisor
            .spawn(request(
                "IFS= read line; if [ \"$line\" = stop ]; then exit 0; else exit 2; fi",
            ))
            .unwrap();

        supervisor.request_graceful_stop(pid).unwrap();
        let events = drain_until_exit(&supervisor, pid);

        assert!(
            events
                .iter()
                .any(|event| matches!(event, ProcessEvent::Exited(status) if status.success()))
        );
    }

    #[test]
    fn process_supervisor_real_force_terminate_kills_process_group() {
        let supervisor = LinuxJavaProcessSupervisor::new();
        let pid = supervisor
            .spawn(ProcessSpawnRequest::new("/bin/sh", PathBuf::from("/")).args(["-c", "sleep 30"]))
            .unwrap();

        supervisor.force_terminate(pid).unwrap();
        let events = drain_until_exit(&supervisor, pid);

        assert!(events.iter().any(|event| matches!(
            event,
            ProcessEvent::Exited(status) if status.signal == Some(libc::SIGTERM)
        )));
    }
}
