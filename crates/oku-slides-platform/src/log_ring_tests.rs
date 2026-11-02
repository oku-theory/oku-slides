use rstest::rstest;
use speculoos::prelude::*;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Layer, SubscriberExt};

use super::{LOG_LINE_MAX, LOG_RING_CAP, LogLine, LogRing, RingLayer, captured, clip_line, redact_home};
use crate::log_secrets::{REDACTED, register_secret};

/// A fresh ring for one test, living as long as the test binary.
fn ring() -> &'static LogRing {
    Box::leak(Box::new(LogRing::new()))
}

/// Runs `f` under a subscriber whose only layer fills `ring` from `WARN` up, redacting `home`.
fn logged(ring: &'static LogRing, home: Option<&str>, f: impl FnOnce()) {
    let layer = RingLayer::new(ring, home.map(str::to_owned)).with_filter(LevelFilter::WARN);
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), f);
}

fn line(n: usize) -> LogLine {
    LogLine { level: "WARN".into(), at: String::new(), target: "t".into(), message: format!("line {n}"), tab: None }
}

#[test]
fn ring_layer_info_warn_and_error_keeps_only_warn_and_error() {
    let ring = ring();
    logged(ring, None, || {
        tracing::info!("not kept");
        tracing::warn!("kept warning");
        tracing::error!("kept error");
    });
    let levels: Vec<String> = ring.lines().into_iter().map(|l| l.level).collect();
    assert_that!(levels).is_equal_to(vec!["WARN".to_owned(), "ERROR".to_owned()]);
}

#[test]
fn ring_layer_event_with_fields_puts_them_after_the_message() {
    let ring = ring();
    logged(ring, None, || tracing::warn!(tab = "t-1", count = 3, "render failed"));
    let lines = ring.lines();
    assert_that!(lines[0].message.as_str()).is_equal_to("render failed tab=t-1 count=3");
    assert_that!(lines[0].target.as_str()).is_equal_to(module_path!());
}

#[test]
fn ring_layer_event_time_is_rfc3339() {
    let ring = ring();
    logged(ring, None, || tracing::warn!("x"));
    let at = ring.lines()[0].at.clone();
    let parsed = time::OffsetDateTime::parse(&at, &time::format_description::well_known::Rfc3339);
    assert_that!(parsed).is_ok();
}

#[test]
fn ring_layer_message_holding_the_home_dir_comes_back_with_a_tilde() {
    let ring = ring();
    logged(ring, Some("/Users/ann"), || tracing::error!(path = "/Users/ann/deck.pdf", "open failed"));
    assert_that!(ring.lines()[0].message.as_str()).is_equal_to("open failed path=~/deck.pdf");
}

#[test]
fn ring_layer_long_line_is_cut_with_an_ellipsis() {
    let ring = ring();
    let long = "é".repeat(LOG_LINE_MAX + 20);
    logged(ring, None, || tracing::warn!("{long}"));
    let message = ring.lines()[0].message.clone();
    assert_that!(message.chars().count()).is_equal_to(LOG_LINE_MAX);
    assert_that!(message.ends_with('…')).is_true();
}

#[test]
fn log_ring_past_the_cap_drops_the_oldest() {
    let ring = LogRing::new();
    for n in 0..LOG_RING_CAP + 5 {
        ring.push(line(n));
    }
    let lines = ring.lines();
    assert_that!(lines.len()).is_equal_to(LOG_RING_CAP);
    assert_that!(lines[0].message.as_str()).is_equal_to("line 5");
    assert_that!(lines[LOG_RING_CAP - 1].message.clone()).is_equal_to(format!("line {}", LOG_RING_CAP + 4));
}

#[rstest]
#[case("/Users/ann/x and /Users/ann", Some("/Users/ann"), "~/x and ~")]
#[case("/Users/ann/x", Some("/Users/ann/"), "~/x")]
#[case("/Users/anna/x", Some("/Users/ann"), "/Users/anna/x")]
#[case("/etc/x", Some("/"), "/etc/x")]
#[case("/etc/x", Some(""), "/etc/x")]
#[case("/Users/ann/x", None, "/Users/ann/x")]
fn redact_home_cases_replace_only_whole_home_paths(
    #[case] text: &str,
    #[case] home: Option<&str>,
    #[case] expected: &str,
) {
    assert_that!(redact_home(text, home).as_str()).is_equal_to(expected);
}

#[rstest]
#[case("short", 10, "short")]
#[case("exactly10!", 10, "exactly10!")]
#[case("eleven char", 10, "eleven ch…")]
fn clip_line_cases_cut_past_max(#[case] text: &str, #[case] max: usize, #[case] expected: &str) {
    assert_that!(clip_line(text, max).as_str()).is_equal_to(expected);
}

#[test]
fn ring_layer_event_with_a_tab_field_keeps_the_tab() {
    let ring = ring();
    let id = String::from("t-7");
    logged(ring, None, || {
        tracing::warn!(tab = "t-1", "as str");
        tracing::warn!(tab = %id, "as display");
        tracing::warn!("no tab");
    });
    let tabs: Vec<Option<String>> = ring.lines().into_iter().map(|l| l.tab).collect();
    assert_that!(tabs).is_equal_to(vec![Some("t-1".to_owned()), Some("t-7".to_owned()), None]);
}

#[test]
fn ring_layer_registered_secret_never_reaches_the_ring() {
    register_secret("ring-secret-0123456789");
    let ring = ring();
    logged(ring, None, || tracing::error!(header = "Bearer ring-secret-0123456789", "mcp refused"));
    let message = ring.lines()[0].message.clone();
    assert_that!(message.contains("ring-secret")).is_false();
    assert_that!(message).is_equal_to(format!("mcp refused header=Bearer {REDACTED}"));
}

#[test]
fn captured_registered_secret_is_masked() {
    register_secret("stderr-secret-abcdefgh");
    assert_that!(captured("KEY=stderr-secret-abcdefgh", None)).is_equal_to(format!("KEY={REDACTED}"));
}
