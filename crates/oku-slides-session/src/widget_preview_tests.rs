use serde_json::{Map, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamValue, WidgetFile, WidgetSchema};
use oku_slides_render::RenderError;

use super::{PreviewFormat, failure, preview_params};
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
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
}

#[test]
fn preview_widget_png_is_an_image_for_the_agent() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.png.unwrap_or_default().starts_with(b"\x89PNG")).is_true();
}

#[test]
fn preview_widget_after_a_worker_restart_still_renders() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    s.worker.kill_for_test();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
}

#[test]
fn preview_widget_of_a_broken_file_returns_its_diagnostics_without_image() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget(None, "casse", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
}

#[test]
fn preview_widget_of_common_or_unknown_file_is_refused() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "_commun", "#let x = 1".into()).unwrap();
    let common = s.preview_widget(None, "_commun", &Map::new(), PreviewFormat::Svg).unwrap_err();
    let unknown = s.preview_widget(None, "absent", &Map::new(), PreviewFormat::Svg).unwrap_err();
    assert_that!(common.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
    assert_that!(unknown.kind()).is_equal_to(crate::SessionErrorKind::UnknownWidget);
}

#[test]
fn preview_widget_that_fails_in_typst_returns_its_diagnostics() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "rate", FAILS_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget(None, "rate", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error && d.message.contains("aperçu raté"))).is_true();
}

#[test]
fn preview_widget_that_loops_returns_a_diagnostic_and_keeps_the_pack() {
    let data = fresh_dir("oku-slides-preview-");
    let mut settings = settings_on(data.path());
    settings.render.timeout_ms = 2000;
    settings.render.probe_budget_ms = 2000;
    let mut s = session_with(settings);
    s.set_widget(None, "boucle", LOOP_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget(None, "boucle", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
    // Le pack n'est pas écarté : le deck garde ses widgets.
    let pack = s.packs.packs().first().cloned().unwrap();
    assert_that!(s.worker.is_faulty(&pack)).is_false();
    assert_that!(s.registry.get("deck/boucle")).is_some();
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
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
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
    s.set_widget(None, "couverture", COVER.into()).unwrap();
    // Un widget de slide va au pack de layouts propre du deck.
    let own = oku_slides_core::Namespace::deck_layouts();
    let preview = s.preview_widget(Some(&own), "couverture", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.is_some()).is_true();
    assert_that!(preview.bbox).is_none();
}

#[test]
fn preview_keeps_the_bare_slide_png_between_previews() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let first = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    let kept = s.widget_preview.as_ref().and_then(|p| p.bare.clone()).unwrap();
    // Une autre source du widget montré : la slide sans éléments ne change pas.
    s.set_widget(None, "quote", QUOTE.replace("emph", "strong")).unwrap();
    let second = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.widget_preview.as_ref().and_then(|p| p.bare.clone())).is_equal_to(Some(kept));
    assert_that!(first.bbox.is_some() && second.bbox.is_some()).is_true();
}

#[test]
fn png_preview_for_the_agent_has_no_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Png).unwrap();
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
    s.set_widget(None, "logo", LOGO.into()).unwrap();
    let id = s.doc.sources.add_asset(red_png());
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget(None, "logo", &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    // L'image, pas le cadre « image manquante ».
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}

#[test]
fn preview_widget_in_pack_mode_shows_an_asset_of_the_set_aside_deck() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let id = s.doc.sources.add_asset(red_png());
    let set = crate::BASE_PACK;
    s.edit_pack(set).unwrap();
    s.set_widget(None, "logo", LOGO.into()).unwrap();
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget(None, "logo", &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}

#[test]
fn preview_widget_in_history_mode_shows_an_asset_of_the_deck() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "logo", LOGO.into()).unwrap();
    s.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    s.history_open().unwrap();
    // L'historique a pris les images du deck mis de côté (`history.rs`) :
    // celle-ci y est, sans rendu qui l'aurait déjà envoyée au processus de rendu.
    let id = s.doc.sources.add_asset(red_png());
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "asset": id })).unwrap();
    let preview = s.preview_widget(None, "logo", &given, PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    assert_that!(preview.svg.unwrap_or_default().contains("<image")).is_true();
}
fn in_pack_mode_with_quote() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let set = crate::BASE_PACK;
    s.edit_pack(set).unwrap();
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
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
    let (_d, mut s) = in_pack_mode_with_quote();
    let plain = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    s.set_example_values("quote", body("Une autre")).unwrap();
    let tried = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_not_equal_to(plain);
}

#[test]
fn preview_widget_with_an_example_value_renders_like_the_given_value() {
    let (_d, mut s) = in_pack_mode_with_quote();
    let explicit = s.preview_widget(None, "quote", &body("Une autre"), PreviewFormat::Svg).unwrap().svg;
    s.set_example_values("quote", body("Une autre")).unwrap();
    let tried = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_equal_to(explicit);
}

#[test]
fn preview_inputs_given_value_wins_over_the_example() {
    let (_d, mut s) = in_pack_mode_with_quote();
    let quote = parse_name("quote").unwrap();
    s.set_example_values("quote", body("Essai")).unwrap();
    let schema = s.valid_schema(&quote).unwrap().unwrap();
    let inputs = s.preview_inputs(None, &quote, &schema, &body("Donné")).unwrap();
    assert_that!(inputs.get("body")).is_equal_to(body("Donné").get("body"));
}

#[test]
fn preview_widget_twice_keeps_the_examples_revision() {
    let (_d, mut s) = in_pack_mode_with_quote();
    s.set_example_values("quote", body("Essai")).unwrap();
    let before = s.snapshot().widget_trials;
    s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.snapshot().widget_trials).is_equal_to(before);
}

#[test]
fn set_widget_keeps_an_example_value_still_valid() {
    let (_d, mut s) = in_pack_mode_with_quote();
    s.set_example_values("quote", body("Essai")).unwrap();
    s.set_widget(None, "quote", QUOTE_V2.into()).unwrap();
    let quote = parse_name("quote").unwrap();
    assert_that!(s.example_values(None, &quote).unwrap().contains_key("body")).is_true();
}

/// Une valeur que le schéma n'admet plus reste dans le pack, mais l'aperçu l'ignore.
#[test]
fn preview_inputs_ignore_an_example_value_whose_type_changed() {
    let (_d, mut s) = in_pack_mode_with_quote();
    let values: Map<String, serde_json::Value> = serde_json::from_value(json!({ "source": "Livre" })).unwrap();
    s.set_example_values("quote", values).unwrap();
    s.set_widget(None, "quote", QUOTE_V2.into()).unwrap();
    let quote = parse_name("quote").unwrap();
    let schema = s.valid_schema(&quote).unwrap().unwrap();
    assert_that!(s.example_values(None, &quote).unwrap().contains_key("source")).is_true();
    assert_that!(s.preview_inputs(None, &quote, &schema, &Map::new()).unwrap().contains_key("source")).is_false();
}

#[test]
fn rename_widget_moves_its_example_values() {
    let (_d, mut s) = in_pack_mode_with_quote();
    s.set_example_values("quote", body("Essai")).unwrap();
    s.rename_widget(None, "quote", "citation").unwrap();
    let citation = parse_name("citation").unwrap();
    assert_that!(s.example_values(None, &citation).unwrap().contains_key("body")).is_true();
}

/// Un widget d'élément de 200 × 60 pt : un titre et un corps.
const PANEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("panneau", params: (title: p.plain(), body: p.rich()),
  (args, ctx) => block(width: 200pt, height: 60pt, {
    field(ctx, "title", args.title)
    field(ctx, "body", args.body)
  }))
"#;

fn field_names(view: &serde_json::Value) -> Vec<String> {
    view["fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn preview_of_an_element_widget_gives_its_fields_inside_the_box() {
    let data = fresh_dir("oku-slides-preview-");
    let mut session = opened_on(data.path());
    session.set_widget(None, "panneau", PANEL.into()).unwrap();
    let preview = session.preview_widget(None, "panneau", &Map::new(), PreviewFormat::Svg).unwrap();
    let bbox = preview.bbox.unwrap();
    let view = preview.view.unwrap();
    assert_that!(field_names(&view)).is_equal_to(vec!["title".to_owned(), "body".to_owned()]);
    assert_that!(view["width"]).is_equal_to(json!(1920.0));
    for entry in view["fields"].as_array().unwrap() {
        let (x, y) = (entry["x"].as_f64().unwrap(), entry["y"].as_f64().unwrap());
        let (w, h) = (entry["w"].as_f64().unwrap(), entry["h"].as_f64().unwrap());
        assert_that!(x).is_greater_than_or_equal_to(bbox.x - 1.0);
        assert_that!(y).is_greater_than_or_equal_to(bbox.y - 1.0);
        assert_that!(x + w).is_less_than_or_equal_to(bbox.x + bbox.w + 1.0);
        assert_that!(y + h).is_less_than_or_equal_to(bbox.y + bbox.h + 1.0);
        assert_that!(entry["bound"]).is_equal_to(json!(false));
    }
}

#[test]
fn preview_of_a_slide_widget_gives_its_fields() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "couverture", COVER.into()).unwrap();
    // Un widget de slide va au pack de layouts propre du deck.
    let own = oku_slides_core::Namespace::deck_layouts();
    let preview = s.preview_widget(Some(&own), "couverture", &Map::new(), PreviewFormat::Svg).unwrap();
    let view = preview.view.unwrap();
    assert_that!(field_names(&view)).is_equal_to(vec!["title".to_owned()]);
    assert_that!(view["targets"]).is_equal_to(json!([]));
}

#[test]
fn preview_of_a_broken_widget_has_no_view() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget(None, "casse", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.view).is_none();
}

#[test]
fn png_preview_for_the_agent_has_no_view() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "panneau", PANEL.into()).unwrap();
    let preview = s.preview_widget(None, "panneau", &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.view).is_none();
}

/// Un client d'avant t-526 envoie encore le schéma et le format de l'aperçu : la requête reste lisible.
#[test]
fn preview_widget_request_with_the_removed_look_still_parses() {
    let request = serde_json::from_value::<crate::Request>(
        json!({ "cmd": "preview_widget", "name": "kpi", "scheme": "dark", "ratio": "4:3" }),
    );
    assert_that!(matches!(request, Ok(crate::Request::PreviewWidget { ref name, .. }) if name == "kpi")).is_true();
}

/// Un widget `quote` d'un autre pack : un autre champ, `titre`.
const QUOTE_TITRE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (titre: p.plain()),
  (args, ctx) => field(ctx, "titre", args.titre))
"#;

/// Les champs (`widget`, `field`) de la vue d'un aperçu.
fn preview_fields(preview: &super::WidgetPreview) -> Vec<(String, String)> {
    let fields = preview.view.as_ref().and_then(|v| v["fields"].as_array().cloned()).unwrap_or_default();
    fields
        .iter()
        .map(|f| (f["widget"].as_str().unwrap_or("").to_owned(), f["field"].as_str().unwrap_or("").to_owned()))
        .collect()
}

#[test]
fn preview_widget_of_a_named_pack_renders_that_pack_widget_not_the_deck_one() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let ess = crate::test_support::ns("base");
    s.set_widget(Some(&ess), "quote", QUOTE_TITRE.into()).unwrap();
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(Some(&ess), "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview_fields(&preview)).is_equal_to(vec![("base/quote".to_owned(), "titre".to_owned())]);
    let deck = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview_fields(&deck)).is_equal_to(vec![("deck/quote".to_owned(), "body".to_owned())]);
}

/// t-1057 : l'aperçu prend le `base` qu'épingle le deck, pas le `base` intégré courant.
#[test]
fn preview_set_takes_the_deck_pinned_base_not_the_current_builtin() {
    let data = fresh_dir("oku-slides-preview-set-");
    let mut s = opened_on(data.path());
    let builtin = crate::test_support::builtin(crate::BASE_PACK);
    let mut widgets = builtin.files.widgets().clone();
    let marque = QUOTE.replace("\"quote\"", "\"marque\"");
    widgets.extend(crate::test_support::widget_files(&[("marque", marque.as_str())]));
    let files = oku_slides_core::PackFiles::new(builtin.files.kind(), builtin.files.manifest().clone(), widgets);
    crate::test_support::library_of(&s).add_version(crate::BASE_PACK, &files).unwrap();
    let pin = oku_slides_core::PackPin { id: crate::BASE_PACK, version: files.version(), requires: Vec::new() };
    let op =
        oku_slides_core::Op::AddPack { kind: files.kind(), namespace: oku_slides_core::kinds::base::namespace(), pin };
    s.apply("Base retouché", vec![op], None).unwrap();
    let set = s.preview_set(&oku_slides_core::Namespace::deck(), None).unwrap();
    let base = set
        .packs()
        .iter()
        .find(|p| p.namespace() == &oku_slides_core::kinds::base::namespace())
        .map(|p| p.id().to_owned());
    assert_that!(base).is_equal_to(Some(files.version().hex().to_owned()));
}

#[test]
fn preview_widget_of_an_element_in_a_widget_pack_has_a_box_and_fields() {
    let (_d, mut s) = in_pack_mode_with_quote();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.bbox.is_some()).is_true();
    assert_that!(preview.view.and_then(|v| v["fields"].as_array().map(Vec::len)).unwrap_or_default())
        .is_greater_than(0);
}

#[test]
fn preview_widget_of_an_element_in_a_copied_widget_pack_has_a_box_and_fields() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.edit_pack(crate::BASE_PACK).unwrap();
    s.save_pack(true).unwrap();
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget(None, "quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.bbox.is_some()).is_true();
    assert_that!(preview.view.and_then(|v| v["fields"].as_array().map(Vec::len)).unwrap_or_default())
        .is_greater_than(0);
}

#[test]
fn preview_widget_of_a_pinned_builtin_pack_by_namespace_and_file_renders() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let base = crate::test_support::ns("base");
    let preview = s.preview_widget(Some(&base), "card", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
}

#[test]
fn preview_widget_with_a_qualified_kind_as_name_is_refused() {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let base = crate::test_support::ns("base");
    let err = s.preview_widget(Some(&base), "base/card", &Map::new(), PreviewFormat::Svg).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidWidget);
}
