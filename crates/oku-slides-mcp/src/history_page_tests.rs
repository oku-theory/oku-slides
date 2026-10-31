use oku_slides_core::Version;
use rstest::rstest;
use speculoos::prelude::*;

use super::{DEFAULT_COUNT, MAX_COUNT, MAX_MILESTONES, history_page};

/// `n` plain versions; those whose rank is in `milestones` start a session.
fn versions(n: usize, milestones: &[usize]) -> Vec<Version> {
    (0..n)
        .map(|i| Version {
            author: None,
            timestamp: i64::try_from(i).unwrap_or_default(),
            label: Some(format!("v{i}")),
            session_start: milestones.contains(&i),
            merge: false,
        })
        .collect()
}

fn indices(page: &[super::Ranked<'_>]) -> Vec<usize> {
    page.iter().map(|r| r.index).collect()
}

#[test]
fn history_page_by_default_returns_the_latest_versions() {
    let all = versions(50, &[]);
    let page = history_page(&all, None, None);
    assert_that!(page.total).is_equal_to(50);
    assert_that!(indices(&page.versions)).is_equal_to((50 - DEFAULT_COUNT..50).collect::<Vec<_>>());
    assert_that!(page.next_before).is_equal_to(Some(50 - DEFAULT_COUNT));
}

#[test]
fn history_page_before_and_count_return_the_older_page() {
    let all = versions(50, &[]);
    let page = history_page(&all, Some(10), Some(4));
    assert_that!(indices(&page.versions)).is_equal_to(vec![6, 7, 8, 9]);
    assert_that!(page.next_before).is_equal_to(Some(6));
}

#[test]
fn history_page_reaching_the_oldest_has_no_next_before() {
    let all = versions(5, &[]);
    let page = history_page(&all, Some(3), Some(10));
    assert_that!(indices(&page.versions)).is_equal_to(vec![0, 1, 2]);
    assert_that!(page.next_before).is_none();
}

#[rstest]
#[case::zero(0, 1)]
#[case::huge(10_000, MAX_COUNT)]
fn history_page_count_is_clamped(#[case] asked: usize, #[case] len: usize) {
    let all = versions(500, &[]);
    assert_that!(history_page(&all, None, Some(asked)).versions).has_length(len);
}

#[test]
fn history_page_before_past_the_end_means_the_end() {
    let all = versions(3, &[]);
    assert_that!(indices(&history_page(&all, Some(99), None).versions)).is_equal_to(vec![0, 1, 2]);
}

#[test]
fn history_page_lists_milestones_older_than_the_page_only() {
    let mut all = versions(40, &[0, 5, 35]);
    all[12].merge = true;
    let page = history_page(&all, None, Some(10));
    assert_that!(indices(&page.milestones)).is_equal_to(vec![0, 5, 12]);
}

#[test]
fn history_page_keeps_the_most_recent_milestones() {
    let starts: Vec<usize> = (0..60).collect();
    let all = versions(70, &starts);
    let page = history_page(&all, None, Some(10));
    assert_that!(indices(&page.milestones)).is_equal_to((60 - MAX_MILESTONES..60).collect::<Vec<_>>());
}

#[test]
fn history_page_serializes_each_version_with_its_index() {
    let all = versions(2, &[0]);
    let json = serde_json::to_value(history_page(&all, None, None)).unwrap();
    assert_that!(json["versions"][1]["index"].as_u64()).is_equal_to(Some(1));
    assert_that!(json["versions"][1]["label"].as_str()).is_equal_to(Some("v1"));
    assert_that!(json.get("next_before")).is_none();
}
