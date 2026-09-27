use std::io::Write;

use speculoos::prelude::*;

use super::*;
use crate::error::{ReplicaErrorKind, SourcesErrorKind};

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 };

/// Un zip fait à la main, entrée par entrée.
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        for (name, data) in entries {
            zw.start_file(*name, SimpleFileOptions::default()).unwrap();
            zw.write_all(data).unwrap();
        }
        zw.finish().unwrap();
    }
    buf.into_inner()
}

const MANIFEST: &[u8] = br#"{"format":"oku-slides-sources","version":2}"#;
/// L'ancien `deck.json` (format v1) : ignoré en v2.
const DECK_JSON: &[u8] = br#"{"slides":[]}"#;

/// Un `deck.loro` valide : le snapshot d'un deck vide.
fn deck_loro() -> Vec<u8> {
    let empty = Deck::from_json(std::str::from_utf8(DECK_JSON).unwrap()).unwrap();
    DeckReplica::new(&empty).unwrap().snapshot().unwrap()
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn from_zip_not_a_zip_returns_unreadable() {
    let err = SourceBundle::from_zip(b"pas un zip", MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Unreadable);
}

#[test]
fn from_zip_without_manifest_returns_not_sources() {
    let err = SourceBundle::from_zip(&zip(&[("deck.loro", &deck_loro())]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_other_format_returns_not_sources() {
    let manifest = br#"{"format":"slider-sources","version":2}"#;
    let err =
        SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_newer_version_returns_unsupported_version() {
    let manifest = br#"{"format":"oku-slides-sources","version":99}"#;
    let err =
        SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::UnsupportedVersion);
}

#[test]
fn from_zip_version_1_returns_not_sources() {
    let manifest = br#"{"format":"oku-slides-sources","version":1}"#;
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.json", DECK_JSON)]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_without_deck_loro_returns_missing_deck() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn from_zip_deck_json_only_returns_missing_deck() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST), ("deck.json", DECK_JSON)]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn from_zip_invalid_deck_loro_returns_replica_corrupted() {
    let err =
        SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST), ("deck.loro", b"pas loro")]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_zip_deck_loro_over_max_returns_replica_too_large() {
    let loro = deck_loro();
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]),
        SourceLimits { replica_bytes: loro.len() - 1, ..MAX },
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(ReplicaErrorKind::TooLarge));
}

#[test]
fn from_zip_hidden_font_name_returns_bad_name() {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("fonts/.cache", b"x")];
    let err = SourceBundle::from_zip(&zip(entries), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_unknown_entries_are_ignored() {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("history.jsonl", b"{}")];
    assert_that!(SourceBundle::from_zip(&zip(entries), MAX)).is_ok();
}

#[test]
fn to_zip_then_from_zip_returns_same_bundle() {
    let mut b = SourceBundle::new(roadmap()).unwrap();
    b.add_asset(b"png".to_vec());
    let (back, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&b);
    assert_that!(back.replica().deck_uid()).is_equal_to(b.replica().deck_uid());
}

#[test]
fn to_zip_has_no_deck_json() {
    let zip = SourceBundle::new(roadmap()).unwrap().to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n == "deck.json")).is_false();
    assert_that!(za.file_names().any(|n| n == "deck.loro")).is_true();
}

#[test]
fn apply_then_undo_restores_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let mut next = deck.clone();
    next.theme = Some("pro".into());
    b.apply(next, "Thème").unwrap();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(b.undo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&deck);
}

#[test]
fn apply_silent_then_undo_returns_false() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let mut next = deck;
    next.theme = Some("pro".into());
    b.apply_silent(next).unwrap();
    assert_that!(b.undo().unwrap()).is_false();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("pro"));
}

#[test]
fn undo_then_redo_restores_applied_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let mut next = deck;
    next.theme = Some("pro".into());
    b.apply(next.clone(), "Thème").unwrap();
    b.undo().unwrap();
    assert_that!(b.redo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&next);
}

#[test]
fn from_zip_assets_over_total_limit_returns_too_large() {
    let loro = deck_loro();
    let big = vec![1u8; 4096];
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("assets/aa", &big), ("assets/bb", &big)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + big.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to("Les sources de ce fichier sont trop volumineuses pour être ouvertes".to_owned());
}

#[test]
fn from_zip_assets_within_total_limit_are_read() {
    let loro = deck_loro();
    let big = vec![1u8; 4096];
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("assets/aa", &big)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + big.len(), ..MAX };
    let (b, _) = SourceBundle::from_zip(&zip(entries), limits).unwrap();
    assert_that!(b.assets.get("sha256:aa")).is_equal_to(Some(&big));
}

/// Bombe de décompression : quelques Kio compressés, 64 Mio une fois lus.
#[test]
fn from_zip_deflate_bomb_returns_too_large() {
    let zeros = vec![0u8; 64 << 20];
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("fonts/f.ttf", &zeros)];
    let bomb = zip(entries);
    assert_that!(bomb.len()).is_less_than(1 << 20);
    let err = SourceBundle::from_zip(&bomb, SourceLimits { total_bytes: 1 << 20, ..MAX }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn sources_error_localizes_by_kind() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)]), MAX).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to("Le deck manque dans les sources".to_owned());
}

#[test]
fn sources_error_replica_localizes_as_replica_error() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST), ("deck.loro", b"x")]), MAX).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected =
        oku_slides_i18n::Localize::localize(&DeckReplica::from_snapshot(b"x", MAX.replica_bytes).unwrap_err(), &fr);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr)).is_equal_to(expected);
}
