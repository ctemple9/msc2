//! Real Java process supervision for Windows.
//!
//! Every spawned server process is assigned to a Job Object. Force
//! termination uses the job rather than the root process handle so Java
//! helper children are cleaned up before the service-management layer lands.

use msc_infrastructure::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessExitStatus, ProcessId, ProcessSpawnRequest,
    ProcessSupervisor,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;
use std::process::{ChildStdin, Command, Stdio};
use std::ptr;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

#[derive(Debug, Default)]
pub struct WindowsJavaProcessSupervisor {
    processes: Mutex<BTreeMap<ProcessId, RealProcess>>,
}

#[derive(Debug)]
struct RealProcess {
    stdin: Mutex<Option<ChildStdin>>,
    events: Arc<Mutex<Vec<ProcessEvent>>>,
    running: Arc<AtomicBool>,
    job: JobHandle,
}

#[derive(Debug)]
struct JobHandle(HANDLE);

// A Windows HANDLE is an opaque kernel handle. Moving the numeric handle value
// between threads is safe as long as ownership is still closed exactly once by
// this wrapper's Drop implementation.
unsafe impl Send for JobHandle {}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this wrapper owns the handle and closes it once.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

impl WindowsJavaProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProcessSupervisor for WindowsJavaProcessSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        let job = create_kill_on_close_job()?;
        let mut child = Command::new(&request.executable_path)
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .envs(request.environment.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ProcessError::Spawn(format!("spawning Java process: {e}")))?;

        // SAFETY: both handles are valid here: `job` was just created, and
        // `child.as_raw_handle()` borrows the live process handle owned by
        // `child`.
        let assigned = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle() as HANDLE) };
        if assigned == 0 {
            let err = unsafe { GetLastError() };
            return Err(ProcessError::Spawn(format!(
                "assigning process to Job Object: Win32 error {err}"
            )));
        }

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
                job,
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
        // SAFETY: `process.job` is a live Job Object handle owned by the
        // supervisor entry. TerminateJobObject kills all assigned descendants.
        let ok = unsafe { TerminateJobObject(process.job.0, 1) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            Err(ProcessError::Terminate(format!(
                "terminating Job Object for process {}: Win32 error {err}",
                pid.raw()
            )))
        } else {
            Ok(())
        }
    }

    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;
        Ok(std::mem::take(&mut process.events.lock().unwrap()))
    }
}

fn create_kill_on_close_job() -> Result<JobHandle, ProcessError> {
    // SAFETY: null security attributes and null name are permitted by
    // CreateJobObjectW; the returned handle is owned by JobHandle.
    let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if handle.is_null() {
        let err = unsafe { GetLastError() };
        return Err(ProcessError::Spawn(format!(
            "creating Job Object: Win32 error {err}"
        )));
    }
    let job = JobHandle(handle);
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    // SAFETY: `limits` points to a properly initialized
    // JOBOBJECT_EXTENDED_LIMIT_INFORMATION value for the duration of the call.
    let ok = unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        return Err(ProcessError::Spawn(format!(
            "configuring Job Object kill-on-close: Win32 error {err}"
        )));
    }
    Ok(job)
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
                signal: None,
            },
            Err(_) => ProcessExitStatus {
                code: None,
                signal: None,
            },
        };
        events.lock().unwrap().push(ProcessEvent::Exited(exit));
    });
}
