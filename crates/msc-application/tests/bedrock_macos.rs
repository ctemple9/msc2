//! P10.16: macOS sidecar process transport and runtime adapter.

use msc_application::bedrock_macos::{
    MacosBedrockHost, MacosBedrockRuntime, SidecarProcessTransport,
};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeError,
    BedrockRuntimeEvent, BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason,
    SidecarFrame, SidecarRuntime, SidecarTransport, decode_frame, encode_frame,
};
use msc_infrastructure::bedrock_sidecar::BedrockSidecarProcess;
use msc_infrastructure::process::FakeProcessSupervisor;
use std::collections::VecDeque;

#[derive(Default)]
struct FakeTransport {
    sent: Vec<String>,
    received: VecDeque<Option<String>>,
}

impl FakeTransport {
    fn response(&mut self, frame: SidecarFrame) {
        self.received.push_back(Some(encode_frame(&frame).unwrap()));
    }
}

impl SidecarTransport for FakeTransport {
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.sent.push(line.to_owned());
        Ok(())
    }

    fn receive_line(&mut self) -> Result<Option<String>, String> {
        Ok(self.received.pop_front().flatten())
    }
}

fn provision_request() -> BedrockProvisionRequest {
    BedrockProvisionRequest {
        server_dir: "/srv/bedrock".to_owned(),
        version: "1.26.32.2".to_owned(),
    }
}

fn start_request() -> BedrockStartRequest {
    BedrockStartRequest {
        memory_gb: 2,
        bedrock_port: 19132,
    }
}

#[test]
fn intel_mac_adapter_uses_shared_runtime_vocabulary() {
    let mut transport = FakeTransport::default();
    transport.response(SidecarFrame::Provisioned {
        ok: true,
        reason: None,
    });
    transport.response(SidecarFrame::Started {
        accepted: true,
        reason: None,
    });
    transport.response(SidecarFrame::Ready {
        guest_ip: "192.168.64.7".to_owned(),
        port: 19132,
        relay_up: true,
    });
    transport.response(SidecarFrame::Terminated {
        reason: BedrockTerminationReason::Clean,
    });

    let mut runtime = MacosBedrockRuntime::with_transport(transport, MacosBedrockHost::Intel);
    assert_eq!(
        runtime.capabilities().backend,
        BedrockRuntimeBackend::Sidecar
    );
    runtime.provision(provision_request()).unwrap();
    runtime.start(start_request()).unwrap();
    assert_eq!(runtime.state(), BedrockRuntimeState::Starting);
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Ready {
            address: Some(address),
            port: 19132
        }) if address == "192.168.64.7"
    ));
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Terminated {
            reason: BedrockTerminationReason::Clean
        })
    ));
    assert_eq!(runtime.state(), BedrockRuntimeState::Stopped);
}

#[test]
fn apple_silicon_is_explicitly_unavailable_without_starting_a_sidecar() {
    let mut runtime = MacosBedrockRuntime::with_transport(
        FakeTransport::default(),
        MacosBedrockHost::AppleSilicon,
    );
    assert_eq!(runtime.state(), BedrockRuntimeState::Unavailable);
    assert!(!runtime.capabilities().supported);
    assert_eq!(
        runtime.capabilities().unavailable_reason.as_deref(),
        Some("apple-silicon-unavailable-no-test-hardware")
    );
    let error = runtime.provision(provision_request()).unwrap_err();
    assert!(matches!(error, BedrockRuntimeError::Transport(_)));
}

#[test]
fn process_transport_frames_stdout_and_reports_pending_before_eof() {
    let supervisor = FakeProcessSupervisor::new();
    let mut process = BedrockSidecarProcess::spawn(&supervisor, "/sidecar", "/srv/bedrock")
        .expect("fake sidecar spawn");
    let pid = process.process_id();

    assert_eq!(
        process.receive().unwrap(),
        msc_infrastructure::bedrock_sidecar::SidecarReceive::Pending
    );
    supervisor
        .emit_stderr(pid, b"diagnostic\n".to_vec())
        .unwrap();
    supervisor
        .emit_stdout(pid, b"{\"type\":\"ready\"".to_vec())
        .unwrap();
    assert_eq!(
        process.receive().unwrap(),
        msc_infrastructure::bedrock_sidecar::SidecarReceive::Pending
    );
    supervisor.emit_stdout(pid, b"}\n".to_vec()).unwrap();
    assert_eq!(
        process.receive().unwrap(),
        msc_infrastructure::bedrock_sidecar::SidecarReceive::Line(
            "{\"type\":\"ready\"}".to_owned()
        )
    );
    supervisor.exit_normally(pid).unwrap();
    assert_eq!(
        process.receive().unwrap(),
        msc_infrastructure::bedrock_sidecar::SidecarReceive::Eof
    );
}

#[test]
fn spawned_process_transport_writes_json_lines_to_sidecar_stdin() {
    let supervisor = FakeProcessSupervisor::new();
    let process = BedrockSidecarProcess::spawn(&supervisor, "/sidecar", "/srv/bedrock")
        .expect("fake sidecar spawn");
    let pid = process.process_id();
    let mut runtime = MacosBedrockRuntime::with_transport(
        SidecarProcessTransport::from_process(process),
        MacosBedrockHost::Intel,
    );
    supervisor
        .emit_stdout(
            pid,
            encode_frame(&SidecarFrame::Provisioned {
                ok: true,
                reason: None,
            })
            .unwrap(),
        )
        .unwrap();

    runtime.provision(provision_request()).unwrap();
    let writes = supervisor.stdin_writes(pid).unwrap();
    assert_eq!(writes.len(), 1);
    assert_eq!(
        decode_frame(std::str::from_utf8(&writes[0]).unwrap()).unwrap(),
        SidecarFrame::Provision {
            server_dir: "/srv/bedrock".to_owned(),
            version: "1.26.32.2".to_owned()
        }
    );
}

#[test]
fn repeated_ready_and_termination_frames_are_rejected_by_state_order() {
    let mut transport = FakeTransport::default();
    transport.response(SidecarFrame::Provisioned {
        ok: true,
        reason: None,
    });
    transport.response(SidecarFrame::Started {
        accepted: true,
        reason: None,
    });
    transport.response(SidecarFrame::Ready {
        guest_ip: "192.168.64.7".to_owned(),
        port: 19132,
        relay_up: true,
    });
    transport.response(SidecarFrame::Ready {
        guest_ip: "192.168.64.7".to_owned(),
        port: 19132,
        relay_up: true,
    });

    let mut runtime = SidecarRuntime::new(transport);
    runtime.provision(provision_request()).unwrap();
    runtime.start(start_request()).unwrap();
    runtime.poll_event().unwrap().unwrap();
    let error = runtime.poll_event().unwrap_err();
    assert!(matches!(error, BedrockRuntimeError::InvalidState { .. }));
    assert_eq!(runtime.state(), BedrockRuntimeState::Running);
}
