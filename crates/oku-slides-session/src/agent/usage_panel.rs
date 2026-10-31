//! The `show_usage` box: totals as badges, then the rows as a Markdown table.
//!
//! Cells come from [`super::usage_table`] as plain text; [`md_escape`] keeps every one plain in Markdown.

use oku_slides_a2ui::{Component, ComponentKind, Dynamic, TextVariant};
use oku_slides_core::{UsageReport, shown_text};
use oku_slides_i18n::Localizer;
use serde_json::json;

use super::forms::{Place, Surface, surface};
use super::usage_table::{USAGE_COLUMNS, usage_cells, usage_cost, usage_headers, usage_total_cells};

/// Rows the box shows at most; the totals still cover every row.
///
/// A forged deck may hold thousands of groups: the box stays a thread message, never megabytes.
pub(crate) const USAGE_BOX_MAX_ROWS: usize = 100;

/// Builds the usage box of `report`, filed in the turn's thread by the hub.
#[must_use]
pub fn usage_panel(id: &str, report: &UsageReport, l: &Localizer) -> Surface {
    let t = &report.totals;
    let mut badges = vec![
        badge("turns", l.tr_with("usage-badge-turns", &[("count", t.turns.into())])),
        badge("tokens", l.tr_with("usage-badge-tokens", &[("count", t.total.into())])),
    ];
    if t.turns_without_tokens > 0 {
        badges
            .push(badge("no-tokens", l.tr_with("usage-badge-no-tokens", &[("count", t.turns_without_tokens.into())])));
    }
    // A badge is plain text, not Markdown: the cost needs no escaping there.
    for (i, c) in t.costs.iter().enumerate() {
        badges.push(badge(&format!("cost-{i}"), usage_cost(c, l)));
    }
    let ids: Vec<String> = badges.iter().map(|b| b.id.clone()).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let text = if report.rows.is_empty() { l.tr("usage-empty") } else { table(report, l) };
    let mut components = vec![
        Component::column("root", &["title", "badges", "table"]),
        Component::text("title", l.tr("usage-title"), Some(TextVariant::H4)),
        Component::row("badges", &refs),
        Component::new("table", ComponentKind::Markdown { text: Dynamic::Literal(text) }),
    ];
    components.extend(badges);
    surface(id, Place::Thread, None, components, json!({}))
}

fn badge(id: &str, text: String) -> Component {
    Component::new(id, ComponentKind::Badge { text: Dynamic::Literal(text), tone: None })
}

/// The GFM table: headers, delimiter, the first rows, then the totals in bold.
///
/// Past [`USAGE_BOX_MAX_ROWS`], a paragraph after the table counts the rows left out.
fn table(report: &UsageReport, l: &Localizer) -> String {
    let mut out = line(&usage_headers(report.by, l).map(|c| md_escape(&c)));
    out.push('|');
    out.push_str(&"---|".repeat(USAGE_COLUMNS));
    out.push('\n');
    for row in report.rows.iter().take(USAGE_BOX_MAX_ROWS) {
        out.push_str(&line(&usage_cells(row, l).map(|c| md_escape(&c))));
    }
    let mut total = usage_total_cells(&report.totals, l).map(|c| md_escape(&c));
    total[0] = format!("**{}**", total[0]);
    out.push_str(&line(&total));
    let omitted = report.rows.len().saturating_sub(USAGE_BOX_MAX_ROWS);
    if omitted > 0 {
        // A blank line ends the table: the count is a paragraph, not one more row.
        out.push('\n');
        out.push_str(&md_escape(&l.tr_with("usage-rows-omitted", &[("count", omitted.into())])));
        out.push('\n');
    }
    out
}

fn line(cells: &[String; USAGE_COLUMNS]) -> String {
    format!("| {} |\n", cells.join(" | "))
}

/// Backslash-escapes ASCII punctuation and drops what `shown_text` refuses: text, never markup.
fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in shown_text(s).chars() {
        if ch.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
#[path = "usage_panel_tests.rs"]
mod usage_panel_tests;
