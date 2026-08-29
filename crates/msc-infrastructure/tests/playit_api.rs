use msc_infrastructure::playit_api::{
    PlayitApi, PlayitApiError, PlayitHttpResponse, PlayitHttpTransport, PlayitTransportError,
};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::Mutex;

struct FakeTransport {
    responses: Mutex<VecDeque<PlayitHttpResponse>>,
    requests: Mutex<Vec<(String, Value, Option<String>)>>,
}

impl FakeTransport {
    fn new(responses: impl IntoIterator<Item = (u16, Value)>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|(status, body)| PlayitHttpResponse {
                        status,
                        body: serde_json::to_vec(&body).unwrap(),
                    })
                    .collect(),
            ),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl PlayitHttpTransport for FakeTransport {
    fn post_json(
        &self,
        path: &str,
        body: &Value,
        authorization: Option<&str>,
    ) -> Result<PlayitHttpResponse, PlayitTransportError> {
        self.requests.lock().unwrap().push((
            path.to_owned(),
            body.clone(),
            authorization.map(str::to_owned),
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(PlayitTransportError::Network)
    }
}

#[test]
fn account_calls_use_the_expected_claim_shapes_and_authentication_boundary() {
    let transport = FakeTransport::new([
        (
            200,
            json!({"status":"success","data":{"session_key":"session-secret"}}),
        ),
        (200, json!({"status":"success","data":"WaitingForAgent"})),
        (200, json!({"status":"success","data":"ok"})),
        (
            200,
            json!({"status":"success","data":{"agent_id":"agent-123"}}),
        ),
        (
            200,
            json!({"status":"success","data":{"secret_key":"agent-secret"}}),
        ),
    ]);
    let api = PlayitApi::new(&transport);
    let session = api.sign_in("owner@example.test", "password").unwrap();
    api.claim_setup("0123456789").unwrap();
    api.claim_details("0123456789", &session).unwrap();
    assert_eq!(
        api.claim_accept("0123456789", "MSC Agent", &session)
            .unwrap(),
        "agent-123"
    );
    let secret = match api.claim_exchange("0123456789") {
        Ok(Some(secret)) => secret,
        Ok(None) => panic!("claim exchange was unexpectedly pending"),
        Err(error) => panic!("claim exchange failed: {error}"),
    };
    assert_eq!(secret.as_str(), "agent-secret");

    let requests = transport.requests.lock().unwrap();
    assert_eq!(requests[0].0, "/login/signin");
    assert_eq!(requests[0].1["email"], "owner@example.test");
    assert_eq!(requests[0].1["password"], "password");
    assert_eq!(requests[0].2, None);
    assert_eq!(requests[1].0, "/claim/setup");
    assert_eq!(requests[1].1["agent_type"], "assignable");
    assert_eq!(requests[1].1["version"], "playit 1.0.10");
    assert_eq!(requests[2].2.as_deref(), Some("session session-secret"));
    assert_eq!(requests[3].1["name"], "MSC Agent");
    assert_eq!(requests[4].0, "/claim/exchange");
    assert_eq!(requests[4].2, None);
}

#[test]
fn session_auth_falls_back_without_leaking_the_session_in_errors() {
    let transport = FakeTransport::new([
        (
            200,
            json!({"status":"success","data":{"session_key":"session-secret"}}),
        ),
        (
            401,
            json!({"status":"fail","data":{"type":"auth","message":"wrong scheme"}}),
        ),
        (200, json!({"status":"success","data":"ok"})),
    ]);
    let api = PlayitApi::new(&transport);
    let session = api.sign_in("owner@example.test", "password").unwrap();
    api.claim_details("0123456789", &session).unwrap();

    let requests = transport.requests.lock().unwrap();
    assert_eq!(requests[1].2.as_deref(), Some("session session-secret"));
    assert_eq!(requests[2].2.as_deref(), Some("agent-key session-secret"));
}

#[test]
fn provider_failures_map_to_stable_safe_categories() {
    for (status, body, expected) in [
        (
            401,
            json!({"status":"fail","data":"IncorrectCredentials"}),
            PlayitApiError::IncorrectCredentials,
        ),
        (
            403,
            json!({"status":"fail","data":"AccountBanned"}),
            PlayitApiError::AccountBanned,
        ),
        (
            401,
            json!({"status":"fail","data":"TotpRequired"}),
            PlayitApiError::TwoFactorRequired,
        ),
        (
            429,
            json!({"status":"fail","data":"slow down"}),
            PlayitApiError::RateLimited,
        ),
        (
            404,
            json!({"status":"fail","data":"AgentNotFound"}),
            PlayitApiError::AgentNotFound,
        ),
        (
            500,
            json!({"status":"error","data":{"message":"provider detail"}}),
            PlayitApiError::ApiFailure,
        ),
    ] {
        let transport = FakeTransport::new([(status, body)]);
        let error = PlayitApi::new(&transport)
            .sign_in("owner@example.test", "password")
            .err()
            .expect("sign-in should fail");
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("provider detail"));
        assert!(!error.to_string().contains("password"));
    }
}

#[test]
fn pending_claim_exchange_is_not_reported_as_an_api_failure() {
    let transport = FakeTransport::new([(
        200,
        json!({"status":"fail","data":{"type":"claim","message":"NotReady"}}),
    )]);
    let api = PlayitApi::new(&transport);
    assert!(matches!(api.claim_exchange("0123456789"), Ok(None)));
}
