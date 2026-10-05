//! Own the dedicated launcher tree; cancellation cannot target an ordinary client.
use std::path::Path;

#[cfg(unix)]
pub struct Process(Option<std::process::Child>);
#[cfg(unix)]
impl Process {
    pub fn launch(launcher: &Path, root: &Path, id: &str) -> std::io::Result<Self> {
        use std::os::unix::process::CommandExt;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("launcher.log"))?;
        Ok(Self(Some(
            std::process::Command::new(launcher)
                .arg("--dir")
                .arg(root)
                .arg("--launch")
                .arg(id)
                .current_dir(root)
                .stdin(std::process::Stdio::null())
                .stdout(log.try_clone()?)
                .stderr(log)
                .process_group(0)
                .spawn()?,
        )))
    }
    pub fn stop(&mut self) -> bool {
        // Do not reap the leader first. Its PID remains reserved even if Prism
        // exited, so the owned group cannot be confused with a reused process ID.
        let Some(mut child) = self.0.take() else {
            return true;
        };
        let group_stopped = std::process::Command::new("/bin/kill")
            .args(["-KILL", &format!("-{}", child.id())])
            .output()
            .is_ok_and(|output| output.status.success());
        let _ = child.kill();
        let leader_stopped = child.wait().is_ok();
        group_stopped && leader_stopped
    }
}
#[cfg(unix)]
impl Drop for Process {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, ResumeThread, TerminateProcess, WaitForSingleObject,
        CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
    };

    pub struct Process {
        job: OwnedHandle,
        process: OwnedHandle,
    }
    fn quote(value: &std::ffi::OsStr) -> Vec<u16> {
        let mut out = vec![b'"' as u16];
        let mut slashes = 0;
        for c in value.encode_wide() {
            if c == b'\\' as u16 {
                slashes += 1;
                continue;
            }
            out.extend(std::iter::repeat_n(
                b'\\' as u16,
                if c == b'"' as u16 {
                    slashes * 2 + 1
                } else {
                    slashes
                },
            ));
            slashes = 0;
            out.push(c);
        }
        out.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        out.push(b'"' as u16);
        out
    }
    impl Process {
        pub fn launch(launcher: &Path, root: &Path, id: &str) -> std::io::Result<Self> {
            let executable = launcher
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            let cwd = root
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            let mut line = Vec::new();
            for value in [
                launcher.as_os_str(),
                std::ffi::OsStr::new("--dir"),
                root.as_os_str(),
                std::ffi::OsStr::new("--launch"),
                std::ffi::OsStr::new(id),
            ] {
                if !line.is_empty() {
                    line.push(b' ' as u16);
                }
                line.extend(quote(value));
            }
            line.push(0);
            // SAFETY: initialized Win32 structures, owned non-null handles and
            // nul-terminated UTF-16 buffers outlive each synchronous API call.
            unsafe {
                let raw_job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if raw_job.is_null() {
                    return Err(std::io::Error::last_os_error());
                }
                let job = OwnedHandle::from_raw_handle(raw_job);
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    raw_job,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32,
                ) == 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                let mut startup: STARTUPINFOW = std::mem::zeroed();
                startup.cb = std::mem::size_of_val(&startup) as u32;
                let mut info: PROCESS_INFORMATION = std::mem::zeroed();
                if CreateProcessW(
                    executable.as_ptr(),
                    line.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP,
                    std::ptr::null(),
                    cwd.as_ptr(),
                    &startup,
                    &mut info,
                ) == 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                let process = OwnedHandle::from_raw_handle(info.hProcess);
                if AssignProcessToJobObject(raw_job, info.hProcess) == 0 {
                    let error = std::io::Error::last_os_error();
                    TerminateProcess(info.hProcess, 1);
                    CloseHandle(info.hThread);
                    return Err(error);
                }
                // Assign before resuming: even immediately spawned Java children
                // are contained in this job, without a process-tree discovery race.
                let resumed = ResumeThread(info.hThread);
                CloseHandle(info.hThread);
                if resumed == u32::MAX {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(Self { job, process })
            }
        }
        pub fn stop(&mut self) -> bool {
            // Refuse directory deletion if termination cannot be confirmed.
            // SAFETY: both handles remain owned and valid through these calls.
            unsafe {
                let terminated = TerminateJobObject(self.job.as_raw_handle(), 1) != 0;
                terminated && WaitForSingleObject(self.process.as_raw_handle(), 5000) == 0
            }
        }
    }
    impl Drop for Process {
        fn drop(&mut self) {
            self.stop();
        }
    }
}
#[cfg(windows)]
pub use windows::Process;
