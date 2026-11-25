use std::collections::HashSet;

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;
use crate::validate::PEER_TEXT_MAX_CHARS;

const CAUSES: [DropCause; 4] = [DropCause::Files, DropCause::FileBytes, DropCause::TotalBytes, DropCause::Unreadable];

#[test]
fn knowledge_issues_bound_peer_strings() {
    let hostile = format!("\u{202E}{}", "x".repeat(10_000));
    let shown = Issue::knowledge_unreadable(&hostile).localize(oku_slides_i18n::english());
    assert_that!(shown.contains('\u{202E}')).is_false();
    assert_that!(shown.chars().count()).is_less_than(PEER_TEXT_MAX_CHARS + 100);
}

#[test]
fn knowledge_over_limit_is_localized_per_limit() {
    let fr = Localizer::new(Some("fr"));
    let shown = |limit: KnowledgeLimit| Issue::knowledge_over_limit(limit, "ab12", 3).localize(&fr);
    let many = shown(KnowledgeLimit::TooManyFiles);
    assert_that!(many.contains("trop de fichiers")).is_true();
    assert_that!(many.contains("ab12")).is_true();
    assert_that!(many.contains('3')).is_true();
    assert_that!(shown(KnowledgeLimit::FileTooLarge)).is_not_equal_to(shown(KnowledgeLimit::TotalTooLarge));
}

#[test]
fn knowledge_over_limit_names_each_limit_in_every_locale() {
    let limits = [KnowledgeLimit::TooManyFiles, KnowledgeLimit::FileTooLarge, KnowledgeLimit::TotalTooLarge];
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        let said: HashSet<String> =
            limits.iter().map(|&limit| Issue::knowledge_over_limit(limit, "ab12", 3).localize(&l)).collect();
        assert_that!(said.len()).named(locale).is_equal_to(limits.len());
        for limit in limits {
            let reason = crate::error::KnowledgeReason::from(limit).message(&l);
            let shown = Issue::knowledge_over_limit(limit, "ab12", 3).localize(&l);
            assert_that!(shown.contains(&reason)).named(locale).is_true();
        }
    }
}

#[test]
fn knowledge_entries_dropped_carries_its_key_cause_and_count() {
    let issue = Issue::knowledge_entries_dropped(DropCause::FileBytes, 7);
    assert_that!(issue.message.id()).is_equal_to("issue-knowledge-entries-dropped");
    let keys: Vec<&str> = CAUSES.iter().map(|c| c.key()).collect();
    assert_that!(keys).is_equal_to(vec!["files", "file-bytes", "total-bytes", "unreadable"]);
}

#[test]
fn knowledge_entries_dropped_is_localized_per_cause_in_english_and_french() {
    let fr = Localizer::new(Some("fr"));
    for l in [oku_slides_i18n::english(), &fr] {
        let said: HashSet<String> =
            CAUSES.iter().map(|&c| Issue::knowledge_entries_dropped(c, 4099).localize(l)).collect();
        assert_that!(said.len()).is_equal_to(CAUSES.len());
        assert_that!(said.iter().all(|s| s.contains("4099"))).is_true();
    }
    let shown = |l: &Localizer| Issue::knowledge_entries_dropped(DropCause::Files, 3).localize(l);
    assert_that!(shown(&fr)).is_not_equal_to(shown(oku_slides_i18n::english()));
}

#[test]
fn knowledge_entries_dropped_names_each_cause_in_every_locale() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        let said: HashSet<String> =
            CAUSES.iter().map(|&c| Issue::knowledge_entries_dropped(c, 3).localize(&l)).collect();
        assert_that!(said.len()).named(locale).is_equal_to(CAUSES.len());
    }
}
