//! The desktop pairing boundary remains part of the public `/v1` contract.

use serde_json::Value;
use std::path::Path;

fn contract() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/api-contract/openapi.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read openapi.json"))
        .expect("openapi.json is valid JSON")
}

fn schema<'a>(contract: &'a Value, name: &str) -> &'a Value {
    &contract["components"]["schemas"][name]
}

fn assert_required_fields(contract: &Value, name: &str, expected: &[&str]) {
    let required = schema(contract, name)["required"]
        .as_array()
        .expect("required fields")
        .iter()
        .map(|field| field.as_str().expect("field name"))
        .collect::<Vec<_>>();
    assert_eq!(required, expected, "{name} required fields changed");
}

#[test]
fn phase11_auth_conformance_keeps_desktop_pairing_on_the_public_contract() {
    let contract = contract();
    let expected = [(
        "/v1/auth/desktop-pairings",
        "post",
        "exchangeDesktopPairing",
        "desktop-pairing-code",
        "none",
    )];

    for (path, method, operation_id, authentication, permission) in expected {
        let operation = &contract["paths"][path][method];
        assert_eq!(operation["operationId"], operation_id, "{method} {path}");
        assert_eq!(
            operation["x-authentication"], authentication,
            "{method} {path}"
        );
        assert_eq!(
            operation["x-permission-category"], permission,
            "{method} {path}"
        );
        for (status, response) in operation["responses"].as_object().expect("responses") {
            if !status.starts_with('2') {
                assert_eq!(
                    response["content"]["application/json"]["schema"]["$ref"],
                    "#/components/schemas/ErrorDTO",
                    "{method} {path} {status}"
                );
            }
        }
    }
}

#[test]
fn phase11_auth_conformance_desktop_pairing_keeps_secrets_in_the_native_boundary() {
    let contract = contract();

    assert_required_fields(
        &contract,
        "DesktopPairingExchangeRequestDTO",
        &["pairingCode"],
    );
    assert_required_fields(
        &contract,
        "DesktopCredentialResultDTO",
        &["agentHostId", "credentialId", "token"],
    );

    assert_eq!(
        contract["paths"]["/v1/auth/desktop-pairings"]["post"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/DesktopCredentialResultDTO"
    );
}

#[test]
fn phase11_auth_conformance_design_records_the_unavailable_lan_shortcut() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/clients/phase11-auth.md");
    let design = std::fs::read_to_string(path)
        .expect("read phase11-auth.md")
        .replace("\r\n", "\n");
    for required in [
        "General-LAN management",
        "Tailscale encrypts the network path but is not identity",
        "not an unauthenticated\nloopback HTTP exception",
        "automatic bootstrap is unavailable",
        "ordinary remote-pairing code flow",
    ] {
        assert!(
            design.contains(required),
            "missing design boundary: {required}"
        );
    }
}
