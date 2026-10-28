//! Outils de l'app hors du deck : fichier (déplacer, copier, fusionner), historique, présentation,
//! interface. Ce que le menu principal faisait, l'agent le fait (t-988).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use common::*;
use oku_slides_mcp::{InterfaceChange, McpEvent, PresentMode, SchemeChoice, SurfaceAction};
use oku_slides_session::Request;
use rstest::rstest;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use tokio::sync::broadcast::Receiver;

/// Appelle `tool`, rend son résultat et sa confirmation (sérialisée), répondue `confirmed`.
async fn confirmed(h: &Harness, tool: &'static str, args: Value, confirmed: bool) -> (CallToolResult, String) {
    let mut rx = h.hub.subscribe();
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    let shown = serde_json::to_string(&surface).unwrap_or_else(|e| panic!("surface : {e}"));
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(confirmed));
    h.hub
        .act(SurfaceAction { surface: surface.id, name: "confirm".into(), context })
        .unwrap_or_else(|e| panic!("réponse à la confirmation : {e}"));
    (pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}")), shown)
}

/// Appelle `tool` sans confirmation attendue.
async fn direct(h: &Harness, tool: &str, args: Value) -> CallToolResult {
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    call(&c, tool, args).await
}

/// `req` sur la session, hors du fil async : `Storage` (OpenDAL) bloque.
fn handle(h: &Harness, req: Request) -> String {
    let session = || h.session.lock().unwrap_or_else(PoisonError::into_inner);
    tokio::task::block_in_place(|| session().handle(req)).unwrap_or_else(|e| panic!("requête de test : {e}"))
}

/// Une requête écrite comme l'interface l'envoie.
fn req(v: Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête de test : {e}"))
}

/// Change le titre du deck : une version de plus dans l'historique.
fn retitle(h: &Harness) {
    let op = json!({ "op": "set_deck_param", "key": "title", "value": "Neuf" });
    handle(h, req(json!({ "cmd": "apply", "label": "Titre", "ops": [op] })));
}

/// Enregistre le deck de test dans `dir` ; rend son fichier.
fn saved_in(h: &Harness, dir: &Path) -> PathBuf {
    let path = dir.join("deck.oku-slides.pdf");
    handle(h, Request::Save { path: Some(path.clone()), overwrite: false });
    path
}

/// Un fichier qui n'est pas le deck, à ne pas toucher.
fn other_file(dir: &Path) -> PathBuf {
    let path = dir.join("autre.oku-slides.pdf");
    std::fs::write(&path, b"autre").unwrap_or_else(|e| panic!("autre fichier : {e}"));
    path
}

fn bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_default()
}

fn snapshot<T>(h: &Harness, f: impl FnOnce(oku_slides_session::Snapshot<'_>) -> T) -> T {
    f(h.session.lock().unwrap_or_else(PoisonError::into_inner).snapshot())
}

fn own_path(h: &Harness) -> Option<PathBuf> {
    snapshot(h, |s| s.path.map(Path::to_path_buf))
}

fn title(h: &Harness) -> Option<String> {
    snapshot(h, |s| s.title)
}

/// Le prochain évènement d'interface du hub.
async fn next_interface(rx: &mut Receiver<McpEvent>) -> InterfaceChange {
    wait_event(rx, |e| match e {
        McpEvent::Interface { change } => Some(change),
        _ => None,
    })
    .await
}

/// Le mode du prochain évènement de présentation du hub.
async fn next_present(rx: &mut Receiver<McpEvent>) -> PresentMode {
    wait_event(rx, |e| match e {
        McpEvent::Present { mode } => Some(mode),
        _ => None,
    })
    .await
}

/// Aucun évènement de la page n'attend dans `rx`.
fn no_page_event(rx: &mut Receiver<McpEvent>) -> bool {
    std::iter::from_fn(|| rx.try_recv().ok())
        .all(|e| !matches!(e, McpEvent::Present { .. } | McpEvent::Interface { .. } | McpEvent::OpenHistory))
}

// --- move, save_copy, merge ------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn move_confirmed_moves_the_file_and_deletes_the_old_one() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-");
    let old = saved_in(&h, dir.path());
    let new = dir.path().join("range.oku-slides.pdf");
    confirmed(&h, "move", json!({ "path": new.to_string_lossy() }), true).await;
    assert_that!((own_path(&h), new.exists(), old.exists())).is_equal_to((Some(new.clone()), true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn move_denied_writes_nothing_and_keeps_the_file() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-no-");
    let old = saved_in(&h, dir.path());
    let new = dir.path().join("range.oku-slides.pdf");
    confirmed(&h, "move", json!({ "path": new.to_string_lossy() }), false).await;
    assert_that!((own_path(&h), new.exists(), old.exists())).is_equal_to((Some(old.clone()), false, true));
}

/// Revue t-988 : `move` ne remplace jamais un autre fichier, sans rien demander.
#[tokio::test(flavor = "multi_thread")]
async fn move_onto_an_existing_file_is_refused_without_asking_and_leaves_it() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-over-");
    saved_in(&h, dir.path());
    let other = other_file(dir.path());
    let r = direct(&h, "move", json!({ "path": other.to_string_lossy() })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-target-exists"));
    assert_that!((h.hub.has_pending_ask(), bytes(&other))).is_equal_to((false, b"autre".to_vec()));
}

/// Revue t-988 : déplacer sur son propre fichier n'est pas un enregistrement déguisé.
#[tokio::test(flavor = "multi_thread")]
async fn move_onto_its_own_file_is_refused_without_asking() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-self-");
    let own = saved_in(&h, dir.path());
    let r = direct(&h, "move", json!({ "path": own.to_string_lossy() })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-target-exists"));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

/// Revue t-988 : un fichier apparu pendant la confirmation n'est pas remplacé (vérifié en écrivant).
#[tokio::test(flavor = "multi_thread")]
async fn move_onto_a_file_created_during_the_confirmation_is_refused() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-race-");
    saved_in(&h, dir.path());
    let target = dir.path().join("autre.oku-slides.pdf");
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client : {e}"));
    let args = json!({ "path": target.to_string_lossy() });
    let pending = tokio::spawn(async move { call(&c, "move", args).await });
    let surface = next_surface(&mut rx).await;
    other_file(dir.path());
    h.hub
        .act(action(&surface.id, "confirm", &json!({ "confirmed": true })))
        .unwrap_or_else(|e| panic!("réponse : {e}"));
    let r = pending.await.unwrap_or_else(|e| panic!("tâche : {e}"));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-target-exists"));
    assert_that!(bytes(&target)).is_equal_to(b"autre".to_vec());
}

/// Revue t-988 : le chemin vérifié, montré et écrit est le même (`.pdf` → `.oku-slides.pdf`).
#[tokio::test(flavor = "multi_thread")]
async fn move_to_a_plain_pdf_name_confirms_and_writes_the_deck_file() {
    let h = start().await;
    let dir = temp_dir("oku-slides-move-pdf-");
    saved_in(&h, dir.path());
    let written = dir.path().join("range.oku-slides.pdf");
    let asked = dir.path().join("range.pdf");
    let (_, shown) = confirmed(&h, "move", json!({ "path": asked.to_string_lossy() }), true).await;
    assert_that!(shown).contains(written.to_string_lossy().as_ref());
    assert_that!(own_path(&h)).is_equal_to(Some(written));
}

/// Revue t-988 : le fichier d'un autre onglet est reconnu sous son nom normalisé.
#[tokio::test(flavor = "multi_thread")]
async fn move_to_the_plain_pdf_name_of_a_file_open_elsewhere_is_refused() {
    let dir = temp_dir("oku-slides-move-pris-");
    let h = start_with_one_file_elsewhere(dir.path().join("deck.oku-slides.pdf")).await;
    let r = direct(&h, "move", json!({ "path": dir.path().join("deck.pdf").to_string_lossy() })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-open-elsewhere"));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn move_in_the_history_is_refused_without_asking() {
    let h = start().await;
    handle(&h, Request::HistoryOpen);
    let dir = temp_dir("oku-slides-move-hist-");
    let r = direct(&h, "move", json!({ "path": dir.path().join("x.pdf").to_string_lossy() })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-in-history"));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_copy_confirmed_writes_a_copy_and_keeps_the_file() {
    let h = start().await;
    let dir = temp_dir("oku-slides-copy-");
    let own = saved_in(&h, dir.path());
    let copy = dir.path().join("copie.oku-slides.pdf");
    confirmed(&h, "save_copy", json!({ "path": copy.to_string_lossy() }), true).await;
    assert_that!((copy.exists(), own_path(&h))).is_equal_to((true, Some(own)));
}

#[tokio::test(flavor = "multi_thread")]
async fn save_copy_onto_an_existing_file_is_refused_without_asking_and_leaves_it() {
    let h = start().await;
    let dir = temp_dir("oku-slides-copy-over-");
    saved_in(&h, dir.path());
    let other = other_file(dir.path());
    let r = direct(&h, "save_copy", json!({ "path": other.to_string_lossy() })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-target-exists"));
    assert_that!((h.hub.has_pending_ask(), bytes(&other))).is_equal_to((false, b"autre".to_vec()));
}

/// Rouvre `path` dans une autre session, en change le titre et l'enregistre : une copie modifiée
/// à part, comme une autre machine.
fn retitled_elsewhere(path: &Path) {
    let (platform, _data) = platform();
    let mut other = oku_slides_session::Session::new(oku_slides_session::Fonts::default(), &platform)
        .unwrap_or_else(|e| panic!("autre session : {e}"));
    other.open(path).unwrap_or_else(|e| panic!("ouverture de la copie : {e}"));
    let op = json!({ "op": "set_deck_param", "key": "title", "value": "Neuf" });
    other
        .handle(req(json!({ "cmd": "apply", "label": "Titre", "ops": [op] })))
        .unwrap_or_else(|e| panic!("titre : {e}"));
    other.save(None).unwrap_or_else(|e| panic!("enregistrement de la copie : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn merge_confirmed_brings_the_edits_of_the_other_copy() {
    let h = start().await;
    let dir = temp_dir("oku-slides-merge-");
    saved_in(&h, dir.path());
    let copy = dir.path().join("copie.oku-slides.pdf");
    handle(&h, Request::SaveCopy { path: copy.clone() });
    tokio::task::block_in_place(|| retitled_elsewhere(&copy));
    confirmed(&h, "merge", json!({ "path": copy.to_string_lossy() }), true).await;
    assert_that!(title(&h)).is_equal_to(Some("Neuf".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn merge_confirmation_names_the_file() {
    let h = start().await;
    let dir = temp_dir("oku-slides-merge-ask-");
    let path = dir.path().join("autre.oku-slides.pdf");
    let (_, shown) = confirmed(&h, "merge", json!({ "path": path.to_string_lossy() }), false).await;
    assert_that!(shown).contains("autre.oku-slides.pdf");
}

#[tokio::test(flavor = "multi_thread")]
async fn merge_in_pack_mode_is_refused_without_asking() {
    let h = start_pack_tab(oku_slides_session::ESSENTIEL_PACK).await;
    let dir = temp_dir("oku-slides-merge-tpl-");
    let r = direct(&h, "merge", json!({ "path": dir.path().join("x.oku-slides.pdf").to_string_lossy() })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

// --- history ----------------------------------------------------------------

/// Joue l'onglet : à la demande d'ouverture de l'historique, l'ouvre comme sa propre commande.
fn tab_opens_history(h: &Harness) -> tokio::task::JoinHandle<()> {
    let (mut rx, session) = (h.hub.subscribe(), h.session.clone());
    tokio::spawn(async move {
        wait_event(&mut rx, |e| matches!(e, McpEvent::OpenHistory).then_some(())).await;
        let opened = tokio::task::spawn_blocking(move || {
            session.lock().unwrap_or_else(PoisonError::into_inner).handle(Request::HistoryOpen).map(|_| ())
        });
        opened.await.unwrap_or_else(|e| panic!("tâche : {e}")).unwrap_or_else(|e| panic!("ouverture : {e}"));
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn history_open_waits_for_the_tab_and_returns_the_versions() {
    let h = start().await;
    retitle(&h);
    let tab = tab_opens_history(&h);
    let r = direct(&h, "history", json!({ "action": "open" })).await;
    tab.await.unwrap_or_else(|e| panic!("onglet : {e}"));
    let versions = json_of(&r).as_array().map_or(0, Vec::len);
    assert_that!(versions).is_greater_than_or_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn history_open_the_tab_never_opens_is_said_in_the_user_language() {
    let h = start().await;
    let r = direct(&h, "history", json!({ "action": "open" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-history-not-opened"));
}

#[tokio::test(flavor = "multi_thread")]
async fn history_open_already_open_is_refused_without_asking_the_tab() {
    let h = start().await;
    handle(&h, Request::HistoryOpen);
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "history", json!({ "action": "open" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-in-history"));
    assert_that!(no_page_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn history_open_on_a_hidden_tab_is_refused() {
    let h = start_hidden().await;
    let r = direct(&h, "history", json!({ "action": "open" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-not-shown"));
}

#[tokio::test(flavor = "multi_thread")]
async fn history_list_returns_every_version_oldest_first() {
    let h = start().await;
    retitle(&h);
    handle(&h, Request::HistoryOpen);
    let expected: Value =
        serde_json::from_str(&handle(&h, Request::HistoryVersions)).unwrap_or_else(|e| panic!("versions : {e}"));
    let r = direct(&h, "history", json!({ "action": "list" })).await;
    assert_that!(expected.as_array().map_or(0, Vec::len)).is_greater_than_or_equal_to(2);
    assert_that!(json_of(&r)).is_equal_to(expected);
}

#[rstest]
#[case::first(0, 0)]
#[case::past_the_last(99, 1)]
#[tokio::test(flavor = "multi_thread")]
async fn history_seek_shows_the_version_asked_bounded_to_the_last(#[case] asked: u64, #[case] shown: u64) {
    let h = start().await;
    retitle(&h);
    handle(&h, Request::HistoryOpen);
    let count = snapshot(&h, |s| s.history.map(|v| v.count)).unwrap_or(0) as u64;
    let r = direct(&h, "history", json!({ "action": "seek", "index": asked })).await;
    let last = count.saturating_sub(1);
    let expected = if shown == 0 { 0 } else { last };
    assert_that!(json_of(&r)["shown"]).is_equal_to(json!({ "index": expected, "count": count }));
    assert_that!(snapshot(&h, |s| s.history.map(|v| v.index as u64))).is_equal_to(Some(expected));
}

#[tokio::test(flavor = "multi_thread")]
async fn history_restore_brings_the_version_back() {
    let h = start().await;
    let before = title(&h);
    retitle(&h);
    handle(&h, Request::HistoryOpen);
    direct(&h, "history", json!({ "action": "seek", "index": 0 })).await;
    direct(&h, "history", json!({ "action": "restore" })).await;
    assert_that!(title(&h)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn history_close_gives_the_deck_back() {
    let h = start().await;
    handle(&h, Request::HistoryOpen);
    direct(&h, "history", json!({ "action": "close" })).await;
    assert_that!(snapshot(&h, |s| s.history.is_none())).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn history_seek_without_index_is_refused_in_the_user_language() {
    let h = start().await;
    handle(&h, Request::HistoryOpen);
    let r = direct(&h, "history", json!({ "action": "seek" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-history-index"));
}

// --- present ----------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn present_emits_present_event_with_its_mode() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "present", json!({ "mode": "window" })).await;
    assert_that!(next_present(&mut rx).await).is_equal_to(PresentMode::Window);
}

#[tokio::test(flavor = "multi_thread")]
async fn present_without_mode_presents_on_screen() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "present", json!({})).await;
    assert_that!(next_present(&mut rx).await).is_equal_to(PresentMode::Screen);
}

/// Un mode où la page ne présente pas.
#[derive(Debug, Clone, Copy)]
enum NoPresent {
    History,
    Pack,
    NoSlide,
}

/// A harness whose session is in `mode`; pack mode is a pack tab of Essentiel.
async fn start_in(mode: NoPresent) -> Harness {
    if let NoPresent::Pack = mode {
        return start_pack_tab(oku_slides_session::ESSENTIEL_PACK).await;
    }
    let h = start().await;
    match mode {
        NoPresent::History => {
            handle(&h, Request::HistoryOpen);
        }
        NoPresent::Pack => {}
        NoPresent::NoSlide => {
            let ids: Vec<String> = snapshot(&h, |s| s.slides.iter().map(|s| s.id.to_owned()).collect());
            let ops: Vec<Value> = ids.iter().map(|id| json!({ "op": "remove_slide", "slide": id })).collect();
            handle(&h, req(json!({ "cmd": "apply", "label": "Vider", "ops": ops })));
        }
    }
    h
}

#[rstest]
#[case::history(NoPresent::History)]
#[case::pack(NoPresent::Pack)]
#[case::no_slide(NoPresent::NoSlide)]
#[tokio::test(flavor = "multi_thread")]
async fn present_where_the_page_cannot_present_is_refused_and_emits_nothing(#[case] mode: NoPresent) {
    let h = start_in(mode).await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "present", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-present"));
    assert_that!(no_page_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn present_on_a_hidden_tab_is_refused_in_the_user_language() {
    let h = start_hidden().await;
    let r = direct(&h, "present", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-not-shown"));
}

// --- set_interface ----------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn set_interface_emits_the_requested_changes_only() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "set_interface", json!({ "scheme": "dark", "guides": true })).await;
    assert_that!(next_interface(&mut rx).await).is_equal_to(InterfaceChange {
        scheme: Some(SchemeChoice::Dark),
        locale: None,
        grid: None,
        guides: Some(true),
    });
}

/// Revue t-988 : l'interface ne reçoit qu'un code livré.
#[rstest]
#[case("fr", "fr")]
#[case("FR-ca", "fr")]
#[case("system", "system")]
#[case("System", "system")]
#[tokio::test(flavor = "multi_thread")]
async fn set_interface_sends_a_shipped_locale_code(#[case] asked: &str, #[case] sent: &str) {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "set_interface", json!({ "locale": asked })).await;
    let locale = next_interface(&mut rx).await.locale;
    assert_that!(locale).is_equal_to(Some(sent.to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_interface_with_an_unknown_locale_is_refused_and_emits_nothing() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "set_interface", json!({ "locale": "xx" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-locale"));
    assert_that!(no_page_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_interface_with_nothing_to_change_is_refused() {
    let h = start().await;
    let r = direct(&h, "set_interface", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-empty-interface"));
}

/// Revue t-988 : un champ inconnu (`language` au lieu de `locale`) est refusé, pas ignoré.
#[tokio::test(flavor = "multi_thread")]
async fn set_interface_with_an_unknown_field_is_refused_and_emits_nothing() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "set_interface", json!({ "language": "fr" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(no_page_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_interface_on_a_hidden_tab_is_refused() {
    let h = start_hidden().await;
    let r = direct(&h, "set_interface", json!({ "grid": true })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-not-shown"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_no_longer_writes_the_locale() {
    let h = start().await;
    let asked = direct(&h, "set_setting", json!({ "key": "i18n.locale", "value": "en" }));
    let r =
        tokio::time::timeout(EVENT_WAIT, asked).await.unwrap_or_else(|_| panic!("set_setting attend une confirmation"));
    let refused = h.localizer.tr_with("agent-error-setting-locked", &[("key", "i18n.locale".into())]);
    assert_that!(text_of(&r)).is_equal_to(refused);
}

// --- edit_pack --------------------------------------------------------------

/// Review t-988: without `pack`, the deck's main pack (the one the interface opens too): a
/// non-built-in pack, layouts first, else Essentiel; never `base`.
#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_without_pack_opens_the_deck_pack_not_base() {
    use oku_slides_session::{BASE_PACK, ESSENTIEL_PACK};
    let (h, tabs) = start_other_tabs().await;
    h.hub.begin_turn();
    let builtin = |id: &uuid::Uuid| *id == BASE_PACK || *id == ESSENTIEL_PACK;
    let (layouts, widgets): (Vec<_>, Vec<_>) = snapshot(&h, |s| {
        let own = |pins: &std::collections::BTreeMap<oku_slides_core::Namespace, oku_slides_core::PackPin>| {
            pins.values().map(|pin| pin.id).filter(|id| !builtin(id)).collect::<Vec<_>>()
        };
        (own(s.layouts), own(s.widgets))
    });
    let expected = layouts.first().or(widgets.first()).copied().unwrap_or(ESSENTIEL_PACK);
    direct(&h, "edit_pack", json!({})).await;
    let opened: Vec<uuid::Uuid> = lock(&tabs.opened_packs).iter().map(|(_, pack, _)| *pack).collect();
    assert_that!(opened).is_equal_to(vec![expected]);
}

/// A single-document host opens no pack tab: `edit_pack` there is refused, the deck unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn edit_pack_on_a_single_document_host_is_refused_as_single_document() {
    let h = start().await;
    let r = direct(&h, "edit_pack", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let refused = snapshot(&h, |s| s.editing.is_none());
    assert_that!(refused).is_true();
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-single-document"));
}

// --- contrat avec l'interface -------------------------------------------------

/// Revue t-988 : la forme JSON des évènements, telle que `ui/src/api.ts` la lit.
#[test]
fn mcp_event_page_events_serialize_as_the_ui_reads_them() {
    let change = InterfaceChange { guides: Some(true), ..InterfaceChange::default() };
    let events =
        [McpEvent::Present { mode: PresentMode::Window }, McpEvent::Interface { change }, McpEvent::OpenHistory];
    let json: Vec<Value> = events.iter().map(|e| serde_json::to_value(e).unwrap_or(Value::Null)).collect();
    assert_that!(json).is_equal_to(vec![
        json!({ "kind": "present", "mode": "window" }),
        json!({ "kind": "interface", "change": { "scheme": null, "locale": null, "grid": null, "guides": true } }),
        json!({ "kind": "open_history" }),
    ]);
}
