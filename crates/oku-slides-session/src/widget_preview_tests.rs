use serde_json::{Map, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamValue, WidgetFile, WidgetSchema};
use oku_slides_render::RenderError;

use super::{
    ExampleAlert, PreviewFormat, PreviewLook, PreviewRatio, PreviewScheme, failure, preview_params, scheme_colors,
};
use crate::Appearance;
use crate::test_support::{fresh_dir, opened_on, session_with, settings_on};
use crate::widgets_edit::parse_name;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

const FAILS_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("rate", params: (body: p.plain()),
  (args, ctx) => panic("aperçu raté"))
"#;

fn schema() -> WidgetSchema {
    serde_json::from_value(json!({
        "name": "kpi", "label": null, "category": "element",
        "params": [
            { "name": "value", "type": "plain", "label": null, "default": "42 %", "bind": null },
            { "name": "label", "type": "plain", "label": null, "default": null, "bind": null },
            { "name": "body", "type": "rich", "label": null, "default": null },
            { "name": "trend", "type": "choice", "values": ["up", "down"], "label": null, "default": null },
            { "name": "count", "type": "number", "min": 3.0, "max": null, "label": null, "default": null },
            { "name": "on", "type": "flag", "label": null, "default": null },
            { "name": "logo", "type": "asset", "label": null, "default": null }
        ]
    }))
    .unwrap()
}

#[test]
fn preview_params_fill_given_then_defaults_then_examples_and_neutral_values() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "label": "Donné" })).unwrap();
    let p = preview_params(&schema(), &given, &l);
    assert_that!(p.get("value")).is_equal_to(Some(&ParamValue::Text("42 %".into())));
    assert_that!(p.get("label")).is_equal_to(Some(&ParamValue::Text("Donné".into())));
    assert_that!(p.get("body").is_some_and(|v| matches!(v, ParamValue::Rich(_)))).is_true();
    assert_that!(p.get("trend")).is_equal_to(Some(&ParamValue::Text("up".into())));
    assert_that!(p.get("count")).is_equal_to(Some(&ParamValue::Value(json!(3.0))));
    assert_that!(p.get("on")).is_equal_to(Some(&ParamValue::Value(json!(false))));
    assert_that!(p.get("logo")).is_none();
    let example = l.tr("widget-preview-plain");
    let without = preview_params(&schema(), &Map::new(), &l);
    assert_that!(without.get("label")).is_equal_to(Some(&ParamValue::Text(example)));
}

#[test]
fn preview_widget_renders_an_svg_in_deck_mode() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
}

#[test]
fn preview_widget_png_is_an_image_for_the_agent() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.png.unwrap_or_default().starts_with(b"\x89PNG")).is_true();
}

#[test]
fn preview_widget_after_a_worker_restart_still_renders() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    s.worker.kill_for_test();
    let preview = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
}

#[test]
fn preview_widget_of_a_broken_file_returns_its_diagnostics_without_image() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget("casse", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
}

#[test]
fn preview_widget_of_common_or_unknown_file_is_refused() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("_commun", "#let x = 1".into()).unwrap();
    let common = s.preview_widget("_commun", None, &Map::new(), PreviewFormat::Svg).unwrap_err();
    let unknown = s.preview_widget("absent", None, &Map::new(), PreviewFormat::Svg).unwrap_err();
    assert_that!(common.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
    assert_that!(unknown.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
}

#[test]
fn preview_widget_that_fails_in_typst_returns_its_diagnostics() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("rate", FAILS_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget("rate", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error && d.message.contains("aperçu raté"))).is_true();
}

#[test]
fn preview_widget_that_loops_returns_a_diagnostic_and_keeps_the_pack() {
    let data = fresh_dir("oku-slides-preview-");
    let mut settings = settings_on(data.path());
    settings.render.timeout_ms = 2000;
    let mut s = session_with(settings);
    s.set_widget("boucle", LOOP_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget("boucle", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
    // Le pack n'est pas écarté : le deck garde ses widgets.
    let pack = s.pack.clone().unwrap();
    assert_that!(s.worker.is_faulty(&pack)).is_false();
    assert_that!(s.registry.get("boucle")).is_some();
}

#[test]
fn failure_of_a_disabled_pack_is_an_explicit_diagnostic() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let name = WidgetFile::parse("kpi").unwrap();
    let diagnostics = failure(&name, &RenderError::pack_disabled("mon-pack"), &l);
    assert_that!(diagnostics).has_length(1);
    assert_that!(diagnostics[0].error).is_true();
    assert_that!(diagnostics[0].file).is_equal_to(Some(name));
    assert_that!(diagnostics[0].message.contains("mon-pack")).is_true();
}

#[test]
fn no_schema_gives_an_explicit_diagnostic_not_an_empty_preview() {
    let name = oku_slides_core::WidgetFile::parse("kpi").unwrap();
    let preview = super::no_schema(&name);
    assert_that!(preview.svg.is_none() && preview.png.is_none()).is_true();
    assert_that!(preview.diagnostics.len()).is_equal_to(1);
    assert_that!(preview.diagnostics[0].error).is_true();
    assert_that!(preview.diagnostics[0].file.clone()).is_equal_to(Some(name));
}

const COVER: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("couverture", category: "slide", params: (title: p.plain()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

#[test]
fn preview_of_an_element_widget_gives_the_box_of_what_it_draws() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let bbox = preview.bbox.unwrap();
    // Une ligne de texte : bien plus petite que la grille (1680 × 790 pt).
    assert_that!(bbox.w).is_greater_than(0.0);
    assert_that!(bbox.h).is_greater_than(0.0);
    assert_that!(bbox.w).is_less_than(1680.0);
    assert_that!(bbox.h).is_less_than(200.0);
    let json = serde_json::to_value(bbox).unwrap();
    assert_that!(json).is_equal_to(json!({ "x": bbox.x, "y": bbox.y, "w": bbox.w, "h": bbox.h }));
}

#[test]
fn preview_of_a_slide_widget_has_no_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let preview = s.preview_widget("couverture", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.is_some()).is_true();
    assert_that!(preview.bbox).is_none();
}

#[test]
fn preview_keeps_the_bare_slide_png_between_previews() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let first = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let kept = s.widget_preview.as_ref().and_then(|p| p.bare.clone()).unwrap();
    // Une autre source du widget montré : la slide sans éléments ne change pas.
    s.set_widget("quote", QUOTE.replace("emph", "strong")).unwrap();
    let second = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.widget_preview.as_ref().and_then(|p| p.bare.clone())).is_equal_to(Some(kept));
    assert_that!(first.bbox.is_some() && second.bbox.is_some()).is_true();
}

#[test]
fn png_preview_for_the_agent_has_no_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.png.is_some()).is_true();
    assert_that!(preview.bbox).is_none();
}

const LOGO: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("logo", params: (asset: p.asset()),
  (args, ctx) => (ctx.load)(asset: args.asset, fit: "contain"))
"#;

/// Un PNG de 2 × 2 pixels rouges.
fn red_png() -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 2, 2);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&[255, 0, 0].repeat(4)).unwrap();
    out
}

#[test]
fn preview_widget_shows_an_asset_of_the_deck() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("logo", LOGO.into()).unwrap();
    let id = s.doc.sources.add_asset(red_png());
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget("logo", None, &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    // L'image, pas le cadre « image manquante ».
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}

#[test]
fn preview_widget_in_templates_mode_shows_an_asset_of_the_set_aside_deck() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let id = s.doc.sources.add_asset(red_png());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("logo", LOGO.into()).unwrap();
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget("logo", None, &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}

#[test]
fn preview_widget_in_history_mode_shows_an_asset_of_the_deck() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("logo", LOGO.into()).unwrap();
    s.apply("Thème", vec![oku_slides_core::Op::SetTheme { theme: Some("pro".into()) }], None).unwrap();
    s.history_open().unwrap();
    // L'historique a pris les images du deck mis de côté (`history.rs`) :
    // celle-ci y est, sans rendu qui l'aurait déjà envoyée au processus de rendu.
    let id = s.doc.sources.add_asset(red_png());
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget("logo", None, &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}
fn in_templates_with_quote() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    (data, s)
}

fn body(t: &str) -> Map<String, serde_json::Value> {
    serde_json::from_value(json!({ "body": [{ "runs": [{ "t": t }] }] })).unwrap()
}

/// `QUOTE` : `source` devient un nombre, `body` reste.
const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.number()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

#[test]
fn preview_widget_without_params_differs_once_an_example_value_is_set() {
    let (_d, mut s) = in_templates_with_quote();
    let plain = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap().svg;
    s.set_example_values("quote", None, body("Une autre")).unwrap();
    let tried = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_not_equal_to(plain);
}

#[test]
fn preview_widget_with_an_example_value_renders_like_the_given_value() {
    let (_d, mut s) = in_templates_with_quote();
    let explicit = s.preview_widget("quote", None, &body("Une autre"), PreviewFormat::Svg).unwrap().svg;
    s.set_example_values("quote", None, body("Une autre")).unwrap();
    let tried = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_equal_to(explicit);
}

#[test]
fn preview_widget_of_a_named_example_uses_its_values() {
    let (_d, mut s) = in_templates_with_quote();
    let explicit = s.preview_widget("quote", None, &body("Long"), PreviewFormat::Svg).unwrap().svg;
    s.add_example("quote", "Long", None).unwrap();
    s.set_example_values("quote", Some("Long"), body("Long")).unwrap();
    let named = s.preview_widget("quote", Some("Long"), &Map::new(), PreviewFormat::Svg).unwrap().svg;
    let first = s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(named).is_equal_to(&explicit);
    assert_that!(first).is_not_equal_to(&explicit);
}

#[test]
fn preview_widget_of_an_unknown_example_is_refused() {
    let (_d, mut s) = in_templates_with_quote();
    let err = s.preview_widget("quote", Some("Absent"), &Map::new(), PreviewFormat::Svg).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownExample);
}

#[test]
fn preview_inputs_given_value_wins_over_the_example() {
    let (_d, mut s) = in_templates_with_quote();
    let quote = parse_name("quote").unwrap();
    s.set_example_values("quote", None, body("Essai")).unwrap();
    let schema = s.valid_schema(&quote).unwrap().unwrap();
    let inputs = s.preview_inputs(&quote, &schema, None, &body("Donné")).unwrap();
    assert_that!(inputs.get("body")).is_equal_to(body("Donné").get("body"));
}

#[test]
fn preview_widget_twice_keeps_the_examples_revision() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_example_values("quote", None, body("Essai")).unwrap();
    let before = s.snapshot().widget_trials;
    s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    s.preview_widget("quote", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.snapshot().widget_trials).is_equal_to(before);
}

#[test]
fn set_widget_keeps_an_example_value_still_valid() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_example_values("quote", None, body("Essai")).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let quote = parse_name("quote").unwrap();
    assert_that!(s.example_values(&quote, None).unwrap().contains_key("body")).is_true();
}

/// Une valeur que le schéma n'admet plus reste dans le pack, mais l'aperçu l'ignore.
#[test]
fn preview_inputs_ignore_an_example_value_whose_type_changed() {
    let (_d, mut s) = in_templates_with_quote();
    let values: Map<String, serde_json::Value> = serde_json::from_value(json!({ "source": "Livre" })).unwrap();
    s.set_example_values("quote", None, values).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let quote = parse_name("quote").unwrap();
    let schema = s.valid_schema(&quote).unwrap().unwrap();
    assert_that!(s.example_values(&quote, None).unwrap().contains_key("source")).is_true();
    assert_that!(s.preview_inputs(&quote, &schema, None, &Map::new()).unwrap().contains_key("source")).is_false();
}

#[test]
fn rename_widget_moves_its_example_values() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_example_values("quote", None, body("Essai")).unwrap();
    s.rename_widget("quote", "citation").unwrap();
    let citation = parse_name("citation").unwrap();
    assert_that!(s.example_values(&citation, None).unwrap().contains_key("body")).is_true();
}

/// Un widget d'élément de 200 × 60 pt : un titre et un corps.
const PANEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("panneau", params: (title: p.plain(), body: p.rich()),
  (args, ctx) => block(width: 200pt, height: 60pt, {
    field(ctx, "title", args.title)
    field(ctx, "body", args.body)
  }))
"#;

fn values(v: serde_json::Value) -> Map<String, serde_json::Value> {
    serde_json::from_value(v).unwrap()
}

fn field_names(view: &serde_json::Value) -> Vec<String> {
    view["fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn preview_of_an_element_widget_gives_its_fields_inside_the_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("panneau", PANEL.into()).unwrap();
    let preview = s.preview_widget("panneau", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let bbox = preview.bbox.unwrap();
    let view = preview.view.unwrap();
    assert_that!(field_names(&view)).is_equal_to(vec!["title".to_owned(), "body".to_owned()]);
    assert_that!(view["width"]).is_equal_to(json!(1920.0));
    for f in view["fields"].as_array().unwrap() {
        let (x, y) = (f["x"].as_f64().unwrap(), f["y"].as_f64().unwrap());
        let (w, h) = (f["w"].as_f64().unwrap(), f["h"].as_f64().unwrap());
        assert_that!(x).is_greater_than_or_equal_to(bbox.x - 1.0);
        assert_that!(y).is_greater_than_or_equal_to(bbox.y - 1.0);
        assert_that!(x + w).is_less_than_or_equal_to(bbox.x + bbox.w + 1.0);
        assert_that!(y + h).is_less_than_or_equal_to(bbox.y + bbox.h + 1.0);
        assert_that!(f["bound"]).is_equal_to(json!(false));
    }
    assert_that!(preview.alerts).is_empty();
}

#[test]
fn preview_of_a_slide_widget_gives_its_fields() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let preview = s.preview_widget("couverture", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let view = preview.view.unwrap();
    assert_that!(field_names(&view)).is_equal_to(vec!["title".to_owned()]);
    assert_that!(view["targets"]).is_equal_to(json!([]));
}

#[test]
fn preview_of_an_empty_example_alerts_on_its_text_fields() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("panneau", PANEL.into()).unwrap();
    let empty = values(json!({ "title": "", "body": [] }));
    let preview = s.preview_widget("panneau", None, &empty, PreviewFormat::Svg).unwrap();
    assert_that!(preview.alerts)
        .is_equal_to(vec![ExampleAlert::Empty { field: "title".into() }, ExampleAlert::Empty { field: "body".into() }]);
}

#[test]
fn preview_of_a_long_text_in_a_narrow_widget_alerts_on_overflow() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("panneau", PANEL.into()).unwrap();
    let long = "Un texte bien trop long pour ce panneau étroit. ".repeat(200);
    let given = values(json!({ "title": "Titre", "body": [{ "runs": [{ "t": long }] }] }));
    let preview = s.preview_widget("panneau", None, &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.alerts).is_equal_to(vec![ExampleAlert::Overflow { field: "body".into() }]);
}

#[test]
fn preview_of_a_broken_widget_has_no_view_nor_alerts() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget("casse", None, &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.view).is_none();
    assert_that!(preview.alerts).is_empty();
}

#[test]
fn png_preview_for_the_agent_has_no_view() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("panneau", PANEL.into()).unwrap();
    let preview = s.preview_widget("panneau", None, &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.view).is_none();
}

#[test]
fn example_alerts_serialize_with_their_kind() {
    let alerts = vec![ExampleAlert::Empty { field: "title".into() }, ExampleAlert::Overflow { field: "body".into() }];
    assert_that!(serde_json::to_value(alerts).unwrap()).is_equal_to(json!([
        { "kind": "empty", "field": "title" },
        { "kind": "overflow", "field": "body" }
    ]));
}

fn look(scheme: Option<PreviewScheme>, ratio: Option<PreviewRatio>) -> PreviewLook {
    PreviewLook { scheme, ratio }
}

/// La balise ouvrante du SVG : sa taille.
fn svg_tag(svg: &str) -> &str {
    &svg[..svg.find('>').unwrap()]
}

/// Le deck du fixture est au thème Papier : son schéma sombre est celui de `app-dark`.
#[test]
fn preview_in_dark_paints_the_slide_with_the_dark_colors_of_the_theme() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let light = s.preview_widget("couverture", None, &Map::new(), PreviewFormat::Svg).unwrap().svg.unwrap();
    let dark = s
        .preview_widget_with("couverture", None, &Map::new(), PreviewFormat::Svg, look(Some(PreviewScheme::Dark), None))
        .unwrap()
        .svg
        .unwrap();
    assert_that!(light.as_str()).does_not_contain("#34312c");
    assert_that!(dark.as_str()).contains("#34312c");
}

#[test]
fn preview_in_light_of_a_light_deck_is_the_deck_as_is() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let plain = s.preview_widget("couverture", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let light = s
        .preview_widget_with(
            "couverture",
            None,
            &Map::new(),
            PreviewFormat::Svg,
            look(Some(PreviewScheme::Light), None),
        )
        .unwrap();
    assert_that!(light.svg).is_equal_to(plain.svg);
}

#[test]
fn preview_in_four_thirds_renders_a_narrower_slide() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let preview = s
        .preview_widget_with(
            "couverture",
            None,
            &Map::new(),
            PreviewFormat::Svg,
            look(None, Some(PreviewRatio::Standard)),
        )
        .unwrap();
    let svg = preview.svg.unwrap();
    assert_that!(svg_tag(&svg)).contains("viewBox=\"0 0 1440 1080\"");
    let view = preview.view.unwrap();
    assert_that!(view["width"]).is_equal_to(json!(1440.0));
    assert_that!(view["height"]).is_equal_to(json!(1080.0));
}

#[test]
fn preview_of_an_element_widget_in_four_thirds_stays_inside_the_narrower_slide() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("panneau", PANEL.into()).unwrap();
    let preview = s
        .preview_widget_with("panneau", None, &Map::new(), PreviewFormat::Svg, look(None, Some(PreviewRatio::Standard)))
        .unwrap();
    let bbox = preview.bbox.unwrap();
    assert_that!(bbox.x + bbox.w).is_less_than_or_equal_to(1440.0);
    assert_that!(preview.view.unwrap()["width"]).is_equal_to(json!(1440.0));
    assert_that!(preview.alerts).is_empty();
}

#[test]
fn preview_in_sixteen_ninths_is_the_usual_slide() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("couverture", COVER.into()).unwrap();
    let plain = s.preview_widget("couverture", None, &Map::new(), PreviewFormat::Svg).unwrap();
    let wide = s
        .preview_widget_with("couverture", None, &Map::new(), PreviewFormat::Svg, look(None, Some(PreviewRatio::Wide)))
        .unwrap();
    assert_that!(svg_tag(wide.svg.as_deref().unwrap())).contains("viewBox=\"0 0 1920 1080\"");
    // Même page, même champs (l'empreinte de page suit les jetons posés).
    let (wide, plain) = (wide.view.unwrap(), plain.view.unwrap());
    assert_that!(wide["width"]).is_equal_to(json!(1920.0));
    assert_that!(wide["fields"].clone()).is_equal_to(plain["fields"].clone());
}

fn appearance(scheme: &str, variants: &[(&str, &[(&str, &str)])]) -> Appearance {
    let owned = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    Appearance {
        scheme: scheme.into(),
        variants: variants.iter().map(|(s, pairs)| ((*s).to_owned(), owned(pairs))).collect(),
        ..Appearance::default()
    }
}

#[test]
fn scheme_colors_of_the_scheme_of_the_deck_are_none() {
    let light = appearance("light", &[]);
    assert_that!(scheme_colors(&light, PreviewScheme::Light)).is_none();
    // Sans rendu (apparence vide) : un deck clair.
    assert_that!(scheme_colors(&Appearance::default(), PreviewScheme::Light)).is_none();
    assert_that!(scheme_colors(&appearance("dark", &[]), PreviewScheme::Dark)).is_none();
}

#[test]
fn scheme_colors_take_the_other_scheme_of_the_theme() {
    let a = appearance("light", &[("dark", &[("piece", "#34312c"), ("sheet", "#2a2824"), ("ink", "#f1ece3")])]);
    let colors = scheme_colors(&a, PreviewScheme::Dark).unwrap();
    assert_that!(colors["bg"]).is_equal_to(json!("#34312c"));
    assert_that!(colors["surface"]).is_equal_to(json!("#2a2824"));
    assert_that!(colors["ink"]).is_equal_to(json!("#f1ece3"));
    // Absente du thème : la couleur de repli du schéma.
    assert_that!(colors["rule"].as_str().is_some_and(|c| c.starts_with('#'))).is_true();
}

#[test]
fn scheme_colors_without_the_other_scheme_fall_back_to_neutral_colors() {
    let dark = scheme_colors(&appearance("light", &[]), PreviewScheme::Dark).unwrap();
    let light = scheme_colors(&appearance("dark", &[]), PreviewScheme::Light).unwrap();
    for key in ["bg", "surface", "ink", "ink-muted", "rule"] {
        assert_that!(dark.contains_key(key)).is_true();
        assert_that!(light.contains_key(key)).is_true();
    }
    assert_that!(dark["bg"]).is_not_equal_to(light["bg"].clone());
}

#[test]
fn preview_look_reads_from_the_request() {
    let r: crate::Request =
        serde_json::from_value(json!({ "cmd": "preview_widget", "name": "kpi", "scheme": "dark", "ratio": "4:3" }))
            .unwrap();
    assert_that!(matches!(
        r,
        crate::Request::PreviewWidget {
            look: PreviewLook { scheme: Some(PreviewScheme::Dark), ratio: Some(PreviewRatio::Standard) },
            ..
        }
    ))
    .is_true();
    let wide: PreviewLook = serde_json::from_value(json!({ "ratio": "16:9" })).unwrap();
    assert_that!(wide).is_equal_to(look(None, Some(PreviewRatio::Wide)));
    assert_that!(serde_json::from_value::<PreviewLook>(json!({ "ratio": "3:2" })).is_err()).is_true();
}
