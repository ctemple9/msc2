//! Hand-written coverage of operation-model.md §3's transition table.
//!
//! Greenfield MSC 2 construction, not a port (per P2.9's own framing in
//! `rolling-plan.md`: "MSC 1 has no operation-journal concept... there is
//! no MSC 1 fixture to extract"), so unlike most of `msc-domain`'s test
//! suite this exercises the implementation directly against its written
//! specification rather than against extracted fixtures.
//!
//! Test functions are prefixed `operation_` so the plan's Verify command
//! (a plain nextest substring filter, which matches on test name, not
//! file/binary name) selects all of them.

use msc_domain::operation::{OperationId, OperationState};

const ALL_STATES: [OperationState; 5] = [
    OperationState::Queued,
    OperationState::Running,
    OperationState::Succeeded,
    OperationState::Failed,
    OperationState::Cancelled,
];

const LEGAL_TRANSITIONS: [(OperationState, OperationState); 5] = [
    (OperationState::Queued, OperationState::Running),
    (OperationState::Running, OperationState::Succeeded),
    (OperationState::Running, OperationState::Failed),
    (OperationState::Queued, OperationState::Cancelled),
    (OperationState::Running, OperationState::Cancelled),
];

fn is_legal(from: OperationState, to: OperationState) -> bool {
    LEGAL_TRANSITIONS.contains(&(from, to))
}

#[test]
fn operation_legal_transitions_succeed() {
    for (from, to) in LEGAL_TRANSITIONS {
        assert_eq!(
            from.transition_to(to),
            Ok(to),
            "{from:?} -> {to:?} should be legal"
        );
    }
}

/// Exhaustive over all 25 (from, to) pairs, not just a handful of illegal
/// examples — every pair outside the five legal transitions above must be
/// rejected, including same-state "transitions" and every transition out
/// of a terminal state.
#[test]
fn operation_illegal_transitions_are_rejected() {
    for from in ALL_STATES {
        for to in ALL_STATES {
            let result = from.transition_to(to);
            if is_legal(from, to) {
                assert_eq!(result, Ok(to));
            } else {
                assert_eq!(
                    result,
                    Err(msc_domain::operation::IllegalTransition { from, to }),
                    "{from:?} -> {to:?} should be illegal"
                );
            }
        }
    }
}

#[test]
fn operation_terminal_states_accept_no_further_transition() {
    for terminal in [
        OperationState::Succeeded,
        OperationState::Failed,
        OperationState::Cancelled,
    ] {
        assert!(terminal.is_terminal());
        for to in ALL_STATES {
            assert!(
                terminal.transition_to(to).is_err(),
                "{terminal:?} -> {to:?} should be rejected: terminal states are final"
            );
        }
    }
}

#[test]
fn operation_non_terminal_states_are_not_terminal() {
    assert!(!OperationState::Queued.is_terminal());
    assert!(!OperationState::Running.is_terminal());
}

#[test]
fn operation_state_raw_value_round_trips() {
    for state in ALL_STATES {
        assert_eq!(
            OperationState::from_raw_value(state.raw_value()),
            Some(state)
        );
    }
    assert_eq!(OperationState::from_raw_value("bogus"), None);
}

#[test]
fn operation_id_preserves_its_string() {
    let id = OperationId::new("01J8XG7K9QZR3F5T6M2N8P0VBC");
    assert_eq!(id.as_str(), "01J8XG7K9QZR3F5T6M2N8P0VBC");
}
