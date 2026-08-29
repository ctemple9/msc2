//! Agent routes for the shared handbook and router guides. Rendering and
//! first-launch presentation stay in the clients; this module returns data.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use msc_api::dto::ErrorDto;
use msc_domain::router::composer;
use msc_domain::router::fallback_tree::{self, FallbackState};
use msc_domain::router::matcher;
use msc_domain::router::runtime_resolver::{self, ResolvedItem, RuntimeContext};
use msc_domain::router::troubleshooting::{self, SymptomId};
use msc_domain::router_guides::{self, RouterGuide, RouterTroubleshootingTopic};
use serde::{Deserialize, Serialize};

use crate::help::HelpContent;

pub fn router(content: HelpContent) -> Router {
    Router::new()
        .route("/help/catalog", get(catalog))
        .route("/help/:help_id", get(topic))
        .route("/guides/concept-guide", get(concept_guide))
        .route("/guides/onboarding", get(onboarding))
        .route("/guides/router-catalog", get(router_catalog))
        .route("/guides/router/search", get(router_search))
        .route("/guides/router/:guide_id", get(router_guide))
        .route(
            "/guides/router/troubleshooting/analyze",
            axum::routing::post(router_troubleshooting_analyze),
        )
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

#[derive(Debug, Deserialize)]
struct RouterSearchQuery {
    q: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouterGuideSummary {
    id: String,
    family: String,
    category: String,
    display_name: String,
    provider_display_name: Option<String>,
    device_display_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouterMatchCandidate {
    guide: RouterGuideSummary,
    score: i32,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouterFallbackResolution {
    kind: String,
    availability: String,
    matched_guide_id: Option<String>,
    fallback_guide_id: Option<String>,
    desired_family: Option<String>,
    inferred_families: Vec<String>,
    explanation_bullets: Vec<String>,
    recommended_next_node_id: Option<String>,
    suggested_search_terms: Vec<String>,
    matched_query: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouterSearchResponse {
    query: String,
    normalized_query: String,
    normalized_tokens: Vec<String>,
    inferred_families: Vec<String>,
    candidates: Vec<RouterMatchCandidate>,
    suggested_fallback_guide: Option<RouterGuideSummary>,
    is_ambiguous: bool,
    matched_direct_guide: bool,
    fallback_resolution: RouterFallbackResolution,
}

async fn router_search(Query(query): Query<RouterSearchQuery>) -> Response {
    let (raw_guides, matcher_guides, _) = match engine_catalogs() {
        Ok(value) => value,
        Err(response) => return response,
    };
    let result = matcher::match_query(&query.q, &matcher_guides);
    let fallback = fallback_tree::resolve(
        &FallbackState {
            search_query: query.q.clone(),
            ..FallbackState::default()
        },
        &matcher_guides,
        None,
    );
    let candidates = result
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            raw_guides
                .iter()
                .find(|guide| guide.id == candidate.guide.id)
                .map(|guide| RouterMatchCandidate {
                    guide: guide_summary(guide),
                    score: candidate.score,
                    reasons: candidate.reasons.into_iter().map(str::to_string).collect(),
                })
        })
        .collect();
    let suggested_fallback_guide = result.suggested_fallback_guide.and_then(|guide| {
        raw_guides
            .iter()
            .find(|raw| raw.id == guide.id)
            .map(guide_summary)
    });

    Json(RouterSearchResponse {
        query: query.q,
        normalized_query: result.normalized_query,
        normalized_tokens: result.normalized_tokens,
        inferred_families: result
            .inferred_families
            .into_iter()
            .map(|family| family.raw_value().to_string())
            .collect(),
        candidates,
        suggested_fallback_guide,
        is_ambiguous: result.is_ambiguous,
        matched_direct_guide: result.matched_direct_guide,
        fallback_resolution: fallback_resolution(&fallback),
    })
    .into_response()
}

async fn router_guide(
    State(content): State<HelpContent>,
    Path(guide_id): Path<String>,
) -> Response {
    let raw_guides = match router_guides::embedded_catalog() {
        Ok(guides) => guides,
        Err(error) => return content_error(error.to_string()),
    };
    let Some(raw_guide) = raw_guides.iter().find(|guide| guide.id == guide_id) else {
        return not_found(&format!("router guide {guide_id}"));
    };
    let Some(context) = content.router_runtime_context() else {
        return error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "Select a server before opening a resolved router guide.",
        );
    };
    let (_, _, composer_guides, topics) = match composer_catalog() {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(composed) = composer::compose_guide_by_id(&guide_id, &composer_guides, &topics) else {
        return not_found(&format!("router guide {guide_id}"));
    };
    let resolved = runtime_resolver::resolve(composed, &context);
    Json(ResolvedRouterGuide {
        guide: raw_guide.clone(),
        runtime: runtime_summary(&context),
        sections: resolved
            .sections
            .into_iter()
            .map(resolved_section)
            .collect(),
        unresolved_tokens: resolved
            .unresolved_tokens
            .into_iter()
            .map(|token| UnresolvedTokenDto {
                section_id: token.section_id,
                token: token.token.raw_value().to_string(),
            })
            .collect(),
    })
    .into_response()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSummary {
    selected_server_id: Option<String>,
    selected_server_name: Option<String>,
    detected_local_ip_address: Option<String>,
    detected_gateway_ip_address: Option<String>,
    java_port: Option<i32>,
    bedrock_port: Option<i32>,
    recommended_protocol: Option<String>,
    bedrock_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedRouterGuide {
    guide: RouterGuide,
    runtime: RuntimeSummary,
    sections: Vec<ResolvedSectionDto>,
    unresolved_tokens: Vec<UnresolvedTokenDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedSectionDto {
    id: String,
    kind: String,
    title: String,
    origin: String,
    items: Vec<ResolvedItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ResolvedItemDto {
    Paragraph {
        title: Option<String>,
        body: String,
    },
    BulletList {
        title: Option<String>,
        bullets: Vec<String>,
    },
    MenuPath {
        title: Option<String>,
        path: Vec<String>,
        alternate_menu_names: Vec<String>,
    },
    Step {
        id: String,
        kind: String,
        title: String,
        body: String,
        alternate_terms: Vec<String>,
    },
    Note {
        id: String,
        title: Option<String>,
        body: String,
    },
    TroubleshootingTopic {
        id: String,
        title: String,
        summary: String,
        suggested_next_actions: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnresolvedTokenDto {
    section_id: String,
    token: String,
}

fn resolved_section(section: runtime_resolver::ResolvedSection) -> ResolvedSectionDto {
    ResolvedSectionDto {
        id: section.id,
        kind: section.kind.raw_value().to_string(),
        title: section.title.to_string(),
        origin: section.origin.raw_value().to_string(),
        items: section.items.into_iter().map(resolved_item).collect(),
    }
}

fn resolved_item(item: ResolvedItem) -> ResolvedItemDto {
    match item {
        ResolvedItem::Paragraph { title, body } => ResolvedItemDto::Paragraph { title, body },
        ResolvedItem::BulletList { title, bullets } => {
            ResolvedItemDto::BulletList { title, bullets }
        }
        ResolvedItem::MenuPath {
            title,
            path,
            alternate_menu_names,
        } => ResolvedItemDto::MenuPath {
            title,
            path,
            alternate_menu_names,
        },
        ResolvedItem::Step(step) => ResolvedItemDto::Step {
            id: step.id,
            kind: step.kind.raw_value().to_string(),
            title: step.title,
            body: step.body,
            alternate_terms: step.alternate_terms,
        },
        ResolvedItem::Note(note) => ResolvedItemDto::Note {
            id: note.id,
            title: note.title,
            body: note.body,
        },
        ResolvedItem::TroubleshootingTopic(topic) => ResolvedItemDto::TroubleshootingTopic {
            id: topic.id.raw_value().to_string(),
            title: topic.title,
            summary: topic.summary,
            suggested_next_actions: topic.suggested_next_actions,
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeRequest {
    symptoms: Vec<String>,
    #[serde(default)]
    fallback_state: Option<FallbackStateRequest>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FallbackStateRequest {
    network_type: Option<String>,
    search_query: Option<String>,
    only_knows_isp: bool,
    only_knows_mesh_system: bool,
    unsure_whether_isp_or_own_router: bool,
    wants_advanced_troubleshooting: bool,
}

async fn router_troubleshooting_analyze(
    State(content): State<HelpContent>,
    body: Result<Json<AnalyzeRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "Request body must be valid JSON.",
            );
        }
    };
    let symptoms = match request
        .symptoms
        .iter()
        .map(|raw| SymptomId::from_raw_value(raw).ok_or_else(|| raw.clone()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(symptoms) => symptoms,
        Err(raw) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_symptom",
                &format!("Unknown router troubleshooting symptom '{raw}'."),
            );
        }
    };
    let (_, matcher_guides, _composer_guides, topics) = match composer_catalog() {
        Ok(value) => value,
        Err(response) => return response,
    };
    let fallback_state = request.fallback_state.map(fallback_state);
    let context = content.router_runtime_context();
    let report = troubleshooting::analyze(
        &symptoms,
        &troubleshooting::make_rules(),
        &topics,
        &matcher_guides,
        fallback_state.as_ref(),
        context
            .as_ref()
            .and_then(|context| context.detected_gateway_ip_address.as_deref()),
    );
    Json(analysis_response(report)).into_response()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisResponse {
    symptoms: Vec<String>,
    likely_causes: Vec<AnalysisCause>,
    recommended_actions: Vec<String>,
    escalation_bullets: Vec<String>,
    fallback_resolution: Option<RouterFallbackResolution>,
    summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisCause {
    id: String,
    confidence: String,
    score: i32,
    severity: String,
    matched_symptoms: Vec<String>,
    topic: RouterTroubleshootingTopic,
}

fn analysis_response(report: troubleshooting::Report<'_>) -> AnalysisResponse {
    AnalysisResponse {
        symptoms: report
            .symptoms
            .into_iter()
            .map(|symptom| symptom.raw_value().to_string())
            .collect(),
        likely_causes: report
            .likely_causes
            .into_iter()
            .map(|cause| AnalysisCause {
                id: cause.id.raw_value().to_string(),
                confidence: cause.confidence.raw_value().to_string(),
                score: cause.score,
                severity: cause.severity().raw_value().to_string(),
                matched_symptoms: cause
                    .matched_symptoms
                    .into_iter()
                    .map(|symptom| symptom.raw_value().to_string())
                    .collect(),
                topic: RouterTroubleshootingTopic {
                    id: cause.topic.id.raw_value().to_string(),
                    title: cause.topic.title.clone(),
                    summary: cause.topic.summary.clone(),
                    suggested_next_actions: cause.topic.suggested_next_actions.clone(),
                },
            })
            .collect(),
        recommended_actions: report.recommended_actions,
        escalation_bullets: report.escalation_bullets,
        fallback_resolution: report.fallback_resolution.as_ref().map(fallback_resolution),
        summary: report.summary,
    }
}

fn fallback_state(request: FallbackStateRequest) -> FallbackState {
    FallbackState {
        network_type: request
            .network_type
            .as_deref()
            .and_then(fallback_tree::NetworkType::from_raw_value),
        search_query: request.search_query.unwrap_or_default(),
        only_knows_isp: request.only_knows_isp,
        only_knows_mesh_system: request.only_knows_mesh_system,
        unsure_whether_isp_or_own_router: request.unsure_whether_isp_or_own_router,
        wants_advanced_troubleshooting: request.wants_advanced_troubleshooting,
    }
}

type EngineCatalogs = (Vec<RouterGuide>, Vec<matcher::Guide>, Vec<composer::Guide>);
type ComposerCatalog = (
    Vec<RouterGuide>,
    Vec<matcher::Guide>,
    Vec<composer::Guide>,
    Vec<composer::TroubleshootingTopic>,
);

#[allow(clippy::result_large_err)]
fn engine_catalogs() -> Result<EngineCatalogs, Response> {
    let raw_guides =
        router_guides::embedded_catalog().map_err(|error| content_error(error.to_string()))?;
    let matcher_guides = raw_guides
        .iter()
        .map(RouterGuide::to_matcher_guide)
        .collect::<Result<Vec<_>, _>>()
        .map_err(content_error)?;
    let composer_guides = raw_guides
        .iter()
        .map(RouterGuide::to_composer_guide)
        .collect::<Result<Vec<_>, _>>()
        .map_err(content_error)?;
    Ok((raw_guides, matcher_guides, composer_guides))
}

#[allow(clippy::result_large_err)]
fn composer_catalog() -> Result<ComposerCatalog, Response> {
    let (raw_guides, matcher_guides, composer_guides) = engine_catalogs()?;
    let topics = router_guides::embedded_troubleshooting_topics()
        .map_err(|error| content_error(error.to_string()))?
        .iter()
        .map(RouterTroubleshootingTopic::to_engine_topic)
        .collect::<Result<Vec<_>, _>>()
        .map_err(content_error)?;
    Ok((raw_guides, matcher_guides, composer_guides, topics))
}

fn guide_summary(guide: &RouterGuide) -> RouterGuideSummary {
    RouterGuideSummary {
        id: guide.id.clone(),
        family: guide.family.clone(),
        category: guide.category.clone(),
        display_name: guide.display_name.clone(),
        provider_display_name: guide.provider_display_name.clone(),
        device_display_name: guide.device_display_name.clone(),
    }
}

fn runtime_summary(context: &RuntimeContext) -> RuntimeSummary {
    RuntimeSummary {
        selected_server_id: context.selected_server_id.clone(),
        selected_server_name: context.selected_server_name.clone(),
        detected_local_ip_address: context.detected_local_ip_address.clone(),
        detected_gateway_ip_address: context.detected_gateway_ip_address.clone(),
        java_port: context.java_port,
        bedrock_port: context.bedrock_port,
        recommended_protocol: context.recommended_protocol.clone(),
        bedrock_enabled: context.bedrock_enabled,
    }
}

fn fallback_resolution(
    resolution: &fallback_tree::FallbackResolution<'_>,
) -> RouterFallbackResolution {
    RouterFallbackResolution {
        kind: resolution.kind.raw_value().to_string(),
        availability: resolution.availability.raw_value().to_string(),
        matched_guide_id: resolution.matched_guide.map(|guide| guide.id.clone()),
        fallback_guide_id: resolution.fallback_guide.map(|guide| guide.id.clone()),
        desired_family: resolution
            .desired_family
            .map(|family| family.raw_value().to_string()),
        inferred_families: resolution
            .inferred_families
            .iter()
            .map(|family| family.raw_value().to_string())
            .collect(),
        explanation_bullets: resolution.explanation_bullets.clone(),
        recommended_next_node_id: resolution
            .recommended_next_node_id
            .map(|node| node.raw_value().to_string()),
        suggested_search_terms: resolution
            .suggested_search_terms
            .iter()
            .map(|term| (*term).to_string())
            .collect(),
        matched_query: resolution.matched_query.clone(),
    }
}

fn json_or_error(value: Result<serde_json::Value, String>) -> Response {
    match value {
        Ok(value) => Json(value).into_response(),
        Err(message) => content_error(message),
    }
}

fn content_error(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "content_invalid",
        &message.into(),
    )
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(ErrorDto {
            code: code.into(),
            message: message.into(),
            help_id: None,
            details: None,
        }),
    )
        .into_response()
}

fn not_found(help_id: &str) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("No help topic named '{help_id}'."),
    )
}
