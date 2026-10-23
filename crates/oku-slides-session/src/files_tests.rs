use oku_slides_core::{Deck, Issue, SourceBundle};
use oku_slides_i18n::{Localizer, Message};
use speculoos::prelude::*;

use super::*;

fn registry() -> std::sync::Arc<oku_slides_core::WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    std::sync::Arc::new(oku_slides_core::WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap())
}

fn unreadable() -> Issue {
    Issue { slide: Some("s-1".into()), element: None, message: Message::new("issue-unreadable-slide") }
}

/// Un constat de `validate` : rien n'a été perdu à la lecture.
fn finding() -> Issue {
    Issue { slide: Some("s-1".into()), element: Some("e-1".into()), message: Message::new("issue-empty-frame") }
}

fn opened(modified_elsewhere: bool, issues: usize) -> Opened {
    with_issues(modified_elsewhere, vec![unreadable(); issues])
}

fn with_issues(modified_elsewhere: bool, issues: Vec<Issue>) -> Opened {
    let deck = Deck::from_json(r#"{"version": 9,"slides":[]}"#).unwrap();
    Opened {
        sources: SourceBundle::new(deck, registry()).unwrap(),
        issues,
        pdf: true,
        modified_elsewhere,
        hash: "sha256:00".into(),
    }
}

#[test]
fn notice_without_issues_nor_change_elsewhere_is_none() {
    assert_that!(opened(false, 0).notice(&Localizer::new(Some("fr")))).is_none();
}

#[test]
fn notice_with_unreadable_parts_counts_them() {
    let notice = opened(false, 3).notice(&Localizer::new(Some("fr"))).unwrap();
    assert_that!(notice).is_equal_to(
        "Des parties de cette présentation n'ont pas pu être lues et ont été mises de côté (3).".to_owned(),
    );
}

#[test]
fn notice_modified_elsewhere_with_unreadable_parts_joins_both_lines() {
    let l = Localizer::new(Some("fr"));
    let notice = opened(true, 1).notice(&l).unwrap();
    let lines: Vec<&str> = notice.lines().collect();
    assert_that!(lines.len()).is_equal_to(2);
    assert_that!(lines[0].to_owned()).is_equal_to(l.tr("notice-modified-elsewhere"));
    assert_that!(lines[1]).ends_with("(1).");
}

#[test]
fn notice_with_validation_findings_only_is_none() {
    assert_that!(with_issues(false, vec![finding(), finding()]).notice(&Localizer::new(Some("fr")))).is_none();
}

#[test]
fn notice_with_findings_and_unreadable_parts_counts_the_unreadable_only() {
    let notice = with_issues(false, vec![finding(), unreadable(), finding()]).notice(&Localizer::new(Some("fr")));
    assert_that!(notice.unwrap()).ends_with("(1).");
}

#[test]
fn is_unreadable_tells_read_losses_from_validation_findings() {
    assert_that!(is_unreadable(&unreadable())).is_true();
    assert_that!(is_unreadable(&finding())).is_false();
}

#[test]
fn log_issues_counts_the_unreadable_parts_only() {
    let issues = [finding(), unreadable(), unreadable(), finding()];
    let l = Localizer::new(Some("fr"));
    for at in [IssuesAt::Open, IssuesAt::Merge, IssuesAt::Restore] {
        assert_that!(log_issues(&issues, &l, at)).is_equal_to(2);
    }
}

#[test]
fn deck_uid_of_a_json_deck_is_none() {
    let dir = crate::test_support::fresh_dir("oku-slides-files-").keep();
    let path = dir.join("deck.json");
    std::fs::write(&path, r#"{"version": 9,"slides":[]}"#).unwrap();
    let limits = SourceLimits { replica_bytes: 1 << 20, total_bytes: 1 << 20, compact_after_secs: 0 };
    let uid = deck_uid_of(&Storage::local().unwrap(), &path, limits, &registry());
    std::fs::remove_dir_all(&dir).ok();
    assert_that!(uid.unwrap()).is_none();
}

/// La relecture du replica écarte les slots trop profonds : des éléments
/// ont été perdus, comme pour une partie illisible.
fn too_deep() -> Issue {
    Issue { slide: None, element: None, message: Message::new("issue-slot-too-deep") }
}

#[test]
fn is_unreadable_counts_a_slot_too_deep_as_a_read_loss() {
    assert_that!(is_unreadable(&too_deep())).is_true();
    let l = Localizer::new(Some("fr"));
    assert_that!(log_issues(&[finding(), too_deep(), unreadable()], &l, IssuesAt::Merge)).is_equal_to(2);
}

#[test]
fn notice_with_a_slot_too_deep_counts_it_as_an_unreadable_part() {
    let notice = with_issues(false, vec![finding(), too_deep()]).notice(&Localizer::new(Some("fr")));
    assert_that!(notice.unwrap()).ends_with("(1).");
}

#[test]
fn is_unreadable_counts_a_discarded_pack_version_but_not_a_missing_one() {
    let version = oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap();
    assert_that!(is_unreadable(&oku_slides_core::Issue::pack_version_mismatch(version.hex()))).is_true();
    assert_that!(is_unreadable(&oku_slides_core::Issue::pack_version_missing(&version))).is_false();
}

#[test]
fn is_unreadable_counts_a_discarded_theme_version_but_not_a_missing_one() {
    let version = oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap();
    assert_that!(is_unreadable(&oku_slides_core::Issue::theme_version_mismatch(version.hex()))).is_true();
    assert_that!(is_unreadable(&oku_slides_core::Issue::theme_version_missing(&version))).is_false();
}
