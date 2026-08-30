#[path = "../src/web_ui.rs"]
mod web_ui;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::routing::get;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    axum::Router::new()
        .route("/v1/health", get(|| async { "agent health" }))
        .fallback(get(web_ui::serve))
}

#[tokio::test]
async fn serves_packaged_hashed_assets_with_browser_safe_headers() {
    let response = app()
        .oneshot(
            Request::get("/")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-cache",
        "the HTML entry point must pick up a newly packaged asset manifest"
    );
    assert!(response.headers().contains_key("content-security-policy"));
    let index = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let index = std::str::from_utf8(&index).expect("UTF-8 index");
    let asset = index
        .split('"')
        .find(|part| part.starts_with("/assets/") && part.ends_with(".js"))
        .expect("hashed JavaScript entry in the packaged index");

    let response = app()
        .oneshot(
            Request::get(asset)
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn serves_the_splash_video_with_a_video_content_type() {
    let response = app()
        .oneshot(
            Request::get("/splash_intro.mp4")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "video/mp4");
    assert!(
        response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .len()
            > 0
    );
}

#[tokio::test]
async fn deep_links_use_the_app_but_v1_and_missing_assets_do_not() {
    let response = app()
        .oneshot(
            Request::get("/hosts/local/servers/demo/console")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = app()
        .oneshot(
            Request::get("/v1/health")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
        "agent health"
    );

    let response = app()
        .oneshot(
            Request::get("/v1/unknown")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app()
        .oneshot(
            Request::get("/assets/missing.js")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
