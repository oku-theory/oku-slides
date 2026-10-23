use std::collections::BTreeSet;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{Fonts, PackSet, builtin_registry, load_registry};
use oku_slides_core::{Category, Grow, Namespace, PackKind, PackManifest, WidgetRegistry, kinds};
use serde_json::{Map, Value};

fn registry_of(pack: BuiltinPack) -> WidgetRegistry {
    load_registry(&Fonts::default(), &PackSet::new(vec![pack.widgets().unwrap()]).unwrap()).unwrap()
}

fn ns(name: &str) -> Namespace {
    Namespace::parse(name).unwrap()
}

#[rstest]
#[case(BuiltinPack::Base, "base", PackKind::Widgets, 7)]
#[case(BuiltinPack::Essentiel, "essentiel", PackKind::Layouts, 6)]
fn builtin_pack_files_have_their_namespace_kind_and_widgets(
    #[case] pack: BuiltinPack,
    #[case] namespace: &str,
    #[case] kind: PackKind,
    #[case] widgets: usize,
) {
    let files = pack.files().unwrap();
    let manifest = PackManifest::from_files(&files).unwrap();
    let count = files.widgets().keys().filter(|f| !f.is_common()).count();
    assert_that!((manifest.namespace.as_str(), files.kind(), count)).is_equal_to((namespace, kind, widgets));
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn builtin_pack_current_version_is_listed_as_shipped(#[case] pack: BuiltinPack) {
    let hex = pack.files().unwrap().version().hex().to_owned();
    asserting(&format!("{pack:?} : version courante {hex} absente de versions.txt"))
        .that(&pack.shipped_versions().any(|v| v == hex))
        .is_true();
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn builtin_pack_widgets_have_its_namespace_kind_and_version_id(#[case] pack: BuiltinPack) {
    let w = pack.widgets().unwrap();
    let hex = pack.files().unwrap().version().hex().to_owned();
    assert_that!((w.namespace().clone(), w.kind(), w.id().to_owned())).is_equal_to((
        pack.namespace(),
        pack.kind(),
        hex,
    ));
}

#[test]
fn essentiel_manifest_has_the_lot_3_id_and_requires_base() {
    let m = PackManifest::from_files(&BuiltinPack::Essentiel.files().unwrap()).unwrap();
    assert_that!((m.id.to_string(), m.requires.clone()))
        .is_equal_to(("d5eb8a57-07b4-47f7-a818-dde5347160cf".to_owned(), vec![kinds::base::namespace()]));
}

#[test]
fn essentiel_profiles_are_the_ten_former_templates() {
    let m = PackManifest::from_files(&BuiltinPack::Essentiel.files().unwrap()).unwrap();
    let ids: Vec<&str> = m.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec![
        "texte",
        "cartes",
        "colonnes",
        "citation",
        "message",
        "libre",
        "chapitre",
        "sommaire",
        "couverture",
        "fin",
    ]);
}

#[test]
fn essentiel_profiles_name_layouts_of_the_pack() {
    let files = BuiltinPack::Essentiel.files().unwrap();
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!(m.profiles.iter().all(|p| files.widget(&p.layout).is_some())).is_true();
}

#[test]
fn base_pack_registers_exactly_the_seven_qualified_widgets() {
    let r = registry_of(BuiltinPack::Base);
    let names: BTreeSet<&str> = r.iter().map(|w| w.name.as_str()).collect();
    assert_that!(names).is_equal_to(kinds::base::ALL.into_iter().collect::<BTreeSet<_>>());
}

#[test]
fn essentiel_pack_registers_exactly_the_six_slide_layouts() {
    let r = registry_of(BuiltinPack::Essentiel);
    let names: BTreeSet<&str> = r.iter().map(|w| w.name.as_str()).collect();
    let all_slides = r.iter().all(|w| w.category == Category::Slide);
    assert_that!((names, all_slides)).is_equal_to((kinds::essentiel::ALL.into_iter().collect::<BTreeSet<_>>(), true));
}

#[test]
fn base_widgets_declare_their_grow() {
    use kinds::base;
    let r = registry_of(BuiltinPack::Base);
    let grows: Vec<(&str, Grow)> = base::ALL.iter().map(|k| (*k, r.get(k).unwrap().grow)).collect();
    let expected = vec![
        (base::TEXT, Grow::Fixed),
        (base::CARD, Grow::Fixed),
        (base::IMAGE, Grow::Fill),
        (base::SHAPE, Grow::Fixed),
        (base::CHART, Grow::Fill),
        (base::TABLE, Grow::Fit),
        (base::STACK, Grow::Fixed),
    ];
    assert_that!(grows).is_equal_to(expected);
}

#[test]
fn only_base_text_declares_typography() {
    let r = registry_of(BuiltinPack::Base);
    let typo: Vec<&str> = kinds::base::ALL.into_iter().filter(|k| r.get(k).is_some_and(|w| w.typography)).collect();
    assert_that!(typo).is_equal_to(vec![kinds::base::TEXT]);
}

#[rstest]
#[case(BuiltinPack::Base, "base/widgets")]
#[case(BuiltinPack::Essentiel, "essentiel/layouts")]
fn builtin_pack_embeds_every_file_of_its_directory(#[case] pack: BuiltinPack, #[case] dir: &str) {
    let dir = format!("{}/../../typst/oku-slides/packs/{dir}", env!("CARGO_MANIFEST_DIR"));
    let on_disk: BTreeSet<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    let embedded: BTreeSet<String> = pack.files().unwrap().widgets().keys().map(|n| n.file_name()).collect();
    assert_that!(embedded).is_equal_to(on_disk);
}

#[test]
fn builtin_registry_is_base_plus_essentiel() {
    let r = builtin_registry(&Fonts::default()).unwrap();
    let found = (r.get(kinds::essentiel::CONTENT).is_some(), r.get(kinds::base::TABLE).is_some(), r.get("content"));
    assert_that!(found).is_equal_to((true, true, None));
}

#[test]
fn builtin_pack_set_holds_base_and_essentiel() {
    let set = builtin_pack_set().unwrap();
    let names: Vec<&str> = set.packs().iter().map(|p| p.namespace().as_str()).collect();
    assert_that!(names).is_equal_to(vec!["base", "essentiel"]);
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn is_trusted_pack_of_a_current_builtin_is_true(#[case] pack: BuiltinPack) {
    assert_that!(is_trusted_pack(&pack.widgets().unwrap())).is_true();
}

#[test]
fn is_trusted_pack_an_edited_essentiel_is_not_trusted() {
    let shipped = BuiltinPack::Essentiel.widgets().unwrap();
    let edited =
        PackWidgets::new("abc123", shipped.namespace().clone(), PackKind::Layouts, shipped.files().clone()).unwrap();
    assert_that!((is_trusted_pack(&shipped), is_trusted_pack(&edited))).is_equal_to((true, false));
}

#[test]
fn is_trusted_pack_of_an_edited_base_is_false() {
    let files = BuiltinPack::Base.files().unwrap();
    let mut widgets = files.widgets().clone();
    widgets.insert(WidgetFile::parse("text").unwrap(), "// modifié".to_owned());
    let edited = PackFiles::new(PackKind::Widgets, files.manifest().clone(), widgets.clone());
    let pack = PackWidgets::new(edited.version().hex(), kinds::base::namespace(), PackKind::Widgets, widgets).unwrap();
    assert_that!(is_trusted_pack(&pack)).is_false();
}

#[test]
fn is_trusted_pack_another_namespace_with_a_shipped_id_is_not_trusted() {
    let shipped = BuiltinPack::Base.widgets().unwrap();
    let claim = PackWidgets::new(shipped.id(), ns("acme"), PackKind::Widgets, shipped.files().clone()).unwrap();
    assert_that!(is_trusted_pack(&claim)).is_false();
}

#[test]
fn is_trusted_pack_essentiel_under_the_base_id_is_not_trusted() {
    let base = BuiltinPack::Base.widgets().unwrap();
    let essentiel = BuiltinPack::Essentiel.widgets().unwrap();
    let swapped =
        PackWidgets::new(base.id(), essentiel.namespace().clone(), PackKind::Layouts, essentiel.files().clone())
            .unwrap();
    assert_that!(is_trusted_pack(&swapped)).is_false();
}

#[rstest]
#[case(BuiltinPack::Base, &["card", "table"])]
#[case(BuiltinPack::Essentiel, &["chapter", "columns", "end-slide", "quote", "title", "title-slide", "toc"])]
fn builtin_pack_declares_its_component_groups(#[case] pack: BuiltinPack, #[case] groups: &[&str]) {
    let defaults = pack.component_defaults().unwrap();
    // Les clés suivent l'ordre du manifeste (`preserve_order`) : comparées triées.
    let mut keys = defaults.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_that!(keys).is_equal_to(groups.to_vec());
}

#[test]
fn default_tokens_have_no_component_and_the_four_frame_families() {
    let defaults: Map<String, Value> =
        serde_json::from_slice(include_bytes!("../../../typst/oku-slides/src/default-tokens.json")).unwrap();
    assert_that!(defaults.contains_key("component")).is_false();
    for family in ["header", "footer", "list", "kicker"] {
        assert_that!(defaults.contains_key(family)).is_true();
    }
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn builtin_pack_has_two_shipped_versions(#[case] pack: BuiltinPack) {
    assert_that!(pack.shipped_versions().count()).is_equal_to(2);
}

#[test]
fn builtin_component_defaults_lists_base_then_essentiel() {
    let names: Vec<String> =
        builtin_component_defaults().unwrap().into_iter().map(|(ns, _)| ns.as_str().to_owned()).collect();
    assert_that!(names).is_equal_to(vec!["base".to_owned(), "essentiel".to_owned()]);
}
