# crates/oku-slides-data — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Depend on no oku-slides crate but oku-slides-i18n; this crate sits below core and chart. (plan:2026-11-21-data-entities-phase1)
- Bound every list and map of deck input while parsing, through the bounded module; tagged ops are buffered whole by serde, so their only cap is the input byte size. (spec:2026-11-21-data-entities-design)
- Keep every table edit O(log n) in rows and overlay size: look rows up through Rows, never by scanning. (plan:2026-11-21-data-entities-phase1)
- Keep Table::apply all or nothing: check everything before the first change. (plan:2026-11-21-data-entities-phase1)
- Draw a new id only through new_id and check it with valid_id; never build an id by hand. (ADR-0004, plan:2026-11-21-data-entities-phase1)
- Compile polars (feature engine) only into the render worker; the host never decodes a base. (plan:2026-11-21-data-entities-phase1, t-1690)
- Parse SQL in the host (sqlparser, whitelist) and execute it only in the render worker; the host never runs a query. (plan:2026-11-26-data-entities-phase2, t-1780)
