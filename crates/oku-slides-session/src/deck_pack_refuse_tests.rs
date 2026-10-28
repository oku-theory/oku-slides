use std::path::{Path, PathBuf};

use rstest::rstest;
use speculoos::prelude::*;

use oku_slides_core::{Op, PackKind};

use super::*;
use crate::test_support::{fresh_dir, ns, opened_on, pin, rename_title, session_on, slide_widget};
use crate::test_support_packs::{LINKED_TAB, applied, deck_layouts_pack, ok, owner_state, valid_send};
use crate::{Request, SessionErrorKind};

/// The linked tab's title, which every refusal names.
const TITLE: &str = "Mise en page";

/// An owner saved at `deck.oku-slides.pdf` in `dir`, its `deck-layouts` pack linked (pending) to `t-2`.
fn owner_in(data: &Path, dir: &Path) -> (Session, PackPin, PathBuf) {
    let mut owner = opened_on(data);
    let layouts = deck_layouts_pack(&mut owner);
    let path = dir.join(format!("deck{}", crate::EXTENSION));
    ok(owner.save(Some(&path)), "save");
    drop(ok(owner.link_deck_pack(LINKED_TAB, TITLE, layouts.id), "link"));
    (owner, layouts, path)
}

/// A copy of the owner's file, changed by `change` in another session on `data`, saved beside it.
fn changed_copy(data: &Path, path: &Path, change: impl FnOnce(&mut Session)) -> PathBuf {
    let mut other = session_on(data);
    ok(other.open(path), "open the copy");
    change(&mut other);
    let copy = path.with_file_name(format!("copie{}", crate::EXTENSION));
    ok(other.save(Some(&copy)), "save the copy");
    copy
}

/// One change of the linked pack's pin the owner must refuse.
#[derive(Debug, Clone, Copy)]
enum Change {
    AgentWidget,
    AddPackOnItsNamespace,
    AddPackOfItsIdUnderANewNamespace,
    RemovePack,
    RemoveDeckPack,
    MergeOfAnotherPin,
    MergeOfASecondNamespace,
    HistoryRestore,
    OwnPackSession,
}

/// Sets `owner` up for `change`; the changed copy to merge, for a merge.
fn prepare(owner: &mut Session, data: &Path, (layouts, path): (&PackPin, &Path), change: Change) -> Option<PathBuf> {
    match change {
        Change::MergeOfAnotherPin => Some(changed_copy(data, path, |other| {
            ok(other.set_widget(None, "cover2", slide_widget("cover2")), "other layout");
        })),
        Change::MergeOfASecondNamespace => Some(changed_copy(data, path, |other| {
            let mut deck = other.doc.sources.deck().clone();
            deck.layouts.insert(ns("seconde"), layouts.clone());
            other.doc.sources.apply(deck, "seconde", None).unwrap();
        })),
        Change::HistoryRestore => {
            applied(owner, LINKED_TAB, &valid_send(owner, LINKED_TAB));
            ok(owner.history_open(), "history");
            ok(owner.history_seek(0), "first version");
            None
        }
        _ => None,
    }
}

/// Tries `change` on `owner`, whose linked pack is `layouts`, merging `copy` for a merge.
fn try_change(owner: &mut Session, layouts: &PackPin, copy: Option<&Path>, change: Change) -> Result<()> {
    let deck_layouts = ns("deck-layouts");
    let apply = |ops| Request::Apply { label: "packs".into(), ops, merge: None };
    match change {
        Change::AgentWidget => {
            owner.set_widget_by_agent(Some(&deck_layouts), "cover", slide_widget("couverture")).map(|_| ())
        }
        Change::AddPackOnItsNamespace => {
            let other = pin(owner, "essentiel").unwrap();
            let pin = PackPin { id: layouts.id, version: other.version, requires: layouts.requires.clone() };
            owner.handle(apply(vec![Op::AddPack { kind: PackKind::Layouts, namespace: deck_layouts, pin }])).map(|_| ())
        }
        Change::AddPackOfItsIdUnderANewNamespace => {
            let op = Op::AddPack { kind: PackKind::Layouts, namespace: ns("seconde"), pin: layouts.clone() };
            owner.handle(apply(vec![op])).map(|_| ())
        }
        Change::RemovePack => {
            owner.handle(apply(vec![Op::RemovePack { kind: PackKind::Layouts, namespace: deck_layouts }])).map(|_| ())
        }
        Change::RemoveDeckPack => owner.remove_deck_pack(PackKind::Layouts, &deck_layouts),
        Change::MergeOfAnotherPin | Change::MergeOfASecondNamespace => owner.merge_file(copy.unwrap()).map(|_| ()),
        Change::HistoryRestore => owner.history_restore(),
        Change::OwnPackSession => owner.set_widget(None, "cover", slide_widget("couverture")).map(|_| ()),
    }
}

/// The user deck and its undo steps, the stashed deck's in history.
fn user_state(owner: &Session) -> (oku_slides_core::Deck, u64) {
    (owner.user_deck().clone(), owner.user_sources().undo_steps())
}

#[rstest]
#[case::agent_widget_write(Change::AgentWidget)]
#[case::raw_add_pack_on_its_namespace(Change::AddPackOnItsNamespace)]
#[case::raw_add_pack_of_its_id_under_a_new_namespace(Change::AddPackOfItsIdUnderANewNamespace)]
#[case::raw_remove_pack(Change::RemovePack)]
#[case::remove_deck_pack(Change::RemoveDeckPack)]
#[case::merge_of_a_copy_whose_pin_differs(Change::MergeOfAnotherPin)]
#[case::merge_of_a_second_namespace(Change::MergeOfASecondNamespace)]
#[case::history_restore_before_the_receive(Change::HistoryRestore)]
#[case::own_pack_session(Change::OwnPackSession)]
fn owner_refuses_changes_to_a_linked_pack_pin(#[case] change: Change) {
    let (data, dir) = (fresh_dir("oku-slides-deck-pack-refuse-"), fresh_dir("oku-slides-deck-pack-refuse-file-"));
    let (mut owner, layouts, path) = owner_in(data.path(), dir.path());
    assert_that!(owner.deck_links[LINKED_TAB].pending).is_true();
    let copy = prepare(&mut owner, data.path(), (&layouts, &path), change);
    let history = matches!(change, Change::HistoryRestore);
    let (before, user_before) = ((!history).then(|| owner_state(&owner)), user_state(&owner));
    let refused = try_change(&mut owner, &layouts, copy.as_deref(), change).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(refused.lease_holder()).is_equal_to(Some(TITLE));
    assert_that!(user_state(&owner)).is_equal_to(user_before);
    assert_that!((!history).then(|| owner_state(&owner))).is_equal_to(before);
    let expect = owner.deck_links[LINKED_TAB].expect.clone();
    assert_that!(owner.user_deck().layouts.get(&ns("deck-layouts")).map(|p| p.version.clone()))
        .is_equal_to(Some(expect));
}

#[test]
fn owner_writes_to_an_unlinked_pack_still_pass() {
    let (data, dir) = (fresh_dir("oku-slides-deck-pack-refuse-"), fresh_dir("oku-slides-deck-pack-refuse-file-"));
    let (mut owner, _, _) = owner_in(data.path(), dir.path());
    let note = crate::test_support_packs::CARD_SRC.replace("\"card\"", "\"note\"");
    ok(owner.set_widget(None, "note", note), "a widget of the deck's own widgets pack");
    assert_that!(pin(&owner, "deck")).is_some();
}

#[test]
fn owner_apply_of_unrelated_ops_still_passes() {
    let (data, dir) = (fresh_dir("oku-slides-deck-pack-refuse-"), fresh_dir("oku-slides-deck-pack-refuse-file-"));
    let (mut owner, _, _) = owner_in(data.path(), dir.path());
    ok(owner.apply("titre", vec![rename_title("Autre titre")], None), "rename");
    assert_that!(owner.deck().params.title.as_deref()).is_equal_to(Some("Autre titre"));
}
