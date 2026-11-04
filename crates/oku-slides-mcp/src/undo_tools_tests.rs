use oku_slides_core::{Deck, Op, Target};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use oku_slides_i18n::{Localize, Localizer};

use super::{Direction, Played, Stepper, Steps, StepsArgs, outcome, play};

/// A session on the roadmap fixture, its data dir kept alive with it.
fn session() -> (Session, tempfile::TempDir) {
    let data =
        tempfile::Builder::new().prefix("oku-slides-mcp-undo-").tempdir().unwrap_or_else(|e| panic!("test dir: {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("default settings: {e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("test platform: {e}"));
    let mut s = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("test session: {e}"));
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("fixture: {e}"));
    (s, data)
}

/// Retitles the third slide: one undo step.
fn retitle(s: &mut Session, title: &str) {
    let slide = s.deck().slides[2].id.clone();
    let op = Op::SetParam { target: Target { slide, element: None }, param: "title".into(), value: json!(title) };
    s.apply("Titre", vec![op], None).unwrap_or_else(|e| panic!("retitle: {e}"));
}

/// `n` retitles; the deck after each one, the untouched deck first.
fn edits(s: &mut Session, n: usize) -> Vec<Deck> {
    let mut decks = vec![s.deck().clone()];
    for i in 0..n {
        retitle(s, &format!("Titre {i}"));
        decks.push(s.deck().clone());
    }
    decks
}

fn steps(n: u8) -> Steps {
    Steps::try_from(n).unwrap_or_else(|e| panic!("steps {n}: {e}"))
}

#[test]
fn undo_steps_three_undoes_three() {
    let (mut s, _data) = session();
    let decks = edits(&mut s, 4);
    let played = play(&mut s, Direction::Undo, steps(3));
    assert_that!(played.done).is_equal_to(3);
    assert_that!(played.refused.is_none()).is_true();
    assert_that!(s.deck()).is_equal_to(&decks[1]);
}

#[test]
fn undo_steps_beyond_stack_stops_and_reports_count() {
    let (mut s, _data) = session();
    let decks = edits(&mut s, 2);
    let played = play(&mut s, Direction::Undo, steps(5));
    assert_that!(played.done).is_equal_to(2);
    assert_that!(played.refused.is_none()).is_true();
    assert_that!(s.deck()).is_equal_to(&decks[0]);
}

#[test]
fn redo_steps_two_redoes_two() {
    let (mut s, _data) = session();
    let decks = edits(&mut s, 3);
    play(&mut s, Direction::Undo, steps(3));
    let played = play(&mut s, Direction::Redo, steps(2));
    assert_that!(played.done).is_equal_to(2);
    assert_that!(s.deck()).is_equal_to(&decks[2]);
}

#[test]
fn redo_steps_with_nothing_undone_does_nothing() {
    let (mut s, _data) = session();
    edits(&mut s, 1);
    assert_that!(play(&mut s, Direction::Redo, steps(1)).done).is_equal_to(0);
}

#[rstest]
#[case::zero(json!({ "steps": 0 }))]
#[case::above_cap(json!({ "steps": 21 }))]
#[case::unknown_field(json!({ "count": 2 }))]
fn steps_args_invalid_refused(#[case] args: serde_json::Value) {
    assert_that!(serde_json::from_value::<StepsArgs>(args).is_err()).is_true();
}

#[rstest]
#[case::absent(json!({}), 1)]
#[case::cap(json!({ "steps": 20 }), 20)]
fn steps_args_valid_read(#[case] args: serde_json::Value, #[case] expected: u8) {
    let parsed = serde_json::from_value::<StepsArgs>(args).unwrap_or_else(|e| panic!("steps: {e}"));
    assert_that!(parsed.steps).is_equal_to(steps(expected));
}

/// A refusal the fake stack says in the language asked.
#[derive(Debug, PartialEq)]
struct Refused;

impl Localize for Refused {
    fn localize(&self, l: &Localizer) -> String {
        format!("refused in {}", l.locale())
    }
}

/// An undo stack of `left` steps that refuses the step after `refuse_after` played ones.
struct Stack {
    left: u8,
    played: u8,
    refuse_after: Option<u8>,
}

impl Stepper for Stack {
    type Error = Refused;

    fn left(&self, _way: Direction) -> bool {
        self.left > 0
    }

    fn step(&mut self, _way: Direction) -> Result<(), Refused> {
        if self.refuse_after == Some(self.played) {
            return Err(Refused);
        }
        self.left -= 1;
        self.played += 1;
        Ok(())
    }
}

#[test]
fn undo_steps_refused_after_one_reports_done_and_stopped() {
    let mut stack = Stack { left: 5, played: 0, refuse_after: Some(1) };
    let played = play(&mut stack, Direction::Undo, steps(3));
    let reply = outcome(played, &Localizer::new(Some("fr")));
    assert_that!(reply).is_equal_to(Ok(json!({ "done": 1, "stopped": "refused in fr" })));
}

#[test]
fn undo_steps_refused_first_is_the_error() {
    let mut stack = Stack { left: 5, played: 0, refuse_after: Some(0) };
    let played = play(&mut stack, Direction::Undo, steps(3));
    assert_that!(outcome(played, &Localizer::new(Some("fr")))).is_equal_to(Err(Refused));
}

#[test]
fn outcome_all_played_reports_done_only() {
    let played: Played<Refused> = Played { done: 2, refused: None };
    assert_that!(outcome(played, &Localizer::new(Some("fr")))).is_equal_to(Ok(json!({ "done": 2 })));
}
