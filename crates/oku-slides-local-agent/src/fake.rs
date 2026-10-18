//! Moteur factice scripté des tests (builds de débogage seulement).
//!
//! Le script et le journal sont lus et écrits avec `std::fs` : c'est de l'outillage de
//! test, comme `fake_agent.rs`, jamais compilé dans un build de production.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt};
use tokio_util::sync::CancellationToken;

use crate::engine::{CancelledSnafu, Completion, Engine, EngineError, FailedSnafu, Reply, ScriptExhaustedSnafu, Step};
use crate::error::{LocalAgentError, ScriptFormatSnafu, ScriptSnafu};

#[derive(Debug, Deserialize)]
struct Script {
    turns: Vec<Turn>,
}

#[derive(Debug, Deserialize)]
struct Turn {
    route: String,
    #[serde(default)]
    replies: Vec<ScriptReply>,
    #[serde(default)]
    hang: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ScriptReply {
    Call {
        call: String,
        #[serde(default)]
        args: Value,
    },
    Text {
        text: String,
    },
}

#[derive(Debug, Default)]
struct Cursor {
    /// Tours déjà routés ; le tour courant est `routed - 1`.
    routed: usize,
    /// Réponses `Act` déjà servies pour le tour courant.
    replied: usize,
}

/// Moteur qui rejoue un script JSON, un tour à la fois.
#[derive(Debug)]
pub(crate) struct FakeEngine {
    script: Script,
    log: Option<PathBuf>,
    cursor: Mutex<Cursor>,
}

impl FakeEngine {
    /// Charge le script ; un script illisible ou invalide garde sa cause.
    pub(crate) fn from_file(script: &Path, log: Option<PathBuf>) -> Result<Self, LocalAgentError> {
        let text = std::fs::read_to_string(script).context(ScriptSnafu { path: script })?;
        let script: Script = serde_json::from_str(&text).context(ScriptFormatSnafu { path: script })?;
        Ok(Self { script, log, cursor: Mutex::new(Cursor::default()) })
    }

    fn write_log(&self, c: &Completion) -> Result<(), EngineError> {
        let Some(path) = &self.log else { return Ok(()) };
        let step = match c.step {
            Step::Route => "route",
            Step::Act => "act",
        };
        let tools: Vec<&str> = c.tools.iter().map(|t| t.name.as_str()).collect();
        let line = json!({"step": step, "tools": tools, "messages": c.messages.len()});
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| writeln!(f, "{line}"))
            .map_err(|e| FailedSnafu { message: format!("cannot write the engine log: {e}") }.build().into())
    }

    /// Prochaine réponse du script et indicateur `hang` du tour, sans verrou conservé ensuite.
    fn next(&self, step: Step) -> Result<(Reply, bool), EngineError> {
        let mut cur = self.cursor.lock().unwrap_or_else(PoisonError::into_inner);
        match step {
            Step::Route => {
                let turn = self.script.turns.get(cur.routed).context(ScriptExhaustedSnafu)?;
                cur.routed += 1;
                cur.replied = 0;
                Ok((Reply::Text(turn.route.clone()), turn.hang))
            }
            Step::Act => {
                let turn =
                    cur.routed.checked_sub(1).and_then(|i| self.script.turns.get(i)).context(ScriptExhaustedSnafu)?;
                let reply = turn.replies.get(cur.replied).context(ScriptExhaustedSnafu)?;
                let reply = match reply {
                    ScriptReply::Call { call, args } => Reply::Call { name: call.clone(), args: args.clone() },
                    ScriptReply::Text { text } => Reply::Text(text.clone()),
                };
                cur.replied += 1;
                Ok((reply, turn.hang))
            }
        }
    }
}

#[async_trait]
impl Engine for FakeEngine {
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        self.write_log(c)?;
        let (reply, hang) = self.next(c.step)?;
        if hang {
            cancel.cancelled().await;
            return CancelledSnafu.fail().map_err(Into::into);
        }
        Ok(reply)
    }
}

#[cfg(test)]
#[path = "fake_tests.rs"]
mod fake_tests;
