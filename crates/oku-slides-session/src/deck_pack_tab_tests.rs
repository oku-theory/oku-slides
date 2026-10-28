use speculoos::prelude::*;

use oku_slides_core::{PackPin, PackScope, WidgetFile};

use super::*;
use crate::error::DeckPackMismatchSnafu;
use crate::library_index::library_writes;
use crate::pack::PackManifest;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, library_of, opened_on, rename_title, session_on};
use crate::test_support_packs::{CARD_SRC, CARD_V2, deck_pack_tab_of, ok};
use crate::{Localize, Request, SessionErrorKind, TabDocument};

/// An owner deck pinning a deck-only pack, and a deck-pack tab of `t-1` on that pack.
fn linked() -> (tempfile::TempDir, Session, Session, PackPin) {
    let data = fresh_dir("oku-slides-deck-pack-tab-");
    let mut owner = opened_on(data.path());
    let pin = adopt_deck_only_pack(&mut owner);
    let tab = deck_pack_tab_of(&mut owner, data.path(), pin.id);
    (data, owner, tab, pin)
}

/// Saves the tab's pack through the UI's request.
fn save(t: &mut Session) {
    ok(t.handle(Request::SavePack { as_new: false }), "save pack");
}

/// Takes the tab's due send, which must be there.
fn take(t: &mut Session) -> Arc<DeckPackSend> {
    t.take_deck_pack_send().unwrap_or_else(|| panic!("a due send"))
}

/// A refusal as the owner would answer it.
fn mismatch() -> SessionError {
    DeckPackMismatchSnafu { pack: "Importé" }.build().into()
}

/// Writes the `card` widget into the edited pack.
fn write_card(t: &mut Session, source: &str) {
    ok(t.set_widget(None, "card", source.into()), "card");
}

/// The `card` source of a send's files.
fn card_of(send: &DeckPackSend) -> Option<String> {
    send.files.widget(&WidgetFile::try_from("card".to_owned()).unwrap()).map(ToOwned::to_owned)
}

#[test]
fn deck_pack_tab_info_names_its_owner_and_pack() {
    let (_data, _owner, t, pin) = linked();
    let info = t.tab_info();
    assert_that!(info.document).is_equal_to(TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: pin.id });
    assert_that!(info.title).is_equal_to("Importé".to_owned());
}

#[test]
fn deck_pack_save_fills_a_due_outbox_and_writes_no_library_version() {
    let (_data, _owner, mut t, pin) = linked();
    // nextest runs each test in its own process: `WRITES` moves only for this test.
    let writes = library_writes();
    ok(t.apply("x", vec![rename_title("Importé 2")], None), "edit");
    save(&mut t);
    let send = take(&mut t);
    let manifest = PackManifest::from_files(&send.files).unwrap();
    assert_that!(manifest.scope).is_equal_to(PackScope::Deck);
    assert_that!(send.base).is_equal_to(SendBase { pack: pin.id, parent: Some(pin.version.clone()) });
    assert_that!(library_writes()).is_equal_to(writes);
    assert_that!(library_of(&t).versions(pin.id)).is_empty();
}

#[test]
fn deck_pack_widget_write_fills_the_outbox_but_is_not_due() {
    let (_data, _owner, mut t, _) = linked();
    write_card(&mut t, CARD_SRC);
    assert_that!(t.take_deck_pack_send()).is_none();
    assert_that!(t.unsent()).is_true();
    save(&mut t);
    assert_that!(card_of(&take(&mut t))).is_equal_to(Some(CARD_SRC.to_owned()));
}

#[test]
fn deck_pack_taken_send_is_not_taken_twice() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    assert_that!(t.take_deck_pack_send()).is_some();
    assert_that!(t.take_deck_pack_send()).is_none();
}

#[test]
fn deck_pack_seq_never_repeats_after_acknowledgement() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let first = take(&mut t).seq;
    t.acknowledge_deck_pack(first, Ok(()));
    write_card(&mut t, CARD_SRC);
    save(&mut t);
    // Seqs are compared with each other, never with absolute values.
    assert_that!(take(&mut t).seq).is_greater_than(first);
}

#[test]
fn deck_pack_tab_is_unsent_until_acknowledged() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let flags = |t: &Session| {
        let snapshot = t.snapshot();
        let unsent = snapshot.editing.as_ref().is_some_and(|e| e.unsent);
        (t.tab_info().dirty, unsent, snapshot.dirty)
    };
    assert_that!(flags(&t)).is_equal_to((true, true, false));
    let seq = take(&mut t).seq;
    t.acknowledge_deck_pack(seq, Ok(()));
    assert_that!(flags(&t)).is_equal_to((false, false, false));
}

#[test]
fn deck_pack_refused_send_waits_for_the_next_save_and_notices() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let seq = take(&mut t).seq;
    let refusal = mismatch();
    t.acknowledge_deck_pack(seq, Err(&refusal));
    assert_that!(t.take_deck_pack_send()).is_none();
    assert_that!(t.unsent()).is_true();
    let expected = refusal.localize(&t.localizer);
    assert_that!(t.snapshot().notice).is_equal_to(Some(expected.as_str()));
    save(&mut t);
    assert_that!(t.take_deck_pack_send()).is_some();
}

#[test]
fn deck_pack_parked_send_goes_after_unpark() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let seq = take(&mut t).seq;
    t.park_deck_pack(seq);
    assert_that!(t.take_deck_pack_send()).is_none();
    assert_that!(t.snapshot().notice).is_none();
    t.unpark_deck_pack();
    assert_that!(take(&mut t).seq).is_equal_to(seq);
}

#[test]
fn deck_pack_parked_send_is_due_after_flush() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let seq = take(&mut t).seq;
    t.park_deck_pack(seq);
    ok(t.flush_deck_pack(), "flush");
    assert_that!(t.take_deck_pack_send()).is_some();
}

#[test]
fn deck_pack_unpark_while_in_flight_makes_the_park_due() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let seq = take(&mut t).seq;
    t.unpark_deck_pack();
    t.park_deck_pack(seq);
    assert_that!(t.take_deck_pack_send()).is_some();
}

#[test]
fn deck_pack_save_while_in_flight_sends_again_after_acknowledgement() {
    let (_data, _owner, mut t, _) = linked();
    write_card(&mut t, CARD_SRC);
    save(&mut t);
    let s1 = take(&mut t).seq;
    write_card(&mut t, CARD_V2);
    save(&mut t);
    assert_that!(t.take_deck_pack_send()).is_none();
    t.acknowledge_deck_pack(s1, Ok(()));
    let again = take(&mut t);
    assert_that!(again.seq).is_greater_than(s1);
    assert_that!(card_of(&again)).is_equal_to(Some(CARD_V2.to_owned()));
}

#[test]
fn deck_pack_second_save_keeps_the_first_unacknowledged_base() {
    let (_data, _owner, mut t, pin) = linked();
    ok(t.handle(Request::SavePack { as_new: true }), "save as copy");
    let seq = take(&mut t).seq;
    t.park_deck_pack(seq);
    write_card(&mut t, CARD_SRC);
    save(&mut t);
    let send = take(&mut t);
    assert_that!(PackManifest::from_files(&send.files).unwrap().id).is_not_equal_to(pin.id);
    assert_that!(send.base.pack).is_equal_to(pin.id);
}

#[test]
fn deck_pack_undo_fills_the_outbox_with_the_version_before_the_last_edit() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let v1 = take(&mut t);
    t.acknowledge_deck_pack(v1.seq, Ok(()));
    // The edit: a widget write, one undo step of the working deck.
    write_card(&mut t, CARD_SRC);
    save(&mut t);
    let v2 = take(&mut t);
    t.acknowledge_deck_pack(v2.seq, Ok(()));
    ok(t.handle(Request::Undo), "undo");
    t.mark_due();
    // Versions are compared, never bytes: `created` differs between saves.
    assert_that!(take(&mut t).files.version()).is_equal_to(v1.files.version());
}

#[test]
fn deck_pack_end_turn_marks_the_outbox_due() {
    let (_data, _owner, mut t, _) = linked();
    write_card(&mut t, CARD_SRC);
    assert_that!(t.take_deck_pack_send()).is_none();
    t.end_turn();
    assert_that!(t.take_deck_pack_send()).is_some();
}

#[test]
fn deck_pack_flush_saves_a_dirty_tab_first() {
    let (_data, _owner, mut t, _) = linked();
    ok(t.apply("x", vec![rename_title("Importé 2")], None), "edit");
    assert_that!(t.snapshot().dirty).is_true();
    ok(t.flush_deck_pack(), "flush");
    let send = take(&mut t);
    assert_that!(PackManifest::from_files(&send.files).unwrap().name).is_equal_to("Importé 2".to_owned());
    assert_that!(t.snapshot().dirty).is_false();
}

#[test]
fn init_deck_pack_tab_on_a_used_session_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-tab-");
    let mut owner = opened_on(data.path());
    let pin = adopt_deck_only_pack(&mut owner);
    let seed = ok(owner.preview_seed(), "seed");
    let mut used = session_on(data.path());
    ok(used.apply("x", vec![rename_title("used")], None), "use");
    let refused = used.init_deck_pack_tab(seed, "t-1", pin.id, None).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::PackTabNotFresh);
}

#[test]
fn enter_pack_with_a_link_on_a_library_pack_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-tab-");
    let mut s = opened_on(data.path());
    let link = Box::new(TabLink { owner: "t-1".to_owned(), outbox: None });
    let refused = s.enter_pack(crate::BASE_PACK, None, Some(link)).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::DeckPackMismatch);
    assert_that!(s.editing_view()).is_none();
}

#[test]
fn deck_pack_stale_answer_leaves_the_outbox_due_at_the_next_flush() {
    let (_data, _owner, mut t, _) = linked();
    save(&mut t);
    let seq = take(&mut t).seq;
    crate::deck_pack_receive::settle_answer(&mut t, seq, &Ok(crate::deck_pack_receive::Received::Stale));
    assert_that!(t.take_deck_pack_send()).is_none();
    ok(t.flush_deck_pack(), "flush");
    assert_that!(t.take_deck_pack_send()).is_some();
}
