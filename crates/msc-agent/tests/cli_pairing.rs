#[allow(dead_code)]
#[path = "../src/auth.rs"]
mod auth;

use std::sync::Arc;

use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use msc_infrastructure::xbox_broadcast::{alt_password_secret_key, auth_token_secret_key};

#[test]
fn host_local_pairing_can_issue_browser_and_desktop_recovery_codes() {
    let auth = auth::AuthState::new(Arc::new(FakeSecretStore::new()));
    let desktop = auth
        .create_host_local_pairing("desktop", "desktop-recovery".to_string())
        .expect("desktop recovery pairing");
    let browser = auth
        .create_host_local_pairing("browser", "browser-recovery".to_string())
        .expect("browser recovery pairing");

    assert!(desktop.pairing_code.starts_with("pair_"));
    assert!(browser.pairing_code.starts_with("pair_"));
    assert_eq!(desktop.agent_host_id, browser.agent_host_id);
    assert_ne!(desktop.pairing_code, browser.pairing_code);
}

#[test]
fn host_reset_revokes_old_credentials_and_rotates_host_identity() {
    let secrets = Arc::new(FakeSecretStore::new());
    let auth = auth::AuthState::new(secrets.clone());
    let old_host_id = auth.agent_host_id().unwrap();
    secrets
        .set(&alt_password_secret_key("paper"), "old-alt-password")
        .unwrap();
    secrets
        .set(&auth_token_secret_key("paper"), "old-auth-token")
        .unwrap();
    let issued = auth
        .issue_credential(
            "old-admin",
            auth::CredentialRole::Admin,
            auth::all_permissions(),
            None,
        )
        .unwrap();
    auth.reset_for_host_reset(&["paper".to_string()])
        .expect("host reset clears auth state");

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Bearer {}", issued.token)).unwrap(),
    );
    assert!(!auth.bearer_is_authenticated(&headers, "old-client"));
    assert_ne!(auth.agent_host_id().unwrap(), old_host_id);
    assert_eq!(
        secrets.get(&alt_password_secret_key("paper")).unwrap(),
        None
    );
    assert_eq!(secrets.get(&auth_token_secret_key("paper")).unwrap(), None);
}
