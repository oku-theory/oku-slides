//! Le zip des sources n'accepte que des assets nommés par leur empreinte
//! (hex) : un nom avec guillemet ou chemin serait repris tel quel ailleurs.

use oku_slides_core::{Deck, SourceBundle, SourceLimits, SourcesErrorKind};
use speculoos::prelude::*;

fn bundle_with_asset(id: &str) -> SourceBundle {
    let deck = Deck::from_json(r#"{"slides":[]}"#).unwrap_or_else(|e| panic!("deck vide invalide : {e}"));
    let mut b = SourceBundle::new(deck).unwrap_or_else(|e| panic!("bundle : {e}"));
    b.assets.insert(id.to_owned(), vec![1, 2, 3]);
    b
}

#[test]
fn to_zip_asset_id_with_quote_returns_bad_name() {
    let err = bundle_with_asset(r#"sha256:ab",panic("x"),""#).to_zip().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn to_zip_asset_id_with_hex_digest_succeeds() {
    let b = bundle_with_asset("sha256:00ff");
    let (back, _) =
        SourceBundle::from_zip(&b.to_zip().unwrap(), SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 })
            .unwrap();
    assert_that!(back.assets.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec!["sha256:00ff"]);
}
