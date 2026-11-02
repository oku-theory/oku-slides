use oku_slides_core::{GuideKind, Guides, PackManifest};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, session_on};
use crate::test_support_packs::pack_tab_of;
use crate::{Request, Session, SessionErrorKind};

/// A pack tab on the built-in Essentiel pack, on a fresh folder.
fn essentiel_pack_tab() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-guides-");
    let s = pack_tab_of(data.path(), ESSENTIEL_PACK);
    (data, s)
}

/// The guides of the session's version of the edited pack.
fn edited_guides(s: &Session) -> Guides {
    let t = s.widget_target(None).unwrap();
    PackManifest::from_files(&t.files).unwrap().guides
}

#[test]
fn set_guide_in_pack_tab_stores_it_in_the_session_version() {
    let (_d, mut s) = essentiel_pack_tab();
    s.set_guide(GuideKind::Use, "Use the title layout first.".into()).unwrap();
    assert_that!(edited_guides(&s).use_.as_str()).is_equal_to("Use the title layout first.");
}

#[test]
fn set_guide_then_undo_brings_the_guide_back() {
    let (_d, mut s) = essentiel_pack_tab();
    let before = edited_guides(&s);
    s.set_guide(GuideKind::Edit, "Keep slots named.".into()).unwrap();
    s.handle(Request::Undo).unwrap();
    assert_that!(edited_guides(&s)).is_equal_to(before);
}

#[test]
fn set_guide_same_text_makes_no_version() {
    let (_d, mut s) = essentiel_pack_tab();
    s.set_guide(GuideKind::Use, "a".into()).unwrap();
    let steps = s.doc.sources.undo_steps();
    s.set_guide(GuideKind::Use, "a".into()).unwrap();
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps);
}

#[test]
fn set_guide_with_empty_text_clears_it() {
    let (_d, mut s) = essentiel_pack_tab();
    s.set_guide(GuideKind::Use, "a".into()).unwrap();
    s.set_guide(GuideKind::Use, String::new()).unwrap();
    // Only the use guide goes: Essentiel's own edit guide stays.
    let shipped_edit = crate::test_support::builtin(ESSENTIEL_PACK).manifest.guides.edit.clone();
    assert_that!((edited_guides(&s).use_, edited_guides(&s).edit)).is_equal_to((String::new(), shipped_edit));
}

#[rstest]
#[case::too_long("x".repeat(4001))]
#[case::nul("a\0b".to_owned())]
fn set_guide_bad_text_is_refused(#[case] text: String) {
    let (_d, mut s) = essentiel_pack_tab();
    let e = s.set_guide(GuideKind::Use, text).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::InvalidGuide);
}

#[test]
fn set_guide_at_the_limit_is_accepted() {
    let (_d, mut s) = essentiel_pack_tab();
    assert_that!(s.set_guide(GuideKind::Use, "é".repeat(4000)).is_ok()).is_true();
}

#[test]
fn set_guide_in_deck_tab_is_wrong_document() {
    let data = fresh_dir("oku-slides-guides-");
    let mut s = session_on(data.path());
    let e = s.set_guide(GuideKind::Use, "a".into()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

#[test]
fn request_set_guide_in_deck_tab_is_wrong_document() {
    let data = fresh_dir("oku-slides-guides-");
    let mut s = session_on(data.path());
    let e = s.handle(Request::SetGuide { which: GuideKind::Use, text: "a".into() }).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

#[test]
fn request_set_guide_tells_the_agent() {
    let (_d, mut s) = essentiel_pack_tab();
    s.handle(Request::SetGuide { which: GuideKind::Use, text: "a".into() }).unwrap();
    let context = s.agent_context(&crate::agent::AgentUi::default());
    assert_that!(context.as_str()).contains("user edited the use guide");
}

#[test]
fn snapshot_in_pack_tab_holds_the_guides_and_their_limit() {
    let (_d, mut s) = essentiel_pack_tab();
    s.set_guide(GuideKind::Edit, "e".into()).unwrap();
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.guides.edit, view.max_chars)).is_equal_to(("e".to_owned(), 4000));
}

#[test]
fn snapshot_in_deck_tab_holds_no_guides() {
    let data = fresh_dir("oku-slides-guides-");
    let s = session_on(data.path());
    assert_that!(s.snapshot().guides.is_none()).is_true();
}

#[test]
fn guide_too_long_error_is_localized_in_french() {
    let (_d, mut s) = essentiel_pack_tab();
    let e = s.set_guide(GuideKind::Use, "x".repeat(4001)).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("fr"))).as_str()).contains("4000");
}

#[test]
fn guide_with_nul_error_is_localized_in_french() {
    let (_d, mut s) = essentiel_pack_tab();
    let e = s.set_guide(GuideKind::Use, "a\0".into()).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("fr"))).as_str()).contains("caractère nul");
}

#[test]
fn guide_write_fact_in_pack_tab_names_the_pack_the_guide_and_its_length() {
    let (_d, mut s) = essentiel_pack_tab();
    s.set_guide(GuideKind::Edit, "Keep the grid.".into()).unwrap();
    let fact = s.guide_write_fact(GuideKind::Edit, 14);
    assert_that!((fact.contains("Essentiel"), fact.contains("14"), fact.contains("édition")))
        .is_equal_to((true, true, true));
}

#[test]
fn guide_write_fact_in_theme_tab_names_the_draft() {
    let data = fresh_dir("oku-slides-guides-");
    let mut s = session_on(data.path());
    s.open_theme_here(crate::PAPIER_THEME).unwrap();
    let name = s.edited_theme_name().unwrap();
    assert_that!(s.guide_write_fact(GuideKind::Use, 3)).contains(name.as_str());
}
