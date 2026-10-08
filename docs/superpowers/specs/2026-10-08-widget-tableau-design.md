# Widget tableau — design

Date : 2026-10-08 · Tâche : t-578

## Objectif

Un nouveau widget d'élément **`table`** affiche un tableau de données dans une
slide. Il réutilise les données des graphiques : même `Dataset`
(`source: Vec<Vec<Cell>>`), mêmes opérations d'édition (insérer, supprimer une
ligne ou une colonne, régler une cellule, remplacer), même grille d'édition
dans l'interface. Un graphique peut s'afficher en tableau et inversement, en
gardant ses données.

Options de la V1 : ligne d'en-tête, première colonne en en-tête, lignes
zébrées, alignement par colonne.

Hors périmètre : cellules fusionnées, texte riche dans les cellules, formats de
nombre (devise, pourcentage, séparateur décimal selon la langue), tri,
pagination d'un tableau sur plusieurs slides.

## Décisions

- **Rendu en Typst natif** (`table(...)`), pas en SVG : le texte reste net et
  sélectionnable dans le PDF, et il suit les jetons du thème (police, tailles,
  couleurs) sans passer par la palette des graphiques.
- **Un nouveau type d'hôte `host("table")`** (`HostType::Table`). Le widget
  Typst déclare `params: (data: p.host("table"))`. Tout le traitement propre
  au tableau en Rust part de ce type, comme `host("chart")` pour les graphiques.
- **Les données restent dans `oku-slides-chart`**, qui gagne un module
  `table`. `Dataset`, `Cell` et l'op de données sont partagés tels quels (G11).
  Le crate n'est pas renommé.
- **Op de données générique.** `ChartDataOp` devient **`DataOp`** et
  `Op::SetChartData` devient **`Op::SetData`**, valable pour `host(chart)` et
  `host(table)`. C'est un changement cassant du format des ops, ce que le MVP
  permet. Le pendant de `SetChartOption` est **`Op::SetTableOption`** : un
  merge patch qui refuse de toucher `dataset`.
- **Débordement : mise à l'échelle.** Un tableau plus grand que son cadre est
  réduit pour y tenir (mesure puis `scale`), sans pagination.
- **Conversion = un seul `ReplaceElement`.** Elle garde l'id, le cadre, la case
  et le style, et compte pour une seule étape d'annulation. Elle n'ajoute aucune
  op.

## Modèle (`oku-slides-chart::table`)

```rust
pub struct TableOption {
    pub dataset: Dataset,
    #[serde(default = "yes")] pub header_row: bool,     // true
    #[serde(default)]         pub header_column: bool,  // false
    #[serde(default)]         pub striped: bool,        // false
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<ColumnAlign>,
}
pub enum ColumnAlign { Auto, Start, Center, End }   // sérialisé en minuscules
```

- Le JSON utilise `deny_unknown_fields`, avec les noms `header-row`,
  `header-column`, `striped`, `align` et `dataset`, en kebab-case comme les
  jetons et les params Typst.
- `align[i]` s'applique à la colonne `i`. Une colonne sans entrée vaut `auto`.
  `auto` aligne à droite (à la fin) une colonne dont toutes les cellules hors
  en-tête sont des nombres ou vides, et au début sinon.
- `validate_table(&TableOption)` vérifie :
  - au moins une ligne et une colonne ;
  - des lignes de même largeur ;
  - `align.len() <= colonnes` ;
  - des limites de taille (`MAX_TABLE_ROWS = 100` et `MAX_TABLE_COLUMNS = 20`).

  Les erreurs passent par `ChartError`, avec de nouvelles variantes et de
  nouvelles clés `chart-table-*` dans `chart.ftl`.
- `table_data(option, DataOp) -> (TableOption, DataOp)` applique l'op au
  dataset et recale `align` : une colonne insérée reçoit `auto` (uniquement si
  `align` couvre déjà cette position), une colonne supprimée retire son entrée,
  et un `replace` tronque `align`. Elle valide puis renvoie l'inverse.
- `patch_table(option, patch)` est un merge patch JSON qui refuse `dataset`,
  sur le modèle de `patch.rs`.
- `table_schema()` est le JSON Schema destiné au LLM, sur le modèle de
  `schema.rs`.

## Cœur (`oku-slides-core`)

- `HostType::Table` est ajouté, ainsi que ses branches dans toutes les
  correspondances : `widget.rs` (`check_param`, `compatible`), `ops/value.rs`,
  `ops/check.rs` (« a table »), `validate.rs` (`fits`),
  `replica/schema.rs` (Atom, comme le graphique).
- `ops/mod.rs` :
  - les helpers `is_chart` et `chart_param` laissent place à un `host_of(ty)`
    commun ;
  - `check_host_params` contrôle les graphiques et les tableaux à l'ajout, au
    remplacement et à l'ajout de slide ;
  - `SetData` choisit le traitement selon l'hôte du param : `chart_data`
    (inchangé) ou `table_data` ;
  - `SetTableOption` lit, patche, valide et écrit l'option.
- `ops_schema` et la description de l'outil MCP `apply` sont mis à jour.

## Session

- `charts.rs` (canonicalisation) ignore les tableaux. Leur forme canonique est
  celle qu'ils ont déjà : rien à recalculer.
- `target_params.rs` : pas de réglages génériques pour un widget `host(table)`,
  car le panneau dédié s'en charge.
- Les branches `param_fields.rs` et `widget_preview.rs` du nouveau type d'hôte
  sont ajoutées.
- i18n : `session-widget-kind-table`, `session-param-table-data`, plus les
  jetons du thème (voir Rendu), dans les 20 locales.

## Rendu (Typst)

- `typst/oku-slides/src/widgets.typ` : `_table`, déclaré dans `registry` après
  `_chart`. Il rend :
  - `table(columns: n, align: …, fill: …, stroke: …, ..cellules)` ;
  - l'en-tête de ligne et l'en-tête de colonne en `table.header`, avec le
    gras et le fond des jetons ;
  - le zébrage, en alternant le fond des lignes de données ;
  - le tout dans `layout(size => …)`, qui mesure le tableau et applique
    `scale` si la largeur ou la hauteur dépasse.
- Cellules :
  - un nombre est affiché avec `str(n)` (entier sans décimales) ;
  - un texte tel quel ;
  - un booléen en `✓` ou vide ;
  - `null` vide.
- Nouveaux jetons `component.table` dans `default-tokens.json` et les thèmes
  intégrés (`papier`, `pro`) : `size`, `header-bg`, `header-color`,
  `header-weight`, `stripe-bg`, `rule-color`, `rule-width`, `padding-x`,
  `padding-y`.
- Ces jetons sont décrits dans `theme_schema.json` (section `components`) et
  ont leurs clés `theme-token-component-table-*` dans les 20 locales.
- Le tableau ne grandit pas dans un slide de contenu : la règle `grow` du
  graphique ne s'applique pas à lui.
- Registre figé : `fixtures/widgets/base-registry.json`, le snapshot
  `widgets__base_registry`, la liste des widgets d'élément dans
  `tests/widgets.rs`, et le README (« 14 widgets »).

## Interface (`ui/`)

- `ChartDataOp` devient `DataOp` dans `chart/chartTypes.ts`, et l'api envoie
  `set_data`.
- Insertion :
  - `ADD_KINDS` gagne `"table"` ;
  - `newElement.ts` utilise `starterTable()` (3×3 avec en-tête) ;
  - `Icon.table` est ajoutée ;
  - les sélecteurs `ui-add-kind` et `ui-undo-add` ont une branche `[table]`.
- `ui/src/table/TablePanel.tsx` (inspecteur, affiché par `TargetPanel` pour
  un widget `host(table)`) :
  - les bascules en-tête, 1re colonne et zébrage, qui envoient
    `set_table_option` ;
  - l'alignement de la colonne sélectionnée ;
  - **`ChartDataGrid`** réutilisé tel quel, qui envoie `set_data` ;
  - le bouton « Afficher en graphique ».
- `ChartPanel` gagne le bouton « Afficher en tableau ».
- `ui/src/table/convert.ts` : `chartToTable(el)` et `tableToChart(el)`
  produisent l'élément de remplacement.
  - Graphique → tableau : on garde `dataset`, avec `header-row: true` et
    `header-column: true`.
  - Tableau → graphique : `dataset`, `xAxis` catégorie et une série `bar`, que
    la canonicalisation de la session aligne sur les colonnes.

  Les deux passent par l'op `replace_element`.
- Le cas `"chart"` codé en dur (`TargetPanel`, `useTargetSettings`,
  `chartDeck.optionOf`) passe à une détection par hôte quand c'est nécessaire.
- i18n : `ui-table-*` et `ui-undo-table-*` dans les 20 locales.
- L'aide (`docs/aide/en.json` et `fr.json`) décrit le widget et la conversion.

## Tests

- `oku-slides-chart` : `table_tests.rs`, qui couvre
  - la validation : bornes, lignes irrégulières, `align` trop long ;
  - la synchronisation de `align` et l'inverse de chaque op ;
  - le patch qui refuse `dataset`.
- `oku-slides-core` : `tests/tables.rs`, qui couvre
  - l'ajout d'un tableau valide ou invalide ;
  - `SetData` et `SetTableOption` avec leur annulation ;
  - `SetData` sur un graphique, qui reste valide (tests `charts.rs` passés à
    `SetData`) ;
  - `validate` et `ops_schema`.
- `oku-slides-render` :
  - le rendu d'un tableau (texte présent, en-tête en gras) ;
  - la mise à l'échelle d'un grand tableau dans un petit cadre ;
  - les jetons du thème appliqués ;
  - un golden pixel ajouté à `tests/golden/widgets.json`.
- `oku-slides-session` : l'aller-retour remplacement graphique ↔ tableau en une
  étape d'historique.
- `oku-slides-i18n` : la parité des clés, déjà testée.
- UI :
  - `TablePanel.render.test.tsx` ;
  - `convert.test.ts` ;
  - `newElement.test.ts` et `slideTools.test.ts`, mis à jour ;
  - `tableTokens.test.ts`, si `TablePanel` a du CSS.
