use oku_slides_chart::ChartLabels;
use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use oku_slides_core::WidgetRegistry;

use super::{Charts, SVG_FIELD, TextSizes, text_sizes};
use crate::fonts::Fonts;
use crate::world::file_id;

fn deck(option: serde_json::Value, frame: Option<serde_json::Value>) -> serde_json::Value {
    let mut chart = json!({ "id": "g", "kind": "chart", "params": { "option": option } });
    if let Some(f) = frame {
        chart["frame"] = f;
    }
    json!({ "slides": [{ "id": "s", "kind": "content", "params": {
        "blocks": [{ "id": "grp", "kind": "stack", "params": { "children": [chart] } }] } }] })
}

/// Registre de base, lu dans sa sonde figée (`fixtures/widgets`).
fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
}

fn bars() -> serde_json::Value {
    json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Janvier", 3], ["Février", 5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    })
}

fn key(deck: &serde_json::Value) -> Option<String> {
    deck["slides"][0]["params"]["blocks"][0]["params"]["children"][0][SVG_FIELD].as_str().map(str::to_owned)
}

fn svg(charts: &Charts, key: &str) -> String {
    let id = file_id(&format!("/charts/{key}.svg")).unwrap();
    String::from_utf8(charts.file(id).unwrap().to_vec()).unwrap()
}

#[test]
fn attach_nested_chart_renders_its_svg() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    let k = key(&d).unwrap();
    let svg = svg(&charts, &k);
    assert_that!(svg).starts_with("<svg");
    assert_that!(svg).contains("Janvier");
}

#[test]
fn attach_same_chart_twice_keeps_one_file_and_key() {
    let mut charts = Charts::default();
    let mut a = deck(bars(), None);
    charts.attach(&mut a, &Fonts::default(), &registry()).unwrap();
    let mut b = deck(bars(), None);
    charts.attach(&mut b, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&a)).is_equal_to(key(&b));
    assert_that!(charts.files.len()).is_equal_to(1);
}

#[test]
fn attach_frame_size_changes_the_key_and_drops_the_old_file() {
    let mut charts = Charts::default();
    let mut small = deck(bars(), Some(json!({ "x": 0, "y": 0, "w": 400, "h": 300 })));
    charts.attach(&mut small, &Fonts::default(), &registry()).unwrap();
    let mut large = deck(bars(), Some(json!({ "x": 0, "y": 0, "w": 900, "h": 500 })));
    charts.attach(&mut large, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&small)).is_not_equal_to(key(&large));
    assert_that!(charts.files.len()).is_equal_to(1);
    assert_that!(svg(&charts, &key(&large).unwrap())).contains("width=\"900\"");
}

#[test]
fn attach_unreadable_option_leaves_no_key() {
    let mut charts = Charts::default();
    let mut d = deck(json!({ "series": "pas une liste" }), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&d)).is_none();
    assert_that!(charts.files.len()).is_equal_to(0);
}

#[test]
fn set_labels_names_unnamed_series_in_the_user_language() {
    let inline = json!({
        "xAxis": { "type": "category", "data": ["A", "B"] },
        "series": [{ "type": "bar", "data": [1, 2] }, { "type": "bar", "data": [2, 1] }]
    });
    let mut charts = Charts::default();
    charts.set_labels(ChartLabels::new(&Localizer::new(Some("fr"))));
    let mut d = deck(inline, None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    assert_that!(svg(&charts, &key(&d).unwrap())).contains("Série 2");
}

#[test]
fn set_labels_then_attach_redraws_with_new_key() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    let english = key(&d);
    charts.set_labels(ChartLabels::new(&Localizer::new(Some("fr"))));
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&d)).is_not_equal_to(english);
    assert_that!(charts.files.len()).is_equal_to(1);
}

#[test]
fn attach_after_set_palette_draws_with_that_palette() {
    let mut charts = Charts::default();
    charts.set_palette(vec!["#123456".into()]);
    let mut d = deck(bars(), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    let svg = svg(&charts, &key(&d).unwrap()).to_lowercase();
    assert_that!(svg.contains("#123456") || svg.contains("rgb(18,52,86")).is_true();
}

#[test]
fn set_palette_changes_the_key_and_reports_the_change() {
    let mut charts = Charts::default();
    let mut a = deck(bars(), None);
    charts.attach(&mut a, &Fonts::default(), &registry()).unwrap();
    assert_that!(charts.set_palette(vec!["#123456".into()])).is_true();
    assert_that!(charts.set_palette(vec!["#123456".into()])).is_false();
    let mut b = deck(bars(), None);
    charts.attach(&mut b, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&a)).is_not_equal_to(key(&b));
}

#[test]
fn attach_element_without_a_chart_param_gets_no_key() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    d["slides"][0]["params"]["blocks"][0]["params"]["children"][0]["kind"] = json!("text");
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&d)).is_none();
    assert_that!(charts.files.len()).is_equal_to(0);
}

#[test]
fn attach_pack_widget_with_a_host_chart_param_renders_it() {
    let probe = json!({ "base": [], "pack": [{ "name": "kpi", "category": "element",
        "params": [{ "name": "data", "type": "host", "host": "chart" }] }] });
    let reg = WidgetRegistry::from_probe(probe).unwrap();
    let mut d = json!({ "slides": [{ "id": "s", "kind": "blank", "params": {
        "elements": [{ "id": "k", "kind": "kpi", "params": { "data": bars() } }] } }] });
    let mut charts = Charts::default();
    charts.attach(&mut d, &Fonts::default(), &reg).unwrap();
    assert_that!(d["slides"][0]["params"]["elements"][0][SVG_FIELD].as_str()).is_some();
}

#[test]
fn attach_with_theme_font_draws_the_chart_in_that_font() {
    let mut charts = Charts::default();
    assert_that!(charts.set_font(Some("Inter".into()))).is_true();
    let inter = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/inter-400.ttf")).unwrap();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, &Fonts::new([inter]), &registry()).unwrap();
    let svg = svg(&charts, &key(&d).unwrap());
    assert_that!(svg.contains("Inter")).is_true();
}

#[test]
fn attach_without_theme_font_uses_libertinus() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    let svg = svg(&charts, &key(&d).unwrap());
    assert_that!(svg.contains("Libertinus Serif")).is_true();
}

#[test]
fn text_sizes_reads_body_and_lead_from_the_size_tokens() {
    let tokens = json!({ "size": { "body": 32, "lead": 40, "caption": 22 } });
    assert_that!(text_sizes(&tokens)).is_equal_to(Some(TextSizes { body: 32.0, title: 40.0 }));
}

#[test]
fn text_sizes_without_body_or_lead_is_none() {
    assert_that!(text_sizes(&json!({ "size": { "body": 32 } }))).is_none();
    assert_that!(text_sizes(&json!({ "color": {} }))).is_none();
}

#[test]
fn attach_after_set_text_sizes_draws_axes_and_legend_at_the_body_size() {
    let mut charts = Charts::default();
    assert_that!(charts.set_text_sizes(Some(TextSizes { body: 32.0, title: 40.0 }))).is_true();
    assert_that!(charts.set_text_sizes(Some(TextSizes { body: 32.0, title: 40.0 }))).is_false();
    let mut option = bars();
    option["title"] = json!({ "text": "Ventes" });
    let mut d = deck(option, None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    let svg = svg(&charts, &key(&d).unwrap());
    assert_that!(svg).contains("font-size=\"40\"");
    assert_that!(svg).contains("font-size=\"32\"");
    assert_that!(svg).does_not_contain("font-size=\"12\"");
}

fn with_lang(mut d: serde_json::Value, lang: &str) -> serde_json::Value {
    d["params"] = json!({ "lang": lang });
    d
}

fn labelled() -> serde_json::Value {
    json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Janvier", 12345.5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    })
}

#[test]
fn attach_deck_language_changes_the_key_and_the_numbers() {
    let mut charts = Charts::default();
    let mut fr = with_lang(deck(labelled(), None), "fr");
    charts.attach(&mut fr, &Fonts::default(), &registry()).unwrap();
    let fr_svg = svg(&charts, &key(&fr).unwrap());
    let mut en = with_lang(deck(labelled(), None), "en");
    charts.attach(&mut en, &Fonts::default(), &registry()).unwrap();
    assert_that!(key(&fr)).is_not_equal_to(key(&en));
    assert_that!(fr_svg).contains("12\u{202F}345,5");
    assert_that!(svg(&charts, &key(&en).unwrap())).contains("12,345.5");
}

#[test]
fn attach_deck_without_language_keeps_plain_numbers() {
    let mut charts = Charts::default();
    let mut d = deck(labelled(), None);
    charts.attach(&mut d, &Fonts::default(), &registry()).unwrap();
    assert_that!(svg(&charts, &key(&d).unwrap())).contains("12345.5");
}
