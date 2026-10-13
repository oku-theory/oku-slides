# crates/oku-slides-chart — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Enforce the 100 x 20 table bound on ops only; a merged table over it renders as a placeholder. (ADR-0004, plan:2026-10-08-fusion-cellules)
- Keep dataset row ids and column ids unique per list and matching [A-Za-z0-9_-]{1,64}. (plan:2026-10-08-fusion-cellules)
- Keep dataset row 0 as column names; DataOp must never insert or delete at row 0. (spec:2026-10-08-widget-tableau-design)
- Reject a data op that would exceed table limits before cloning anything. (spec:2026-10-08-widget-tableau-design)
- Attach chart series to dataset columns by column id, never by position. (t-653)

## Pitfalls

- Count the header row within MAX_TABLE_ROWS (100); tables allow at most 20 columns and 1000 chars per cell. (plan:2026-10-08-widget-tableau)
- Do not pass an id to insert_row or insert_column; let the core mint it. (plan:2026-10-08-fusion-cellules)
- Never insert, remove or index into dataset ids or columns without checking the length first. (plan:2026-10-08-fusion-cellules)

## Settled decisions

- Do not return inverses from data ops; undo of data edits goes through the replica history. (ADR-0004)
- Keep ids by position on replace or paste and mint only extras; two concurrent pastes may mix cells. (ADR-0004, plan:2026-10-08-fusion-cellules)
- Report table errors through ChartError with chart-table-* keys; do not create a separate TableError. (plan:2026-10-08-widget-tableau)
- Draw chart title, subtitle and legend in oku-slides-chart at theme sizes; charts-rs only leaves room for them. (t-569)
- Mark waterfall totals with the boolean Total column, not empty values; the waterfall always reads dataset. (t-90)
