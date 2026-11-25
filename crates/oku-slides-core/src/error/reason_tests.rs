use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{KnowledgeLimit, PathReason, Shape, ValueReason};

/// Each path reason has its key in every locale, and French differs from English.
#[test]
fn every_path_reason_key_exists_in_every_locale() {
    let reasons = [
        PathReason::Empty,
        PathReason::NotAbsolute,
        PathReason::BadSegment,
        PathReason::DotSegment,
        PathReason::Reserved,
        PathReason::TooDeep,
        PathReason::TooLong,
        PathReason::NotMarkdown,
    ];
    let en = Localizer::new(Some("en"));
    let fr = Localizer::new(Some("fr"));
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for reason in reasons {
            assert!(l.has(reason.key()), "{locale} : {} absent", reason.key());
        }
    }
    for reason in reasons {
        assert_ne!(en.tr(reason.key()), fr.tr(reason.key()), "{} not translated in fr", reason.key());
    }
}

/// Chaque langue a sa propre traduction de chaque raison.
#[test]
fn every_value_reason_key_exists_in_every_locale() {
    let reasons = [
        ValueReason::NoValue,
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
        ValueReason::Valign,
        ValueReason::Path,
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

/// The slugs the interface maps to `ui-knowledge-limit-<slug>` (`KnowledgeNotice.tsx`): pinned.
#[test]
fn knowledge_limit_slugs_are_the_interface_names() {
    let slugs = [KnowledgeLimit::TooManyFiles, KnowledgeLimit::FileTooLarge, KnowledgeLimit::TotalTooLarge]
        .map(KnowledgeLimit::slug);
    assert_that!(slugs).is_equal_to(["too-many-files", "file-too-large", "total-too-large"]);
}
