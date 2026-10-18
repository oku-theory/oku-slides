use oku_slides_core::{Op, PackPin, PackVersion, SourceBundle};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::SessionErrorKind;
use crate::library::builtin_versions;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{
    add_pack, add_version, fresh_dir, library_of, library_with, manifest, manifest_ns, ns, pin, session_on,
};

/// Des sources vierges : un deck vide, sans pack, sur le registre de base des tests.
fn blank_sources() -> SourceBundle {
    SourceBundle::new(oku_slides_core::Deck::empty(), crate::test_support::registry()).unwrap()
}

/// L'épinglage de la version intégrée du pack par défaut.
fn builtin_pin() -> PackPin {
    PackPin { id: DEFAULT_PACK, version: builtin_versions()[0].version.clone(), requires: Vec::new() }
}

#[test]
fn adopt_default_packs_empty_deck_pins_essentiel() {
    let (lib, mut sources) = (library_with(&[]), blank_sources());
    adopt_default_packs(&lib, &mut sources).unwrap();
    let pinned: Vec<String> = sources.deck().widgets.keys().map(ToString::to_string).collect();
    assert_that!(pinned).is_equal_to(vec!["essentiel".to_owned()]);
    assert_that!(sources.packs.contains_key(&builtin_pin().version)).is_true();
}

#[test]
fn adopt_default_packs_deck_with_a_pin_is_unchanged() {
    let (lib, mut sources) = (library_with(&[("acme", &[])]), blank_sources());
    let mut deck = sources.deck().clone();
    deck.widgets.insert(ns("acme"), crate::test_support::current_pin(&lib, "acme"));
    sources.apply_silent(deck).unwrap();
    adopt_default_packs(&lib, &mut sources).unwrap();
    let pinned: Vec<String> = sources.deck().widgets.keys().map(ToString::to_string).collect();
    assert_that!(pinned).is_equal_to(vec!["acme".to_owned()]);
}

#[test]
fn new_session_blank_deck_embeds_and_pins_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let s = session_on(data.path());
    assert_that!(pin(&s, "essentiel")).is_equal_to(Some(builtin_pin()));
    assert_that!(s.sources_for_tests().packs.contains_key(&builtin_pin().version)).is_true();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn new_deck_takes_the_current_version_of_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let builtin = builtin_pin().version;
    let mine =
        add_version(library_of(&s), &manifest(DEFAULT_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    library_of(&s).set_current(DEFAULT_PACK, &mine.version).unwrap();
    s.new_deck().unwrap();
    assert_that!(pin(&s, "essentiel")).is_equal_to(Some(mine.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&mine.version)).is_true();
}

#[test]
fn json_deck_without_widgets_adopts_the_default_pack() {
    let s = crate::test_support::session();
    assert_that!(pin(&s, "essentiel").map(|p| p.id)).is_equal_to(Some(DEFAULT_PACK));
    assert_that!(s.sources_for_tests().packs.len()).is_equal_to(1);
}

#[test]
fn add_pack_embeds_the_library_version_in_the_sources() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let r = add_version(library_of(&s), &m, &[]);
    s.apply("Pack", vec![add_pack(&m, &r)], None).unwrap();
    assert_that!(s.deck().widgets.get(&m.namespace).cloned()).is_equal_to(Some(r.clone()));
    assert_that!(pin(&s, "essentiel")).is_equal_to(Some(builtin_pin()));
    assert_that!(s.sources_for_tests().packs.contains_key(&r.version)).is_true();
}

#[test]
fn add_pack_also_pins_and_embeds_its_missing_requirements() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let base = manifest_ns(Uuid::new_v4(), "base", &[]);
    let base_pin = add_version(library_of(&s), &base, &[]);
    let acme = manifest_ns(Uuid::new_v4(), "acme", &["base"]);
    let acme_pin = add_version(library_of(&s), &acme, &[]);
    s.apply("Pack", vec![add_pack(&acme, &acme_pin)], None).unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(base_pin.clone()));
    assert_that!(pin(&s, "acme")).is_equal_to(Some(acme_pin));
    assert_that!(s.sources_for_tests().packs.contains_key(&base_pin.version)).is_true();
}

#[test]
fn add_pack_to_an_unknown_version_returns_pack_version_missing() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = PackPin { version: PackVersion::from_hex(&"0".repeat(64)).unwrap(), ..builtin_pin() };
    let err = s.apply("Pack", vec![Op::AddPack { namespace: ns("essentiel"), pin: r }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackVersionMissing);
}

/// t-328 : la version embarquée appartient à un autre pack que l'épinglage.
#[test]
fn add_pack_whose_version_belongs_to_another_pack_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let before = s.deck().widgets.clone();
    let other = Uuid::new_v4();
    let r = PackPin { id: other, ..builtin_pin() };
    let err = s.apply("Pack", vec![Op::AddPack { namespace: ns("essentiel"), pin: r }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidDocument);
    let shown = oku_slides_i18n::Localize::localize(&err, &s.localizer);
    assert_that!(shown).contains(other.to_string().as_str());
    assert_that!(shown).contains(DEFAULT_PACK.to_string().as_str());
    assert_that!(s.deck().widgets.clone()).is_equal_to(before);
}

#[test]
fn add_pack_under_another_namespace_than_its_manifest_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let before = s.deck().widgets.clone();
    let err = s.apply("Pack", vec![Op::AddPack { namespace: ns("autre"), pin: builtin_pin() }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidDocument);
    assert_that!(s.deck().widgets.clone()).is_equal_to(before);
}

/// t-355 : un deck qui épingle une version du pack par défaut absente de ses
/// sources comme de la bibliothèque s'ouvre, avec l'avis du pack écarté ; son
/// épinglage reste le sien (pas de repli muet sur la version courante).
#[test]
fn load_of_a_deck_whose_default_pack_version_is_missing_keeps_it_and_says_so() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let absent = PackPin { version: PackVersion::from_hex(&"0".repeat(64)).unwrap(), ..builtin_pin() };
    let mut deck = s.deck().clone();
    deck.widgets.insert(ns("essentiel"), absent.clone());
    let sources = SourceBundle::new(deck, s.registry.clone()).unwrap();
    s.load(sources, None).unwrap();
    assert_that!(pin(&s, "essentiel")).is_equal_to(Some(absent.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&absent.version)).is_false();
    let disabled = s.localizer.tr_with("session-packs-disabled", &[("packs", "essentiel".into())]);
    assert_that!(s.doc.notice.clone().unwrap_or_default()).contains(disabled.as_str());
}
