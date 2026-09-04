use std::path::Path;

use msc_infrastructure::console_buffer::{
    ConsoleBuffer, ConsoleLine, ConsoleLineFramer, http_tail_count,
};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/console-framing")
        .join(format!("{name}.json"));
    let data = std::fs::read_to_string(&path).expect("fixture should be readable");
    serde_json::from_str(&data).expect("fixture should be valid JSON")
}

fn run_framing_fixture(name: &str) {
    let fixture = fixture(name);
    let chunks: Vec<Vec<u8>> = serde_json::from_value(fixture["input"]["chunks"].clone()).unwrap();
    let flush = fixture["input"]["flush"].as_bool().unwrap_or(false);
    let expected: Vec<String> =
        serde_json::from_value(fixture["expected"]["lines"].clone()).unwrap();

    let mut framer = ConsoleLineFramer::new();
    let mut actual = Vec::new();
    for chunk in chunks {
        actual.extend(framer.push_bytes(&chunk));
    }
    if flush {
        actual.extend(framer.flush());
    }

    assert_eq!(actual, expected);
}

#[test]
fn console_framing_split_line_across_byte_chunks() {
    run_framing_fixture("split-line-across-byte-chunks");
}

#[test]
fn console_framing_mixed_newline_boundaries_strip_carriage_return() {
    run_framing_fixture("mixed-newline-boundaries-strip-carriage-return");
}

#[test]
fn console_framing_trailing_partial_line_flushes_on_eof() {
    run_framing_fixture("trailing-partial-line-flushes-on-eof");
}

#[test]
fn console_framing_flush_after_complete_line_emits_no_extra_line() {
    run_framing_fixture("flush-after-complete-line-emits-no-extra-line");
}

#[test]
fn console_framing_blank_console_lines_are_preserved() {
    run_framing_fixture("blank-console-lines-are-preserved");
}

#[test]
fn console_framing_invalid_utf8_is_lossy_decoded() {
    run_framing_fixture("invalid-utf8-is-lossy-decoded");
}

#[test]
fn console_framing_history_limit_backfill_and_tail_clamps() {
    let fixture = fixture("history-limit-backfill-and-tail-clamps");
    let line_count = fixture["input"]["lineCount"].as_u64().unwrap();
    let expected = &fixture["expected"];

    let mut buffer = ConsoleBuffer::new();
    for index in 1..=line_count {
        buffer.push(ConsoleLine {
            ts: index.to_string(),
            source: "server".to_string(),
            level: Some("info".to_string()),
            auto: false,
            text: format!("line-{index}"),
        });
    }

    assert_eq!(
        buffer.len(),
        expected["historyLen"].as_u64().unwrap() as usize
    );
    assert_eq!(
        buffer.oldest().map(|line| line.text.as_str()),
        expected["firstStored"].as_str()
    );

    assert_eq!(
        buffer.tail(http_tail_count(None)).len(),
        expected["defaultTailLen"].as_u64().unwrap() as usize
    );
    assert_eq!(
        buffer
            .tail(http_tail_count(
                fixture["input"]["tailRequests"]["belowMin"].as_str()
            ))
            .len(),
        expected["belowMinTailLen"].as_u64().unwrap() as usize
    );
    assert_eq!(
        buffer
            .tail(http_tail_count(
                fixture["input"]["tailRequests"]["aboveMax"].as_str()
            ))
            .len(),
        expected["aboveMaxTailLen"].as_u64().unwrap() as usize
    );
    assert_eq!(
        buffer
            .tail(http_tail_count(
                fixture["input"]["tailRequests"]["invalid"].as_str()
            ))
            .len(),
        expected["invalidTailLen"].as_u64().unwrap() as usize
    );

    let backfill = buffer.websocket_backfill();
    assert_eq!(
        backfill.len(),
        expected["webSocketBackfillLen"].as_u64().unwrap() as usize
    );
    assert_eq!(
        backfill.first().map(|line| line.text.as_str()),
        expected["webSocketBackfillFirst"].as_str()
    );

    let three_tail_texts = buffer
        .tail(http_tail_count(
            fixture["input"]["tailRequests"]["three"].as_str(),
        ))
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>();
    let expected_three: Vec<String> =
        serde_json::from_value(expected["threeTailTexts"].clone()).unwrap();
    assert_eq!(three_tail_texts, expected_three);
}

#[test]
fn console_auto_classifier_marks_metrics_as_a_bounded_family() {
    let mut buffer = ConsoleBuffer::new();
    let spark_lines = [
        "[12:00:00] [spark/INFO]: TPS from last 5s, 10s, 1m, 5m, 15m:",
        "[12:00:00] [spark/INFO]: *20.0, *20.0, *20.0, *20.0, 19.85",
        "[12:00:00] [spark/INFO]:",
        "[12:00:00] [spark/INFO]: Tick durations (min/med/95%ile/max ms) from last 10s, 1m:",
        "[12:00:00] [spark/INFO]: 0.4/1.3/3.7/23.3; 0.4/1.3/2.6/23.3",
        "[12:00:00] [spark/INFO]:",
        "[12:00:00] [spark/INFO]: CPU usage from last 10s, 1m, 15m:",
        "[12:00:00] [spark/INFO]: 47%, 46%, 62% (system)",
        "[12:00:00] [spark/INFO]: 3%, 3%, 5% (process)",
    ];

    let classified = spark_lines
        .into_iter()
        .map(|text| buffer.push(ConsoleLine::new("stdout", None, text)).auto)
        .collect::<Vec<_>>();

    assert!(classified.into_iter().all(|is_auto| is_auto));
    assert!(
        !buffer
            .push(ConsoleLine::new("stdout", None, "A player joined the game"))
            .auto
    );
    assert!(
        buffer
            .push(ConsoleLine::new(
                "stdout",
                None,
                "There are 0 of a max of 20 players online:"
            ))
            .auto
    );
    assert!(
        buffer
            .push(ConsoleLine::new(
                "stdout",
                None,
                "[13:15:01 INFO] [Primary Session] Updated session!"
            ))
            .auto
    );
    assert!(
        buffer
            .push(ConsoleLine::new(
                "stdout",
                None,
                "\u{fffd}[36;1m[Primary Session] Updated session!\u{fffd}[m"
            ))
            .auto
    );
}

#[test]
fn console_auto_classifier_hides_routine_helpers_but_keeps_attention_lines() {
    let mut buffer = ConsoleBuffer::new();

    assert!(
        buffer
            .push(ConsoleLine::new(
                "xbox-broadcast",
                None,
                "Broadcast connected"
            ))
            .auto
    );
    assert!(
        !buffer
            .push(ConsoleLine::new(
                "xbox-broadcast",
                None,
                "[Xbox Broadcast stderr] connection failed"
            ))
            .auto
    );
    assert!(
        !buffer
            .push(ConsoleLine::new(
                "playit",
                None,
                "Please visit the login page"
            ))
            .auto
    );
}
