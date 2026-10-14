# Fine-grained merge of cell edits — design (v2)

Date: 2026-10-08 · Task: t-589 · ADR: 0004 (partly replaces ADR-0002)
Revised after the team review of plan v1 and the user's choices:
- no backward compatibility;
- repair on read-back;
- column ids right away;
- a paste keeps ids by position.

## Goal

Two people modifying a table's or a chart's data at the same time
both keep their modifications. Today, a
`host(chart)` or `host(table)` parameter is a Loro `Atom`: the last
write replaces the whole option.

Targeted cases, which must all merge without loss:
- two different cells, even in the same row or the same column;
- two row insertions, or two column insertions;
- a row deletion on one side, a cell of another row
  modified on the other;
- a column inserted on one side, a cell modified on the other;
- **a row inserted on one side, a column inserted on the other.** The
  new row has no value in the new column, and its values
  are not shifted.

Out of scope:
- character-by-character merge within a cell. A cell is a
  scalar, the last write wins;
- fine-grained merge of the other keys of the option (`series`, colors, flags),
  which stay last writer wins, key by key.

## Decisions

- **Stable row and column ids**, in two parallel arrays
  of the `dataset`:

  ```json
  { "source": [["Pays","Ventes"],["FR",10]], "ids": ["h3k9…","a71c…"], "columns": ["c0f2…","b8e1…"] }
  ```

  - `source` keeps the ECharts format.
  - `ids` has the length of `source`. The header row also has an
    id.
  - `columns` has the width of row 0.
- **The core creates the ids**: 8 hexadecimal characters taken
  from a v4 `uuid`, drawing again on a collision with an
  existing id.
  - Data arriving without ids receives them on the first write
    by the core. This is the case for a chart written by the LLM, a starter
    table or a `set_param`. The mandatory checkpoint is
    `ops/host.rs::host_option`, with `chart_data` and `table_data`.
  - The stored deck **always** carries `ids` and `columns` for each host
    parameter that has a dataset. The read-back replica is therefore identical to the written deck
    (`debug_assert_view_matches_replica`).
- **No backward compatibility.** `DECK_VERSION` goes from 4 to 5, and
  `SOURCES_VERSION` from 6 to 7. An older deck or zip is refused with
  the existing version error. There is no read path for the
  `Atom` form, and the fixtures are migrated.
- **Ops**, with the same JSON as today, plus optional fields:
  - `InsertRow { at, cells, id? }` and `InsertColumn { at, cells, id? }`:
    without `id`, a new id is drawn;
  - `RemoveRow` and `RemoveColumn` remove the id;
  - `Replace { source }` keeps the ids **by position**
    (rows `0..min(ancien, nouveau)`, columns likewise) and draws new ones
    for the excess;
  - the inverses are not exposed: `DataOp::apply` no longer returns them. The
    core and the session did not use them, since history goes through
    the replica. The crate's inverse tests become state tests.
  - `id` is documented in the LLM schema as optional ("do not
    provide it").
- **Repair on read-back** (`replica/host.rs`, reading). After a
  merge, the read-back state is brought back into order:
  - a cell missing from a row is `null`;
  - duplicates of the same row or column are dropped first: a
    column whose id was already seen, a row with the same id and
    the same cells as a kept row (double removal then double
    undo); the next write removes them from the replica;
  - another duplicated or invalid row or column id is
    replaced by a deterministic id derived from the position
    (`r{i}`/`c{j}`, then a numeric suffix if it is already taken);
  - an `align` longer than the columns is truncated. `parse_table` truncates it
    instead of refusing.

  A table exceeding 100 × 20 after a merge stays refused by
  validation: it is shown as a placeholder. Deleting a row or a
  column stays allowed, which is enough to restore it.
- **Robustness to untrusted data**:
  - `DataOp::apply` never panics: lengths checked before any
    access;
  - reading bounds the depth (`MAX_DEPTH`) and the number of rows and
    columns (10,000 × 1,000), with a report;
  - a JSON dataset that does not have the expected shape (rows that are not
    lists, extra keys) is written as `Opaque`, without loss. It does not merge
    finely.
- **Converted charts** (`convert`, `canonicalize`): row and column
  ids are kept if the number of rows, or of columns,
  does not change, and drawn anew otherwise. A switch to scatter, for example,
  groups the rows. `chart_to_table` and `table_to_chart` keep them.

## Replica (`oku-slides-core/src/replica`)

- `Kind::Host` for `ParamType::Host { .. }`. The option is a `LoroMap`, and
  each key other than `dataset` is written as `Nested(Opaque)`.
- `dataset` becomes
  `LoroMap { columns: LoroMovableList<id>, rows: LoroMovableList<LoroMap { id, cells: LoroMap<id de colonne, scalaire> }> }`.
  - Rows are matched by id, with `Writer::sync_list`
    reused as is.
  - Columns are matched by id, in an ordered list.
  - Cells are matched by key: `set` if the value changes,
    `delete` if the column has disappeared.
  - Nothing is written when nothing changes.
- Reading rebuilds `source` in the order of `columns`, then applies the
  repair described above.
- The marker is a `{columns, rows}` map under the `dataset` key of a
  host parameter, and only there.

## Tests

- **Model**:
  - the ops: ids drawn, kept, without collision;
  - `Replace` keeping by position;
  - no panic on inconsistent lengths;
  - id validation (length, duplicate, format, bounds 64/65);
  - the conversions.
- **Core**: `host_option` sets the ids, and the read-back deck is
  identical to the written deck.
- **Replica**:
  - chart and table round trip;
  - a cell edit touches only one entry (`len_ops` + 1, same
    row container);
  - unexpected shape written as `Opaque`;
  - reading forged states: duplicates, container cells, depth,
    size;
  - undo and redo each op.
- **Convergence**:
  - the five cases from the goal;
  - the same deletions on both sides, then double undo, without
    a duplicate after merge;
  - the extended proptest (cells, rows, columns), which requires that after
    merge `parse_table` succeeds and `ids` and `columns` are aligned.

  These tests are written before the replica and must fail on the current
  code.
- **Version**: a v4 deck and a v6 zip are refused.
