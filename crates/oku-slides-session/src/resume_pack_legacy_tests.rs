use std::path::Path;

use oku_slides_core::Op;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::library_kind::LibraryKind;
use crate::resume::Resumed;
use crate::test_support::{edit_state, fresh_dir, session_on};
use crate::test_support_packs::{Live, live_editor};
use crate::theme::{PAPIER_THEME, ThemeManifest, Themes};
use crate::theme_closure::pin_of;
use crate::theme_edit::ThemeEdit;
use crate::theme_edit::theme_edit_tests::p;
use crate::{Mode, Session};

/// The draft 6.0.0 started from: the built-in Papier, as a child.
fn papier_draft() -> Value {
    let b = Themes::builtins().iter().find(|b| b.manifest.id == PAPIER_THEME).unwrap();
    let manifest: &ThemeManifest = &b.manifest;
    serde_json::to_value(ThemeEdit::starting(pin_of(manifest, b.version.clone()), manifest)).unwrap()
}

/// What a 6.0.0 working deck held: no theme, the theme draft `tokens` as its tokens.
fn forge_working_deck(s: &mut Session, tokens: &Value) {
    let mut deck = s.doc.sources.deck().clone();
    deck.theme = None;
    deck.themes.clear();
    deck.tokens = tokens.clone();
    s.doc.sources.apply_silent(deck).unwrap();
}

/// `s` written as a 6.0.0 pack tab state of `version` (host `tab`), its draft holding `tokens`, then gone.
fn as_legacy(mut s: Session, dir: &Path, version: u32, tokens: &Value) {
    forge_working_deck(&mut s, tokens);
    s.persist(true);
    s.flush_resume();
    drop(s);
    edit_state(dir, |state| {
        state["version"] = json!(version);
        state["mode"]["pack"]["host"] = json!("tab");
        state["theme_edit"] = papier_draft();
    });
}

/// A new session on `data` resuming `dir`, and what it gave.
fn resumed(data: &Path, dir: &Path) -> (Session, Resumed) {
    let mut s = session_on(data);
    let result = s.resume_checked(dir.to_path_buf(), None);
    (s, result)
}

/// The theme, pins and tokens of a deck.
fn style(deck: &oku_slides_core::Deck) -> (Option<String>, Vec<String>, Value) {
    let pins = deck.themes.values().map(|p| p.version.hex().to_owned()).collect();
    (deck.theme.as_ref().map(ToString::to_string), pins, deck.tokens.clone())
}

fn dropped_notice(s: &Session) -> String {
    s.localizer.tr("notice-theme-draft-dropped")
}

#[test]
fn resume_v4_pack_tab_with_a_changed_theme_draft_notices_its_loss() {
    let data = fresh_dir("oku-slides-legacy-");
    let Live { editor, dir, deck_dir: _deck_dir, .. } = live_editor(data.path());
    as_legacy(editor, dir.path(), 4, &json!({ "color": { "accent": "#123456" } }));
    let (back, result) = resumed(data.path(), dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
    let Mode::Pack { preview, .. } = &back.mode else { panic!("pack mode expected") };
    assert_that!(style(back.deck())).is_equal_to(style(preview.sources.deck()));
    assert_that!(back.snapshot().notice.map(str::to_owned)).is_equal_to(Some(dropped_notice(&back)));
}

#[test]
fn pack_states_are_written_at_version_5_without_theme_edit() {
    let data = fresh_dir("oku-slides-legacy-");
    let Live { mut editor, dir, deck_dir: _deck_dir, .. } = live_editor(data.path());
    editor.persist(true);
    editor.flush_resume();
    let state: Value = serde_json::from_slice(&std::fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_that!(state["version"]).is_equal_to(json!(5));
    assert_that!(state.get("theme_edit")).is_none();
}

#[test]
fn pack_states_are_written_without_host() {
    let data = fresh_dir("oku-slides-legacy-");
    let Live { mut editor, dir, deck_dir: _deck_dir, .. } = live_editor(data.path());
    editor.persist(true);
    editor.flush_resume();
    let state: Value = serde_json::from_slice(&std::fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_that!(state["mode"]["pack"]["pack"]).is_equal_to(json!(editor.editing_view().unwrap().pack));
    assert_that!(state["mode"]["pack"].get("host")).is_none();
}

#[test]
fn resume_v4_pack_tab_with_malformed_draft_tokens_notices_its_loss() {
    let data = fresh_dir("oku-slides-legacy-");
    let Live { editor, dir, deck_dir: _deck_dir, .. } = live_editor(data.path());
    as_legacy(editor, dir.path(), 4, &json!("not an object"));
    let (back, result) = resumed(data.path(), dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
    assert_that!(back.snapshot().notice.map(str::to_owned)).is_equal_to(Some(dropped_notice(&back)));
}

#[test]
fn resume_v4_pack_tab_keeps_its_theme_restyle() {
    let data = fresh_dir("oku-slides-legacy-");
    let Live { editor, dir, deck_dir: _deck_dir, .. } = live_editor(data.path());
    as_legacy(editor, dir.path(), 4, &Value::Null);
    let (back, result) = resumed(data.path(), dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
    let Mode::Pack { preview, .. } = &back.mode else { panic!("pack mode expected") };
    assert_that!(style(back.deck())).is_equal_to(style(preview.sources.deck()));
}

#[test]
fn restyle_legacy_adds_no_undo_step_and_keeps_dirty() {
    let data = fresh_dir("oku-slides-legacy-");
    let mut s = session_on(data.path());
    let ink = Op::SetToken { path: p(&["color", "ink"]), value: json!("#101010") };
    s.apply("ink", vec![ink], None).unwrap();
    let sources = oku_slides_core::SourceBundle::new(s.deck().clone(), s.registry.clone()).unwrap();
    let mut legacy = crate::Document::new(sources, None, &s.doc_defaults);
    let mut working = legacy.sources.deck().clone();
    (working.theme, working.tokens) = (None, json!({ "color": { "accent": "#123456" } }));
    working.themes.clear();
    legacy.sources.apply(working, "draft", None).unwrap();
    legacy.dirty = true;
    let steps = legacy.sources.undo_steps();
    let draft: ThemeEdit = serde_json::from_value(papier_draft()).unwrap();
    assert_that!(s.restyle_legacy(&mut legacy, Some(&draft)).unwrap()).is_true();
    assert_that!(legacy.sources.undo_steps()).is_equal_to(steps);
    assert_that!(legacy.dirty).is_true();
    assert_that!(style(legacy.sources.deck())).is_equal_to(style(s.deck()));
}
