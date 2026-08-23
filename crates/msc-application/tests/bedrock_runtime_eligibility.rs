//! P10.30: eligibility is based on host prerequisites, not a platform label.

use msc_application::bedrock_runtime::{
    BedrockHost, BedrockRuntimeBackend, BedrockRuntimeEligibility, BedrockRuntimeEligibilityState,
    BedrockRuntimePaths, BedrockSidecarResources,
};
use msc_infrastructure::bedrock_distribution::{
    BEDROCK_PROVENANCE_MARKER, BedrockDistributionProvenance, BedrockPlatform,
};
use msc_infrastructure::fs::FakeFileSystem;
use serde_json::Value;
use std::path::{Path, PathBuf};

const SERVER: &str = "servers/bedrock";

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bedrock-runtime")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn paths(sidecar: Option<BedrockSidecarResources>) -> BedrockRuntimePaths {
    BedrockRuntimePaths {
        server_dir: PathBuf::from(SERVER),
        sidecar,
    }
}

fn verified_fs(platform: BedrockPlatform) -> FakeFileSystem {
    let executable = PathBuf::from(SERVER).join(platform.executable_name());
    let provenance = serde_json::to_vec(&BedrockDistributionProvenance {
        version: "1.26.32.2".to_owned(),
        platform,
        sha256: "a".repeat(64),
    })
    .unwrap();
    FakeFileSystem::new()
        .with_file(executable, b"bds".to_vec(), true)
        .with_file(
            PathBuf::from(SERVER).join(BEDROCK_PROVENANCE_MARKER),
            provenance,
            false,
        )
}

fn assert_fixture(name: &str, eligibility: BedrockRuntimeEligibility) {
    let expected = fixture(name)["expected"].clone();
    assert_eq!(
        serde_json::to_value(eligibility.state).unwrap(),
        expected["state"]
    );
    assert_eq!(
        serde_json::to_value(eligibility.backend).unwrap(),
        expected["backend"]
    );
    assert_eq!(
        eligibility.reason_code.as_deref(),
        expected["reasonCode"].as_str()
    );
}

#[test]
fn native_runtime_requires_a_verified_distribution_on_the_current_host() {
    let missing = BedrockRuntimeEligibility::for_host(
        &FakeFileSystem::new(),
        BedrockHost::Linux,
        &paths(None),
    );
    assert_fixture("eligibility-linux-missing-distribution", missing);

    let unverified = FakeFileSystem::new().with_file(
        PathBuf::from(SERVER).join("bedrock_server"),
        b"hand copied".to_vec(),
        true,
    );
    assert_fixture(
        "eligibility-linux-unverified-distribution",
        BedrockRuntimeEligibility::for_host(&unverified, BedrockHost::Linux, &paths(None)),
    );

    let verified = verified_fs(BedrockPlatform::Linux);
    let result = BedrockRuntimeEligibility::for_host(&verified, BedrockHost::Linux, &paths(None));
    assert_fixture("eligibility-linux-verified-distribution", result.clone());
    assert_eq!(result.state, BedrockRuntimeEligibilityState::Available);
    assert_eq!(result.backend, Some(BedrockRuntimeBackend::Native));
}

#[test]
fn windows_uses_the_windows_distribution_and_executable() {
    let fs = verified_fs(BedrockPlatform::Windows);
    let result = BedrockRuntimeEligibility::for_host(&fs, BedrockHost::Windows, &paths(None));
    assert_fixture("eligibility-windows-verified-distribution", result);
}

#[test]
fn intel_sidecar_requires_both_distribution_and_distributable_resources() {
    let resource_paths = BedrockSidecarResources {
        executable: PathBuf::from("sidecar/BedrockSidecar"),
        kernel: PathBuf::from("sidecar/vmlinuz-kata"),
        initramfs: PathBuf::from("sidecar/appliance-initramfs.gz"),
    };
    let fs = verified_fs(BedrockPlatform::Macos).with_file(
        resource_paths.executable.clone(),
        b"sidecar".to_vec(),
        true,
    );
    let missing_appliance = BedrockRuntimeEligibility::for_host(
        &fs,
        BedrockHost::MacosIntel,
        &paths(Some(resource_paths.clone())),
    );
    assert_fixture("eligibility-intel-missing-appliance", missing_appliance);

    let fs = fs
        .with_file(resource_paths.kernel.clone(), b"kernel".to_vec(), false)
        .with_file(
            resource_paths.initramfs.clone(),
            b"initramfs".to_vec(),
            false,
        );
    let ready = BedrockRuntimeEligibility::for_host(
        &fs,
        BedrockHost::MacosIntel,
        &paths(Some(resource_paths)),
    );
    assert_fixture("eligibility-intel-sidecar-ready", ready);
}

#[test]
fn apple_silicon_is_unavailable_even_when_fixture_files_exist() {
    let fs = verified_fs(BedrockPlatform::Macos);
    let result =
        BedrockRuntimeEligibility::for_host(&fs, BedrockHost::MacosAppleSilicon, &paths(None));
    assert_fixture("eligibility-apple-silicon-no-test-hardware", result);
}
