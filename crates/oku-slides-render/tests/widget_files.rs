//! Le widget exemple du README de `typst/oku-slides` : il est la fixture, et il se rend.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, Namespace, PackKind, WidgetFile, tokens::theme_layer};
use oku_slides_render::{BuiltinPack, Fonts, PackSet, PackWidgets, Renderer, load_registry, probe_file};
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
    assert_that!(probe_file(&fonts, PackKind::Widgets, &wf("kpi"), KPI, Some(COMMON)).is_valid()).is_true();
    let files = BTreeMap::from([(wf("kpi"), KPI.to_owned()), (wf("_commun"), COMMON.to_owned())]);
    let exemple = PackWidgets::new("exemple", Namespace::parse("exemple").unwrap(), PackKind::Widgets, files).unwrap();
    // La slide qui porte le bloc vient d'`essentiel`.
    let set = PackSet::new(vec![exemple, BuiltinPack::Essentiel.widgets().unwrap()]).unwrap_or_else(|e| panic!("{e}"));
    let registry = Arc::new(load_registry(&fonts, &set).unwrap());
    let mut r = Renderer::new(fonts).unwrap();
    r.set_widgets(&set, registry).unwrap();
    // La couche de thème porte les défauts de composant d'`essentiel` (sa slide les lit).
    let essentiel = (BuiltinPack::Essentiel.namespace(), BuiltinPack::Essentiel.component_defaults().unwrap());
    r.set_theme(Some(theme_layer([(&essentiel.0, essentiel.1)], []))).unwrap();
    let deck = Deck::from_json(&format!(
        r#"{{"version":{},"slides":[{{"id":"s","kind":"{}","params":{{"blocks":[
            {{"id":"k","kind":"exemple/kpi","params":{{"value":"42 %","label":"Croissance"}}}}]}}}}]}}"#,
        oku_slides_core::DECK_VERSION,
        oku_slides_core::kinds::essentiel::CONTENT
    ))
    .unwrap();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
    let fields: Vec<&str> = compiled.layout().fields.iter().map(|f| f.field.as_str()).collect();
    assert_that!(fields).contains("value");
    assert_that!(fields).contains("label");
}
