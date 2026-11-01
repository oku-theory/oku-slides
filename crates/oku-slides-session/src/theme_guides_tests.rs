use oku_slides_core::GuideKind;
use speculoos::prelude::*;

use crate::test_support::fresh_dir;
use crate::theme::{PAPIER_THEME, ThemeManifest};
use crate::theme_tab::theme_tab_tests::{draft_of, theme_tab};
use crate::{Request, Session};

/// A theme tab on the built-in Papier, with the data dir it lives in.
fn papier_theme_tab() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-theme-guides-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    (data, s)
}

#[test]
fn set_guide_in_theme_tab_changes_the_draft() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Use, "Accent for one word per slide.".into()).unwrap();
    assert_that!(draft_of(&s).guides.use_.as_str()).is_equal_to("Accent for one word per slide.");
}

#[test]
fn theme_guide_undo_and_redo_follow_the_switch() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Edit, "Keep contrast AA.".into()).unwrap();
    s.handle(Request::Undo).unwrap();
    let undone = draft_of(&s).guides.edit;
    s.handle(Request::Redo).unwrap();
    assert_that!((undone.as_str(), draft_of(&s).guides.edit.as_str())).is_equal_to(("", "Keep contrast AA."));
}

#[test]
fn saved_theme_keeps_the_draft_guides() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Use, "u".into()).unwrap();
    let pin = s.save_theme(false).unwrap().unwrap();
    let files = s.themes.version(pin.id, &pin.version).unwrap();
    let saved = ThemeManifest::from_files(&files).unwrap();
    assert_that!(saved.guides.use_.as_str()).is_equal_to("u");
}

#[test]
fn guide_only_change_makes_the_draft_differ() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Use, "u".into()).unwrap();
    assert_that!(s.snapshot().theming.map(|t| t.theme.dirty)).is_equal_to(Some(true));
}

#[test]
fn snapshot_in_theme_tab_holds_the_draft_guides() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Edit, "e".into()).unwrap();
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.guides.edit.as_str(), view.max_chars)).is_equal_to(("e", s.guide_limit));
}

#[test]
fn set_guide_to_the_same_text_writes_no_step() {
    let (_d, mut s) = papier_theme_tab();
    s.set_guide(GuideKind::Use, String::new()).unwrap();
    assert_that!(s.snapshot().theming.map(|t| t.theme.dirty)).is_equal_to(Some(false));
}

/// A child of a built-in starts without its guides: the built-in sends its own as the parent.
#[test]
fn theme_tab_on_a_builtin_starts_with_empty_guides() {
    let (_d, s) = papier_theme_tab();
    assert_that!(draft_of(&s).guides.is_empty()).is_true();
}

/// R17: a theme tab on Papier sends Papier's edit guide, then its use guide.
#[test]
fn agent_guides_theme_tab_on_papier_sends_its_edit_guide_then_its_use_guide() {
    let (_d, mut s) = papier_theme_tab();
    let block = s.agent_guides(true).map(|g| g.block).unwrap_or_default();
    let head = "<guide for=\"edit\" kind=\"theme\" name=\"Papier\" ns=\"papier\" origin=\"builtin\">\n# Editing Papier";
    let edit = block.find(head);
    let use_ = block.find("<guide for=\"use\" kind=\"theme\" name=\"Papier\"");
    assert_that!(edit.is_some() && use_.is_some() && edit < use_).is_true();
}

/// A theme tab on a theme the library only received, with guides `edit` and `use_`, never approved.
fn received_theme_tab(edit: &str, use_: &str) -> (tempfile::TempDir, Session) {
    use crate::library_kind::LibraryKind;
    let data = fresh_dir("oku-slides-theme-guides-");
    let mut s = crate::test_support::session_on(data.path());
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == crate::theme::PRO_THEME).unwrap();
    let id = uuid::Uuid::new_v4();
    let manifest = ThemeManifest {
        id,
        name: "Brought".into(),
        namespace: crate::test_support::ns("brought"),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: pro.manifest.tokens.clone(),
        guides: oku_slides_core::Guides { edit: edit.into(), use_: use_.into() },
    };
    s.themes.receive_version(id, &manifest.to_files().unwrap()).unwrap();
    s.open_theme_here(id).unwrap();
    (data, s)
}

/// I1: E1 awaits approval and is never sent, before or after the tab rewrites it to E2.
#[test]
fn agent_guides_theme_tab_rewriting_a_received_edit_guide_sends_the_new_text_only() {
    let (_d, mut s) = received_theme_tab("hostile E1", "");
    let before = s.agent_guides(true).map(|g| g.block).unwrap_or_default();
    s.set_guide(GuideKind::Edit, "mine E2".into()).unwrap();
    let after = s.agent_guides(false).map(|g| g.block).unwrap_or_default();
    assert_that!(before.contains("status=\"awaiting-approval\"/>") && !before.contains("hostile E1")).is_true();
    assert_that!(after.as_str()).contains("name=\"Brought\" ns=\"brought\" origin=\"local\">\nmine E2\n</guide>");
    assert_that!(after.contains("hostile E1")).is_false();
}

/// I1: the untouched draft shows the received text beside Approve.
#[test]
fn guides_view_theme_tab_untouched_received_draft_offers_approve_beside_its_text() {
    let (_d, s) = received_theme_tab("hostile E1", "");
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.approvable, view.guides.edit.as_str())).is_equal_to((true, "hostile E1"));
}

/// I1: once the draft differs from its base, Approve would approve text not shown: not offered.
#[test]
fn guides_view_theme_tab_rewritten_draft_is_local_and_offers_no_approve() {
    let (_d, mut s) = received_theme_tab("hostile E1", "");
    s.set_guide(GuideKind::Edit, "mine E2".into()).unwrap();
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.origin, view.approvable)).is_equal_to((crate::GuideOrigin::Local, false));
}

/// A guide kept unchanged from the received base still awaits, but Approve stays off while the draft differs.
#[test]
fn guides_view_theme_tab_rewritten_edit_with_kept_received_use_awaits_without_approve() {
    let (_d, mut s) = received_theme_tab("hostile E1", "hostile U1");
    s.set_guide(GuideKind::Edit, "mine E2".into()).unwrap();
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.origin, view.approved, view.approvable)).is_equal_to((
        crate::GuideOrigin::Received,
        false,
        false,
    ));
}

/// Undoing the rewrite gives the base's text back, and with it the approval.
#[test]
fn guides_view_theme_tab_rewrite_undone_offers_approve_again() {
    let (_d, mut s) = received_theme_tab("hostile E1", "");
    s.set_guide(GuideKind::Edit, "mine E2".into()).unwrap();
    s.handle(Request::Undo).unwrap();
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.approvable, view.guides.edit.as_str())).is_equal_to((true, "hostile E1"));
}
