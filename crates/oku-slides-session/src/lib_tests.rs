use std::path::PathBuf;

use oku_slides_core::{Op, Target as OpTarget, WidgetRegistry};
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
    Data(crate::test_support::fresh_dir("oku-slides-lib-").keep())
}

fn session_in(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
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
    let slide = s.deck().slides[2].id.clone();
    Op::SetParam { target: OpTarget { slide, element: None }, param: "title".into(), value: json!(title) }
}

/// Le registre de base figé (`fixtures/widgets/base-registry.json`).
fn base_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
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
    let target = OpTarget { slide: "s-constat".into(), element: Some("b-points".into()) };
    let op = Op::SetParam { target, param: "body".into(), value: serde_json::to_value(text).unwrap() };
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
        json!({ "cmd": "widgets" }),
        json!({ "cmd": "panel", "slide": "s-1" }),
        json!({ "cmd": "templates" }),
        json!({ "cmd": "template_sets" }),
        json!({ "cmd": "ui", "slide": null, "grid": false }),
        json!({ "cmd": "save_templates" }),
        json!({ "cmd": "delete_template_set", "set": "x" }),
        json!({ "cmd": "history_versions" }),
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
        json!({ "cmd": "history_open" }),
        json!({ "cmd": "history_seek", "index": 0 }),
        json!({ "cmd": "history_restore" }),
        json!({ "cmd": "history_close" }),
    ];
    let kept: Vec<_> = edits
        .into_iter()
        .map(|r| serde_json::from_value::<Request>(r).unwrap())
        .filter(|r| !r.changes_deck())
        .collect();
    assert_that!(kept).is_empty();
}

#[test]
fn slide_view_of_cover_lists_title_field_with_deck_title_text() {
    let s = session_with_fixture();
    let view = s.slide("s-title").unwrap();
    let title = view.targets.iter().find(|t| t.id == "@title").unwrap();
    assert_that!(title.kind).is_equal_to(TargetKind::Field("title"));
    assert_that!(title.param).is_equal_to(Some("title"));
    assert_that!(title.free).is_false();
    // Paramètre lié : le texte est celui du deck, tel que saisi.
    assert_that!(title.text.as_deref()).is_equal_to(Some("Feuille de route {{annee}}"));
    let subtitle = view.targets.iter().find(|t| t.id == "@subtitle").map(|t| t.text.as_deref());
    assert_that!(subtitle).is_equal_to(Some(Some("Trois chantiers pour {{client}}")));
}

#[test]
fn slide_view_of_cover_with_its_own_title_param_shows_the_bound_deck_title() {
    // Comme le rendu (`_args`) : le paramètre lié l'emporte sur celui de la slide.
    let data = data_dir();
    let mut deck: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(fixture_path()).unwrap()).unwrap();
    let slides = deck["slides"].as_array_mut().unwrap();
    let cover = slides.iter_mut().find(|s| s["id"] == "s-title").unwrap();
    cover["params"]["title"] = json!("Titre propre à la slide");
    let path = data.0.join("cover.json");
    std::fs::write(&path, deck.to_string()).unwrap();
    let mut s = session_in(&data.0);
    s.open(&path).unwrap();
    let view = s.slide("s-title").unwrap();
    let title = view.targets.iter().find(|t| t.id == "@title").map(|t| t.text.as_deref());
    assert_that!(title).is_equal_to(Some(Some("Feuille de route {{annee}}")));
}

#[test]
fn slide_view_of_content_lists_element_target_with_its_rich_param() {
    let s = session_with_fixture();
    let constat = s.slide("s-constat").unwrap();
    let text = constat.targets.iter().find(|t| t.id == "b-points").unwrap();
    assert_that!(text.kind).is_equal_to(TargetKind::Element("text"));
    assert_that!(text.param).is_equal_to(Some("body"));
    assert_that!(text.text.as_deref().unwrap_or_default()).is_not_equal_to("");
    let title = constat.targets.iter().find(|t| t.id == "@title").map(|t| t.text.as_deref());
    assert_that!(title).is_equal_to(Some(Some("Ce qui a changé en {{annee}}")));
    let priorites = s.slide("s-priorites").unwrap();
    let card = priorites.targets.iter().find(|t| t.id == "c1").unwrap();
    assert_that!(card.kind).is_equal_to(TargetKind::Element("card"));
    assert_that!(card.param).is_equal_to(Some("body"));
    // Un widget sans paramètre `rich` n'est pas éditable en place.
    let libre = s.slide("s-libre").unwrap();
    let shape = libre.targets.iter().find(|t| t.id == "e-fond").unwrap();
    assert_that!(shape.param).is_none();
    assert_that!(shape.free).is_true();
}

#[test]
fn apply_set_param_on_card_title_sets_that_title() {
    let mut s = session_with_fixture();
    let target = OpTarget { slide: "s-priorites".into(), element: Some("c1".into()) };
    s.apply("Titre", vec![Op::SetParam { target, param: "title".into(), value: json!("Nouveau titre") }]).unwrap();
    let card = s.deck().slide("s-priorites").and_then(|sl| sl.locate("c1")).map(|l| l.element.clone()).unwrap();
    assert_that!(card.params.text("title")).is_equal_to(Some("Nouveau titre"));
    assert_view_matches_replica(&s);
}

#[test]
fn handle_widgets_returns_the_base_registry() {
    let mut s = session_with_fixture();
    let reply: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    assert_that!(reply).is_equal_to(serde_json::to_value(base_registry()).unwrap());
}

#[test]
fn new_deck_is_valid_v2() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.new_deck().unwrap();
    let kinds: Vec<&str> = s.deck().slides.iter().map(|sl| sl.kind.as_str()).collect();
    assert_that!(kinds).is_equal_to(vec!["title", "content"]);
    let body = s.deck().slides[1].walk().map(|l| l.element.kind.as_str()).collect::<Vec<_>>();
    assert_that!(body).is_equal_to(vec!["text"]);
    assert_that!(oku_slides_core::validate(s.deck(), &base_registry(), None)).is_empty();
}

#[test]
fn no_chart_error_localizes_with_its_own_key() {
    let mut s = session_with_fixture();
    let err = s.set_chart_kind("x", "s-constat".into(), "b-points", oku_slides_chart::SeriesType::Pie).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoChart);
    assert_that!(err.localize(&Localizer::new(Some("en")))).is_equal_to("No chart here: b-points".to_owned());
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to("Pas de graphique ici : b-points".to_owned());
}

#[test]
fn snapshot_slide_summary_takes_bound_title_from_the_deck() {
    let s = session_with_fixture();
    let snap = s.snapshot();
    let cover = snap.slides.iter().find(|sl| sl.id == "s-title").unwrap();
    assert_that!(cover.kind).is_equal_to("title");
    // Le titre de la couverture est lié à celui du deck, variables remplacées.
    assert_that!(cover.title.clone()).is_equal_to(Some("Feuille de route 2027".to_owned()));
    let constat = snap.slides.iter().find(|sl| sl.id == "s-constat").unwrap();
    assert_that!(constat.title.clone()).is_equal_to(Some("Ce qui a changé en 2027".to_owned()));
}
