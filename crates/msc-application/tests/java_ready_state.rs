use msc_application::output_reducer::{
    JavaOutputReducer, OutputEvent, UnexpectedStopKind, parse_java_player_name,
};
use msc_domain::tps::Sample;
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-ready-state")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&raw).expect("fixture should be valid JSON")
}

fn line_events(case: &Value) -> Vec<OutputEvent> {
    let mut reducer = JavaOutputReducer::new();
    case["input"]["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .flat_map(|line| reducer.process_line(line.as_str().expect("line")))
        .collect()
}

fn event_kinds(events: &[OutputEvent]) -> Vec<&'static str> {
    events
        .iter()
        .map(|event| match event {
            OutputEvent::Ready => "ready",
            OutputEvent::PlayerJoined(_) => "player_joined",
            OutputEvent::PlayerLeft(_) => "player_left",
            OutputEvent::TpsSample(_) => "tps_sample",
        })
        .collect()
}

fn assert_events(case_name: &str) -> (Value, Vec<OutputEvent>) {
    let case = fixture(case_name);
    let events = line_events(&case);
    let expected = case["expected"]["events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|event| event.as_str().expect("event"))
        .collect::<Vec<_>>();
    assert_eq!(event_kinds(&events), expected, "case {case_name}");
    (case, events)
}

fn assert_sample(actual: &Sample, expected: &Value) {
    assert_eq!(actual.t1, expected["t1"].as_f64().expect("t1"));
    assert_eq!(actual.t5, expected["t5"].as_f64());
    assert_eq!(actual.t15, expected["t15"].as_f64());
}

#[test]
fn java_ready_state_paper_done_line_marks_ready() {
    let (case, events) = assert_events("paper-done-line-marks-ready");
    assert_eq!(events, vec![OutputEvent::Ready]);
    let mut reducer = JavaOutputReducer::new();
    for line in case["input"]["lines"].as_array().unwrap() {
        reducer.process_line(line.as_str().unwrap());
    }
    assert_eq!(
        reducer.reached_ready(),
        case["expected"]["reachedReady"].as_bool().unwrap()
    );
}

#[test]
fn java_ready_state_ready_is_emitted_once() {
    let (_case, events) = assert_events("ready-is-emitted-once");
    assert_eq!(events, vec![OutputEvent::Ready]);
}

#[test]
fn java_ready_state_exit_before_ready_is_startup_failure() {
    let case = fixture("exit-before-ready-is-startup-failure");
    let mut reducer = JavaOutputReducer::new();
    for line in case["input"]["lines"].as_array().unwrap() {
        reducer.process_line(line.as_str().unwrap());
    }
    assert_eq!(
        reducer.classify_unexpected_stop(),
        UnexpectedStopKind::StartupFailedBeforeReady
    );
    assert_eq!(
        case["expected"]["stopKind"].as_str().unwrap(),
        "startup_failed_before_ready"
    );
}

#[test]
fn java_ready_state_exit_after_ready_is_not_startup_failure() {
    let case = fixture("exit-after-ready-is-stopped-after-ready");
    let mut reducer = JavaOutputReducer::new();
    for line in case["input"]["lines"].as_array().unwrap() {
        reducer.process_line(line.as_str().unwrap());
    }
    assert_eq!(
        reducer.classify_unexpected_stop(),
        UnexpectedStopKind::StoppedAfterReady
    );
    assert_eq!(
        case["expected"]["stopKind"].as_str().unwrap(),
        "stopped_after_ready"
    );
}

#[test]
fn java_ready_state_java_join_line_parses_player_name() {
    let (case, events) = assert_events("java-join-line-parses-player-name");
    assert_eq!(
        parse_java_player_name(
            case["input"]["lines"][0].as_str().unwrap(),
            " joined the game"
        ),
        Some("camkage".to_string())
    );
    assert_eq!(events, vec![OutputEvent::PlayerJoined("camkage".into())]);
}

#[test]
fn java_ready_state_java_leave_line_removes_online_player() {
    let case = fixture("java-leave-line-removes-online-player");
    let mut reducer = JavaOutputReducer::new();
    let mut events = Vec::new();
    for line in case["input"]["lines"].as_array().unwrap() {
        events.extend(reducer.process_line(line.as_str().unwrap()));
    }
    assert_eq!(
        events,
        vec![
            OutputEvent::PlayerJoined("camkage".into()),
            OutputEvent::PlayerLeft("camkage".into())
        ]
    );
    let expected_players = case["expected"]["onlinePlayers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(reducer.online_players(), expected_players);
}

#[test]
fn java_ready_state_tps_line_hands_off_to_phase1_parser() {
    let (case, events) = assert_events("tps-line-hands-off-to-phase1-parser");
    match &events[..] {
        [OutputEvent::TpsSample(sample)] => assert_sample(sample, &case["expected"]["sample"]),
        other => panic!("unexpected events: {other:?}"),
    }
}

#[test]
fn java_ready_state_armed_spark_reply_emits_tps_sample() {
    let mut reducer = JavaOutputReducer::new();
    reducer.expect_spark_tps_reply();

    assert!(
        reducer
            .process_line("[08:02:10] [Server thread/INFO]: TPS from last 5s, 10s, 1m, 5m, 15m:")
            .is_empty()
    );
    assert_eq!(
        reducer.process_line("[08:02:10] [Server thread/INFO]: 20.0, 20.0, 19.8, 19.5, 18.0"),
        vec![OutputEvent::TpsSample(msc_domain::tps::Sample {
            t1: 19.8,
            t5: Some(19.5),
            t15: Some(18.0),
        })]
    );
}

#[test]
fn java_ready_state_unarmed_spark_reply_is_ignored() {
    let mut reducer = JavaOutputReducer::new();
    reducer.process_line("[08:02:10] [Server thread/INFO]: TPS from last 5s, 10s, 1m, 5m, 15m:");

    assert!(
        reducer
            .process_line("[08:02:10] [Server thread/INFO]: 20.0, 20.0, 19.8, 19.5, 18.0")
            .is_empty()
    );
}

#[test]
fn java_ready_state_bedrock_and_world_time_lines_are_ignored() {
    let (case, events) = assert_events("bedrock-and-world-time-lines-ignored");
    assert!(events.is_empty());
    assert!(!case["expected"]["reachedReady"].as_bool().unwrap());
}
