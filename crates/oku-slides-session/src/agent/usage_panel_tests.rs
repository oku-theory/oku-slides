use oku_slides_core::{By, UsageReport};
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{USAGE_BOX_MAX_ROWS, md_escape, usage_panel};
use crate::agent::usage_table::usage_table_tests::{record, report};
use crate::deck_packs::deck_packs_tests::{component, text_of};

fn two_turns() -> UsageReport {
    report(
        &[record(1, "2026-10-29", "claude-opus-5-5", true), record(2, "2026-10-30", "claude-opus-5-5", false)],
        By::Day,
    )
}

#[test]
fn usage_panel_french_titles_the_box_and_counts_turns() {
    let s = usage_panel("usage-1", &two_turns(), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "title")).is_equal_to("Usage de l'agent".to_owned());
    assert_that!(text_of(&s, "turns")).is_equal_to("2 tours".to_owned());
}

#[test]
fn usage_panel_tokens_badge_uses_the_plural_form() {
    let one = report(&[record(1, "2026-10-30", "m", false)], By::Day);
    let s = usage_panel("usage-1", &one, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "tokens")).is_equal_to("0 tokens".to_owned());
    assert_that!(text_of(&s, "no-tokens")).is_equal_to("1 turn without token counts".to_owned());
}

#[test]
fn usage_panel_french_table_has_localized_headers_and_a_total_row() {
    let s = usage_panel("usage-1", &two_turns(), &Localizer::new(Some("fr")));
    let table = text_of(&s, "table");
    assert_that!(table.as_str()).starts_with("| Jour | Tours | Entrée | Sortie | Raisonnement |");
    assert_that!(table.as_str()).contains("| 2026\\-10\\-29 | 1 | 1840 | 10 |");
    assert_that!(table.as_str()).contains("| **Total** | 2 |");
}

#[test]
fn usage_panel_cost_cell_is_escaped_like_every_cell() {
    let mut no_cost = record(2, "2026-10-30", "m", true);
    no_cost.cost = None;
    let r = report(&[record(1, "2026-10-30", "m", true), no_cost], By::Day);
    let s = usage_panel("usage-1", &r, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "table").as_str()).contains(r"| 0\.0400 USD \(partial\) |");
}

#[test]
fn usage_panel_row_without_cost_shows_the_placeholder_cell() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.cost = None;
    let s = usage_panel("usage-1", &report(&[r], By::Day), &Localizer::new(Some("en")));
    // Eight columns of numbers or text, then the cost placeholder: no column shifts.
    assert_that!(text_of(&s, "table").as_str()).contains("| 1850 | – |");
}

#[test]
fn usage_panel_empty_report_says_so_and_has_no_table() {
    let s = usage_panel("usage-1", &report(&[], By::Day), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "table")).is_equal_to("Aucun tour d'agent enregistré pour l'instant.".to_owned());
    // No table line and no cost badge: only the two count badges remain.
    assert_that!(text_of(&s, "table").lines().any(|l| l.starts_with('|'))).is_false();
    assert_that!(component(&s, "badges")["children"]).is_equal_to(serde_json::json!(["turns", "tokens"]));
}

#[test]
fn usage_panel_unknown_model_key_is_localized() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.model = None;
    let s = usage_panel("usage-1", &report(&[r], By::Model), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "table").as_str()).contains("| inconnu | 1 |");
}

#[test]
fn md_escape_link_and_pipe_are_plain_text() {
    assert_that!(md_escape("[x](https://evil)|")).is_equal_to(r"\[x\]\(https\:\/\/evil\)\|".to_owned());
}

#[test]
fn md_escape_control_and_bidi_characters_are_dropped() {
    assert_that!(md_escape("a\nb\u{1b}[31mc\u{202E}d")).is_equal_to(r"ab\[31mcd".to_owned());
}

#[test]
fn usage_panel_model_key_with_markup_is_escaped_in_the_table() {
    let r = report(&[record(1, "2026-10-30", "[x](https://evil)|", true)], By::Model);
    let s = usage_panel("usage-1", &r, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "table").as_str()).contains(r"| \[x\]\(https\:\/\/evil\)\| | 1 |");
}

#[test]
fn usage_panel_hostile_model_key_stays_in_its_cell() {
    // Backticks, pipes, emphasis, a bidi override and a line break: one escaped, single-line cell.
    let hostile = "`x`|**y**\u{202E}\nz";
    let r = report(&[record(1, "2026-10-30", hostile, true)], By::Model);
    let table = text_of(&usage_panel("usage-1", &r, &Localizer::new(Some("en"))), "table");
    assert_that!(table.as_str()).contains(r"| \`x\`\|\*\*y\*\*z | 1 |");
    // Header, delimiter, one row, the totals: the key broke no line.
    assert_that!(table.lines().count()).is_equal_to(4);
}

/// A report of `n` turns, one model each: `n` rows by model.
fn models(n: u128) -> UsageReport {
    let records: Vec<_> = (0..n).map(|i| record(i, "2026-10-30", &format!("m{i:04}"), true)).collect();
    report(&records, By::Model)
}

#[test]
fn usage_panel_rows_past_the_cap_are_counted_after_the_table() {
    let r = models(150);
    let table = text_of(&usage_panel("usage-1", &r, &Localizer::new(Some("fr"))), "table");
    let cell = |i: usize| format!("| {} |", md_escape(r.rows[i].key.as_deref().unwrap_or_default()));
    assert_that!(table.contains(&cell(USAGE_BOX_MAX_ROWS - 1))).is_true();
    assert_that!(table.contains(&cell(USAGE_BOX_MAX_ROWS))).is_false();
    // Header, delimiter, the capped rows and the totals, then a blank line and the count.
    assert_that!(table.lines().filter(|l| l.starts_with('|')).count()).is_equal_to(USAGE_BOX_MAX_ROWS + 3);
    assert_that!(table.lines().last()).is_equal_to(Some(r"50 lignes de plus non affichées\."));
}

#[test]
fn usage_panel_rows_past_the_cap_keep_totals_over_every_row() {
    let table = text_of(&usage_panel("usage-1", &models(150), &Localizer::new(Some("en"))), "table");
    assert_that!(table.as_str()).contains("| **Total** | 150 |");
}

#[test]
fn usage_panel_rows_at_the_cap_add_no_count_line() {
    let table = text_of(&usage_panel("usage-1", &models(100), &Localizer::new(Some("en"))), "table");
    assert_that!(table.lines().last().unwrap_or_default()).starts_with("| **Total** | 100 |");
}

#[test]
fn usage_panel_one_row_past_the_cap_uses_the_singular() {
    let table = text_of(&usage_panel("usage-1", &models(101), &Localizer::new(Some("en"))), "table");
    assert_that!(table.lines().last()).is_equal_to(Some(r"1 more row not shown\."));
}
