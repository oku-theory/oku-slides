mod common;

use common::*;
use oku_slides_mcp::SettingsPort;
use serde_json::{Value, json};
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn save_denied_writes_nothing() {
    let h = start().await;
    let dir = temp_dir("oku-slides-guarded-");
    let path = dir.path().join("deck.oku-slides.pdf");
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    // Dit dans la langue de l'utilisateur (`fr`), comme toutes les erreurs des outils.
    assert_that!(text_of(&r).as_str()).is_equal_to("L'utilisateur a refusé.");
    assert_that!(path.exists()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_confirmed_writes_file() {
    let h = start().await;
    let dir = temp_dir("oku-slides-guarded-ok-");
    let path = dir.path().join("deck.oku-slides.pdf");
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(path.exists()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_with_parent_component_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "open", json!({ "path": "/tmp/../etc/deck.pdf" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    // Sans le chemin donné : rien d'absolu dans une erreur.
    assert_that!(text_of(&r).as_str()).is_equal_to("Le chemin ne doit pas contenir « .. ».");
    assert_that!(h.hub.has_pending_ask()).is_false();
}

/// Le fichier de la session de test (celui de la fixture).
fn own_path(h: &Harness) -> Option<std::path::PathBuf> {
    h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).snapshot().path.map(|p| p.to_path_buf())
}

/// t-242 : le fichier s'ouvre dans un autre onglet ; celui de l'agent garde son deck.
#[tokio::test(flavor = "multi_thread")]
async fn open_confirmed_opens_the_file_in_another_tab_and_keeps_this_deck() {
    let (h, tabs) = start_other_tabs().await;
    let before = own_path(&h);
    let path = std::env::temp_dir().join("oku-slides-voisin.oku-slides.pdf");
    let r = guarded(&h, "open", json!({ "path": path.to_string_lossy() }), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true, "tab": "t-7" }));
    let opened = tabs.opened.lock().unwrap().clone();
    assert_that!(opened).is_equal_to(vec![("t-1".to_owned(), path)]);
    assert_that!(own_path(&h)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_denied_opens_nothing() {
    let (h, tabs) = start_other_tabs().await;
    let path = std::env::temp_dir().join("oku-slides-refuse.oku-slides.pdf");
    let r = guarded(&h, "open", json!({ "path": path.to_string_lossy() }), false).await;
    // Dit dans la langue de l'utilisateur (`fr`), comme toutes les erreurs des outils.
    assert_that!(text_of(&r).as_str()).is_equal_to("L'utilisateur a refusé.");
    assert_that!(tabs.opened.lock().unwrap().is_empty()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_on_a_single_document_host_fails_and_keeps_this_deck() {
    let h = start().await;
    let before = own_path(&h);
    let path = std::env::temp_dir().join("oku-slides-seul.oku-slides.pdf");
    let r = guarded(&h, "open", json!({ "path": path.to_string_lossy() }), true).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).is_equal_to("Une seule présentation peut être ouverte ici.");
    assert_that!(own_path(&h)).is_equal_to(before);
}

/// t-242 : enregistrer sous le fichier d'un autre onglet est refusé, sans rien demander.
#[tokio::test(flavor = "multi_thread")]
async fn save_to_a_file_open_in_another_tab_is_refused_without_asking() {
    let (h, _) = start_other_tabs().await;
    let dir = temp_dir("oku-slides-pris-");
    let path = dir.path().join("deck.oku-slides.pdf");
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "save", json!({ "path": path.to_string_lossy() })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-open-elsewhere"));
    assert_that!(h.hub.has_pending_ask()).is_false();
    assert_that!(path.exists()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_outside_whitelist_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_setting", json!({ "key": "paths.data", "value": "/tmp" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.settings.read().contains_key("paths.data")).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_confirmed_writes_through_port() {
    let h = start().await;
    let r = guarded(&h, "set_setting", json!({ "key": "history.limit", "value": 42 }), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "next_launch" }));
    assert_that!(h.settings.read().get("history.limit").cloned()).is_equal_to(Some(json!(42)));
}

/// La confirmation de `delete_pack`, sérialisée ; l'utilisateur refuse.
async fn delete_confirmation(h: &Harness, pack: &str) -> String {
    let mut rx = h.hub.subscribe();
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let args = json!({ "pack": pack });
    let pending = tokio::spawn(async move { call(&c, "delete_pack", args).await });
    let surface = next_surface(&mut rx).await;
    let json = serde_json::to_string(&surface).unwrap_or_else(|e| panic!("sérialisation de la surface : {e}"));
    answer(h, &surface.id, false);
    pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}"));
    json
}

/// t-328 : la confirmation nomme le pack, pas son UUID.
#[tokio::test(flavor = "multi_thread")]
async fn delete_pack_confirmation_names_the_pack() {
    let h = start().await;
    let id = oku_slides_session::pack::ESSENTIEL_PACK.to_string();
    let json = delete_confirmation(&h, &id).await;
    assert_that!(json.contains("Essentiel")).is_true();
    assert_that!(json.contains(&id)).is_false();
}

/// t-328 : un id inconnu de la bibliothèque est montré tel quel.
#[tokio::test(flavor = "multi_thread")]
async fn delete_pack_confirmation_of_unknown_pack_shows_its_id() {
    let h = start().await;
    let json = delete_confirmation(&h, "00000000-0000-4000-8000-000000000001").await;
    assert_that!(json.contains("00000000-0000-4000-8000-000000000001")).is_true();
}

/// t-355 : un id qui n'est pas un UUID est refusé sans rien demander, par le
/// message qui renvoie vers `packs`.
#[tokio::test(flavor = "multi_thread")]
async fn delete_pack_with_a_slug_points_to_packs_without_asking() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let r = call(&c, "delete_pack", json!({ "pack": "essentiel" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-pack-id"));
    assert_that!(rx.try_recv().is_err()).is_true();
}

/// L'espace de noms du thème intégré Papier, thème du deck de test.
const PAPIER: &str = "papier";

/// Le thème du deck, lu par `deck_outline`.
async fn deck_theme_name(h: &Harness) -> Value {
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    json_of(&call(&c, "deck_outline", json!({})).await)["theme"]["name"].clone()
}

/// t-783 : l'agent édite le thème du deck sans passer par le mode pack (une variante, Papier étant intégré).
#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_confirmed_of_the_deck_builtin_theme_names_a_variant_in_deck_mode() {
    let h = start().await;
    let r = guarded(&h, "edit_theme", json!({ "theme": PAPIER, "name": "Vinci" }), true).await;
    let v = json_of(&r);
    assert_that!(v["name"]).is_equal_to(json!("Vinci"));
    assert_that!(v["adopted"]).is_equal_to(json!(true));
    assert_that!(deck_theme_name(&h).await).is_equal_to(json!("Vinci"));
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    assert_that!(json_of(&call(&c, "deck_outline", json!({})).await)["mode"]).is_equal_to(json!("deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_denied_changes_nothing() {
    let h = start().await;
    let r = guarded(&h, "edit_theme", json!({ "theme": PAPIER, "name": "Vinci" }), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(deck_theme_name(&h).await).is_equal_to(json!("Papier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_of_an_unknown_theme_is_refused_without_asking() {
    let h = start().await;
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let r = call(&c, "edit_theme", json!({ "theme": "00000000-0000-4000-8000-000000000001", "name": "Vinci" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

/// t-1100 : éditer un thème intégré demande toujours, puis crée une variante qui l'étend.
#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_of_a_builtin_asks_and_creates_a_variant() {
    let h = start().await;
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let before = json_of(&call(&c, "theme_tokens", json!({ "theme": "pro" })).await);
    let op = json!({ "op": "set_token", "path": ["color", "accent"], "value": "#112233" });
    let r = guarded(&h, "edit_theme", json!({ "theme": "pro", "name": "Marque", "ops": [op] }), true).await;
    let edited = json_of(&r);
    assert_that!(edited["namespace"].clone()).is_equal_to(json!("marque"));
    assert_that!(edited["tokens"]["color"]["accent"].clone()).is_equal_to(json!("#112233"));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    let listed = outline["themes"].as_array().unwrap().iter().find(|t| t["namespace"] == "marque").cloned();
    let listed = listed.unwrap_or_else(|| panic!("la variante est absente de la liste"));
    assert_that!((listed["extends"].clone(), listed["builtin"].clone())).is_equal_to((json!(["pro"]), json!(false)));
    // Le thème intégré n'a pas bougé.
    let after = json_of(&call(&c, "theme_tokens", json!({ "theme": "pro" })).await);
    assert_that!(after).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_of_a_builtin_denied_creates_no_variant() {
    let h = start().await;
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let r = guarded(&h, "edit_theme", json!({ "theme": "pro", "name": "Marque" }), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["themes"].as_array().unwrap().iter().any(|t| t["namespace"] == "marque")).is_false();
}
