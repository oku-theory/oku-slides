use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackKind, PackScope, PackVersion};

use super::*;
use crate::deck_pack_tab::{DeckPackSend, SendBase};
use crate::test_support::{add_version, fresh_dir, manifest, ns, session_on, slide_widget};
use crate::test_support_packs::{
    LINKED_TAB, OwnerState, applied, apply_valid, copy_send, deck_layouts_pack, embedded, files_with, link_of,
    linked_owner, owner_state, valid_send,
};
use crate::{BASE_PACK, Localizer, PRO_THEME, SessionErrorKind};

/// `send` with its manifest changed by `change`, its files rebuilt.
fn with_manifest(send: DeckPackSend, change: impl FnOnce(&mut PackManifest)) -> DeckPackSend {
    let mut manifest = PackManifest::from_files(&send.files).unwrap();
    change(&mut manifest);
    let files = files_with(&manifest, send.files.kind(), send.files.widgets());
    DeckPackSend { files, ..send }
}

/// Asserts `received` is a refusal of `kind` and the owner unchanged since `before`.
fn assert_refused(owner: &Session, received: Result<Received>, kind: SessionErrorKind, before: &OwnerState) {
    assert_that!(received.map_err(|e| e.kind())).is_equal_to(Err(kind));
    assert_that!(&owner_state(owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_valid_send_is_applied() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    let send = valid_send(&owner, LINKED_TAB);
    applied(&mut owner, LINKED_TAB, &send);
    assert_that!(link_of(&owner, LINKED_TAB).expect).is_equal_to(send.files.version());
}

#[test]
fn receive_deck_pack_first_send_is_applied() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    assert_that!(link_of(&owner, LINKED_TAB).last_seq).is_none();
    let send = DeckPackSend { seq: 1, ..valid_send(&owner, LINKED_TAB) };
    applied(&mut owner, LINKED_TAB, &send);
    assert_that!(link_of(&owner, LINKED_TAB).last_seq).is_equal_to(Some(1));
}

#[test]
fn receive_deck_pack_from_an_unlinked_tab_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let send = valid_send(&owner, LINKED_TAB);
    let before = owner_state(&owner);
    let received = owner.receive_deck_pack("t-9", &send);
    assert_refused(&owner, received, SessionErrorKind::DeckPackMismatch, &before);
}

#[test]
fn receive_deck_pack_older_seq_is_stale() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let fifth = DeckPackSend { seq: 5, ..valid_send(&owner, LINKED_TAB) };
    applied(&mut owner, LINKED_TAB, &fifth);
    let send = DeckPackSend { seq: 3, ..valid_send(&owner, LINKED_TAB) };
    let before = owner_state(&owner);
    assert_that!(owner.receive_deck_pack(LINKED_TAB, &send).ok()).is_equal_to(Some(Received::Stale));
    assert_that!(owner_state(&owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_same_seq_is_delivered() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let send = valid_send(&owner, LINKED_TAB);
    applied(&mut owner, LINKED_TAB, &send);
    let before = owner_state(&owner);
    assert_that!(owner.receive_deck_pack(LINKED_TAB, &send).ok()).is_equal_to(Some(Received::Delivered));
    assert_that!(owner_state(&owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_of_the_expected_version_is_delivered() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, pin) = linked_owner(data.path());
    let send = DeckPackSend { files: embedded(&owner, &pin.version), ..valid_send(&owner, LINKED_TAB) };
    let before = owner_state(&owner);
    assert_that!(owner.receive_deck_pack(LINKED_TAB, &send).ok()).is_equal_to(Some(Received::Delivered));
    assert_that!(owner_state(&owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_during_an_agent_turn_is_busy() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    owner.begin_turn(1, "tour");
    let before = owner_state(&owner);
    let received = owner.receive_deck_pack(LINKED_TAB, &valid_send(&owner, LINKED_TAB));
    assert_that!(received.ok()).is_equal_to(Some(Received::Busy));
    assert_that!(owner_state(&owner)).is_equal_to(before);
}

#[test]
fn receive_deck_pack_in_history_is_busy() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let send = valid_send(&owner, LINKED_TAB);
    owner.history_open().unwrap();
    assert_that!(owner.receive_deck_pack(LINKED_TAB, &send).ok()).is_equal_to(Some(Received::Busy));
    owner.history_close().unwrap();
    assert_that!(link_of(&owner, LINKED_TAB).last_seq).is_none();
}

#[test]
fn receive_deck_pack_after_the_owner_opened_another_document_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let send = valid_send(&owner, LINKED_TAB);
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    owner.open(&fixture).unwrap();
    assert_that!(owner.user_deck_uid()).is_not_equal_to(link_of(&owner, LINKED_TAB).deck_uid.clone());
    let before = owner_state(&owner);
    let received = owner.receive_deck_pack(LINKED_TAB, &send);
    assert_refused(&owner, received, SessionErrorKind::DeckPackMismatch, &before);
}

/// One way a send or its owner differs from the valid case, by one field or one owner fact.
#[derive(Debug, Clone, Copy)]
enum Mismatch {
    BasePack,
    BaseParent,
    LibraryScope,
    SameIdOtherParent,
    SameIdOtherNamespace,
    OtherKind,
    CopyIdInLibrary,
    CopyIdBuiltin,
    CopyIdPinnedInOtherKind,
    CopyWithParent,
    CopyOtherNamespace,
    ClosureMovesAnotherPin,
    PinMovedFromExpect,
    LibraryHasCurrent,
    OverBudget,
}

/// The owner's sources as their zip holds them: replica snapshot, images, fonts, packs, themes.
type SourceContent = (Vec<u8>, BTreeMap<String, Vec<u8>>, BTreeMap<String, Vec<u8>>, Embedded, Embedded);

/// Embedded pack or theme versions.
type Embedded = BTreeMap<PackVersion, PackFiles>;

/// The owner's [`SourceContent`], to compare byte for byte.
fn source_content(owner: &Session) -> SourceContent {
    let s = &owner.doc.sources;
    (s.replica().snapshot().unwrap(), s.assets.clone(), s.fonts.clone(), s.packs.clone(), s.themes.clone())
}

/// The send of case `case`, the owner changed first when the case is an owner fact.
fn mismatched(owner: &mut Session, case: Mismatch) -> DeckPackSend {
    let send = valid_send(owner, LINKED_TAB);
    match case {
        Mismatch::BasePack => DeckPackSend { base: SendBase { pack: Uuid::new_v4(), ..send.base.clone() }, ..send },
        Mismatch::BaseParent => DeckPackSend { base: SendBase { parent: None, ..send.base.clone() }, ..send },
        Mismatch::LibraryScope => with_manifest(send, |m| m.scope = PackScope::Library),
        Mismatch::SameIdOtherParent => with_manifest(send, |m| m.parent = None),
        Mismatch::SameIdOtherNamespace => with_manifest(send, |m| m.namespace = ns("autre")),
        Mismatch::OtherKind => {
            let manifest = PackManifest::from_files(&send.files).unwrap();
            let files = files_with(&manifest, PackKind::Layouts, send.files.widgets());
            DeckPackSend { files, ..send }
        }
        Mismatch::CopyIdInLibrary => {
            let library = manifest(Uuid::new_v4(), "Bibliothèque", None, "2026-10-01T10:00:00Z");
            add_version(&owner.library, &library, &[]);
            with_manifest(copy_send(owner, LINKED_TAB), |m| m.id = library.id)
        }
        Mismatch::CopyIdBuiltin => with_manifest(copy_send(owner, LINKED_TAB), |m| m.id = BASE_PACK),
        Mismatch::CopyIdPinnedInOtherKind => {
            let layouts = deck_layouts_pack(owner);
            with_manifest(copy_send(owner, LINKED_TAB), |m| m.id = layouts.id)
        }
        Mismatch::CopyWithParent => {
            let parent = send.base.parent.clone();
            with_manifest(copy_send(owner, LINKED_TAB), |m| m.parent = parent)
        }
        Mismatch::CopyOtherNamespace => with_manifest(copy_send(owner, LINKED_TAB), |m| m.namespace = ns("autre")),
        // `essentiel` is pinned as layouts: requiring it as widgets would move that pin.
        Mismatch::ClosureMovesAnotherPin => with_manifest(send, |m| m.requires.push(ns("essentiel"))),
        Mismatch::PinMovedFromExpect => {
            // Written straight into the sources: every public path is refused while linked.
            let moved = with_manifest(valid_send(owner, LINKED_TAB), |m| m.description.push('!')).files;
            let version = owner.doc.sources.add_pack(moved);
            let mut deck = owner.doc.sources.deck().clone();
            deck.widgets.get_mut(&ns("deck")).unwrap().version = version;
            owner.doc.sources.apply(deck, "moved", None).unwrap();
            send
        }
        Mismatch::LibraryHasCurrent => {
            let mut m = PackManifest::from_files(&send.files).unwrap();
            // A library pack of the same id, under a free namespace (`deck` is reserved).
            m.scope = PackScope::Library;
            m.parent = None;
            m.namespace = ns("importe");
            owner.library.add_version(m.id, &files_with(&m, send.files.kind(), send.files.widgets())).unwrap();
            send
        }
        Mismatch::OverBudget => {
            let total = source_bytes(&owner.doc.sources).unwrap();
            owner.source_limits.total_bytes = total + 1;
            send
        }
    }
}

#[rstest]
#[case::wrong_base_pack(Mismatch::BasePack)]
#[case::wrong_base_parent(Mismatch::BaseParent)]
#[case::library_scope_manifest(Mismatch::LibraryScope)]
#[case::same_id_with_another_parent(Mismatch::SameIdOtherParent)]
#[case::same_id_another_namespace(Mismatch::SameIdOtherNamespace)]
#[case::another_kind(Mismatch::OtherKind)]
#[case::copy_whose_id_the_library_has(Mismatch::CopyIdInLibrary)]
#[case::copy_whose_id_is_built_in(Mismatch::CopyIdBuiltin)]
#[case::copy_whose_id_the_owner_pins_in_the_other_kind(Mismatch::CopyIdPinnedInOtherKind)]
#[case::copy_with_a_parent(Mismatch::CopyWithParent)]
#[case::copy_under_another_namespace(Mismatch::CopyOtherNamespace)]
#[case::closure_would_move_another_pin(Mismatch::ClosureMovesAnotherPin)]
#[case::owner_pin_moved_from_expect(Mismatch::PinMovedFromExpect)]
#[case::library_now_has_a_current_version(Mismatch::LibraryHasCurrent)]
#[case::over_the_sources_budget(Mismatch::OverBudget)]
fn receive_deck_pack_mismatch_is_refused(#[case] case: Mismatch) {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    let send = mismatched(&mut owner, case);
    let (before, content) = (owner_state(&owner), source_content(&owner));
    let received = owner.receive_deck_pack(LINKED_TAB, &send);
    assert_refused(&owner, received, SessionErrorKind::DeckPackMismatch, &before);
    // Refused before anything is written: the sources are byte-for-byte unchanged.
    assert_that!(source_content(&owner)).is_equal_to(content);
    assert_that!(link_of(&owner, LINKED_TAB).last_seq).is_none();
}

#[test]
fn receive_deck_pack_refused_keeps_the_owner_widget_session() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, _) = linked_owner(data.path());
    // An open deck-mode widget session: its next write merges into the same step.
    owner.set_widget(None, "cover2", slide_widget("cover2")).unwrap();
    let send = mismatched(&mut owner, Mismatch::ClosureMovesAnotherPin);
    let received = owner.receive_deck_pack(LINKED_TAB, &send);
    assert_that!(received.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::DeckPackMismatch));
    assert_that!(owner.widget_session.is_some()).is_true();
}

#[test]
fn receive_deck_pack_back_to_the_opened_version_is_accepted() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (mut owner, pin) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    let back = DeckPackSend { files: embedded(&owner, &pin.version), ..valid_send(&owner, LINKED_TAB) };
    applied(&mut owner, LINKED_TAB, &back);
    assert_that!(crate::test_support::pin(&owner, "deck").map(|p| p.version)).is_equal_to(Some(pin.version));
}

#[test]
fn receive_deck_pack_outside_deck_mode_is_wrong_document() {
    let data = fresh_dir("oku-slides-deck-pack-validate-");
    let (owner, _) = linked_owner(data.path());
    let send = valid_send(&owner, LINKED_TAB);
    let mut theme_tab = session_on(data.path());
    theme_tab.open_theme_here(PRO_THEME).unwrap();
    let refused = theme_tab.receive_deck_pack(LINKED_TAB, &send).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

/// The fr text of `error`, without Fluent's isolation marks.
fn fr(error: &SessionError) -> String {
    error.localize(&Localizer::new(Some("fr"))).replace(['\u{2068}', '\u{2069}'], "")
}

#[rstest]
#[case::undo_in_pack_tab(
    crate::error::UndoInPackTabSnafu { pack: "Importé", tab: "Importé" }.build().into(),
    "Cette étape change le pack Importé, en cours d'édition dans l'onglet « Importé » : annulez-la dans cet onglet, ou fermez-le d'abord."
)]
#[case::deck_pack_mismatch(
    crate::error::DeckPackMismatchSnafu { pack: "Importé" }.build().into(),
    "Le pack Importé ne correspond plus à ce que la présentation embarque : ses changements restent dans son onglet."
)]
#[case::undo_compensation_failed(
    crate::error::UndoCompensationFailedSnafu.build().into(),
    "L'annulation n'a pas pu être défaite : rouvrez la présentation avant de continuer."
)]
fn deck_pack_errors_localize_in_fr(#[case] error: SessionError, #[case] expected: &str) {
    assert_that!(fr(&error).as_str()).is_equal_to(expected);
}
