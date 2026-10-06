//! Le widget exemple du README de `typst/oku-slides` : il est la fixture, et il se rend.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, WidgetFile};
use oku_slides_render::{Fonts, PackWidgets, Renderer, load_registry, probe_file};
use speculoos::prelude::*;

const KPI: &str = include_str!("../../../fixtures/widgets/kpi/kpi.typ");
const COMMON: &str = include_str!("../../../fixtures/widgets/kpi/_commun.typ");
const README: &str = include_str!("../../../typst/oku-slides/README.md");

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap_or_else(|| panic!("nom de widget invalide : {name}"))
}

#[test]
fn readme_example_widget_is_the_fixture() {
    assert_that!(README.contains(KPI)).is_true();
    assert_that!(README.contains(COMMON)).is_true();
}

#[test]
fn readme_example_widget_probes_valid_and_renders_its_fields() {
    let fonts = Fonts::default();
    assert_that!(probe_file(&fonts, &wf("kpi"), KPI, Some(COMMON)).is_valid()).is_true();
    let files = BTreeMap::from([(wf("kpi"), KPI.to_owned()), (wf("_commun"), COMMON.to_owned())]);
    let pack = PackWidgets::new("exemple", files).unwrap();
    let registry = Arc::new(load_registry(&fonts, Some(&pack)).unwrap());
    let mut r = Renderer::new(fonts).unwrap();
    r.set_widgets(Some(&pack), registry).unwrap();
    let deck = Deck::from_json(
        r#"{"version":4,"slides":[{"id":"s","kind":"content","params":{"blocks":[
            {"id":"k","kind":"kpi","params":{"value":"42 %","label":"Croissance"}}]}}]}"#,
    )
    .unwrap();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
    let fields: Vec<&str> = compiled.layout().fields.iter().map(|f| f.field.as_str()).collect();
    assert_that!(fields).contains("value");
    assert_that!(fields).contains("label");
}
