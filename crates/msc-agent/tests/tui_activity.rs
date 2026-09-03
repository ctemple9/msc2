mod test_cli {
    pub use crate::cli::{CliError, CommonArgs};

    pub fn resolve_base_url(common: &CommonArgs) -> String {
        common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", common.host, common.port))
    }

    pub fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
        common
            .token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/mod.rs"]
mod cli;

use cli::tui::activity::{ActivityState, NOTIFICATION_HISTORY_LIMIT, OPERATION_HISTORY_LIMIT};
use cli::tui::confirm::{
    ConfirmAction, ConfirmationRequest, ConfirmationResult, ConfirmationState,
};
use crossterm::event::KeyCode;
use msc_api::dto::{NotificationEventDto, OperationDto, OperationStateDto};

fn operation(id: &str, state: OperationStateDto) -> OperationDto {
    OperationDto {
        id: id.to_string(),
        r#type: "backup".to_string(),
        target: Some("paper-1".to_string()),
        state,
        progress: None,
        status_line: Some("working".to_string()),
        result: None,
        error: None,
    }
}

fn notification(id: &str) -> NotificationEventDto {
    NotificationEventDto {
        id: id.to_string(),
        server_id: "paper-1".to_string(),
        occurred_at_iso8601: "2026-09-02T12:00:00Z".to_string(),
        kind: "server_started".to_string(),
        title: "Server Started".to_string(),
        body: "Paper is now online.".to_string(),
        help_id: None,
    }
}

#[test]
fn activity_keeps_bounded_deduplicated_operation_and_notification_state() {
    let mut activity = ActivityState::default();
    for index in 0..(OPERATION_HISTORY_LIMIT + 1) {
        activity.accept_operation_for_test(operation(
            &format!("op-{index}"),
            OperationStateDto::Running,
        ));
    }
    assert_eq!(activity.operations().count(), OPERATION_HISTORY_LIMIT);
    assert_eq!(activity.operations().next().unwrap().id, "op-1");

    let mut updated = operation("op-64", OperationStateDto::Succeeded);
    updated.status_line = Some("complete".to_string());
    activity.accept_operation_for_test(updated);
    assert_eq!(activity.operations().count(), OPERATION_HISTORY_LIMIT);
    assert_eq!(
        activity.operations().last().unwrap().state,
        OperationStateDto::Succeeded
    );

    for index in 0..(NOTIFICATION_HISTORY_LIMIT + 1) {
        activity.accept_notification_for_test(notification(&format!("note-{index}")));
    }
    assert_eq!(activity.notifications().count(), NOTIFICATION_HISTORY_LIMIT);
    assert_eq!(activity.notifications().next().unwrap().id, "note-1");
    activity.accept_notification_for_test(notification("note-200"));
    assert_eq!(activity.notifications().count(), NOTIFICATION_HISTORY_LIMIT);
}

#[test]
fn confirmation_exposes_target_and_only_dispatches_after_explicit_acceptance() {
    let mut state = ConfirmationState::default();
    state.begin(ConfirmationRequest {
        host: "host-a:48001".to_string(),
        server: "Paper".to_string(),
        target: "paper-1".to_string(),
        consequence: "Players may be disconnected.".to_string(),
        action: ConfirmAction::StopServer,
    });
    assert_eq!(state.request().unwrap().target, "paper-1");
    assert_eq!(
        state.handle_key(KeyCode::Char('n')),
        Some(ConfirmationResult::Cancelled)
    );
    assert!(state.is_open());
    let request = state.resolve(ConfirmationResult::Cancelled).unwrap();
    assert_eq!(request.server, "Paper");
    assert!(!state.is_open());
}
