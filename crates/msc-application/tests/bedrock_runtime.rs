//! P10.9: the backend-neutral runtime boundary and frozen sidecar frames.

use msc_application::bedrock_runtime::*;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

#[derive(Default)]
struct FakeTransport {
    sent: Vec<String>,
    received: VecDeque<Option<String>>,
}

impl FakeTransport {
    fn response(&mut self, frame: SidecarFrame) {
        self.received.push_back(Some(encode_frame(&frame).unwrap()));
    }

    fn eof(&mut self) {
        self.received.push_back(None);
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

fn sidecar_fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bedrock-sidecar")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap()
}

fn frame_from_fixture(name: &str) -> SidecarFrame {
    let fixture = sidecar_fixture(name);
    serde_json::from_value(fixture["input"]["message"].clone()).unwrap()
}

#[test]
fn all_well_formed_sidecar_fixture_frames_encode_and_decode() {
    for entry in std::fs::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bedrock-sidecar"),
    )
    .unwrap()
    {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        if name.contains("malformed")
            || name.contains("out-of-order")
            || name.contains("eof-before-response")
            || name.contains("host-directory")
            || name.contains("shared-directory")
        {
            continue;
        }
        let frame = frame_from_fixture(name);
        let encoded = encode_frame(&frame).unwrap();
        assert_eq!(decode_frame(&encoded).unwrap(), frame, "fixture {name}");
    }
}

#[test]
fn malformed_frames_are_rejected_before_runtime_actions() {
    let malformed = [
        ("command-malformed-frame-rejected", "command-must-be-string"),
        ("ready-malformed-frame-rejected", "port-must-be-integer"),
    ];
    for (name, expected_reason) in malformed {
        let fixture = sidecar_fixture(name);
        let wire = fixture["input"].get("wire_line").and_then(Value::as_str);
        let result = wire.map(decode_frame).unwrap_or_else(|| {
            decode_frame(&serde_json::to_string(&fixture["input"]["message"]).unwrap())
        });
        let error = result.expect_err(name);
        assert!(
            error.to_string().contains(expected_reason)
                || matches!(error, BedrockRuntimeError::Protocol(_)),
            "{name}: {error}"
        );
    }
}

#[test]
fn sidecar_runtime_uses_shared_lifecycle_and_event_vocabulary() {
    let mut transport = FakeTransport::default();
    transport.response(SidecarFrame::Provisioned {
        ok: true,
        reason: None,
    });
    transport.response(SidecarFrame::Started {
        accepted: true,
        reason: None,
    });
    transport.response(frame_from_fixture("ready-round-trip"));
    transport.response(frame_from_fixture("console-line-round-trip"));
    transport.response(SidecarFrame::CommandResult {
        ok: true,
        reason: None,
    });
    transport.response(frame_from_fixture("terminated-clean-round-trip"));

    let mut runtime = SidecarRuntime::new(transport);
    assert_eq!(
        runtime.capabilities().backend,
        BedrockRuntimeBackend::Sidecar
    );
    assert!(
        runtime
            .capabilities()
            .capabilities
            .contains(&BedrockRuntimeCapability::Metrics)
    );

    runtime
        .provision(BedrockProvisionRequest {
            server_dir: "/srv/bedrock".to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();
    assert_eq!(runtime.state(), BedrockRuntimeState::Provisioned);
    assert_eq!(runtime.directory_mapping().unwrap().guest_mount, "/mnt");
    assert_eq!(runtime.directory_mapping().unwrap().tag, "world");

    runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .unwrap();
    assert_eq!(runtime.state(), BedrockRuntimeState::Starting);
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Ready {
            address: Some(_),
            port: 19132
        })
    ));
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::ConsoleLine(line)) if line == "Player connected: Alex"
    ));
    runtime.command("say hello").unwrap();
    assert!(matches!(
        runtime.poll_event().unwrap(),
        Some(BedrockRuntimeEvent::Terminated {
            reason: BedrockTerminationReason::Clean
        })
    ));
    assert_eq!(runtime.state(), BedrockRuntimeState::Stopped);
}

#[test]
fn sidecar_commands_are_json_lines_and_stop_is_terminally_observed() {
    let mut transport = FakeTransport::default();
    transport.response(SidecarFrame::Provisioned {
        ok: true,
        reason: None,
    });
    transport.response(SidecarFrame::Started {
        accepted: true,
        reason: None,
    });
    transport.response(frame_from_fixture("ready-round-trip"));
    transport.eof();

    let mut runtime = SidecarRuntime::new(transport);
    runtime
        .provision(BedrockProvisionRequest {
            server_dir: "/srv/bedrock".to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();
    runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .unwrap();
    runtime.poll_event().unwrap();
    runtime.stop().unwrap();
    assert_eq!(runtime.state(), BedrockRuntimeState::Stopping);
    let error = runtime.poll_event().expect_err("EOF must be terminal");
    assert_eq!(error, BedrockRuntimeError::SidecarEof);
    assert_eq!(runtime.state(), BedrockRuntimeState::Unavailable);

    let sent = &runtime.transport_mut().sent;
    let frames: Vec<SidecarFrame> = sent
        .iter()
        .map(|line| decode_frame(line).unwrap())
        .collect();
    assert_eq!(frames[0], frame_from_fixture("provision-round-trip"));
    assert_eq!(frames[1], frame_from_fixture("start-round-trip"));
    assert_eq!(frames[2], frame_from_fixture("stop-round-trip"));
}

#[test]
fn lifecycle_order_is_rejected_without_sending_a_frame() {
    let mut runtime = SidecarRuntime::new(FakeTransport::default());
    let error = runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .expect_err("start requires provisioning");
    assert!(matches!(error, BedrockRuntimeError::InvalidState { .. }));
    assert!(runtime.transport_mut().sent.is_empty());

    let mut transport = FakeTransport::default();
    transport.response(SidecarFrame::Terminated {
        reason: BedrockTerminationReason::Clean,
    });
    let mut runtime = SidecarRuntime::new(transport);
    let error = runtime
        .provision(BedrockProvisionRequest {
            server_dir: "/srv/bedrock".to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .expect_err("the queued terminated frame is not a provision response");
    assert!(matches!(error, BedrockRuntimeError::Protocol(_)));
    assert_eq!(runtime.state(), BedrockRuntimeState::New);
}
