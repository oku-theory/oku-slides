use oku_slides_i18n::Localizer;

use super::{Shape, ValueReason};

/// Chaque langue a sa propre traduction de chaque raison.
#[test]
fn every_value_reason_key_exists_in_every_locale() {
    let reasons = [
        ValueReason::NoValue,
        ValueReason::NoDataset,
        ValueReason::GridTracks,
        ValueReason::BadItemId,
        ValueReason::MissingItemId,
        ValueReason::TooDeep,
        ValueReason::EmptyPath,
        ValueReason::NotText,
        ValueReason::EmptyFrame,
        ValueReason::BlocksOffGrid,
        ValueReason::OutsideGrid,
        ValueReason::NotTokens,
        ValueReason::NotASlot,
        ValueReason::OutOfBounds,
        ValueReason::Unreadable,
    ];
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        assert!(l.has("op-invalid-value-because"), "{locale} : op-invalid-value-because absent");
        for reason in reasons {
            assert!(l.has(reason.key()), "{locale} : {} absent", reason.key());
        }
    }
}

/// `Shape::ALL` couvre chaque forme, une fois, dans l'ordre de `position`.
#[test]
fn all_shapes_lists_each_shape_at_its_position() {
    for (i, shape) in Shape::ALL.into_iter().enumerate() {
        assert_eq!(shape.position(), i, "{shape:?} n'est pas à son rang dans Shape::ALL");
    }
}

/// Les clés `op-param-shape-*` du `core.ftl` anglais (la référence des langues).
fn english_shape_keys() -> Vec<&'static str> {
    include_str!("../../../oku-slides-i18n/src/locales/en/core.ftl")
        .lines()
        .filter_map(|line| line.split_once(" =").map(|(id, _)| id.trim()))
        .filter(|id| id.starts_with("op-param-shape-"))
        .collect()
}

/// Une forme oubliée de `Shape::ALL` ne passe pas inaperçue : le `core.ftl` anglais dit chaque
/// forme, donc ses clés `op-param-shape-*` sont exactement celles de `ALL`, ni plus ni moins, et
/// les rangs de `position` sont distincts (`ALL` et `position` ne se contredisent pas).
#[test]
fn all_shapes_matches_the_shape_keys_of_the_english_catalog() {
    let catalog = english_shape_keys();
    let listed: Vec<&str> = Shape::ALL.iter().map(|shape| shape.key()).collect();
    assert_eq!(catalog.len(), Shape::ALL.len(), "core.ftl dit {catalog:?}, Shape::ALL liste {listed:?}");
    for key in &catalog {
        assert!(listed.contains(key), "{key} est dans core.ftl mais aucune forme de Shape::ALL n'a cette clé");
    }
    for key in &listed {
        assert!(catalog.contains(key), "{key} (Shape::ALL) est absente de core.ftl");
    }
    let mut positions: Vec<usize> = Shape::ALL.iter().map(|shape| shape.position()).collect();
    positions.sort_unstable();
    positions.dedup();
    assert_eq!(positions.len(), Shape::ALL.len(), "deux formes de Shape::ALL partagent un rang");
}
