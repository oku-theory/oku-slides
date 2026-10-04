use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{KEPT_TURNS, TurnOut, Turns};
use crate::AgentEvent;

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn user(turn: u32) -> AgentEvent {
    AgentEvent::UserText { turn, text: "Change le fond".into() }
}

/// Les données du dernier tour, telles qu'envoyées (le module de test voit les champs privés).
fn data(t: &Turns) -> serde_json::Value {
    t.turns.back().map(|turn| turn.sent.clone()).unwrap_or_default()
}

#[test]
fn turns_user_text_opens_a_turn_surface() {
    let mut t = Turns::default();
    let out = t.apply(&user(1), Instant::now(), &fr());
    assert_that!(matches!(&out[..], [TurnOut::Show(s)] if s.id == "turn-1")).is_true();
}

#[test]
fn turns_chunks_merge_into_the_narration() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Je lis ".into() }, now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "la slide".into() }, now, &fr());
    assert_that!(data(&t)["narration"].as_str().unwrap()).is_equal_to("Je lis la slide");
}

#[test]
fn turns_tool_after_text_turns_it_into_a_note() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Bleu nuit choisi.".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "apply".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    let d = data(&t);
    assert_that!(d["steps"][0]["label"].as_str().unwrap()).is_equal_to("Bleu nuit choisi.");
    assert_that!(d["steps"][1]["count"].as_u64().unwrap()).is_equal_to(1);
    assert_that!(d["narration"].as_str().unwrap()).is_equal_to("");
}

#[test]
fn turns_consecutive_tools_share_one_step() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "slide".into(), status: "completed".into() },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "b".into(), title: "apply".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate { turn: 1, id: "b".into(), title: None, status: Some("completed".into()) },
        now,
        &fr(),
    );
    let d = data(&t);
    assert_that!(d["steps"].as_array().unwrap().len()).is_equal_to(1);
    assert_that!(d["steps"][0]["count"].as_u64().unwrap()).is_equal_to(2);
    assert_that!(d["steps"][0]["status"].as_str().unwrap()).is_equal_to("completed");
}

#[test]
fn turns_latest_plan_replaces_the_previous_one() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let item = |c: &str, s: &str| crate::PlanItem { content: c.into(), status: s.into() };
    t.apply(&AgentEvent::Plan { turn: 1, entries: vec![item("Lire", "pending")] }, now, &fr());
    t.apply(
        &AgentEvent::Plan { turn: 1, entries: vec![item("Lire", "completed"), item("Peindre", "pending")] },
        now,
        &fr(),
    );
    assert_that!(data(&t)["steps"].as_array().unwrap().len()).is_equal_to(2);
}

#[test]
fn turns_end_turn_collapses_and_reports_narration() {
    let mut t = Turns::default();
    let start = Instant::now();
    t.apply(&user(1), start, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Fond posé. Titre en blanc.".into() }, start, &fr());
    let out =
        t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, start + Duration::from_secs(38), &fr());
    assert_that!(data(&t)["collapsed"].as_bool().unwrap()).is_true();
    assert_that!(
        out.iter()
            .any(|o| matches!(o, TurnOut::Ended { turn: 1, narration } if narration == "Fond posé. Titre en blanc."))
    )
    .is_true();
}

#[test]
fn turns_cancelled_turn_says_stopped() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() }, now, &fr());
    assert_that!(data(&t)["outcome"].as_str().unwrap()).is_equal_to("Arrêté.");
}

#[test]
fn turns_conversation_forgets_every_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&AgentEvent::Conversation, now, &fr());
    assert_that!(out).is_empty();
    assert_that!(t.turns.is_empty()).is_true(); // speculoos n'a pas `is_empty` pour `VecDeque`
}

#[test]
fn turns_status_without_a_turn_changes_nothing() {
    let mut t = Turns::default();
    let out = t.apply(&AgentEvent::Status { state: crate::AgentState::Working }, Instant::now(), &fr());
    assert_that!(out).is_empty();
}

#[test]
fn turns_unchanged_data_sends_no_patch() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let long = "x".repeat(200);
    t.apply(&AgentEvent::Chunk { turn: 1, text: long }, now, &fr());
    // Au-delà de la première ligne affichée (120 caractères), un morceau ne change rien à l'écran.
    let out = t.apply(&AgentEvent::Chunk { turn: 1, text: "y".into() }, now, &fr());
    assert_that!(out).is_empty();
}

#[test]
fn turns_update_of_an_earlier_tool_is_applied_and_a_failure_stays_visible() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "slide".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "b".into(), title: "apply".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    // Une note entre deux groupes : l'outil `a` n'est plus dans la dernière étape.
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Je vérifie.".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "c".into(), title: "render_slide".into(), status: "completed".into() },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate { turn: 1, id: "a".into(), title: None, status: Some("failed".into()) },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate { turn: 1, id: "b".into(), title: None, status: Some("completed".into()) },
        now,
        &fr(),
    );
    assert_that!(data(&t)["steps"][0]["status"].as_str().unwrap()).is_equal_to("failed");
}

#[test]
fn turns_update_of_an_unknown_tool_is_ignored() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(
        &AgentEvent::ToolUpdate { turn: 1, id: "zz".into(), title: None, status: Some("failed".into()) },
        now,
        &fr(),
    );
    assert_that!(out).is_empty();
}

#[test]
fn turns_tools_still_running_at_the_end_are_settled() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "apply".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, now, &fr());
    assert_that!(data(&t)["steps"][0]["status"].as_str().unwrap()).is_equal_to("completed");
}

#[test]
fn turns_events_after_the_end_are_ignored() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, now, &fr());
    let out = t.apply(&AgentEvent::Chunk { turn: 1, text: "tard".into() }, now, &fr());
    assert_that!(out).is_empty();
}

#[test]
fn turns_max_tokens_says_the_length_limit() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "max_tokens".into() }, now, &fr());
    assert_that!(data(&t)["outcome"].as_str().unwrap()).is_equal_to("L'agent a atteint sa limite de longueur.");
}

#[test]
fn turns_text_then_tool_then_end_reports_the_text() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Fond posé. Titre en blanc.".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "render_slide".into(), status: "completed".into() },
        now,
        &fr(),
    );
    let out = t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, now, &fr());
    let narration = out.iter().find_map(|o| match o {
        TurnOut::Ended { narration, .. } => Some(narration.clone()),
        _ => None,
    });
    assert_that!(narration).is_equal_to(Some("Fond posé. Titre en blanc.".to_owned()));
}

#[test]
fn turns_idle_without_turn_ended_closes_the_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&AgentEvent::Status { state: crate::AgentState::Idle }, now, &fr());
    assert_that!(data(&t)["collapsed"].as_bool().unwrap()).is_true();
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Ended { turn: 1, .. }))).is_true();
}

#[test]
fn turns_new_turn_closes_a_previous_one_left_running() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&user(2), now, &fr());
    // `open` rend : la mise à jour du tour 1, sa fin, puis la surface du tour 2.
    assert_that!(matches!(&out[0], TurnOut::Patch { id, .. } if id == "turn-1")).is_true();
    assert_that!(matches!(&out[1], TurnOut::Ended { turn: 1, .. })).is_true();
    assert_that!(matches!(&out[2], TurnOut::Show(s) if s.id == "turn-2")).is_true();
}

#[test]
fn turns_repeated_tool_call_counts_once() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let call = AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "apply".into(), status: "in_progress".into() };
    t.apply(&call, now, &fr());
    t.apply(&call, now, &fr());
    assert_that!(data(&t)["steps"][0]["count"].as_u64().unwrap()).is_equal_to(1);
    assert_that!(t.turns.back().unwrap().view.tools).is_equal_to(1);
}

#[test]
fn turns_blank_chunk_between_tools_keeps_one_step() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "slide".into(), status: "completed".into() },
        now,
        &fr(),
    );
    t.apply(&AgentEvent::Chunk { turn: 1, text: "\n\n".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "b".into(), title: "apply".into(), status: "completed".into() },
        now,
        &fr(),
    );
    assert_that!(data(&t)["steps"].as_array().unwrap()).has_length(1);
}

#[test]
fn turns_interrupted_turn_says_so() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&user(2), now, &fr());
    let patch = out.iter().find_map(|o| match o {
        TurnOut::Patch { id, messages } if id == "turn-1" => Some(serde_json::to_string(messages).unwrap()),
        _ => None,
    });
    assert_that!(patch.unwrap()).contains("Interrompu.");
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Ended { turn: 1, .. }))).is_true();
}

#[test]
fn turns_narration_stops_growing_past_the_cap() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "x".repeat(10) }, now, &fr());
    // Un seul morceau plus long que la borne, puis d'autres : la narration s'arrête exactement à la borne.
    t.apply(&AgentEvent::Chunk { turn: 1, text: "x".repeat(2 * oku_slides_session::agent::WHY_MAX) }, now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "y".into() }, now, &fr());
    assert_that!(t.turns.back().unwrap().view.narration.chars().count())
        .is_equal_to(oku_slides_session::agent::WHY_MAX);
}

#[test]
fn turns_eviction_removes_exactly_the_oldest() {
    let mut t = Turns::default();
    let now = Instant::now();
    let mut evicted = Vec::new();
    for n in 1..=(KEPT_TURNS as u32 + 2) {
        for o in t.apply(&user(n), now, &fr()) {
            if let TurnOut::Evict(k) = o {
                evicted.push(k);
            }
        }
    }
    assert_that!(evicted).is_equal_to(vec![1, 2]);
}
#[test]
fn turns_error_right_after_user_text_fails_and_ends_that_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&AgentEvent::Error { turn: 1, message: "Agent introuvable".into() }, now, &fr());
    assert_that!(data(&t)["outcome"].as_str().unwrap()).is_equal_to("Agent introuvable");
    assert_that!(data(&t)["collapsed"].as_bool().unwrap()).is_true();
    // Le tour est clos pour le hub aussi (sans reçu de repli : la narration est vide).
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Ended { turn: 1, narration } if narration.is_empty())))
        .is_true();
}

#[test]
fn turns_tool_group_past_the_step_cap_is_only_counted() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    for i in 0..super::STEPS_MAX / 2 {
        t.apply(&AgentEvent::Chunk { turn: 1, text: format!("note {i}") }, now, &fr());
        t.apply(
            &AgentEvent::ToolCall { turn: 1, id: format!("t{i}"), title: "apply".into(), status: "completed".into() },
            now,
            &fr(),
        );
    }
    assert_that!(t.turns.back().unwrap().view.steps.len()).is_equal_to(super::STEPS_MAX);
    let before = data(&t)["steps"][super::STEPS_MAX - 1].clone();
    let tools = t.turns.back().unwrap().view.tools;
    t.apply(&AgentEvent::Chunk { turn: 1, text: "encore".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "late".into(), title: "render".into(), status: "in_progress".into() },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate { turn: 1, id: "late".into(), title: None, status: Some("failed".into()) },
        now,
        &fr(),
    );
    assert_that!(t.turns.back().unwrap().view.steps.len()).is_equal_to(super::STEPS_MAX);
    assert_that!(data(&t)["steps"][super::STEPS_MAX - 1].clone()).is_equal_to(before);
    assert_that!(t.turns.back().unwrap().view.tools).is_equal_to(tools + 1);
}

/// Le relais a perdu `UserText` du tour 2 (retard) : les évènements du tour 2 vont au tour 2,
/// jamais au tour 1, et sa fin clôt bien le tour 2 pour le hub.
#[test]
fn turns_events_of_a_turn_whose_opening_was_lost_go_to_that_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let out = t.apply(&AgentEvent::Chunk { turn: 2, text: "Deux".into() }, now, &fr());
    // Le tour 1, resté ouvert, est interrompu ; le tour 2 s'ouvre sans consigne.
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Ended { turn: 1, .. }))).is_true();
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Show(s) if s.id == "turn-2"))).is_true();
    let out = t.apply(&AgentEvent::TurnEnded { turn: 2, stop: "end_turn".into() }, now, &fr());
    assert_that!(out.iter().any(|o| matches!(o, TurnOut::Ended { turn: 2, narration } if narration == "Deux")))
        .is_true();
    assert_that!(t.turns.front().unwrap().view.narration.as_str()).is_equal_to("");
}

#[test]
fn turns_late_event_of_a_closed_turn_does_not_touch_the_next_one() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&user(2), now, &fr());
    let out = t.apply(&AgentEvent::Chunk { turn: 1, text: "tard".into() }, now, &fr());
    assert_that!(out).is_empty();
    assert_that!(data(&t)["narration"].as_str().unwrap()).is_equal_to("");
}

#[test]
fn turns_stale_event_after_a_new_conversation_opens_no_ghost_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Conversation, now, &fr());
    let out = t.apply(&AgentEvent::Chunk { turn: 1, text: "tard".into() }, now, &fr());
    assert_that!(out).is_empty();
    assert_that!(t.turns.is_empty()).is_true();
}

/// La fin d'un tour change plusieurs clés : elles partent dans une seule mise à jour, une écriture par clé.
#[test]
fn turns_end_turn_patches_every_changed_key_at_once() {
    let mut t = Turns::default();
    let start = Instant::now();
    t.apply(&user(1), start, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Fond posé.".into() }, start, &fr());
    let out =
        t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() }, start + Duration::from_secs(4), &fr());
    let patches: Vec<_> = out.iter().filter(|o| matches!(o, TurnOut::Patch { .. })).collect();
    assert_that!(patches).has_length(1);
    let TurnOut::Patch { id, messages } = patches[0] else { unreachable!() };
    assert_that!(id.as_str()).is_equal_to("turn-1");
    let paths: Vec<String> = messages
        .iter()
        .map(|m| serde_json::to_value(m).unwrap()["updateDataModel"]["path"].as_str().unwrap().to_owned())
        .collect();
    assert_that!(paths).is_equal_to(["/collapsed", "/summary", "/narration", "/outcome"].map(str::to_owned).to_vec());
}

/// Une narration toute blanche ne devient pas une note vide quand un outil arrive.
#[test]
fn turns_blank_narration_before_a_tool_adds_no_note() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "  \n\t\n ".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall { turn: 1, id: "a".into(), title: "apply".into(), status: "completed".into() },
        now,
        &fr(),
    );
    let d = data(&t);
    assert_that!(d["steps"].as_array().unwrap()).has_length(1);
    assert_that!(d["steps"][0]["label"].as_str().unwrap()).is_equal_to("apply");
    assert_that!(d["narration"].as_str().unwrap()).is_equal_to("");
}
