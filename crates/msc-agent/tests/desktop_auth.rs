//! P11.23's desktop credential boundary. The tests use the same durable
//! registry and secret-store shape as the agent, while keeping raw tokens out
//! of test fixtures and every webview-facing result.

#[allow(dead_code)]
#[path = "../src/auth.rs"]
mod auth;

use auth::desktop::CreateDesktopPairing;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::http::{HeaderMap, HeaderValue, header};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::secret_store::FakeSecretStore;

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    headers
}

#[test]
fn desktop_pairing_is_one_use_host_scoped_and_revocable() {
    let secret_store = Arc::new(FakeSecretStore::new());
    let fs: &'static dyn FileSystem = Box::leak(Box::new(FakeFileSystem::new().with_dir("/agent")));
    let auth = auth::AuthState::with_persistent_registry(
        secret_store,
        fs,
        "/agent/credential-registry.json",
    )
    .expect("durable credential registry");

    let pairing = auth
        .create_desktop_pairing(CreateDesktopPairing {
            label: "desktop-admin".into(),
            role: auth::CredentialRole::Admin,
            permissions: auth::all_permissions(),
            expires_at: None,
        })
        .expect("desktop pairing verifier is stored");
    let credential = auth
        .exchange_desktop_pairing(&pairing.pairing_code)
        .expect("the Tauri backend receives one bearer credential");

    assert_eq!(credential.agent_host_id, pairing.agent_host_id);
    assert!(credential.agent_host_id.starts_with("agent_"));
    assert!(auth.bearer_is_authenticated(&bearer_headers(&credential.issued.token), "desktop"));
    assert!(matches!(
        auth.exchange_desktop_pairing(&pairing.pairing_code),
        Err(auth::DesktopPairingError::Consumed)
    ));

    auth.revoke_credential(&credential.issued.credential_id, "owner-admin")
        .expect("revocation removes the token verifier");
    assert!(
        !auth.bearer_is_authenticated(&bearer_headers(&credential.issued.token), "desktop"),
        "the Tauri store cannot make a revoked credential usable"
    );
}

#[test]
fn desktop_pairing_preserves_credential_expiry_and_host_identity_across_restart() {
    let secret_store = Arc::new(FakeSecretStore::new());
    let fs: &'static dyn FileSystem = Box::leak(Box::new(FakeFileSystem::new().with_dir("/agent")));
    let auth = auth::AuthState::with_persistent_registry(
        secret_store.clone(),
        fs,
        "/agent/credential-registry.json",
    )
    .expect("durable credential registry");
    let host_id = auth.agent_host_id().expect("durable host identity");
    let pairing = auth
        .create_desktop_pairing(CreateDesktopPairing {
            label: "expired desktop".into(),
            role: auth::CredentialRole::Named,
            permissions: vec![],
            expires_at: Some(SystemTime::now() - Duration::from_secs(1)),
        })
        .expect("desktop pairing");
    let credential = auth
        .exchange_desktop_pairing(&pairing.pairing_code)
        .expect("a credential can be issued with its original expiry");
    assert_eq!(credential.agent_host_id, host_id);
    assert!(
        !auth.bearer_is_authenticated(&bearer_headers(&credential.issued.token), "desktop"),
        "the registry rejects expired desktop credentials"
    );

    let restarted = auth::AuthState::with_persistent_registry(
        secret_store,
        fs,
        "/agent/credential-registry.json",
    )
    .expect("restart reloads the credential registry");
    assert_eq!(
        restarted
            .agent_host_id()
            .expect("host identity survives restart"),
        host_id
    );
}
