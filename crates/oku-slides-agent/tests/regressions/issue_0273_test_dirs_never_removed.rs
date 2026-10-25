//! t-273 : les piles de test (`start`, `stack_with_sink_and_log`) créaient
//! `$TMPDIR/oku-slides-agent-*` et `$TMPDIR/oku-slides-stack-*` (scénario,
//! journal du faux agent, données des sessions) sans jamais les supprimer :
//! des milliers de dossiers s'accumulaient d'un lancement à l'autre.

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{stack_with_sink_and_log, start};

#[tokio::test(flavor = "multi_thread")]
async fn harness_removes_its_dir_when_dropped() {
    let h = start(json!({ "turns": [] })).await;
    let dir = h.log.parent().map_or_else(|| panic!("journal sans dossier"), ToOwned::to_owned);
    asserting("le dossier de test vit autant que la pile").that(&dir.is_dir()).is_true();
    drop(h);
    asserting("puis il est supprimé avec elle").that(&dir.exists()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn stack_removes_its_dir_when_dropped() {
    let (stack, _, log) = stack_with_sink_and_log(json!({ "turns": [] }), &["t-1"]).await;
    let dir = log.parent().map_or_else(|| panic!("journal sans dossier"), ToOwned::to_owned);
    asserting("le dossier de test vit autant que la pile").that(&dir.is_dir()).is_true();
    drop(stack);
    asserting("puis il est supprimé avec elle").that(&dir.exists()).is_false();
}
