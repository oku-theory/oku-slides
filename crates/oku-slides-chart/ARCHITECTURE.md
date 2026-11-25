# oku-slides-chart

## Role

Charts (ECharts options to SVG) and tables (model only: Typst renders them, in another crate).

## A chart's path

`parse_option` → `canonicalize` → `apply_option_patch` → `ChartRenderer`.

- `parse_option` checks ECharts options (v1 subset);
- `canonicalize` moves inline data into `dataset`, which the core's `prepare_ops` lifts into deck data;
- `apply_option_patch` (merge patch, outside `dataset`) edits and returns its inverse; data edits are deck `edit_data` ops, not this crate's;
- `ChartRenderer` (charts-rs) produces the SVG, its plot area inset by 2 % of the chart on each side as in LibreOffice, 5 pt at least (t-1514).
- `scale`: a value axis `interval` becomes exact bounds and a split count; without it, missing bounds follow PowerPoint's and
  LibreOffice's automatic scale (5 % margin, 1/2/5 step, at most 10 ticks, t-1493); `splitLine.show: false` hides the grid (transparent stroke);
  `bar_gap` resizes charts-rs's bar `<rect>`s (and their value labels) to `barCategoryGap`, which charts-rs 1.1 lacks (t-1384).

## Identifiers

`Dataset.ids` (one per row, header included) and `Dataset.columns` (one per column), types of `oku-slides-data`; `ids.rs` checks them aligned, well formed and unique (`chart-ids-*`). Series find their column by id (`canonical.rs`).

## Data references

`refs.rs`: `data` (a deck data id) and `columns` (its projection) in chart and table options; refused: `data: null` (`chart-ref-null`), a ref beside inline data (`chart-ref-and-inline`), bad `columns` (`chart-columns-invalid`). A stored view always names deck data; `dataset` is only the render input the core resolves from it.

## Table

- `TableOption`: `dataset` (row 0 = column names; render input only), `header-row`, `header-column`, `striped`, `align`, `column-widths`, `row-heights`.
- `parse_table` reads, fits `align` and the weights to the data (a merge may leave them off), validates (`valid_weights`: one weight in (0, `MAX_TABLE_WEIGHT`] per column or row); `patch_table` merge-patches outside `dataset`.
- Limits: `MAX_TABLE_ROWS` (100, header included), `MAX_TABLE_COLUMNS` (20), `MAX_TABLE_CELL_CHARS` (1000).
- `chart_to_table` and `table_to_chart` switch widgets without losing the data.

## Errors

`ChartError` covers charts and tables, with the `chart-*` and `chart-table-*` keys (crate `oku-slides-i18n`). No `TableError`.

## Extension points

- A chart type: `SeriesType` and `model.rs`.
- A table option: `TableOption`, `validate_table`, and the Typst widget `_table`.

## Dependencies

`oku-slides-data` (`Dataset`, `DataCell`, ids, numbers), `oku-slides-i18n`, `charts-rs`, `json-patch`, `serde`, `schemars`, `snafu`. No dependency on Typst or Tauri.
