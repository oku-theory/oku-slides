//! t-1104 : un manifeste de pack illisible n'avait pas de texte localisé, et un jeton invalide
//! d'un pack épinglé était signalé comme un épinglage qui ne correspond pas.

use std::collections::BTreeMap;

use oku_slides_core::{
    Deck, Namespace, PackFiles, PackKind, PackManifest, PackManifestErrorKind, PackPin, SourceBundle, SourceLimits,
};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

const PACK_ID: &str = "6a4f1c2e-1d3b-4e5f-8a9b-0c1d2e3f4a5b";
const LIMITS: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

/// Un pack de widgets `p` dont `tokens` porte `tokens` (valide ou non).
fn pack_with_tokens(tokens: Value) -> PackFiles {
    let manifest = json!({ "id": PACK_ID, "name": "P", "namespace": "p", "requires": [],
        "created": "2026-10-19T00:00:00Z", "tokens": tokens });
    PackFiles::new(PackKind::Widgets, manifest.as_object().cloned().unwrap_or_default(), BTreeMap::new())
}

#[test]
fn issue_1104_unreadable_manifest_is_a_json_error_with_a_localized_text() {
    let manifest = json!({ "id": 3 });
    let files = PackFiles::new(PackKind::Widgets, manifest.as_object().cloned().unwrap_or_default(), BTreeMap::new());
    let err = PackManifest::from_files(&files).expect_err("manifeste illisible");
    assert_that!(err.kind()).is_equal_to(PackManifestErrorKind::Json);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(oku_slides_i18n::english()));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn issue_1104_pinned_pack_with_an_invalid_token_is_reported_as_tokens_invalid() {
    let files = pack_with_tokens(json!([{ "path": ["a"], "control": "color" }]));
    let pin = PackPin {
        id: PACK_ID.parse().unwrap_or_else(|e| panic!("{e}")),
        version: files.version(),
        requires: Vec::new(),
    };
    let mut deck = Deck::from_json(r#"{ "slides": [] }"#).unwrap_or_else(|e| panic!("deck : {e}"));
    deck.widgets.insert(Namespace::parse("p").unwrap_or_else(|| panic!("espace de noms")), pin);
    let mut bundle = SourceBundle::new(deck, common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"));
    bundle.add_pack(files);
    let zip = bundle.to_zip().unwrap_or_else(|e| panic!("zip : {e}"));
    let (_, issues) = SourceBundle::from_zip(&zip, LIMITS, common::shared()).unwrap_or_else(|e| panic!("relu : {e}"));
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).contains("issue-pack-tokens-invalid");
    assert_that!(ids).does_not_contain("issue-pack-pin-mismatch");
}
