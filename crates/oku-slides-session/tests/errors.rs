//! Chaque erreur de la session, provoquée pour de vrai, avec son message
//! dans la langue de l'utilisateur.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use oku_slides_session::{
    Fonts, Localize, Localizer, Platform, Request, Session, SessionError, SessionErrorKind, Settings,
};
use speculoos::prelude::*;

fn workdir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("oku-slides-session-errors-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("dossier de test : {e}"));
    dir
}

fn session(dir: &std::path::Path) -> Session {
    session_in(dir, "fr")
}

/// Une session dans une langue donnée.
fn session_in(dir: &std::path::Path, locale: &str) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
    // Les messages attendus sont dans cette langue, quelle que soit celle du système.
    settings.i18n.locale = Some(locale.into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

fn french(e: &SessionError) -> String {
    e.localize(&Localizer::new(Some("fr")))
}

fn request(s: &mut Session, json: serde_json::Value) -> Result<String, SessionError> {
    s.handle(serde_json::from_value::<Request>(json).unwrap_or_else(|e| panic!("requête : {e}")))
}

#[test]
fn open_missing_file_returns_file_error() {
    let dir = workdir();
    let err = session(&dir).open(&dir.join("absent.oku-slides.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::File);
    assert_that!(french(&err)).starts_with("Impossible de lire");
}

#[test]
fn open_invalid_json_returns_invalid_document() {
    let dir = workdir();
    let file = dir.join("deck.json");
    std::fs::write(&file, "{ pas du json").unwrap();
    let err = session(&dir).open(&file).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidDocument);
    assert_that!(french(&err)).is_equal_to("Deck JSON invalide".to_owned());
}

#[test]
fn open_foreign_pdf_returns_not_oku_slides() {
    let dir = workdir();
    let mut s = session(&dir);
    s.save(Some(&dir.join("vrai.pdf"))).unwrap();
    let bare = oku_slides_pdf::strip_sources(&std::fs::read(dir.join("vrai.oku-slides.pdf")).unwrap()).unwrap();
    std::fs::write(dir.join("autre.pdf"), bare).unwrap();
    let err = s.open(&dir.join("autre.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotOkuSlides);
    assert_that!(french(&err)).contains("n'a pas été créé par Oku Slides");
}

#[test]
fn open_damaged_pdf_returns_invalid_document() {
    let dir = workdir();
    std::fs::write(dir.join("abime.pdf"), b"%PDF-1.7 rien ici").unwrap();
    let err = session(&dir).open(&dir.join("abime.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidDocument);
    assert_that!(french(&err)).is_equal_to("PDF illisible".to_owned());
}

#[test]
fn save_new_deck_without_path_returns_no_path() {
    let dir = workdir();
    let err = session(&dir).save(None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoPath);
    assert_that!(french(&err)).is_equal_to("Aucun fichier : choisissez où enregistrer".to_owned());
}

#[test]
fn save_into_a_file_used_as_folder_returns_file_error() {
    let dir = workdir();
    std::fs::write(dir.join("fichier"), "x").unwrap();
    let err = session(&dir).save(Some(&dir.join("fichier").join("deck.pdf"))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::File);
    assert_that!(french(&err)).starts_with("Impossible d'écrire");
}

#[test]
fn svg_of_missing_slide_returns_no_slide() {
    let dir = workdir();
    let err = session(&dir).svg("absente").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn edit_unknown_template_set_returns_no_template_set() {
    let dir = workdir();
    let err = session(&dir).edit_templates("inconnu").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoTemplateSet);
    assert_that!(french(&err)).is_equal_to("Ensemble de modèles introuvable : inconnu".to_owned());
}

#[test]
fn save_templates_while_not_editing_returns_not_editing() {
    let dir = workdir();
    let err = session(&dir).save_templates(false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingTemplates);
    assert_that!(french(&err)).is_equal_to("Pas de modèles en cours d'édition".to_owned());
}

#[test]
fn delete_builtin_template_set_is_refused() {
    let dir = workdir();
    let err =
        request(&mut session(&dir), serde_json::json!({"cmd": "delete_template_set", "set": "essentiel"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::BuiltinTemplateSet);
    assert_that!(french(&err)).is_equal_to("Un ensemble intégré ne se supprime pas".to_owned());
}

#[test]
fn open_while_editing_templates_is_refused() {
    let dir = workdir();
    let mut s = session(&dir);
    s.edit_templates("essentiel").unwrap();
    let err = request(&mut s, serde_json::json!({"cmd": "new"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingTemplates);
    assert_that!(french(&err)).is_equal_to("Terminez d'abord l'édition des modèles".to_owned());
}

#[test]
fn handle_queries_answer_in_json() {
    let dir = workdir();
    let mut s = session(&dir);
    let schema = request(&mut s, serde_json::json!({"cmd": "schema"})).unwrap();
    assert_that!(schema).contains("set_text");
    let sets = request(&mut s, serde_json::json!({"cmd": "template_sets"})).unwrap();
    assert_that!(sets).contains("\"default\":true");
    let templates = request(&mut s, serde_json::json!({"cmd": "templates", "set": "grilles"})).unwrap();
    assert_that!(templates).contains("\"svg\":\"<svg");
    let undone = request(&mut s, serde_json::json!({"cmd": "undo"})).unwrap();
    assert_that!(undone).contains("\"can_undo\":false");
    let redone = request(&mut s, serde_json::json!({"cmd": "redo"})).unwrap();
    assert_that!(redone).contains("\"can_redo\":false");
}

#[test]
fn new_deck_is_written_in_the_user_language() {
    let dir = workdir();
    let mut s = session(&dir);
    s.new_deck().unwrap();
    let snap = s.snapshot();
    assert_that!(snap.title.as_deref()).is_equal_to(Some("Nouvelle présentation"));
    assert_that!(snap.slides[1].title.as_deref()).is_equal_to(Some("Titre de la slide"));
}

#[test]
fn apply_inline_chart_stores_it_with_a_localized_dataset() {
    let dir = workdir();
    let mut s = session(&dir);
    s.new_deck().unwrap();
    let slide = s.snapshot().slides[1].id.to_string();
    let add = serde_json::json!({ "cmd": "apply", "label": "graphique", "ops": [{
        "op": "add_element", "slide": slide, "zone": "blocks",
        "element": { "id": "g", "kind": "chart", "option": {
            "xAxis": { "type": "category", "data": ["1", "2"] },
            "series": [{ "type": "bar", "data": [3, 5] }] } } }] });
    request(&mut s, add).unwrap();
    let json = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    assert_that!(json).contains(r#"["Catégorie","Série 1"]"#);
    // Annuler retire le graphique.
    request(&mut s, serde_json::json!({ "cmd": "undo" })).unwrap();
    let json = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    assert_that!(json).does_not_contain("\"chart\"");
    // Rétablir le rend sous sa forme canonique.
    request(&mut s, serde_json::json!({ "cmd": "redo" })).unwrap();
    let json = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    assert_that!(json).contains(r#"["Catégorie","Série 1"]"#);
}

#[test]
fn open_deck_with_inline_chart_makes_its_data_editable() {
    let dir = workdir();
    let file = dir.join("deck.json");
    let deck = serde_json::json!({ "slides": [{ "id": "s", "kind": "content", "blocks": [{
        "id": "g", "kind": "chart", "option": {
            "xAxis": { "type": "category", "data": ["a", "b"] },
            "series": [{ "type": "bar", "name": "Ventes", "data": [1, 2] }] } }] }] });
    std::fs::write(&file, deck.to_string()).unwrap();
    let mut s = session(&dir);
    s.open(&file).unwrap();
    let edit = serde_json::json!({ "cmd": "apply", "label": "cellule", "ops": [{
        "op": "set_chart_data", "slide": "s", "element": "g",
        "change": { "op": "set_cell", "row": 1, "col": 1, "value": 7 } }] });
    request(&mut s, edit).unwrap();
    let json = request(&mut s, serde_json::json!({ "cmd": "slide", "id": "s" })).unwrap();
    assert_that!(json).contains(r#"["a",7.0]"#);
}

#[test]
fn messages_give_the_ui_its_language_and_fluent_resources() {
    let dir = workdir();
    let json = request(&mut session(&dir), serde_json::json!({ "cmd": "messages" })).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let locale = v["locale"].as_str().unwrap();
    let resources: Vec<&str> = v["resources"].as_array().unwrap().iter().filter_map(|r| r.as_str()).collect();
    assert_that!(resources).is_equal_to(Localizer::new(Some(locale)).ui_messages());
}

#[test]
fn chart_kind_converts_the_data_and_undoes_in_one_step() {
    let dir = workdir();
    let mut s = session(&dir);
    s.new_deck().unwrap();
    let slide = s.snapshot().slides[1].id.to_string();
    let add = serde_json::json!({ "cmd": "apply", "label": "graphique", "ops": [{
        "op": "add_element", "slide": slide, "zone": "blocks",
        "element": { "id": "g", "kind": "chart", "option": {
            "xAxis": { "type": "category", "data": ["a", "b"] },
            "series": [{ "type": "bar", "data": [3, 5] }] } } }] });
    request(&mut s, add).unwrap();
    let before = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    let to_scatter =
        serde_json::json!({ "cmd": "chart_kind", "label": "nuage", "slide": slide, "element": "g", "kind": "scatter" });
    request(&mut s, to_scatter).unwrap();
    let after = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    assert_that!(after).contains(r#"["Série 1 (x)","Série 1"]"#);
    assert_that!(after).contains(r#""type":"scatter""#);
    request(&mut s, serde_json::json!({ "cmd": "undo" })).unwrap();
    let undone = request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap();
    assert_that!(undone).is_equal_to(before);
}

#[test]
fn chart_kind_on_a_text_or_missing_slide_is_refused() {
    let dir = workdir();
    let mut s = session(&dir);
    s.new_deck().unwrap();
    let slide = s.snapshot().slides[1].id.to_string();
    let text =
        serde_json::json!({ "cmd": "chart_kind", "label": "x", "slide": slide, "element": "b-1", "kind": "pie" });
    let err = request(&mut s, text).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoChart);
    assert_that!(french(&err)).is_equal_to("b-1 n'est pas un graphique".to_owned());
    let missing =
        serde_json::json!({ "cmd": "chart_kind", "label": "x", "slide": "absente", "element": "g", "kind": "pie" });
    assert_that!(request(&mut s, missing).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn new_deck_in_arabic_is_written_right_to_left() {
    let dir = workdir();
    let mut s = session_in(&dir, "ar");
    s.new_deck().unwrap();
    assert_that!(s.snapshot().slides.len()).is_equal_to(2);
    let slide = s.snapshot().slides[1].id.to_string();
    let view: serde_json::Value =
        serde_json::from_str(&request(&mut s, serde_json::json!({ "cmd": "slide", "id": slide })).unwrap()).unwrap();
    assert_that!(view["slide"].is_object()).is_true();
    assert_that!(s.deck().params.lang.as_deref()).is_equal_to(Some("ar"));
    let aligns: Vec<&str> =
        view["targets"].as_array().unwrap().iter().filter_map(|t| t["style"]["align"].as_str()).collect();
    assert_that!(aligns.is_empty()).is_false();
    assert_that!(aligns.iter().all(|a| *a == "right")).is_true();
}

#[test]
fn new_deck_in_chinese_uses_the_language_code_typst_knows() {
    let dir = workdir();
    let mut s = session_in(&dir, "zh-TW");
    s.new_deck().unwrap();
    assert_that!(s.deck().params.lang.as_deref()).is_equal_to(Some("zh"));
}
