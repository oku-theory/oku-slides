# ADR-0015: Data as deck entities, views by reference

- **Status:** accepted
- **Date:** 2026-11-22
- **Tasks:** t-1690
- **Supersedes:** [ADR-0004](0004-donnees-des-hotes-fusionnees-par-cellule.md)
- **Spec:** [data-entities-design.md](../superpowers/specs/2026-11-21-data-entities-design.md)

## Context

ADR-0004 stored each chart's and table's data inline, as a `dataset` of
the host option, merged cell by cell in the replica. The same figures were
copied into every view, edited in several places, and could be neither
derived nor compared. Edits went through positional `set_data` ops
(`DataOp`) whose row 0 held the column names.

## Decision

- **Data are deck entities.** `deck.data` holds tables keyed by data id,
  with stable row and column ids; in the replica each lives under
  `data/<id>`, one table container per generation, merged cell by cell.
- **Views name data.** A chart or table option names a data (`data`) and
  optionally projects its columns (`columns`); it never stores a `dataset`.
  `dataset` survives only as the render input the core resolves from a ref.
- **Edits are data ops.** `add_data`, `edit_data` (by row and column id) and
  the other data ops replace `set_data`; `prepare_ops` lifts inline data an
  agent or a host writes into a data and a ref before the apply.
- **No inline path left.** A plain apply refuses inline data
  (`op-data-inline`); `Deck::from_json` refuses a stored `dataset`
  (`deck-inline-data`), after the version check so an old deck gets the
  version error; the replica read drops one and reports it
  (`issue-data-inline-dropped`).

## Compatibility

None. `DECK_VERSION` moved to 10, the replica format to 9 and
`SOURCES_VERSION` to 15 with `deck.data` (after the knowledge base's 8 and
14); older decks, v12's format 8 included, are refused with the usual version
error: opening one would drop its charts' and tables' inline data.

## Consequences

- The `oku-slides-chart` crate edits no data: `DataOp`, `chart_data`,
  `table_data` and `data_op_schema` are gone.
- The import cuts chart rows to what deck data holds (`MAX_ROWS`,
  `MAX_OVERLAY` cells) instead of the old replica row budget.
- Undo of data edits still goes through the replica history; data ops
  return no inverse.
