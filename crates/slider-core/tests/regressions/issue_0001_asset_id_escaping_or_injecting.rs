//! Le zip des sources n'accepte que des assets nommés par leur empreinte
//! (hex) : un nom avec guillemet ou chemin serait repris tel quel ailleurs.

use slider_core::{Deck, SourceBundle, SourcesErrorKind};

fn bundle_with_asset(id: &str) -> SourceBundle {
    let deck = Deck::from_json(r#"{"slides":[]}"#).unwrap_or_else(|e| panic!("deck vide invalide : {e}"));
    let mut b = SourceBundle::new(deck);
    b.assets.insert(id.to_owned(), vec![1, 2, 3]);
    b
}

#[test]
fn to_zip_asset_id_with_quote_returns_bad_name() {
    let err = bundle_with_asset(r#"sha256:ab",panic("x"),""#).to_zip().unwrap_err();
    assert_eq!(err.kind(), SourcesErrorKind::BadName);
}

#[test]
fn to_zip_asset_id_with_hex_digest_succeeds() {
    let b = bundle_with_asset("sha256:00ff");
    let back = SourceBundle::from_zip(&b.to_zip().unwrap()).unwrap();
    assert_eq!(back.assets.keys().collect::<Vec<_>>(), ["sha256:00ff"]);
}
