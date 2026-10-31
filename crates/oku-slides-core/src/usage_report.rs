//! Usage reports: a ledger's turns summed by day, month, model, agent, tab or deck.
//!
//! [`aggregate`] is the only usage sum: the CLI and the agent tools both call it. Days and months
//! are UTC; costs are never summed across currencies, and a record's amount is at most 1e9, so
//! a sum stays finite.

use std::collections::BTreeMap;

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use snafu::{Backtrace, Snafu};
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use crate::usage::UsageRecord;

/// What a report's rows group by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum By {
    /// UTC day, `YYYY-MM-DD`.
    Day,
    /// UTC month, `YYYY-MM`.
    Month,
    /// The session's model.
    Model,
    /// The agent preset.
    Agent,
    /// The tab kind.
    Tab,
    /// The deck, by its label.
    Deck,
}

/// A usage date range that cannot be used.
#[derive(Debug, Snafu)]
pub struct UsageRangeError(RangeInner);

/// What is wrong with a [`UsageRangeError`]'s range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UsageRangeErrorKind {
    /// A bound is not a `YYYY-MM-DD` date.
    BadDate,
    /// `since` is after `until`.
    Reversed,
}

#[derive(Debug, Snafu)]
enum RangeInner {
    #[snafu(display("invalid date {text}, expected YYYY-MM-DD"))]
    BadDate { text: String, backtrace: Backtrace },
    #[snafu(display("since {since} is after until {until}"))]
    Reversed { since: Date, until: Date, backtrace: Backtrace },
}

impl UsageRangeError {
    /// What is wrong with the range.
    #[must_use]
    pub fn kind(&self) -> UsageRangeErrorKind {
        match self.0 {
            RangeInner::BadDate { .. } => UsageRangeErrorKind::BadDate,
            RangeInner::Reversed { .. } => UsageRangeErrorKind::Reversed,
        }
    }
}

impl Localize for UsageRangeError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            RangeInner::BadDate { text, .. } => l.tr_with("usage-range-bad-date", &[("date", text.as_str().into())]),
            RangeInner::Reversed { since, until, .. } => l.tr_with(
                "usage-range-reversed",
                &[("since", since.to_string().into()), ("until", until.to_string().into())],
            ),
        }
    }
}

/// Reads a `YYYY-MM-DD` UTC day.
///
/// # Errors
///
/// [`UsageRangeErrorKind::BadDate`]: not such a date (the text is kept, cut to 32 characters).
pub fn parse_day(text: &str) -> Result<Date, UsageRangeError> {
    Date::parse(text, format_description!("[year]-[month]-[day]"))
        .map_err(|_| BadDateSnafu { text: text.chars().take(32).collect::<String>() }.build().into())
}

/// Inclusive UTC dates a report keeps; open ends allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    since: Option<Date>,
    until: Option<Date>,
}

impl Range {
    /// Every turn.
    pub const ALL: Self = Self { since: None, until: None };

    /// A range from optional bounds.
    ///
    /// # Errors
    ///
    /// [`UsageRangeErrorKind::Reversed`]: `since` after `until`.
    pub fn new(since: Option<Date>, until: Option<Date>) -> Result<Self, UsageRangeError> {
        match (since, until) {
            (Some(since), Some(until)) if since > until => Err(ReversedSnafu { since, until }.build().into()),
            _ => Ok(Self { since, until }),
        }
    }

    /// A range from optional `YYYY-MM-DD` bounds, as the CLI and the tools take them.
    ///
    /// # Errors
    ///
    /// A bound that is not a date, or `since` after `until`.
    pub fn parse(since: Option<&str>, until: Option<&str>) -> Result<Self, UsageRangeError> {
        Self::new(since.map(parse_day).transpose()?, until.map(parse_day).transpose()?)
    }

    /// Whether the UTC day of `at` is in the range.
    #[must_use]
    pub fn holds(self, at: OffsetDateTime) -> bool {
        let day = at.date();
        self.since.is_none_or(|s| day >= s) && self.until.is_none_or(|u| day <= u)
    }
}

/// A currency's summed cost; `partial` when some turns had none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostTotal {
    /// ISO 4217 code.
    pub currency: String,
    /// Sum of the turns' amounts.
    pub amount: f64,
    /// Some turns of the row reported no cost.
    pub partial: bool,
}

/// One report row: the sums of the turns sharing its key.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UsageRow {
    /// The group's key; `None`: unknown (no model reported).
    pub key: Option<String>,
    /// Turns in the row.
    pub turns: u64,
    /// Turns whose reply had no token counts.
    pub turns_without_tokens: u64,
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Reasoning tokens.
    pub thought: u64,
    /// Cache read tokens.
    pub cache_read: u64,
    /// Cache write tokens.
    pub cache_write: u64,
    /// All tokens, as the agents counted them.
    pub total: u64,
    /// Costs by currency, in currency order.
    pub costs: Vec<CostTotal>,
    /// Turns that reported a cost, to mark costs partial.
    #[serde(skip)]
    with_cost: u64,
}

impl UsageRow {
    fn add(&mut self, r: &UsageRecord) {
        self.turns += 1;
        match r.tokens {
            Some(t) => {
                self.input = self.input.saturating_add(t.input);
                self.output = self.output.saturating_add(t.output);
                self.thought = self.thought.saturating_add(t.thought.unwrap_or(0));
                self.cache_read = self.cache_read.saturating_add(t.cache_read.unwrap_or(0));
                self.cache_write = self.cache_write.saturating_add(t.cache_write.unwrap_or(0));
                self.total = self.total.saturating_add(t.total);
            }
            None => self.turns_without_tokens += 1,
        }
        if let Some(c) = &r.cost {
            self.with_cost += 1;
            match self.costs.binary_search_by(|t| t.currency.as_str().cmp(&c.currency)) {
                Ok(i) => self.costs[i].amount += c.amount,
                Err(i) => {
                    self.costs.insert(i, CostTotal { currency: c.currency.clone(), amount: c.amount, partial: false });
                }
            }
        }
    }

    fn close(mut self) -> Self {
        let partial = self.with_cost < self.turns;
        for c in &mut self.costs {
            c.partial = partial;
        }
        self
    }
}

/// A report: rows in key order, then their totals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageReport {
    /// What the rows group by.
    pub by: By,
    /// The rows, unknown key first.
    pub rows: Vec<UsageRow>,
    /// All rows together.
    pub totals: UsageRow,
}

/// Sums `records` (deck label, record) by `by`, keeping those in `range`.
#[must_use]
pub fn aggregate<'a>(
    records: impl IntoIterator<Item = (&'a str, &'a UsageRecord)>,
    by: By,
    range: Range,
) -> UsageReport {
    let mut rows: BTreeMap<Option<String>, UsageRow> = BTreeMap::new();
    let mut totals = UsageRow::default();
    for (deck, r) in records {
        if !range.holds(r.at) {
            continue;
        }
        let day = r.at.date();
        let key = match by {
            By::Day => Some(format!("{:04}-{:02}-{:02}", day.year(), u8::from(day.month()), day.day())),
            By::Month => Some(format!("{:04}-{:02}", day.year(), u8::from(day.month()))),
            By::Model => r.model.clone(),
            By::Agent => Some(r.agent.clone()),
            By::Tab => Some(r.tab.as_str().to_owned()),
            By::Deck => Some(deck.to_owned()),
        };
        rows.entry(key.clone()).or_insert_with(|| UsageRow { key, ..UsageRow::default() }).add(r);
        totals.add(r);
    }
    UsageReport { by, rows: rows.into_values().map(UsageRow::close).collect(), totals: totals.close() }
}

#[cfg(test)]
#[path = "usage_report_tests.rs"]
mod usage_report_tests;
