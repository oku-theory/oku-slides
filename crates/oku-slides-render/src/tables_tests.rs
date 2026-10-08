use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{e}"));
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn deck(data: Json) -> Json {
    json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "t1", "kind": "table", "params": { "data": data } } ] } }] })
}

fn data_of(d: &Json) -> &Json {
    &d["slides"][0]["params"]["blocks"][0]["params"]["data"]
}

#[test]
fn sanitize_valid_table_writes_its_canonical_form() {
    let mut d = deck(json!({ "dataset": { "source": [["A"], [1]] } }));
    sanitize(&mut d, &registry());
    assert_that!(data_of(&d)["header-row"]).is_equal_to(json!(true));
}

#[test]
fn sanitize_malformed_table_clears_its_data() {
    let mut d = deck(json!({ "dataset": { "source": [5] } }));
    sanitize(&mut d, &registry());
    assert_that!(data_of(&d)).is_equal_to(&Json::Null);
}

#[test]
fn sanitize_oversized_table_clears_its_data() {
    let rows: Vec<Json> = (0..101).map(|i| json!([i])).collect();
    let mut d = deck(json!({ "dataset": { "source": rows } }));
    sanitize(&mut d, &registry());
    assert_that!(data_of(&d)).is_equal_to(&Json::Null);
}
