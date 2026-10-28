use oku_slides_core::{Op, Target as OpTarget, WidgetRegistry};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::lib_tests_support::{data_dir, fixture_path, session_in, session_with_fixture};

/// Nouveau titre de la troisième slide.
fn title_op(s: &Session, title: &str) -> Op {
    let slide = s.deck().slides[2].id.clone();
    Op::SetParam { target: OpTarget { slide, element: None }, param: "title".into(), value: json!(title) }
}

/// Le registre de base figé (`fixtures/widgets/builtin-registry.json`).
fn base_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
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
    // Une taille qui n'est pas une taille fait échouer le rendu Typst (seules
    // les couleurs des surcharges sont vérifiées avant, t-194).
    let bad = Op::SetToken { path: vec!["size".into(), "body".into()], value: json!(true) };
    let err = s.apply("Jeton", vec![bad], None);
    assert_that!(err.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Render));
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(replica_ops(&s)).is_equal_to(ops_before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn apply_then_undo_then_redo_keeps_labels() {
    let mut s = session_with_fixture();
    s.apply("Renommer", vec![title_op(&s, "Nouveau")], None).unwrap();
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
    s.apply("Renommer", vec![title_op(&s, "Nouveau")], None).unwrap();
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
    s.apply("Texte", vec![op], None).unwrap();
    assert_view_matches_replica(&s);
    s.apply("Notes", vec![Op::SetNotes { slide: "s-constat".into(), notes: "  \n".into() }], None).unwrap();
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
    s.apply("Renommer", vec![title_op(&s, "Nouveau")], None).unwrap();
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
    a.handle(Request::Apply { label: "Renommer".into(), ops: vec![title_op(&a, "Nouveau")], merge: None }).unwrap();
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
        json!({ "cmd": "layouts" }),
        json!({ "cmd": "packs", "kind": "layouts" }),
        json!({ "cmd": "ui", "slide": null, "grid": false, "ghosts": true }),
        json!({ "cmd": "delete_pack", "pack": uuid::Uuid::new_v4() }),
        json!({ "cmd": "history_versions" }),
        json!({ "cmd": "widget_status" }),
        json!({ "cmd": "widget_source", "name": "kpi" }),
        json!({ "cmd": "preview_widget", "name": "kpi" }),
        json!({ "cmd": "host_settings", "widget": null }),
        json!({ "cmd": "data_panel", "slide": "s-1", "element": "e" }),
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
        json!({ "cmd": "save_pack" }),
        json!({ "cmd": "merge", "path": "/tmp/x.oku-slides.pdf" }),
        json!({ "cmd": "history_open" }),
        json!({ "cmd": "history_seek", "index": 0 }),
        json!({ "cmd": "history_restore" }),
        json!({ "cmd": "history_close" }),
        json!({ "cmd": "set_widget", "name": "kpi", "source": "x" }),
        json!({ "cmd": "remove_widget", "name": "kpi" }),
        json!({ "cmd": "rename_widget", "from": "kpi", "to": "k" }),
        json!({ "cmd": "set_example_value", "name": "kpi", "param": "p", "value": 1 }),
        json!({ "cmd": "data_edit", "slide": "s-1", "element": "e", "edit": { "edit": "stack", "stacked": true } }),
    ];
    let kept: Vec<_> = edits
        .into_iter()
        .map(|r| serde_json::from_value::<Request>(r).unwrap())
        .filter(|r| !r.changes_deck())
        .collect();
    assert_that!(kept).is_empty();
}

#[test]
fn edits_data_edit_returns_true_data_panel_false() {
    let classify = |v: serde_json::Value| serde_json::from_value::<Request>(v).unwrap().edits();
    let edit =
        json!({ "cmd": "data_edit", "slide": "s-1", "element": "e", "edit": { "edit": "kind", "kind": "line" } });
    assert_that!(classify(edit)).is_true();
    assert_that!(classify(json!({ "cmd": "data_panel", "slide": "s-1", "element": "e" }))).is_false();
}

#[test]
fn is_widget_edit_widget_writes_return_true_and_others_false() {
    let request = |v| serde_json::from_value::<Request>(v).unwrap();
    let writes = [
        json!({ "cmd": "set_widget", "name": "kpi", "source": "x" }),
        json!({ "cmd": "remove_widget", "name": "kpi" }),
        json!({ "cmd": "rename_widget", "from": "kpi", "to": "k" }),
        json!({ "cmd": "set_example_value", "name": "kpi", "param": "p", "value": 1 }),
    ];
    assert_that!(writes.into_iter().map(request).all(|r| r.is_widget_edit())).is_true();
    let others = [json!({ "cmd": "widget_status" }), json!({ "cmd": "undo" }), json!({ "cmd": "save_pack" })];
    assert_that!(others.into_iter().map(request).any(|r| r.is_widget_edit())).is_false();
}

#[test]
fn ends_widget_session_spares_save_and_widget_writes() {
    let request = |v| serde_json::from_value::<Request>(v).unwrap();
    let kept = [
        json!({ "cmd": "save" }),
        json!({ "cmd": "set_widget", "name": "kpi", "source": "x" }),
        json!({ "cmd": "snapshot" }),
    ];
    assert_that!(kept.into_iter().map(request).any(|r| r.ends_widget_session())).is_false();
    let ending = [json!({ "cmd": "undo" }), json!({ "cmd": "merge", "path": "x" })];
    assert_that!(ending.into_iter().map(request).all(|r| r.ends_widget_session())).is_true();
}

#[test]
fn apply_set_param_on_card_title_sets_that_title() {
    let mut s = session_with_fixture();
    let target = OpTarget { slide: "s-priorites".into(), element: Some("c1".into()) };
    s.apply("Titre", vec![Op::SetParam { target, param: "title".into(), value: json!("Nouveau titre") }], None)
        .unwrap();
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
    assert_that!(kinds)
        .is_equal_to(vec![oku_slides_core::kinds::essentiel::TITLE, oku_slides_core::kinds::essentiel::CONTENT]);
    let body = s.deck().slides[1].walk().map(|l| l.element.kind.as_str()).collect::<Vec<_>>();
    assert_that!(body).is_equal_to(vec![oku_slides_core::kinds::base::TEXT]);
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
    assert_that!(cover.kind).is_equal_to(oku_slides_core::kinds::essentiel::TITLE);
    // Le titre de la couverture est lié à celui du deck, variables remplacées.
    assert_that!(cover.title.clone()).is_equal_to(Some("Feuille de route 2027".to_owned()));
    let constat = snap.slides.iter().find(|sl| sl.id == "s-constat").unwrap();
    assert_that!(constat.title.clone()).is_equal_to(Some("Ce qui a changé en 2027".to_owned()));
}

#[test]
fn handle_apply_with_same_merge_key_makes_one_undo_step() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    let apply = |s: &mut Session, title: &str| {
        let op = serde_json::to_value(title_op(s, title)).unwrap();
        let req = json!({ "cmd": "apply", "label": "Modifier la slide", "ops": [op], "merge": "k" });
        s.handle(serde_json::from_value(req).unwrap()).unwrap();
    };
    apply(&mut s, "Un");
    apply(&mut s, "Deux");
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
}

#[test]
fn snapshot_lists_widget_pins_by_namespace() {
    let s = session_with_fixture();
    let snap = serde_json::to_value(s.snapshot()).unwrap();
    let pin = &snap["widgets"]["base"];
    assert_that!(pin["id"].as_str()).is_equal_to(Some(crate::BASE_PACK.to_string().as_str()));
    assert_that!(pin["version"].as_str().unwrap_or_default()).starts_with("sha256:");
    assert_that!(snap.get("templates").is_none()).is_true();
}

#[test]
fn snapshot_host_params_maps_every_base_host() {
    use oku_slides_core::{HostType, kinds};
    let s = session_with_fixture();
    let expected = std::collections::BTreeMap::from([
        (kinds::base::CHART.to_owned(), std::collections::BTreeMap::from([(HostType::Chart, "option".to_owned())])),
        (kinds::base::SHAPE.to_owned(), std::collections::BTreeMap::from([(HostType::Path, "path".to_owned())])),
        (kinds::base::TABLE.to_owned(), std::collections::BTreeMap::from([(HostType::Table, "data".to_owned())])),
    ]);
    assert_that!(s.snapshot().host_params).is_equal_to(expected);
}
