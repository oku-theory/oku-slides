use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{MAX_TEMPLATE_DEPTH, fill_template};

#[test]
fn fill_template_replaces_every_value_marker() {
    let t = json!({ "op": "set_param", "target": { "slide": "s-1" }, "param": "grid",
        "value": { "columns": "$value", "rows": 6 }, "tags": ["$value"] });
    assert_that!(fill_template(&t, &json!(8))).is_equal_to(json!({ "op": "set_param", "target": { "slide": "s-1" },
        "param": "grid", "value": { "columns": 8, "rows": 6 }, "tags": [8] }));
}

#[test]
fn fill_template_leaves_other_strings() {
    let t = json!({ "op": "set_var", "key": "theme", "value": "$value-x" });
    assert_that!(fill_template(&t, &json!("pro"))).is_equal_to(t);
}

/// `leaf` niché sous `n` tableaux imbriqués.
fn nested(n: usize, leaf: Value) -> Value {
    (0..n).fold(leaf, |v, _| json!([v]))
}

#[test]
fn fill_template_replaces_a_marker_within_the_depth_limit() {
    let n = MAX_TEMPLATE_DEPTH - 1;
    let t = nested(n, json!("$value"));
    assert_that!(fill_template(&t, &json!(1))).is_equal_to(nested(n, json!(1)));
}

#[test]
fn fill_template_leaves_a_marker_past_the_depth_limit_untouched() {
    let t = nested(MAX_TEMPLATE_DEPTH, json!("$value"));
    assert_that!(fill_template(&t, &json!(1))).is_equal_to(t);
}

/// L'op `set_theme` de l'agent, sans épinglages : la forme documentée.
fn set_theme(theme: &str) -> oku_slides_core::Op {
    serde_json::from_value(json!({ "op": "set_theme", "theme": theme })).unwrap()
}

#[test]
fn variant_decks_with_a_pinless_set_theme_pin_its_closure() {
    let s = crate::test_support::session();
    let variant =
        |label: &str, theme: &str| super::super::forms::Variant { label: label.into(), ops: vec![set_theme(theme)] };
    let slide = s.deck().slides[0].id.clone();
    let spec =
        super::super::forms::ShowVariants { slide, variants: vec![variant("Pro", "pro"), variant("Neutre", "neutre")] };
    let decks = s.variant_decks(&spec).unwrap();
    let pinned: Vec<Vec<&str>> =
        decks.iter().map(|d| d.themes.keys().map(oku_slides_core::Namespace::as_str).collect()).collect();
    assert_that!(pinned).is_equal_to(vec![vec!["pro"], vec!["neutre"]]);
    assert_that!(decks[0].themes[&crate::test_support::ns("pro")].id).is_equal_to(crate::PRO_THEME);
}

#[test]
fn params_surface_with_a_pinless_set_theme_template_is_accepted() {
    let s = crate::test_support::session();
    let field: super::super::forms::ParamField = serde_json::from_value(json!({
        "label": "Thème", "kind": "text", "value": "pro",
        "op_template": { "op": "set_theme", "theme": "$value" }
    }))
    .unwrap();
    let spec = super::super::forms::ShowParams {
        title: "Thème".into(),
        slide: s.deck().slides[0].id.clone(),
        fields: vec![field],
    };
    assert_that!(s.params_surface("p-1", &spec).map(drop)).is_ok();
}

/// A one-field `show_params` spec on `s`'s first slide.
fn theme_params(s: &crate::Session) -> super::super::forms::ShowParams {
    let field: super::super::forms::ParamField = serde_json::from_value(json!({
        "label": "Thème", "kind": "text", "value": "papier",
        "op_template": { "op": "set_var", "key": "theme", "value": "$value" }
    }))
    .unwrap();
    super::super::forms::ShowParams {
        title: "Thème".into(), slide: s.deck().slides[0].id.clone(), fields: vec![field]
    }
}

/// The place of `s`'s `show_params` surface.
fn params_place(s: &crate::Session) -> super::super::forms::Place {
    s.params_surface("p-1", &theme_params(s)).unwrap().place
}

#[test]
fn params_surface_in_deck_mode_is_in_the_inspector() {
    let mut s = crate::test_support::session();
    s.begin_turn(1, "x");
    assert_that!(params_place(&s)).is_equal_to(super::super::forms::Place::Inspector);
}

#[rstest::rstest]
#[case::pack(crate::DocumentKind::Pack)]
#[case::theme(crate::DocumentKind::Theme)]
#[case::help(crate::DocumentKind::Help)]
fn params_surface_in_pack_or_theme_tab_stays_in_the_thread(#[case] kind: crate::DocumentKind) {
    let data = crate::test_support::fresh_dir("oku-slides-params-place-");
    let mut s = crate::test_support::session_on(data.path());
    match kind {
        crate::DocumentKind::Pack => s.open_pack_tab_here(crate::pack::ESSENTIEL_PACK, None).unwrap(),
        crate::DocumentKind::Theme => s.open_theme_here(crate::theme::PAPIER_THEME).unwrap(),
        crate::DocumentKind::Help => s.open_help("fr", None).unwrap(),
        crate::DocumentKind::Deck => {}
    }
    s.begin_turn(1, "x");
    assert_that!(params_place(&s)).is_equal_to(super::super::forms::Place::Thread);
}

#[test]
fn params_surface_without_turn_stays_in_the_thread() {
    let s = crate::test_support::session();
    assert_that!(params_place(&s)).is_equal_to(super::super::forms::Place::Thread);
}
