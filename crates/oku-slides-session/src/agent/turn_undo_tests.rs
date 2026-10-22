use oku_slides_core::{Op, Target};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::json;
use speculoos::prelude::*;

use crate::test_support::{fresh_dir, opened_on, session};
use crate::{Request, Session, SessionErrorKind};

/// Nouveau titre de la troisième slide.
fn title_op(s: &Session, title: &str) -> Op {
    let slide = s.deck().slides[2].id.clone();
    Op::SetParam { target: Target { slide, element: None }, param: "title".into(), value: json!(title) }
}

/// Le tour `turn` de l'agent, qui retitre la troisième slide.
fn agent_turn(s: &mut Session, turn: u32, title: &str) {
    s.begin_turn(turn, "retitre la slide");
    let op = title_op(s, title);
    s.apply("Titre", vec![op], None).unwrap();
    s.end_turn();
}

/// Une modification faite à la main.
fn manual_edit(s: &mut Session, title: &str) {
    let op = title_op(s, title);
    s.apply("Titre", vec![op], None).unwrap();
}

#[test]
fn undo_turn_after_an_agent_turn_names_that_turn() {
    let mut s = session();
    agent_turn(&mut s, 7, "Par l'agent");
    assert_that!(s.snapshot().undo_turn).is_equal_to(Some(7));
}

#[test]
fn undo_turn_while_the_turn_is_open_names_the_open_turn() {
    let mut s = session();
    s.begin_turn(3, "retitre");
    manual_edit(&mut s, "Pendant");
    assert_that!(s.undo_turn()).is_equal_to(Some(3));
}

#[test]
fn undo_turn_after_a_manual_edit_is_none() {
    let mut s = session();
    agent_turn(&mut s, 7, "Par l'agent");
    manual_edit(&mut s, "À la main");
    assert_that!(s.snapshot().undo_turn).is_none();
}

#[test]
fn undo_turn_of_a_turn_without_changes_is_none() {
    let mut s = session();
    manual_edit(&mut s, "À la main");
    s.begin_turn(2, "explique");
    s.end_turn();
    assert_that!(s.undo_turn()).is_none();
}

#[test]
fn undo_turn_after_undoing_the_last_turn_names_the_previous_one() {
    let mut s = session();
    agent_turn(&mut s, 1, "Un");
    agent_turn(&mut s, 2, "Deux");
    s.undo().unwrap();
    assert_that!(s.undo_turn()).is_equal_to(Some(1));
}

#[test]
fn undo_turn_request_restores_the_deck_before_the_turn() {
    let mut s = session();
    manual_edit(&mut s, "Avant");
    let before = s.deck().clone();
    agent_turn(&mut s, 4, "Par l'agent");
    s.handle(Request::UndoTurn { turn: 4 }).unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.undo_turn()).is_none();
}

#[test]
fn undo_turn_request_is_redone_by_the_ordinary_redo() {
    let mut s = session();
    agent_turn(&mut s, 4, "Par l'agent");
    let after = s.deck().clone();
    s.handle(Request::UndoTurn { turn: 4 }).unwrap();
    s.handle(Request::Redo).unwrap();
    assert_that!(s.deck()).is_equal_to(&after);
    assert_that!(s.undo_turn()).is_equal_to(Some(4));
}

#[test]
fn undo_turn_request_for_a_turn_no_longer_on_top_is_refused_and_changes_nothing() {
    let mut s = session();
    agent_turn(&mut s, 4, "Par l'agent");
    manual_edit(&mut s, "À la main");
    let deck = s.deck().clone();
    let err = s.handle(Request::UndoTurn { turn: 4 }).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TurnNotOnTop);
    assert_that!(s.deck()).is_equal_to(&deck);
}

#[test]
fn undo_turn_refusal_is_localized() {
    let mut s = session();
    let err = s.handle(Request::UndoTurn { turn: 9 }).map(|_| ()).unwrap_err();
    let fr = err.localize(&Localizer::new(Some("fr")));
    let en = err.localize(&Localizer::new(Some("en")));
    assert_that!(fr.is_empty()).is_false();
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn undo_turn_in_pack_mode_is_none() {
    let data = fresh_dir("oku-slides-turn-undo-");
    let mut s = opened_on(data.path());
    agent_turn(&mut s, 5, "Par l'agent");
    let set = crate::ESSENTIEL_PACK;
    s.edit_pack(set).unwrap();
    assert_that!(s.snapshot().undo_turn).is_none();
    s.close_pack().unwrap();
    assert_that!(s.undo_turn()).is_equal_to(Some(5));
}

#[test]
fn undo_turn_after_opening_another_document_is_none() {
    let mut s = session();
    agent_turn(&mut s, 1, "Par l'agent");
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap();
    manual_edit(&mut s, "À la main");
    assert_that!(s.undo_turn()).is_none();
}

#[test]
fn undo_after_a_tune_turn_restores_the_imported_deck_then_undoes_the_import() {
    use crate::test_support::imported_fixture;
    let data = fresh_dir("oku-slides-turn-undo-tune-");
    let mut s = imported_fixture(data.path(), "minimal", crate::test_support::hifi(), |_| {});
    let imported = s.deck().clone();
    // Le tour de fine-tuning : deux corrections de l'agent, sur deux slides.
    s.begin_turn(1, "fine-tune l'import");
    for slide in [imported.slides[0].id.clone(), imported.slides[1].id.clone()] {
        let op = Op::SetNotes { slide, notes: "Revue".into() };
        s.apply("Noter", vec![op], None).unwrap();
    }
    s.end_turn();
    assert_that!(s.deck()).is_not_equal_to(&imported);
    s.undo().unwrap();
    assert_that!(s.deck()).is_equal_to(&imported);
    s.undo().unwrap();
    assert_that!(s.deck().slides.is_empty()).is_true();
}
