# crates/oku-slides-chart — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Cap a table view at MAX_TABLE_ROWS × MAX_TABLE_COLUMNS when core plan_views plans it, reporting issue-view-truncated; deck data bounds live in oku_slides_data::limits. (ADR-0015)
- Keep dataset row ids and column ids unique per list and matching [A-Za-z0-9_-]{1,64}. (plan:2026-10-08-fusion-cellules)
- Keep dataset row 0 as column names. (spec:2026-10-08-widget-tableau-design)
- Treat `dataset` as render input only: views name deck data, and this crate edits no data. (ADR-0015, spec:2026-11-21-data-entities-design)
- Attach chart series to dataset columns by column id, never by position. (t-653)

## Pitfalls

- Count the header row within MAX_TABLE_ROWS (100); tables allow at most 20 columns and 1000 chars per cell. (plan:2026-10-08-widget-tableau)
- Never index into dataset ids or columns without checking the length first. (plan:2026-10-08-fusion-cellules)
- Keep bar_gap's MARGIN and SLOT_GAP equal to charts-rs's bar margin and gap when bumping charts-rs; its render tests are the oracle. (t-1384)

## Settled decisions

- Report table errors through ChartError with chart-table-* keys; do not create a separate TableError. (plan:2026-10-08-widget-tableau)
- Draw chart title, subtitle and legend in oku-slides-chart at theme sizes; charts-rs only leaves room for them. (t-569)
- Mark waterfall totals with the boolean Total column, not empty values; the waterfall always reads dataset. (t-90)
