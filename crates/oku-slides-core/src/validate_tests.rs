use std::collections::HashSet;

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::test_registry::registry;

fn deck(slides: Value) -> Deck {
    serde_json::from_value(json!({ "version": 9, "slides": slides })).unwrap()
}

/// Une slide `content` dont le slot `slot` porte `elements`.
fn content(slot: &str, elements: Value) -> Deck {
    deck(json!([{ "id": "s", "kind": "essentiel/content", "params": { slot: elements } }]))
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

/// Registre de test : les packs intégrés, plus des widgets aux contraintes que la base
/// n'a pas (`accepts`, `bind` inconnu, bornes numériques).
fn custom_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let mut probe: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let elements = json!([
        { "name": "gallery", "category": "element", "params": [
            { "name": "items", "type": "slot", "mode": "flow", "accepts": ["base/image"] }
        ] },
        { "name": "meter", "category": "element", "params": [
            { "name": "level", "type": "number", "min": 0, "max": 10 }
        ] }
    ]);
    let slides = json!([
        { "name": "cover", "category": "slide", "params": [
            { "name": "brand", "type": "plain", "bind": "nope" },
            { "name": "title", "type": "plain", "bind": "title" }
        ] }
    ]);
    let packs = probe["packs"].as_array_mut().unwrap();
    packs.push(json!({ "namespace": "t", "kind": "widgets", "widgets": elements }));
    packs.push(json!({ "namespace": "t", "kind": "layouts", "widgets": slides }));
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
    let d = deck(json!([{ "id": "", "kind": "essentiel/title" }, { "id": "", "kind": "essentiel/title" }]));
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
        json!([{ "id": "u", "kind": "nope", "params": { "x": 1, "children": [{ "id": "", "kind": "base/text" }] } }]),
    );
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-widget", "issue-empty-element-id"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("u"));
}

#[rstest]
#[case::element_widget_as_slide(json!([{ "id": "s", "kind": "base/text" }]))]
#[case::slide_widget_as_element(json!([{ "id": "s", "kind": "essentiel/content", "params": { "blocks": [{ "id": "e", "kind": "essentiel/toc" }] } }]))]
fn validate_widget_of_another_category_is_reported(#[case] slides: Value) {
    let issues = run(&deck(slides));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-wrong-category"]);
}

#[test]
fn validate_unknown_param_is_reported_with_its_name() {
    let d = content("blocks", json!([{ "id": "e", "kind": "base/text", "params": { "colour": "red" } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-param"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-unknown-param", "param", "colour"));
}

#[rstest]
#[case::plain_given_a_number(json!({ "kind": "base/card", "params": { "title": 3 } }))]
#[case::rich_given_a_string(json!({ "kind": "base/text", "params": { "body": "texte" } }))]
#[case::rich_given_elements(json!({ "kind": "base/text", "params": { "body": [{ "id": "x", "kind": "base/text" }] } }))]
#[case::flag_given_a_string(json!({ "kind": "base/card", "params": { "highlight": "oui" } }))]
#[case::number_below_min(json!({ "kind": "base/stack", "params": { "layout": "rows", "count": 0 } }))]
#[case::number_given_a_string(json!({ "kind": "base/stack", "params": { "layout": "rows", "count": "2" } }))]
#[case::choice_given_a_number(json!({ "kind": "base/shape", "params": { "shape": 1 } }))]
#[case::asset_given_a_number(json!({ "kind": "base/image", "params": { "asset": 1 } }))]
#[case::slot_given_paragraphs(json!({ "kind": "test/frame-box", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 }, "params": { "children": [{ "runs": [] }] } }))]
#[case::chart_given_a_string(json!({ "kind": "base/chart", "params": { "option": "barres" } }))]
#[case::chart_without_a_valid_series(json!({ "kind": "base/chart", "params": { "option": {
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
    let d = deck(json!([{ "id": "s", "kind": "essentiel/blank", "params": { "grid": grid } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-param-type", "param", "grid"));
}

#[rstest]
#[case::rich_as_empty_slot(json!({ "kind": "base/text", "params": { "body": [] } }))]
#[case::number_in_range(json!({ "kind": "base/stack", "params": { "layout": "rows", "count": 3 } }))]
#[case::valid_chart(json!({ "kind": "base/chart", "params": { "option": {
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
    let d = deck(json!([{ "id": "s", "kind": "essentiel/blank", "params": { "elements": [] } }]));
    assert_that!(run(&d)).is_empty();
}

#[test]
fn validate_choice_out_of_its_values_is_reported() {
    let d = content("blocks", json!([{ "id": "e", "kind": "base/shape", "params": { "shape": "star" } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-choice-out-of-range"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-choice-out-of-range", "param", "shape"));
}

#[test]
fn validate_number_above_max_is_reported() {
    let d = content("blocks", json!([{ "id": "e", "kind": "t/meter", "params": { "level": 11 } }]));
    assert_that!(ids(&validate(&d, &custom_registry(), None))).is_equal_to(vec!["issue-param-type"]);
}

#[test]
fn validate_child_rejected_by_its_slot_is_reported() {
    let d = content(
        "blocks",
        json!([{ "id": "g", "kind": "t/gallery", "params": { "items": [
            { "id": "i", "kind": "base/image", "params": { "asset": "sha256:00" } },
            { "id": "t", "kind": "base/text" }
        ] } }]),
    );
    let issues = validate(&d, &custom_registry(), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-slot-rejects"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("t"));
    assert_that!(issues[0].message).is_equal_to(message("issue-slot-rejects", "kind", "base/text"));
}

#[test]
fn validate_empty_and_duplicate_element_ids_are_reported() {
    let d = content(
        "blocks",
        json!([
            { "id": "", "kind": "base/text" },
            { "id": "a", "kind": "base/text" },
            { "id": "st", "kind": "base/stack", "params": { "layout": "rows", "children": [{ "id": "a", "kind": "base/text" }] } }
        ]),
    );
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-empty-element-id", "issue-duplicate-element-id"]);
}

#[test]
fn validate_free_slot_child_without_frame_or_cell_is_reported() {
    let d = content(
        "elements",
        json!([
            { "id": "a", "kind": "base/text" },
            { "id": "c", "kind": "test/frame-box", "cell": { "col": 0, "row": 0 }, "params": { "children": [{ "id": "b", "kind": "base/text" }] } }
        ]),
    );
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-free-without-frame", "issue-free-without-frame"]);
    let where_: Vec<_> = issues.iter().map(|i| i.element.clone()).collect();
    assert_that!(where_).is_equal_to(vec![Some("a".to_owned()), Some("b".to_owned())]);
}

#[rstest]
#[case::frame(json!({ "id": "e", "kind": "base/text", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 } }))]
#[case::cell(json!({ "id": "e", "kind": "base/text", "cell": { "col": 0, "row": 0 } }))]
fn validate_flow_slot_child_with_frame_or_cell_is_reported(#[case] element: Value) {
    let issues = run(&content("blocks", json!([element])));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-placed-in-flow"]);
}

#[test]
fn validate_zero_size_frame_is_reported() {
    let d =
        content("elements", json!([{ "id": "e", "kind": "base/text", "frame": { "x": 0, "y": 0, "w": 0, "h": 5 } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-empty-frame"]);
}

#[rstest]
#[case::valign(json!({ "valign": "centre" }), "style.valign", "valign")]
#[case::inset(json!({ "inset": [0, -2, 0, 0] }), "style.inset", "out-of-bounds")]
#[case::tracking(json!({ "tracking": 1e6 }), "style.tracking", "out-of-bounds")]
#[case::spacing(json!({ "spacing": -1 }), "style.spacing", "out-of-bounds")]
#[case::indent(json!({ "indent": [-1, 0] }), "style.indent", "out-of-bounds")]
fn validate_style_out_of_its_domain_is_reported_with_its_reason(
    #[case] style: Value,
    #[case] field: &str,
    #[case] reason: &str,
) {
    let d = content("blocks", json!([{ "id": "e", "kind": "base/text", "style": style }]));
    let issues = run(&d);
    let expected = Message::new("issue-invalid-style").arg("field", field).arg("reason", reason);
    assert_that!(issues.iter().map(|i| i.message.clone()).collect::<Vec<_>>()).is_equal_to(vec![expected]);
}

/// La raison se dit dans la langue de l'utilisateur, par sa propre clé.
#[rstest]
#[case::valign("valign", "top, middle ou bottom est attendu")]
#[case::bounds("out-of-bounds", "hors de")]
fn validate_invalid_style_reason_is_localized(#[case] reason: &str, #[case] expected: &str) {
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let msg = Message::new("issue-invalid-style").arg("field", "style.inset").arg("reason", reason).localize(&fr);
    assert_that!(msg).contains("style.inset");
    assert_that!(msg).contains(expected);
}

#[test]
fn validate_cell_outside_the_slide_grid_is_reported() {
    let d = deck(json!([{ "id": "s", "kind": "essentiel/blank", "params": {
        "grid": { "columns": 2, "rows": 2 },
        "elements": [
            { "id": "in", "kind": "base/text", "cell": { "col": 1, "row": 1 } },
            { "id": "out", "kind": "base/text", "cell": { "col": 1, "row": 0, "cols": 2 } }
        ]
    } }]));
    let issues = run(&d);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-cell-outside-grid"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("out"));
}

#[test]
fn validate_cell_outside_the_default_grid_is_reported() {
    let d = content("elements", json!([{ "id": "e", "kind": "base/text", "cell": { "col": 12, "row": 0 } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec!["issue-cell-outside-grid"]);
}

#[test]
fn validate_cell_inside_a_frame_box_is_not_checked_against_the_slide_grid() {
    let d = content(
        "elements",
        json!([{ "id": "c", "kind": "test/frame-box", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 }, "params": { "children": [
            { "id": "e", "kind": "base/text", "cell": { "col": 40, "row": 0 } }
        ] } }]),
    );
    assert_that!(run(&d)).is_empty();
}

#[test]
fn validate_missing_asset_is_reported_only_when_assets_are_given() {
    let d = content("blocks", json!([{ "id": "i", "kind": "base/image", "params": { "asset": "sha256:ab" } }]));
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
    let d = content("blocks", json!([{ "id": "c", "kind": "base/card", "params": { "body": [{ "runs": [run_] }] } }]));
    assert_that!(ids(&run(&d))).is_equal_to(vec![id]);
}

#[test]
fn validate_safe_links_are_accepted() {
    let body = json!([{ "runs": [
        { "t": "a", "link": "https://oku.test" }, { "t": "b", "link": "HTTP://x" }, { "t": "c", "link": "mailto:a@b.c" }
    ] }]);
    assert_that!(run(&content("blocks", json!([{ "id": "t", "kind": "base/text", "params": { "body": body } }]))))
        .is_empty();
}

#[test]
fn validate_unknown_bind_is_reported_once_per_widget() {
    let d = deck(json!([{ "id": "a", "kind": "t/cover" }, { "id": "b", "kind": "t/cover" }]));
    let issues = validate(&d, &custom_registry(), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unknown-bind"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-unknown-bind", "param", "brand"));
}

#[test]
fn validate_bind_to_a_deck_extra_is_accepted() {
    let mut d = deck(json!([{ "id": "a", "kind": "t/cover" }]));
    d.params.extra.insert("nope".into(), json!("Marque"));
    assert_that!(validate(&d, &custom_registry(), None)).is_empty();
}

/// Chaîne de `n` `test/frame-box` imbriqués (`c0` ⊃ `c1` ⊃ …), `c0` dans `elements`.
fn frame_chain(n: usize) -> Deck {
    let frame = json!({ "x": 0, "y": 0, "w": 1, "h": 1 });
    let mut e = json!({ "id": format!("c{}", n - 1), "kind": "test/frame-box", "frame": frame });
    for i in (0..n - 1).rev() {
        e = json!({ "id": format!("c{i}"), "kind": "test/frame-box", "frame": frame, "params": { "children": [e] } });
    }
    content("elements", json!([e]))
}

/// Même borne que les ops : `frame_chain(MAX_SLOT_DEPTH)` s'y construit,
/// un niveau de plus est refusé (`tests/ops.rs`).
#[test]
fn validate_elements_nested_to_the_depth_bound_are_accepted() {
    assert_that!(run(&frame_chain(MAX_SLOT_DEPTH))).is_empty();
}

#[test]
fn validate_element_nested_one_level_past_the_bound_is_reported() {
    let issues = run(&frame_chain(MAX_SLOT_DEPTH + 1));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-slot-too-deep"]);
    assert_that!(issues[0].element.clone()).is_equal_to(Some(format!("c{MAX_SLOT_DEPTH}")));
}

#[test]
fn validate_issue_display_names_slide_and_element() {
    let d = content("blocks", json!([{ "id": "", "kind": "base/text" }]));
    assert_that!(run(&d)[0].to_string()).is_equal_to("[s/] empty identifier".to_owned());
}

fn validate_element(element: Value, assets: Option<&HashSet<String>>) -> Vec<Issue> {
    validate(&content("blocks", json!([element])), &crate::test_registry::with_list(), assets)
}

#[test]
fn validate_well_formed_list_has_no_issue() {
    let issues = validate_element(
        json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
            { "id": "a", "date": "2019", "fait": true }, { "id": "b", "texte": "x" }
        ] } }),
        None,
    );
    assert_that!(ids(&issues)).is_empty();
}

#[test]
fn validate_list_with_duplicate_item_ids_reports_it() {
    let issues = validate_element(
        json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [{ "id": "a" }, { "id": "a" }] } }),
        None,
    );
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-duplicate-item-id"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-duplicate-item-id", "param", "etapes"));
    assert_that!(issues[0].element.clone()).is_equal_to(Some("f".to_owned()));
}

#[test]
fn validate_list_with_empty_item_id_reports_it() {
    let issues =
        validate_element(json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [{ "id": "" }] } }), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-empty-item-id"]);
}

#[test]
fn validate_list_above_max_reports_bounds() {
    let three = json!([{ "id": "a" }, { "id": "b" }, { "id": "c" }]);
    let issues = validate_element(json!({ "id": "f", "kind": "test-el/bornes", "params": { "bornee": three } }), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-list-bounds"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-list-bounds", "param", "bornee"));
    let two = json!([{ "id": "a" }, { "id": "b" }]);
    assert_that!(validate_element(json!({ "id": "f", "kind": "test-el/bornes", "params": { "bornee": two } }), None))
        .is_empty();
}

#[rstest]
#[case::at_default_cap(crate::LIST_ITEMS_MAX, &[])]
#[case::over_default_cap(crate::LIST_ITEMS_MAX + 1, &["issue-list-bounds"])]
fn validate_list_without_schema_max_applies_default_cap(#[case] count: usize, #[case] expected: &[&str]) {
    let items: Vec<Value> = (0..count).map(|i| json!({ "id": format!("i{i}") })).collect();
    let issues = validate_element(json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": items } }), None);
    assert_that!(ids(&issues)).is_equal_to(expected.to_vec());
}

#[test]
fn validate_absent_or_empty_list_below_min_is_accepted() {
    assert_that!(validate_element(json!({ "id": "f", "kind": "test-el/bornes" }), None)).is_empty();
    assert_that!(validate_element(json!({ "id": "f", "kind": "test-el/bornes", "params": { "bornee": [] } }), None))
        .is_empty();
}

#[test]
fn validate_list_column_of_wrong_type_reports_param_type() {
    let issues = validate_element(
        json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [{ "id": "a", "date": 3 }] } }),
        None,
    );
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-param-type", "param", "etapes"));
}

#[test]
fn validate_list_column_unknown_reports_param_type() {
    let issues = validate_element(
        json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [{ "id": "a", "inconnue": "x" }] } }),
        None,
    );
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
}

#[test]
fn validate_list_shaped_value_on_a_plain_param_reports_param_type() {
    let d = content("blocks", json!([{ "id": "c", "kind": "base/card", "params": { "title": [{ "id": "a" }] } }]));
    let issues = validate(&d, &crate::test_registry::with_list(), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-param-type", "param", "title"));
}

#[test]
fn validate_list_asset_column_missing_from_sources_reports_it() {
    let element = json!({ "id": "f", "kind": "test-el/bornes", "params": { "images": [
        { "id": "a", "img": "sha256:ab" }, { "id": "b", "img": "sha256:cd" }
    ] } });
    assert_that!(validate_element(element.clone(), None)).is_empty();
    let present = HashSet::from(["sha256:ab".to_owned()]);
    let issues = validate_element(element, Some(&present));
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-missing-asset"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-missing-asset", "asset", "sha256:cd"));
}

/// Un item de liste dont la colonne `note` porte un seul run.
fn noted(id: &str, run_: Value) -> Value {
    json!({ "id": id, "note": [{ "runs": [run_] }] })
}

/// Un lien non sûr dans la cellule rich d'un item d'une liste d'élément : une issue, portée par l'élément.
#[rstest]
#[case::javascript("javascript:alert(1)")]
#[case::data("data:text/html,x")]
#[case::vbscript("vbscript:x")]
#[case::leading_space(" javascript:alert(1)")]
#[case::uppercase("JAVASCRIPT:alert(1)")]
fn validate_list_cell_with_unsafe_link_is_reported_on_the_element(#[case] link: &str) {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
        noted("a", json!({ "t": "x", "link": link }))
    ] } });
    let issues = validate_element(element, None);
    assert_that!(issues.len()).is_equal_to(1);
    assert_that!(issues[0].message).is_equal_to(message("issue-unsupported-link", "link", link));
    assert_that!(issues[0].slide.as_deref()).is_equal_to(Some("s"));
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("f"));
}

#[test]
fn validate_list_cell_with_safe_links_has_no_issue() {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
        noted("a", json!({ "t": "x", "link": "https://oku.test" })),
        noted("b", json!({ "t": "y", "link": "MAILTO:a@b.c" }))
    ] } });
    assert_that!(validate_element(element, None)).is_empty();
}

#[test]
fn validate_list_cell_with_highlight_zero_is_reported_on_the_element() {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
        noted("a", json!({ "t": "x", "highlight": 0 }))
    ] } });
    let issues = validate_element(element, None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-highlight-zero"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("f"));
}

#[test]
fn validate_list_cell_with_valid_highlight_has_no_issue() {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
        noted("a", json!({ "t": "x", "highlight": 1 }))
    ] } });
    assert_that!(validate_element(element, None)).is_empty();
}

#[test]
fn validate_list_walks_every_item() {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": [
        noted("a", json!({ "t": "ok", "link": "https://oku.test" })),
        noted("b", json!({ "t": "x", "link": "javascript:b" }))
    ] } });
    let issues = validate_element(element, None);
    assert_that!(issues.len()).is_equal_to(1);
    assert_that!(issues[0].message).is_equal_to(message("issue-unsupported-link", "link", "javascript:b"));
}

#[test]
fn validate_list_on_a_slide_reports_unsafe_links_of_both_rich_columns_on_the_slide() {
    let d = deck(json!([{ "id": "s", "kind": "test/chrono", "params": { "etapes": [
        { "id": "a", "note": [{ "runs": [{ "t": "x", "link": "javascript:n" }] }],
          "suite": [{ "runs": [{ "t": "y", "link": "data:s" }] }] }
    ] } }]));
    let issues = validate(&d, &crate::test_registry::with_list(), None);
    assert_that!(issues.len()).is_equal_to(2);
    assert_that!(issues[0].message).is_equal_to(message("issue-unsupported-link", "link", "javascript:n"));
    assert_that!(issues[1].message).is_equal_to(message("issue-unsupported-link", "link", "data:s"));
    assert_that!(issues.iter().all(|i| i.slide.as_deref() == Some("s") && i.element.is_none())).is_true();
}

#[rstest]
#[case::empty_list(json!({ "etapes": [] }))]
#[case::absent_list(json!({}))]
#[case::empty_rich_cell(json!({ "etapes": [{ "id": "a", "note": [] }] }))]
#[case::paragraph_without_runs(json!({ "etapes": [{ "id": "a", "note": [{ "runs": [] }] }] }))]
fn validate_list_without_rich_content_has_no_issue(#[case] params: Value) {
    let element = json!({ "id": "f", "kind": "test-el/liste", "params": params });
    assert_that!(validate_element(element, None)).is_empty();
}

// Listes imbriquées (t-495), widget `plan` : étapes à tâches.

fn plan(steps: Value) -> Value {
    json!({ "id": "p", "kind": "test-el/plan", "params": { "etapes": steps } })
}

#[test]
fn validate_well_formed_nested_list_has_no_issue() {
    let steps = json!([{ "id": "a", "taches": [{ "id": "t1", "texte": "x", "fait": true }] }, { "id": "b" }]);
    assert_that!(ids(&validate_element(plan(steps), None))).is_empty();
}

#[test]
fn validate_nested_list_with_duplicate_ids_reports_it_with_its_path() {
    let steps = json!([{ "id": "a", "taches": [{ "id": "t" }, { "id": "t" }] }]);
    let issues = validate_element(plan(steps), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-duplicate-item-id"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-duplicate-item-id", "param", "etapes/a/taches"));
}

#[test]
fn validate_nested_lists_of_two_items_may_share_ids() {
    let steps = json!([{ "id": "a", "taches": [{ "id": "d0" }] }, { "id": "b", "taches": [{ "id": "d0" }] }]);
    assert_that!(ids(&validate_element(plan(steps), None))).is_empty();
}

#[test]
fn validate_nested_list_above_its_max_reports_bounds() {
    let tasks: Vec<Value> = (0..4).map(|i| json!({ "id": format!("t{i}") })).collect();
    let issues = validate_element(plan(json!([{ "id": "a", "taches": tasks }])), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-list-bounds"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-list-bounds", "param", "etapes/a/taches"));
}

#[test]
fn validate_nested_cell_of_wrong_type_reports_param_type() {
    let issues = validate_element(plan(json!([{ "id": "a", "taches": [{ "id": "t", "fait": "oui" }] }])), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-param-type"]);
}

#[test]
fn validate_nested_rich_cell_with_an_unsafe_link_reports_it() {
    let task = noted("t", json!({ "t": "a", "link": "javascript:n" }));
    let issues = validate_element(plan(json!([{ "id": "a", "taches": [task] }])), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-unsupported-link"]);
}

#[rstest]
#[case::top(json!([{ "id": "a/b" }]), "etapes")]
#[case::nested(json!([{ "id": "a", "taches": [{ "id": "x/y" }] }]), "etapes/a/taches")]
fn validate_item_id_with_a_slash_reports_invalid_item_id(#[case] steps: Value, #[case] path: &str) {
    let issues = validate_element(plan(steps), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-invalid-item-id"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-invalid-item-id", "param", path));
}

#[test]
fn validate_nested_items_beyond_the_total_cap_report_bounds_on_the_list() {
    let steps: Vec<Value> = (0..crate::NESTED_ITEMS_MAX / 3 + 1)
        .map(|i| json!({ "id": format!("e{i}"), "taches": [{ "id": "a" }, { "id": "b" }, { "id": "c" }] }))
        .collect();
    let issues = validate_element(plan(Value::Array(steps)), None);
    assert_that!(ids(&issues)).is_equal_to(vec!["issue-list-bounds"]);
    assert_that!(issues[0].message).is_equal_to(message("issue-list-bounds", "param", "etapes"));
}

#[test]
fn issue_pack_kind_conflict_localizes_with_its_pack_in_french() {
    let issue = Issue::pack_kind_conflict(&crate::kinds::essentiel::namespace());
    let fr = issue.localize(&Localizer::new(Some("fr")));
    assert_that!((issue.message.id(), fr.contains("essentiel"))).is_equal_to(("issue-pack-kind-conflict", true));
    assert_that!(fr).is_not_equal_to(issue.localize(oku_slides_i18n::english()));
}

#[test]
fn issue_pack_kind_mismatch_localizes_with_its_pack_in_french() {
    let issue = Issue::pack_kind_mismatch(&crate::kinds::essentiel::namespace());
    let fr = issue.localize(&Localizer::new(Some("fr")));
    assert_that!((issue.message.id(), fr.contains("essentiel"))).is_equal_to(("issue-pack-kind-mismatch", true));
    assert_that!(fr).is_not_equal_to(issue.localize(oku_slides_i18n::english()));
}
