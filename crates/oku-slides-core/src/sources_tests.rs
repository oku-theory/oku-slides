use std::io::Write;

use speculoos::prelude::*;

use super::*;
use crate::error::SourcesErrorKind;

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

const MANIFEST: &[u8] = br#"{"format":"oku-slides-sources","version":1}"#;
const DECK: &[u8] = br#"{"slides":[]}"#;

#[test]
fn from_zip_not_a_zip_returns_unreadable() {
    let err = SourceBundle::from_zip(b"pas un zip").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Unreadable);
}

#[test]
fn from_zip_without_manifest_returns_not_sources() {
    let err = SourceBundle::from_zip(&zip(&[("deck.json", DECK)])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_other_format_returns_not_sources() {
    let manifest = br#"{"format":"slider-sources","version":1}"#;
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.json", DECK)])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_newer_version_returns_unsupported_version() {
    let manifest = br#"{"format":"oku-slides-sources","version":99}"#;
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.json", DECK)])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::UnsupportedVersion);
}

#[test]
fn from_zip_without_deck_returns_missing_deck() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn from_zip_invalid_deck_json_returns_unreadable() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST), ("deck.json", b"{")])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Unreadable);
}

#[test]
fn from_zip_hidden_font_name_returns_bad_name() {
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.json", DECK), ("fonts/.cache", b"x")];
    let err = SourceBundle::from_zip(&zip(entries)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_unknown_entries_are_ignored() {
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.json", DECK), ("history.jsonl", b"{}")];
    assert_that!(SourceBundle::from_zip(&zip(entries))).is_ok();
}

#[test]
fn sources_error_localizes_by_kind() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)])).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to("Le deck manque dans les sources".to_owned());
}
