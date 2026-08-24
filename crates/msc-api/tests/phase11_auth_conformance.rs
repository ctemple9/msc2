//! P11.21 freezes the browser/desktop authentication boundary before either
//! implementation starts. These checks keep later auth work on the public
//! `/v1` contract instead of an unreviewed private protocol.

use serde_json::{Value, json};
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
fn phase11_auth_conformance_routes_are_additive_and_use_error_dto() {
    let contract = contract();
    let expected = [
        (
            "/v1/auth/pairings",
            "post",
            "createPairing",
            "bearer-admin",
            "admin",
        ),
        (
            "/v1/auth/browser-sessions",
            "post",
            "exchangeBrowserSession",
            "same-origin-pairing-code",
            "none",
        ),
        (
            "/v1/auth/csrf",
            "get",
            "getCsrfToken",
            "browser-session",
            "none",
        ),
        (
            "/v1/auth/browser-sessions/current",
            "delete",
            "logoutBrowserSession",
            "browser-session-csrf",
            "none",
        ),
        (
            "/v1/auth/desktop-pairings",
            "post",
            "exchangeDesktopPairing",
            "desktop-pairing-code",
            "none",
        ),
    ];

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
fn phase11_auth_conformance_dtos_keep_secrets_out_of_browser_results() {
    let contract = contract();

    assert_required_fields(
        &contract,
        "PairingCreateRequestDTO",
        &["clientKind", "label", "role", "permissions"],
    );
    assert_eq!(
        schema(&contract, "PairingCreateRequestDTO")["properties"]["clientKind"]["enum"],
        json!(["browser", "desktop"])
    );
    assert_required_fields(
        &contract,
        "PairingCreateResultDTO",
        &["pairingCode", "agentHostId", "clientKind", "expiresAt"],
    );
    assert_required_fields(
        &contract,
        "BrowserSessionExchangeRequestDTO",
        &["pairingCode"],
    );
    assert_required_fields(
        &contract,
        "CsrfTokenResponseDTO",
        &["csrfToken", "expiresAt"],
    );
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

    let browser_exchange = &contract["paths"]["/v1/auth/browser-sessions"]["post"];
    assert!(
        browser_exchange["responses"]["204"]["content"].is_null(),
        "a browser exchange must set an httpOnly cookie, not return a secret body"
    );
    assert_eq!(
        contract["paths"]["/v1/auth/desktop-pairings"]["post"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/DesktopCredentialResultDTO"
    );
}

#[test]
fn phase11_auth_conformance_requires_origin_and_csrf_without_weakening_bearer() {
    let contract = contract();
    let policy = &contract["x-authentication"];
    let default = policy["default"]
        .as_str()
        .expect("default authentication policy");
    let origin = policy["browserOrigin"].as_str().expect("origin policy");
    let mutation = policy["cookieMutation"].as_str().expect("CSRF policy");
    let cookie = policy["sessionCookie"].as_str().expect("cookie policy");
    let local = policy["localDesktopBootstrap"]
        .as_str()
        .expect("local desktop policy");

    assert!(default.contains("Bearer") && default.contains("takes precedence"));
    assert!(origin.contains("exactly match") && origin.contains("No permissive CORS"));
    assert!(
        mutation.contains("X-MSC-CSRF")
            && mutation.contains("Bearer-authenticated requests are exempt")
    );
    assert!(cookie.contains("httpOnly") && cookie.contains("SameSite=Strict"));
    assert!(local.contains("local IPC") && local.contains("never an HTTP loopback exception"));
}

#[test]
fn phase11_auth_conformance_design_records_the_unavailable_lan_shortcut() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/clients/phase11-auth.md");
    let design = std::fs::read_to_string(path).expect("read phase11-auth.md");
    for required in [
        "General-LAN management",
        "Tailscale encrypts the network path but is not identity",
        "never an unauthenticated\n+loopback HTTP exception",
        "automatic bootstrap is unavailable",
        "not a retroactive owner approval",
    ] {
        assert!(
            design.contains(required),
            "missing design boundary: {required}"
        );
    }
}
