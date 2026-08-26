//! P11.22's public browser-auth boundary, exercised without a production
//! keychain so the test stays portable and cannot change a developer's tokens.

#[allow(dead_code)]
#[path = "../src/auth.rs"]
mod auth;

use std::sync::Arc;

use axum::http::{HeaderMap, HeaderValue, header};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::secret_store::FakeSecretStore;

fn browser_headers(cookie: &str, origin: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::HOST, HeaderValue::from_static("agent.test"));
    headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("msc2_session={cookie}")).unwrap(),
    );
    headers
}

#[test]
fn browser_auth_one_use_cookie_csrf_and_revocation_boundary() {
    let secret_store = Arc::new(FakeSecretStore::new());
    let fs: &'static dyn FileSystem = Box::leak(Box::new(FakeFileSystem::new().with_dir("/agent")));
    let auth = auth::AuthState::with_persistent_registry(
        secret_store.clone(),
        fs,
        "/agent/credential-registry.json",
    )
    .expect("durable credential registry");
    let pairing = auth
        .create_browser_pairing(auth::CreateBrowserPairing {
            label: "browser-admin".into(),
            role: auth::CredentialRole::Admin,
            permissions: auth::all_permissions(),
            expires_at: None,
        })
        .expect("pairing is stored as a verifier");
    let session = auth
        .exchange_browser_pairing(&pairing.pairing_code)
        .expect("one-use code creates an httpOnly-cookie session");
    assert!(matches!(
        auth.exchange_browser_pairing(&pairing.pairing_code),
        Err(auth::BrowserSessionError::Consumed)
    ));

    let cookie = auth::session_cookie(&session, false);
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(cookie.contains("Path=/v1"));
    let value = cookie
        .split(';')
        .next()
        .and_then(|part| part.split_once('='))
        .map(|(_, value)| value)
        .expect("cookie value");
    let headers = browser_headers(value, "http://agent.test");
    let authenticated = auth
        .authenticate_browser_session(&headers)
        .expect("cookie verifier authenticates the browser session");
    let restarted = auth::AuthState::with_persistent_registry(
        secret_store,
        fs,
        "/agent/credential-registry.json",
    )
    .expect("restart reloads the durable registry");
    assert!(restarted.authenticate_browser_session(&headers).is_ok());
    let credential = auth
        .credential_for_browser_session(&authenticated.credential_id)
        .expect("session inherits the paired credential permissions");
    assert_eq!(credential.label, "browser-admin");

    assert!(!auth::browser_mutation_is_authorized(
        &headers,
        &authenticated.csrf_token
    ));
    let mut csrf_headers = headers.clone();
    csrf_headers.insert(
        "X-MSC-CSRF",
        HeaderValue::from_str(&authenticated.csrf_token).unwrap(),
    );
    assert!(auth::browser_mutation_is_authorized(
        &csrf_headers,
        &authenticated.csrf_token
    ));
    let hostile = browser_headers(value, "http://hostile.test");
    assert!(!auth::browser_mutation_is_authorized(
        &hostile,
        &authenticated.csrf_token
    ));

    auth.revoke_credential(&credential.credential_id, "owner-admin")
        .expect("credential revocation removes its verifier");
    assert!(
        auth.credential_for_browser_session(&authenticated.credential_id)
            .is_err()
    );
}

#[test]
fn browser_auth_bearer_uses_no_csrf_material() {
    let auth = auth::AuthState::new(Arc::new(FakeSecretStore::new()));
    let issued = auth
        .issue_credential(
            "cli-admin",
            auth::CredentialRole::Admin,
            auth::all_permissions(),
            None,
        )
        .expect("bearer credential");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", issued.token)).unwrap(),
    );

    assert!(
        auth.bearer_is_authenticated(&headers, "cli-client"),
        "bearer clients remain independent of browser CSRF"
    );
}
