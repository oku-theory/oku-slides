//! Agent token usage: one record per turn, kept in an add-only ledger.
//!
//! The ledger is the `usage.jsonl` entry of the sources zip; its bytes are untrusted, so reading
//! drops what fails the checks and reports it as an [`Issue`], never failing the deck. Reading is
//! capped (line size, record count, issue count) so a hostile entry costs little.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::validate::Issue;

/// The ledger's zip entry and resume file name.
pub const USAGE_FILE: &str = "usage.jsonl";
/// The most `usage.jsonl` bytes ever read or written, outside the zip budget.
pub const USAGE_MAX_BYTES: u64 = 16 << 20;
const MAX_LINE: usize = 4 << 10;
/// The most turns a ledger holds.
///
/// [`UsageLedger::insert`] also keeps its lines under [`USAGE_MAX_BYTES`] in all, so a full
/// ledger is always read back whole.
pub const USAGE_MAX_RECORDS: usize = 10_000;
const MAX_ISSUES: usize = 20;
const MAX_AGENT: usize = 64;
const MAX_MODEL: usize = 200;
/// The largest cost amount kept: 10 000 turns of it still sum to a finite total.
const MAX_COST: f64 = 1e9;

/// The kind of tab a turn ran in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageTab {
    /// A deck tab.
    Deck,
    /// A pack tab linked to the deck.
    Pack,
    /// A theme tab linked to the deck.
    Theme,
}

/// A turn's token counts, as the agent reported them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTokens {
    /// All token kinds together, as the agent counted them.
    pub total: u64,
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Reasoning tokens; `None`: not reported.
    pub thought: Option<u64>,
    /// Cache read tokens; `None`: not reported.
    pub cache_read: Option<u64>,
    /// Cache write tokens; `None`: not reported.
    pub cache_write: Option<u64>,
}

/// A turn's cost, in the agent's currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageCost {
    /// Amount, finite, from 0 to 1e9.
    pub amount: f64,
    /// ISO 4217 code, three upper-case letters.
    pub currency: String,
}

impl UsageCost {
    /// Whether the amount is finite, 0 to 1e9, the currency three upper-case letters.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.amount.is_finite()
            && (0.0..=MAX_COST).contains(&self.amount)
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase())
    }
}

/// One agent turn: when, where, which agent and model, what it spent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// The turn's identity, made by the app.
    pub turn: Uuid,
    /// End of the turn, UTC.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// The tab the turn ran in.
    pub tab: UsageTab,
    /// The agent preset id.
    pub agent: String,
    /// The session's model; `None`: the agent offers none.
    pub model: Option<String>,
    /// Token counts; `None`: the reply had no usage.
    pub tokens: Option<UsageTokens>,
    /// The turn's cost; `None`: the agent sent none.
    pub cost: Option<UsageCost>,
    /// Fields a newer app wrote, kept as read.
    ///
    /// A rewrite does not drop them.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Whether `c` may appear in a shown agent or model name.
///
/// The name goes to a terminal or Markdown. Refuses controls (Cc), format characters (Cf:
/// bidi marks, overrides and isolates, U+061C, zero-width characters, U+FEFF, tags…) and the
/// line and paragraph separators.
#[must_use]
pub fn shown_char(c: char) -> bool {
    !c.is_control() && !is_format(c) && !matches!(c, '\u{2028}' | '\u{2029}')
}

/// Whether `c` is in Unicode's general category Cf (format), as of Unicode 15.1.
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// `s` without every character [`shown_char`] refuses.
#[must_use]
pub fn shown_text(s: &str) -> String {
    s.chars().filter(|c| shown_char(*c)).collect()
}

/// Whether `s` has at most `max` characters, all of them shown.
fn is_shown(s: &str, max: usize) -> bool {
    s.chars().count() <= max && s.chars().all(shown_char)
}

/// `s` without the characters [`shown_char`] refuses, cut to `max` characters.
fn clean(s: &str, max: usize) -> String {
    shown_text(s).chars().take(max).collect()
}

impl UsageTab {
    /// The tab kind's name, as written in `usage.jsonl`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deck => "deck",
            Self::Pack => "pack",
            Self::Theme => "theme",
        }
    }
}

impl UsageRecord {
    /// A turn with no model, token counts or cost yet, and no extra fields.
    #[must_use]
    pub fn new(turn: Uuid, at: OffsetDateTime, tab: UsageTab, agent: impl Into<String>) -> Self {
        Self { turn, at, tab, agent: agent.into(), model: None, tokens: None, cost: None, extra: Map::new() }
    }

    /// Whether the record passes the ledger's checks.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.at.offset() == UtcOffset::UTC
            && !self.agent.is_empty()
            && is_shown(&self.agent, MAX_AGENT)
            && self.model.as_deref().is_none_or(|m| is_shown(m, MAX_MODEL))
            && self.cost.as_ref().is_none_or(UsageCost::is_valid)
    }

    /// The record brought into the checks' shape; `None` when no agent name is left.
    ///
    /// For records the app makes from agent values: names cleaned and cut, the end of turn put
    /// in UTC, an unusable cost dropped.
    #[must_use]
    pub fn sanitized(self) -> Option<Self> {
        let agent = clean(&self.agent, MAX_AGENT);
        if agent.is_empty() {
            return None;
        }
        let model = self.model.map(|m| clean(&m, MAX_MODEL)).filter(|m| !m.is_empty());
        let cost = self.cost.filter(UsageCost::is_valid);
        Some(Self { at: self.at.to_offset(UtcOffset::UTC), agent, model, cost, ..self })
    }
}

/// What [`UsageLedger::insert`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inserted {
    /// A new turn.
    Added,
    /// The same record was already there.
    Same,
    /// Another record holds this turn.
    ///
    /// The shorter JSON line of the two is kept (byte order on a tie).
    Forged,
    /// The record fails [`UsageRecord::is_valid`] or its line is over 4 KiB: not stored.
    Invalid,
    /// The ledger holds its most turns or bytes: not stored.
    Full,
}

/// The deck's usage records by turn, add-only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageLedger {
    records: BTreeMap<Uuid, UsageRecord>,
    /// Bytes of `to_jsonl`: each record's line plus its newline.
    bytes: u64,
}

/// Issues of a read or a union: at most [`MAX_ISSUES`], then a count; each forged turn once.
#[derive(Default)]
struct Report {
    issues: Vec<Issue>,
    unreported: usize,
    forged: HashSet<Uuid>,
}

impl Report {
    fn push(&mut self, issue: Issue) {
        if self.issues.len() < MAX_ISSUES {
            self.issues.push(issue);
        } else {
            self.unreported += 1;
        }
    }

    fn forged(&mut self, turn: Uuid) {
        if self.forged.insert(turn) {
            self.push(Issue::usage_line_forged(&turn.to_string()));
        }
    }

    fn finish(mut self) -> Vec<Issue> {
        if self.unreported > 0 {
            self.issues.push(Issue::usage_lines_dropped(self.unreported));
        }
        self.issues
    }
}

/// `r` as one JSON line; empty (and logged) if serde ever fails.
fn json_line(r: &UsageRecord) -> Vec<u8> {
    serde_json::to_vec(r).unwrap_or_else(|e| {
        // Plain fields, string keys and JSON values: serde_json does not fail on them.
        tracing::error!(error = %e, turn = %r.turn, "usage record not serializable");
        Vec::new()
    })
}

/// What `line` adds to `usage.jsonl`: its bytes and the newline.
fn line_size(line: &[u8]) -> u64 {
    u64::try_from(line.len()).unwrap_or(u64::MAX).saturating_add(1)
}

impl UsageLedger {
    /// Adds a valid record.
    ///
    /// A turn already held keeps the shorter of the two lines (byte order on a tie).
    pub fn insert(&mut self, record: UsageRecord) -> Inserted {
        let line = json_line(&record);
        if !record.is_valid() || line.is_empty() || line.len() > MAX_LINE {
            return Inserted::Invalid;
        }
        let size = line_size(&line);
        let full = self.records.len() >= USAGE_MAX_RECORDS || self.bytes.saturating_add(size) > USAGE_MAX_BYTES;
        match self.records.entry(record.turn) {
            Entry::Vacant(_) if full => Inserted::Full,
            Entry::Vacant(e) => {
                e.insert(record);
                self.bytes += size;
                Inserted::Added
            }
            Entry::Occupied(e) if *e.get() == record => Inserted::Same,
            Entry::Occupied(mut e) => {
                let held = json_line(e.get());
                if (line.len(), &line) < (held.len(), &held) {
                    // The kept line is the shorter one: the byte count never grows.
                    self.bytes = self.bytes.saturating_sub(line_size(&held)) + size;
                    e.insert(record);
                }
                Inserted::Forged
            }
        }
    }

    /// The records, by end of turn then turn id.
    pub fn records(&self) -> impl Iterator<Item = &UsageRecord> {
        let mut v: Vec<_> = self.records.values().collect();
        v.sort_by_key(|r| (r.at, r.turn));
        v.into_iter()
    }

    /// Number of turns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no turn is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Adds `other`'s turns; each turn held with other content is reported once.
    pub fn union(&mut self, other: UsageLedger) -> Vec<Issue> {
        let mut report = Report::default();
        for record in other.records.into_values() {
            let turn = record.turn;
            match self.insert(record) {
                Inserted::Forged => report.forged(turn),
                Inserted::Full => report.unreported += 1,
                Inserted::Added | Inserted::Same | Inserted::Invalid => {}
            }
        }
        report.finish()
    }

    /// Reads `usage.jsonl`; bad, forged or surplus lines are dropped and reported.
    #[must_use]
    pub fn from_jsonl(bytes: &[u8]) -> (Self, Vec<Issue>) {
        Self::read_lines(bytes, USAGE_MAX_RECORDS)
    }

    /// [`Self::from_jsonl`] keeping at most `max_records` turns.
    fn read_lines(bytes: &[u8], max_records: usize) -> (Self, Vec<Issue>) {
        let (mut ledger, mut report) = (Self::default(), Report::default());
        for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let parsed = (line.len() <= MAX_LINE).then(|| serde_json::from_slice::<UsageRecord>(line).ok()).flatten();
            let Some(record) = parsed else {
                report.push(Issue::usage_line_invalid(i + 1));
                continue;
            };
            if ledger.len() >= max_records && !ledger.records.contains_key(&record.turn) {
                report.unreported += 1;
                continue;
            }
            let turn = record.turn;
            match ledger.insert(record) {
                Inserted::Forged => report.forged(turn),
                Inserted::Invalid => report.push(Issue::usage_line_invalid(i + 1)),
                Inserted::Full => report.unreported += 1,
                Inserted::Added | Inserted::Same => {}
            }
        }
        (ledger, report.finish())
    }

    /// Writes `usage.jsonl`, one record per line, in [`Self::records`] order.
    #[must_use]
    pub fn to_jsonl(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for line in self.records().map(json_line).filter(|l| !l.is_empty()) {
            out.extend(line);
            out.push(b'\n');
        }
        out
    }
}

impl FromIterator<UsageRecord> for UsageLedger {
    fn from_iter<I: IntoIterator<Item = UsageRecord>>(iter: I) -> Self {
        let mut l = Self::default();
        for r in iter {
            l.insert(r);
        }
        l
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
pub(crate) mod usage_tests;
