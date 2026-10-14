# Table widget — design

Date: 2026-10-08 · Task: t-578 · Revised after the team review of the plan.

## Goal

A new element widget **`table`** displays a data table in a
slide. It reuses chart data: the same `Dataset`
(`source: Vec<Vec<Cell>>`), the same editing operations (insert or
delete a row or a column, set a cell, replace everything) and the
same editing grid in the UI. A chart can be shown as a
table and vice versa, keeping its data.

V1 options: header row, first column as header, striped
rows, per-column alignment.

Out of scope, to be handled in backlog tasks:
- merged cells, rich text in cells, sorting, paginating a
  table over several slides;
- locale-aware number formatting (t-590);
- fine-grained merging of concurrent edits (t-589);
- chart → table → chart round trip without losing the type (t-593).

## Decisions

- **Native Typst rendering** (`table(...)`), not SVG. The text stays sharp and
  selectable in the PDF, and it follows the theme tokens.
- **New host type `host("table")`** (`HostType::Table`). The Typst
  widget declares `params: (data: p.host("table"))`.
- **The data stays in `oku-slides-chart`**, which gains a `table` module
  and now handles "charts and tables". `Dataset`, `Cell` and
  the data op are shared as is (G11). The crate keeps its name, but
  its description and its new `ARCHITECTURE.md` say so.
- **Generic data op.** `ChartDataOp` becomes **`DataOp`** and
  `Op::SetChartData` becomes **`Op::SetData`**. This is a breaking change to the
  op format, which the MVP allows. The counterpart of `SetChartOption` is
  **`Op::SetTableOption`**, a merge patch that rejects `dataset`.
- **Conversion on the Rust side.** The session request **`show_as`** (`view: "table"`
  or `"chart"`) is exposed to the UI and to the agent (MCP tool `show_as`),
  modeled on `chart_kind`. The crate provides
  `chart_to_table`/`table_to_chart`:
  - rows that are too short are padded;
  - limits are checked;
  - a chart gets one series per value column.

  The replacement is done by a single `ReplaceElement`, which gives a single
  undo step, and keeps the id, the frame, the slot and the style.
- **Overflow: scaling.** The table is measured at the available
  width: cells wrap. It is shrunk (`scale`) only
  if it still overflows, mostly in height. There is no pagination.
- **Render guard.** An open deck is not rejected: `validate` only
  reports. Before Typst, the render therefore passes each `host(table)` value through
  `parse_table`:
  - if it is valid, Typst receives the canonical form;
  - otherwise, it receives `null`, the widget shows its placeholder and a `warn` is
    logged.

  The Typst widget also checks types, as defense in depth.
- **Detection by `kind` in the UI**, as for the chart. The
  dedicated panel opens for `kind: "table"` and reads the `data` param. A pack
  widget that declares `host("table")` under another name has no dedicated
  panel. This is a deliberate departure from the detection-by-host rule.

## Model (`oku-slides-chart::table`)

```rust
pub struct TableOption {
    pub dataset: Dataset,
    #[serde(default = "default_true")] pub header_row: bool,  // true
    #[serde(default)]                  pub header_column: bool,  // false
    #[serde(default)]                  pub striped: bool,        // false
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<ColumnAlign>,
}
pub enum ColumnAlign { Auto, Start, Center, End }   // serialized in lowercase
```

- The JSON is kebab-case, with `deny_unknown_fields`: `header-row`,
  `header-column`, `striped`, `align`, `dataset`.
- `align[i]` applies to column `i`. A column with no entry is `auto`.
  `auto` aligns to the end a column whose non-header cells are all
  numbers or empty, with at least one number. Otherwise, alignment is at the
  start, which also holds for an entirely empty column.
- Limits: `MAX_TABLE_ROWS = 100` (header included), `MAX_TABLE_COLUMNS = 20`
  and `MAX_TABLE_CELL_CHARS = 1000` characters per text cell.
- `validate_table` checks, in this order:
  1. the sizes, before any walk;
  2. at least one row and one column;
  3. rows of equal width;
  4. the length of texts;
  5. finite numbers;
  6. `align.len() <= columns`.

  It is `pub(crate)`. Errors go through `ChartError::Inconsistent`
  with `chart-table-*` keys. No variant is added: `kind()`
  is enough, and the display of `Inconsistent` becomes "inconsistent data".
- `parse_table(&Value)` reads (without cloning) then validates.
- `table_data(&TableOption, &DataOp) -> TableOption`, with no inverse (the core
  does not use one: history goes through the replica):
  - rejects, before any cloning, an op that would exceed the limits;
  - applies the op, then realigns `align`:
    - an inserted column gets `auto`, but only if `align` already covers
      that position;
    - a deleted column removes its entry;
    - a `replace` truncates `align`;
  - validates the result.
- `chart_data(&ChartOption, &DataOp) -> ChartOption` joins the crate, from
  the core, as the counterpart of `table_data`.
- `patch_table` shares with `apply_option_patch` a `merge_patch<T>` helper,
  which keeps `dataset`, applies the merge and computes the inverse.
- Row 0 keeps the meaning it has in a chart: the column names.
  `DataOp` protects it (no insertion or deletion at 0). `header-row` only
  decides how it is displayed.
- Conversion:
  - `chart_to_table(&ChartOption)`: rejected if the chart has no
    dataset; short rows are padded with `null`; the result is
    validated; `header-column: true`.
  - `table_to_chart(&TableOption)`: bars on the same dataset, `xAxis`
    category, one series per value column (`sync_series_with_columns`),
    then normalization.

## Core (`oku-slides-core`)

- `HostType::Table`, plus two public helpers in `widget.rs`:
  - `ParamType::host() -> Option<HostType>`;
  - `WidgetSchema::host_param(HostType) -> Option<&ParamSchema>`.

  They are used in `ops/host.rs`, in `render/src/charts.rs` and in
  `session/src/charts.rs`.
- **`ops/host.rs`** receives everything related to hosts:
  - `host_option`, `expected_host`;
  - the insertion check: `check_host_params` and `check_hosts`;
  - a single `set_option(.., HostType, ..)` for `set_chart_option` and
    `set_table_option`;
  - `set_data`.

  The `pub(super)` items of `value.rs` and the helpers of `mod.rs` stay private: a
  child module already sees them.
- `validate.rs` (`fits`) reuses `host_option`: a single `Host` arm.
- `replica/schema.rs`: nothing to change, a `host` already falls into `Atom`.
- Errors: `OpInner::Table`, `OpErrorKind::InvalidTable`, key
  `op-invalid-table` ("Tableau t1 : …", i.e. "Table t1: …").
- Core `ARCHITECTURE.md`: add `ops/host.rs` and the "new host type"
  extension point.

## Session

- Request `show_as`, implemented in the new module `tables.rs`
  (`lib.rs` is already close to 1000 lines). It is exposed over MCP (`show_as`),
  and `SessionErrorKind::NoTable` reports that an element has no table.
- `charts.rs` ignores tables (canonicalization applies to `Chart`
  only) and uses `ParamType::host()`.
- `target_params.rs`, `param_fields.rs`, `widget_preview.rs`: nothing to change,
  since `Host { .. }` is never settable in the preview.
- i18n: `session-no-table`, plus the theme tokens.

## Rendering

- `render/src/tables.rs`: the guard described above, called in `set_deck`
  before the charts. `for_each_element` is shared with `charts.rs`.
- `typst/oku-slides/src/widgets.typ`: `_table`, inserted in `registry` after
  `_chart`. It keeps the placeholder for any malformed data:
  - `source` that is not a list of lists;
  - a flag that is not a boolean;
  - `align` that is not a list;
  - a cell of an unknown type, rendered empty;
  - a row that is too long, truncated;
  - more than 100 rows or 20 columns.

  The rendering itself:
  - alignments are precomputed per column;
  - the header row goes into `table.header`, in bold on the
    `header-bg` background;
  - the 1st column as header is bold only: a column background
    would compete with the striping;
  - the striping alternates `stripe-bg` on data rows;
  - measurement is done at the available width; shrinking happens only if the
    table still overflows, and never from a zero measurement.

  The placeholder has a fixed height in a flow, and fills its frame otherwise.
- `data.typ:154`: the box of a `table` in a flow is measured within the available
  height, like that of a `chart`, otherwise the selection frame does not
  match the drawing.
- Cells:
  - an integer number is displayed without decimals;
  - a text is displayed as is;
  - a boolean is `✓` (true) or nothing (false);
  - `null` is empty.

  Texts are data, never markup.
- `component.table` tokens in `default-tokens.json` (the merge
  `default-tokens ← theme` gives them to `papier`) and in `pro.json`, which is a
  full snapshot: `size`, `header-bg`, `header-color`, `header-weight`,
  `stripe-bg`, `rule-color`, `rule-width`, `padding-x`, `padding-y`. In
  `pro`, `stripe-bg` differs from `rule`.
- These tokens are described in `theme_schema.json` (`components` section) and
  have their `theme-token-component-table-*` keys in the 20 locales.
- The table does not grow in a content slide (no `grow`).
- The frozen registry (fixture, sorted by name) and the Typst widget land in
  the same commit.
- Docs to update:
  - README ("14 widgets", `p.host` line);
  - `params.typ`;
  - the render `ARCHITECTURE.md` (the table is rendered by Typst, with no Rust
    drawing pass).

## UI (`ui/`)

- `ChartDataOp` becomes `DataOp` in `chart/chartTypes.ts`, and the
  `set_data` op is sent.
- Insertion:
  - `ADD_KINDS` gains `"table"`;
  - `newElement` uses `starterTable()`, with "Colonne n" (Column n) headers
    in the user's language;
  - `Icon.table`;
  - the `[table]` branches of `ui-add-kind` and `ui-undo-add`.
- `hasOwnPanel(kind)` lives in a neutral module (`ownPanel.ts`) and serves
  `TargetPanel` as well as `useTargetSettings`.
- `ui/src/table/TablePanel.tsx`:
  - the header, 1st column and striping toggles, which send
    `set_table_option`;
  - **a per-column alignment selector** (the grid exposes no selected
    column);
  - `ChartDataGrid`:
    - with `firstColumnIsLabel` following `header-column`;
    - with a new `newColumnName` prop, so that "Ajouter une colonne" (Add a
      column) names the column "Colonne n" (Column n) and not "Série n"
      (Series n);
  - the "Afficher en graphique" (Show as chart) button.
- `ChartPanel` gains the "Afficher en tableau" (Show as table) button. Its group
  has its own class, so as not to hijack the stacking test.
- Both buttons call `api.showAs`, a single undoable request. If the
  conversion is rejected (text in a value column, table too
  large), the error is shown.
- CSS: `.table-align`, in `ChartDataGrid.css`, with existing tokens.
- The `toolbar.cjs` e2e counts 5 add buttons.
- i18n `ui-table-*` and `ui-undo-*` in the 20 locales, in the same key order
  everywhere.
- The help (`docs/aide/en.json` and `fr.json`) is updated:
  - the add and widget lists (14);
  - the `p.host` type (chart, table);
  - a paragraph about the table.

## Tests

- `oku-slides-chart`, `table_tests.rs`:
  - the validation bounds, including exactly at the limit;
  - irregular rows, text too long, `align` too long;
  - the synchronization of `align` (insertion at 0, at `align.len()`, beyond;
    deletion; replacement);
  - rejection of an op that exceeds the limits;
  - the patch that rejects `dataset`;
  - localization of the reasons;
  - conversion in both directions (short rows, limits, one series per
    column).
- `oku-slides-core`, `tests/tables.rs`:
  - adding, replacing and nesting in a stack a table,
    valid or not;
  - `SetParam` of an invalid object;
  - `SetData` and `SetTableOption`, and the rejections that leave the deck intact;
  - `set_chart_option` applied to a table;
  - `validate`, on a valid as well as an invalid table;
  - `ops_schema`.

  `value_tests.rs` gains its `host(table)` cases.
- `oku-slides-render`, `tests/tables.rs`, by page fingerprints:
  - a shrunk table fits in its frame: an opaque rectangle covering the
    frame renders the page identically with or without the table;
  - in a flow, the box of a tall table stays within the page;
  - the placeholder differs from an unknown element;
  - the column header, alignment, striping and tokens change the
    page, and `end` equals `auto` on a column of numbers;
  - malformed or oversized data in a deck give the placeholder and
    compile;
  - marked-up text (`#panic`, `$x$`…) stays literal;
  - a pixel golden.
- `oku-slides-session`:
  - `show_as` in both directions, in a single undoable step;
  - undoing `set_data` and `set_table_option`;
  - rejection of a table that is too large.
- `oku-slides-mcp`: the `apply` schema names `set_data` and
  `set_table_option`, and the `show_as` tool exists.
- UI:
  - `tableDeck`, `ownPanel`, `TablePanel` (options, alignment, grid,
    conversion);
  - `TargetPanel` and `useTargetSettings` for a table;
  - `newElement`, `slideTools`;
  - the `ChartPanel` stacking test.
