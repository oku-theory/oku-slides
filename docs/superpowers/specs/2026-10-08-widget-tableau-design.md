# Widget tableau — design

Date : 2026-10-08 · Tâche : t-578 · Révisé après la revue d'équipe du plan.

## Objectif

Un nouveau widget d'élément **`table`** affiche un tableau de données dans une
slide. Il réutilise les données des graphiques : le même `Dataset`
(`source: Vec<Vec<Cell>>`), les mêmes opérations d'édition (insérer ou
supprimer une ligne ou une colonne, régler une cellule, tout remplacer) et la
même grille d'édition dans l'interface. Un graphique peut s'afficher en
tableau et inversement, en gardant ses données.

Options de la V1 : ligne d'en-tête, première colonne en en-tête, lignes
zébrées, alignement par colonne.

Hors périmètre, à traiter dans des tâches du backlog :
- cellules fusionnées, texte riche dans les cellules, tri, pagination d'un
  tableau sur plusieurs slides ;
- format des nombres selon la langue (t-590) ;
- fusion fine des éditions concurrentes (t-589) ;
- aller-retour graphique → tableau → graphique sans perte du type (t-593).

## Décisions

- **Rendu en Typst natif** (`table(...)`), pas en SVG. Le texte reste net et
  sélectionnable dans le PDF, et il suit les jetons du thème.
- **Nouveau type d'hôte `host("table")`** (`HostType::Table`). Le widget
  Typst déclare `params: (data: p.host("table"))`.
- **Les données restent dans `oku-slides-chart`**, qui gagne un module `table`
  et s'occupe désormais des « graphiques et tableaux ». `Dataset`, `Cell` et
  l'op de données sont partagés tels quels (G11). Le crate garde son nom, mais
  sa description et son nouvel `ARCHITECTURE.md` le disent.
- **Op de données générique.** `ChartDataOp` devient **`DataOp`** et
  `Op::SetChartData` devient **`Op::SetData`**. C'est un changement cassant du
  format des ops, ce que le MVP permet. Le pendant de `SetChartOption` est
  **`Op::SetTableOption`**, un merge patch qui refuse `dataset`.
- **Conversion côté Rust.** La requête de session **`show_as`** (`view: "table"`
  ou `"chart"`) est exposée à l'interface et à l'agent (outil MCP `show_as`),
  sur le modèle de `chart_kind`. Le crate fournit
  `chart_to_table`/`table_to_chart` :
  - les lignes trop courtes sont complétées ;
  - les limites sont vérifiées ;
  - un graphique reçoit une série par colonne de valeurs.

  Le remplacement se fait par un seul `ReplaceElement`, ce qui donne une seule
  étape d'annulation, et garde l'id, le cadre, la case et le style.
- **Débordement : mise à l'échelle.** Le tableau est mesuré à la largeur
  disponible : les cellules passent à la ligne. Il n'est réduit (`scale`) que
  s'il dépasse encore, surtout en hauteur. Il n'y a pas de pagination.
- **Garde au rendu.** Un deck ouvert n'est pas refusé : `validate` ne fait que
  signaler. Avant Typst, le rendu passe donc chaque valeur `host(table)` par
  `parse_table` :
  - si elle est valide, Typst reçoit la forme canonique ;
  - sinon, il reçoit `null`, le widget affiche son substitut et un `warn` est
    journalisé.

  Le widget Typst vérifie lui aussi les types, en défense en profondeur.
- **Détection par `kind` dans l'interface**, comme pour le graphique. Le
  panneau dédié s'ouvre pour `kind: "table"` et lit le param `data`. Un widget
  de pack qui déclare `host("table")` sous un autre nom n'a pas de panneau
  dédié. C'est un écart assumé à la règle de détection par hôte.

## Modèle (`oku-slides-chart::table`)

```rust
pub struct TableOption {
    pub dataset: Dataset,
    #[serde(default = "default_true")] pub header_row: bool,  // true
    #[serde(default)]                  pub header_column: bool,  // false
    #[serde(default)]                  pub striped: bool,        // false
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<ColumnAlign>,
}
pub enum ColumnAlign { Auto, Start, Center, End }   // sérialisé en minuscules
```

- Le JSON est en kebab-case, avec `deny_unknown_fields` : `header-row`,
  `header-column`, `striped`, `align`, `dataset`.
- `align[i]` s'applique à la colonne `i`. Une colonne sans entrée vaut `auto`.
  `auto` aligne à la fin une colonne dont les cellules hors en-tête sont toutes
  des nombres ou vides, avec au moins un nombre. Sinon, l'alignement est au
  début, ce qui vaut aussi pour une colonne entièrement vide.
- Limites : `MAX_TABLE_ROWS = 100` (en-tête compris), `MAX_TABLE_COLUMNS = 20`
  et `MAX_TABLE_CELL_CHARS = 1000` caractères par cellule de texte.
- `validate_table` vérifie, dans cet ordre :
  1. les tailles, avant tout parcours ;
  2. au moins une ligne et une colonne ;
  3. des lignes de même largeur ;
  4. la longueur des textes ;
  5. des nombres finis ;
  6. `align.len() <= colonnes`.

  Elle est `pub(crate)`. Les erreurs passent par `ChartError::Inconsistent`
  avec des clés `chart-table-*`. Aucune variante n'est ajoutée : `kind()`
  suffit, et l'affichage de `Inconsistent` devient « inconsistent data ».
- `parse_table(&Value)` lit (sans clonage) puis valide.
- `table_data(&TableOption, &DataOp) -> TableOption`, sans inverse (le cœur
  n'en utilise pas : l'historique passe par le replica) :
  - refuse avant tout clonage une op qui dépasserait les limites ;
  - applique l'op, puis recale `align` :
    - une colonne insérée reçoit `auto`, mais seulement si `align` couvre
      déjà cette position ;
    - une colonne supprimée retire son entrée ;
    - un `replace` tronque `align` ;
  - valide le résultat.
- `chart_data(&ChartOption, &DataOp) -> ChartOption` rejoint le crate, depuis
  le cœur, pour faire pendant à `table_data`.
- `patch_table` partage avec `apply_option_patch` un helper `merge_patch<T>`,
  qui garde `dataset`, applique le merge et calcule l'inverse.
- La ligne 0 garde le sens qu'elle a dans un graphique : les noms de colonnes.
  `DataOp` la protège (ni insertion ni suppression en 0). `header-row` ne
  décide que de son affichage.
- Conversion :
  - `chart_to_table(&ChartOption)` : refusée si le graphique n'a pas de
    dataset ; les lignes courtes sont complétées par `null` ; le résultat est
    validé ; `header-column: true`.
  - `table_to_chart(&TableOption)` : barres sur le même dataset, `xAxis`
    catégorie, une série par colonne de valeurs (`sync_series_with_columns`),
    puis normalisation.

## Cœur (`oku-slides-core`)

- `HostType::Table`, plus deux helpers publics dans `widget.rs` :
  - `ParamType::host() -> Option<HostType>` ;
  - `WidgetSchema::host_param(HostType) -> Option<&ParamSchema>`.

  Ils servent dans `ops/host.rs`, dans `render/src/charts.rs` et dans
  `session/src/charts.rs`.
- **`ops/host.rs`** reçoit tout ce qui touche aux hôtes :
  - `host_option`, `expected_host` ;
  - le contrôle à l'insertion : `check_host_params` et `check_hosts` ;
  - un seul `set_option(.., HostType, ..)` pour `set_chart_option` et
    `set_table_option` ;
  - `set_data`.

  Les `pub(super)` de `value.rs` et des helpers de `mod.rs` restent privés : un
  module enfant les voit déjà.
- `validate.rs` (`fits`) réutilise `host_option` : un seul bras `Host`.
- `replica/schema.rs` : rien à changer, un `host` tombe déjà en `Atom`.
- Erreurs : `OpInner::Table`, `OpErrorKind::InvalidTable`, clé
  `op-invalid-table` (« Tableau t1 : … »).
- `ARCHITECTURE.md` du cœur : ajouter `ops/host.rs` et le point d'extension
  « nouveau type d'hôte ».

## Session

- Requête `show_as`, implémentée dans le nouveau module `tables.rs`
  (`lib.rs` approche déjà 1000 lignes). Elle est exposée par MCP (`show_as`),
  et `SessionErrorKind::NoTable` signale qu'un élément n'a pas de tableau.
- `charts.rs` ignore les tableaux (la canonicalisation porte sur `Chart`
  seulement) et utilise `ParamType::host()`.
- `target_params.rs`, `param_fields.rs`, `widget_preview.rs` : rien à changer,
  puisque `Host { .. }` n'est jamais réglable dans l'aperçu.
- i18n : `session-no-table`, plus les jetons du thème.

## Rendu

- `render/src/tables.rs` : le garde décrit plus haut, appelé dans `set_deck`
  avant les graphiques. `for_each_element` est partagé avec `charts.rs`.
- `typst/oku-slides/src/widgets.typ` : `_table`, inséré dans `registry` après
  `_chart`. Il garde le substitut sur toute donnée mal formée :
  - `source` qui n'est pas une liste de listes ;
  - drapeau qui n'est pas un booléen ;
  - `align` qui n'est pas une liste ;
  - cellule d'un type inconnu, rendue vide ;
  - ligne trop longue, tronquée ;
  - plus de 100 lignes ou 20 colonnes.

  Le rendu lui-même :
  - les alignements sont précalculés par colonne ;
  - la ligne d'en-tête va en `table.header`, en gras sur le fond
    `header-bg` ;
  - la 1re colonne en en-tête est en gras seulement : un fond de colonne
    concurrencerait le zébrage ;
  - le zébrage alterne `stripe-bg` sur les lignes de données ;
  - la mesure se fait à la largeur disponible ; la réduction n'a lieu que si le
    tableau dépasse encore, et jamais à partir d'une mesure nulle.

  Le substitut fait une hauteur fixe dans un flux, et remplit son cadre sinon.
- `data.typ:154` : la boîte d'un `table` en flux est mesurée dans la hauteur
  disponible, comme celle d'un `chart`, sinon le cadre de sélection ne
  correspond pas au dessin.
- Cellules :
  - un nombre entier s'affiche sans décimales ;
  - un texte s'affiche tel quel ;
  - un booléen vaut `✓` (vrai) ou rien (faux) ;
  - `null` est vide.

  Les textes sont des données, jamais du balisage.
- Jetons `component.table` dans `default-tokens.json` (la fusion
  `default-tokens ← thème` les donne à `papier`) et dans `pro.json`, qui est un
  instantané complet : `size`, `header-bg`, `header-color`, `header-weight`,
  `stripe-bg`, `rule-color`, `rule-width`, `padding-x`, `padding-y`. Dans
  `pro`, `stripe-bg` diffère de `rule`.
- Ces jetons sont décrits dans `theme_schema.json` (section `components`) et
  ont leurs clés `theme-token-component-table-*` dans les 20 locales.
- Le tableau ne grandit pas dans une slide de contenu (pas de `grow`).
- Le registre figé (fixture, triée par nom) et le widget Typst arrivent dans
  le même commit.
- Docs à mettre à jour :
  - README (« 14 widgets », ligne `p.host`) ;
  - `params.typ` ;
  - `ARCHITECTURE.md` du rendu (le tableau est rendu par Typst, sans passe
    Rust de dessin).

## Interface (`ui/`)

- `ChartDataOp` devient `DataOp` dans `chart/chartTypes.ts`, et l'op
  `set_data` est envoyée.
- Insertion :
  - `ADD_KINDS` gagne `"table"` ;
  - `newElement` utilise `starterTable()`, avec des en-têtes « Colonne n »
    dans la langue de l'utilisateur ;
  - `Icon.table` ;
  - les branches `[table]` de `ui-add-kind` et `ui-undo-add`.
- `hasOwnPanel(kind)` vit dans un module neutre (`ownPanel.ts`) et sert à
  `TargetPanel` comme à `useTargetSettings`.
- `ui/src/table/TablePanel.tsx` :
  - les bascules en-tête, 1re colonne et zébrage, qui envoient
    `set_table_option` ;
  - **un sélecteur d'alignement par colonne** (la grille n'expose pas de
    colonne sélectionnée) ;
  - `ChartDataGrid` :
    - avec `firstColumnIsLabel` qui suit `header-column` ;
    - avec une nouvelle prop `newColumnName`, pour que « Ajouter une colonne »
      nomme la colonne « Colonne n » et non « Série n » ;
  - le bouton « Afficher en graphique ».
- `ChartPanel` gagne le bouton « Afficher en tableau ». Son groupe a sa propre
  classe, pour ne pas détourner le test de l'empilement.
- Les deux boutons appellent `api.showAs`, une seule requête annulable. Si la
  conversion est refusée (texte dans une colonne de valeurs, tableau trop
  grand), l'erreur s'affiche.
- CSS : `.table-align`, dans `ChartDataGrid.css`, avec des jetons existants.
- L'e2e `toolbar.cjs` compte 5 boutons d'ajout.
- i18n `ui-table-*` et `ui-undo-*` dans les 20 locales, dans le même ordre de
  clés partout.
- L'aide (`docs/aide/en.json` et `fr.json`) est mise à jour :
  - les listes d'ajout et de widgets (14) ;
  - le type `p.host` (chart, table) ;
  - un paragraphe sur le tableau.

## Tests

- `oku-slides-chart`, `table_tests.rs` :
  - les bornes de validation, y compris pile à la limite ;
  - lignes irrégulières, texte trop long, `align` trop long ;
  - la synchronisation de `align` (insertion en 0, à `align.len()`, au-delà ;
    suppression ; remplacement) ;
  - le refus d'une op qui dépasse les limites ;
  - le patch qui refuse `dataset` ;
  - la localisation des raisons ;
  - la conversion dans les deux sens (lignes courtes, limites, une série par
    colonne).
- `oku-slides-core`, `tests/tables.rs` :
  - l'ajout, le remplacement et l'imbrication dans une pile d'un tableau,
    valide ou non ;
  - `SetParam` d'un objet invalide ;
  - `SetData` et `SetTableOption`, et les refus qui laissent le deck intact ;
  - `set_chart_option` appliqué à un tableau ;
  - `validate`, sur un tableau valide comme invalide ;
  - `ops_schema`.

  `value_tests.rs` gagne ses cas `host(table)`.
- `oku-slides-render`, `tests/tables.rs`, par empreintes de page :
  - un tableau réduit tient dans son cadre : un rectangle opaque qui couvre le
    cadre rend la page identique avec ou sans tableau ;
  - en flux, la boîte d'un tableau haut reste dans la page ;
  - le substitut diffère d'un élément inconnu ;
  - l'en-tête de colonne, l'alignement, le zébrage et les jetons changent la
    page, et `end` égale `auto` sur une colonne de nombres ;
  - les données mal formées ou trop grandes d'un deck donnent le substitut et
    compilent ;
  - le texte balisé (`#panic`, `$x$`…) reste littéral ;
  - un golden pixel.
- `oku-slides-session` :
  - `show_as` dans les deux sens, en une étape annulable ;
  - l'annulation de `set_data` et de `set_table_option` ;
  - le refus d'un tableau trop grand.
- `oku-slides-mcp` : le schéma d'`apply` nomme `set_data` et
  `set_table_option`, et l'outil `show_as` existe.
- UI :
  - `tableDeck`, `ownPanel`, `TablePanel` (options, alignement, grille,
    conversion) ;
  - `TargetPanel` et `useTargetSettings` pour un tableau ;
  - `newElement`, `slideTools` ;
  - le test d'empilement de `ChartPanel`.
