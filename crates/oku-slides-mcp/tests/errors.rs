//! Erreurs des outils : dites à l'agent dans la langue de l'utilisateur (`fr` ici), sans
//! chemin absolu ni chaîne d'erreur complète (t-403).

mod common;

use common::*;
use oku_slides_mcp::SurfaceAction;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn invalid_arguments_and_unknown_tool_are_said_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "slide", json!({ "id": 3 })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(
        "Les arguments de l'outil sont invalides (invalid type: integer `3`, expected a string) : vérifiez-les d'après son schéma."
            .to_owned(),
    );
    let r = call(&c, "nope", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Outil inconnu : nope.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relative_path_is_refused_without_echoing_it() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "save", json!({ "path": "decks/secret.oku-slides.pdf" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Le chemin doit être absolu.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_setting_off_the_whitelist_is_refused_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_setting", json!({ "key": "paths.data", "value": "/x" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Le réglage paths.data ne peut pas être changé par l'agent.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_that_cannot_be_written_is_said_without_its_absolute_path() {
    let h = start().await;
    let dir = temp_dir("oku-slides-errors-");
    // Un fichier à la place du dossier : l'écriture échoue.
    let blocker = dir.path().join("pas-un-dossier");
    std::fs::write(&blocker, b"x").unwrap();
    let path = blocker.join("deck.oku-slides.pdf");
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn(async move { call(&c, "save", json!({ "path": path.to_string_lossy() })).await });
    let surface = next_surface(&mut rx).await;
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(true));
    h.hub.act(SurfaceAction { surface: surface.id, name: "confirm".into(), context }).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
    let text = text_of(&r);
    assert_that!(text.as_str()).is_equal_to("Le fichier n'a pas pu être lu ou écrit.");
    assert_that!(text.contains(dir.path().to_string_lossy().as_ref())).is_false();
    assert_that!(text.contains('/')).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_argument_is_named_so_the_agent_can_fix_its_call() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "slide", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let text = text_of(&r);
    // En français (la langue de l'utilisateur), le détail de serde nomme le champ manquant.
    assert_that!(text.as_str()).starts_with("Les arguments de l'outil sont invalides");
    assert_that!(text.as_str()).contains("missing field `id`");
}
