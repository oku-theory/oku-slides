use std::collections::HashSet;

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::test_registry::registry;

fn deck(slides: Value) -> Deck {
    serde_json::from_value(json!({ "version": 2, "slides": slides })).unwrap()
}

/// Une slide `content` dont le slot `slot` porte `elements`.
fn content(slot: &str, elements: Value) -> Deck {
    deck(json!([{ "id": "s", "kind": "content", "params": { slot: elements } }]))
}

fn run(d: &Deck) -> Vec<Issue> {
    validate(d, &registry(), None)
}

fn ids(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

fn message(id: &'static str, name: &'static str, value: &str) -> Message {
    Message::new(id).arg(name, value)
}

/// Registre de test : la base, plus des widgets aux contraintes que la base
/// n'a pas (`accepts`, `bind` inconnu, bornes numériques).
fn custom_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let mut probe: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let extra = json!([
        { "name": "gallery", "category": "element", "params": [
            { "name": "items", "type": "slot", "mode": "flow", "accepts": ["image"] }
        ] },
        { "name": "cover", "category": "slide", "params": [
            { "name": "brand", "type": "plain", "bind": "nope" },
            { "name": "title", "type": "plain", "bind": "title" }
        ] },
        { "name": "meter", "category": "element", "params": [
            { "name": "level", "type": "number", "min": 0, "max": 10 }
        ] }
    ]);
    probe["base"].as_array_mut().unwrap().extend(extra.as_array().unwrap().iter().cloned());
    WidgetRegistry::from_probe(probe).unwrap()
}

#[test]
fn validate_reference_decks_have_no_issue() {
    for path in ["/../../fixtures/roadmap.json", "/../oku-slides-render/tests/golden/widgets.json"] {
        let path = format!("{}{path}", env!("CARGO_MANIFEST_DIR"));
        let d = Deck::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let issues: Vec<String> = run(&d).iter().map(ToString::to_string).collect();
        assert_that!(issues).named(&path).is_empty();
    }
}

#[test]
fn validate_empty_and_duplicate_slide_ids_are_reported() {
    let d = deck(json!([{ "id": "", "kind": "title" }, { "id": "", "kind": "title" }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec![
        "issue-empty-slide-id",
        "issue-empty-slide-id",
        "issue-duplicate-slide-id",
    ]);
}

#[test]
fn validate_unknown_slide_widget_is_reported_and_its_params_are_not_checked() {
    let d = deck(json!([{ "id": "s", "kind": "nope", "params": { "x": 1 } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-widget"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-unknown-widget", "kind", "nope"));
}

#[test]
fn validate_unknown_element_widget_is_reported_and_its_slots_are_walked() {
    let d = content(
        "blocks",
        json!([{ "id": "u", "kind": "nope", "params": { "x": 1, "children": [{ "id": "", "kind": "text" }] } }]),
    );
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-widget", "issue-empty-element-id"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("u"));
}

#[rstest]
#[case::element_widget_as_slide(json!([{ "id": "s", "kind": "text" }]))]
#[case::slide_widget_as_element(json!([{ "id": "s", "kind": "content", "params": { "blocks": [{ "id": "e", "kind": "toc" }] } }]))]
fn validate_widget_of_another_category_is_reported(#[case] slides: Value) {
    let issues = run(&deck(slides));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-wrong-category"]);
}

#[test]
fn validate_unknown_param_is_reported_with_its_name() {
    let d = content("blocks", json!([{ "id": "e", "kind": "text", "params": { "colour": "red" } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-param"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-unknown-param", "param", "colour"));
}

#[rstest]
#[case::plain_given_a_number(json!({ "kind": "card", "params": { "title": 3 } }))]
#[case::rich_given_a_string(json!({ "kind": "text", "params": { "body": "texte" } }))]
#[case::rich_given_elements(json!({ "kind": "text", "params": { "body": [{ "id": "x", "kind": "text" }] } }))]
#[case::flag_given_a_string(json!({ "kind": "card", "params": { "highlight": "oui" } }))]
#[case::number_below_min(json!({ "kind": "stack", "params": { "layout": "rows", "count": 0 } }))]
#[case::number_given_a_string(json!({ "kind": "stack", "params": { "layout": "rows", "count": "2" } }))]
#[case::choice_given_a_number(json!({ "kind": "shape", "params": { "shape": 1 } }))]
#[case::asset_given_a_number(json!({ "kind": "image", "params": { "asset": 1 } }))]
#[case::slot_given_paragraphs(json!({ "kind": "canvas", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 }, "params": { "children": [{ "runs": [] }] } }))]
#[case::chart_given_a_string(json!({ "kind": "chart", "params": { "option": "barres" } }))]
#[case::chart_without_a_valid_series(json!({ "kind": "chart", "params": { "option": {
    "dataset": { "source": [["Poste", "A", "B"], ["Début", 1, 2]] },
    "series": [{ "type": "waterfall" }, { "type": "waterfall" }]
} } }))]
fn validate_param_of_the_wrong_shape_is_reported(#[case] element: Value) {
    let mut element = element;
    element["id"] = json!("e");
    let slot = if element.get("frame").is_some() { "elements" } else { "blocks" };
    let issues = run(&content(slot, json!([element])));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
}

#[rstest]
#[case::grid_given_a_number(json!(3))]
#[case::grid_unreadable(json!({ "columns": "x" }))]
fn validate_grid_param_of_the_wrong_shape_is_reported(#[case] grid: Value) {
    let d = deck(json!([{ "id": "s", "kind": "blank", "params": { "grid": grid } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-param-type", "param", "grid"));
}

#[rstest]
#[case::rich_as_empty_slot(json!({ "kind": "text", "params": { "body": [] } }))]
#[case::number_in_range(json!({ "kind": "stack", "params": { "layout": "rows", "count": 3 } }))]
#[case::valid_chart(json!({ "kind": "chart", "params": { "option": {
    "dataset": { "source": [["mois", "ventes"], ["jan", 3]] },
    "series": [{ "type": "bar", "encode": { "x": "mois", "y": "ventes" } }]
} } }))]
fn validate_param_of_the_right_shape_is_accepted(#[case] element: Value) {
    let mut element = element;
    element["id"] = json!("e");
    assert_that!(run(&content("blocks", json!([element])))).is_empty();
}

#[test]
fn validate_empty_array_for_a_slot_is_accepted() {
    let d = deck(json!([{ "id": "s", "kind": "blank", "params": { "elements": [] } }]));
    assert_that!(run(&d)).is_empty();
}

#[test]
fn validate_choice_out_of_its_values_is_reported() {
    let d = content("blocks", json!([{ "id": "e", "kind": "shape", "params": { "shape": "star" } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-choice-out-of-range"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-choice-out-of-range", "param", "shape"));
}

#[test]
fn validate_number_above_max_is_reported() {
    let d = content("blocks", json!([{ "id": "e", "kind": "meter", "params": { "level": 11 } }]));
    assert_that!(ids(&validate(&d, &custom_registry(), None))).is_equal_to(vec!["issue-param-type"]);
}

#[test]
fn validate_child_rejected_by_its_slot_is_reported() {
    let d = content(
        "blocks",
        json!([{ "id": "g", "kind": "gallery", "params": { "items": [
            { "id": "i", "kind": "image", "params": { "asset": "sha256:00" } },
            { "id": "t", "kind": "text" }
        ] } }]),
    );
    let issues = validate(&d, &custom_registry(), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-slot-rejects"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("t"));
    assert_that!(issues[0].message).is_equal_to(message("issue-slot-rejects", "kind", "text"));
}

#[test]
fn validate_empty_and_duplicate_element_ids_are_reported() {
    let d = content(
        "blocks",
        json!([
            { "id": "", "kind": "text" },
            { "id": "a", "kind": "text" },
            { "id": "st", "kind": "stack", "params": { "layout": "rows", "children": [{ "id": "a", "kind": "text" }] } }
        ]),
    );
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-empty-element-id", "issue-duplicate-element-id"]);
}

#[test]
fn validate_free_slot_child_without_frame_or_cell_is_reported() {
    let d = content(
        "elements",
        json!([
            { "id": "a", "kind": "text" },
            { "id": "c", "kind": "canvas", "cell": { "col": 0, "row": 0 }, "params": { "children": [{ "id": "b", "kind": "text" }] } }
        ]),
    );
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-free-without-frame", "issue-free-without-frame"]);
    let where_: Vec<_> = issues.iter().map(|i| i.element.clone()).collect();
    assert_that!(where_).is_equal_to(vec![Some("a".to_owned()), Some("b".to_owned())]);
}

#[rstest]
#[case::frame(json!({ "id": "e", "kind": "text", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 } }))]
#[case::cell(json!({ "id": "e", "kind": "text", "cell": { "col": 0, "row": 0 } }))]
fn validate_flow_slot_child_with_frame_or_cell_is_reported(#[case] element: Value) {
    let issues = run(&content("blocks", json!([element])));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-placed-in-flow"]);
}

#[test]
fn validate_zero_size_frame_is_reported() {
    let d = content("elements", json!([{ "id": "e", "kind": "text", "frame": { "x": 0, "y": 0, "w": 0, "h": 5 } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-empty-frame"]);
}

#[test]
fn validate_cell_outside_the_slide_grid_is_reported() {
    let d = deck(json!([{ "id": "s", "kind": "blank", "params": {
        "grid": { "columns": 2, "rows": 2 },
        "elements": [
            { "id": "in", "kind": "text", "cell": { "col": 1, "row": 1 } },
            { "id": "out", "kind": "text", "cell": { "col": 1, "row": 0, "cols": 2 } }
        ]
    } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-cell-outside-grid"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("out"));
}

#[test]
fn validate_cell_outside_the_default_grid_is_reported() {
    let d = content("elements", json!([{ "id": "e", "kind": "text", "cell": { "col": 12, "row": 0 } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-cell-outside-grid"]);
}

#[test]
fn validate_cell_inside_a_canvas_is_not_checked_against_the_slide_grid() {
    let d = content(
        "elements",
        json!([{ "id": "c", "kind": "canvas", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 }, "params": { "children": [
            { "id": "e", "kind": "text", "cell": { "col": 40, "row": 0 } }
        ] } }]),
    );
    assert_that!(run(&d)).is_empty();
}

#[test]
fn validate_missing_asset_is_reported_only_when_assets_are_given() {
    let d = content("blocks", json!([{ "id": "i", "kind": "image", "params": { "asset": "sha256:ab" } }]));
    assert_that!(run(&d)).is_empty();
    let issues = validate(&d, &registry(), Some(&HashSet::new()));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-missing-asset"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-missing-asset", "asset", "sha256:ab"));
    let present = HashSet::from(["sha256:ab".to_owned()]);
    assert_that!(validate(&d, &registry(), Some(&present))).is_empty();
}

#[rstest]
#[case::javascript(json!({ "t": "a", "link": "javascript:alert(1)" }), "issue-unsupported-link")]
#[case::highlight_zero(json!({ "t": "a", "highlight": 0 }), "issue-highlight-zero")]
fn validate_rich_text_problems_are_reported(#[case] run_: Value, #[case] id: &str) {
    let d = content("blocks", json!([{ "id": "c", "kind": "card", "params": { "body": [{ "runs": [run_] }] } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec![id]);
}

#[test]
fn validate_safe_links_are_accepted() {
    let body = json!([{ "runs": [
        { "t": "a", "link": "https://oku.test" }, { "t": "b", "link": "HTTP://x" }, { "t": "c", "link": "mailto:a@b.c" }
    ] }]);
    assert_that!(run(&content("blocks", json!([{ "id": "t", "kind": "text", "params": { "body": body } }]))))
        .is_empty();
}

#[test]
fn validate_unknown_bind_is_reported_once_per_widget() {
    let d = deck(json!([{ "id": "a", "kind": "cover" }, { "id": "b", "kind": "cover" }]));
    let issues = validate(&d, &custom_registry(), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-bind"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-unknown-bind", "param", "brand"));
}

#[test]
fn validate_bind_to_a_deck_extra_is_accepted() {
    let mut d = deck(json!([{ "id": "a", "kind": "cover" }]));
    d.params.extra.insert("nope".into(), json!("Marque"));
    assert_that!(validate(&d, &custom_registry(), None)).is_empty();
}

/// Chaîne de `n` canvas imbriqués (`c0` ⊃ `c1` ⊃ …), `c0` dans `elements`.
fn canvas_chain(n: usize) -> Deck {
    let frame = json!({ "x": 0, "y": 0, "w": 1, "h": 1 });
    let mut e = json!({ "id": format!("c{}", n - 1), "kind": "canvas", "frame": frame });
    for i in (0..n - 1).rev() {
        e = json!({ "id": format!("c{i}"), "kind": "canvas", "frame": frame, "params": { "children": [e] } });
    }
    content("elements", json!([e]))
}

/// Même borne que les ops : `canvas_chain(MAX_SLOT_DEPTH)` s'y construit,
/// un niveau de plus est refusé (`tests/ops.rs`).
#[test]
fn validate_elements_nested_to_the_depth_bound_are_accepted() {
    assert_that!(run(&canvas_chain(MAX_SLOT_DEPTH))).is_empty();
}

#[test]
fn validate_element_nested_one_level_past_the_bound_is_reported() {
    let issues = run(&canvas_chain(MAX_SLOT_DEPTH + 1));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-slot-too-deep"]);
    assert_that!(issues[0].element.clone()).is_equal_to(Some(format!("c{MAX_SLOT_DEPTH}")));
}

#[test]
fn validate_issue_display_names_slide_and_element() {
    let d = content("blocks", json!([{ "id": "", "kind": "text" }]));
    assert_that!(run(&d)[0].to_string()).is_equal_to("[s/] empty identifier".to_owned());
}
