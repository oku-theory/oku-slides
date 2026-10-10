use oku_slides_a2ui::{StepStatus, TimelineStep};
use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use super::{TurnState, TurnView, first_line, turn_data, turn_surface};

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn view(state: TurnState) -> TurnView {
    TurnView {
        n: 3,
        prompt: "Change le fond de la slide quatre.".into(),
        plan: Vec::new(),
        steps: vec![
            TimelineStep { label: "Bleu nuit choisi".into(), status: StepStatus::Completed, count: None },
            TimelineStep { label: "apply".into(), status: StepStatus::Completed, count: Some(6) },
        ],
        tools: 6,
        notes: 1,
        narration: "Je vérifie le rendu".into(),
        state,
    }
}

#[test]
fn turn_data_running_is_collapsed_with_narration() {
    let d = turn_data(&view(TurnState::Running), &fr());
    assert_that!(d["collapsed"].clone()).is_equal_to(json!(true));
    assert_that!(d["running"].clone()).is_equal_to(json!(true));
    assert_that!(d["narration"].clone()).is_equal_to(json!("Je vérifie le rendu"));
    assert_that!(d["outcome"].clone()).is_equal_to(json!(""));
}

/// Pendant le tour, le résumé compte outils et notes ; la durée ne vient qu'à la fin.
#[test]
fn turn_data_running_summary_omits_the_duration() {
    let d = turn_data(&view(TurnState::Running), &fr());
    assert_that!(d["summary"].as_str().unwrap()).is_equal_to("6 outils · 1 note");
}

#[test]
fn turn_data_done_collapses_with_summary() {
    let d = turn_data(&view(TurnState::Done { secs: 38 }), &fr());
    assert_that!(d["collapsed"].clone()).is_equal_to(json!(true));
    assert_that!(d["running"].clone()).is_equal_to(json!(false));
    assert_that!(d["narration"].clone()).is_equal_to(json!(""));
    insta::assert_snapshot!(d["summary"].as_str().unwrap());
}

#[test]
fn turn_data_stopped_says_stopped() {
    let d = turn_data(&view(TurnState::Stopped { stop: "cancelled".into(), secs: 4 }), &fr());
    assert_that!(d["outcome"].clone()).is_equal_to(json!("Arrêté."));
}

#[test]
fn turn_data_failed_shows_the_message() {
    let d = turn_data(&view(TurnState::Failed { message: "L'agent s'est arrêté.".into(), secs: 2 }), &fr());
    assert_that!(d["outcome"].clone()).is_equal_to(json!("L'agent s'est arrêté."));
}

#[test]
fn turn_data_without_notes_omits_them_from_the_summary() {
    let mut v = view(TurnState::Done { secs: 4 });
    v.notes = 0;
    v.tools = 1;
    assert_that!(turn_data(&v, &fr())["summary"].as_str().unwrap()).is_equal_to("1 outil · 4 s");
}

#[test]
fn turn_data_with_plan_lists_plan_before_steps() {
    let mut v = view(TurnState::Running);
    v.plan = vec![TimelineStep { label: "Lire la slide".into(), status: StepStatus::Completed, count: None }];
    let d = turn_data(&v, &fr());
    let labels: Vec<_> = d["steps"].as_array().unwrap().iter().map(|s| s["label"].clone()).collect();
    assert_that!(labels).is_equal_to(vec![json!("Lire la slide"), json!("Bleu nuit choisi"), json!("apply")]);
}

#[test]
fn turn_surface_done_matches_snapshot() {
    let s = turn_surface(&view(TurnState::Done { secs: 38 }), &fr());
    assert_that!(s.id.as_str()).is_equal_to("turn-3");
    insta::assert_json_snapshot!(s);
}

#[test]
fn turn_patches_writes_only_the_changed_keys() {
    let sent = json!({"narration": "Je lis", "collapsed": false, "steps": []});
    let data = json!({"narration": "Je lis la slide", "collapsed": false, "steps": []});
    let patches = serde_json::to_value(super::turn_patches(3, &sent, &data)).unwrap();
    assert_that!(patches).is_equal_to(json!([
        {"version": "v0.9", "updateDataModel": {"surfaceId": "turn-3", "path": "/narration", "value": "Je lis la slide"}}
    ]));
}

#[test]
fn turn_patches_writes_one_update_per_changed_key() {
    let sent = json!({"narration": "Je lis", "collapsed": false, "summary": "0 outil · 0 s"});
    let data = json!({"narration": "", "collapsed": true, "summary": "0 outil · 0 s"});
    let patches = serde_json::to_value(super::turn_patches(3, &sent, &data)).unwrap();
    let paths: Vec<_> = patches.as_array().unwrap().iter().map(|m| m["updateDataModel"]["path"].clone()).collect();
    // Dans l'ordre des clés du nouveau modèle ; `summary`, inchangé, ne repart pas.
    assert_that!(paths).is_equal_to(vec![json!("/narration"), json!("/collapsed")]);
}

#[test]
fn turn_patches_unchanged_data_writes_nothing() {
    let data = json!({"narration": "Je lis", "collapsed": false, "steps": []});
    assert_that!(super::turn_patches(3, &data, &data)).is_empty();
}

/// Un modèle qui n'est pas un objet n'a pas de clés racine : il est réécrit en entier.
#[test]
fn turn_patches_non_object_data_rewrites_the_whole_model() {
    let patches = serde_json::to_value(super::turn_patches(3, &json!({"narration": "x"}), &json!([1]))).unwrap();
    assert_that!(patches).is_equal_to(json!([
        {"version": "v0.9", "updateDataModel": {"surfaceId": "turn-3", "value": [1]}}
    ]));
}

#[rstest::rstest]
#[case::max_turn_requests("max_turn_requests", "L'agent a atteint sa limite d'étapes.")]
#[case::refusal("refusal", "L'agent a refusé.")]
#[case::interrupted("interrupted", "Interrompu.")]
#[case::unknown("bizarre", "Tour terminé (bizarre).")]
fn turn_data_stop_reason_has_its_outcome(#[case] stop: &str, #[case] outcome: &str) {
    let d = turn_data(&view(TurnState::Stopped { stop: stop.into(), secs: 1 }), &fr());
    assert_that!(d["outcome"].as_str().unwrap()).is_equal_to(outcome);
}

#[rstest::rstest]
#[case::exact("abcd", 4, "abcd")]
#[case::empty("", 4, "")]
#[case::multibyte("éàüöß", 3, "éà…")]
#[case::zero("abc", 0, "")]
#[case::max_one_cut("abc", 1, "…")]
#[case::max_one_fits("a", 1, "a")]
#[case::blank("  \n\t \n ", 120, "")]
#[case::blank_lines_then_text("\n   \n  Bleu nuit  \n", 120, "Bleu nuit")]
fn first_line_edge_cases(#[case] text: &str, #[case] max: usize, #[case] expected: &str) {
    assert_that!(first_line(text, max)).is_equal_to(expected.to_owned());
}

#[test]
fn first_line_with_newline_and_length_cuts() {
    assert_that!(first_line("\nBleu nuit.\nPuis…", 120)).is_equal_to("Bleu nuit.".to_owned());
    assert_that!(first_line("abcdef", 4)).is_equal_to("abc…".to_owned());
}

/// Un tour ouvert sans sa consigne (perdue par un relais en retard) n'affiche pas de bulle vide.
#[test]
fn turn_surface_without_prompt_has_no_prompt_component() {
    let v = TurnView { prompt: String::new(), ..view(TurnState::Running) };
    let json = serde_json::to_string(&turn_surface(&v, &fr())).unwrap();
    assert_that!(json).does_not_contain("\"Prompt\"");
    assert_that!(json).does_not_contain("\"prompt\"");
    assert_that!(json).contains("\"Timeline\"");
}
