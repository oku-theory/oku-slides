use serde_json::{Map, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamValue, WidgetFile, WidgetSchema};
use oku_slides_render::RenderError;

use super::{PreviewFormat, failure, preview_params};
use crate::test_support::{fresh_dir, opened_on, session_with, settings_on};

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
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
}

#[test]
fn preview_widget_png_is_an_image_for_the_agent() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.png.unwrap_or_default().starts_with(b"\x89PNG")).is_true();
}

#[test]
fn preview_widget_after_a_worker_restart_still_renders() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    s.worker.kill_for_test();
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
}

#[test]
fn preview_widget_of_a_broken_file_returns_its_diagnostics_without_image() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget("casse", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
}

#[test]
fn preview_widget_of_common_or_unknown_file_is_refused() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("_commun", "#let x = 1".into()).unwrap();
    let common = s.preview_widget("_commun", &Map::new(), PreviewFormat::Svg).unwrap_err();
    let unknown = s.preview_widget("absent", &Map::new(), PreviewFormat::Svg).unwrap_err();
    assert_that!(common.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
    assert_that!(unknown.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
}

#[test]
fn preview_widget_that_fails_in_typst_returns_its_diagnostics() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("rate", FAILS_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget("rate", &Map::new(), PreviewFormat::Svg).unwrap();
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
    let preview = s.preview_widget("boucle", &Map::new(), PreviewFormat::Svg).unwrap();
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
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
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
    let preview = s.preview_widget("couverture", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.is_some()).is_true();
    assert_that!(preview.bbox).is_none();
}

#[test]
fn preview_keeps_the_bare_slide_png_between_previews() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let first = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    let kept = s.widget_preview.as_ref().and_then(|p| p.bare.clone()).unwrap();
    // Une autre source du widget montré : la slide sans éléments ne change pas.
    s.set_widget("quote", QUOTE.replace("emph", "strong")).unwrap();
    let second = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.widget_preview.as_ref().and_then(|p| p.bare.clone())).is_equal_to(Some(kept));
    assert_that!(first.bbox.is_some() && second.bbox.is_some()).is_true();
}

#[test]
fn png_preview_for_the_agent_has_no_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Png).unwrap();
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
    let preview = s.preview_widget("logo", &given, PreviewFormat::Svg).unwrap();
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
    let preview = s.preview_widget("logo", &given, PreviewFormat::Svg).unwrap();
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
    let preview = s.preview_widget("logo", &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}
