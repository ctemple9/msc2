#[path = "../src/help.rs"]
mod help;
#[path = "../src/routes/help.rs"]
mod help_routes;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use msc_domain::router::runtime_resolver::RuntimeContext;
use std::sync::Arc;
use tower::ServiceExt;

fn app() -> axum::Router {
    help_routes::router(help::HelpContent::embedded().expect("valid embedded corpus"))
}

fn app_with_runtime_context() -> axum::Router {
    let content = help::HelpContent::embedded()
        .expect("valid embedded corpus")
        .with_router_runtime_provider(Arc::new(|| {
            Some(RuntimeContext {
                selected_server_id: Some("server-1".into()),
                selected_server_name: Some("Survival Server".into()),
                detected_local_ip_address: Some("192.168.1.20".into()),
                detected_gateway_ip_address: Some("192.168.1.1".into()),
                java_port: Some(25570),
                bedrock_port: Some(19140),
                recommended_protocol: Some("Forward TCP and UDP".into()),
                bedrock_enabled: Some(true),
            })
        }));
    help_routes::router(content)
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
    assert_eq!(guides["symptoms"].as_array().unwrap().len(), 18);
    assert_eq!(guides["guides"][0]["steps"][0]["kind"], "intro");
    assert!(
        guides["guides"][0]["searchKeywords"]
            .as_array()
            .unwrap()
            .len()
            > 3
    );
}

#[tokio::test]
async fn composes_and_resolves_a_router_guide_for_the_selected_server() {
    let response = app_with_runtime_context()
        .oneshot(
            Request::get("/guides/router/generic-mesh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let guide: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(guide["guide"]["id"], "generic-mesh");
    assert_eq!(guide["runtime"]["selectedServerName"], "Survival Server");
    assert_eq!(guide["runtime"]["javaPort"], 25570);
    assert_eq!(guide["unresolvedTokens"].as_array().unwrap().len(), 0);
    assert!(
        guide["sections"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|section| section["items"].as_array().unwrap())
            .any(|item| item["type"] == "step" && item["body"].as_str().unwrap().contains("25570"))
    );
}

#[tokio::test]
async fn searches_and_analyzes_router_help_with_real_engine_output() {
    let response = app()
        .oneshot(
            Request::get("/guides/router/search?q=xfinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let search: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(search["candidates"][0]["guide"]["id"], "xfinity-gateway");
    assert_eq!(search["fallbackResolution"]["kind"], "exactGuide");

    let response = app()
        .oneshot(
            Request::post("/guides/router/troubleshooting/analyze")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symptoms":["cannot_connect_externally","router_rule_points_to_old_ip"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let analysis: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(analysis["likelyCauses"][0]["id"], "local_ip_changed");
    assert!(analysis["recommendedActions"].as_array().unwrap().len() >= 3);
}
