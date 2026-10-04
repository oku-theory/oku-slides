use std::sync::{Arc, Mutex};

use serde_json::json;
use speculoos::prelude::*;

use oku_slides_core::Op;

use super::*;
use crate::agent::TabSession;
use crate::lib_tests_support::{data_dir, session_in};
use crate::{Request, SessionErrorKind};

fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap()
}

fn help_session(locale: &str) -> (crate::lib_tests_support::Data, Session) {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help(locale).unwrap();
    (data, s)
}

#[test]
fn help_deck_in_every_shipped_language_parses_and_validates() {
    for locale in help_locales() {
        let deck = help_deck(locale).unwrap();
        let issues = oku_slides_core::validate(&deck, &crate::test_support::registry(), None);
        assert_that!(issues).named(locale).is_empty();
    }
}

#[test]
fn help_locale_of_an_unshipped_language_falls_back_to_english() {
    assert_that!(help_locale("de")).is_equal_to("en");
}

#[test]
fn help_locale_of_a_shipped_language_keeps_it() {
    assert_that!(help_locale("fr")).is_equal_to("fr");
}

#[test]
fn help_decks_in_every_language_have_the_same_slides() {
    let ids = |l: &str| help_deck(l).unwrap().slides.into_iter().map(|s| s.id).collect::<Vec<_>>();
    for locale in help_locales() {
        assert_that!(ids(locale)).named(locale).is_equal_to(ids("en"));
    }
}

#[test]
fn open_help_shows_a_read_only_deck_without_file() {
    let (_data, s) = help_session("fr");
    let snap = s.snapshot();
    assert_that!(snap.read_only).is_true();
    assert_that!(snap.path).is_none();
    assert_that!(snap.slides.len()).is_greater_than(1);
    assert_that!(s.tab_info().help).is_equal_to(Some("fr".to_owned()));
}

#[test]
fn help_handle_apply_is_refused_as_read_only() {
    let (_data, mut s) = help_session("en");
    let deck = s.deck().clone();
    let apply = req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": "pro"}]}));
    let err = s.handle(apply).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
    assert_that!(s.deck()).is_equal_to(&deck);
}

#[test]
fn help_handle_history_open_is_refused_as_read_only() {
    let (_data, mut s) = help_session("en");
    let err = s.handle(req(json!({"cmd": "history_open"}))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_handle_ui_moves_the_current_slide() {
    let (_data, mut s) = help_session("fr");
    let second = s.snapshot().slides[1].id.to_owned();
    s.handle(req(json!({"cmd": "ui", "slide": second}))).unwrap();
    assert_that!(s.snapshot().ui.slide.as_deref()).is_equal_to(Some(second.as_str()));
}

/// L'agent appelle les méthodes directement : chacune se garde.
#[test]
fn help_agent_apply_and_new_deck_are_refused_as_read_only() {
    let (_data, s) = help_session("fr");
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(s)));
    let op = Op::SetTheme { theme: Some("pro".into()) };
    let err = tab.with(|s| s.apply("Agent", vec![op], None)).unwrap().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
    let err = tab.with(Session::new_deck).unwrap().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_tab_resumes_as_help_in_the_same_language() {
    let data = data_dir();
    let dir = data.0.join("tabs").join("t-1");
    {
        let mut s = session_in(&data.0);
        s.open_help("en").unwrap();
        assert_that!(s.resume_fresh(dir.clone())).is_true();
        s.handle(req(json!({"cmd": "ui", "slide": "s-sommaire"}))).unwrap();
        s.flush_resume();
    }
    let mut s = session_in(&data.0);
    assert_that!(s.resume(dir)).is_true();
    assert_that!(s.snapshot().read_only).is_true();
    assert_that!(s.tab_info().help).is_equal_to(Some("en".to_owned()));
    assert_that!(s.snapshot().ui.slide.as_deref()).is_equal_to(Some("s-sommaire"));
}

#[test]
fn help_tab_is_never_dirty_nor_pristine() {
    let (_data, s) = help_session("fr");
    let info = s.tab_info();
    assert_that!(info.dirty).is_false();
    assert_that!(info.pristine).is_false();
}

/// Chaque méthode qui modifie se garde elle-même, avant tout changement :
/// l'agent les appelle sans passer par `handle`.
#[test]
fn help_agent_set_widget_is_refused_as_read_only() {
    let (_data, mut s) = help_session("fr");
    let err = s.set_widget_by_agent("kpi", "#let widget = none".into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_direct_merge_file_is_refused_as_read_only() {
    let (data, mut s) = help_session("fr");
    let err = s.merge_file(&data.0.join("copie.oku-slides.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_direct_history_open_is_refused_as_read_only() {
    let (_data, mut s) = help_session("fr");
    let err = s.history_open().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_direct_edit_templates_is_refused_as_read_only() {
    let (_data, mut s) = help_session("fr");
    let err = s.edit_templates(crate::DEFAULT_PACK).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReadOnly);
}

#[test]
fn help_handle_undo_redo_and_pack_make_current_are_refused_as_read_only() {
    let (_data, mut s) = help_session("en");
    for cmd in ["undo", "redo", "pack_make_current"] {
        let err = s.handle(req(json!({ "cmd": cmd }))).unwrap_err();
        assert_that!(err.kind()).named(cmd).is_equal_to(SessionErrorKind::ReadOnly);
    }
}

/// Le pack par défaut modifié par l'utilisateur ne change pas l'aide : elle
/// se rend avec la version livrée avec l'app.
#[test]
fn open_help_uses_the_builtin_default_pack_not_the_users_current_one() {
    use crate::test_support::{add_version, library_of, manifest};

    let data = data_dir();
    let mut s = session_in(&data.0);
    let builtin = crate::builtin_versions()[0].version.clone();
    let mine = manifest(crate::DEFAULT_PACK, "Mon Essentiel", Some(&builtin), "2027-01-01T00:00:00Z");
    let r = add_version(library_of(&s), &mine, &[("title", "#let widget = none")]);
    library_of(&s).set_current(crate::DEFAULT_PACK, &r.version).unwrap();
    s.open_help("fr").unwrap();
    let used = s.deck().templates.clone().unwrap();
    assert_that!(used.version).is_equal_to(builtin);
}
