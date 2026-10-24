use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{KEPT_TURNS, TurnOut, Turns};
use crate::AgentEvent;
use oku_slides_session::agent::AgentScope;

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn user(turn: u32) -> AgentEvent {
    user_in(turn, AgentScope::Slides)
}

fn user_in(turn: u32, scope: AgentScope) -> AgentEvent {
    AgentEvent::UserText { turn, scope, text: "Change le fond".into(), images: Vec::new() }
}

fn atelier() -> AgentScope {
    AgentScope::Atelier { pack: uuid::Uuid::nil() }
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "apply".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "slide".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "b".into(),
            title: "apply".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "b".into(),
            title: None,
            status: Some("completed".into()),
            input: None,
            output: None,
        },
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
    let out = t.apply(&AgentEvent::Conversation { scope: AgentScope::Slides }, now, &fr());
    assert_that!(out).is_empty();
    assert_that!(t.turns.is_empty()).is_true(); // speculoos n'a pas `is_empty` pour `VecDeque`
}

#[test]
fn turns_status_without_a_turn_changes_nothing() {
    let mut t = Turns::default();
    let out = t.apply(&AgentEvent::Status { state: crate::AgentState::Working, scope: None }, Instant::now(), &fr());
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "slide".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "b".into(),
            title: "apply".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    // Une note entre deux groupes : l'outil `a` n'est plus dans la dernière étape.
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Je vérifie.".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "c".into(),
            title: "render_slide".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "a".into(),
            title: None,
            status: Some("failed".into()),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "b".into(),
            title: None,
            status: Some("completed".into()),
            input: None,
            output: None,
        },
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
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "zz".into(),
            title: None,
            status: Some("failed".into()),
            input: None,
            output: None,
        },
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "apply".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "render_slide".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
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
    let out = t.apply(&AgentEvent::idle(), now, &fr());
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
    let call = AgentEvent::ToolCall {
        turn: 1,
        id: "a".into(),
        title: "apply".into(),
        status: "in_progress".into(),
        input: None,
        output: None,
    };
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
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "slide".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(&AgentEvent::Chunk { turn: 1, text: "\n\n".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "b".into(),
            title: "apply".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
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
            &AgentEvent::ToolCall {
                turn: 1,
                id: format!("t{i}"),
                title: "apply".into(),
                status: "completed".into(),
                input: None,
                output: None,
            },
            now,
            &fr(),
        );
    }
    assert_that!(t.turns.back().unwrap().view.steps.len()).is_equal_to(super::STEPS_MAX);
    let before = data(&t)["steps"][super::STEPS_MAX - 1].clone();
    let tools = t.turns.back().unwrap().view.tools;
    t.apply(&AgentEvent::Chunk { turn: 1, text: "encore".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "late".into(),
            title: "render".into(),
            status: "in_progress".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    t.apply(
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "late".into(),
            title: None,
            status: Some("failed".into()),
            input: None,
            output: None,
        },
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
    t.apply(&AgentEvent::Conversation { scope: AgentScope::Slides }, now, &fr());
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
    assert_that!(paths).is_equal_to(["/running", "/summary", "/narration", "/outcome"].map(str::to_owned).to_vec());
}

/// Une narration toute blanche ne devient pas une note vide quand un outil arrive.
#[test]
fn turns_blank_narration_before_a_tool_adds_no_note() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "  \n\t\n ".into() }, now, &fr());
    t.apply(
        &AgentEvent::ToolCall {
            turn: 1,
            id: "a".into(),
            title: "apply".into(),
            status: "completed".into(),
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    let d = data(&t);
    assert_that!(d["steps"].as_array().unwrap()).has_length(1);
    assert_that!(d["steps"][0]["label"].as_str().unwrap()).is_equal_to("apply");
    assert_that!(d["narration"].as_str().unwrap()).is_equal_to("");
}

#[test]
fn turns_evict_only_the_oldest_turn_of_the_same_scope() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user_in(1, atelier()), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: "end_turn".into() }, now, &fr());
    let mut evicted = Vec::new();
    for n in 2..=(KEPT_TURNS as u32 + 2) {
        for out in t.apply(&user_in(n, AgentScope::Slides), now, &fr()) {
            if let TurnOut::Evict(k) = out {
                evicted.push(k);
            }
        }
    }
    assert_that!(evicted).is_equal_to(vec![2]);
}

#[test]
fn turns_conversation_forgets_only_its_scope() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user_in(1, atelier()), now, &fr());
    t.apply(&user_in(2, AgentScope::Slides), now, &fr());
    t.apply(&AgentEvent::Conversation { scope: AgentScope::Slides }, now, &fr());
    assert_that!(t.turns.iter().map(|x| x.view.n).collect::<Vec<_>>()).is_equal_to(vec![1]);
}

#[test]
fn apply_draft_event_emits_nothing() {
    let mut turns = Turns::default();
    let out = turns.apply(
        &AgentEvent::Draft { text: Some("x".into()), origin: None, images: Vec::new() },
        Instant::now(),
        &fr(),
    );
    assert_that!(out).is_empty();
}

fn call(id: &str, title: &str, status: &str, input: Option<&str>) -> AgentEvent {
    AgentEvent::ToolCall {
        turn: 1,
        id: id.into(),
        title: title.into(),
        status: status.into(),
        input: input.map(str::to_owned),
        output: None,
    }
}

fn update(id: &str, status: &str, output: Option<&str>) -> AgentEvent {
    AgentEvent::ToolUpdate {
        turn: 1,
        id: id.into(),
        title: None,
        status: Some(status.into()),
        input: None,
        output: output.map(str::to_owned),
    }
}

/// Déplié, un groupe d'outils montre chaque appel : son nom, son statut, ses arguments, son résultat.
#[test]
fn turns_tool_step_lists_each_call_with_its_input_and_output() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("a", "slide", "completed", Some("{\"n\": 4}")), now, &fr());
    t.apply(&call("b", "apply", "in_progress", None), now, &fr());
    t.apply(&update("b", "failed", Some("index hors bornes")), now, &fr());
    let calls = data(&t)["steps"][0]["calls"].clone();
    assert_that!(calls).is_equal_to(serde_json::json!([
        {"label": "slide", "status": "completed", "input": "{\"n\": 4}"},
        {"label": "apply", "status": "failed", "output": "index hors bornes"},
    ]));
}

/// Le titre ACP d'un outil MCP (`mcp__serveur__outil`) se lit par son seul nom d'outil.
#[test]
fn turns_mcp_tool_title_shows_only_the_tool_name() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("a", "mcp__oku-slides__widgets", "completed", None), now, &fr());
    let d = data(&t);
    assert_that!(d["steps"][0]["label"].as_str().unwrap()).is_equal_to("widgets");
    assert_that!(d["steps"][0]["calls"][0]["label"].as_str().unwrap()).is_equal_to("widgets");
}

/// Une note coupée à sa première ligne garde son texte entier pour le dépliage.
#[test]
fn turns_long_note_keeps_the_rest_of_its_text_as_detail() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let text = "Corrections en une transaction.\n\nPuis le placement sur la grille.";
    t.apply(&AgentEvent::Chunk { turn: 1, text: text.into() }, now, &fr());
    t.apply(&call("a", "apply", "completed", None), now, &fr());
    let d = data(&t);
    assert_that!(d["steps"][0]["label"].as_str().unwrap()).is_equal_to("Corrections en une transaction.");
    // Le libellé montre déjà la première ligne entière : le détail n'en est que la suite.
    assert_that!(d["steps"][0]["detail"].as_str().unwrap()).is_equal_to("Puis le placement sur la grille.");
}

/// Une première ligne coupée (« … ») : le détail reprend le texte entier, pour la lire jusqu'au bout.
#[test]
fn turns_note_with_a_cut_first_line_keeps_its_full_text() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let text = format!("{}\n\nPuis la grille.", "Corrections en une transaction. ".repeat(5).trim());
    t.apply(&AgentEvent::Chunk { turn: 1, text: text.clone() }, now, &fr());
    t.apply(&call("a", "apply", "completed", None), now, &fr());
    assert_that!(data(&t)["steps"][0]["detail"].as_str().unwrap()).is_equal_to(text.as_str());
}

#[test]
fn turns_one_line_note_has_no_detail() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Bleu nuit choisi.".into() }, now, &fr());
    t.apply(&call("a", "apply", "completed", None), now, &fr());
    assert_that!(data(&t)["steps"][0].get("detail")).is_none();
}

/// Au-delà de `CALLS_MAX` appels dans un tour, les plus anciens sortent de la liste (ils restent
/// comptés) : les derniers, souvent ceux qui ont échoué, gardent leur détail.
#[test]
fn turns_calls_past_the_cap_keep_the_latest() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    for i in 0..=super::CALLS_MAX {
        t.apply(&call(&format!("t{i}"), "apply", "completed", Some(&i.to_string())), now, &fr());
    }
    let d = data(&t);
    assert_that!(d["steps"][0]["count"].as_u64().unwrap()).is_equal_to(super::CALLS_MAX as u64 + 1);
    let calls = d["steps"][0]["calls"].as_array().unwrap();
    assert_that!(calls).has_length(super::CALLS_MAX);
    assert_that!(calls[0]["input"].as_str().unwrap()).is_equal_to("1");
    assert_that!(calls[super::CALLS_MAX - 1]["input"].as_str().unwrap())
        .is_equal_to(super::CALLS_MAX.to_string().as_str());
}

/// Un appel sorti de la liste puis mis à jour ne touche à aucun autre appel ; les suivants restent à leur place.
#[test]
fn turns_update_of_a_dropped_call_changes_no_other_call() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    for i in 0..=super::CALLS_MAX {
        t.apply(&call(&format!("t{i}"), "apply", "in_progress", None), now, &fr());
    }
    let before = data(&t)["steps"][0]["calls"].clone();
    t.apply(&update("t0", "completed", Some("vieux")), now, &fr());
    assert_that!(data(&t)["steps"][0]["calls"].clone()).is_equal_to(before);
    t.apply(&update("t1", "failed", Some("index hors bornes")), now, &fr());
    let first = data(&t)["steps"][0]["calls"][0].clone();
    assert_that!(first["status"].as_str().unwrap()).is_equal_to("failed");
    assert_that!(first["output"].as_str().unwrap()).is_equal_to("index hors bornes");
}

/// Un groupe de milliers d'appels soldés n'en retient que les listés ; le compte et le statut restent justes.
#[test]
fn turns_thousands_of_settled_calls_keep_only_the_listed_ids() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("bad", "apply", "failed", None), now, &fr());
    for i in 0..3000 {
        t.apply(&call(&format!("t{i}"), "apply", "completed", None), now, &fr());
    }
    let kept = t.turns.back().map_or(0, |turn| turn.tools.len());
    assert_that!(kept).is_less_than_or_equal_to(super::CALLS_MAX);
    let d = data(&t);
    assert_that!(d["steps"][0]["count"].as_u64().unwrap()).is_equal_to(3001);
    assert_that!(d["steps"][0]["status"].as_str().unwrap()).is_equal_to("failed");
}

/// Un appel sorti de la liste encore en cours reste suivi : sa fin solde l'étape, puis il est oublié.
#[test]
fn turns_unlisted_open_call_settles_its_step_then_is_forgotten() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("slow", "apply", "in_progress", None), now, &fr());
    for i in 0..super::CALLS_MAX {
        t.apply(&call(&format!("t{i}"), "apply", "completed", None), now, &fr());
    }
    assert_that!(data(&t)["steps"][0]["status"].as_str().unwrap()).is_equal_to("in_progress");
    t.apply(&update("slow", "completed", None), now, &fr());
    assert_that!(data(&t)["steps"][0]["status"].as_str().unwrap()).is_equal_to("completed");
    assert_that!(t.turns.back().map(|turn| turn.tools.contains_key("slow"))).is_equal_to(Some(false));
}

/// La limite compte les appels du tour, pas ceux d'une étape.
#[test]
fn turns_call_cap_spans_the_steps_of_the_turn() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("first", "slide", "completed", None), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Je corrige.".into() }, now, &fr());
    for i in 0..super::CALLS_MAX {
        t.apply(&call(&format!("t{i}"), "apply", "completed", None), now, &fr());
    }
    let d = data(&t);
    assert_that!(d["steps"][0].get("calls")).is_none();
    assert_that!(d["steps"][0]["count"].as_u64().unwrap()).is_equal_to(1);
    assert_that!(d["steps"][2]["calls"].as_array().unwrap()).has_length(super::CALLS_MAX);
}

/// Les appels restés en cours sont soldés avec leur étape : aucun ◌ ne reste au dépliage.
fn settled_call_status(stop: &str) -> String {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("a", "apply", "in_progress", None), now, &fr());
    t.apply(&AgentEvent::TurnEnded { turn: 1, stop: stop.into() }, now, &fr());
    data(&t)["steps"][0]["calls"][0]["status"].as_str().unwrap_or_default().to_owned()
}

#[test]
fn turns_end_settles_each_call_as_completed() {
    assert_that!(settled_call_status("end_turn")).is_equal_to("completed".to_owned());
}

#[test]
fn turns_cancelled_end_settles_each_call_as_failed() {
    assert_that!(settled_call_status("cancelled")).is_equal_to("failed".to_owned());
}

/// Un appel annoncé deux fois reste une ligne, complétée par la seconde annonce.
#[test]
fn turns_repeated_call_fills_its_single_row() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("a", "apply", "in_progress", None), now, &fr());
    t.apply(&call("a", "apply", "in_progress", Some("{\"n\": 4}")), now, &fr());
    let calls = data(&t)["steps"][0]["calls"].clone();
    assert_that!(calls)
        .is_equal_to(serde_json::json!([{"label": "apply", "status": "in_progress", "input": "{\"n\": 4}"}]));
}

/// Une note d'une seule ligne trop longue (coupée par « … ») garde, elle aussi, son texte entier.
#[test]
fn turns_long_single_line_note_keeps_its_full_text() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let text = "Corrections en une transaction : puces, puis placement sur la grille. ".repeat(3);
    t.apply(&AgentEvent::Chunk { turn: 1, text: text.clone() }, now, &fr());
    t.apply(&call("a", "apply", "completed", None), now, &fr());
    let d = data(&t);
    assert_that!(d["steps"][0]["label"].as_str().unwrap().ends_with('…')).is_true();
    assert_that!(d["steps"][0]["detail"].as_str().unwrap()).is_equal_to(text.trim());
}

/// Le texte entier d'une note vient de l'agent : ses caractères invisibles ne s'affichent pas.
#[test]
fn turns_note_detail_drops_hidden_controls() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&AgentEvent::Chunk { turn: 1, text: "Ligne un.\n\nDeux\u{202E}trois.".into() }, now, &fr());
    t.apply(&call("a", "apply", "completed", None), now, &fr());
    assert_that!(data(&t)["steps"][0]["detail"].as_str().unwrap()).is_equal_to("Deuxtrois.");
}

/// Un nouveau titre d'appel (outil MCP) se lit lui aussi par son seul nom.
#[test]
fn turns_updated_mcp_title_shows_only_the_tool_name() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    t.apply(&call("a", "outil", "in_progress", None), now, &fr());
    t.apply(
        &AgentEvent::ToolUpdate {
            turn: 1,
            id: "a".into(),
            title: Some("mcp__oku-slides__widgets".into()),
            status: None,
            input: None,
            output: None,
        },
        now,
        &fr(),
    );
    assert_that!(data(&t)["steps"][0]["calls"][0]["label"].as_str().unwrap()).is_equal_to("widgets");
}

/// Le titre d'un outil vient de l'agent : borné, sans caractères invisibles.
#[test]
fn turns_tool_label_is_bounded_and_filtered() {
    let mut t = Turns::default();
    let now = Instant::now();
    t.apply(&user(1), now, &fr());
    let title = format!("mcp__s__a\u{202E}b{}", "x".repeat(500));
    t.apply(&call("a", &title, "completed", None), now, &fr());
    let d = data(&t);
    let label = d["steps"][0]["calls"][0]["label"].as_str().unwrap_or_default().to_owned();
    assert_that!(label.starts_with("abx")).is_true();
    assert_that!(label.ends_with('…')).is_true();
    assert_that!(label.chars().count()).is_equal_to(oku_slides_session::agent::LABEL_MAX);
    assert_that!(d["steps"][0]["label"].as_str().unwrap_or_default()).is_equal_to(label.as_str());
}
