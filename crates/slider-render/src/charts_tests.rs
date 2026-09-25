use serde_json::json;
use slider_chart::ChartLabels;
use slider_i18n::Localizer;
use speculoos::prelude::*;

use super::{Charts, SVG_FIELD};
use crate::fonts::Fonts;
use crate::world::file_id;

fn deck(option: serde_json::Value, frame: Option<serde_json::Value>) -> serde_json::Value {
    let mut chart = json!({ "id": "g", "kind": "chart", "option": option });
    if let Some(f) = frame {
        chart["frame"] = f;
    }
    json!({ "slides": [{ "id": "s", "kind": "content",
        "blocks": [{ "id": "grp", "kind": "group", "children": [chart] }] }] })
}

fn bars() -> serde_json::Value {
    json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Janvier", 3], ["Février", 5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    })
}

fn key(deck: &serde_json::Value) -> Option<String> {
    deck["slides"][0]["blocks"][0]["children"][0][SVG_FIELD].as_str().map(str::to_owned)
}

fn svg(charts: &Charts, key: &str) -> String {
    let id = file_id(&format!("/charts/{key}.svg")).unwrap();
    String::from_utf8(charts.file(id).unwrap().to_vec()).unwrap()
}

#[test]
fn attach_nested_chart_renders_its_svg() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, None, &Fonts::default()).unwrap();
    let k = key(&d).unwrap();
    let svg = svg(&charts, &k);
    assert_that!(svg).starts_with("<svg");
    assert_that!(svg).contains("Janvier");
}

#[test]
fn attach_same_chart_twice_keeps_one_file_and_key() {
    let mut charts = Charts::default();
    let mut a = deck(bars(), None);
    charts.attach(&mut a, None, &Fonts::default()).unwrap();
    let mut b = deck(bars(), None);
    charts.attach(&mut b, None, &Fonts::default()).unwrap();
    assert_that!(key(&a)).is_equal_to(key(&b));
    assert_that!(charts.files.len()).is_equal_to(1);
}

#[test]
fn attach_frame_size_changes_the_key_and_drops_the_old_file() {
    let mut charts = Charts::default();
    let mut small = deck(bars(), Some(json!({ "x": 0, "y": 0, "w": 400, "h": 300 })));
    charts.attach(&mut small, None, &Fonts::default()).unwrap();
    let mut large = deck(bars(), Some(json!({ "x": 0, "y": 0, "w": 900, "h": 500 })));
    charts.attach(&mut large, None, &Fonts::default()).unwrap();
    assert_that!(key(&small)).is_not_equal_to(key(&large));
    assert_that!(charts.files.len()).is_equal_to(1);
    assert_that!(svg(&charts, &key(&large).unwrap())).contains("width=\"900\"");
}

#[test]
fn attach_unreadable_option_leaves_no_key() {
    let mut charts = Charts::default();
    let mut d = deck(json!({ "series": "pas une liste" }), None);
    charts.attach(&mut d, None, &Fonts::default()).unwrap();
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
    charts.attach(&mut d, None, &Fonts::default()).unwrap();
    assert_that!(svg(&charts, &key(&d).unwrap())).contains("Série 2");
}

#[test]
fn set_labels_then_attach_redraws_with_new_key() {
    let mut charts = Charts::default();
    let mut d = deck(bars(), None);
    charts.attach(&mut d, None, &Fonts::default()).unwrap();
    let english = key(&d);
    charts.set_labels(ChartLabels::new(&Localizer::new(Some("fr"))));
    charts.attach(&mut d, None, &Fonts::default()).unwrap();
    assert_that!(key(&d)).is_not_equal_to(english);
    assert_that!(charts.files.len()).is_equal_to(1);
}
