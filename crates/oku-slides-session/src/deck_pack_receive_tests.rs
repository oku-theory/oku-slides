use serde_json::json;
use speculoos::prelude::*;

use oku_slides_core::{Namespace, Op};

use super::*;
use crate::deck_pack_link::LinkSpec;
use crate::deck_pack_tab::SendBase;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, opened_on, pin};
use crate::test_support_packs::{
    CARD_SRC, CARD_V2, LINKED_TAB, applied, apply_valid, copy_send, deck_pack_tab_of, embedded, files_with,
    forward_to_owner, link_of, linked_owner, ok, valid_send,
};
use crate::{Request, SessionErrorKind};

/// A `card` source whose body fails at render.
const CARD_FAILING: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => panic("corps fautif"))
"#;

/// Saves the tab's pack through the UI's request.
fn save(t: &mut Session) {
    ok(t.handle(Request::SavePack { as_new: false }), "save pack");
}

/// Writes `card` into the tab's pack, saves, and forwards to `owner`, which must apply it.
fn send_card(t: &mut Session, owner: &mut Session, source: &str) {
    ok(t.set_widget(None, "card", source.into()), "card");
    save(t);
    assert_that!(forward_to_owner(t, LINKED_TAB, owner).ok()).is_equal_to(Some(Received::Applied));
}

/// `valid_send` with `card` written as `source`.
fn card_send(owner: &Session, source: &str) -> crate::deck_pack_tab::DeckPackSend {
    let send = valid_send(owner, LINKED_TAB);
    let manifest = PackManifest::from_files(&send.files).unwrap();
    let mut widgets = send.files.widgets().clone();
    widgets.insert(oku_slides_core::WidgetFile::parse("card").unwrap(), source.to_owned());
    crate::deck_pack_tab::DeckPackSend { files: files_with(&manifest, send.files.kind(), &widgets), ..send }
}

/// An owner whose deck shows a `deck/card` element, its deck-only pack linked to `t-2`.
fn owner_showing_card(data: &std::path::Path) -> Session {
    let mut owner = opened_on(data);
    adopt_deck_only_pack(&mut owner);
    ok(owner.set_widget(Some(&Namespace::deck()), "card", CARD_SRC.into()), "card");
    owner.end_widget_session();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "c", "kind": "deck/card", "params": { "body": [{ "runs": [{ "t": "Carte" }] }] } }
    }))
    .unwrap();
    ok(owner.apply("carte", vec![add], None), "card element");
    let pack = pin(&owner, "deck").unwrap().id;
    drop(ok(owner.link_deck_pack(LINKED_TAB, "Importé", pack), "link"));
    owner
}

#[test]
fn deck_pack_save_reaches_owner_as_one_undo_step() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let mut owner = opened_on(data.path());
    let opened = adopt_deck_only_pack(&mut owner);
    let mut t = deck_pack_tab_of(&mut owner, data.path(), opened.id);
    owner.doc.dirty = false;
    let steps = owner.doc.sources.undo_steps();
    send_card(&mut t, &mut owner, CARD_SRC);
    send_card(&mut t, &mut owner, CARD_V2);
    assert_that!(owner.doc.sources.undo_steps()).is_equal_to(steps + 1);
    let pinned = pin(&owner, "deck").unwrap();
    let card = embedded(&owner, &pinned.version)
        .widget(&oku_slides_core::WidgetFile::parse("card").unwrap())
        .map(str::to_owned);
    assert_that!(card).is_equal_to(Some(CARD_V2.to_owned()));
    assert_that!(owner.doc.dirty).is_true();
}

#[test]
fn receive_deck_pack_drops_the_replaced_version_when_merged() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let (mut owner, _) = linked_owner(data.path());
    let v2 = valid_send(&owner, LINKED_TAB);
    applied(&mut owner, LINKED_TAB, &v2);
    apply_valid(&mut owner, LINKED_TAB);
    assert_that!(owner.doc.sources.packs.contains_key(&v2.files.version())).is_false();
}

#[test]
fn receive_deck_pack_keeps_the_opened_version_after_a_send_back_to_it() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let (mut owner, opened) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    let back = crate::deck_pack_tab::DeckPackSend {
        files: embedded(&owner, &opened.version),
        ..valid_send(&owner, LINKED_TAB)
    };
    applied(&mut owner, LINKED_TAB, &back);
    let v3 = valid_send(&owner, LINKED_TAB);
    assert_that!(v3.files.version()).is_not_equal_to(opened.version.clone());
    applied(&mut owner, LINKED_TAB, &v3);
    assert_that!(owner.doc.sources.packs.contains_key(&opened.version)).is_true();
}

#[test]
fn receive_deck_pack_installs_the_new_widgets() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let mut owner = owner_showing_card(data.path());
    let note = CARD_SRC.replace("\"card\"", "\"note\"");
    let send = valid_send(&owner, LINKED_TAB);
    let manifest = PackManifest::from_files(&send.files).unwrap();
    let mut widgets = send.files.widgets().clone();
    widgets.insert(oku_slides_core::WidgetFile::parse("note").unwrap(), note);
    let send = crate::deck_pack_tab::DeckPackSend { files: files_with(&manifest, send.files.kind(), &widgets), ..send };
    applied(&mut owner, LINKED_TAB, &send);
    assert_that!(owner.registry.get("deck/note").is_some()).is_true();
    assert_that!(owner.snapshot().notice).is_none();
}

#[test]
fn receive_deck_pack_of_a_copy_replaces_the_original_pins_in_both_kinds() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let (mut owner, original) = linked_owner(data.path());
    let copy = copy_send(&owner, LINKED_TAB);
    let copy_id = PackManifest::from_files(&copy.files).unwrap().id;
    applied(&mut owner, LINKED_TAB, &copy);
    assert_that!(owner.deck().all_pins().any(|(_, _, p)| p.id == original.id)).is_false();
    assert_that!(pin(&owner, "deck").map(|p| p.id)).is_equal_to(Some(copy_id));
    assert_that!(link_of(&owner, LINKED_TAB).pack).is_equal_to(copy_id);
    let later = valid_send(&owner, LINKED_TAB);
    assert_that!(later.base.clone()).is_equal_to(SendBase { pack: copy_id, parent: None });
    applied(&mut owner, LINKED_TAB, &later);
}

#[test]
fn receive_deck_pack_of_a_copy_changes_no_other_pin() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let (mut owner, _) = linked_owner(data.path());
    let others = |s: &Session| {
        let mut pins = combined_pins(s.deck());
        pins.remove(&Namespace::deck());
        pins
    };
    let before = others(&owner);
    let copy = copy_send(&owner, LINKED_TAB);
    applied(&mut owner, LINKED_TAB, &copy);
    assert_that!(others(&owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_failed_render_keeps_the_change_and_notices() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let mut owner = owner_showing_card(data.path());
    owner.doc.notice = None;
    let send = card_send(&owner, CARD_FAILING);
    applied(&mut owner, LINKED_TAB, &send);
    assert_that!(pin(&owner, "deck").map(|p| p.version)).is_equal_to(Some(send.files.version()));
    assert_that!(owner.snapshot().notice).is_some();
}

#[test]
fn owner_history_restore_pulls_parked_sends() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let mut owner = opened_on(data.path());
    let opened = adopt_deck_only_pack(&mut owner);
    let mut t = deck_pack_tab_of(&mut owner, data.path(), opened.id);
    ok(t.set_widget(None, "card", CARD_SRC.into()), "card");
    save(&mut t);
    ok(owner.history_open(), "history");
    assert_that!(forward_to_owner(&mut t, LINKED_TAB, &mut owner).ok()).is_equal_to(Some(Received::Busy));
    assert_that!(t.take_deck_pack_send()).is_none();
    ok(owner.history_restore(), "restore the latest version");
    assert_that!(owner.take_link_pull()).is_true();
    t.unpark_deck_pack();
    assert_that!(forward_to_owner(&mut t, LINKED_TAB, &mut owner).ok()).is_equal_to(Some(Received::Applied));
    assert_that!(t.unsent()).is_false();
}

#[test]
fn reseeded_link_accepts_the_tab_parent_then_fixes_it() {
    let data = fresh_dir("oku-slides-deck-pack-receive-");
    let mut owner = opened_on(data.path());
    let opened = adopt_deck_only_pack(&mut owner);
    let spec = LinkSpec { tab: LINKED_TAB.to_owned(), title: "Importé".to_owned(), pack: opened.id };
    assert_that!(owner.set_deck_links(1, &[spec])).is_empty();
    let pin_parent = PackManifest::from_files(&embedded(&owner, &opened.version)).unwrap().parent;
    // The tab's editing session started on the pin's parent before the resume.
    let resumed = |owner: &Session, parent: Option<oku_slides_core::PackVersion>| {
        let send = valid_send(owner, LINKED_TAB);
        let mut manifest = PackManifest::from_files(&send.files).unwrap();
        manifest.parent.clone_from(&parent);
        let files = files_with(&manifest, send.files.kind(), send.files.widgets());
        crate::deck_pack_tab::DeckPackSend { base: SendBase { pack: opened.id, parent }, files, ..send }
    };
    let first = resumed(&owner, pin_parent);
    applied(&mut owner, LINKED_TAB, &first);
    let refused = owner.receive_deck_pack(LINKED_TAB, &resumed(&owner, Some(opened.version.clone()))).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::DeckPackMismatch);
}
