use std::collections::BTreeMap;

use oku_slides_core::{Namespace, PackPin, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::main_pack;

/// Des épingles `espace de noms → pack`, l'id du pack tiré de son rang.
fn pins(namespaces: &[&str]) -> BTreeMap<Namespace, PackPin> {
    namespaces
        .iter()
        .enumerate()
        .map(|(i, ns)| {
            let ns = Namespace::parse(ns).unwrap_or_else(|| panic!("espace de noms : {ns}"));
            let version = PackVersion::from_hex(&"0".repeat(64)).unwrap_or_else(|| panic!("version de test"));
            (ns, PackPin { id: Uuid::from_u128(i as u128 + 1), version, requires: Vec::new() })
        })
        .collect()
}

/// Comme `firstPackId` : bibliothèque, puis pack propre (`deck`), puis `base`.
#[rstest]
#[case::library_beats_deck_and_base(&["base", "deck", "acme"], Some(3))]
#[case::deck_beats_base(&["base", "deck"], Some(2))]
#[case::base_alone(&["base"], Some(1))]
#[case::nothing_pinned(&[], None)]
fn main_pack_picks_like_the_interface(#[case] pinned: &[&str], #[case] rank: Option<u128>) {
    assert_that!(main_pack(&pins(pinned))).is_equal_to(rank.map(Uuid::from_u128));
}
