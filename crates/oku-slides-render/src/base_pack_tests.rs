use std::collections::BTreeSet;

use speculoos::prelude::*;

use super::*;
use crate::{Fonts, PackSet, builtin_registry, core_registry, load_registry};
use oku_slides_core::{Category, Grow, Namespace, PackManifest, WidgetRegistry, kinds::base};

fn base_registry_of_pack() -> WidgetRegistry {
    load_registry(&Fonts::default(), &PackSet::new(vec![base_pack_widgets().unwrap()]).unwrap()).unwrap()
}

#[test]
fn base_pack_files_read_as_the_base_manifest() {
    let m = PackManifest::from_files(&base_pack_files().unwrap()).unwrap();
    assert_that!((m.namespace, m.requires.is_empty())).is_equal_to((base::namespace(), true));
}

#[test]
fn base_pack_registers_exactly_the_seven_qualified_widgets() {
    let r = base_registry_of_pack();
    let names: BTreeSet<&str> = r.iter().filter(|w| w.name.starts_with("base/")).map(|w| w.name.as_str()).collect();
    assert_that!(names).is_equal_to(base::ALL.into_iter().collect::<BTreeSet<_>>());
}

#[test]
fn core_registry_holds_the_six_slide_widgets_only() {
    let r = core_registry(&Fonts::default()).unwrap();
    let names: BTreeSet<&str> = r.iter().map(|w| w.name.as_str()).collect();
    let slides = BTreeSet::from(["title", "toc", "chapter", "content", "end", "blank"]);
    assert_that!((names, r.iter().all(|w| w.category == Category::Slide))).is_equal_to((slides, true));
}

#[test]
fn base_widgets_declare_their_grow() {
    let r = base_registry_of_pack();
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
    let r = base_registry_of_pack();
    let typo: Vec<&str> = base::ALL.into_iter().filter(|k| r.get(k).is_some_and(|w| w.typography)).collect();
    assert_that!(typo).is_equal_to(vec![base::TEXT]);
}

#[test]
fn base_pack_embeds_every_file_of_its_directory() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../typst/oku-slides/packs/base/widgets");
    let on_disk: BTreeSet<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    let embedded: BTreeSet<String> = BASE_WIDGETS.iter().map(|(n, _)| format!("{n}.typ")).collect();
    assert_that!(embedded).is_equal_to(on_disk);
}

#[test]
fn builtin_registry_is_core_plus_base() {
    let r = builtin_registry(&Fonts::default()).unwrap();
    assert_that!((r.get("content").is_some(), r.get(base::TABLE).is_some())).is_equal_to((true, true));
}

#[test]
fn base_versions_list_the_current_base_version() {
    let current = base_pack_files().unwrap().version().hex().to_owned();
    assert_that!(shipped_base_versions().any(|v| v == current)).is_true();
}

#[test]
fn is_trusted_pack_of_the_current_base_is_true() {
    assert_that!(is_trusted_pack(&base_pack_widgets().unwrap())).is_true();
}

#[test]
fn is_trusted_pack_of_an_edited_base_is_false() {
    let mut files = base_pack_files().unwrap().widgets().clone();
    files.insert(WidgetFile::parse("text").unwrap(), "// modifié".to_owned());
    let edited = PackFiles::new(base_pack_files().unwrap().manifest().clone(), files.clone());
    let pack = PackWidgets::new(edited.version().hex(), base::namespace(), files).unwrap();
    assert_that!(is_trusted_pack(&pack)).is_false();
}

#[test]
fn is_trusted_pack_of_another_namespace_with_the_base_id_is_false() {
    let base = base_pack_widgets().unwrap();
    let other = PackWidgets::new(base.id(), Namespace::parse("acme").unwrap(), base.files().clone()).unwrap();
    assert_that!(is_trusted_pack(&other)).is_false();
}
