//! t-811 : le test d'une consigne en file à la fermeture de l'onglet supposait que rien ne
//! s'exécute entre `handle` et `tab_closed`. Sur un runtime `multi_thread` chargé, le fil du
//! test peut être suspendu entre les deux : les outils de l'onglet démarrent, la consigne
//! atteint l'agent (légitimement : l'onglet était encore ouvert) et le test échouait.
//! Sur un runtime `current_thread`, aucune tâche lancée par `handle` (`TabEntry::start`) ne
//! tourne avant le premier `await` : la fermeture précède toujours le démarrage de l'onglet.
//! Suppose que la pile tourne sur le runtime du test (`Handle::current()` dans `build_stack`).
//! Le test fige à dessein cet ordre-là ; dans l'app, les deux appels arrivent séparément et
//! l'onglet peut démarrer entre eux. La fermeture pendant un tour est couverte par
//! `tab_closed_during_a_turn_releases_its_questions_and_other_tabs_go_on` (tests/stack.rs).

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{cmd, logged, stack_with_sink_and_log, wait_turn_done};

// Surtout pas `multi_thread` : la course reviendrait (voir plus haut).
#[tokio::test(flavor = "current_thread")]
async fn issue_0811_a_prompt_queued_before_tab_closed_never_reaches_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    // Fermé avant que ses outils n'aient démarré : la consigne est encore en file.
    // Aucun `await` entre les deux appels : rien de l'onglet ne tourne avant sa fermeture.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "perdue" }))).unwrap();
    stack.tab_closed("t-1");
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "gardée" }))).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_that!(logged(&log, "session/new")).has_length(1);
    let prompts = logged(&log, "session/prompt");
    assert_that!(prompts).has_length(1);
    assert_that!(prompts[0].to_string()).contains("gardée");
    let from_closed = rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1").count();
    assert_that!(from_closed).is_equal_to(0);
}
