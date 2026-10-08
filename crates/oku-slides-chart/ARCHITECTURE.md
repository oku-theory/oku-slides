# oku-slides-chart

## Rôle

Graphiques (options ECharts → SVG) et tableaux (modèle seul : le rendu est fait par Typst, dans un autre crate).

## Chemin d'un graphique

`parse_option` → `canonicalize` → `apply_option_patch` / `chart_data` → `ChartRenderer`.

- `parse_option` valide les options ECharts (sous-ensemble v1) ;
- `canonicalize` range les données dans `dataset` avant stockage ;
- `apply_option_patch` (merge patch, hors `dataset`) et `chart_data` (`DataOp`) éditent et renvoient leur inverse ;
- `ChartRenderer` (charts-rs) produit le SVG.

## Tableau

- `TableOption` : `dataset` (ligne 0 = noms de colonnes), `header-row`, `header-column`, `striped`, `align`.
- `parse_table` lit et valide ; `table_data` applique une `DataOp` et recale `align` ; `patch_table` fait un merge patch hors `dataset`.
- Limites : `MAX_TABLE_ROWS` (100, en-tête compris), `MAX_TABLE_COLUMNS` (20), `MAX_TABLE_CELL_CHARS` (1000).
- `chart_to_table` et `table_to_chart` passent d'un widget à l'autre sans perdre les données.

## Erreurs

`ChartError` couvre graphiques et tableaux, avec les clés `chart-*` et `chart-table-*` (crate `oku-slides-i18n`). Pas de `TableError`.

## Points d'extension

- Un type de graphique : `SeriesType` et `model.rs`.
- Une option de tableau : `TableOption`, `validate_table`, et le widget Typst `_table`.

## Dépendances

`oku-slides-i18n`, `charts-rs`, `json-patch`, `serde`, `schemars`, `snafu`. Aucune dépendance à Typst ni à Tauri.
