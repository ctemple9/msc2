//! Agent routes for the shared handbook and structured guides. Rendering and
//! first-launch presentation stay in the clients; this module returns data.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::ErrorDto;

use crate::help::HelpContent;

pub fn router(content: HelpContent) -> axum::Router {
    axum::Router::new()
        .route("/help/catalog", axum::routing::get(catalog))
        .route("/help/:help_id", axum::routing::get(topic))
        .route("/guides/concept-guide", axum::routing::get(concept_guide))
        .route("/guides/onboarding", axum::routing::get(onboarding))
        .route("/guides/router-catalog", axum::routing::get(router_catalog))
        .with_state(content)
}

pub async fn catalog(State(content): State<HelpContent>) -> Json<crate::help::HelpCatalog> {
    Json(content.catalog())
}

pub async fn topic(State(content): State<HelpContent>, Path(help_id): Path<String>) -> Response {
    match content.topic(&help_id) {
        Some(topic) => Json(topic).into_response(),
        None => not_found(&help_id),
    }
}

pub async fn concept_guide(State(content): State<HelpContent>) -> Response {
    json_or_error(content.concept_guide())
}

pub async fn onboarding(State(content): State<HelpContent>) -> Response {
    json_or_error(content.onboarding())
}

pub async fn router_catalog(State(content): State<HelpContent>) -> Response {
    json_or_error(content.router_catalog())
}

fn json_or_error(value: Result<serde_json::Value, String>) -> Response {
    match value {
        Ok(value) => Json(value).into_response(),
        Err(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorDto {
                code: "content_invalid".into(),
                message,
                help_id: None,
                details: None,
            }),
        )
            .into_response(),
    }
}

fn not_found(help_id: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorDto {
            code: "not_found".into(),
            message: format!("No help topic named '{help_id}'."),
            help_id: None,
            details: None,
        }),
    )
        .into_response()
}
