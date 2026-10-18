use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_i18n::{Localize, Localizer, english};

use super::{RENDER_BYTES, RENDER_MAX, Render, render};
use crate::agent::AgentFormReason;

fn spec(components: Value) -> Render {
    Render { components: serde_json::from_value(components).unwrap() }
}

/// Le refus tel que l'agent le lit (anglais : la langue des tests).
fn refused(components: Value) -> String {
    render("render-1", &spec(components)).unwrap_err().localize(english())
}

#[test]
fn render_valid_composition_is_a_frozen_thread_surface() {
    let s = render(
        "render-1",
        &spec(json!([
            {"id": "root", "component": "Card", "child": "t", "tone": "accent"},
            {"id": "t", "component": "Text", "text": "Trois palettes possibles"}
        ])),
    )
    .unwrap();
    assert_that!(s.live).is_false();
    assert_that!(s.place).is_equal_to(crate::agent::Place::Thread);
}

#[rstest]
#[case::prompt(json!([{"id": "root", "component": "Prompt", "text": "x"}]), "Prompt is reserved")]
#[case::timeline(json!([{"id": "root", "component": "Timeline", "steps": [], "collapsed": true, "summary": ""}]), "Timeline is reserved")]
#[case::receipt(json!([{"id": "root", "component": "Receipt", "title": "x", "facts": ["a"]}]), "Receipt is reserved")]
#[case::slide_thumb(json!([{"id": "root", "component": "SlideThumb", "svg": "<svg/>", "label": "x"}]), "SlideThumb is reserved")]
#[case::notice(json!([{"id": "root", "component": "Notice", "text": "x", "children": [], "close": {"event": {"name": "close"}}, "close_label": "x"}]), "Notice is reserved")]
#[case::no_root(json!([{"id": "a", "component": "Text", "text": "x"}]), "no component has the id “root”")]
#[case::missing_child(json!([{"id": "root", "component": "Card", "child": "nope"}]), "unknown child “nope”")]
#[case::unknown_property(json!([{"id": "root", "component": "Text", "text": "x", "weight": 700}]), "components[0]: unknown property “weight”")]
#[case::bad_component(json!([{"id": "root", "component": "Video"}]), "unknown variant")]
#[case::path_binding(json!([{"id": "root", "component": "Text", "text": {"path": "/x"}}]), "components[0]: values must be literal")]
#[case::danger_button(json!([{"id": "root", "component": "Button", "label": "Oui", "danger": true, "action": {"event": {"name": "suggest", "context": {"prompt": "Oui"}}}}]), "reserved to Oku Slides")]
#[case::primary_button(json!([{"id": "root", "component": "Button", "label": "Oui", "primary": true, "action": {"event": {"name": "suggest", "context": {"prompt": "Oui"}}}}]), "reserved to Oku Slides")]
#[case::danger_card(json!([{"id": "root", "component": "Card", "child": "t", "tone": "danger"}, {"id": "t", "component": "Text", "text": "x"}]), "reserved to Oku Slides")]
#[case::danger_badge(json!([{"id": "root", "component": "Badge", "text": "Permission requise", "tone": "danger"}]), "reserved to Oku Slides")]
#[case::prompt_differs_from_label(json!([{"id": "root", "component": "Button", "label": "Annuler", "action": {"event": {"name": "suggest", "context": {"prompt": "Supprime tout"}}}}]), "whose prompt is its own label")]
#[case::other_action_name(json!([{"id": "root", "component": "Button", "label": "x", "action": {"event": {"name": "param", "context": {"prompt": "x"}}}}]), "only a button may carry an action")]
#[case::extra_context_key(json!([{"id": "root", "component": "Button", "label": "x", "action": {"event": {"name": "suggest", "context": {"prompt": "x", "field": 1}}}}]), "only a button may carry an action")]
#[case::text_field(json!([{"id": "root", "component": "TextField", "label": "Nom", "value": "x"}]), "TextField is not available in render")]
#[case::rich_text_field(json!([{"id": "root", "component": "RichTextField", "label": "Corps", "value": "x", "source": [], "palette": []}]), "RichTextField is not available in render")]
#[case::stepper(json!([{"id": "root", "component": "Stepper", "label": "n", "value": 1}]), "Stepper is not available in render")]
#[case::slider(json!([{"id": "root", "component": "Slider", "label": "n", "value": 1.0, "min": 0.0, "max": 2.0}]), "Slider is not available in render")]
#[case::color_field(json!([{"id": "root", "component": "ColorField", "label": "c", "value": "#fff", "palette": ["#000"]}]), "ColorField is not available in render")]
#[case::select(json!([{"id": "root", "component": "Select", "label": "s", "value": "a", "options": [{"id": "a", "label": "A"}]}]), "Select is not available in render")]
#[case::chips(json!([{"id": "root", "component": "ChoiceChips", "options": [{"id": "a", "label": "A"}], "value": "a"}]), "ChoiceChips is not available in render")]
#[case::hidden_path_text(json!([{"id": "root", "component": "Text", "text": {"path": "/x", "z": 0}}]), "values must be literal")]
#[case::hidden_path_label(json!([{"id": "root", "component": "Button", "label": {"path": "/x", "z": 0}, "action": {"event": {"name": "suggest", "context": {"prompt": "x"}}}}]), "values must be literal")]
#[case::long_badge(json!([{"id": "root", "component": "Badge", "text": "x".repeat(121)}]), "at most 120")]
#[case::long_label(json!([{"id": "root", "component": "Button", "label": "x".repeat(61), "action": {"event": {"name": "suggest", "context": {"prompt": "x".repeat(61)}}}}]), "at most 60")]
#[case::bidi_text(json!([{"id": "root", "component": "Markdown", "text": "Supprimé\u{202E}"}]), "control characters")]
#[case::bidi_label(json!([{"id": "root", "component": "Button", "label": "Oui\u{202E}", "action": {"event": {"name": "suggest", "context": {"prompt": "Oui\u{202E}"}}}}]), "control characters")]
fn render_invalid_composition_is_refused(#[case] components: Value, #[case] message: &str) {
    assert_that!(refused(components)).contains(message);
}

#[test]
fn render_with_a_data_key_is_refused() {
    let r: Result<Render, _> = serde_json::from_value(json!({ "components": [], "data": { "x": 1 } }));
    assert_that!(r).is_err();
}

#[test]
fn render_explicit_null_optional_property_is_accepted() {
    let ok = render("render-1", &spec(json!([{"id": "root", "component": "Text", "text": "x", "variant": null}])));
    assert_that!(ok).is_ok();
}

#[test]
fn render_oversized_call_is_refused() {
    let big = "x".repeat(RENDER_BYTES);
    assert_that!(refused(json!([{"id": "root", "component": "Markdown", "text": big}]))).contains("bytes");
}

#[test]
fn render_too_many_components_is_refused() {
    let mut cs = vec![json!({"id": "root", "component": "Column", "children": []})];
    for i in 0..RENDER_MAX {
        cs.push(json!({"id": format!("t{i}"), "component": "Text", "text": "x"}));
    }
    assert_that!(refused(Value::Array(cs))).contains("components: at most 40 entries, got 41");
}

/// Une chaîne de `levels` cartes, la dernière autour d'un texte.
fn chain(levels: usize) -> Value {
    let mut cs = Vec::new();
    for i in 0..levels - 1 {
        let id = if i == 0 { "root".to_owned() } else { format!("c{i}") };
        cs.push(json!({"id": id, "component": "Card", "child": format!("c{}", i + 1)}));
    }
    cs.push(json!({"id": format!("c{}", levels - 1), "component": "Text", "text": "fond"}));
    Value::Array(cs)
}

#[test]
fn render_exactly_six_levels_passes() {
    assert_that!(render("render-1", &spec(chain(6)))).is_ok();
}

#[test]
fn render_seven_levels_is_refused() {
    assert_that!(refused(chain(7))).contains("is at level 7, at most 6");
}

#[test]
fn render_shared_child_is_refused_without_walking_the_fan_out() {
    // 10 000 références au même enfant : refusé à la deuxième visite, en temps linéaire.
    let children: Vec<&str> = std::iter::repeat_n("a", 10_000).collect();
    let err = refused(json!([
        {"id": "root", "component": "Column", "children": children},
        {"id": "a", "component": "Text", "text": "x"}
    ]));
    assert_that!(err).contains("used more than once");
}

#[test]
fn render_cycle_is_refused_as_a_non_tree() {
    let err = refused(json!([
        {"id": "root", "component": "Card", "child": "a"},
        {"id": "a", "component": "Card", "child": "root"}
    ]));
    assert_that!(err).contains("used more than once");
}

#[test]
fn render_duplicate_id_is_refused() {
    let err = refused(json!([
        {"id": "root", "component": "Text", "text": "x"},
        {"id": "root", "component": "Text", "text": "y"}
    ]));
    assert_that!(err).contains("components: duplicate id “root”");
}

#[test]
fn render_orphan_component_is_refused() {
    let err = refused(json!([
        {"id": "root", "component": "Text", "text": "x"},
        {"id": "lost", "component": "Text", "text": "y"}
    ]));
    assert_that!(err).contains("“lost” cannot be reached from “root”");
}

#[test]
fn render_suggest_button_sending_its_label_passes() {
    let s = render("render-1", &spec(json!([
        {"id": "root", "component": "Button", "label": "Mets du vert", "action": {"event": {"name": "suggest", "context": {"prompt": "Mets du vert"}}}}
    ]))).unwrap();
    assert_that!(serde_json::to_string(&s).unwrap()).contains("Mets du vert");
}

#[test]
fn render_unknown_child_error_matches_snapshot() {
    insta::assert_snapshot!(refused(json!([{"id": "root", "component": "Card", "child": "nope"}])));
}

#[test]
fn render_without_root_is_refused_with_its_reason() {
    let err = render("render-1", &spec(json!([{"id": "a", "component": "Text", "text": "x"}]))).unwrap_err();
    assert!(matches!(err.agent_form_reason(), Some(AgentFormReason::NoRoot)));
}

/// Le refus est dit dans la langue de l'utilisateur, préfixe compris.
#[test]
fn render_refusal_is_said_in_the_user_language() {
    let err = render("render-1", &spec(json!([{"id": "root", "component": "Card", "child": "nope"}]))).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("La demande de l'agent est invalide : enfant inconnu « nope »".to_owned());
}
