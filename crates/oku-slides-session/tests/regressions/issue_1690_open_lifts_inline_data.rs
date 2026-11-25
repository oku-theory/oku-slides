//! t-1690: opening a JSON deck kept the inline data of its views. A table's `dataset` and an
//! unliftable chart's data stayed in the view while the replica dropped them, so the view and
//! the saved deck disagreed. Every inline data is now lifted into deck data, or stripped and
//! reported to the user.

use oku_slides_core::{ParamValue, kinds};
use oku_slides_session::Session;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{Data, data_dir, ok, req, session_in, snapshot_json};

/// A JSON deck of one slide holding `block`, opened in a fresh session.
fn open_with(block: &Value) -> (Data, Session) {
    let data = data_dir();
    let file = data.as_ref().join("deck.json");
    let deck = json!({ "version": 10, "slides": [{ "id": "s", "kind": kinds::essentiel::CONTENT,
        "params": { "blocks": [block] } }] });
    std::fs::write(&file, deck.to_string()).unwrap_or_else(|e| panic!("écriture : {e}"));
    let mut s = session_in(data.as_ref());
    ok(s.open(&file));
    (data, s)
}

/// The value of param `param` of element `element` of slide `s`.
fn param(s: &Session, element: &str, param: &str) -> Value {
    let found = s.deck().slides[0].locate(element).and_then(|l| l.element.params.get(param).cloned());
    let Some(ParamValue::Value(v)) = found else { panic!("pas de valeur {param} sur {element}") };
    v
}

fn table() -> Value {
    json!({ "id": "t", "kind": kinds::base::TABLE, "params": { "data": {
        "dataset": { "source": [["A", "B"], [1, 2]] }, "striped": true } } })
}

/// A chart naming a data next to its inline data: `prepare_ops` cannot lift it.
fn unliftable_chart() -> Value {
    json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": { "data": "absente",
        "xAxis": { "type": "category", "data": ["a", "b"] },
        "series": [{ "type": "bar", "data": [1, 2] }] } } })
}

#[test]
fn issue_1690_open_lifts_a_table_dataset_into_deck_data() {
    let (_data, s) = open_with(&table());
    let view = param(&s, "t", "data");
    assert_that!(s.deck().data.len()).is_equal_to(1);
    assert_that!(view.get("dataset")).is_none();
    assert_that!(view["data"].is_string()).is_true();
    assert_that!(view["striped"]).is_equal_to(json!(true));
}

#[test]
fn issue_1690_open_strips_and_reports_an_unliftable_chart() {
    let (_data, mut s) = open_with(&unliftable_chart());
    let view = param(&s, "g", "option");
    assert_that!(view["series"][0].get("data")).is_none();
    assert_that!(view["xAxis"].get("data")).is_none();
    assert_that!(view["data"]).is_equal_to(json!("absente"));
    let notice = snapshot_json(&mut s)["notice"].as_str().map(str::to_owned);
    assert_that!(notice).is_equal_to(Some(
        "Des parties de cette présentation n'ont pas pu être lues et ont été mises de côté (1).".to_owned(),
    ));
}

#[test]
fn issue_1690_reopened_deck_view_matches_its_replica() {
    let (data, mut s) = open_with(&json!({ "id": "c", "kind": kinds::base::STACK, "params": { "children": [
        table(), unliftable_chart()] } }));
    let saved = data.as_ref().join("deck.oku-slides.pdf");
    ok(s.save(Some(&saved)));
    let before = s.deck().clone();
    let mut reopened = session_in(data.as_ref());
    ok(reopened.open(&saved));
    assert_that!(reopened.deck().data).is_equal_to(&before.data);
    assert_that!(reopened.deck().slides).is_equal_to(&before.slides);
    // An edit checks, in debug, that the view equals the replica read back.
    let rename = json!({ "op": "set_param", "target": { "slide": "s", "element": "t" }, "param": "data",
        "value": param(&reopened, "t", "data") });
    ok(reopened.handle(req(json!({ "cmd": "apply", "label": "x", "ops": [rename] }))));
}
