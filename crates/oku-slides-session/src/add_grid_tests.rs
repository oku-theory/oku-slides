use oku_slides_core::kinds;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::ElementSeed;
use crate::Session;
use crate::pack::{ESSENTIEL_PACK, PackManifest};
use crate::test_support::{add_pack, add_version, manifest_ns, rename_title, session, session_in_locale};

/// Un widget `badge` au paramètre `label` en texte simple.
const BADGE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("badge", params: (label: p.plain()),
  (args, ctx) => field(ctx, "label", args.label))
"#;

/// Une session dont le deck épingle le pack de widgets `namespace` de la bibliothèque, au widget `badge`
/// (`label: p.plain()`) et à la valeur d'exemple `example` de `widget`.
fn session_with_widget_pack_example(namespace: &str, widget: &str, example: Value) -> Session {
    let mut s = session();
    let mut m = manifest_ns(Uuid::new_v4(), namespace, &["base"]);
    let file = oku_slides_core::WidgetFile::parse(widget).unwrap();
    m.examples.insert(file, example.as_object().cloned().unwrap());
    let pin = add_version(&s.library, &m, &[(widget, BADGE)]);
    s.apply("Pack", vec![add_pack(&m, &pin)], None).unwrap_or_else(|e| panic!("pack : {}", e.one_line()));
    s
}

/// Édite Essentiel, puis l'enregistre comme copie nommée `name` ; le manifeste de la copie.
fn save_builtin_layouts_as_copy(s: &mut Session, name: &str) -> PackManifest {
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title(name)], None).unwrap();
    let saved = s.save_pack(true).unwrap();
    let files = s.doc.sources.packs.get(&saved).unwrap_or_else(|| panic!("copie embarquée"));
    PackManifest::from_files(files).unwrap()
}

/// Le départ que la grille d'ajout (`add_packs`) offre pour le widget `kind` ; `None` s'il n'y est pas.
fn seed_of(s: &Session, kind: &str) -> Option<ElementSeed> {
    s.add_packs().into_iter().flat_map(|p| p.widgets).find(|w| w.name == kind).map(|w| w.seed)
}

#[test]
fn add_packs_shape_seed_has_the_manifest_style() {
    let s = session();
    let seed = seed_of(&s, kinds::base::SHAPE).unwrap();
    assert_that!(serde_json::to_value(seed.style).unwrap()).is_equal_to(json!({"fill": "surface", "radius": "md"}));
}

#[test]
fn add_packs_card_seed_title_is_french_in_french() {
    let s = session_in_locale("fr");
    let seed = seed_of(&s, kinds::base::CARD).unwrap();
    assert_that!(seed.params["title"].as_str()).is_equal_to(Some("Titre de la carte"));
}

#[test]
fn add_packs_text_seed_body_is_english_in_english() {
    let s = session_in_locale("en");
    let seed = seed_of(&s, kinds::base::TEXT).unwrap();
    assert_that!(seed.params["body"][0]["runs"][0]["t"].as_str()).is_equal_to(Some("New text"));
}

#[test]
fn add_packs_layout_kind_is_not_offered() {
    let s = session();
    assert_that!(seed_of(&s, kinds::essentiel::CONTENT)).is_none();
}

#[test]
fn add_packs_seed_drops_a_value_the_schema_rejects() {
    let s = session_with_widget_pack_example("acme", "badge", json!({"label": 42}));
    assert_that!(seed_of(&s, "acme/badge").unwrap().params.contains_key("label")).is_false();
}

#[test]
fn add_packs_seed_keeps_a_value_the_schema_accepts() {
    let s = session_with_widget_pack_example("acme", "badge", json!({"label": "Neuf"}));
    assert_that!(seed_of(&s, "acme/badge").unwrap().params["label"].as_str()).is_equal_to(Some("Neuf"));
}

#[test]
fn add_packs_seed_without_style_serializes_no_style_key() {
    let s = session_with_widget_pack_example("acme", "badge", json!({"label": "Neuf"}));
    let seed = serde_json::to_value(seed_of(&s, "acme/badge").unwrap()).unwrap();
    assert_that!(seed.get("style").is_none()).is_true();
}

#[test]
fn add_packs_lists_base_with_seven_widgets_labelled() {
    // En anglais : les noms des widgets intégrés sont traduits.
    let s = session_in_locale("en");
    let base = s.add_packs().into_iter().find(|p| p.namespace == "base").unwrap();
    assert_that!(base.widgets).has_length(7);
    assert_that!(base.widgets.iter().find(|w| w.name == kinds::base::CARD).unwrap().label.as_str()).is_equal_to("Card");
}

#[test]
fn add_packs_describes_a_builtin_widget_in_the_user_language() {
    let s = session();
    let base = s.add_packs().into_iter().find(|p| p.namespace == "base").unwrap();
    let image = base.widgets.iter().find(|w| w.name == kinds::base::IMAGE).unwrap();
    assert_that!(image.description.as_deref()).is_equal_to(Some("Une image tirée d'un fichier"));
}

#[test]
fn add_packs_lists_a_pinned_library_pack_after_base() {
    let s = session_with_widget_pack_example("acme", "badge", json!({"label": "Neuf"}));
    let names: Vec<String> = s.add_packs().into_iter().map(|p| p.namespace).collect();
    assert_that!(names).is_equal_to(vec!["acme".to_owned(), "base".to_owned()]);
}

#[test]
fn add_packs_in_widget_pack_mode_lists_only_the_edited_pack() {
    let mut s = session();
    s.edit_pack(crate::pack::BASE_PACK).unwrap();
    let names: Vec<String> = s.add_packs().into_iter().map(|p| p.namespace).collect();
    assert_that!(names).is_equal_to(vec!["base".to_owned()]);
}

#[test]
fn free_slots_content_and_blank_accept_everything() {
    let slots = session().free_slots();
    assert_that!(slots.get(kinds::essentiel::CONTENT)).is_equal_to(Some(&None));
    assert_that!(slots.get(kinds::essentiel::BLANK)).is_equal_to(Some(&None));
    assert_that!(slots.contains_key(kinds::essentiel::TITLE)).is_false();
}

#[test]
fn saving_essentiel_as_copy_writes_plain_text_examples() {
    let mut s = session();
    let copy = save_builtin_layouts_as_copy(&mut s, "Ma copie");
    assert_that!(serde_json::to_string(&copy.examples).unwrap()).does_not_contain("ftl:");
}

#[test]
fn saving_essentiel_keeps_markers_of_untouched_examples() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let saved = s.save_pack(false).unwrap();
    let m = PackManifest::from_files(s.doc.sources.packs.get(&saved).unwrap()).unwrap();
    assert_that!(serde_json::to_string(&m.examples).unwrap()).contains("ftl:session-example-essentiel-end-text");
}

/// Le cache des départs n'accumule pas les versions : une autre version épinglée change le jeu de
/// packs, qui vide le cache (`install_widgets`).
#[test]
fn seed_cache_holds_one_entry_per_pinned_pack_across_versions() {
    let mut s = session();
    let (id, file) = (Uuid::new_v4(), oku_slides_core::WidgetFile::parse("badge").unwrap());
    for label in ["Un", "Deux"] {
        let mut m = manifest_ns(id, "acme", &["base"]);
        m.examples.insert(file.clone(), json!({ "label": label }).as_object().cloned().unwrap());
        let pin = add_version(&s.library, &m, &[("badge", BADGE)]);
        s.apply("Pack", vec![add_pack(&m, &pin)], None).unwrap_or_else(|e| panic!("pack : {}", e.one_line()));
        assert_that!(seed_of(&s, "acme/badge").map(|seed| seed.params["label"].clone()))
            .is_equal_to(Some(json!(label)));
    }
    let cached = s.seed_cache.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len();
    // `base` et `acme`, une langue.
    assert_that!(cached).is_equal_to(2);
}
