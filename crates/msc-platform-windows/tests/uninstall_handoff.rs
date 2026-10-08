// A separate executable lets the fixed worker arguments select the controlled
// helper role. No production uninstall command or MSC data path is used.
#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    windows::run();
}

#[cfg(windows)]
mod windows {
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, EVENT_MODIFY_STATE, OpenEventW, OpenProcess, SetEvent, WaitForSingleObject,
    };

    const SYNCHRONIZE: u32 = 0x0010_0000;
    struct Event(HANDLE);
    impl Event {
        fn create(name: &str) -> Self {
            let name: Vec<u16> = std::ffi::OsStr::new(name)
                .encode_wide()
                .chain([0])
                .collect();
            // SAFETY: The name is terminated, security is default, and the
            // returned handle is owned by this RAII wrapper.
            let handle = unsafe { CreateEventW(std::ptr::null(), 1, 0, name.as_ptr()) };
            assert!(!handle.is_null(), "could not create coordination event");
            Self(handle)
        }
        fn open(name: &str) -> Self {
            let name: Vec<u16> = std::ffi::OsStr::new(name)
                .encode_wide()
                .chain([0])
                .collect();
            // SAFETY: The live UTF-16 name is terminated. No handle inheritance.
            let handle = unsafe { OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, name.as_ptr()) };
            assert!(!handle.is_null(), "could not open coordination event");
            Self(handle)
        }
        fn signal(&self) {
            // SAFETY: This wrapper owns a live event handle.
            assert_ne!(unsafe { SetEvent(self.0) }, 0);
        }
        fn wait(&self) -> bool {
            // SAFETY: This wrapper owns a live waitable event handle. The bound
            // prevents a broken helper from hanging the owner-requested check.
            unsafe { WaitForSingleObject(self.0, 10_000) == WAIT_OBJECT_0 }
        }
    }
    impl Drop for Event {
        fn drop(&mut self) {
            // SAFETY: This is the single owning wrapper for the handle.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub fn run() {
        let arguments: Vec<_> = std::env::args_os().collect();
        if arguments.get(1).is_some_and(|value| value == "uninstall") {
            let job = PathBuf::from(arguments.last().unwrap());
            let names = fs::read_to_string(&job).unwrap();
            let (started, release) = names.split_once('\n').unwrap();
            let started = Event::open(started);
            let release = Event::open(release);
            fs::write(
                job.with_file_name("worker.pid"),
                std::process::id().to_string(),
            )
            .unwrap();
            started.signal();
            assert!(release.wait(), "fixture worker was not released");
            return;
        }
        if arguments.get(1).is_some_and(|value| value == "--producer") {
            let root = Path::new(arguments.get(2).unwrap());
            if std::env::var_os("MSC2_HANDOFF_LEGACY").is_some() {
                use std::os::windows::process::CommandExt;
                // Optional negative control reproduces the old launch while
                // running only this fixture, never a real uninstall command.
                // The coordinator waits for this process through OpenProcess;
                // waiting here would add a different scheduling dependency.
                #[allow(clippy::zombie_processes)]
                let _worker = Command::new(root.join("msc-uninstall-worker.exe"))
                    .args([
                        "uninstall",
                        "--danger",
                        "--confirm",
                        "UNINSTALL MSC 2",
                        "--apply",
                    ])
                    .arg(root.join("job.json"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(0x08000000)
                    .spawn()
                    .unwrap();
            } else {
                msc_platform_windows::uninstall::launch_uninstall_worker(
                    &root.join("msc-uninstall-worker.exe"),
                    &root.join("job.json"),
                )
                .unwrap();
            }
            println!("scheduled");
            return;
        }

        let unique = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(format!("msc2-handoff-{unique}"));
        fs::create_dir(&root).unwrap();
        let started_name = format!("Local\\MscHandoffStarted-{unique}");
        let release_name = format!("Local\\MscHandoffRelease-{unique}");
        let started = Event::create(&started_name);
        let release = Event::create(&release_name);
        let executable = std::env::current_exe().unwrap();
        fs::copy(&executable, root.join("msc-uninstall-worker.exe")).unwrap();
        fs::write(
            root.join("job.json"),
            format!("{started_name}\n{release_name}"),
        )
        .unwrap();
        let mut producer = Command::new(&executable);
        producer.arg("--producer").arg(&root).stdin(Stdio::null());
        let (send, receive) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let _ = send.send(producer.output());
        });
        let worker_started = started.wait();
        // EOF must arrive while the worker is deliberately still waiting. This
        // reproduces the scheduling/desktop dependency without deleting data.
        let scheduling = receive.recv_timeout(Duration::from_secs(5));
        release.signal();
        if worker_started {
            let pid: u32 = fs::read_to_string(root.join("worker.pid"))
                .unwrap()
                .parse()
                .unwrap();
            // SAFETY: OpenProcess returns our own wait-only handle for this
            // fixture PID; it is closed once the worker has exited.
            let process = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
            if !process.is_null() {
                unsafe {
                    WaitForSingleObject(process, 10_000);
                    CloseHandle(process);
                }
            }
        }
        reader.join().unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(worker_started, "fixture worker did not start");
        if std::env::var_os("MSC2_HANDOFF_LEGACY").is_some() {
            assert!(
                scheduling.is_err(),
                "negative control did not reproduce the inherited-pipe wait"
            );
            println!("Negative control reproduced the old Windows inherited-pipe deadlock.");
            return;
        }
        let output = scheduling
            .expect("scheduler output was held open by its waiting worker")
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "scheduled");
        println!("Windows uninstall handoff passed: scheduling completes while worker waits.");
    }
}
