use oku_slides_core::{Op, PackVersion};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::test_support::{fresh_dir, rename_title, set_theme_op};
use crate::test_support_packs::{opener_on, pack_tab_from};
use crate::theme_edit::theme_edit_tests::p;
use crate::theme_tab::theme_tab_tests::{theme_tab, with_maison};
use crate::{Session, SessionErrorKind};

/// An opener pinning `acme`, and the copy `c` at `v` a pack tab saved of it.
struct Copied {
    s: Session,
    _data: tempfile::TempDir,
    _deck: tempfile::TempDir,
    acme: Uuid,
    copy: Uuid,
    v: PackVersion,
}

/// The copy fixture: the tab renamed `acme` and saved it as a new pack.
fn copied() -> Copied {
    let data = fresh_dir("oku-slides-offer-copy-");
    let (mut s, deck, acme) = opener_on(data.path());
    let mut tab = pack_tab_from(&mut s, data.path(), acme);
    tab.apply("x", vec![rename_title("ma charte")], None).unwrap();
    let v = tab.save_pack(true).unwrap();
    let copy = tab.wrote_in_pack_tab().unwrap().pack;
    drop(tab);
    Copied { s, _data: data, _deck: deck, acme, copy, v }
}

/// The deck's identity in `s`.
fn uid(s: &Session) -> String {
    s.doc.sources.replica().deck_uid()
}

/// The namespace `s`'s deck pins `pack` under, if it does.
fn ns_of(s: &Session, pack: Uuid) -> Option<String> {
    s.user_deck().all_pins().find(|(_, _, pin)| pin.id == pack).map(|(_, ns, _)| ns.to_string())
}

#[test]
fn wrote_in_pack_tab_after_a_save_as_new_names_the_original() {
    let data = fresh_dir("oku-slides-offer-copy-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut tab = pack_tab_from(&mut s, data.path(), acme);
    tab.save_pack(true).unwrap();
    tab.save_pack(true).unwrap();
    assert_that!(tab.wrote_in_pack_tab().and_then(|w| w.copied_from)).is_equal_to(Some(acme));
}

#[test]
fn opener_copy_offer_names_a_deck_pinning_the_original() {
    let c = copied();
    let offer = c.s.opener_copy_offer(c.acme, c.copy, &c.v);
    assert_that!(offer.map(|o| o.deck_uid)).is_equal_to(Some(uid(&c.s)));
}

#[test]
fn opener_copy_offer_is_none_once_the_deck_no_longer_pins_the_original() {
    let mut c = copied();
    c.s.new_deck().unwrap();
    assert_that!(c.s.opener_copy_offer(c.acme, c.copy, &c.v).is_none()).is_true();
}

#[test]
fn use_pack_copy_replaces_the_pin_and_renames_the_slides() {
    let mut c = copied();
    let old = ns_of(&c.s, c.acme).unwrap();
    let card: Op = serde_json::from_value(json!({
        "op": "add_element", "slot": { "owner": { "slide": "s-1" }, "param": "blocks" }, "defaults": true,
        "element": { "id": "k-1", "kind": format!("{old}/card"), "params": {} },
    }))
    .unwrap();
    c.s.apply("card", vec![card], None).unwrap();
    let deck_uid = uid(&c.s);
    c.s.use_pack_copy(c.acme, c.copy, &c.v, &deck_uid).unwrap();
    let new = ns_of(&c.s, c.copy).unwrap_or_else(|| panic!("the copy is pinned"));
    let slides = serde_json::to_string(&c.s.user_deck().slides).unwrap();
    let state = (ns_of(&c.s, c.acme), slides.contains(&format!("\"{old}/")), slides.contains(&format!("\"{new}/")));
    assert_that!(state).is_equal_to((None, false, true));
}

#[test]
fn use_pack_copy_undoes_in_one_step() {
    let mut c = copied();
    let before = c.s.user_deck().clone();
    let deck_uid = uid(&c.s);
    c.s.use_pack_copy(c.acme, c.copy, &c.v, &deck_uid).unwrap();
    c.s.undo().unwrap();
    assert_that!(c.s.user_deck().clone()).is_equal_to(before);
}

#[test]
fn use_pack_copy_in_another_deck_is_expired() {
    let mut c = copied();
    let err = c.s.use_pack_copy(c.acme, c.copy, &c.v, "autre").unwrap_err();
    assert_that!((err.kind(), ns_of(&c.s, c.copy))).is_equal_to((SessionErrorKind::PackOfferExpired, None));
}

/// An opener whose deck uses theme `maison` at its first version, and a theme tab that saved a second.
fn themed() -> (Session, Uuid, PackVersion, tempfile::TempDir) {
    let data = fresh_dir("oku-slides-offer-theme-");
    let (mut s, id) = with_maison(data.path());
    s.new_deck().unwrap();
    s.apply("thème", vec![set_theme_op("maison")], None).unwrap();
    let mut tab = theme_tab(data.path(), id);
    tab.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("#123456") }], None).unwrap();
    tab.save_theme(false).unwrap();
    let wrote = tab.wrote_in_theme_tab().unwrap_or_else(|| panic!("the tab wrote a version"));
    assert_that!(wrote.theme).is_equal_to(id);
    (s, id, wrote.version, data)
}

#[test]
fn opener_theme_offer_names_a_deck_on_an_older_version() {
    let (s, id, v, _data) = themed();
    assert_that!(s.opener_theme_offer(id, &v).map(|o| o.deck_uid)).is_equal_to(Some(uid(&s)));
}

#[test]
fn use_theme_version_pins_the_written_version() {
    let (mut s, id, v, _data) = themed();
    let deck_uid = uid(&s);
    s.use_theme_version(id, &v, &deck_uid).unwrap();
    assert_that!(s.deck_theme_pin().map(|pin| pin.version.clone())).is_equal_to(Some(v));
}

#[test]
fn use_theme_version_no_longer_current_is_expired() {
    let (mut s, id, _v, _data) = themed();
    let deck_uid = uid(&s);
    let stale = s.deck_theme_pin().map(|pin| pin.version.clone()).unwrap();
    let err = s.use_theme_version(id, &stale, &deck_uid).unwrap_err();
    let pinned = s.deck_theme_pin().map(|pin| pin.version.clone());
    assert_that!((err.kind(), pinned)).is_equal_to((SessionErrorKind::PackOfferExpired, Some(stale)));
}

/// The copy's namespace, read from its manifest in the library.
fn copy_ns(c: &Copied) -> oku_slides_core::Namespace {
    let files = c.s.library.version(c.copy, &c.v).unwrap();
    crate::pack::PackManifest::from_files(&files).unwrap().namespace
}

/// Writes `edit` of the user deck straight into the sources, as another client could.
fn edit_deck(s: &mut Session, edit: impl FnOnce(&mut oku_slides_core::Deck)) {
    let mut deck = s.user_deck().clone();
    edit(&mut deck);
    s.doc.sources.apply(deck, "test", None).unwrap();
}

#[test]
fn opener_copy_offer_is_none_when_the_copy_namespace_is_taken() {
    let mut c = copied();
    let to_ns = copy_ns(&c);
    edit_deck(&mut c.s, |deck| {
        let base = deck.widgets.values().next().cloned().unwrap();
        deck.layouts.insert(to_ns, base);
    });
    assert_that!(c.s.opener_copy_offer(c.acme, c.copy, &c.v).is_none()).is_true();
}

#[test]
fn opener_copy_offer_is_none_when_another_pin_requires_the_original() {
    let mut c = copied();
    let from_ns = oku_slides_core::Namespace::parse(&ns_of(&c.s, c.acme).unwrap()).unwrap();
    edit_deck(&mut c.s, |deck| {
        let other = deck.widgets.values_mut().find(|pin| pin.id != c.acme).unwrap();
        other.requires.push(from_ns);
    });
    assert_that!(c.s.opener_copy_offer(c.acme, c.copy, &c.v).is_none()).is_true();
}

#[test]
fn opener_copy_offer_is_none_for_a_version_no_longer_current() {
    let c = copied();
    let old = c.s.user_deck().all_pins().find(|(_, _, pin)| pin.id == c.acme).map(|(_, _, pin)| pin.version.clone());
    assert_that!(c.s.opener_copy_offer(c.acme, c.copy, &old.unwrap()).is_none()).is_true();
}
