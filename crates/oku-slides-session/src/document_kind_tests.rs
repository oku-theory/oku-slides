use rstest::rstest;
use speculoos::prelude::*;

use super::DocumentKind;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, opened_on, session_on};
use crate::test_support_packs::deck_pack_tab_of;
use crate::theme::PAPIER_THEME;

#[test]
fn document_kind_of_a_deck_tab_is_deck() {
    let data = fresh_dir("oku-slides-kind-");
    let s = session_on(data.path());
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Deck);
}

#[test]
fn document_kind_in_history_is_deck() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.history_open().unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Deck);
}

#[test]
fn document_kind_of_a_help_tab_is_help() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_help("fr", None).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Help);
}

#[test]
fn document_kind_of_a_pack_tab_is_pack() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Pack);
}

#[test]
fn document_kind_of_a_theme_tab_is_theme() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_theme_here(PAPIER_THEME).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Theme);
}

#[rstest]
#[case::deck(DocumentKind::Deck, "\"deck\"")]
#[case::help(DocumentKind::Help, "\"help\"")]
#[case::pack(DocumentKind::Pack, "\"pack\"")]
#[case::theme(DocumentKind::Theme, "\"theme\"")]
fn document_kind_serializes_as_its_name(#[case] kind: DocumentKind, #[case] json: &str) {
    assert_that!(serde_json::to_string(&kind).unwrap().as_str()).is_equal_to(json);
    assert_that!(format!("\"{}\"", kind.as_str()).as_str()).is_equal_to(json);
}

/// The session modes whose cached document must agree with their live kind.
#[derive(Debug, Clone, Copy)]
enum Mode {
    Deck,
    Help,
    Pack,
    DeckPack,
    Theme,
}

/// A session in `mode`, with the data dir that backs it.
fn session_in(mode: Mode) -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-kind-");
    let session = match mode {
        Mode::Deck => session_on(data.path()),
        Mode::Help => {
            let mut s = session_on(data.path());
            s.open_help("fr", None).unwrap();
            s
        }
        Mode::Pack => {
            let mut s = session_on(data.path());
            s.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
            s
        }
        Mode::DeckPack => {
            let mut owner = opened_on(data.path());
            let pin = adopt_deck_only_pack(&mut owner);
            deck_pack_tab_of(&mut owner, data.path(), pin.id)
        }
        Mode::Theme => {
            let mut s = session_on(data.path());
            s.open_theme_here(PAPIER_THEME).unwrap();
            s
        }
    };
    (data, session)
}

#[rstest]
#[case::deck(Mode::Deck)]
#[case::help(Mode::Help)]
#[case::pack(Mode::Pack)]
#[case::deck_pack(Mode::DeckPack)]
#[case::theme(Mode::Theme)]
fn tab_document_kind_matches_session_document_kind(#[case] mode: Mode) {
    let (_data, s) = session_in(mode);
    assert_that!(s.tab_info().document.kind()).is_equal_to(s.document_kind());
}

/// Decision 23: the session itself refuses a pack tab's deck requests, not only the tool list.
///
/// `handle` refuses them before `ensure_fits`, with the pack tab's own refusal `EditingPack`
/// (as `pack_tab_tests` pins for the file writes).
#[rstest]
#[case::history(crate::Request::HistoryOpen)]
#[case::save(crate::Request::Save { path: None, overwrite: false })]
#[case::move_(crate::Request::Move { path: std::env::temp_dir().join("moved.oku-slides.pdf") })]
fn pack_tab_refuses_a_deck_request_itself(#[case] req: crate::Request) {
    let (_data, mut s) = session_in(Mode::Pack);
    let err = s.handle(req).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingPack);
}

/// Pins are deck-only, like the `add_pack` and `remove_pack` tools: a pack tab refuses both ops.
#[rstest]
#[case::add(oku_slides_core::Op::AddPack {
    kind: oku_slides_core::PackKind::Widgets,
    namespace: crate::test_support::ns("acme"),
    pin: oku_slides_core::PackPin { id: uuid::Uuid::new_v4(), version: oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap(), requires: Vec::new() },
})]
#[case::remove(oku_slides_core::Op::RemovePack {
    kind: oku_slides_core::PackKind::Widgets,
    namespace: crate::test_support::ns("base"),
})]
fn pack_tab_refuses_a_pin_op_from_a_caller(#[case] op: oku_slides_core::Op) {
    let (_data, s) = session_in(Mode::Pack);
    let err = s.ensure_caller_ops(&[op]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::WrongDocument);
}
