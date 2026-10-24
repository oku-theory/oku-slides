use oku_slides_core::{Element, Op};
use serde_json::json;
use speculoos::prelude::*;

use crate::test_support::{png, session};
use crate::{Session, SessionErrorKind};

/// L'op qui donne `asset` à l'image `e-image` de `s-libre` (roadmap.json).
fn set_image(asset: &str) -> Op {
    serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset", "value": asset
    }))
    .unwrap()
}

/// L'image `e-image` de `s-libre`, telle que le deck de `s` la tient.
fn image_element(s: &Session) -> Element {
    let slide = s.doc.sources.deck().slides.iter().find(|x| x.id == "s-libre").unwrap();
    slide.locate("e-image").unwrap().element.clone()
}

/// Une session dont l'image `e-image` cite un asset du bundle ; ses octets.
fn session_with_image(seed: u8) -> (Session, Vec<u8>) {
    let mut s = session();
    let bytes = png(4, 4, seed);
    let r = s.paste_image(bytes.clone(), &[]).unwrap();
    s.apply("image", vec![set_image(&r.id)], None).unwrap();
    (s, bytes)
}

#[test]
fn element_assets_returns_the_bytes_of_the_cited_assets() {
    let (s, bytes) = session_with_image(20);
    assert_that!(s.element_assets(&image_element(&s))).is_equal_to(vec![bytes]);
}

#[test]
fn element_assets_skips_an_asset_missing_from_the_bundle() {
    // roadmap.json cite `sha256:0000` sans ses octets.
    let s = session();
    assert_that!(s.element_assets(&image_element(&s))).is_equal_to(Vec::<Vec<u8>>::new());
}

#[test]
fn apply_citing_a_carried_asset_adds_it_to_the_bundle() {
    let mut s = session();
    let bytes = png(4, 4, 21);
    let id = oku_slides_core::asset_id(&bytes);
    s.carry_assets(vec![bytes.clone()]);
    s.apply("image", vec![set_image(&id)], None).unwrap();
    assert_that!(s.doc.sources.assets.get(&id)).is_equal_to(Some(&bytes));
}

#[test]
fn apply_citing_a_carried_asset_forgets_its_carried_copy() {
    let mut s = session();
    let bytes = png(4, 4, 27);
    let id = oku_slides_core::asset_id(&bytes);
    s.carry_assets(vec![bytes]);
    s.apply("image", vec![set_image(&id)], None).unwrap();
    assert_that!(s.carried.contains_key(&id)).is_false();
}

#[test]
fn apply_citing_a_carried_and_an_unknown_asset_adds_neither() {
    let mut s = session();
    let bytes = png(4, 4, 28);
    let id = oku_slides_core::asset_id(&bytes);
    s.carry_assets(vec![bytes]);
    // Une seconde image, qui cite un asset inconnu : le deck cite alors les deux.
    let other: Op = serde_json::from_value(json!({
        "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "e-inconnue", "kind": "base/image", "params": { "asset": "sha256:1234" } }
    }))
    .unwrap();
    let e = s.apply("image", vec![set_image(&id), other], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
    assert_that!((s.doc.sources.assets.contains_key(&id), s.carried.contains_key(&id))).is_equal_to((false, true));
}

#[test]
fn carry_assets_keys_bytes_by_their_own_id_not_the_bundle_key() {
    // Un bundle lu d'un zip range un asset sous le nom de son fichier, sans vérifier son empreinte.
    let mut a = session();
    let bytes = png(4, 4, 29);
    let forged = format!("sha256:{}", "0".repeat(64));
    a.doc.sources.assets.insert(forged.clone(), bytes.clone());
    let mut element = image_element(&a);
    element.params = serde_json::from_value(json!({ "asset": forged })).unwrap();
    let mut b = session();
    b.carry_assets(a.element_assets(&element));
    let e = b.apply("image", vec![set_image(&forged)], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
    assert_that!(b.doc.sources.assets.contains_key(&forged)).is_false();
}

#[test]
fn carry_assets_alone_adds_nothing_to_the_bundle() {
    let mut s = session();
    let bytes = png(4, 4, 22);
    let id = oku_slides_core::asset_id(&bytes);
    s.carry_assets(vec![bytes]);
    assert_that!(s.doc.sources.assets.contains_key(&id)).is_false();
}

#[test]
fn carry_assets_forgets_the_previous_ones() {
    let mut s = session();
    let old = png(4, 4, 23);
    let id = oku_slides_core::asset_id(&old);
    s.carry_assets(vec![old]);
    s.carry_assets(vec![png(4, 4, 24)]);
    let e = s.apply("image", vec![set_image(&id)], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
}

#[test]
fn carry_assets_returns_how_many_it_keeps() {
    let mut s = session();
    assert_that!(s.carry_assets(vec![png(2, 2, 25), png(2, 2, 26)])).is_equal_to(2);
}

#[test]
fn carry_assets_skips_an_asset_the_bundle_already_holds() {
    let (mut s, bytes) = session_with_image(30);
    assert_that!(s.carry_assets(vec![bytes])).is_equal_to(0);
}
