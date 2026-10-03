use speculoos::prelude::*;

use super::*;

#[test]
fn apply_partial_replaces_preview() {
    let (mut phrases, mut partial) = (Vec::new(), "ancien".to_owned());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Partial("  bonjour  à ".into()))).is_true();
    assert_that!(partial).is_equal_to("bonjour à".to_owned());
}

#[test]
fn apply_final_appends_clean_text_and_clears_preview() {
    let (mut phrases, mut partial) = (vec![Some("Un.".to_owned())], "deu".to_owned());
    apply(&mut phrases, &mut partial, &Event::Final { text: " Deux. ".into(), start: 0.0, end: 1.0 });
    assert_that!(phrases).is_equal_to(vec![Some("Un.".to_owned()), Some("Deux.".to_owned())]);
    assert_that!(partial).is_equal_to(String::new());
}

#[test]
fn apply_empty_final_adds_nothing() {
    let (mut phrases, mut partial) = (Vec::new(), String::new());
    apply(&mut phrases, &mut partial, &Event::Final { text: "   ".into(), start: 0.0, end: 1.0 });
    assert_that!(phrases).is_empty();
}

#[test]
fn apply_other_events_are_not_results() {
    let (mut phrases, mut partial) = (Vec::new(), String::new());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Done)).is_false();
}
