use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{OfferKind, OpenerOffer};
use crate::deck_offer::DeckOffer;

/// An offer of kind `kind` for pack `p` at `v2`.
fn offer(kind: OfferKind) -> OpenerOffer {
    let deck = DeckOffer { title: "Roadmap".into(), deck_uid: "d1".into() };
    let version = crate::test_support::builtin(crate::BASE_PACK).version.clone();
    OpenerOffer { tab: "t-1".into(), deck, kind, pack: Uuid::nil(), pack_name: "Acme".into(), version }
}

#[rstest]
#[case::version(OfferKind::Version, json!({ "kind": "version" }))]
#[case::copy(OfferKind::Copy { from: Uuid::nil() }, json!({ "kind": "copy", "from": Uuid::nil() }))]
#[case::theme(OfferKind::Theme, json!({ "kind": "theme" }))]
fn opener_offer_serializes_its_kind_flat(#[case] kind: OfferKind, #[case] expected: serde_json::Value) {
    let value = serde_json::to_value(offer(kind)).unwrap();
    let keys = ["kind", "from"];
    let shown: serde_json::Map<String, serde_json::Value> =
        keys.iter().filter_map(|k| value.get(*k).map(|v| ((*k).to_owned(), v.clone()))).collect();
    assert_that!(serde_json::Value::Object(shown)).is_equal_to(expected);
}
