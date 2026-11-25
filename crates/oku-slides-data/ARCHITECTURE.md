# oku-slides-data

Deck data: the `Data` model (a named manual table), its id-based edits, the dataset types
shared with the chart, core and session crates, and the number rules. Depends only on
`oku-slides-i18n` among oku-slides crates (for localized errors).

## Modules

- `data.rs`: `Data` (name, label, inline, kind, table), `DataId`, `DataKind::Manual`.
- `rows.rs`: `Rows`, row ids in order with hidden order labels: id lookup in O(log² n), O(1) clone;
  `Rows::repaired` makes forged ids unique.
- `table.rs`: `Table` (generation and epoch ids, typed `Column`s, `Rows`, optional
  `BaseRef`, overlay cells in an `imbl::OrdMap<CellKey, DataCell>`), `CellKey` (`row/column`),
  `CellRead`; `cell`, `to_dataset`, `set_column_type`, size counters, `canonicalize` (unique
  column ids, orphan overlay cells dropped; applied on parse). Clones share structure.
- `cell_patch.rs`: `CellPatch`, the overlay cells set and removed between two versions of a data
  (what the host sends the render worker after a cell edit; `None` when anything else changed).
- `table_op.rs`: `TableOp` (set cell, rename, insert/remove row and column, replace) and
  `Table::apply`, all or nothing; ids come from the op, drawn beforehand by `fill_ids`
  away from the row and column ids the batch gives (`GivenIds`).
- `new_table.rs`: `NewTable`/`NewColumn`/`NewRow`, the full-table input; `from_dataset`,
  `fill_ids` (keeps the ids of a previous table by column name and row position), `into_table`, `row_loads` (what each row stores, for import budgets).
- `column_type.rs`: `ColumnType` (number, integer, bool, date, text); `fits`, `infer`, lossless conversion.
- `base_ref.rs`: `BaseId` (`sha256:` content address), `BaseRef`, `BaseColumn` (Parquet base).
- `engine/` (feature `engine`, polars lazy+parquet+sql, render worker only): `write_base`
  (overlay folded, zstd, hashed), `read_view`, `check_column_type`, `check_footer` (bounds read
  before decoding); every call under `catch_unwind`, a panic or mismatch is `data-base-invalid`.
- `test_support.rs` (feature `test-support`): `typed`, `fold`, `based_table` fixtures.
- `dataset.rs`: `Dataset` (2D `source`, row 0 the column names, stable row and column ids) and
  `DataCell` (number, text, bool or empty). Its JSON schema name stays `Cell`.
- `number.rs`: `parse_number` (ASCII sign, one `.` or `,` separator, at most 15 significant
  digits, `1,234` grouping refused) and `format_number`.
- `ids.rs`: `IdGen`, `valid_id` (`[A-Za-z0-9_-]`, 1 to 64 chars), `new_id` (8 hex chars),
  `IdRepair` (deterministic `<id>_<n>` repair of duplicate or invalid ids, linear).
- `name.rs`: `valid_name` (lowercase snake case, at most 64 chars), `default_name`.
- `text.rs`: `strip_controls`, `clean_label` (labels cut to 200 chars).
- `limits.rs`: per-data bounds (rows, columns, text bytes, overlay entries), per-deck bounds
  (`MAX_DATA`, `MAX_DECK_*`), id/name lengths and the batch budget (`MAX_BATCH_CELLS`,
  `REMOVE_COLUMN_WEIGHT`, `BIND_DATA_WEIGHT`).
- `bounded.rs`: serde helpers that stop a list or map at its bound while parsing (not inside a
  tagged `TableOp`, which serde buffers whole: op input is capped by its byte size only).
- `error.rs`: opaque `DataError`, `DataErrorKind`, Fluent keys `data-*` (`data.ftl`).

## Dependencies

`oku-slides-i18n`, `imbl`, `serde`, `serde_json`, `schemars`, `snafu`, `time`, `uuid`, `sha2`,
`hex`; optional `polars` (engine), `tempfile` (test-support).
