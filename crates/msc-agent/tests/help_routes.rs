#[path = "../src/help.rs"]
mod help;
#[path = "../src/routes/help.rs"]
mod help_routes;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    help_routes::router(help::HelpContent::embedded().expect("valid embedded corpus"))
}

#[tokio::test]
async fn serves_raw_markdown_and_an_honest_unknown_topic_error() {
    let response = app()
        .oneshot(
            Request::get("/help/bedrock.runtime-unavailable")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let topic: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(topic["helpId"], "bedrock.runtime-unavailable");
    assert!(topic["body"].as_str().unwrap().contains("reason code"));

    let response = app()
        .oneshot(
            Request::get("/help/future.topic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["code"],
        "not_found"
    );
}

#[tokio::test]
async fn serves_browsable_catalogs_without_client_presentation_data() {
    let response = app()
        .oneshot(Request::get("/help/catalog").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let catalog: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(catalog["topics"].as_array().unwrap().len() >= 60);

    let response = app()
        .oneshot(
            Request::get("/guides/router-catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let guides: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(guides["guides"].as_array().unwrap().len(), 14);
    assert_eq!(guides["troubleshooting"].as_array().unwrap().len(), 9);
}
