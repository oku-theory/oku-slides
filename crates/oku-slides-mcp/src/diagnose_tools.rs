//! The read-only `diagnose` tool: the agent, the dictation, recent log problems, the tab's errors.
//!
//! Text is redacted where it is captured (log ring, stderr tail) and once more here, on the
//! whole reply: registered secrets masked, the home dir shown as `~`, control characters and
//! terminal escapes removed. The log keeps the calling tab's lines and those of no tab. The
//! caps hold here too, whatever the port gives.

use oku_slides_platform::telemetry::{
    LOG_LINE_MAX, LOG_RING_CAP, LogLine, clip_line, home, recent_problems, redact_home, redact_secrets,
};
use rust_mcp_sdk::schema::CallToolResult;
use serde::Serialize;
use serde_json::Value;
use snafu::ResultExt;

use crate::diagnosis::{AgentDiagnosis, Diagnosis, DictationDiagnosis};
use crate::tool_error::{ResponseSnafu, SessionSnafu, TaskSnafu, ToolError};
use crate::tools::Tools;

/// Lines of the agent's stderr the reply keeps at most, the last ones.
pub(crate) const STDERR_MAX_LINES: usize = 40;

/// Log levels the reply keeps: `WARN` and up, never `INFO`, `DEBUG` or `TRACE`.
const KEPT_LEVELS: [&str; 2] = ["WARN", "ERROR"];

/// The `diagnose` reply; its field names are fixed (plan: lot 2).
#[derive(Debug, Serialize)]
pub(crate) struct DiagnoseReply {
    /// The selected app agent.
    agent: AgentDiagnosis,
    /// The dictation, or `null` without one.
    dictation: Option<DictationDiagnosis>,
    /// The app's last warnings and errors, oldest first.
    log: Vec<LogLine>,
    /// The tab's last render error, on one line.
    render_error: Option<String>,
    /// The tab's last import error, on one line.
    import_error: Option<String>,
}

/// The last `max` of `items`, in order.
fn last<T>(mut items: Vec<T>, max: usize) -> Vec<T> {
    let extra = items.len().saturating_sub(max);
    items.drain(..extra);
    items
}

/// The reply from its parts for tab `tab`, capped and clipped; [`redacted`] still has to run on it.
///
/// The log keeps only `tab`'s lines and the lines of no tab: a tab never reads another's.
pub(crate) fn reply(
    host: Diagnosis,
    log: Vec<LogLine>,
    errors: (Option<String>, Option<String>),
    tab: &str,
) -> DiagnoseReply {
    let Diagnosis { mut agent, dictation } = host;
    // Masked before the cut: a cut never leaves half a secret behind.
    let clip = |s: &str| clip_line(&redact_secrets(s), LOG_LINE_MAX);
    agent.stderr = last(agent.stderr, STDERR_MAX_LINES).iter().map(|l| clip(l)).collect();
    let log = log
        .into_iter()
        .filter(|l| KEPT_LEVELS.contains(&l.level.as_str()) && l.tab.as_deref().is_none_or(|t| t == tab))
        .collect();
    let log = last(log, LOG_RING_CAP).into_iter().map(|l| LogLine { message: clip(&l.message), ..l }).collect();
    let (render, import) = errors;
    DiagnoseReply {
        agent,
        dictation,
        log,
        render_error: render.as_deref().map(clip),
        import_error: import.as_deref().map(clip),
    }
}

/// `text` without terminal escapes nor control characters; tabs and line breaks become spaces.
///
/// Escapes (`ESC [ … final`, `ESC ] … BEL` or `ESC \\`, `ESC` + one char) go whole, so no
/// leftover can restyle a terminal or hide text from whoever reads the reply.
pub(crate) fn printable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.next() {
                Some('[') => while chars.next().is_some_and(|c| !('\u{40}'..='\u{7e}').contains(&c)) {},
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\t' | '\n' | '\r' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// `value` with, in every string, registered secrets masked, the home dir shown as `~`, and
/// only printable characters ([`printable`]).
pub(crate) fn redacted(value: Value, home: Option<&str>) -> Value {
    match value {
        Value::String(s) => Value::String(printable(&redact_home(&redact_secrets(&s), home))),
        Value::Array(items) => Value::Array(items.into_iter().map(|v| redacted(v, home)).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, redacted(v, home))).collect()),
        other => other,
    }
}

impl Tools {
    /// The troubleshooting state, read off the async thread: the port and the session block.
    pub(crate) async fn diagnose(&self) -> CallToolResult {
        let (voice, session) = (self.hub.voice().clone(), self.session.clone());
        let read = tokio::task::spawn_blocking(move || -> Result<Value, ToolError> {
            let errors = session.last_errors().context(SessionSnafu)?;
            let reply = reply(voice.diagnosis(), recent_problems(), errors, session.tab());
            let value = serde_json::to_value(reply).context(ResponseSnafu)?;
            Ok(redacted(value, home()))
        })
        .await
        .context(TaskSnafu)
        .and_then(|r| r);
        self.reply(read, |v| self.json_result(&v))
    }
}

#[cfg(test)]
#[path = "diagnose_tools_tests.rs"]
mod diagnose_tools_tests;
