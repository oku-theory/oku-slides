use oku_slides_platform::telemetry::LOG_LINE_MAX;
use rstest::rstest;
use speculoos::prelude::*;

use super::{LINE_KEPT, LineCut, STDERR_TAIL_CAP, StderrTail, forward};

/// Forwards `input` through a fresh launch of `tail`; returns what reached the writer.
async fn forwarded(tail: &StderrTail, input: &[u8]) -> Vec<u8> {
    let launch = tail.begin();
    let mut out = Vec::new();
    forward(input, &mut out, tail.clone(), launch).await;
    out
}

#[tokio::test]
async fn forward_more_than_the_cap_keeps_the_last_lines() {
    let tail = StderrTail::new(None);
    let input = (0..STDERR_TAIL_CAP + 7).map(|n| format!("line {n}")).collect::<Vec<_>>().join("\n");
    forwarded(&tail, input.as_bytes()).await;
    let lines = tail.lines();
    assert_that!(lines.len()).is_equal_to(STDERR_TAIL_CAP);
    assert_that!(lines[0].as_str()).is_equal_to("line 7");
    assert_that!(lines.last().cloned()).is_equal_to(Some(format!("line {}", STDERR_TAIL_CAP + 6)));
}

#[tokio::test]
async fn forward_every_line_reaches_the_writer_unchanged() {
    let tail = StderrTail::new(None);
    let input = b"first\r\nsecond \xff\nlast without newline";
    let out = forwarded(&tail, input).await;
    assert_that!(out.as_slice()).is_equal_to(&input[..]);
    assert_that!(tail.lines()).is_equal_to(vec![
        "first".to_owned(),
        "second \u{fffd}".to_owned(),
        "last without newline".to_owned(),
    ]);
}

#[tokio::test]
async fn begin_new_launch_clears_the_tail() {
    let tail = StderrTail::new(None);
    forwarded(&tail, b"old\n").await;
    tail.begin();
    assert_that!(tail.lines()).is_empty();
}

#[tokio::test]
async fn push_from_an_earlier_launch_is_ignored() {
    let tail = StderrTail::new(None);
    let old = tail.begin();
    tail.begin();
    tail.push(old, "late line of the old agent");
    assert_that!(tail.lines()).is_empty();
}

#[tokio::test]
async fn forward_long_line_is_cut() {
    let tail = StderrTail::new(None);
    let input = format!("{}\n", "x".repeat(LOG_LINE_MAX * 2));
    forwarded(&tail, input.as_bytes()).await;
    let line = tail.lines()[0].clone();
    assert_that!(line.chars().count()).is_equal_to(LOG_LINE_MAX);
    assert_that!(line.ends_with('…')).is_true();
}

#[tokio::test]
async fn forward_home_path_comes_back_with_a_tilde() {
    let tail = StderrTail::new(Some("/Users/ann".into()));
    forwarded(&tail, b"cannot read /Users/ann/.npmrc\n").await;
    assert_that!(tail.lines()).is_equal_to(vec!["cannot read ~/.npmrc".to_owned()]);
}

/// Ten megabytes: far more than any line kept.
const HUGE: usize = 10 * 1024 * 1024;

#[test]
fn feed_line_without_end_keeps_a_bounded_prefix() {
    let mut cut = LineCut::default();
    let chunk = [b'x'; 4096];
    for _ in 0..HUGE / chunk.len() {
        assert_that!(cut.feed(&chunk)).is_empty();
        assert_that!(cut.pending()).is_less_than_or_equal_to(LINE_KEPT);
    }
    let lines = cut.feed(b"\nlater\n");
    assert_that!(lines).is_equal_to(vec!["x".repeat(LINE_KEPT), "later".to_owned()]);
}

#[rstest]
#[case::carriage_return_ends_a_line(&["50%\r60%\r"], &["50%", "60%"])]
#[case::crlf_ends_one_line(&["a\r\nb\n"], &["a", "b"])]
#[case::crlf_split_across_reads(&["a\r", "\nb\n"], &["a", "b"])]
#[case::empty_line_is_kept(&["a\n\nb\n"], &["a", "", "b"])]
#[case::empty_line_after_cr_is_kept(&["a\r\n\nb\n"], &["a", "", "b"])]
fn feed_line_ends_give_lines(#[case] reads: &[&str], #[case] expected: &[&str]) {
    let mut cut = LineCut::default();
    let lines: Vec<String> = reads.iter().flat_map(|r| cut.feed(r.as_bytes())).collect();
    assert_that!(lines).is_equal_to(expected.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>());
}

#[tokio::test]
async fn forward_huge_line_without_newline_is_clipped_and_later_lines_arrive() {
    let tail = StderrTail::new(None);
    let mut input = vec![b'x'; HUGE];
    input.extend_from_slice(b"\nlater line\n");
    let out = forwarded(&tail, &input).await;
    assert_that!(out.len()).is_equal_to(input.len());
    let lines = tail.lines();
    assert_that!(lines.len()).is_equal_to(2);
    assert_that!(lines[0].chars().count()).is_equal_to(LOG_LINE_MAX);
    assert_that!(lines[1].as_str()).is_equal_to("later line");
}
