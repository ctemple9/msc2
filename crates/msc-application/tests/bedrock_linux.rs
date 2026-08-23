//! P10.11: native Linux Bedrock process and lifecycle behavior.

use msc_application::bedrock_linux::{
    BedrockRuntimeClock, GRACEFUL_STOP_TIMEOUT, LinuxBedrockRuntime,
};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeEvent,
    BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason,
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
) -> LinuxBedrockRuntime<'a, FakeClock> {
    LinuxBedrockRuntime::with_host(process, NativeBedrockHost::Linux, clock)
}

fn provision_and_start(runtime: &mut LinuxBedrockRuntime<'_, FakeClock>, port: u16) -> u32 {
    runtime
        .provision(BedrockProvisionRequest {
            server_dir: "/srv/bedrock".to_owned(),
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
fn native_linux_runtime_uses_bedrock_server_and_direct_udp_port() {
    let process = FakeProcessSupervisor::new();
    let clock = FakeClock::new();
    let mut runtime = runtime(&process, clock);
    let port = free_udp_port();

    let pid = provision_and_start(&mut runtime, port);

    assert_eq!(runtime.state(), BedrockRuntimeState::Starting);
    let requests = process.spawned_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0.raw(), pid);
    assert_eq!(
        requests[0].1.executable_path.to_str(),
        Some("/srv/bedrock/bedrock_server")
    );
    assert_eq!(
        requests[0].1.working_directory.to_str(),
        Some("/srv/bedrock")
    );
}

#[test]
fn native_linux_runtime_frames_output_and_matches_readiness_substring() {
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    let pid = provision_and_start(&mut runtime, free_udp_port());
    let pid = msc_infrastructure::process::ProcessId::new(pid);

    process
        .emit_stdout(pid, b"starting\n[INFO] Server started")
        .unwrap();
    process.emit_stdout(pid, b" successfully\n").unwrap();
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
fn native_linux_runtime_strips_one_leading_slash_from_commands() {
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    let pid = provision_and_start(&mut runtime, free_udp_port());
    let pid = msc_infrastructure::process::ProcessId::new(pid);
    process.emit_stdout(pid, b"Server started\n").unwrap();
    runtime.poll_event().unwrap();
    runtime.poll_event().unwrap();

    runtime.command("/say hello").unwrap();

    assert_eq!(
        process.stdin_writes(pid).unwrap(),
        vec![b"say hello\n".to_vec()]
    );
}

#[test]
fn native_linux_runtime_forces_after_twenty_seconds_and_reports_clean_stop() {
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
fn native_linux_runtime_distinguishes_unrequested_crash() {
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
fn native_linux_runtime_rejects_port_in_use_before_spawning() {
    let listener = UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let process = FakeProcessSupervisor::new();
    let mut runtime = runtime(&process, FakeClock::new());
    runtime
        .provision(BedrockProvisionRequest {
            server_dir: "/srv/bedrock".to_owned(),
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
fn native_linux_runtime_discloses_unsupported_hosts_without_starting() {
    let process = FakeProcessSupervisor::new();
    let mut runtime =
        LinuxBedrockRuntime::with_host(&process, NativeBedrockHost::Macos, FakeClock::new());

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
