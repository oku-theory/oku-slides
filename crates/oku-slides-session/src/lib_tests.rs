use std::path::PathBuf;

use oku_slides_core::{Op, SlideField};
use serde_json::json;
use speculoos::prelude::*;

use super::*;

/// Un dossier neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn data_dir() -> Data {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Data(std::env::temp_dir().join(format!("oku-slides-lib-{}-{n}", std::process::id())))
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

fn session_with_fixture() -> Session {
    let mut s = session_in(&data_dir().0);
    s.open(&fixture_path()).unwrap();
    s
}

/// Nouveau titre de la troisième slide.
fn title_op(s: &Session, title: &str) -> Op {
    Op::SetSlideField { slide: s.deck().slides[2].id.clone(), field: SlideField::Title, value: json!(title) }
}

fn replica_ops(s: &Session) -> usize {
    s.doc.sources.replica().len_ops()
}

/// La vue du deck est bien la relecture du replica.
fn assert_view_matches_replica(s: &Session) {
    assert_that!(s.doc.sources.replica().deck().0).is_equal_to(s.doc.sources.deck());
}

#[test]
fn apply_with_render_failure_leaves_replica_untouched() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    let ops_before = replica_ops(&s);
    // Un jeton de couleur qui n'est pas une couleur fait échouer le rendu Typst.
    let bad = Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!(5) };
    let err = s.apply("Jeton", vec![bad]);
    assert_that!(err.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Render));
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(replica_ops(&s)).is_equal_to(ops_before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn apply_then_undo_then_redo_keeps_labels() {
    let mut s = session_with_fixture();
    s.apply("Renommer", vec![title_op(&s, "Nouveau")]).unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Renommer".to_owned()));
    s.undo().unwrap();
    assert_that!(s.snapshot().redo_label).is_equal_to(Some("Renommer".to_owned()));
    s.redo().unwrap();
    assert_that!(s.snapshot().can_undo).is_true();
}

#[test]
fn apply_undo_redo_view_matches_replica_read() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    s.apply("Renommer", vec![title_op(&s, "Nouveau")]).unwrap();
    assert_view_matches_replica(&s);
    let after = s.deck().clone();
    s.undo().unwrap();
    assert_view_matches_replica(&s);
    assert_that!(s.deck()).is_equal_to(&before);
    s.redo().unwrap();
    assert_view_matches_replica(&s);
    assert_that!(s.deck()).is_equal_to(&after);
}

/// Une ligne vide de l'éditeur arrive en `[{t:""}]` (`markdownToRuns`) ; des
/// runs voisins de même format, un `U+2029` et un titre de carte blanc
/// arrivent aussi de l'UI. La vue reste la relecture du replica.
#[test]
fn apply_non_canonical_text_then_undo_view_matches_replica_read() {
    use oku_slides_core::{Paragraph, Run};
    let mut s = session_with_fixture();
    let text = vec![
        Paragraph { runs: vec![Run::plain("")], list: None },
        Paragraph { runs: vec![Run::plain("ab"), Run::plain("cd\u{2029}e")], list: None },
    ];
    let op = Op::SetText { slide: "s-constat".into(), element: "b-points".into(), text };
    s.apply("Texte", vec![op]).unwrap();
    assert_view_matches_replica(&s);
    s.apply("Notes", vec![Op::SetNotes { slide: "s-constat".into(), notes: "  \n".into() }]).unwrap();
    assert_view_matches_replica(&s);
    s.undo().unwrap();
    assert_view_matches_replica(&s);
    s.undo().unwrap();
    assert_view_matches_replica(&s);
}

/// Bout en bout : modifier, enregistrer en `.oku-slides.pdf`, rouvrir dans
/// une autre session. Même deck, même identité, pas de pile d'annulation.
#[test]
fn save_then_reopen_keeps_deck_and_uid_without_undo() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open(&fixture_path()).unwrap();
    s.apply("Renommer", vec![title_op(&s, "Nouveau")]).unwrap();
    std::fs::create_dir_all(&data.0).unwrap();
    let file = data.0.join("essai.oku-slides.pdf");
    s.save(Some(&file)).unwrap();

    let mut back = session_in(&data.0);
    back.open(&file).unwrap();
    assert_that!(back.deck()).is_equal_to(s.deck());
    assert_that!(back.doc.sources.replica().deck_uid()).is_equal_to(s.doc.sources.replica().deck_uid());
    assert_that!(back.snapshot().can_undo).is_false();
    assert_that!(back.snapshot().notice).is_none();
}

/// Relancement : la reprise relit `deck.loro`, avec son identité ; la pile
/// d'annulation ne survit pas.
#[test]
fn resume_round_trip_keeps_deck_and_uid_without_undo() {
    let data = data_dir();
    let dir = data.0.join("resume");
    let mut a = session_in(&data.0);
    assert!(a.resume(dir.clone()));
    a.open(&fixture_path()).unwrap();
    a.handle(Request::Apply { label: "Renommer".into(), ops: vec![title_op(&a, "Nouveau")] }).unwrap();
    let (deck, uid) = (a.deck().clone(), a.doc.sources.replica().deck_uid());
    drop(a);
    assert_that!(dir.join("deck/deck.loro").exists()).is_true();

    let mut b = session_in(&data.0);
    assert!(b.resume(dir));
    assert_that!(b.deck()).is_equal_to(&deck);
    assert_that!(b.doc.sources.replica().deck_uid()).is_equal_to(uid);
    assert_that!(b.snapshot().can_undo).is_false();
    assert_that!(b.snapshot().dirty).is_true();
}

#[test]
fn changes_deck_read_requests_return_false() {
    let reads = [
        json!({ "cmd": "snapshot" }),
        json!({ "cmd": "slide", "id": "s-1" }),
        json!({ "cmd": "svg", "id": "s-1" }),
        json!({ "cmd": "schema" }),
        json!({ "cmd": "panel", "slide": "s-1" }),
        json!({ "cmd": "templates" }),
        json!({ "cmd": "template_sets" }),
        json!({ "cmd": "ui", "slide": null, "grid": false }),
        json!({ "cmd": "save_templates" }),
        json!({ "cmd": "delete_template_set", "set": "x" }),
    ];
    let changing: Vec<_> = reads
        .into_iter()
        .map(|r| serde_json::from_value::<Request>(r).unwrap())
        .filter(Request::changes_deck)
        .collect();
    assert_that!(changing).is_empty();
}

#[test]
fn changes_deck_edit_requests_return_true() {
    let edits = [
        json!({ "cmd": "apply", "label": "x", "ops": [] }),
        json!({ "cmd": "undo" }),
        json!({ "cmd": "redo" }),
        json!({ "cmd": "save", "path": null }),
        json!({ "cmd": "edit_templates", "set": "x" }),
        json!({ "cmd": "close_templates" }),
        json!({ "cmd": "merge", "path": "/tmp/x.oku-slides.pdf" }),
    ];
    let kept: Vec<_> = edits
        .into_iter()
        .map(|r| serde_json::from_value::<Request>(r).unwrap())
        .filter(|r| !r.changes_deck())
        .collect();
    assert_that!(kept).is_empty();
}
