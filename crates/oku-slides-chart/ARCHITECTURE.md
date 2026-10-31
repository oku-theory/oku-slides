# oku-slides-chart

## Rôle

Graphiques (options ECharts → SVG) et tableaux (modèle seul : le rendu est fait par Typst, dans un autre crate).

## Chemin d'un graphique

`parse_option` → `canonicalize` → `apply_option_patch` / `chart_data` → `ChartRenderer`.

- `parse_option` valide les options ECharts (sous-ensemble v1) ;
- `canonicalize` range les données dans `dataset` avant stockage ;
- `apply_option_patch` (merge patch, hors `dataset`) édite et renvoie son inverse ; `chart_data` applique une `DataOp`, sans inverse ;
- `ChartRenderer` (charts-rs) produit le SVG.

## Identifiants

`Dataset.ids` (un par ligne, en-tête compris) et `Dataset.columns` (un par colonne) : `ensure_ids` les aligne sur `source`, et chaque `DataOp` les garde alignés (le cœur l'appelle avant d'écrire).

## Tableau

- `TableOption`: `dataset` (row 0 = column names), `header-row`, `header-column`, `striped`, `align`, `column-widths`, `row-heights`.
- `parse_table` reads, fits `align` and the weights to the data (a merge may leave them off), validates (`valid_weights`: one weight in (0, `MAX_TABLE_WEIGHT`] per column or row); `table_data` applies a `DataOp` and realigns `align` and the weights; `patch_table` merge-patches outside `dataset`.
- Limites : `MAX_TABLE_ROWS` (100, en-tête compris), `MAX_TABLE_COLUMNS` (20), `MAX_TABLE_CELL_CHARS` (1000).
- `chart_to_table` et `table_to_chart` passent d'un widget à l'autre sans perdre les données.

## Erreurs

`ChartError` couvre graphiques et tableaux, avec les clés `chart-*` et `chart-table-*` (crate `oku-slides-i18n`). Pas de `TableError`.

## Points d'extension

- Un type de graphique : `SeriesType` et `model.rs`.
- Une option de tableau : `TableOption`, `validate_table`, et le widget Typst `_table`.

## Dépendances

`oku-slides-i18n`, `charts-rs`, `json-patch`, `serde`, `schemars`, `snafu`. Aucune dépendance à Typst ni à Tauri.
