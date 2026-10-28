use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::test_support::{fresh_dir, opened_on};
use crate::test_support_packs::{LINKED_TAB, embedded, link_of, linked_owner};
use crate::{BASE_PACK, SessionErrorKind};

/// The layout's link of `tab` on `pack`.
fn spec(tab: &str, pack: Uuid) -> LinkSpec {
    LinkSpec { tab: tab.to_owned(), title: "Importé".to_owned(), pack }
}

#[test]
fn link_deck_pack_of_a_library_pack_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let mut owner = opened_on(data.path());
    let refused = owner.link_deck_pack(LINKED_TAB, "Base", BASE_PACK).map(|_| ()).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::UnknownPack);
    assert_that!(owner.deck_links.is_empty()).is_true();
}

#[test]
fn link_deck_pack_twice_on_one_pack_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    let refused = owner.link_deck_pack("t-3", "Importé", pin.id).map(|_| ()).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(refused.lease_holder()).is_equal_to(Some("Importé"));
    assert_that!(owner.deck_links.contains_key("t-3")).is_false();
}

#[test]
fn set_deck_links_keeps_a_pending_link_until_named() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    assert_that!(owner.set_deck_links(1, &[])).is_empty();
    assert_that!(link_of(&owner, LINKED_TAB).pending).is_true();
    assert_that!(owner.set_deck_links(2, &[spec(LINKED_TAB, pin.id)])).is_empty();
    assert_that!(link_of(&owner, LINKED_TAB).pending).is_false();
}

#[test]
fn set_deck_links_keeps_a_known_link_whatever_pack_the_layout_names() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    let before = link_of(&owner, LINKED_TAB).clone();
    assert_that!(owner.set_deck_links(1, &[spec(LINKED_TAB, Uuid::new_v4())])).is_empty();
    assert_that!(link_of(&owner, LINKED_TAB).pack).is_equal_to(pin.id);
    assert_that!(link_of(&owner, LINKED_TAB).expect).is_equal_to(before.expect);
}

#[test]
fn set_deck_links_of_a_renamed_tab_takes_its_new_title() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    let renamed = LinkSpec { title: "Alto (layouts)".to_owned(), ..spec(LINKED_TAB, pin.id) };
    assert_that!(owner.set_deck_links(1, &[renamed])).is_empty();
    assert_that!(link_of(&owner, LINKED_TAB).title.as_str()).is_equal_to("Alto (layouts)");
}

#[test]
fn set_deck_links_drops_a_confirmed_link_no_longer_named() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    assert_that!(owner.set_deck_links(1, &[spec(LINKED_TAB, pin.id)])).is_empty();
    assert_that!(owner.set_deck_links(2, &[])).is_empty();
    assert_that!(owner.deck_links.contains_key(LINKED_TAB)).is_false();
}

#[test]
fn set_deck_links_reseeds_a_new_link_from_the_pin_and_its_parent() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let mut owner = opened_on(data.path());
    let pin = crate::test_support::adopt_deck_only_pack(&mut owner);
    let pin_parent = PackManifest::from_files(&embedded(&owner, &pin.version)).unwrap().parent;
    assert_that!(pin_parent).is_some();
    assert_that!(owner.set_deck_links(1, &[spec("t-3", pin.id)])).is_empty();
    let expected = DeckLink {
        title: "Importé".to_owned(),
        pack: pin.id,
        deck_uid: owner.user_deck_uid(),
        parent: LinkParent::Reseeded { pin: pin.version.clone(), pin_parent },
        expect: pin.version.clone(),
        last_seq: None,
        pending: false,
    };
    assert_that!(link_of(&owner, "t-3")).is_equal_to(&expected);
}

#[test]
fn set_deck_links_returns_a_new_link_on_a_library_pin() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let mut owner = opened_on(data.path());
    assert_that!(owner.set_deck_links(1, &[spec("t-3", BASE_PACK)])).is_equal_to(vec!["t-3".to_owned()]);
    assert_that!(owner.deck_links.contains_key("t-3")).is_false();
}

#[test]
fn set_deck_links_ignores_an_older_generation() {
    let data = fresh_dir("oku-slides-deck-pack-link-");
    let (mut owner, pin) = linked_owner(data.path());
    assert_that!(owner.set_deck_links(2, &[spec(LINKED_TAB, pin.id)])).is_empty();
    owner.unlink_deck_pack(LINKED_TAB);
    // A stale layout copy, taken before the tab closed, names it again.
    assert_that!(owner.set_deck_links(1, &[spec(LINKED_TAB, pin.id)])).is_empty();
    assert_that!(owner.deck_links.contains_key(LINKED_TAB)).is_false();
}
