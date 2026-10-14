# ADR-0004: Host data merged cell by cell

- **Status:** accepted
- **Date:** 2026-10-08
- **Tasks:** t-589
- **Partially supersedes:** [ADR-0002](0002-sources-du-deck-en-crdt-loro.md)
  (the host option as a JSON atom)
- **Spec:** [fusion-cellules-design.md](../superpowers/specs/2026-10-08-fusion-cellules-design.md)

## Context

The option of a `host(chart)` or `host(table)` parameter is a Loro `Atom`,
as ADR-0002 decided: the last write replaces the whole option.
Two people who edit different cells of the same table or chart at the same
time therefore lose one of the two edits, without any
warning. The same goes for two row insertions, two column
insertions, or a row deletion against a cell edit.

## Decision

- **Stable row and column identifiers**, in two parallel arrays
  of the `dataset`: `ids` (one entry per `source` row, header
  included) and `columns` (one entry per column). The core creates them: 8
  hexadecimal characters drawn from a v4 `uuid`, drawn again on
  collision. Accepted format: `[A-Za-z0-9_-]{1,64}`, unique per list.
- **`dataset` as containers.** In the replica, the `dataset` becomes a
  `LoroMap` `{ columns, rows }`: `columns` is an ordered list
  of identifiers, `rows` an ordered list of rows `{ id, cells }`, and
  `cells` a map of scalars by column identifier. Rows, columns
  and cells are matched by identifier, so concurrent edits
  of different cells, rows and columns are merged.
- **Repair on read-back.** After a merge, the read-back state is put back
  in order: duplicate column, and duplicate row with the same cells, discarded
  (no duplicate after a double undo); missing cell read as
  `null`; any other duplicate or invalid identifier
  replaced by a deterministic identifier derived from the position, `align`
  truncated to the width.
- **The other keys of the option** (`series`, colors, flags) stay
  last-writer-wins, key by key, each as an atom.

**Global invariant.** The core keeps `ids` and `columns` aligned with
`source` on every write. Read-back repairs a merged state, and
validation (100 rows × 20 columns) applies to ops. A table that
exceeds these bounds after a merge is rendered as a placeholder until
rows or columns are deleted.

## Compatibility

None. `DECK_VERSION` goes from 4 to 5 and `SOURCES_VERSION` from 6 to 7. An
older version of the app rejects the new format with its usual version
error, and the new one reads no older format: there is no read
path for the `Atom` shape. The fixtures are migrated.

## Consequences

- Data ops no longer return an inverse: history goes through the
  replica.
- A `dataset` of an unexpected shape is written as `Opaque`, losslessly, and does
  not merge at fine grain.

## Limits

- A cell is a last-writer-wins scalar: no character-by-character
  merge inside a cell.
- A paste keeps identifiers by position (rows and columns
  `0..min(old, new)`) and draws fresh ones for the excess.
- Two concurrent pastes (`Replace`) merge cell by cell
  (last writer wins per cell): since identifiers are kept by
  position, the result may mix the two pastes.
