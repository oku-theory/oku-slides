use oku_slides_platform::telemetry::{LOG_LINE_MAX, LOG_RING_CAP, LogLine};
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_platform::telemetry::{REDACTED, register_secret};
use rstest::rstest;

use super::{STDERR_MAX_LINES, printable, redacted, reply};
use crate::diagnosis::{AgentDiagnosis, Diagnosis};

fn log(level: &str, n: usize) -> LogLine {
    LogLine {
        level: level.into(),
        at: "2026-11-01T10:00:00Z".into(),
        target: "t".into(),
        message: format!("m{n}"),
        tab: None,
    }
}

fn host_with_stderr(lines: usize) -> Diagnosis {
    let stderr = (0..lines).map(|n| format!("e{n}")).collect();
    Diagnosis { agent: AgentDiagnosis { stderr, ..AgentDiagnosis::default() }, dictation: None }
}

fn json_of(host: Diagnosis, log: Vec<LogLine>) -> Value {
    serde_json::to_value(reply(host, log, (None, None), "t-1")).unwrap()
}

#[test]
fn reply_has_the_five_keys() {
    let v = json_of(Diagnosis::default(), Vec::new());
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_that!(keys).is_equal_to(vec!["agent", "dictation", "import_error", "log", "render_error"]);
}

#[test]
fn reply_more_stderr_than_the_cap_keeps_the_last_lines() {
    let v = json_of(host_with_stderr(STDERR_MAX_LINES + 5), Vec::new());
    let stderr = v["agent"]["stderr"].as_array().unwrap();
    assert_that!(stderr.len()).is_equal_to(STDERR_MAX_LINES);
    assert_that!(stderr[0].as_str()).is_equal_to(Some("e5"));
}

#[test]
fn reply_more_log_lines_than_the_cap_keeps_the_last() {
    let lines = (0..LOG_RING_CAP + 10).map(|n| log("ERROR", n)).collect();
    let v = json_of(Diagnosis::default(), lines);
    let log = v["log"].as_array().unwrap();
    assert_that!(log.len()).is_equal_to(LOG_RING_CAP);
    assert_that!(log[0]["message"].as_str()).is_equal_to(Some("m10"));
}

#[test]
fn reply_info_and_debug_lines_are_dropped() {
    let v = json_of(Diagnosis::default(), vec![log("INFO", 0), log("WARN", 1), log("DEBUG", 2)]);
    let levels: Vec<&str> = v["log"].as_array().unwrap().iter().filter_map(|l| l["level"].as_str()).collect();
    assert_that!(levels).is_equal_to(vec!["WARN"]);
}

#[test]
fn reply_long_render_error_is_cut() {
    let long = "x".repeat(LOG_LINE_MAX * 2);
    let r = reply(Diagnosis::default(), Vec::new(), (Some(long), None), "t-1");
    let v = serde_json::to_value(r).unwrap();
    assert_that!(v["render_error"].as_str().map(|s| s.chars().count())).is_equal_to(Some(LOG_LINE_MAX));
}

#[test]
fn redacted_home_in_nested_strings_becomes_a_tilde() {
    let v = json!({ "a": ["/Users/ann/x", { "b": "at /Users/ann" }], "n": 3, "z": null });
    let expected = json!({ "a": ["~/x", { "b": "at ~" }], "n": 3, "z": null });
    assert_that!(redacted(v, Some("/Users/ann"))).is_equal_to(expected);
}

fn log_of(tab: Option<&str>, message: &str) -> LogLine {
    LogLine { tab: tab.map(str::to_owned), message: message.to_owned(), ..log("WARN", 0) }
}

#[test]
fn reply_log_keeps_the_tabs_lines_and_those_of_no_tab() {
    let lines = vec![log_of(Some("t-1"), "mine"), log_of(Some("t-2"), "theirs"), log_of(None, "app")];
    let v = json_of(Diagnosis::default(), lines);
    let messages: Vec<&str> = v["log"].as_array().unwrap().iter().filter_map(|l| l["message"].as_str()).collect();
    assert_that!(messages).is_equal_to(vec!["mine", "app"]);
}

#[test]
fn reply_log_line_has_no_tab_key() {
    let v = json_of(Diagnosis::default(), vec![log_of(Some("t-1"), "mine")]);
    let mut keys: Vec<&str> = v["log"][0].as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_that!(keys).is_equal_to(vec!["at", "level", "message", "target"]);
}

#[test]
fn reply_registered_secret_in_a_long_render_error_is_masked_before_the_cut() {
    register_secret("diag-secret-abcdefghij");
    let error = format!("{}diag-secret-abcdefghij", "x".repeat(LOG_LINE_MAX - 8));
    let v = serde_json::to_value(reply(Diagnosis::default(), Vec::new(), (Some(error), None), "t-1")).unwrap();
    assert_that!(v["render_error"].as_str().unwrap().contains("diag-secr")).is_false();
}

#[test]
fn redacted_registered_secret_in_stderr_is_masked() {
    register_secret("Bearer-diag-0123456789");
    let v = redacted(json!({ "stderr": ["auth: Bearer-diag-0123456789"] }), None);
    assert_that!(v).is_equal_to(json!({ "stderr": [format!("auth: {REDACTED}")] }));
}

#[rstest]
#[case("\u{1b}[31mred\u{1b}[0m text", "red text")]
#[case("title \u{1b}]0;pwned\u{7}after", "title after")]
#[case("osc \u{1b}]8;;http://x\u{1b}\\link", "osc link")]
#[case("bell\u{7} nul\u{0} c1\u{9b}", "bell nul c1")]
#[case("line\nbreak\ttab", "line break tab")]
#[case("plain é ✓", "plain é ✓")]
fn printable_cases_drop_escapes_and_controls(#[case] text: &str, #[case] expected: &str) {
    assert_that!(printable(text).as_str()).is_equal_to(expected);
}

#[test]
fn redacted_escapes_in_nested_strings_are_removed() {
    let v = redacted(json!({ "log": [{ "message": "\u{1b}[2Jcleared" }] }), None);
    assert_that!(v).is_equal_to(json!({ "log": [{ "message": "cleared" }] }));
}
