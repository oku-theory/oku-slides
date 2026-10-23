use serde_json::json;
use speculoos::prelude::*;

use super::*;

/// Un widget de slide `name` au paramètre texte `param`.
fn schema(name: &str, param: &str) -> WidgetSchema {
    serde_json::from_value(json!({ "name": name, "category": "slide", "params": [{ "name": param, "type": "plain" }] }))
        .unwrap_or_else(|e| panic!("schéma : {e}"))
}

/// Le registre sondé d'un pack généré : `schemas` sous l'espace de noms `deck-layouts`.
fn registry(schemas: &[WidgetSchema]) -> WidgetRegistry {
    let probe = json!({ "packs": [{ "namespace": "deck-layouts", "kind": "layouts", "widgets": schemas }] });
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("registre : {e}"))
}

#[test]
fn identity_of_the_same_bytes_is_the_same_v8_uuid() {
    let pack = identity(b"pptx");
    assert_that!(identity(b"pptx")).is_equal_to(pack);
    assert_that!(pack.get_version_num()).is_equal_to(8);
    assert_that!(identity(b"autre pptx")).is_not_equal_to(pack);
}

#[test]
fn check_probe_of_the_declared_schemas_passes() {
    let declared = [schema("l1-titre", "titre"), schema("l2-corps", "corps")];
    let probed = registry(&[declared[0].clone(), declared[1].clone(), schema("text", "body")]);
    assert_that!(check_probe(&probed, &declared, &[])).is_ok();
}

#[test]
fn check_probe_names_a_missing_widget() {
    let declared = [schema("l1-titre", "titre"), schema("l2-corps", "corps")];
    let probed = registry(&[declared[0].clone()]);
    assert_that!(check_probe(&probed, &declared, &[])).is_err_containing("l2-corps".to_owned());
}

#[test]
fn check_probe_names_a_widget_probed_differently() {
    let declared = [schema("l1-titre", "titre")];
    let probed = registry(&[schema("l1-titre", "title")]);
    assert_that!(check_probe(&probed, &declared, &[])).is_err_containing("l1-titre".to_owned());
}

#[test]
fn check_probe_names_a_pack_file_without_declared_schema() {
    let declared = [schema("l1-titre", "titre")];
    let probed = registry(&[declared[0].clone(), schema("l2-corps", "corps")]);
    let files: Vec<WidgetFile> = ["l1-titre", "l2-corps"].iter().map(|n| WidgetFile::parse(n).unwrap()).collect();
    assert_that!(check_probe(&probed, &declared, &files)).is_err_containing("l2-corps".to_owned());
    let declared_only = [WidgetFile::common(), files[0].clone()];
    assert_that!(check_probe(&probed, &declared, &declared_only)).is_ok();
}

#[test]
fn pack_limits_of_the_default_config_are_those_of_the_projector() {
    let settings = oku_slides_platform::config::Settings::defaults().unwrap();
    assert_that!(pack_limits(&settings.render)).is_equal_to(PackLimits::default());
}

/// Un pack de `_commun` et des widgets `names`, chaque fichier de `len` octets.
fn sizes(names: &[&str], len: usize) -> BTreeMap<WidgetFile, usize> {
    let names = names.iter().map(|n| WidgetFile::parse(n).unwrap()).chain([WidgetFile::common()]);
    names.map(|n| (n, len)).collect()
}

#[test]
fn beyond_limits_names_a_file_past_the_loaded_count() {
    let limits = PackLimits { max_files: 2, max_file_bytes: 100 };
    let pack = sizes(&["l1-a", "l2-b"], 10);
    // `_commun` et `l1-a` sont chargés, `l2-b` est le troisième fichier.
    assert_that!(beyond_limits("l1-a", &pack, &BTreeSet::new(), limits)).is_false();
    assert_that!(beyond_limits("l2-b", &pack, &BTreeSet::new(), limits)).is_true();
}

#[test]
fn beyond_limits_names_a_too_large_file_or_common() {
    let limits = PackLimits { max_files: 64, max_file_bytes: 100 };
    let mut pack = sizes(&["l1-a", "l2-b"], 10);
    pack.insert(WidgetFile::parse("l2-b").unwrap(), 101);
    assert_that!(beyond_limits("l1-a", &pack, &BTreeSet::new(), limits)).is_false();
    assert_that!(beyond_limits("l2-b", &pack, &BTreeSet::new(), limits)).is_true();
    pack.insert(WidgetFile::common(), 101);
    assert_that!(beyond_limits("l1-a", &pack, &BTreeSet::new(), limits)).is_true();
}

#[test]
fn beyond_limits_names_an_unprobed_file() {
    let pack = sizes(&["l1-a"], 10);
    let unprobed = BTreeSet::from([WidgetFile::parse("l1-a").unwrap()]);
    assert_that!(beyond_limits("l1-a", &pack, &unprobed, PackLimits::default())).is_true();
}
