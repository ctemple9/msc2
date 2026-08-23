//! P10.14: native Windows Bedrock process and lifecycle behavior.

use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeEvent,
    BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason,
};
use msc_application::bedrock_windows::{
    BedrockRuntimeClock, GRACEFUL_STOP_TIMEOUT, WindowsBedrockRuntime,
};
use msc_infrastructure::bedrock_native::NativeBedrockHost;
use msc_infrastructure::process::FakeProcessSupervisor;
use std::cell::Cell;
use std::net::UdpSocket;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct FakeClock {
    now: Rc<Cell<Instant>>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            now: Rc::new(Cell::new(Instant::now())),
        }
    }

    fn advance(&self, duration: Duration) {
        self.now.set(self.now.get() + duration);
    }
}

impl BedrockRuntimeClock for FakeClock {
    fn now(&self) -> Instant {
        self.now.get()
    }
}

fn free_udp_port() -> u16 {
    UdpSocket::bind("0.0.0.0:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn runtime<'a>(
    process: &'a FakeProcessSupervisor,
    clock: FakeClock,
) -> WindowsBedrockRuntime<'a, FakeClock> {
    WindowsBedrockRuntime::with_host(process, NativeBedrockHost::Windows, clock)
}

fn provision_and_start(runtime: &mut WindowsBedrockRuntime<'_, FakeClock>, port: u16) -> u32 {
    runtime
        .provision(BedrockProvisionRequest {
            server_dir: r"C:\MSC\servers\bedrock".to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();
    runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: port,
        })
        .map(|_| runtime.process_id().unwrap().raw())
        .unwrap()
}

#[test]
fn native_windows_runtime_uses_exe_and_direct_udp_port() {
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    let port = free_udp_port();

    let pid = provision_and_start(&mut runtime, port);

    assert_eq!(runtime.state(), BedrockRuntimeState::Starting);
    let requests = process.spawned_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0.raw(), pid);
    assert_eq!(
        requests[0]
            .1
            .executable_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some("bedrock_server.exe")
    );
    assert_eq!(
        requests[0].1.working_directory.to_str(),
        Some(r"C:\MSC\servers\bedrock")
    );
}

#[test]
fn native_windows_runtime_frames_output_and_matches_readiness_substring() {
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    let pid = provision_and_start(&mut runtime, free_udp_port());
    let pid = msc_infrastructure::process::ProcessId::new(pid);

    process
        .emit_stdout(pid, b"starting\r\n[INFO] Server started")
        .unwrap();
    process.emit_stdout(pid, b" successfully\r\n").unwrap();
    process.emit_stderr(pid, b"final partial").unwrap();

    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::ConsoleLine(line)) if line == "starting"
    ));
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::ConsoleLine(line)) if line == "[INFO] Server started successfully"
    ));
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Ready { port, .. }) if port != 0
    ));

    process.exit_normally(pid).unwrap();
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::ConsoleLine(line)) if line == "final partial"
    ));
}

#[test]
fn native_windows_runtime_sends_stop_and_forces_after_twenty_seconds() {
    let process = FakeProcessSupervisor::new();
    let clock = FakeClock::new();
    let clock_control = clock.clone();
    let mut runtime = runtime(&process, clock);
    let pid = provision_and_start(&mut runtime, free_udp_port());
    let pid = msc_infrastructure::process::ProcessId::new(pid);
    process.emit_stdout(pid, b"Server started\n").unwrap();
    runtime.poll_event().unwrap();
    runtime.poll_event().unwrap();

    runtime.stop().unwrap();
    assert_eq!(process.stdin_writes(pid).unwrap(), vec![b"stop\n".to_vec()]);
    clock_control.advance(GRACEFUL_STOP_TIMEOUT);
    let event = runtime.poll_event().unwrap();

    assert!(matches!(
        event,
        Some(BedrockRuntimeEvent::Terminated {
            reason: BedrockTerminationReason::Clean
        })
    ));
    assert_eq!(process.force_terminations(), vec![pid]);
}

#[test]
fn native_windows_runtime_distinguishes_unrequested_crash() {
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    let pid = provision_and_start(&mut runtime, free_udp_port());
    let pid = msc_infrastructure::process::ProcessId::new(pid);
    process.emit_stdout(pid, b"Server started\n").unwrap();
    runtime.poll_event().unwrap();
    runtime.poll_event().unwrap();
    process.crash(pid, 139).unwrap();

    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Terminated {
            reason: BedrockTerminationReason::GuestError(_)
        })
    ));
}

#[test]
fn native_windows_runtime_rejects_port_in_use_before_spawning() {
    let listener = UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    runtime
        .provision(BedrockProvisionRequest {
            server_dir: r"C:\MSC\servers\bedrock".to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();

    let error = runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: port,
        })
        .expect_err("a bound UDP port must prevent process start");

    assert!(error.to_string().contains("udp-port-in-use"));
    assert!(process.spawned_requests().is_empty());
}

#[test]
fn native_windows_runtime_discloses_unsupported_hosts_without_starting() {
    let process = FakeProcessSupervisor::new();
    let mut runtime =
        WindowsBedrockRuntime::with_host(&process, NativeBedrockHost::Macos, FakeClock::new());

    assert_eq!(
        runtime.capabilities().backend,
        BedrockRuntimeBackend::Native
    );
    assert!(!runtime.capabilities().supported);
    assert_eq!(runtime.state(), BedrockRuntimeState::Unavailable);
    assert!(
        runtime
            .start(BedrockStartRequest {
                memory_gb: 2,
                bedrock_port: free_udp_port(),
            })
            .is_err()
    );
    assert!(process.spawned_requests().is_empty());
}
