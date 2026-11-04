use speculoos::prelude::*;

use super::*;

fn texts(phrases: &Phrases) -> String {
    phrases.committed()
}

#[test]
fn apply_partial_replaces_preview() {
    let (mut phrases, mut partial) = (Phrases::default(), "ancien".to_owned());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Partial("  bonjour  à ".into()), 0)).is_true();
    assert_that!(partial).is_equal_to("bonjour à".to_owned());
}

#[test]
fn apply_final_appends_clean_text_and_clears_preview() {
    let (mut phrases, mut partial) = (Phrases::default(), "deu".to_owned());
    apply(&mut phrases, &mut partial, &Event::Final { text: "Un.".into(), start: 0.0, end: 1.0 }, 0);
    apply(&mut phrases, &mut partial, &Event::Final { text: " Deux. ".into(), start: 1.0, end: 2.0 }, 0);
    assert_that!(texts(&phrases)).is_equal_to("Un. Deux.".to_owned());
    assert_that!(partial).is_equal_to(String::new());
}

#[test]
fn apply_empty_final_adds_nothing() {
    let (mut phrases, mut partial) = (Phrases::default(), String::new());
    apply(&mut phrases, &mut partial, &Event::Final { text: "   ".into(), start: 0.0, end: 1.0 }, 0);
    assert_that!(phrases.is_empty()).is_true();
}

#[test]
fn apply_other_events_are_not_results() {
    let (mut phrases, mut partial) = (Phrases::default(), String::new());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Done, 0)).is_false();
}

#[test]
fn apply_final_is_placed_on_the_listening_clock() {
    let (mut phrases, mut partial) = (Phrases::default(), String::new());
    apply(&mut phrases, &mut partial, &Event::Final { text: "Un.".into(), start: 2.0, end: 3.0 }, 16_000);
    assert_that!(phrases.take_upto(1)).is_equal_to(vec![Spoken {
        text: Some("Un.".into()),
        start: 16_000,
        end: 32_000,
    }]);
}

#[test]
fn apply_final_from_before_the_resume_is_dropped() {
    let (mut phrases, mut partial) = (Phrases::default(), String::new());
    apply(&mut phrases, &mut partial, &Event::Final { text: "Avant.".into(), start: 0.5, end: 1.5 }, 16_000);
    assert_that!(phrases.is_empty()).is_true();
}
