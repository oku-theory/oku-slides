//! Chaque erreur de la session, provoquée pour de vrai, avec son message
//! dans la langue de l'utilisateur.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use slider_session::{
    Fonts, Localize, Localizer, Platform, Request, Session, SessionError, SessionErrorKind, Settings,
};
use speculoos::prelude::*;

fn workdir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("echo-session-errors-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("dossier de test : {e}"));
    dir
}

fn session(dir: &std::path::Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
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
    let err = session(&dir).open(&dir.join("absent.echo.pdf")).unwrap_err();
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
fn open_foreign_pdf_returns_not_echo() {
    let dir = workdir();
    let mut s = session(&dir);
    s.save(Some(&dir.join("vrai.pdf"))).unwrap();
    let bare = slider_pdf::strip_sources(&std::fs::read(dir.join("vrai.echo.pdf")).unwrap()).unwrap();
    std::fs::write(dir.join("autre.pdf"), bare).unwrap();
    let err = s.open(&dir.join("autre.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEcho);
    assert_that!(french(&err)).contains("n'a pas été créé par Écho");
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
