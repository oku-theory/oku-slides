use std::path::{Path, PathBuf};

use speculoos::prelude::*;

use super::*;

#[test]
fn safe_stem_replaces_path_separators() {
    assert_that!(safe_stem("Q1/Q2 : plan", "x")).is_equal_to("Q1-Q2 - plan".to_owned());
}

#[test]
fn safe_stem_of_blank_title_uses_fallback() {
    assert_that!(safe_stem("  \n ", "Sans titre")).is_equal_to("Sans titre".to_owned());
}

#[test]
fn safe_stem_drops_leading_dots() {
    assert_that!(safe_stem("..cache", "x")).is_equal_to("cache".to_owned());
}

#[test]
fn safe_stem_cuts_very_long_titles() {
    assert_that!(safe_stem(&"a".repeat(500), "x").chars().count()).is_equal_to(MAX_STEM_CHARS);
}

#[test]
fn free_path_skips_taken_names() {
    let taken = |p: &Path| p.ends_with("Budget.oku-slides.pdf") || p.ends_with("Budget 2.oku-slides.pdf");
    assert_that!(free_path(Path::new("/d"), "Budget", taken)).is_equal_to(PathBuf::from("/d/Budget 3.oku-slides.pdf"));
}

/// A deck pinning an unfrozen widget draft, installed in another session: nothing is adopted.
#[test]
fn install_document_adopts_nothing() {
    let data = crate::test_support::fresh_dir("oku-slides-install-");
    let mut s = crate::test_support::session_on(data.path());
    let t = crate::test_support_packs::adopting_deck(&mut s, data.path());
    let acme = crate::test_support_packs::acme_of(&s);
    let v = s.user_deck().widgets.values().find(|p| p.id == acme).map(|p| p.version.clone()).unwrap();
    assert_that!(t.deck().all_pins().map(|(_, ns, p)| (ns.clone(), p.clone())).collect::<Vec<_>>())
        .is_equal_to(s.user_deck().all_pins().map(|(_, ns, p)| (ns.clone(), p.clone())).collect::<Vec<_>>());
    let write = t.library.draft_write(&crate::library::draft_lock(), acme, Some(&v));
    assert_that!(write).is_equal_to(crate::library::DraftWrite::Replace(v));
}

/// Characterization: a deck without packs, loaded, still adopts the default packs.
#[test]
fn load_document_still_adopts_its_default_packs() {
    let mut s = crate::test_support::session();
    let deck = oku_slides_core::Deck::from_json(r#"{"slides":[]}"#).unwrap();
    let sources = SourceBundle::new(deck, crate::test_support::registry()).unwrap();
    s.load(sources, None).unwrap();
    let base = oku_slides_core::kinds::base::namespace();
    assert_that!(s.deck().widgets.contains_key(&base)).is_true();
}
