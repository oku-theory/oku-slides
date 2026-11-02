//! Recent warnings and errors of the app log, kept in memory for diagnosis.
//!
//! A `tracing` layer ([`RingLayer`]) copies each event it sees into a bounded ring
//! ([`LogRing`]); `telemetry::init` installs it under the app's filter with a `WARN`
//! floor of its own. Each line is redacted (registered secrets masked, home dir shown as `~`)
//! and cut when captured; an event's `tab` field is kept apart so diagnosis can show a tab its own lines.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::{LazyLock, Mutex, PoisonError};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

use crate::log_secrets::redact_secrets;

/// Lines the ring keeps at most; the oldest go first.
pub const LOG_RING_CAP: usize = 50;

/// Characters a captured line keeps at most, its ellipsis included.
pub const LOG_LINE_MAX: usize = 500;

/// The event field naming the tab a line concerns (`tab = %id`).
const TAB_FIELD: &str = "tab";

/// The user's home directory, read once; `None` when unknown.
static HOME: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::home_dir().and_then(|p| p.to_str().map(str::to_owned)));

/// The ring of the whole app, filled by the layer `telemetry::init` installs.
pub(crate) static APP_RING: LogRing = LogRing::new();

/// One warning or error of the log, as diagnosis shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogLine {
    /// `WARN` or `ERROR`.
    pub level: String,
    /// When the event happened, RFC 3339 in UTC.
    pub at: String,
    /// The event's target (its module, unless set).
    pub target: String,
    /// The event's message, then its other fields as `key=value`.
    pub message: String,
    /// The event's `tab` field, if any: the tab it concerns; not serialized.
    #[serde(skip)]
    pub tab: Option<String>,
}

/// A bounded ring of log lines, safe to share between threads.
#[derive(Debug, Default)]
pub struct LogRing {
    lines: Mutex<VecDeque<LogLine>>,
}

impl LogRing {
    /// An empty ring.
    #[must_use]
    pub const fn new() -> Self {
        Self { lines: Mutex::new(VecDeque::new()) }
    }

    /// Adds `line`, dropping the oldest past [`LOG_RING_CAP`].
    pub fn push(&self, line: LogLine) {
        let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        if lines.len() == LOG_RING_CAP {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    /// The lines kept, oldest first.
    #[must_use]
    pub fn lines(&self) -> Vec<LogLine> {
        self.lines.lock().unwrap_or_else(PoisonError::into_inner).iter().cloned().collect()
    }
}

/// The user's home directory as a string, read once; `None` when unknown.
#[must_use]
pub fn home() -> Option<&'static str> {
    HOME.as_deref()
}

/// `text` with every occurrence of `home` replaced by `~`.
///
/// Only whole path components match: `/Users/ann` leaves `/Users/anna` alone. An empty
/// home or the root (`/`) redacts nothing: it would hide every path.
#[must_use]
pub fn redact_home(text: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(|h| h.trim_end_matches(['/', '\\'])) else { return text.to_owned() };
    if home.is_empty() || Path::new(home).parent().is_none() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let after = &rest[at + home.len()..];
        let whole = after.chars().next().is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.')));
        out.push_str(&rest[..at]);
        out.push_str(if whole { "~" } else { home });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// `text` cut to at most `max` characters, on a char boundary, ending with `…` when cut.
#[must_use]
pub fn clip_line(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A line of text as diagnosis captures it: registered secrets masked, home redacted, then cut
/// to [`LOG_LINE_MAX`].
#[must_use]
pub fn captured(text: &str, home: Option<&str>) -> String {
    clip_line(&redact_home(&redact_secrets(text), home), LOG_LINE_MAX)
}

/// The `tracing` layer that copies each event it sees into a ring.
///
/// Its own level is not checked here: give it a `LevelFilter` with `with_filter`.
#[derive(Debug)]
pub struct RingLayer {
    ring: &'static LogRing,
    home: Option<String>,
}

impl RingLayer {
    /// A layer filling `ring`, redacting `home`.
    #[must_use]
    pub fn new(ring: &'static LogRing, home: Option<String>) -> Self {
        Self { ring, home }
    }
}

impl<S: Subscriber> Layer<S> for RingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut fields = Fields::default();
        event.record(&mut fields);
        let at = OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default();
        self.ring.push(LogLine {
            level: level_name(*meta.level()).to_owned(),
            at,
            target: meta.target().to_owned(),
            message: captured(&fields.text(), self.home.as_deref()),
            tab: fields.tab,
        });
    }
}

/// The name of `level`, as the reply shows it.
fn level_name(level: Level) -> &'static str {
    match level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN",
        Level::INFO => "INFO",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE",
    }
}

/// An event's `message` field and its other fields, in order.
#[derive(Debug, Default)]
struct Fields {
    message: String,
    others: String,
    /// The `tab` field's value, also kept in `others`.
    tab: Option<String>,
}

impl Fields {
    /// The message, then ` key=value` for each other field.
    fn text(&self) -> String {
        let mut out = self.message.clone();
        if !self.others.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(self.others.trim_start());
        }
        out
    }
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            value.clone_into(&mut self.message);
        } else {
            if field.name() == TAB_FIELD {
                self.tab = Some(value.to_owned());
            }
            let _ = write!(self.others, " {}={value}", field.name());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            let shown = format!("{value:?}");
            if field.name() == TAB_FIELD {
                self.tab = Some(shown.clone());
            }
            let _ = write!(self.others, " {}={shown}", field.name());
        }
    }
}

#[cfg(test)]
#[path = "log_ring_tests.rs"]
mod log_ring_tests;
