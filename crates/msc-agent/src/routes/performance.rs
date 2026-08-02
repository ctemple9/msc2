//! `GET /v1/performance` — latest bounded performance snapshot for the
//! active Java server.

use axum::Json;
use axum::extract::State;
use msc_api::dto::PerformanceSnapshotDto;

use crate::routes::status::StatusRoutesState;

pub async fn performance(State(state): State<StatusRoutesState>) -> Json<PerformanceSnapshotDto> {
    Json(state.performance())
}
