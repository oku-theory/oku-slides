use oku_slides_core::{Op, Target as OpTarget};
use serde_json::json;
use speculoos::prelude::*;

use crate::lib_tests_support::session_with_fixture;

/// A token that is not a size: Typst fails to render the deck (see `lib_tests`).
fn bad_size() -> Op {
    Op::SetToken { path: vec!["size".into(), "body".into()], value: json!(true) }
}

#[test]
fn note_render_failed_deck_render_records_its_error() {
    let mut s = session_with_fixture();
    let err = s.apply("Jeton", vec![bad_size()], None).unwrap_err();
    assert_that!(s.last_render_error().map(str::to_owned)).is_equal_to(Some(err.one_line()));
}

#[test]
fn note_render_later_successful_render_clears_it() {
    let mut s = session_with_fixture();
    let _ = s.apply("Jeton", vec![bad_size()], None);
    let slide = s.deck().slides[2].id.clone();
    let title = Op::SetParam { target: OpTarget { slide, element: None }, param: "title".into(), value: json!("Ok") };
    s.apply("Titre", vec![title], None).unwrap();
    assert_that!(s.last_render_error()).is_none();
}

#[test]
fn last_import_error_fresh_session_is_none() {
    let s = session_with_fixture();
    assert_that!(s.last_import_error()).is_none();
}
