use std::path::{Path, PathBuf};

use serde_json::json;
use speculoos::prelude::*;
use time::UtcOffset;

use oku_slides_core::Op;

use super::*;
use crate::{Fonts, Platform, Request, SessionErrorKind, Settings};

/// Un dossier neuf, créé atomiquement sous un nom aléatoire, supprimé à la fin du test.
fn data_dir() -> tempfile::TempDir {
    crate::test_support::fresh_dir("oku-slides-history-")
}

fn session_in(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap()
}

fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap()
}

fn set_theme(theme: &str) -> Request {
    req(
        json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": crate::test_support::theme_ref(theme)}]}),
    )
}

/// Le deck de démonstration avec une image et une police, puis deux thèmes.
fn edited(s: &mut Session) {
    s.open(&fixture_path()).unwrap();
    s.doc.sources.add_asset(b"image".to_vec());
    s.doc.sources.fonts.insert("Inter.ttf".into(), b"police".to_vec());
    s.handle(set_theme("pro")).unwrap();
    s.handle(set_theme("papier")).unwrap();
}

/// 2026-09-27 08:05:59 UTC.
const TIMESTAMP: i64 = 1_790_496_359;

#[test]
fn restore_date_in_an_eastern_offset_is_local_to_the_minute() {
    let offset = UtcOffset::from_hms(2, 0, 0).unwrap();
    assert_that!(restore_date(TIMESTAMP, offset)).is_equal_to("2026-09-27 10:05".to_owned());
}

#[test]
fn restore_date_in_a_western_offset_can_fall_the_day_before() {
    let offset = UtcOffset::from_hms(-9, -30, 0).unwrap();
    assert_that!(restore_date(TIMESTAMP, offset)).is_equal_to("2026-09-26 22:35".to_owned());
}

#[test]
fn restore_date_out_of_range_falls_back_to_the_epoch() {
    assert_that!(restore_date(i64::MAX, UtcOffset::UTC)).is_equal_to("1970-01-01 00:00".to_owned());
}

#[test]
fn restore_label_uses_the_fluent_message() {
    let s = session_in(data_dir().path());
    assert_that!(s.restore_label(TIMESTAMP)).starts_with("Restaurer la version du 2026-09-2");
}

#[test]
fn history_open_in_templates_mode_returns_editing_templates() {
    let mut s = session_in(data_dir().path());
    s.handle(req(json!({ "cmd": "edit_templates", "set": crate::DEFAULT_PACK }))).unwrap();
    let err = s.handle(req(json!({ "cmd": "history_open" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingTemplates);
    assert_that!(matches!(s.mode, Mode::Templates { .. })).is_true();
}

#[test]
fn history_keeps_assets_and_fonts_in_the_preview_then_gives_them_back() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    let (assets, fonts) = (s.doc.sources.assets.clone(), s.doc.sources.fonts.clone());
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    // L'aperçu tient les octets, sans copie : le deck mis de côté n'en a plus.
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
    let Mode::History { stash, .. } = &s.mode else { panic!("pas en mode historique") };
    assert_that!(stash.sources.assets.is_empty()).is_true();
    s.handle(req(json!({ "cmd": "history_restore" }))).unwrap();
    assert_that!(matches!(s.mode, Mode::Deck)).is_true();
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
}

#[test]
fn history_unsaved_and_tab_info_see_the_stashed_deck() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    assert_that!(s.unsaved()).is_true();
    assert_that!(s.user_deck().theme.as_ref().and_then(crate::test_support::theme_name)).is_equal_to(Some("papier"));
    assert_that!(s.tab_info().editing_templates).is_false();
}

/// Un onglet rouvert ne revient pas dans l'historique : la reprise garde le
/// deck courant, ses images et ses polices, en mode deck.
#[test]
fn history_resume_reopens_the_current_deck_not_the_history() {
    let data = data_dir();
    let dir = data.path().join("resume");
    let mut a = session_in(data.path());
    assert!(a.resume(dir.clone()));
    edited(&mut a);
    let (assets, fonts) = (a.doc.sources.assets.clone(), a.doc.sources.fonts.clone());
    a.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    a.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    // Une commande qui écrit aussi les documents, pendant l'historique.
    a.persist(true);
    a.flush_resume();
    drop(a);

    let mut b = session_in(data.path());
    assert!(b.resume(dir));
    assert_that!(matches!(b.mode, Mode::Deck)).is_true();
    assert_that!(b.deck().theme.as_ref().and_then(crate::test_support::theme_name)).is_equal_to(Some("papier"));
    assert_that!(b.doc.sources.assets).is_equal_to(&assets);
    assert_that!(b.doc.sources.fonts).is_equal_to(&fonts);
    assert_that!(b.snapshot().history.is_none()).is_true();
}

/// `edited`, puis une version dont le rendu échoue, aussitôt réparée : rend
/// l'indice de cette version.
fn with_an_unrenderable_version(s: &mut Session) -> usize {
    edited(s);
    let good = s.deck().clone();
    let mut bad = good.clone();
    // Un jeton de couleur qui n'est pas une couleur fait échouer le rendu Typst.
    let token = Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!(5) };
    oku_slides_core::apply_all(&mut bad, vec![token], &crate::test_support::registry()).unwrap();
    s.doc.sources.apply(bad, "Mauvais", None).unwrap();
    s.doc.sources.apply(good, "Réparer", None).unwrap();
    s.doc.sources.history().len() - 2
}

#[test]
fn history_seek_that_fails_to_render_keeps_the_shown_version_and_the_blobs() {
    let mut s = session_in(data_dir().path());
    let bad = with_an_unrenderable_version(&mut s);
    let (assets, fonts) = (s.doc.sources.assets.clone(), s.doc.sources.fonts.clone());
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let err = s.handle(req(json!({ "cmd": "history_seek", "index": bad }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Render);
    let Mode::History { stash, history } = &s.mode else { panic!("l'historique reste ouvert") };
    assert_that!(history.index).is_equal_to(bad + 1);
    assert_that!(stash.sources.assets.is_empty()).is_true();
    // L'aperçu montré garde les images et polices.
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
}

#[test]
fn history_restore_that_fails_keeps_the_history_open_and_the_stashed_deck_intact() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    let (assets, fonts) = (s.doc.sources.assets.clone(), s.doc.sources.fonts.clone());
    let changes = s.doc.sources.replica().len_changes();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    let Mode::History { history, .. } = &s.mode else { panic!("pas en mode historique") };
    // Une version illisible : la lecture de l'aperçu a paniqué.
    history.preview.poison_for_tests();
    let err = s.handle(req(json!({ "cmd": "history_restore" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidDocument);
    let Mode::History { stash, .. } = &s.mode else { panic!("l'historique reste ouvert") };
    assert_that!(stash.sources.replica().deck().0).is_equal_to(stash.sources.deck());
    assert_that!(stash.sources.deck().theme.as_ref().and_then(crate::test_support::theme_name))
        .is_equal_to(Some("papier"));
    assert_that!(stash.sources.replica().len_changes()).is_equal_to(changes);
    assert_that!(stash.sources.assets.is_empty()).is_true();
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    assert_that!(s.deck().theme.as_ref().and_then(crate::test_support::theme_name)).is_equal_to(Some("papier"));
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Thème".to_owned()));
    assert_that!(s.doc.sources.assets).is_equal_to(&assets);
    assert_that!(s.doc.sources.fonts).is_equal_to(&fonts);
}

/// L'agent passe à côté de `handle` (`TabSession::with`) : `apply` refuse
/// lui-même pendant l'historique, sans toucher au deck mis de côté ni à l'aperçu.
#[test]
fn history_agent_apply_is_refused_and_leaves_the_stash_and_the_preview_untouched() {
    use std::sync::{Arc, Mutex};

    use crate::agent::TabSession;

    let data = data_dir();
    let mut s = session_in(data.path());
    edited(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    let (shown, changes) = (s.deck().clone(), s.doc.sources.replica().len_changes());
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(s)));
    let op = Op::SetTheme { theme: crate::test_support::theme_ref("pro") };
    let err = tab.with(|s| s.apply("Agent", vec![op], None)).unwrap().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
    let err = tab.with(Session::new_deck).unwrap().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
    tab.with(|s| {
        assert_that!(s.deck()).is_equal_to(&shown);
        assert_that!(s.doc.sources.replica().len_changes()).is_equal_to(changes);
        let Mode::History { stash, .. } = &s.mode else { panic!("l'historique reste ouvert") };
        assert_that!(stash.sources.deck().theme.as_ref().and_then(crate::test_support::theme_name))
            .is_equal_to(Some("papier"));
        assert_that!(stash.dirty).is_true();
        s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
        assert_that!(s.snapshot().undo_label).is_equal_to(Some("Thème".to_owned()));
    })
    .unwrap();
}

#[test]
fn history_agent_undo_and_redo_are_refused() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    assert_that!(s.undo().unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(s.redo().unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(matches!(s.mode, Mode::History { .. })).is_true();
}

#[test]
fn history_agent_context_says_history_is_open_and_read_only() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    let Mode::History { history, .. } = &s.mode else { panic!("pas en mode historique") };
    let count = history.view().count;
    let context = s.agent_context(&crate::agent::AgentUi::default());
    assert_that!(context).contains(format!("mode: history(read_only, version=1/{count})").as_str());
}

/// Un tour ouvert avant l'historique se ferme aussi sur le deck mis de côté.
#[test]
fn history_end_turn_closes_the_group_on_the_stashed_deck() {
    let mut s = session_in(data_dir().path());
    edited(&mut s);
    s.begin_turn(1, "rends-le plus sobre");
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.end_turn();
    let Mode::History { stash, .. } = &s.mode else { panic!("pas en mode historique") };
    assert_that!(stash.sources.replica().in_group()).is_false();
}

/// Un deck enregistré, modifié encore, l'historique ouvert sur la première version.
fn saved_then_in_history(data: &Path) -> (Session, PathBuf, Vec<u8>) {
    std::fs::create_dir_all(data).unwrap();
    let mut s = session_in(data);
    edited(&mut s);
    let path = data.join("deck.oku-slides.pdf");
    s.save(Some(&path)).unwrap();
    s.handle(set_theme("pro")).unwrap();
    let written = std::fs::read(&path).unwrap();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    (s, path, written)
}

/// Déplacer, copier, enregistrer : rien n'écrit la version montrée, ni ne
/// touche au fichier du deck ; le deck mis de côté reste à enregistrer.
#[test]
fn history_move_save_copy_and_save_are_refused_and_write_nothing() {
    let data = data_dir();
    let (mut s, path, written) = saved_then_in_history(data.path());
    let (moved, copy) = (data.path().join("ailleurs.oku-slides.pdf"), data.path().join("copie.oku-slides.pdf"));
    for r in [
        json!({ "cmd": "move", "path": moved }),
        json!({ "cmd": "save_copy", "path": copy }),
        json!({ "cmd": "save", "path": null }),
    ] {
        let err = s.handle(req(r)).unwrap_err();
        assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
    }
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(&written);
    assert_that!(moved.exists()).is_false();
    assert_that!(copy.exists()).is_false();
    assert_that!(s.unsaved()).is_true();
}

/// L'agent appelle les méthodes directement : elles se gardent elles-mêmes.
#[test]
fn history_move_to_save_copy_and_save_methods_are_refused() {
    let data = data_dir();
    let (mut s, path, written) = saved_then_in_history(data.path());
    let (moved, copy) = (data.path().join("ailleurs.oku-slides.pdf"), data.path().join("copie.oku-slides.pdf"));
    assert_that!(s.move_to(&moved).unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(s.save_copy(&copy).unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(s.save(None).unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(&written);
    assert_that!(moved.exists()).is_false();
    assert_that!(copy.exists()).is_false();
}

/// Fermer l'historique rend le deck : l'enregistrement (automatique) écrit
/// alors le deck mis de côté, pas une version passée.
#[test]
fn history_close_then_save_writes_the_stashed_deck() {
    let data = data_dir();
    let (mut s, path, _) = saved_then_in_history(data.path());
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    s.handle(req(json!({ "cmd": "save", "path": null }))).unwrap();
    let mut back = session_in(data.path());
    back.open(&path).unwrap();
    assert_that!(back.deck().theme.as_ref().and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
}
