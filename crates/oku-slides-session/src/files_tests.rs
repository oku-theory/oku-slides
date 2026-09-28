use oku_slides_core::{Deck, Issue, SourceBundle};
use oku_slides_i18n::{Localizer, Message};
use speculoos::prelude::*;

use super::*;

fn opened(modified_elsewhere: bool, issues: usize) -> Opened {
    let deck = Deck::from_json(r#"{"slides":[]}"#).unwrap();
    let issue = Issue { slide: Some("s-1".into()), element: None, message: Message::new("issue-unreadable-slide") };
    Opened {
        sources: SourceBundle::new(deck).unwrap(),
        issues: vec![issue; issues],
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
fn deck_uid_of_a_json_deck_is_none() {
    let dir = crate::test_support::fresh_dir("oku-slides-files-").keep();
    let path = dir.join("deck.json");
    std::fs::write(&path, r#"{"slides":[]}"#).unwrap();
    let limits = SourceLimits { replica_bytes: 1 << 20, total_bytes: 1 << 20 };
    let uid = deck_uid_of(&Storage::local().unwrap(), &path, limits);
    std::fs::remove_dir_all(&dir).ok();
    assert_that!(uid.unwrap()).is_none();
}
