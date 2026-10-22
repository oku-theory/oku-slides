use std::collections::BTreeMap;

use oku_slides_core::{Namespace, PackPin, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::main_pack;

/// Des épingles `espace de noms → pack`, l'id du pack tiré de son rang plus `first`.
fn pins(namespaces: &[&str], first: u128) -> BTreeMap<Namespace, PackPin> {
    namespaces
        .iter()
        .enumerate()
        .map(|(i, ns)| {
            let ns = Namespace::parse(ns).unwrap_or_else(|| panic!("espace de noms : {ns}"));
            let version = PackVersion::from_hex(&"0".repeat(64)).unwrap_or_else(|| panic!("version de test"));
            (ns, PackPin { id: Uuid::from_u128(first + i as u128), version, requires: Vec::new() })
        })
        .collect()
}

/// Comme `firstPackId` : layouts propres, widgets propres, packs non intégrés (layouts d'abord),
/// Essentiel, puis `base`. Les ids des widgets partent de 1, ceux des layouts de 11.
#[rstest]
#[case::deck_layouts_first(&["base", "deck", "acme"], &["deck-layouts", "essentiel"], Some(11))]
#[case::deck_widgets_next(&["acme", "base", "deck"], &["essentiel"], Some(3))]
#[case::library_layouts_before_widgets(&["acme", "base"], &["essentiel", "maison"], Some(12))]
#[case::library_widgets(&["acme", "base"], &["essentiel"], Some(1))]
#[case::essentiel_before_base(&["base"], &["essentiel"], Some(11))]
#[case::base_alone(&["base"], &[], Some(1))]
#[case::nothing_pinned(&[], &[], None)]
fn main_pack_picks_like_the_interface(#[case] widgets: &[&str], #[case] layouts: &[&str], #[case] rank: Option<u128>) {
    assert_that!(main_pack(&pins(widgets, 1), &pins(layouts, 11))).is_equal_to(rank.map(Uuid::from_u128));
}
