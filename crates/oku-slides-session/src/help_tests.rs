use std::sync::{Arc, Mutex};

use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::agent::TabSession;
use crate::lib_tests_support::{data_dir, session_in};
use crate::search::SEARCH_EXCERPT;
use crate::{Request, SessionErrorKind};

fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap()
}

fn help_session(locale: &str) -> (crate::lib_tests_support::Data, Session) {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help(locale, None).unwrap();
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
    assert_that!(s.tab_info().document.help_locale()).is_equal_to(Some("fr"));
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
    let op = crate::test_support::set_theme_op("pro");
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
        s.open_help("en", Some(crate::PRO_THEME)).unwrap();
        assert_that!(s.resume_fresh(dir.clone())).is_true();
        s.handle(req(json!({"cmd": "ui", "slide": "s-sommaire"}))).unwrap();
        s.flush_resume();
    }
    let mut s = session_in(&data.0);
    assert_that!(s.resume(dir)).is_true();
    assert_that!(s.snapshot().read_only).is_true();
    assert_that!(s.tab_info().document.help_locale()).is_equal_to(Some("en"));
    assert_that!(s.snapshot().ui.slide.as_deref()).is_equal_to(Some("s-sommaire"));
    assert_that!(s.snapshot().theme.and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
}

/// Le format de reprise de l'aide ne change pas (t-489) : un `state.json` écrit
/// avant `Mode::Help` se relit, et se réécrit sous la même forme.
#[test]
fn help_resume_file_written_before_mode_help_still_loads_and_keeps_its_form() {
    let data = data_dir();
    let dir = data.0.join("tabs").join("t-1");
    std::fs::create_dir_all(&dir).unwrap();
    let state = json!({
        "version": crate::resume::VERSION,
        "mode": {"help": "fr"},
        "ui": {"slide": "s-sommaire", "grid": false},
        "help_theme": crate::PRO_THEME,
    });
    std::fs::write(dir.join("state.json"), state.to_string()).unwrap();
    let mut s = session_in(&data.0);
    assert_that!(s.resume(dir.clone())).is_true();
    assert_that!(s.snapshot().read_only).is_true();
    assert_that!(s.tab_info().document.help_locale()).is_equal_to(Some("fr"));
    assert_that!(s.snapshot().theme.and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
    s.flush_resume();
    let written: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap();
    assert_that!(written["mode"]).is_equal_to(json!({"help": "fr"}));
    assert_that!(written["help_theme"]).is_equal_to(json!(crate::PRO_THEME));
}

/// Help is a mode (t-489): opening it while editing a pack is refused, and
/// the pack tab's preview deck stays the opener's.
#[test]
fn open_help_while_editing_pack_is_refused_and_keeps_the_preview_deck() {
    let data = data_dir();
    let mut opener = session_in(&data.0);
    let mut s = crate::test_support_packs::pack_tab_from(&mut opener, &data.0, crate::ESSENTIEL_PACK);
    let err = s.open_help("fr", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingPack);
    assert_that!(s.snapshot().read_only).is_false();
    assert_that!(s.editing_view().is_some()).is_true();
    assert_that!(s.tab_info().document.help_locale()).is_none();
    assert_that!(crate::test_support::preview_of(&s).sources.deck()).is_equal_to(opener.deck());
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
    let err = s.set_widget_by_agent(None, "kpi", "#let widget = none".into()).unwrap_err();
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
fn help_handle_undo_and_redo_are_refused_as_read_only() {
    let (_data, mut s) = help_session("en");
    for cmd in ["undo", "redo"] {
        let err = s.handle(req(json!({ "cmd": cmd }))).unwrap_err();
        assert_that!(err.kind()).named(cmd).is_equal_to(SessionErrorKind::ReadOnly);
    }
}

#[test]
fn help_deck_packs_is_refused_as_read_only() {
    let (_data, s) = help_session("en");
    assert_that!(s.deck_packs().map_err(|e| e.kind()).err()).is_equal_to(Some(SessionErrorKind::ReadOnly));
}

/// Le pack par défaut modifié par l'utilisateur ne change pas l'aide : elle
/// se rend avec la version livrée avec l'app.
#[test]
fn open_help_uses_the_builtin_default_pack_not_the_users_current_one() {
    use crate::test_support::{add_version_of, library_of, manifest};

    let data = data_dir();
    let mut s = session_in(&data.0);
    let builtin = crate::test_support::builtin(crate::pack::ESSENTIEL_PACK).version.clone();
    let mine = manifest(crate::ESSENTIEL_PACK, "Mon Essentiel", Some(&builtin), "2027-01-01T00:00:00Z");
    let r =
        add_version_of(library_of(&s), oku_slides_core::PackKind::Layouts, &mine, &[("title", "#let widget = none")]);
    library_of(&s).set_current(crate::ESSENTIEL_PACK, &r.version).unwrap();
    s.open_help("fr", None).unwrap();
    let used = crate::test_support::pin(&s, "essentiel").unwrap();
    assert_that!(used.version).is_equal_to(builtin);
}

#[test]
fn help_deck_for_windows_and_linux_writes_ctrl_not_command() {
    for locale in help_locales() {
        let deck = help_deck_with(locale, oku_slides_i18n::pc_keys).unwrap();
        let json = serde_json::to_string(&deck).unwrap();
        assert_that!(json.contains(['⌘', '⌥', '⇧', '⌃'])).named(locale).is_false();
        assert_that!(json.contains("Ctrl+Shift+S")).named(locale).is_true();
    }
}

#[test]
fn help_deck_list_shortcuts_read_ctrl_alt_on_windows_and_linux_and_symbols_on_macos() {
    for locale in help_locales() {
        let pc = serde_json::to_string(&help_deck_with(locale, oku_slides_i18n::pc_keys).unwrap()).unwrap();
        assert_that!(pc.contains("\"Ctrl+Alt+←↑→↓\"")).named(locale).is_true();
        assert_that!(pc.contains("(Alt+↑/↓ ")).named(locale).is_true();
        let mac = serde_json::to_string(&help_deck_with(locale, |s| std::borrow::Cow::Borrowed(s)).unwrap()).unwrap();
        assert_that!(mac.contains("\"⌘⌥←↑→↓\"")).named(locale).is_true();
        assert_that!(mac.contains("(⌥↑/↓ ")).named(locale).is_true();
    }
}

#[test]
fn help_deck_for_macos_keeps_the_symbols() {
    let json = serde_json::to_string(&help_deck_with("fr", |s| std::borrow::Cow::Borrowed(s)).unwrap()).unwrap();
    assert_that!(json.contains("⌘⇧S")).is_true();
}

#[test]
fn open_help_with_a_theme_shows_the_help_in_that_theme() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help("fr", Some(crate::PRO_THEME)).unwrap();
    assert_that!(s.snapshot().theme.and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
    assert_that!(s.tab_info().theme).is_equal_to(Some(crate::PRO_THEME));
}

#[test]
fn open_help_with_an_unknown_theme_keeps_the_help_theme() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help("fr", Some(uuid::Uuid::new_v4())).unwrap();
    assert_that!(s.snapshot().theme.and_then(crate::test_support::theme_name)).is_equal_to(Some("papier"));
}

#[test]
fn open_help_embeds_its_theme_version_without_issue() {
    let (_data, s) = help_session("fr");
    let theme = s.deck_theme_pin().cloned().unwrap();
    assert_that!(theme.id).is_equal_to(crate::DEFAULT_THEME);
    assert_that!(s.sources_for_tests().themes.contains_key(&theme.version)).is_true();
    let zip = s.sources_for_tests().to_zip().unwrap();
    let limits = oku_slides_core::SourceLimits { replica_bytes: 1 << 26, total_bytes: 1 << 28, compact_after_secs: 0 };
    let (_, issues) = oku_slides_core::SourceBundle::from_zip(&zip, limits, crate::test_support::registry()).unwrap();
    assert_that!(issues).is_empty();
}

#[test]
fn set_help_theme_changes_the_theme_without_undo_nor_unsaved_work() {
    let (_data, mut s) = help_session("fr");
    s.set_help_theme(crate::PRO_THEME).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.theme.and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
    assert_that!(snap.can_undo).is_false();
    assert_that!(snap.dirty).is_false();
}

#[test]
fn set_help_theme_outside_the_help_is_refused() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.new_deck().unwrap();
    assert_that!(s.set_help_theme(crate::PRO_THEME)).is_err();
}

#[test]
fn search_finds_a_word_of_a_slide_body_without_case_nor_accents() {
    let (_data, s) = help_session("fr");
    let hits = s.search("FUSION SANS CONFLIT");
    assert_that!(hits.iter().any(|h| h.slide == "s-format-loro")).is_true();
    let hits = s.search("depot");
    assert_that!(hits.is_empty()).is_true();
    let hits = s.search("theme");
    assert_that!(hits.iter().any(|h| h.slide == "s-grille")).is_true();
}

#[test]
fn search_puts_title_matches_first_with_their_slide_title() {
    let (_data, s) = help_session("fr");
    let hits = s.search("historique");
    assert_that!(hits.first().map(|h| h.slide)).is_equal_to(Some("s-fichiers"));
    assert_that!(hits[0].title.as_deref()).is_equal_to(Some("Fichiers, onglets, historique"));
}

#[test]
fn search_excerpt_is_the_matching_line_shortened_around_the_match() {
    let (_data, s) = help_session("en");
    let hits = s.search("canonical read-back");
    let hit = hits.iter().find(|h| h.slide == "s-format-loro").unwrap();
    let excerpt = hit.excerpt.as_deref().unwrap();
    assert_that!(excerpt.contains("canonical read-back")).is_true();
    assert_that!(excerpt.chars().count()).is_less_than_or_equal_to(SEARCH_EXCERPT + 2);
}

#[test]
fn search_with_a_blank_query_finds_nothing() {
    let (_data, s) = help_session("en");
    assert_that!(s.search("  ").is_empty()).is_true();
}

#[test]
fn search_is_allowed_in_the_help_through_handle() {
    let (_data, mut s) = help_session("fr");
    let out = s.handle(req(json!({"cmd": "search", "query": "grille"}))).unwrap();
    assert_that!(out.contains("s-grille")).is_true();
}

#[test]
fn search_with_only_the_title_matching_has_no_excerpt() {
    let (_data, s) = help_session("fr");
    let hits = s.search("onglets, historique");
    let hit = hits.iter().find(|h| h.slide == "s-fichiers").unwrap();
    assert_that!(hit.excerpt.as_deref()).is_none();
}

#[test]
fn help_deck_pins_the_builtin_base_and_essentiel() {
    let (_data, s) = help_session("fr");
    let builtin = |id| crate::test_support::builtin(id).version.clone();
    let pins: Vec<(String, oku_slides_core::PackVersion)> =
        s.deck().all_pins().map(|(_, n, p)| (n.to_string(), p.version.clone())).collect();
    assert_that!(pins).is_equal_to(vec![
        ("base".into(), builtin(crate::pack::BASE_PACK)),
        ("essentiel".into(), builtin(crate::ESSENTIEL_PACK)),
    ]);
}
