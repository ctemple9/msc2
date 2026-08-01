//! Dev-mode bearer-token verification — P2.3's scoped-down auth for Phase
//! 2's iOS-only, loopback-only gate. This checks a single fixed token read
//! from an environment variable at request time, not the real
//! pairing-link/`SecretStore` flow (Phase 3, per
//! `docs/msc2/api-contract/auth-scope-phase2.md` §3). Every `/v1/` route
//! except `GET /v1/health` is expected to run behind this middleware.

use axum::Json;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use msc_api::dto::ErrorDto;

/// Env var carrying the one fixed dev token. Unset or empty means every
/// request is rejected — there is no "auth disabled" mode.
const DEV_TOKEN_ENV_VAR: &str = "MSC_DEV_TOKEN";

pub async fn require_bearer_token(request: Request, next: Next) -> Response {
    if is_authorized(request.headers()) {
        next.run(request).await
    } else {
        unauthorized()
    }
}

fn is_authorized(headers: &HeaderMap) -> bool {
    let expected = std::env::var(DEV_TOKEN_ENV_VAR).unwrap_or_default();
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    // A plain equality check is fine — `expected` is a throwaway dev
    // token, not a secret worth defending with constant-time comparison.
    // The real Phase 3 SecretStore-backed check will care about that.
    matches!(presented, Some(token) if !expected.is_empty() && token == expected)
}

fn unauthorized() -> Response {
    let body = ErrorDto {
        code: "unauthorized".to_string(),
        message: "Missing or invalid bearer token.".to_string(),
        help_id: None,
        details: None,
    };
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_bearer(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    #[test]
    fn rejects_missing_header() {
        // SAFETY: tests run single-threaded within this module's env-var
        // mutations via `cargo nextest` (process-per-test isolation).
        unsafe { std::env::set_var(DEV_TOKEN_ENV_VAR, "dev-secret") };
        assert!(!is_authorized(&HeaderMap::new()));
    }

    #[test]
    fn rejects_wrong_token() {
        unsafe { std::env::set_var(DEV_TOKEN_ENV_VAR, "dev-secret") };
        assert!(!is_authorized(&headers_with_bearer("wrong")));
    }

    #[test]
    fn accepts_matching_token() {
        unsafe { std::env::set_var(DEV_TOKEN_ENV_VAR, "dev-secret") };
        assert!(is_authorized(&headers_with_bearer("dev-secret")));
    }

    #[test]
    fn rejects_when_env_var_unset() {
        unsafe { std::env::remove_var(DEV_TOKEN_ENV_VAR) };
        assert!(!is_authorized(&headers_with_bearer("anything")));
    }
}
