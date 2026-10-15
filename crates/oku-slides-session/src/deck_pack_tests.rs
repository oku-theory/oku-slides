use oku_slides_core::{Op, PackRef, PackVersion};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library::builtin_versions;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{add_version, fresh_dir, library_of, manifest, session_on};

#[test]
fn new_session_blank_deck_embeds_and_references_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(PackRef { id: DEFAULT_PACK, version: builtin.clone() }));
    assert_that!(s.sources_for_tests().packs.contains_key(&builtin)).is_true();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn new_deck_takes_the_current_version_of_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    let mine =
        add_version(library_of(&s), &manifest(DEFAULT_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    library_of(&s).set_current(DEFAULT_PACK, &mine.version).unwrap();
    s.new_deck().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(mine.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&mine.version)).is_true();
}

#[test]
fn json_deck_without_templates_adopts_the_default_pack() {
    let s = crate::test_support::session();
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
    assert_that!(s.sources_for_tests().packs.len()).is_equal_to(1);
}

#[test]
fn set_templates_embeds_the_library_version_in_the_sources() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = add_version(library_of(&s), &manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z"), &[]);
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r.clone()) }], None).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(r.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&r.version)).is_true();
}

#[test]
fn set_templates_to_null_references_the_default_pack_explicitly() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = add_version(library_of(&s), &manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z"), &[]);
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    s.apply("Défaut", vec![Op::SetTemplates { templates: None }], None).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
}

#[test]
fn set_templates_to_an_unknown_version_returns_pack_version_missing() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = PackRef { id: DEFAULT_PACK, version: PackVersion::from_hex(&"0".repeat(64)).unwrap() };
    let err = s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}

/// t-328 : la version embarquée appartient à un autre pack que la référence.
#[test]
fn set_templates_whose_version_belongs_to_another_pack_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let before = s.deck().templates.clone();
    let builtin = builtin_versions()[0].version.clone();
    let other = Uuid::new_v4();
    let r = PackRef { id: other, version: builtin };
    let err = s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidDocument);
    let shown = oku_slides_i18n::Localize::localize(&err, &s.localizer);
    assert_that!(shown).contains(other.to_string().as_str());
    assert_that!(shown).contains(DEFAULT_PACK.to_string().as_str());
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
}

/// t-355 : un deck qui référence une version du pack par défaut absente de ses
/// sources comme de la bibliothèque s'ouvre, avec l'avis du pack manquant ; sa
/// référence reste la sienne (pas de repli muet sur la version courante).
#[test]
fn load_of_a_deck_whose_default_pack_version_is_missing_keeps_it_and_says_so() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let absent = PackRef { id: DEFAULT_PACK, version: PackVersion::from_hex(&"0".repeat(64)).unwrap() };
    let mut deck = s.deck().clone();
    deck.templates = Some(absent.clone());
    let sources = oku_slides_core::SourceBundle::new(deck, s.registry.clone()).unwrap();
    s.load(sources, None).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(absent.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&absent.version)).is_false();
    let missing = s.localizer.tr("session-pack-missing");
    assert_that!(s.doc.notice.clone().unwrap_or_default()).contains(missing.as_str());
}
