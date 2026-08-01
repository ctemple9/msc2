//! `GET /v1/health` — a single canned health card, standing in for real
//! health-check detection (later-phase substrate work). Unlike every other
//! route this phase wires, this one runs outside the bearer-auth gate
//! (`auth-scope-phase2.md` §3, item 1).

use axum::Json;
use msc_api::dto::{HealthCardDto, HealthResponseDto};

pub async fn health() -> Json<HealthResponseDto> {
    Json(HealthResponseDto {
        server_type: "paper".to_string(),
        server_name: "demo-survival".to_string(),
        server_running: true,
        overall_severity: "ok".to_string(),
        cards: vec![HealthCardDto {
            id: "demo-card".to_string(),
            title: "No real checks run yet".to_string(),
            short_label: "Demo".to_string(),
            severity: "ok".to_string(),
            detail: Some(
                "msc-agent's skeletal P2.13 handler returns this card unconditionally; \
                 real health checks are later-phase work."
                    .to_string(),
            ),
            icon_system_name: "checkmark.circle".to_string(),
            action_label: None,
            action_code: None,
            help_id: None,
        }],
        note: Some(
            "Placeholder data — msc-agent has no real health-check detection yet.".to_string(),
        ),
    })
}
