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
            TimelineStep::new("Bleu nuit choisi", StepStatus::Completed),
            TimelineStep { count: Some(6), ..TimelineStep::new("apply", StepStatus::Completed) },
        ],
        tools: 6,
        notes: 1,
        narration: "Je vérifie le rendu".into(),
        images: Vec::new(),
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
    v.plan = vec![TimelineStep::new("Lire la slide", StepStatus::Completed)];
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

fn paths(patches: &[oku_slides_a2ui::ServerMessage]) -> Vec<serde_json::Value> {
    let v = serde_json::to_value(patches).unwrap();
    v.as_array().unwrap().iter().map(|m| m["updateDataModel"]["path"].clone()).collect()
}

/// Ce qui change dans une étape repart seul : pas toute la liste, ni les détails des autres appels.
#[test]
fn turn_patches_changed_field_writes_only_that_field() {
    let sent = json!({"steps": [{"label": "note"}, {"label": "widgets", "count": 1}]});
    let data = json!({"steps": [{"label": "note"}, {"label": "widgets", "count": 2}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps/1/count")]);
}

/// Le statut d'un appel qui change dans un long groupe : seul ce statut repart, pas les arguments des autres.
#[test]
fn turn_patches_call_status_writes_only_that_status() {
    let call = |status: &str| json!({"label": "widgets", "status": status, "input": "{\"slide\": 4}"});
    let sent = json!({"steps": [{"label": "widgets", "calls": [call("completed"), call("in_progress")]}]});
    let data = json!({"steps": [{"label": "widgets", "calls": [call("completed"), call("failed")]}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps/0/calls/1/status")]);
}

/// Un objet qui perd une clé est réécrit entier : une écriture par clé ne retirerait rien.
#[test]
fn turn_patches_object_losing_a_key_is_rewritten_whole() {
    let sent = json!({"steps": [{"label": "Lire la slide"}, {"label": "a", "count": 1}]});
    let data = json!({"steps": [{"label": "Lire la slide"}, {"label": "a"}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps/1")]);
}

#[test]
fn turn_patches_appended_step_writes_only_the_new_step() {
    let sent = json!({"steps": [{"label": "note"}]});
    let data = json!({"steps": [{"label": "note"}, {"label": "widgets"}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps/1")]);
}

/// Une liste qui raccourcit (plan remplacé) repart entière : une écriture par index ne retirerait rien.
#[test]
fn turn_patches_shorter_list_rewrites_the_whole_list() {
    let sent = json!({"steps": [{"label": "a"}, {"label": "b"}]});
    let data = json!({"steps": [{"label": "a"}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps")]);
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

#[test]
fn first_line_with_bidi_controls_drops_them() {
    assert_that!(first_line("\u{202E}Bleu\u{2066} nuit\nPuis", 120)).is_equal_to("Bleu nuit".to_owned());
}

#[test]
fn first_line_of_only_hidden_characters_takes_the_next_line() {
    assert_that!(first_line("\u{200B}\u{FEFF}\n  \u{202E} Bleu", 120)).is_equal_to("Bleu".to_owned());
}

#[test]
fn first_line_hidden_characters_do_not_count_towards_max() {
    assert_that!(first_line("a\u{200B}b\u{200B}c\u{200B}d", 4)).is_equal_to("abcd".to_owned());
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

#[test]
fn turn_patches_root_losing_a_key_rewrites_the_whole_model() {
    let patches = serde_json::to_value(super::turn_patches(3, &json!({"a": 1, "b": 2}), &json!({"a": 1}))).unwrap();
    assert_that!(patches).is_equal_to(json!([
        {"version": "v0.9", "updateDataModel": {"surfaceId": "turn-3", "value": {"a": 1}}}
    ]));
}

/// Le plus ancien appel sorti de la liste (plafond) décale tous les autres : la liste repart
/// en une écriture, pas une par champ de chaque appel décalé.
#[test]
fn turn_patches_shifted_list_is_written_whole() {
    let call = |n: u32| json!({"label": "apply", "status": "completed", "input": n.to_string()});
    let sent = json!({"steps": [{"calls": [call(0), call(1), call(2)]}]});
    let data = json!({"steps": [{"calls": [call(1), call(2), call(3)]}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data))).is_equal_to(vec![json!("/steps/0/calls")]);
}

/// Contrat entre l'écrivain (`turn_patches`) et la copie de l'hôte (`apply_patch`) : les écritures
/// appliquées au modèle envoyé donnent le nouveau, quel que soit le changement.
#[rstest::rstest]
#[case::append(json!({"steps": [{"a": 1}]}), json!({"steps": [{"a": 1}, {"b": 2}]}))]
#[case::nested_field(json!({"steps": [{"calls": [{"s": "x"}]}]}), json!({"steps": [{"calls": [{"s": "y"}]}]}))]
#[case::lost_key(json!({"steps": [{"a": 1, "c": 2}]}), json!({"steps": [{"a": 1}]}))]
#[case::shorter_list(json!({"steps": [{"a": 1}, {"a": 2}]}), json!({"steps": [{"a": 3}]}))]
#[case::shift(json!({"steps": [{"calls": [1, 2, 3]}]}), json!({"steps": [{"calls": [2, 3, 4]}]}))]
#[case::lost_root_key(json!({"steps": [], "x": 1}), json!({"steps": []}))]
#[case::escaped_key(json!({"steps": [{"a/b~c": 1}]}), json!({"steps": [{"a/b~c": 2}]}))]
#[case::type_change(json!({"x": "s"}), json!({"x": {"y": 1}}))]
#[case::plan_update(json!({"steps": [{"s": "in_progress"}, {"s": "pending"}, {"calls": [1]}]}), json!({"steps": [{"s": "completed"}, {"s": "in_progress"}, {"calls": [1]}]}))]
fn turn_patches_applied_to_the_sent_model_give_the_new_one(
    #[case] sent: serde_json::Value,
    #[case] data: serde_json::Value,
) {
    let mut s = crate::agent::forms::confirm(
        "turn-3",
        &crate::agent::AskConfirm { message: "Ok ?".into(), danger: false },
        &fr(),
    );
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::set_data_model("turn-3", sent.clone())]);
    s.apply_patch(&super::turn_patches(3, &sent, &data));
    let json = serde_json::to_value(&s).unwrap();
    let models: Vec<_> = json["messages"].as_array().unwrap().iter().filter_map(|m| m.get("updateDataModel")).collect();
    assert_that!(models).has_length(1);
    assert_that!(models[0]["value"].clone()).is_equal_to(data);
}

/// Une mise à jour du plan coche deux entrées : deux écritures, pas la liste entière et ses appels.
#[test]
fn turn_patches_plan_update_writes_only_the_two_entries() {
    let calls = json!([{"label": "apply", "status": "completed", "input": "x".repeat(500)}]);
    let sent = json!({"steps": [{"label": "Lire", "status": "in_progress"}, {"label": "Peindre", "status": "pending"}, {"label": "apply", "calls": calls}]});
    let data = json!({"steps": [{"label": "Lire", "status": "completed"}, {"label": "Peindre", "status": "in_progress"}, {"label": "apply", "calls": calls}]});
    assert_that!(paths(&super::turn_patches(3, &sent, &data)))
        .is_equal_to(vec![json!("/steps/0/status"), json!("/steps/1/status")]);
}

#[test]
fn turn_surface_prompt_bubble_carries_the_pasted_images() {
    let image = oku_slides_a2ui::PromptImage { mime: "image/png".into(), width: 4, height: 3 };
    let v = TurnView { images: vec![image.clone()], ..view(TurnState::Running) };
    let s = turn_surface(&v, &Localizer::new(Some("en")));
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.as_str()).contains("\"images\":[{\"mime\":\"image/png\",\"width\":4,\"height\":3}]");
}
