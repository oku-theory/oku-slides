//! Usage tools: the linked deck's agent turns, as JSON or a thread box.

use oku_slides_core::{By, Range, UsageReport, aggregate};
use oku_slides_session::agent::usage_panel;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt};

use crate::tool_error::{SessionSnafu, TaskSnafu, ToolError, UsageBadRangeSnafu, UsageNoDeckSnafu};
use crate::tools::Tools;

/// Rows the `usage` JSON keeps at most; the totals still cover every row.
pub(crate) const USAGE_JSON_MAX_ROWS: usize = 1000;

/// The `usage` reply: the report, its rows capped, and how many were dropped.
#[derive(Debug, Serialize)]
pub(crate) struct CappedReport {
    /// The report, with at most [`USAGE_JSON_MAX_ROWS`] rows.
    #[serde(flatten)]
    report: UsageReport,
    /// Rows dropped past the cap; absent when none was.
    #[serde(skip_serializing_if = "Option::is_none")]
    rows_omitted: Option<usize>,
}

impl CappedReport {
    /// Keeps the first [`USAGE_JSON_MAX_ROWS`] rows of `report`, in report order.
    pub(crate) fn new(mut report: UsageReport) -> Self {
        let omitted = report.rows.len().saturating_sub(USAGE_JSON_MAX_ROWS);
        report.rows.truncate(USAGE_JSON_MAX_ROWS);
        Self { report, rows_omitted: (omitted > 0).then_some(omitted) }
    }
}

/// Row grouping offered to the agent: one deck, so no `deck`.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UsageBy {
    /// One row per UTC day.
    #[default]
    Day,
    /// One row per UTC month.
    Month,
    /// One row per model.
    Model,
    /// One row per agent preset.
    Agent,
    /// One row per tab kind: deck, pack or theme.
    Tab,
}

impl From<UsageBy> for By {
    fn from(b: UsageBy) -> Self {
        match b {
            UsageBy::Day => Self::Day,
            UsageBy::Month => Self::Month,
            UsageBy::Model => Self::Model,
            UsageBy::Agent => Self::Agent,
            UsageBy::Tab => Self::Tab,
        }
    }
}

/// The report asked by `usage` and `show_usage`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UsageArgs {
    /// Row grouping: day (default), month, model, agent or tab.
    #[serde(default)]
    pub by: UsageBy,
    /// First UTC day kept, as YYYY-MM-DD.
    #[serde(default)]
    pub since: Option<String>,
    /// Last UTC day kept, as YYYY-MM-DD.
    #[serde(default)]
    pub until: Option<String>,
}

impl Tools {
    /// The linked deck's report, as JSON.
    pub(crate) async fn usage(&self, args: Value) -> CallToolResult {
        let args: UsageArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let report = self.usage_report(args).await;
        self.reply(report, |report| self.json_result(&CappedReport::new(report)))
    }

    /// The box goes in the call's turn, like `render`: refused outside a turn.
    pub(crate) async fn show_usage(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let args: UsageArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.no_turn("show_usage") };
        let report = self.usage_report(args).await;
        self.reply(report, |report| {
            let id = self.hub.next_id("usage");
            let surface = usage_panel(&id, &report, &self.localizer.current());
            if self.hub.show_static(turn, surface) {
                self.json_result(&json!({ "shown": id }))
            } else {
                self.no_turn("show_usage")
            }
        })
    }

    /// The linked deck's report, read off the async thread: the ledger read blocks.
    async fn usage_report(&self, args: UsageArgs) -> Result<UsageReport, ToolError> {
        let range = Range::parse(args.since.as_deref(), args.until.as_deref()).context(UsageBadRangeSnafu)?;
        let (session, by) = (self.session.clone(), By::from(args.by));
        tokio::task::spawn_blocking(move || {
            session.usage_ledger().map(|l| l.map(|ledger| aggregate(ledger.records().map(|r| ("", r)), by, range)))
        })
        .await
        .context(TaskSnafu)?
        .context(SessionSnafu)?
        .context(UsageNoDeckSnafu)
    }
}

#[cfg(test)]
#[path = "usage_tools_tests.rs"]
mod usage_tools_tests;
