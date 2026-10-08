# Widget tableau — plan d'implémentation (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** Un widget d'élément `table` qui affiche un tableau de données dans une slide. Il réutilise les données, les ops et la grille des graphiques, et se convertit en graphique, ou inversement, par une requête de session.

**Architecture :**
- **Données.** La `TableOption`, dans le module `table` de `oku-slides-chart`, partage `Dataset`, `Cell` et `DataOp` avec les graphiques.
- **Cœur.** Il contrôle et édite le paramètre `host("table")` dans le nouveau module `ops/host.rs`, avec `Op::SetData` (généralisé) et `Op::SetTableOption`.
- **Rendu.** Une passe Rust ramène chaque tableau à sa forme canonique avant Typst, ou le remplace par un substitut. Typst le rend ensuite nativement avec les jetons `component.table`.
- **Session.** La requête `show_as` fait la conversion, sur le modèle de `chart_kind`.
- **Interface.** Elle ajoute un `TablePanel` qui réutilise `ChartDataGrid`.

**Tech Stack :**
- Rust : `serde`, `schemars`, `snafu`, `json-patch` ; tests avec `speculoos`, `insta`, `rstest`, `cargo nextest`.
- Typst : le paquet `typst/oku-slides`.
- UI : React 19 avec Bun (`bun test`, happy-dom pour les `*.render.test.tsx`).
- i18n : Fluent, 20 locales.

**Spec :** `docs/superpowers/specs/2026-10-08-widget-tableau-design.md`, révisé après la revue d'équipe. Ce plan est la v2 et intègre les corrections de cette revue.

## Global Constraints

- **Tâche Oku et commits.** La tâche est **t-578**. Chaque commit porte `(t-578)`, au format conventionnel en français. **Jamais de ligne `Co-Authored-By`.**
- **Point de départ.** Le worktree part de `HEAD` de `main`. Le travail UI non commité sur `main` (styles.css, TargetPanel, ui.ftl) n'en fait pas partie, et ses conflits se résoudront à la fusion finale.
- **Hook de revue.** Dans le worktree SDD : `git config oku.review deferred`. Faire `git add` à part, puis `git diff --cached --name-only` juste avant chaque `git commit`.
- **Chaque commit laisse l'espace de travail vert.** Avant chaque commit, lancer `cargo nextest run --workspace --exclude oku-slides-app` et `cd ui && bun run test`, en plus des tests ciblés.
- **Pas de rétrocompatibilité.** `SetChartData`, `set_chart_data` et `ChartDataOp` disparaissent. Le squash final garde le `!` (`feat!`) ou un pied `BREAKING CHANGE:`.
- **Limites :**
  - `MAX_TABLE_ROWS = 100`, ligne d'en-tête comprise ;
  - `MAX_TABLE_COLUMNS = 20` ;
  - `MAX_TABLE_CELL_CHARS = 1000`.
- **Valeurs par défaut :** `header-row: true`, `header-column: false`, `striped: false`, `align: []`. Une colonne sans entrée vaut `auto`.
- **Noms JSON en kebab-case :** `dataset`, `header-row`, `header-column`, `striped`, `align`. `ColumnAlign` vaut `auto`, `start`, `center` ou `end`. Le widget s'appelle `table` et son paramètre `data`.
- **i18n.** Les 20 locales (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`) gardent les mêmes clés, **dans le même ordre**. Chaque clé ajoutée en `en`/`fr` est traduite dans les 18 autres, en la plaçant au même endroit. Ne jamais copier le texte anglais dans une autre langue.
- **Clippy.** La lint `unwrap_used` est en `deny`. Dans les fonctions d'aide de `tests/*.rs` (hors `#[test]`), utiliser `unwrap_or_else(|e| panic!(..))`.
- **Documentation.** La première phrase d'une doc fait moins de 15 mots. Chaque élément public a une doc.
- **Taille des fichiers :**
  - 500 lignes visées, 1000 au maximum ;
  - `crates/oku-slides-core/src/ops/mod.rs` en fait 843 et doit baisser ;
  - `crates/oku-slides-session/src/lib.rs` en fait 969 : on n'y ajoute qu'un bras de `match`.
- **Commandes :**
  - Rust : `cargo nextest run -p <crate>` ;
  - UI : `cd ui && bun test <fichier>`, chaque `*.render.test.tsx` lancé seul ;
  - types UI : `cd ui && bun run typecheck`. Ce contrôle ne couvre pas les tests, que `bun test` vérifie ;
  - contrôle complet : `just check && just test`.
- **Données de l'app.** Lancer l'app à la main avec `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` pointés vers un dossier temporaire du scratchpad.

---

### Task 1 : Renommer l'op de données — `DataOp`, `Op::SetData`

Refactorisation : le comportement ne change pas. Les assertions qui citent l'ancien nom changent avec lui.

**Files:**
- Modify (crate des graphiques) :
  - `crates/oku-slides-chart/src/data_ops.rs` : l'enum et la doc l. 1 ;
  - `lib.rs` : l. 6-7 et 27 ;
  - `schema.rs` : l. 6 et 16 ;
  - `error.rs` : l. 23 (doc) et l. 40 (display) ;
  - `data_ops_tests.rs` et `canonical_tests.rs` : remplacer `ChartDataOp` ;
  - `patch_tests.rs` : l. 50, `"utiliser set_chart_data"` devient `"utiliser set_data"`.
- Modify (cœur) :
  - `crates/oku-slides-core/src/ops/mod.rs` : import l. 22, variante l. 151-156, doc l. 451, bras l. 561-567 ;
  - `crates/oku-slides-core/tests/charts.rs` :
    - le type `ChartDataOp` et `Op::SetChartData` ;
    - la fonction `data()` l. 51-53 ;
    - les noms de tests `set_chart_data_*` des l. 81, 89, 102, 112, 186 et 195, qui deviennent `set_data_*` ;
    - l'assertion l. 207 `contains("set_chart_data")`, qui devient `contains("set_data")`.
- Modify (session et MCP) :
  - `crates/oku-slides-session/tests/errors.rs:207` ;
  - `crates/oku-slides-mcp/src/tools.rs:201` ;
  - `crates/oku-slides-mcp/tests/edit.rs:53`.
- Modify (i18n) : `crates/oku-slides-i18n/src/locales/*/chart.ftl` (20 fichiers), l. 2 et 39.
- Modify (UI) :
  - `ui/src/chart/chartTypes.ts` ;
  - `ui/src/chart/ChartDataGrid.tsx` : l. 3, 15 et 26 ;
  - `ui/src/chart/ChartPanel.tsx:72`.

**Interfaces :**
- Produces :
  - `oku_slides_chart::DataOp`, avec les variantes inchangées ;
  - `Op::SetData { target: Target, param: String, change: DataOp }`, en JSON `"op": "set_data"` ;
  - le type TypeScript `DataOp`.

- [ ] **Step 1 : adapter d'abord les tests et constater l'échec.**
  - Renommer les tests `set_chart_data_*` de `core/tests/charts.rs` en `set_data_*`.
  - Changer l'assertion l. 207 en `assert_that!(schema).contains("set_data");`.
  - Changer `patch_tests.rs:50` en `"utiliser set_data"`.
  - Dans `mcp/tests/edit.rs:53`, la liste devient `["set_param", "set_chart_option", "set_data", "add_slide", "set_theme"]`.

  Lancer :

  ```bash
  cargo nextest run -p oku-slides-chart -p oku-slides-core -p oku-slides-mcp
  ```

  Résultat attendu : échec (FAIL) des trois assertions de nom, ou une erreur de compilation sur `ChartDataOp`, selon l'ordre des remplacements.

- [ ] **Step 2 : renommer dans le crate des graphiques.**
  - `data_ops.rs` :
    - `pub enum ChartDataOp` devient `pub enum DataOp`, ainsi que toutes les occurrences `ChartDataOp::` et le type de retour de `apply` ;
    - la ligne 1 devient : `//! \`set_data\` : modifications de \`dataset.source\` (graphiques et tableaux), chacune avec son` ;
    - `chart_data_op_tagged_json_deserializes` devient `data_op_tagged_json_deserializes`.
  - `lib.rs` :
    - `pub use data_ops::DataOp;` ;
    - la doc l. 6-7 devient ``//! 3. les éditions passent par [`apply_option_patch`] (`set_chart_option`)`` puis ``//!    et [`DataOp`] (`set_data`), qui renvoient leur inverse ;``.
  - `schema.rs` : `use crate::data_ops::DataOp;` et `schema_for!(DataOp)`.
  - `error.rs` :
    - l. 23 : `/// Le patch touche \`dataset\` : il faut passer par \`set_data\`.` ;
    - l. 40 : `#[snafu(display("an option patch cannot change \`dataset\`: use set_data"))]`.
  - Les fichiers `*_tests.rs` : remplacer `ChartDataOp` par `DataOp`.

- [ ] **Step 3 : renommer dans le cœur.** Dans `ops/mod.rs` :
  - l'import l. 22 devient `use oku_slides_chart::{ChartOption, DataOp, apply_option_patch, sync_series_with_columns};` ;
  - la variante devient :

    ```rust
        /// Modifie les données d'un graphique ou d'un tableau.
        ///
        /// `dataset.source`, ligne 0 = noms de colonnes ; `param` est de type `host`.
        SetData {
            target: Target,
            param: String,
            change: DataOp,
        },
    ```

  - le bras devient `Op::SetData { target, param, change } =>`, avec un corps inchangé ;
  - la doc l. 451 devient `/// \`set_data\` sur les options \`option\`, rendues modifiées.`, avec la signature `fn chart_data(mut option: ChartOption, change: DataOp, subject: &str)`.

  Dans `tests/charts.rs` : `use oku_slides_chart::{Cell, DataOp, ChartOption};` et `fn data(change: DataOp) -> Op { Op::SetData { target: g(), param: "option".into(), change } }`.

- [ ] **Step 4 : session, MCP et i18n.**
  - `session/tests/errors.rs:207` : `"op": "set_data"`.
  - `mcp/src/tools.rs:201` : remplacer `set_chart_option and set_chart_data change a chart's option or data, addressed by target and param.` par `set_chart_option changes a chart's option; set_data changes the data of a chart or a table (dataset.source, row 0 = column names), addressed by target and param.`.
  - Dans les 20 `chart.ftl`, l. 2 (`chart-patch-dataset`) et l. 39 (`chart-patch-inline-data`) : remplacer le littéral `set_chart_data` par `set_data`, sans rien changer d'autre.

- [ ] **Step 5 : UI.** Dans `chartTypes.ts` :

  ```ts
  /** Opération `set_data` (enum Rust DataOp) : données d'un graphique ou d'un tableau. */
  export type DataOp =
    | { op: "set_cell"; row: number; col: number; value: Cell }
    | { op: "insert_row"; at: number; cells: Cell[] }
    | { op: "remove_row"; at: number }
    | { op: "insert_column"; at: number; cells: Cell[] }
    | { op: "remove_column"; at: number }
    | { op: "replace"; source: Cell[][] };
  ```

  - `ChartDataGrid.tsx` : remplacer les trois `ChartDataOp`, l. 3, 15 et 26, par `DataOp`.
  - `ChartPanel.tsx:72` : `op: "set_chart_data"` devient `op: "set_data"`.

- [ ] **Step 6 : vérifier.**

  ```bash
  git grep -n -E "set_chart_data|SetChartData|ChartDataOp" -- ':!docs/superpowers'
  ```

  Résultat attendu : aucune sortie.

  ```bash
  cargo nextest run --workspace --exclude oku-slides-app
  cd ui && bun run test && bun run typecheck
  ```

  Résultat attendu : tout passe (PASS).

- [ ] **Step 7 : commit.**

  ```bash
  git add -A crates/oku-slides-chart crates/oku-slides-core crates/oku-slides-session crates/oku-slides-mcp crates/oku-slides-i18n ui/src/chart
  git diff --cached --name-only
  git commit -m "refactor! : l'op de données devient DataOp et set_data, pour les graphiques comme pour les tableaux (t-578)"
  ```

---

### Task 2 : Modèle du tableau et conversion — `oku-slides-chart`

**Files:**
- Create :
  - `crates/oku-slides-chart/src/table.rs` ;
  - `crates/oku-slides-chart/src/table_tests.rs` ;
  - `crates/oku-slides-chart/src/table_convert.rs` ;
  - `crates/oku-slides-chart/src/table_convert_tests.rs` ;
  - `crates/oku-slides-chart/ARCHITECTURE.md`.
- Modify :
  - `crates/oku-slides-chart/src/patch.rs` : le helper `merge_patch` ;
  - `crates/oku-slides-chart/src/data_ops.rs` : `chart_data`, déplacé depuis le cœur ;
  - `crates/oku-slides-chart/src/error.rs` : l'affichage de `Inconsistent` ;
  - `crates/oku-slides-chart/src/lib.rs` ;
  - `crates/oku-slides-chart/Cargo.toml:3` : la description ;
  - `crates/oku-slides-i18n/src/locales/*/chart.ftl` (20 fichiers).

**Interfaces :**
- Consumes : `DataOp` (tâche 1), `Dataset`, `Cell` et `ChartOption` (`option.rs`), `sync_series_with_columns` et `normalize`.
- Produces (réexportés à la racine du crate) :
  - les types et constantes :
    - `TableOption { dataset, header_row, header_column, striped, align }` ;
    - `ColumnAlign { Auto, Start, Center, End }` ;
    - `MAX_TABLE_ROWS`, `MAX_TABLE_COLUMNS` et `MAX_TABLE_CELL_CHARS`.
  - les fonctions :
    - `parse_table(&Value) -> Result<TableOption>` ;
    - `table_data(&TableOption, &DataOp) -> Result<TableOption>` ;
    - `patch_table(&TableOption, &Value) -> Result<(TableOption, Value)>` ;
    - `chart_data(&ChartOption, &DataOp) -> Result<ChartOption>` ;
    - `chart_to_table(&ChartOption) -> Result<TableOption>` ;
    - `table_to_chart(&TableOption, &ChartLabels) -> Result<ChartOption>`.

  `validate_table` reste `pub(crate)`.

- [ ] **Step 1 : squelette compilable.**
  - Ajouter `mod table;` et `mod table_convert;` dans `lib.rs`.
  - Créer `table.rs` et `table_convert.rs`. Les signatures publiques de l'interface ci-dessus y renvoient `todo!()`, et les types y sont déjà complets (voir l'étape 3).
  - Ajouter les deux réexports à `lib.rs`.

  Le squelette n'existe qu'entre les étapes 1 et 3, et n'est jamais commité.

- [ ] **Step 2 : écrire les tests qui échouent.** Fichier `table_tests.rs` :

  ```rust
  use oku_slides_i18n::{Localize, Localizer};
  use serde_json::json;
  use speculoos::prelude::*;

  use super::*;
  use crate::{Cell, ChartErrorKind, DataOp};

  fn table() -> TableOption {
      parse_table(&json!({
          "dataset": { "source": [["Pays", "Ventes", "Part"], ["FR", 10, 0.4], ["DE", 15, 0.6]] },
          "align": ["start", "auto", "end"]
      }))
      .unwrap_or_else(|e| panic!("{e}"))
  }

  fn kind_of(v: serde_json::Value) -> ChartErrorKind {
      parse_table(&v).map(|_| ()).unwrap_err().kind()
  }

  #[test]
  fn parse_table_without_options_uses_defaults() {
      let t = parse_table(&json!({ "dataset": { "source": [["A"]] } })).unwrap();
      assert_that!((t.header_row, t.header_column, t.striped, t.align.len())).is_equal_to((true, false, false, 0));
  }

  #[test]
  fn parse_table_with_kebab_case_options_reads_them() {
      let t = parse_table(&json!({ "dataset": { "source": [["A"]] }, "header-row": false,
          "header-column": true, "striped": true, "align": ["center"] }))
      .unwrap();
      assert_that!((t.header_row, t.header_column, t.striped, t.align))
          .is_equal_to((false, true, true, vec![ColumnAlign::Center]));
  }

  #[test]
  fn parse_table_unknown_field_is_invalid_option() {
      assert_that!(kind_of(json!({ "dataset": { "source": [["A"]] }, "series": [] })))
          .is_equal_to(ChartErrorKind::InvalidOption);
  }

  #[test]
  fn parse_table_empty_source_is_inconsistent() {
      assert_that!(kind_of(json!({ "dataset": { "source": [] } }))).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_empty_header_row_is_inconsistent() {
      assert_that!(kind_of(json!({ "dataset": { "source": [[]] } }))).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_ragged_rows_are_inconsistent() {
      assert_that!(kind_of(json!({ "dataset": { "source": [["A", "B"], [1]] } })))
          .is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_at_max_columns_is_accepted() {
      let header: Vec<String> = (0..MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
      assert_that!(parse_table(&json!({ "dataset": { "source": [header] } }))).is_ok();
  }

  #[test]
  fn parse_table_beyond_max_columns_is_inconsistent() {
      let header: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
      assert_that!(kind_of(json!({ "dataset": { "source": [header] } }))).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_beyond_max_rows_is_inconsistent() {
      let rows: Vec<Vec<usize>> = (0..=MAX_TABLE_ROWS).map(|i| vec![i]).collect();
      assert_that!(kind_of(json!({ "dataset": { "source": rows } }))).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_cell_text_too_long_is_inconsistent() {
      let long = "x".repeat(MAX_TABLE_CELL_CHARS + 1);
      assert_that!(kind_of(json!({ "dataset": { "source": [[long]] } }))).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_align_longer_than_columns_is_inconsistent() {
      assert_that!(kind_of(json!({ "dataset": { "source": [["A"]] }, "align": ["start", "end"] })))
          .is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn parse_table_ragged_reason_is_localized_in_french() {
      let err = parse_table(&json!({ "dataset": { "source": [["A", "B"], [1]] } })).unwrap_err();
      assert_that!(err.localize(&Localizer::new(Some("fr")))).contains("même nombre de cellules");
  }

  #[test]
  fn validate_table_non_finite_number_is_inconsistent() {
      let mut t = table();
      t.dataset.source[1][1] = Cell::Number(f64::INFINITY);
      assert_that!(validate_table(&t).unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn table_data_insert_column_at_zero_shifts_align() {
      let next = table_data(&table(), &DataOp::InsertColumn { at: 0, cells: vec![] }).unwrap();
      assert_that!(next.align)
          .is_equal_to(vec![ColumnAlign::Auto, ColumnAlign::Start, ColumnAlign::Auto, ColumnAlign::End]);
  }

  #[test]
  fn table_data_insert_column_at_align_len_leaves_align() {
      let mut t = table();
      t.align = vec![ColumnAlign::End];
      let next = table_data(&t, &DataOp::InsertColumn { at: 1, cells: vec![] }).unwrap();
      assert_that!(next.align).is_equal_to(vec![ColumnAlign::End]);
  }

  #[test]
  fn table_data_remove_column_drops_its_alignment() {
      let next = table_data(&table(), &DataOp::RemoveColumn { at: 0 }).unwrap();
      assert_that!(next.align).is_equal_to(vec![ColumnAlign::Auto, ColumnAlign::End]);
  }

  #[test]
  fn table_data_remove_column_beyond_align_leaves_it() {
      let mut t = table();
      t.align = vec![ColumnAlign::End];
      let next = table_data(&t, &DataOp::RemoveColumn { at: 2 }).unwrap();
      assert_that!(next.align).is_equal_to(vec![ColumnAlign::End]);
  }

  #[test]
  fn table_data_replace_with_fewer_columns_truncates_align() {
      let next = table_data(&table(), &DataOp::Replace { source: vec![vec![Cell::Text("x".into())]] }).unwrap();
      assert_that!(next.align).is_equal_to(vec![ColumnAlign::Start]);
  }

  #[test]
  fn table_data_set_cell_changes_only_that_cell() {
      let mut expected = table();
      expected.dataset.source[1][1] = Cell::Number(99.0);
      let next = table_data(&table(), &DataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) }).unwrap();
      assert_that!(next).is_equal_to(expected);
  }

  #[test]
  fn table_data_insert_row_at_max_rows_is_inconsistent() {
      let rows: Vec<Vec<usize>> = (0..MAX_TABLE_ROWS).map(|i| vec![i]).collect();
      let t = parse_table(&json!({ "dataset": { "source": rows } })).unwrap();
      let r = table_data(&t, &DataOp::InsertRow { at: 1, cells: vec![] });
      assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn table_data_insert_column_at_max_columns_is_inconsistent() {
      let header: Vec<String> = (0..MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
      let t = parse_table(&json!({ "dataset": { "source": [header] } })).unwrap();
      let r = table_data(&t, &DataOp::InsertColumn { at: 0, cells: vec![] });
      assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn table_data_replace_beyond_max_rows_is_inconsistent() {
      let source = vec![vec![Cell::Null]; MAX_TABLE_ROWS + 1];
      let r = table_data(&table(), &DataOp::Replace { source });
      assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn patch_table_then_inverse_restores_original() {
      let original = table();
      let (next, inverse) = patch_table(&original, &json!({ "striped": true, "align": null })).unwrap();
      assert_that!((next.striped, next.align.len())).is_equal_to((true, 0));
      let (restored, _) = patch_table(&next, &inverse).unwrap();
      assert_that!(restored).is_equal_to(original);
  }

  #[test]
  fn patch_table_touching_dataset_is_refused() {
      let r = patch_table(&table(), &json!({ "dataset": null }));
      assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::PatchTouchesDataset);
  }

  #[test]
  fn patch_table_unknown_alignment_is_invalid_option() {
      let r = patch_table(&table(), &json!({ "align": ["left"] }));
      assert_that!(r.unwrap_err().kind()).is_equal_to(ChartErrorKind::InvalidOption);
  }

  #[test]
  fn table_option_serializes_field_names_in_kebab_case() {
      let v = serde_json::to_value(table()).unwrap();
      assert_that!(v.get("header-row")).is_equal_to(Some(&json!(true)));
  }

  #[test]
  fn column_align_serializes_in_lowercase() {
      assert_that!(serde_json::to_value(table().align).unwrap()).is_equal_to(json!(["start", "auto", "end"]));
  }
  ```

  Fichier `table_convert_tests.rs` :

  ```rust
  use serde_json::json;
  use speculoos::prelude::*;

  use super::*;
  use crate::{Cell, ChartErrorKind, ChartLabels, ChartOption, ColumnAlign, MAX_TABLE_COLUMNS, parse_table};

  fn chart(v: serde_json::Value) -> ChartOption {
      serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
  }

  #[test]
  fn chart_to_table_keeps_the_dataset_with_header_column() {
      let c = chart(json!({ "dataset": { "source": [["M", "A"], ["x", 1]] }, "xAxis": { "type": "category" },
          "series": [{ "type": "bar" }] }));
      let t = chart_to_table(&c).unwrap();
      assert_that!((t.dataset.source.len(), t.header_row, t.header_column)).is_equal_to((2, true, true));
  }

  #[test]
  fn chart_to_table_pads_short_rows() {
      let c = chart(json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1]] }, "xAxis": { "type": "category" },
          "series": [{ "type": "bar" }, { "type": "bar" }] }));
      assert_that!(chart_to_table(&c).unwrap().dataset.source[1][2]).is_equal_to(Cell::Null);
  }

  #[test]
  fn chart_to_table_without_dataset_is_a_data_op_error() {
      let c = chart(json!({ "xAxis": { "type": "category", "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] }));
      assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::DataOp);
  }

  #[test]
  fn chart_to_table_beyond_max_columns_is_inconsistent() {
      let header: Vec<String> = (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect();
      let series: Vec<_> = (1..=MAX_TABLE_COLUMNS).map(|_| json!({ "type": "bar" })).collect();
      let c = chart(json!({ "dataset": { "source": [header] }, "xAxis": { "type": "category" }, "series": series }));
      assert_that!(chart_to_table(&c).unwrap_err().kind()).is_equal_to(ChartErrorKind::Inconsistent);
  }

  #[test]
  fn table_to_chart_gets_one_bar_series_per_value_column() {
      let t = parse_table(&json!({ "dataset": { "source": [["M", "A", "B"], ["x", 1, 2]] } })).unwrap();
      let c = table_to_chart(&t, &ChartLabels::english()).unwrap();
      assert_that!(c.series.len()).is_equal_to(2);
  }

  #[test]
  fn table_to_chart_with_text_values_is_refused() {
      let t = parse_table(&json!({ "dataset": { "source": [["M", "A"], ["x", "beaucoup"]] },
          "align": [ColumnAlign::Start] }))
      .unwrap();
      assert_that!(table_to_chart(&t, &ChartLabels::english())).is_err();
  }
  ```

  Remarque : avant d'écrire le dernier test, vérifier dans `model.rs` (`normalize`) qu'une cellule de texte dans une colonne de valeurs fait bien échouer la normalisation. Si elle est seulement ignorée, comme un `null`, ce test devient `table_to_chart_with_text_values_keeps_them_as_missing` et vérifie `is_ok()`. La conversion n'est pas refusée dans ce cas, et la note du bouton en tâche 7 tombe.

- [ ] **Step 3 : lancer les tests et constater l'échec.**

  ```bash
  cargo nextest run -p oku-slides-chart table
  ```

  Résultat attendu : FAIL. Les tests paniquent sur `todo!()`.

- [ ] **Step 4 : partager le merge patch.** Dans `patch.rs`, ajouter le helper suivant, puis réécrire `apply_option_patch` avec : `let (next, inverse): (ChartOption, Value) = merge_patch(option, patch)?;`. Le reste ne change pas : le contrôle des données en ligne, `normalize` et `Ok((next, inverse))`.

  ```rust
  /// Merge patch sur `option` hors `dataset` ; rend le résultat lu et le patch inverse.
  pub(crate) fn merge_patch<T: Serialize + DeserializeOwned>(option: &T, patch: &Value) -> Result<(T, Value)> {
      ensure!(patch.get("dataset").is_none(), PatchTouchesDatasetSnafu);
      ensure!(patch.is_object(), InvalidPatchSnafu { reason: Message::new("chart-patch-object") });
      let before = serde_json::to_value(option).context(InvalidOptionSnafu)?;
      let inverse = reverse_merge_patch(&before, patch);
      let mut after = before;
      json_patch::merge(&mut after, patch);
      let next = serde_json::from_value(after).context(InvalidOptionSnafu)?;
      Ok((next, inverse))
  }
  ```

  Ajouter les imports `serde::{Serialize, de::DeserializeOwned}`.

- [ ] **Step 5 : `chart_data` rejoint le crate.** Dans `data_ops.rs`, après `impl DataOp` :

  ```rust
  /// `set_data` sur un graphique : applique `change`, recale les séries, normalise.
  pub fn chart_data(option: &ChartOption, change: &DataOp) -> Result<ChartOption> {
      let mut next = option.clone();
      let dataset = next.dataset.as_mut().ok_or_else(|| err(Message::new("chart-data-no-dataset")))?;
      change.apply(dataset)?;
      sync_series_with_columns(&mut next);
      normalize(&next, ChartLabels::english())?;
      Ok(next)
  }
  ```

  Ajouter les imports `crate::{canonical::sync_series_with_columns, labels::ChartLabels, model::normalize, option::ChartOption}`, et `chart_data` dans les réexports de `lib.rs`. Le cœur l'utilisera à la tâche 3.

- [ ] **Step 6 : implémenter `table.rs`.**

  ```rust
  //! Tableaux : options d'un widget `host(table)`, données éditées comme celles d'un graphique.
  //!
  //! Le tableau partage [`Dataset`] et [`DataOp`] avec les graphiques : la
  //! ligne 0 est celle des noms de colonnes ; `header-row` décide seulement
  //! de son affichage en en-tête.

  use oku_slides_i18n::Message;
  use schemars::JsonSchema;
  use serde::{Deserialize, Serialize};
  use serde_json::Value;
  use snafu::{ResultExt, ensure};

  use crate::data_ops::DataOp;
  use crate::error::{InconsistentSnafu, InvalidOptionSnafu, Result};
  use crate::option::{Cell, Dataset};
  use crate::patch::merge_patch;

  /// Lignes au plus, ligne d'en-tête comprise.
  pub const MAX_TABLE_ROWS: usize = 100;
  /// Colonnes au plus.
  pub const MAX_TABLE_COLUMNS: usize = 20;
  /// Caractères au plus dans une cellule de texte.
  pub const MAX_TABLE_CELL_CHARS: usize = 1000;

  /// Options d'un tableau : ses données et leur affichage.
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
  #[serde(rename_all = "kebab-case", deny_unknown_fields)]
  pub struct TableOption {
      /// Données ; ligne 0 = noms de colonnes.
      pub dataset: Dataset,
      /// Ligne 0 affichée en en-tête.
      #[serde(default = "default_true")]
      pub header_row: bool,
      /// Colonne 0 affichée en en-tête (libellés des lignes).
      #[serde(default)]
      pub header_column: bool,
      /// Une ligne de données sur deux sur fond.
      #[serde(default)]
      pub striped: bool,
      /// Alignement de chaque colonne ; une colonne sans entrée vaut `auto`.
      #[serde(default, skip_serializing_if = "Vec::is_empty")]
      pub align: Vec<ColumnAlign>,
  }

  fn default_true() -> bool {
      true
  }

  /// Alignement d'une colonne de tableau.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
  #[serde(rename_all = "lowercase")]
  pub enum ColumnAlign {
      /// À la fin pour une colonne de nombres, au début sinon.
      Auto,
      /// Au début de la ligne (à gauche en français).
      Start,
      /// Centré.
      Center,
      /// À la fin de la ligne (à droite en français).
      End,
  }

  /// Lit et valide les options d'un tableau.
  pub fn parse_table(v: &Value) -> Result<TableOption> {
      let option = TableOption::deserialize(v).context(InvalidOptionSnafu)?;
      validate_table(&option)?;
      Ok(option)
  }

  fn too_large() -> Message {
      Message::new("chart-table-too-large").arg("rows", MAX_TABLE_ROWS).arg("columns", MAX_TABLE_COLUMNS)
  }

  fn text_too_long(c: &Cell) -> bool {
      matches!(c, Cell::Text(s) if s.chars().count() > MAX_TABLE_CELL_CHARS)
  }

  /// Refuse un tableau trop grand, vide, irrégulier ou mal aligné.
  pub(crate) fn validate_table(option: &TableOption) -> Result<()> {
      let rows = &option.dataset.source;
      let width = rows.first().map_or(0, Vec::len);
      // Tailles d'abord : un tableau énorme est refusé sans être parcouru.
      ensure!(rows.len() <= MAX_TABLE_ROWS && width <= MAX_TABLE_COLUMNS, InconsistentSnafu { reason: too_large() });
      ensure!(width > 0, InconsistentSnafu { reason: Message::new("chart-table-empty") });
      ensure!(rows.iter().all(|r| r.len() == width), InconsistentSnafu { reason: Message::new("chart-table-ragged") });
      ensure!(
          !rows.iter().flatten().any(text_too_long),
          InconsistentSnafu { reason: Message::new("chart-table-cell-too-long").arg("max", MAX_TABLE_CELL_CHARS) }
      );
      ensure!(
          rows.iter().flatten().all(|c| !matches!(c, Cell::Number(n) if !n.is_finite())),
          InconsistentSnafu { reason: Message::new("chart-table-not-finite") }
      );
      ensure!(option.align.len() <= width, InconsistentSnafu { reason: Message::new("chart-table-align").arg("width", width) });
      Ok(())
  }

  /// Refuse, avant tout clonage, une op qui ferait dépasser les limites.
  fn check_growth(option: &TableOption, change: &DataOp) -> Result<()> {
      let rows = option.dataset.source.len();
      let width = option.dataset.source.first().map_or(0, Vec::len);
      let over = match change {
          DataOp::InsertRow { .. } => rows >= MAX_TABLE_ROWS,
          DataOp::InsertColumn { .. } => width >= MAX_TABLE_COLUMNS,
          DataOp::Replace { source } => {
              source.len() > MAX_TABLE_ROWS || source.iter().any(|r| r.len() > MAX_TABLE_COLUMNS)
          }
          _ => false,
      };
      ensure!(!over, InconsistentSnafu { reason: too_large() });
      Ok(())
  }

  /// `set_data` sur un tableau : applique `change`, recale `align`, valide.
  pub fn table_data(option: &TableOption, change: &DataOp) -> Result<TableOption> {
      check_growth(option, change)?;
      let mut next = option.clone();
      change.apply(&mut next.dataset)?;
      match change {
          DataOp::InsertColumn { at, .. } if *at < next.align.len() => next.align.insert(*at, ColumnAlign::Auto),
          DataOp::RemoveColumn { at } if *at < next.align.len() => {
              next.align.remove(*at);
          }
          DataOp::Replace { source } => next.align.truncate(source.first().map_or(0, Vec::len)),
          _ => {}
      }
      validate_table(&next)?;
      Ok(next)
  }

  /// Merge patch (RFC 7396) sur les options hors `dataset` ; rend le patch inverse.
  pub fn patch_table(option: &TableOption, patch: &Value) -> Result<(TableOption, Value)> {
      let (next, inverse) = merge_patch(option, patch)?;
      validate_table(&next)?;
      Ok((next, inverse))
  }

  #[cfg(test)]
  #[path = "table_tests.rs"]
  mod table_tests;
  ```

  Remarque sur `DataOp::apply` : un `InsertRow` dont `cells` est plus long que la largeur est tronqué par `resize`. Un `InsertColumn` voit `cells.len()` limité au nombre de lignes. Pour `Replace`, `check_growth` borne la taille avant l'appel à `apply`.

- [ ] **Step 7 : implémenter `table_convert.rs`.**

  ```rust
  //! Graphique ↔ tableau : les mêmes données, l'autre widget.

  use oku_slides_i18n::Message;
  use serde_json::json;
  use snafu::ResultExt;

  use crate::canonical::sync_series_with_columns;
  use crate::error::{DataOpSnafu, InvalidOptionSnafu, Result};
  use crate::labels::ChartLabels;
  use crate::model::normalize;
  use crate::option::{Cell, ChartOption, Dataset};
  use crate::table::{TableOption, validate_table};

  /// Le tableau des données d'un graphique, lignes courtes complétées.
  pub fn chart_to_table(chart: &ChartOption) -> Result<TableOption> {
      let dataset = chart
          .dataset
          .as_ref()
          .ok_or_else(|| DataOpSnafu { reason: Message::new("chart-data-no-dataset") }.build())?;
      let width = dataset.source.iter().map(Vec::len).max().unwrap_or(0);
      let source = dataset
          .source
          .iter()
          .map(|r| {
              let mut r = r.clone();
              r.resize(width, Cell::Null);
              r
          })
          .collect();
      let table = TableOption {
          dataset: Dataset { source },
          header_row: true,
          header_column: true,
          striped: false,
          align: Vec::new(),
      };
      validate_table(&table)?;
      Ok(table)
  }

  /// Le graphique en barres d'un tableau : une série par colonne de valeurs.
  pub fn table_to_chart(table: &TableOption, labels: &ChartLabels) -> Result<ChartOption> {
      let v = json!({ "dataset": table.dataset, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
      let mut chart: ChartOption = serde_json::from_value(v).context(InvalidOptionSnafu)?;
      sync_series_with_columns(&mut chart);
      normalize(&chart, labels)?;
      Ok(chart)
  }

  #[cfg(test)]
  #[path = "table_convert_tests.rs"]
  mod table_convert_tests;
  ```

  Remarques :
  - Si `DataOpSnafu { .. }.build()` renvoie un `ChartInner` et non un `ChartError`, terminer par `.into()`, comme le fait `err()` dans `data_ops.rs:44-46`.
  - Si `Dataset` n'implémente pas `Serialize` dans `json!`, c'est qu'il le fait déjà ailleurs : il le dérive, `option.rs:69`.

- [ ] **Step 8 : erreurs, i18n, docs.**
  - `error.rs:36` : `#[snafu(display("inconsistent data: {reason}"))]`. Dans la doc de `ChartError` (l. 9-10), écrire `/// Un graphique ou un tableau refusé, ou un graphique impossible à rendre.`.
  - `Cargo.toml:3` : `description = "Graphiques et tableaux : sous-ensemble des options ECharts, rendu SVG par charts-rs, modèle des tableaux, opérations sur les données."`.
  - `lib.rs`, ligne 1 : `//! Graphiques et tableaux d'Oku Slides : options ECharts → SVG, tableaux → Typst.`. Puis ajouter `//! Un tableau ([\`TableOption\`]) partage \`dataset\` et [\`DataOp\`] avec les graphiques ;` et `//! [\`chart_to_table\`] / [\`table_to_chart\`] passent de l'un à l'autre.`. Réexports :

    ```rust
    pub use data_ops::{DataOp, chart_data};
    pub use table::{ColumnAlign, MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, TableOption, parse_table, patch_table, table_data};
    pub use table_convert::{chart_to_table, table_to_chart};
    ```

  - `ARCHITECTURE.md`, nouveau fichier de moins de 50 lignes, avec ces sections :
    - **Rôle** : graphiques (options ECharts → SVG) et tableaux (modèle seul, rendu par Typst).
    - **Chemin d'un graphique** : `parse_option` → `canonicalize` → `apply_option_patch` / `chart_data` → `ChartRenderer`.
    - **Tableau** : `parse_table`, `table_data`, `patch_table`, limites ; `chart_to_table` / `table_to_chart`.
    - **Erreurs** : `ChartError` couvre les deux, avec les clés `chart-*` et `chart-table-*`. Ne pas créer de `TableError`.
    - **Points d'extension** : un type de graphique (`SeriesType`, `model.rs`), une option de tableau (`TableOption`, `validate_table`, et le widget Typst `_table`).
    - **Dépendances** : `oku-slides-i18n`, `charts-rs`, `json-patch`, `serde`, `schemars`, `snafu`. Aucune dépendance à Typst ni à Tauri.
  - `chart.ftl`, à la fin de chaque fichier, dans cet ordre.

    en :

    ```
    chart-data-no-dataset = the chart has no data table (dataset)
    chart-table-empty = the table needs at least one row and one column
    chart-table-ragged = every row of the table must have the same number of cells
    chart-table-too-large = a table has at most { $rows } rows and { $columns } columns
    chart-table-cell-too-long = a table cell holds at most { $max } characters
    chart-table-not-finite = table numbers must be finite
    chart-table-align = align has more entries than the table has columns ({ $width })
    ```

    fr :

    ```
    chart-data-no-dataset = le graphique n'a pas de tableau de données (dataset)
    chart-table-empty = le tableau doit avoir au moins une ligne et une colonne
    chart-table-ragged = toutes les lignes du tableau doivent avoir le même nombre de cellules
    chart-table-too-large = un tableau a au plus { $rows } lignes et { $columns } colonnes
    chart-table-cell-too-long = une cellule de tableau contient au plus { $max } caractères
    chart-table-not-finite = les nombres du tableau doivent être finis
    chart-table-align = align a plus d'entrées que le tableau n'a de colonnes ({ $width })
    ```

    Traduire ces 7 clés dans les 18 autres locales, sans traduire les noms JSON `align` et `dataset`.

- [ ] **Step 9 : lancer les tests et constater qu'ils passent.**

  ```bash
  cargo nextest run -p oku-slides-chart -p oku-slides-i18n
  cargo clippy -p oku-slides-chart --all-targets -- -D warnings
  cargo nextest run --workspace --exclude oku-slides-app
  ```

  Résultat attendu : PASS. `patch_tests.rs` reste vert avec `merge_patch`, et le cœur compile toujours : il garde son propre `chart_data` jusqu'à la tâche 3.

- [ ] **Step 10 : commit.**

  ```bash
  git add crates/oku-slides-chart crates/oku-slides-i18n
  git diff --cached --name-only
  git commit -m "feat(chart) : modèle des tableaux, conversion graphique ↔ tableau, données partagées avec les graphiques (t-578)"
  ```

---

### Task 3 : Cœur — `host(table)`, `ops/host.rs`, `SetData` générique, `SetTableOption`

Les tests du cœur passent par un registre de test qui déclare `table`. La fixture partagée et le widget Typst arriveront ensemble à la tâche 4, pour que ce commit reste vert.

**Files:**
- Create :
  - `crates/oku-slides-core/src/ops/host.rs` ;
  - `crates/oku-slides-core/tests/tables.rs`.
- Modify :
  - `crates/oku-slides-core/src/widget.rs` : `HostType::Table`, `ParamType::host()`, `WidgetSchema::host_param()` ;
  - `crates/oku-slides-core/src/widget_tests.rs` ;
  - `crates/oku-slides-core/src/ops/mod.rs` : variante `SetTableOption`, bras d'application, retrait des helpers de graphique (l. 340-342 et 402-459) ;
  - `crates/oku-slides-core/src/ops/value.rs` : le bras `Host` appelle `host::host_option` ; `chart_option` disparaît ;
  - `crates/oku-slides-core/src/ops/value_tests.rs:74` : les cas `host(table)` ;
  - `crates/oku-slides-core/src/ops/check.rs:64` ;
  - `crates/oku-slides-core/src/validate.rs:295-298` ;
  - `crates/oku-slides-core/src/error.rs` ;
  - `crates/oku-slides-core/ARCHITECTURE.md` ;
  - `crates/oku-slides-core/tests/common/mod.rs` : `table_registry()` ;
  - `crates/oku-slides-render/src/charts.rs:300-308` et `crates/oku-slides-session/src/charts.rs:32-38` : passer par `host_param` ;
  - `crates/oku-slides-i18n/src/locales/*/core.ftl` (20 fichiers).

**Interfaces :**
- Consumes (tâche 2) : `TableOption`, `parse_table`, `table_data`, `patch_table` et `chart_data`.
- Produces :
  - `HostType::Table`, sérialisé `"table"` ;
  - `ParamType::host(&self) -> Option<HostType>` ;
  - `WidgetSchema::host_param(&self, host: HostType) -> Option<&ParamSchema>` ;
  - `Op::SetTableOption { target: Target, param: String, patch: Value }`, en JSON `"op": "set_table_option"` ;
  - `Op::SetData`, valable pour les deux hôtes ;
  - `OpErrorKind::InvalidTable` et `op-invalid-table`.
  - Le widget de test (`table_registry`), dont la forme est celle que produira Typst :

    ```json
    {"name":"table","label":null,"category":"element","params":[{"name":"data","label":null,"type":"host","host":"table","default":null,"bind":null}]}
    ```

- [ ] **Step 1 : registre de test et tests qui échouent.**

  Dans `tests/common/mod.rs`, après `list_registry`. L'ajout est local : la fixture n'est modifiée qu'à la tâche 4, où cette fonction disparaît.

  ```rust
  /// Le registre de base, plus le widget `table` (un paramètre `data` de type `host("table")`).
  pub fn table_registry() -> WidgetRegistry {
      let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
      let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
      let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
      let table = serde_json::json!({ "name": "table", "category": "element",
          "params": [{ "name": "data", "type": "host", "host": "table" }] });
      let Some(base) = probe["base"].as_array_mut() else { panic!("sonde : base attendu") };
      base.push(table);
      let probe = serde_json::from_value(probe).unwrap_or_else(|e| panic!("sonde : {e}"));
      WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}"))
  }
  ```

  Fichier `tests/tables.rs` :

  ```rust
  //! Les tableaux dans le deck : ajout, remplacement, options, données, validation.

  mod common;

  use common::table_registry as registry;
  use oku_slides_chart::{Cell, DataOp};
  use oku_slides_core::*;
  use oku_slides_i18n::{Localize, Localizer};
  use serde_json::{Value, json};
  use speculoos::prelude::*;

  fn blocks() -> SlotRef {
      SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() }
  }

  fn t1() -> Target {
      Target { slide: "s".into(), element: Some("t1".into()) }
  }

  fn element(v: Value) -> Element {
      serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
  }

  fn table(id: &str, data: Value) -> Element {
      element(json!({ "id": id, "kind": "table", "params": { "data": data } }))
  }

  fn chart(id: &str) -> Element {
      element(json!({ "id": id, "kind": "chart", "params": { "option": {
          "dataset": { "source": [["M", "V"], ["a", 1]] }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }))
  }

  fn data() -> Value {
      json!({ "dataset": { "source": [["Pays", "Ventes"], ["FR", 10]] } })
  }

  fn ragged() -> Value {
      json!({ "dataset": { "source": [["A", "B"], [1]] } })
  }

  fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
      op.apply(deck, &registry())
  }

  fn empty_deck() -> Deck {
      Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#).unwrap_or_else(|e| panic!("{e}"))
  }

  fn add(deck: &mut Deck, e: Element) -> Result<(), OpError> {
      apply(deck, Op::AddElement { slot: blocks(), index: None, defaults: false, element: e })
  }

  fn deck_with(e: Element) -> Deck {
      let mut deck = empty_deck();
      add(&mut deck, e).unwrap_or_else(|e| panic!("{e}"));
      deck
  }

  fn stored(deck: &Deck) -> Value {
      let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("tableau t1 absent")).element;
      e.params.value("data").unwrap_or_else(|| panic!("data absent")).clone()
  }

  fn kind_of(r: Result<(), OpError>) -> OpErrorKind {
      r.err().unwrap_or_else(|| panic!("refus attendu")).kind()
  }

  #[test]
  fn add_element_valid_table_stores_its_data() {
      let deck = deck_with(table("t1", data()));
      assert_that!(stored(&deck)["dataset"]["source"][1][0]).is_equal_to(json!("FR"));
  }

  #[test]
  fn add_element_ragged_table_is_invalid_table() {
      assert_that!(kind_of(add(&mut empty_deck(), table("t1", ragged())))).is_equal_to(OpErrorKind::InvalidTable);
  }

  #[test]
  fn add_element_ragged_table_inside_a_stack_is_invalid_table() {
      let stack = element(json!({ "id": "p", "kind": "stack", "params": { "children": [
          { "id": "t1", "kind": "table", "params": { "data": ragged() } } ] } }));
      assert_that!(kind_of(add(&mut empty_deck(), stack))).is_equal_to(OpErrorKind::InvalidTable);
  }

  #[test]
  fn add_element_empty_table_localizes_subject_and_reason_in_french() {
      let err = add(&mut empty_deck(), table("t1", json!({ "dataset": { "source": [] } }))).unwrap_err();
      let fr = err.localize(&Localizer::new(Some("fr")));
      assert_that!(fr).starts_with("Tableau t1");
      assert_that!(fr).contains("au moins une ligne");
  }

  #[test]
  fn replace_element_chart_by_table_keeps_the_id() {
      let mut deck = deck_with(chart("t1"));
      apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", data()) }).unwrap();
      let e = deck.slides[0].locate("t1").unwrap_or_else(|| panic!("t1 absent")).element;
      assert_that!(e.kind.as_str()).is_equal_to("table");
  }

  #[test]
  fn replace_element_by_ragged_table_is_invalid_table_and_deck_unchanged() {
      let mut deck = deck_with(chart("t1"));
      let before = deck.clone();
      let r = apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: table("t1", ragged()) });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
      assert_that!(deck).is_equal_to(before);
  }

  #[test]
  fn set_param_ragged_object_is_invalid_table() {
      let mut deck = deck_with(table("t1", data()));
      let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: ragged() });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
  }

  #[test]
  fn set_param_string_is_param_type() {
      let mut deck = deck_with(table("t1", data()));
      let r = apply(&mut deck, Op::SetParam { target: t1(), param: "data".into(), value: json!("texte") });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
  }

  #[test]
  fn set_data_on_a_table_edits_its_cell() {
      let mut deck = deck_with(table("t1", data()));
      let change = DataOp::SetCell { row: 1, col: 1, value: Cell::Number(42.0) };
      apply(&mut deck, Op::SetData { target: t1(), param: "data".into(), change }).unwrap();
      assert_that!(stored(&deck)["dataset"]["source"][1][1]).is_equal_to(json!(42.0));
  }

  #[test]
  fn set_data_on_a_table_beyond_limits_is_invalid_table_and_deck_unchanged() {
      let mut deck = deck_with(table("t1", data()));
      let before = deck.clone();
      let change = DataOp::Replace { source: vec![vec![Cell::Null; 21]] };
      let r = apply(&mut deck, Op::SetData { target: t1(), param: "data".into(), change });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
      assert_that!(deck).is_equal_to(before);
  }

  #[test]
  fn set_table_option_striped_true_stores_it() {
      let mut deck = deck_with(table("t1", data()));
      let patch = json!({ "striped": true });
      apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch }).unwrap();
      assert_that!(stored(&deck)["striped"]).is_equal_to(json!(true));
  }

  #[test]
  fn set_table_option_touching_dataset_is_invalid_table_and_deck_unchanged() {
      let mut deck = deck_with(table("t1", data()));
      let before = deck.clone();
      let patch = json!({ "dataset": null });
      let r = apply(&mut deck, Op::SetTableOption { target: t1(), param: "data".into(), patch });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::InvalidTable);
      assert_that!(deck).is_equal_to(before);
  }

  #[test]
  fn set_table_option_on_a_chart_is_param_type() {
      let mut deck = deck_with(chart("t1"));
      let r = apply(&mut deck, Op::SetTableOption { target: t1(), param: "option".into(), patch: json!({}) });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
  }

  #[test]
  fn set_chart_option_on_a_table_is_param_type() {
      let mut deck = deck_with(table("t1", data()));
      let r = apply(&mut deck, Op::SetChartOption { target: t1(), param: "data".into(), patch: json!({}) });
      assert_that!(kind_of(r)).is_equal_to(OpErrorKind::ParamType);
  }

  #[test]
  fn validate_deck_with_valid_table_reports_nothing() {
      assert_that!(validate(&deck_with(table("t1", data())), &registry(), None)).is_empty();
  }

  #[test]
  fn validate_deck_with_ragged_table_reports_one_issue() {
      let deck = Deck::from_json(
          &json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
              { "id": "t1", "kind": "table", "params": { "data": ragged() } } ] } }] })
          .to_string(),
      )
      .unwrap_or_else(|e| panic!("{e}"));
      assert_that!(validate(&deck, &registry(), None)).has_length(1);
  }

  #[test]
  fn ops_schema_lists_set_data() {
      assert_that!(ops_schema().unwrap().to_string()).contains("set_data");
  }

  #[test]
  fn ops_schema_lists_set_table_option() {
      assert_that!(ops_schema().unwrap().to_string()).contains("set_table_option");
  }
  ```

  Dans `src/widget_tests.rs` :

  ```rust
  #[test]
  fn param_type_host_of_a_host_param_returns_it() {
      assert_that!(ParamType::Host { host: HostType::Table }.host()).is_equal_to(Some(HostType::Table));
  }

  #[test]
  fn param_type_host_of_plain_returns_none() {
      assert_that!(ParamType::Plain.host()).is_none();
  }
  ```

  Dans `src/ops/value_tests.rs`, à côté des cas `host(chart)` des l. 74-75, ajouter :
  - un `fn table_ty() -> ParamType { ParamType::Host { host: HostType::Table } }` ;
  - deux cas :

    ```rust
    #[case::table_ragged(table_ty(), json!({ "dataset": { "source": [["A", "B"], [1]] } }), OpErrorKind::InvalidTable)]
    #[case::table_not_object(table_ty(), json!("x"), OpErrorKind::ParamType)]
    ```

  La forme exacte du `rstest` est celle des cas voisins.

- [ ] **Step 2 : lancer les tests et constater l'échec.**

  ```bash
  cargo nextest run -p oku-slides-core
  ```

  Résultat attendu : FAIL. `table_registry` refuse `host: "table"`, et `SetTableOption`, `InvalidTable` et `ParamType::host` n'existent pas.

- [ ] **Step 3 : `widget.rs`.**

  ```rust
  pub enum HostType {
      Chart,
      Table,
  }
  ```

  Ajouter les méthodes suivantes (le `impl WidgetSchema` existe déjà l. 133 : y ajouter la méthode, sans créer un second bloc) :

  ```rust
  impl ParamType {
      /// L'hôte qui fournit ce paramètre, s'il en a un.
      pub fn host(&self) -> Option<HostType> {
          match self {
              ParamType::Host { host } => Some(*host),
              _ => None,
          }
      }
  }

  impl WidgetSchema {
      /// Le premier paramètre fourni par l'hôte `host`, s'il est déclaré.
      pub fn host_param(&self, host: HostType) -> Option<&ParamSchema> {
          self.params.iter().find(|p| p.ty.host() == Some(host))
      }
  }
  ```

  `check_param` et `compatible` ne changent pas : le bras `(b, p) => b == p` distingue déjà les deux hôtes.

- [ ] **Step 4 : `error.rs`.**
  - Dans `OpErrorKind`, après `InvalidChart` :

    ```rust
        /// Options ou données de tableau refusées par `oku-slides-chart`.
        InvalidTable,
    ```

  - Dans `OpInner`, après `Chart`. Le sélecteur `TableSnafu` est `pub(crate)` grâce à la visibilité de l'enum.

    ```rust
        /// Tableau refusé ; `subject` : l'élément, ou le paramètre s'il n'est pas connu.
        #[snafu(display("table {subject} refused the change"))]
        Table { subject: String, source: oku_slides_chart::ChartError, backtrace: Backtrace },
    ```

  - Dans `kind()` : `OpInner::Table { .. } => OpErrorKind::InvalidTable,`.
  - Dans `subject()` : `OpInner::Chart { subject, .. } | OpInner::Table { subject, .. } => Some(subject),`.
  - Dans `localize`, remplacer le `if let OpInner::Chart …` par :

    ```rust
            let host = match &*self.0 {
                OpInner::Chart { subject, source, .. } => Some(("op-invalid-chart", subject, source)),
                OpInner::Table { subject, source, .. } => Some(("op-invalid-table", subject, source)),
                _ => None,
            };
            if let Some((key, subject, source)) = host {
                return l.tr_with(key, &[("subject", subject.as_str().into()), ("reason", source.localize(l).into())]);
            }
    ```

  - Dans la table des clés : `OpErrorKind::InvalidTable => "op-invalid-table",`.

  Dans `core.ftl`, juste après `op-invalid-chart` :
  - en : `op-invalid-table = Table { $subject }: { $reason }` ;
  - fr : `op-invalid-table = Tableau { $subject } : { $reason }` ;
  - les 18 autres locales : le mot « tableau » dans la langue, sur la forme exacte de leur `op-invalid-chart` (ponctuation et ordre des mots compris).

- [ ] **Step 5 : `ops/host.rs`.**

  ```rust
  //! Paramètres fournis par l'hôte (`host(chart)`, `host(table)`) : contrôle
  //! des valeurs, options et données.

  use serde::Serialize;
  use serde::de::DeserializeOwned;
  use serde_json::Value;
  use snafu::ResultExt;

  use oku_slides_chart::{DataOp, apply_option_patch, chart_data, parse_option, parse_table, patch_table, table_data};

  use super::{declared, typed, widget};
  use crate::error::{ChartSnafu, InvalidValueSnafu, OpError, ParamTypeSnafu, ParseValueSnafu, TableSnafu};
  use crate::locate::Walk;
  use crate::model::Element;
  use crate::params::{ParamValue, Params};
  use crate::widget::{HostType, WidgetRegistry};

  /// Forme attendue d'un paramètre de l'hôte `host`, pour `ParamType`.
  pub(super) fn expected_host(host: HostType) -> &'static str {
      match host {
          HostType::Chart => "a chart option",
          HostType::Table => "a table option",
      }
  }

  /// Options de l'hôte `host` contrôlées, rendues sous leur forme sérialisée.
  ///
  /// `subject` nomme l'élément (ou le paramètre) dans l'erreur.
  pub(crate) fn host_option(host: HostType, subject: &str, v: &Value) -> Result<Value, OpError> {
      let serialized = match host {
          // `parse_option` lit les options et les passe par `normalize`.
          HostType::Chart => serde_json::to_value(parse_option(v).context(ChartSnafu { subject })?),
          HostType::Table => serde_json::to_value(parse_table(v).context(TableSnafu { subject })?),
      };
      Ok(serialized.context(ParseValueSnafu { field: subject })?)
  }

  /// L'hôte du paramètre `param` du widget `kind`, s'il est bien `want`.
  fn host_param(registry: &WidgetRegistry, kind: &str, param: &str, want: HostType) -> Result<(), OpError> {
      let p = declared(widget(registry, kind)?, param)?;
      if p.ty.host() != Some(want) {
          return Err(ParamTypeSnafu { param, expected: expected_host(want) }.build().into());
      }
      Ok(())
  }

  /// Refuse des paramètres d'hôte incohérents (`subject` : qui les porte).
  pub(super) fn check_host_params(
      registry: &WidgetRegistry,
      kind: &str,
      params: &Params,
      subject: &str,
  ) -> Result<(), OpError> {
      let Some(schema) = registry.get(kind) else { return Ok(()) };
      for p in &schema.params {
          let Some(host) = p.ty.host() else { continue };
          match params.get(&p.name) {
              None => {}
              Some(ParamValue::Value(v)) => {
                  host_option(host, subject, v)?;
              }
              Some(_) => return Err(ParamTypeSnafu { param: &p.name, expected: expected_host(host) }.build().into()),
          }
      }
      Ok(())
  }

  /// Refuse un élément (ou un de ses enfants) dont un paramètre d'hôte est incohérent.
  pub(super) fn check_hosts(registry: &WidgetRegistry, element: &Element) -> Result<(), OpError> {
      let nested = Walk::over(&element.params, Some(&element.id)).map(|l| l.element);
      for e in std::iter::once(element).chain(nested) {
          check_host_params(registry, &e.kind, &e.params, &e.id)?;
      }
      Ok(())
  }

  fn read<T: DeserializeOwned>(params: &Params, param: &str, host: HostType) -> Result<T, OpError> {
      match params.get(param) {
          Some(ParamValue::Value(v)) => typed(param, v.clone()),
          None => Err(InvalidValueSnafu { field: param, reason: "the parameter has no value" }.build().into()),
          Some(_) => Err(ParamTypeSnafu { param, expected: expected_host(host) }.build().into()),
      }
  }

  fn write<T: Serialize>(params: &mut Params, param: &str, option: &T) -> Result<(), OpError> {
      let v = serde_json::to_value(option).context(ParseValueSnafu { field: param })?;
      params.set(param, ParamValue::Value(v));
      Ok(())
  }

  /// `set_chart_option` / `set_table_option` : merge patch sur les options de l'hôte `host`.
  pub(super) fn set_option(
      registry: &WidgetRegistry,
      kind: &str,
      params: &mut Params,
      param: &str,
      host: HostType,
      patch: &Value,
      subject: &str,
  ) -> Result<(), OpError> {
      host_param(registry, kind, param, host)?;
      match host {
          HostType::Chart => {
              let (next, _) = apply_option_patch(&read(params, param, host)?, patch).context(ChartSnafu { subject })?;
              write(params, param, &next)
          }
          HostType::Table => {
              let (next, _) = patch_table(&read(params, param, host)?, patch).context(TableSnafu { subject })?;
              write(params, param, &next)
          }
      }
  }

  /// `set_data` : applique `change` aux données d'un graphique ou d'un tableau.
  pub(super) fn set_data(
      registry: &WidgetRegistry,
      kind: &str,
      params: &mut Params,
      param: &str,
      change: &DataOp,
      subject: &str,
  ) -> Result<(), OpError> {
      let p = declared(widget(registry, kind)?, param)?;
      match p.ty.host() {
          Some(host @ HostType::Chart) => {
              let next = chart_data(&read(params, param, host)?, change).context(ChartSnafu { subject })?;
              write(params, param, &next)
          }
          Some(host @ HostType::Table) => {
              let next = table_data(&read(params, param, host)?, change).context(TableSnafu { subject })?;
              write(params, param, &next)
          }
          None => Err(ParamTypeSnafu { param, expected: "a chart or table option" }.build().into()),
      }
  }
  ```

  Dans `ops/mod.rs` :
  - ajouter `mod host;` à côté de `mod check;` ;
  - supprimer `is_chart`, `check_chart_params`, `check_charts`, `chart_param`, `read_chart`, `write_chart`, `chart_data` et la section `// ---- Graphiques` ;
  - l'import l. 22 devient `use oku_slides_chart::DataOp;`. Retirer `HostType` et `ChartSnafu` s'ils ne servent plus. Garder `DeserializeOwned`, dont `typed` (l. 520) a besoin ;
  - remplacer `check_charts(` par `host::check_hosts(` et `check_chart_params(` par `host::check_host_params(` (l. 612, 648, 653 et 801) ;
  - `declared`, `widget` et `typed` restent privés : `ops::host` est un module enfant et les voit ;
  - après `SetChartOption`, ajouter :

    ```rust
        /// Modifie les options d'affichage d'un tableau (paramètre `host(table)`).
        ///
        /// JSON Merge Patch sur `header-row`, `header-column`, `striped`, `align` ; jamais `dataset`.
        SetTableOption {
            target: Target,
            param: String,
            patch: Value,
        },
    ```

  - remplacer les bras d'application :

    ```rust
            Op::SetChartOption { target, param, patch } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_option(registry, kind, params, &param, HostType::Chart, &patch, subject)
            }
            Op::SetTableOption { target, param, patch } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_option(registry, kind, params, &param, HostType::Table, &patch, subject)
            }
            Op::SetData { target, param, change } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_data(registry, kind, params, &param, &change, subject)
            }
    ```

    Comme ces bras utilisent `HostType`, son import reste.

  Le bras `SetChartOption` est aujourd'hui exactement cette forme d'emprunt (`subject` avec `target_mut`), donc il compile.

- [ ] **Step 6 : `value.rs`, `check.rs`, `validate.rs`.**
  - `value.rs` : le bras `Host` devient :

    ```rust
            ParamType::Host { host } => match value {
                v @ Value::Object(_) => super::host::host_option(*host, param, &v).map(|v| Some(ParamValue::Value(v))),
                _ => Err(wrong(super::host::expected_host(*host))),
            },
    ```

    Supprimer `chart_option` et l'import `parse_option`. Dans la doc de `param_value`, remplacer « ou `InvalidChart` » par « , `InvalidChart` ou `InvalidTable` ».
  - `check.rs:64` : `ParamType::Host { host } => super::host::expected_host(*host),`.
  - `validate.rs:295-298` : remplacer le bras du graphique par un seul bras commun :

    ```rust
            (ParamType::Host { host }, ParamValue::Value(v)) => crate::ops::host_option(*host, "", v).is_ok(),
    ```

    Dans `ops/mod.rs`, réexporter `pub(crate) use host::host_option;`. Retirer de `validate.rs` les imports de `oku_slides_chart` devenus inutiles.

- [ ] **Step 7 : consommateurs de `host_param`.**
  - `render/src/charts.rs:300-308` :

    ```rust
    /// Nom du paramètre `host("chart")` du widget `kind`, s'il en a un.
    fn chart_param<'r>(registry: &'r WidgetRegistry, kind: &str) -> Option<&'r str> {
        registry.get(kind)?.host_param(HostType::Chart).map(|p| p.name.as_str())
    }
    ```

  - `session/src/charts.rs:36-38` :

    ```rust
    fn chart_params(schema: &WidgetSchema) -> impl Iterator<Item = &str> {
        schema.params.iter().filter(|p| p.ty.host() == Some(HostType::Chart)).map(|p| p.name.as_str())
    }
    ```

  Retirer les imports devenus inutiles (`ParamType` si besoin).

- [ ] **Step 8 : `ARCHITECTURE.md` du cœur.** Le fichier doit rester sous 50 lignes ; resserrer une ligne voisine si besoin.
  - Dans « Types clés », sur la ligne `Op`, ajouter : « `ops/host.rs` : paramètres `host(chart|table)` (contrôle à l'insertion, `set_chart_option`, `set_table_option`, `set_data`) ».
  - Dans « Points d'extension », ajouter : « Un type d'hôte : `HostType` + `ParamType::host` + `ops/host.rs` (`host_option`, `set_option`, `set_data`) ».

- [ ] **Step 9 : lancer les tests et constater qu'ils passent.**

  ```bash
  cargo nextest run -p oku-slides-core -p oku-slides-i18n
  cargo clippy -p oku-slides-core --all-targets -- -D warnings
  cargo nextest run --workspace --exclude oku-slides-app
  ```

  Résultat attendu : PASS. Les tests de `tests/charts.rs` passent sans modification, `ops/mod.rs` compte moins de 843 lignes (`wc -l`), et l'espace de travail reste vert, puisque la fixture n'a pas bougé.

- [ ] **Step 10 : commit.**

  ```bash
  git add crates/oku-slides-core crates/oku-slides-render/src/charts.rs crates/oku-slides-session/src/charts.rs crates/oku-slides-i18n
  git diff --cached --name-only
  git commit -m "feat(core) : paramètres host(table), set_data pour graphiques et tableaux, set_table_option (t-578)"
  ```

---

### Task 4 : Rendu — garde Rust, widget Typst `table`, jetons, registre

**Files:**
- Create :
  - `crates/oku-slides-render/src/tables.rs` ;
  - `crates/oku-slides-render/src/tables_tests.rs` ;
  - `crates/oku-slides-render/tests/tables.rs`.
- Modify :
  - `crates/oku-slides-render/src/lib.rs` : `mod tables;` ;
  - `crates/oku-slides-render/src/charts.rs` : `for_each_element` devient `pub(crate)` ;
  - `crates/oku-slides-render/src/world.rs:235-236` ;
  - `typst/oku-slides/src/widgets.typ` : `_table` et `registry` ;
  - `typst/oku-slides/src/data.typ:154` ;
  - `typst/oku-slides/src/params.typ:32` : la doc ;
  - `typst/oku-slides/src/default-tokens.json` et `typst/oku-slides/themes/pro.json` ;
  - `crates/oku-slides-session/src/theme_schema.json` ;
  - `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 fichiers) ;
  - `fixtures/widgets/base-registry.json` : `table` entre `stack` et `text`, puisque l'ordre est alphabétique ;
  - `crates/oku-slides-core/tests/common/mod.rs` : suppression de `table_registry`, `tests/tables.rs` repasse à `registry()` ;
  - `crates/oku-slides-render/tests/widgets.rs:101` ;
  - les snapshots `crates/oku-slides-render/tests/snapshots/widgets__base_registry.snap` et `reference_pages` ;
  - `crates/oku-slides-render/tests/golden/widgets.json` ;
  - `typst/oku-slides/README.md` : l. 83-84, 216-219 et 279 ;
  - `crates/oku-slides-render/ARCHITECTURE.md:38`.

**Interfaces :**
- Consumes : `parse_table` (tâche 2) et `WidgetSchema::host_param` (tâche 3).
- Produces :
  - `pub(crate) fn sanitize(deck: &mut Json, registry: &WidgetRegistry)` dans `render/src/tables.rs` ;
  - le widget Typst `table` ;
  - les 9 jetons `component.table.*`.

- [ ] **Step 1 : écrire les tests qui échouent.**

  Unitaire, dans `src/tables_tests.rs` :

  ```rust
  use serde_json::json;
  use speculoos::prelude::*;

  use super::*;

  fn registry() -> WidgetRegistry {
      let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
      let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{e}"));
      WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}")))
          .unwrap_or_else(|e| panic!("{e}"))
  }

  fn deck(data: Json) -> Json {
      json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
          { "id": "t1", "kind": "table", "params": { "data": data } } ] } }] })
  }

  fn data_of(d: &Json) -> &Json {
      &d["slides"][0]["params"]["blocks"][0]["params"]["data"]
  }

  #[test]
  fn sanitize_valid_table_writes_its_canonical_form() {
      let mut d = deck(json!({ "dataset": { "source": [["A"], [1]] } }));
      sanitize(&mut d, &registry());
      assert_that!(data_of(&d)["header-row"]).is_equal_to(json!(true));
  }

  #[test]
  fn sanitize_malformed_table_clears_its_data() {
      let mut d = deck(json!({ "dataset": { "source": [5] } }));
      sanitize(&mut d, &registry());
      assert_that!(data_of(&d)).is_equal_to(&Json::Null);
  }

  #[test]
  fn sanitize_oversized_table_clears_its_data() {
      let rows: Vec<Json> = (0..101).map(|i| json!([i])).collect();
      let mut d = deck(json!({ "dataset": { "source": rows } }));
      sanitize(&mut d, &registry());
      assert_that!(data_of(&d)).is_equal_to(&Json::Null);
  }
  ```

  Intégration, dans `tests/tables.rs` :

  ```rust
  //! Widget `table` : rendu Typst natif, mise à l'échelle, garde des données, jetons.

  use std::collections::BTreeMap;

  use oku_slides_core::Deck;
  use oku_slides_render::{Compiled, Fonts, Renderer};
  use serde_json::{Value, json};
  use speculoos::prelude::*;

  fn deck_of(slides: Value, tokens: Option<Value>) -> Deck {
      let mut d = json!({ "slides": slides });
      if let Some(t) = tokens {
          d["tokens"] = t;
      }
      Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"))
  }

  /// Une slide de contenu dont l'unique bloc est le tableau `t1`.
  fn in_flow(data: Value) -> Value {
      json!([{ "id": "s", "kind": "content", "params": { "title": "T",
          "blocks": [{ "id": "t1", "kind": "table", "params": { "data": data } }] } }])
  }

  fn rows(n: usize) -> Value {
      let mut source = vec![json!(["Pays", "Ventes"])];
      source.extend((0..n).map(|i| json!([format!("P{i}"), i])));
      json!({ "dataset": { "source": source } })
  }

  fn with(mut data: Value, key: &str, v: Value) -> Value {
      data[key] = v;
      data
  }

  fn compile(d: &Deck) -> Compiled {
      let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
      r.render(d, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
  }

  fn hash(d: &Deck) -> u128 {
      compile(d).page_hash(0).unwrap_or_else(|| panic!("page 1 absente"))
  }

  fn flow_hash(data: Value) -> u128 {
      hash(&deck_of(in_flow(data), None))
  }

  /// Slide vide : un tableau au cadre 300×120 en (10, 10), puis un rectangle
  /// opaque qui déborde le cadre de 4 pt de chaque côté (dessiné par-dessus).
  fn covered(table: Option<Value>) -> Deck {
      let mut elements = Vec::new();
      if let Some(data) = table {
          elements.push(json!({ "id": "t1", "kind": "table", "params": { "data": data },
              "frame": { "x": 10, "y": 10, "w": 300, "h": 120 } }));
      }
      elements.push(json!({ "id": "cache", "kind": "shape", "params": { "shape": "rect" }, "style": { "fill": "ink" },
          "frame": { "x": 6, "y": 6, "w": 308, "h": 128 } }));
      deck_of(json!([{ "id": "s", "kind": "blank", "params": { "elements": elements } }]), None)
  }

  #[test]
  fn table_larger_than_its_frame_draws_nothing_outside_it() {
      assert_that!(hash(&covered(Some(rows(30))))).is_equal_to(hash(&covered(None)));
  }

  #[test]
  fn table_with_a_long_cell_draws_nothing_outside_its_frame() {
      let data = json!({ "dataset": { "source": [["Note"], ["mot ".repeat(200)]] } });
      assert_that!(hash(&covered(Some(data)))).is_equal_to(hash(&covered(None)));
  }

  #[test]
  fn tall_table_in_flow_keeps_its_box_on_the_page() {
      let c = compile(&deck_of(in_flow(rows(60)), None));
      let b = c.layout().boxes.iter().find(|b| b.id == "t1").cloned().unwrap_or_else(|| panic!("boîte t1 absente"));
      assert_that!(c.page_count()).is_equal_to(1);
      assert_that!(b.y + b.h).is_less_than_or_equal_to(1080.0);
  }

  #[test]
  fn table_in_flow_grows_with_its_rows() {
      let height = |n| {
          let c = compile(&deck_of(in_flow(rows(n)), None));
          c.layout().boxes.iter().find(|b| b.id == "t1").map(|b| b.h).unwrap_or_else(|| panic!("boîte t1 absente"))
      };
      assert_that!(height(6)).is_greater_than(height(2));
  }

  #[test]
  fn table_without_data_differs_from_an_unknown_element() {
      let placeholder = deck_of(json!([{ "id": "s", "kind": "content", "params": { "blocks": [{ "id": "t1", "kind": "table" }] } }]), None);
      let unknown = deck_of(json!([{ "id": "s", "kind": "content", "params": { "blocks": [{ "id": "t1", "kind": "inconnu" }] } }]), None);
      assert_that!(hash(&placeholder)).is_not_equal_to(hash(&unknown));
  }

  #[test]
  fn malformed_table_data_renders_the_placeholder() {
      let placeholder = deck_of(json!([{ "id": "s", "kind": "content", "params": { "blocks": [{ "id": "t1", "kind": "table" }] } }]), None);
      for bad in [json!({ "dataset": { "source": [5] } }), json!({ "dataset": "x" }), json!({ "dataset": { "source": [["A"]] }, "header-row": "oui" })] {
          assert_that!(flow_hash(bad)).is_equal_to(hash(&placeholder));
      }
  }

  #[test]
  fn oversized_table_data_renders_the_placeholder() {
      let placeholder = deck_of(json!([{ "id": "s", "kind": "content", "params": { "blocks": [{ "id": "t1", "kind": "table" }] } }]), None);
      assert_that!(flow_hash(rows(200))).is_equal_to(hash(&placeholder));
  }

  #[test]
  fn markup_in_cells_stays_literal() {
      let data = json!({ "dataset": { "source": [["#panic(\"x\")", "$x$"], ["*b* @ref <l>", "] \\"]] } });
      assert_that!(compile(&deck_of(in_flow(data), None)).page_count()).is_equal_to(1);
  }

  #[test]
  fn header_column_changes_the_page() {
      assert_that!(flow_hash(with(rows(3), "header-column", json!(true)))).is_not_equal_to(flow_hash(rows(3)));
  }

  #[test]
  fn header_row_off_changes_the_page() {
      assert_that!(flow_hash(with(rows(3), "header-row", json!(false)))).is_not_equal_to(flow_hash(rows(3)));
  }

  #[test]
  fn striped_changes_the_page() {
      assert_that!(flow_hash(with(rows(4), "striped", json!(true)))).is_not_equal_to(flow_hash(rows(4)));
  }

  #[test]
  fn auto_alignment_of_a_number_column_equals_end() {
      assert_that!(flow_hash(with(rows(3), "align", json!(["auto", "end"])))).is_equal_to(flow_hash(rows(3)));
  }

  #[test]
  fn start_alignment_of_a_number_column_changes_the_page() {
      assert_that!(flow_hash(with(rows(3), "align", json!(["auto", "start"])))).is_not_equal_to(flow_hash(rows(3)));
  }

  #[test]
  fn header_tokens_change_the_page() {
      let themed = deck_of(in_flow(rows(2)), Some(json!({ "component": { "table": { "header-bg": "#ff0000" } } })));
      assert_that!(hash(&themed)).is_not_equal_to(flow_hash(rows(2)));
  }

  #[test]
  fn zero_padding_single_empty_cell_compiles() {
      let tokens = json!({ "component": { "table": { "padding-x": 0, "padding-y": 0 } } });
      let d = deck_of(in_flow(json!({ "dataset": { "source": [[null]] } })), Some(tokens));
      assert_that!(compile(&d).page_count()).is_equal_to(1);
  }
  ```

  Remarques :
  - Les champs `ElementBox { id, x, y, w, h }`, `page_hash -> Option<u128>`, `render(&Deck, &BTreeMap)` et le `DECK_VERSION` par défaut ont été vérifiés.
  - 1080 est la hauteur de page : `page` dans `default-tokens.json`. Vérifier sa valeur au début du test et en faire une constante nommée `PAGE_H`.
  - Ajouter `"table"` à la liste de `tests/widgets.rs:101`, après `"chart"`.

- [ ] **Step 2 : lancer les tests et constater l'échec.**

  ```bash
  cargo nextest run -p oku-slides-render --test tables --test widgets
  cargo nextest run -p oku-slides-render tables_tests
  ```

  Résultat attendu : FAIL. `tables.rs` n'existe pas, `table` est un widget inconnu, et la mise à l'échelle comme le substitut manquent.

- [ ] **Step 3 : écrire la garde Rust.**

  Dans `charts.rs`, l. 313 : `pub(crate) fn for_each_element(…)`.

  Fichier `src/tables.rs` :

  ```rust
  //! Tableaux du deck avant Typst : forme canonique, ou rien si invalides.
  //!
  //! Un deck ouvert n'est jamais refusé (`validate` signale seulement) : sans
  //! cette passe, un tableau mal formé ou énorme ferait échouer ou traîner la
  //! compilation de tout le deck. Typst affiche le substitut à la place.

  use oku_slides_core::{HostType, WidgetRegistry};
  use serde_json::Value as Json;

  use crate::charts::for_each_element;

  /// Ramène chaque paramètre `host("table")` à sa forme canonique, ou à `null`.
  pub(crate) fn sanitize(deck: &mut Json, registry: &WidgetRegistry) {
      for_each_element(deck, &mut |e| {
          let kind = e.get("kind").and_then(Json::as_str).unwrap_or_default().to_owned();
          let Some(param) = registry.get(&kind).and_then(|w| w.host_param(HostType::Table)) else { return };
          let Some(Json::Object(params)) = e.get_mut("params") else { return };
          let Some(value) = params.get_mut(&param.name) else { return };
          *value = match oku_slides_chart::parse_table(value) {
              // Sérialiser une `TableOption` lue ne peut pas échouer ; au pire, le substitut.
              Ok(table) => serde_json::to_value(table).unwrap_or(Json::Null),
              Err(reason) => {
                  tracing::warn!(reason = %reason, "tableau non rendu");
                  Json::Null
              }
          };
      });
  }
  ```

  Remarques :
  - `tracing` est déjà une dépendance du crate, puisque `charts.rs` l'utilise.
  - `kind` emprunte `e` et `get_mut` aussi. Écrire donc `let kind = e.get("kind").and_then(Json::as_str).unwrap_or_default().to_owned();`, puis `registry.get(&kind)`.

  Ajouter `#[cfg(test)] #[path = "tables_tests.rs"] mod tables_tests;`, puis `mod tables;` dans `lib.rs`, après `mod registry;`.

  Dans `world.rs:235-236` :

  ```rust
          let mut json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
          crate::tables::sanitize(&mut json, registry);
          self.charts.attach(&mut json, &self.fonts, registry)?;
  ```

  `redraw_charts` relit `self.deck`, qui est déjà assaini, donc rien à faire de ce côté.

- [ ] **Step 4 : écrire le widget Typst.** Dans `widgets.typ`, après `_chart` :

  ```typst
  // Tableau rendu en Typst. `data` : options de `oku-slides-chart::TableOption`,
  // déjà ramenées à leur forme canonique par le rendu (render/src/tables.rs) ;
  // les gardes ci-dessous protègent un appel direct (aperçu, pack).
  #let _table-max-rows = 100     // MAX_TABLE_ROWS
  #let _table-max-columns = 20   // MAX_TABLE_COLUMNS

  #let _table-placeholder(t, framed) = rect(
    width: 100%, height: if framed { 100% } else { 6em }, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
    align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), "table")),
  )

  // Texte d'une cellule : nombre entier sans décimales, case cochée en ✓, autre type vide.
  #let _cell-text(v) = {
    if type(v) == str { v }
    else if type(v) == bool { if v { "✓" } else { "" } }
    else if type(v) == int { str(v) }
    else if type(v) == float {
      if calc.fract(v) == 0 and calc.abs(v) < 1e15 { str(int(v)) } else { str(v) }
    }
    else { "" }
  }

  #let _numeric(v) = v == none or type(v) == int or type(v) == float

  // Le drapeau `key` de `d` s'il est booléen, sinon `default`.
  #let _flag(d, key, default) = {
    let v = d.at(key, default: default)
    if type(v) == bool { v } else { default }
  }

  #let _table = widget("table", params: (data: p.host("table")), (args, ctx) => {
    let t = ctx.tokens
    let d = args.data
    let dataset = if type(d) == dictionary { d.at("dataset", default: none) } else { none }
    let source = if type(dataset) == dictionary { dataset.at("source", default: none) } else { none }
    let ok = type(source) == array and source.len() > 0 and source.len() <= _table-max-rows
      and source.all(r => type(r) == array) and source.first().len() > 0
      and source.first().len() <= _table-max-columns
    if not ok { return _table-placeholder(t, ctx.framed) }
    let k = t.component.table
    let n = source.first().len()
    let rows = source.map(r => r.slice(0, calc.min(n, r.len())) + (none,) * calc.max(0, n - r.len()))
    let head-row = _flag(d, "header-row", true)
    let head-col = _flag(d, "header-column", false)
    let striped = _flag(d, "striped", false)
    let aligns = d.at("align", default: ())
    let aligns = if type(aligns) == array { aligns } else { () }
    let first = if head-row { 1 } else { 0 }
    let data-rows = rows.slice(first)
    // `auto` : à la fin pour une colonne de nombres (au moins un), au début sinon.
    let col-aligns = range(n).map(x => {
      let a = aligns.at(x, default: "auto")
      if a == "start" { start } else if a == "center" { center } else if a == "end" { end }
      else if data-rows.all(r => _numeric(r.at(x))) and data-rows.any(r => r.at(x) != none) { end }
      else { start }
    })
    let header-bg = color-of(t, k.header-bg)
    let stripe-bg = color-of(t, k.stripe-bg)
    let header-color = color-of(t, k.header-color)
    let header-weight = weight-of(t, k.header-weight)
    let cells = rows.enumerate().map(((y, r)) => r.enumerate().map(((x, v)) => {
      let body = _cell-text(v)
      if head-row and y == 0 { text(weight: header-weight, fill: header-color, body) }
      else if head-col and x == 0 { text(weight: header-weight, body) }
      else { body }
    })).flatten()
    let rule = stroke-of(t, k.rule-width) + color-of(t, k.rule-color)
    let tbl = {
      set text(size: size-of(t, k.size))
      table(
        columns: n,
        inset: (x: space-of(t, k.padding-x), y: space-of(t, k.padding-y)),
        align: (x, y) => col-aligns.at(x) + horizon,
        fill: (x, y) => if head-row and y == 0 { header-bg }
          else if striped and calc.odd(y - first) { stripe-bg } else { none },
        stroke: (x, y) => (bottom: rule),
        ..if head-row { (table.header(..cells.slice(0, n)),) } else { () },
        ..cells.slice(first * n),
      )
    }
    // Mesuré à la largeur disponible (les cellules passent à la ligne) ; réduit
    // seulement s'il dépasse encore. Pas de pagination.
    layout(size => {
      let m = measure(block(width: size.width, tbl))
      if m.width <= 0pt or m.height <= 0pt { return tbl }
      let fit-w = size.width / m.width
      let fit-h = if size.height == float.inf * 1pt { 1 } else { size.height / m.height }
      let s = calc.max(0.01, calc.min(1, fit-w, fit-h))
      if s >= 1 { tbl } else { scale(s * 100%, reflow: true, block(width: size.width, tbl)) }
    })
  })
  ```

  Ajouter `_table` dans `registry`, après `_chart` : `#let registry = (_text, _card, _image, _shape, _chart, _table, _stack, _canvas,`.

  Remarques :
  - `block(width: size.width, tbl)` donne au tableau la largeur disponible, et les colonnes `auto` y passent à la ligne. La réduction ne joue que sur ce qui dépasse encore, surtout en hauteur.
  - Une réduction de `s` sur un bloc de largeur `size.width` rend un bloc de largeur `s × size.width`, donc plus étroit que le cadre. C'est voulu : le tableau ne déborde jamais.
  - Les textes passent par `_cell-text`, qui renvoie toujours un `str`. Typst les pose en texte, jamais en balisage (pas d'`eval`).
  - `calc.odd(y - first)` n'est évalué que pour `y ≥ first`.

  Dans `data.typ:154`, faire mesurer un tableau en flux dans la hauteur disponible, comme un graphique :

  ```typst
      let real-h = if fixed { sz.height } else if kind in ("chart", "table") and sz.height < float.inf * 1pt {
  ```

  Mettre à jour le commentaire de la ligne au-dessus : « Un graphique ou un tableau s'adapte à la hauteur disponible : il se mesure dans celle-ci. ».

  Dans `params.typ:32`, la doc de `host` cite `"chart"` : écrire `"chart"` ou `"table"`.

- [ ] **Step 5 : jetons.**

  Dans `default-tokens.json`, dans `component`, après le bloc `"columns": { "gap": … }`. Mettre une virgule après l'accolade fermante de `columns`.

  ```json
      "table": {
        "size": "small",
        "header-bg": "accent",
        "header-color": "on-accent",
        "header-weight": "strong",
        "stripe-bg": "surface",
        "rule-color": "rule",
        "rule-width": "hairline",
        "padding-x": "sm",
        "padding-y": "xs"
      }
  ```

  `papier.json` n'a rien à recevoir : `default-tokens ← thème` est une fusion profonde (`slides.typ:12-14`), et `papier` ne redéfinit que `chapter`, `title-slide` et `end-slide`.

  `pro.json` est un instantané complet, avec `card`, `list`, etc. Ajouter sous `tokens.component`, après `columns`, le bloc suivant. `#f3f5f7` diffère de `rule` (`#e1e5ea`), qui reste visible sur les lignes zébrées.

  ```json
      "table": {
        "size": "small",
        "header-bg": "brand-ink",
        "header-color": "white",
        "header-weight": 700,
        "stripe-bg": "#f3f5f7",
        "rule-color": "rule",
        "rule-width": "hairline",
        "padding-x": "sm",
        "padding-y": "xs"
      }
  ```

  Dans `theme_schema.json`, après les entrées `component.card` :

  ```json
    {"path": ["component", "table", "size"], "section": "components", "control": "pt", "min": 0, "max": 400, "refs": "size"},
    {"path": ["component", "table", "header-bg"], "section": "components", "control": "color"},
    {"path": ["component", "table", "header-color"], "section": "components", "control": "color"},
    {"path": ["component", "table", "header-weight"], "section": "components", "control": "weight"},
    {"path": ["component", "table", "stripe-bg"], "section": "components", "control": "color"},
    {"path": ["component", "table", "rule-color"], "section": "components", "control": "color"},
    {"path": ["component", "table", "rule-width"], "section": "components", "control": "pt", "min": 0, "max": 400, "refs": "stroke"},
    {"path": ["component", "table", "padding-x"], "section": "components", "control": "pt", "min": 0, "max": 400, "refs": "space"},
    {"path": ["component", "table", "padding-y"], "section": "components", "control": "pt", "min": 0, "max": 400, "refs": "space"},
  ```

  Si un test de `theme_schema_tests.rs` refuse `header-weight: 700` pour `pro`, prendre `"strong"`.

  Dans `session.ftl`, juste après les clés `theme-token-component-card-*` :

  en :

  ```
  theme-token-component-table-size = Table · text size
  theme-token-component-table-header-bg = Table · header background
  theme-token-component-table-header-color = Table · header text
  theme-token-component-table-header-weight = Table · header weight
  theme-token-component-table-stripe-bg = Table · stripe background
  theme-token-component-table-rule-color = Table · rule color
  theme-token-component-table-rule-width = Table · rule width
  theme-token-component-table-padding-x = Table · horizontal padding
  theme-token-component-table-padding-y = Table · vertical padding
  ```

  fr :

  ```
  theme-token-component-table-size = Tableau · taille du texte
  theme-token-component-table-header-bg = Tableau · fond de l'en-tête
  theme-token-component-table-header-color = Tableau · texte de l'en-tête
  theme-token-component-table-header-weight = Tableau · graisse de l'en-tête
  theme-token-component-table-stripe-bg = Tableau · fond des lignes zébrées
  theme-token-component-table-rule-color = Tableau · couleur des filets
  theme-token-component-table-rule-width = Tableau · épaisseur des filets
  theme-token-component-table-padding-x = Tableau · marge horizontale
  theme-token-component-table-padding-y = Tableau · marge verticale
  ```

  Traduire ces clés dans les 18 autres locales, au même endroit.

- [ ] **Step 6 : le registre figé, dans ce même commit.**
  - Dans `fixtures/widgets/base-registry.json`, insérer entre `stack` et `text` :

    ```json
        {
          "name": "table",
          "label": null,
          "category": "element",
          "params": [
            { "name": "data", "label": null, "type": "host", "host": "table", "default": null, "bind": null }
          ]
        },
    ```

    Reprendre l'indentation du fichier, un champ par ligne comme les voisins.
  - Dans `crates/oku-slides-core/tests/common/mod.rs`, supprimer `table_registry`. Dans `crates/oku-slides-core/tests/tables.rs`, `use common::table_registry as registry;` devient `use common::registry;`.

- [ ] **Step 7 : golden, README, ARCHITECTURE.**
  - Dans `tests/golden/widgets.json`, sur la slide du graphique `g1` (l. 93), juste après lui :

    ```json
    { "id": "tb1", "kind": "table", "params": { "data": {
      "dataset": { "source": [["Mois","Ventes","Marge"],["Jan",3,0.2],["Fév",5,0.25]] },
      "header-column": true, "striped": true } } }
    ```

  - `README.md` :
    - l. 83-84, liste des composants : ajouter `table` ;
    - l. 216-219 : « 13 widgets » devient « 14 widgets », et insérer `` `table`, `` après `` `chart`, `` dans la parenthèse ;
    - l. 279 : la ligne `p.host` porte `("chart", "table")`.
  - `crates/oku-slides-render/ARCHITECTURE.md:38` : après la puce des graphiques, ajouter « tableaux (`tables.rs`) : passe de garde avant Typst (forme canonique, ou `null` si invalide) ; le dessin est fait par Typst (`widgets.typ`), sans passe de dessin Rust ».

- [ ] **Step 8 : régénérer les snapshots et lancer les tests.**

  ```bash
  INSTA_UPDATE=always cargo nextest run -p oku-slides-render --test widgets --test golden
  cargo nextest run --workspace --exclude oku-slides-app
  ```

  Résultat attendu : PASS. Relire ensuite le diff des `.snap` :
  - `base_registry` gagne la seule entrée `table` ;
  - `reference_pages` ne change que pour les pages de `widgets` et `widgets-pro` qui contiennent `tb1`.

  Contrôle visuel, avec les PNG à ouvrir :

  ```bash
  GOLDEN_DUMP_DIR=/private/tmp/claude-501/-Users-matthieu-Projects-slider/512f7cc0-f7ad-4340-9980-2fbb5b14f0f1/scratchpad/golden cargo nextest run -p oku-slides-render --test golden --run-ignored only dump_reference_pages
  ```

- [ ] **Step 9 : commit.**

  ```bash
  git add typst/oku-slides crates/oku-slides-render crates/oku-slides-core/tests crates/oku-slides-session/src/theme_schema.json crates/oku-slides-i18n fixtures/widgets/base-registry.json
  git diff --cached --name-only
  git commit -m "feat(render) : widget table rendu en Typst, garde des données avant Typst, jetons component.table (t-578)"
  ```

---

### Task 5 : Session et MCP — requête `show_as`

**Files:**
- Create :
  - `crates/oku-slides-session/src/tables.rs` : `impl Session { show_as }` ;
  - `crates/oku-slides-session/tests/tables.rs`.
- Modify :
  - `crates/oku-slides-session/src/request.rs` : `ShowAs`, `View`, `changes_deck` et la seconde liste (l. 233, 291) ;
  - `crates/oku-slides-session/src/lib.rs` : `mod tables;` et un bras de `handle` à côté de `ChartKind` (l. 643) ;
  - `crates/oku-slides-session/src/error.rs` : `NoTable` ;
  - `crates/oku-slides-i18n/src/locales/*/session.ftl` : `session-no-table` ;
  - `crates/oku-slides-mcp/src/edit_tools.rs`, `handler.rs:56`, `tools.rs:201-202` ;
  - `crates/oku-slides-mcp/tests/edit.rs`.

**Interfaces :**
- Consumes :
  - de la tâche 2 : `chart_to_table` et `table_to_chart` ;
  - de la tâche 3 : `WidgetSchema::host_param`, `HostType` ;
  - de la tâche 4 : les widgets `table` et `chart` du registre live.
- Produces :
  - `Request::ShowAs { label: String, slide: String, element: String, view: View }`, en JSON `{"cmd":"show_as","label":…,"slide":…,"element":…,"view":"table"|"chart"}` ;
  - `pub enum View { Chart, Table }`, sérialisé en minuscules et réexporté à la racine du crate comme `Request` ;
  - `Session::show_as(&mut self, label: &str, slide: String, element: &str, view: View) -> Result<()>` ;
  - `SessionErrorKind::NoTable` ;
  - l'outil MCP `show_as`.

- [ ] **Step 1 : écrire les tests qui échouent.** Fichier `tests/tables.rs` :

  ```rust
  //! Tableaux dans une session : conversion depuis et vers un graphique, édition, annulation.

  mod common;

  use common::{req, session, snapshot_json};
  use oku_slides_session::{Session, SessionErrorKind};
  use serde_json::{Value, json};
  use speculoos::prelude::*;

  fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
      r.unwrap_or_else(|e| panic!("échec inattendu : {e:?}"))
  }

  /// Une session sur un deck neuf, avec le graphique `g` sur la slide 2 ; rend l'id de la slide.
  fn with_chart(s: &mut Session) -> String {
      ok(s.new_deck());
      let slide = s.snapshot().slides[1].id.to_string();
      ok(s.handle(req(json!({ "cmd": "apply", "label": "graphique", "ops": [{
          "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
          "element": { "id": "g", "kind": "chart", "params": { "option": {
              "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
              "xAxis": { "type": "category" }, "series": [{ "type": "bar" }, { "type": "bar" }] } } } }] }))));
      slide
  }

  fn slide_json(s: &mut Session, slide: &str) -> Value {
      let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
      ok(serde_json::from_str(&text))
  }

  fn element(s: &mut Session, slide: &str) -> Value {
      slide_json(s, slide)["slide"]["params"]["blocks"][0].clone()
  }

  fn show_as(s: &mut Session, slide: &str, view: &str) -> Result<String, oku_slides_session::SessionError> {
      s.handle(req(json!({ "cmd": "show_as", "label": "vue", "slide": slide, "element": "g", "view": view })))
  }

  #[test]
  fn show_as_table_keeps_id_and_dataset() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      let e = element(&mut s, &slide);
      assert_that!(e["kind"]).is_equal_to(json!("table"));
      assert_that!(e["params"]["data"]["dataset"]["source"][2]).is_equal_to(json!(["y", 3.0, 4.0]));
  }

  #[test]
  fn show_as_table_then_undo_restores_the_chart_in_one_step() {
      let mut s = session();
      let slide = with_chart(&mut s);
      let before = slide_json(&mut s, &slide);
      ok(show_as(&mut s, &slide, "table"));
      ok(s.handle(req(json!({ "cmd": "undo" }))));
      assert_that!(slide_json(&mut s, &slide)).is_equal_to(before);
  }

  #[test]
  fn show_as_chart_from_a_table_gets_one_series_per_column() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      ok(show_as(&mut s, &slide, "chart"));
      let e = element(&mut s, &slide);
      assert_that!(e["kind"]).is_equal_to(json!("chart"));
      assert_that!(e["params"]["option"]["series"].as_array().map(Vec::len)).is_equal_to(Some(2));
  }

  #[test]
  fn show_as_table_on_a_table_is_no_chart() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      let err = show_as(&mut s, &slide, "table").unwrap_err();
      assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoChart);
  }

  #[test]
  fn show_as_chart_on_a_chart_is_no_table() {
      let mut s = session();
      let slide = with_chart(&mut s);
      let err = show_as(&mut s, &slide, "chart").unwrap_err();
      assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoTable);
  }

  #[test]
  fn set_table_option_then_undo_restores_the_table() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      let before = element(&mut s, &slide);
      ok(s.handle(req(json!({ "cmd": "apply", "label": "zébrage", "ops": [{ "op": "set_table_option",
          "target": { "slide": slide, "element": "g" }, "param": "data", "patch": { "striped": true } }] }))));
      assert_that!(element(&mut s, &slide)["params"]["data"]["striped"]).is_equal_to(json!(true));
      ok(s.handle(req(json!({ "cmd": "undo" }))));
      assert_that!(element(&mut s, &slide)).is_equal_to(before);
  }

  #[test]
  fn set_data_on_a_table_then_undo_restores_the_cell() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      ok(s.handle(req(json!({ "cmd": "apply", "label": "cellule", "ops": [{ "op": "set_data",
          "target": { "slide": slide, "element": "g" }, "param": "data",
          "change": { "op": "set_cell", "row": 1, "col": 1, "value": 9 } }] }))));
      assert_that!(element(&mut s, &slide)["params"]["data"]["dataset"]["source"][1][1]).is_equal_to(json!(9.0));
      ok(s.handle(req(json!({ "cmd": "undo" }))));
      assert_that!(element(&mut s, &slide)["params"]["data"]["dataset"]["source"][1][1]).is_equal_to(json!(1.0));
  }

  #[test]
  fn snapshot_after_show_as_offers_undo() {
      let mut s = session();
      let slide = with_chart(&mut s);
      ok(show_as(&mut s, &slide, "table"));
      assert_that!(snapshot_json(&mut s)["undo_label"]).is_equal_to(json!("vue"));
  }
  ```

  Remarques :
  - `common::session()` renvoie un `Isolated` qui se déréférence en `Session`, et `common::req` construit une `Request`. Si `Isolated` ne se déréférence pas en `&mut Session`, utiliser son accesseur, comme dans `tests/history.rs`.
  - Le chemin `["slide"]["params"]["blocks"]` est celui de la réponse de `cmd: "slide"`. Vérifier la forme exacte dans `tests/errors.rs:182`, qui cherche `["Catégorie","Série 1"]` dans cette réponse, et ajuster `element()` en un seul endroit.

  Dans `crates/oku-slides-mcp/tests/edit.rs:53`, ajouter `"set_table_option"` à la liste de la tâche 1. Ajouter aussi :

  ```rust
  #[tokio::test(flavor = "multi_thread")]
  async fn tool_list_has_show_as() {
      let h = start().await;
      let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
      let tools = c.request_tool_list(None).await.unwrap().tools;
      assert_that!(tools.iter().any(|t| t.name == "show_as")).is_true();
  }
  ```

- [ ] **Step 2 : lancer les tests et constater l'échec.**

  ```bash
  cargo nextest run -p oku-slides-session --test tables
  cargo nextest run -p oku-slides-mcp
  ```

  Résultat attendu : FAIL. La requête `show_as` est inconnue, et `set_table_option` est absent de la liste MCP.

- [ ] **Step 3 : écrire la requête et la session.**

  Dans `request.rs`, après `ChartKind` :

  ```rust
      /// Affiche un graphique en tableau, ou l'inverse, en gardant ses données. Annulable.
      ShowAs {
          label: String,
          slide: String,
          element: String,
          view: View,
      },
  ```

  et :

  ```rust
  /// Widget sous lequel montrer des données : graphique ou tableau.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
  #[serde(rename_all = "lowercase")]
  pub enum View {
      Chart,
      Table,
  }
  ```

  Les derives sont ceux des autres enums du fichier : si `JsonSchema` n'y est pas importé, l'importer comme dans `edit_tools.rs`. Ajouter `| Self::ShowAs { .. }` à côté de `| Self::ChartKind { .. }` dans les deux listes exhaustives (l. 233 et 291). Exporter `View` depuis `lib.rs`, à côté de `Request`.

  Dans `error.rs` :
  - dans `SessionErrorKind`, après `NoChart` : `/// L'élément n'est pas un tableau (ou n'existe pas).` puis `NoTable,` ;
  - dans l'enum interne, après `NoChart` : `#[snafu(display("not a table: {id}"))] NoTable { id: String, backtrace: Backtrace },` ;
  - `kind()` : `NoTable { .. } => SessionErrorKind::NoTable,` ;
  - `localize` : `NoTable { id, .. } => l.tr_with("session-no-table", &[("subject", id.as_str().into())]),`.

  `session.ftl`, après `session-no-chart` :
  - en : `session-no-table = No table here: { $subject }` ;
  - fr : `session-no-table = Pas de tableau ici : { $subject }` ;
  - les 18 autres locales sur la forme de leur `session-no-chart`.

  Fichier `src/tables.rs` :

  ```rust
  //! Graphique ↔ tableau : la requête `show_as`, en une transaction.

  use oku_slides_core::{Element, HostType, Op, ParamValue, Params};
  use snafu::{OptionExt, ResultExt};

  use crate::error::{ChartSnafu, NoChartSnafu, NoSlideSnafu, NoTableSnafu, ResponseSnafu, Result};
  use crate::request::View;
  use crate::Session;

  /// Widgets de base sous lesquels on montre des données.
  const CHART: &str = "chart";
  const TABLE: &str = "table";

  impl Session {
      /// Montre l'élément `element` en `view`, avec les mêmes données ; une seule étape d'annulation.
      pub fn show_as(&mut self, label: &str, slide: String, element: &str, view: View) -> Result<()> {
          let deck = self.doc.sources.deck();
          let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
          let current = located.map(|l| l.element.clone());
          let (from, to, target_kind) = match view {
              View::Table => (HostType::Chart, HostType::Table, TABLE),
              View::Chart => (HostType::Table, HostType::Chart, CHART),
          };
          let value = current.as_ref().and_then(|e| self.host_value(e, from));
          let value = match (view, value) {
              (View::Table, None) => return Err(NoChartSnafu { id: element }.build().into()),
              (View::Chart, None) => return Err(NoTableSnafu { id: element }.build().into()),
              (_, Some(v)) => v,
          };
          let converted = match view {
              View::Table => {
                  let chart = serde_json::from_value(value).ok().context(NoChartSnafu { id: element })?;
                  serde_json::to_value(oku_slides_chart::chart_to_table(&chart).context(ChartSnafu)?)
              }
              View::Chart => {
                  let table = serde_json::from_value(value).ok().context(NoTableSnafu { id: element })?;
                  serde_json::to_value(oku_slides_chart::table_to_chart(&table, &self.chart_labels).context(ChartSnafu)?)
              }
          }
          .context(ResponseSnafu)?;
          let param = self
              .registry
              .get(target_kind)
              .and_then(|w| w.host_param(to))
              .map(|p| p.name.clone())
              .context(NoChartSnafu { id: element })?;
          let Some(mut next) = current else { return Err(NoChartSnafu { id: element }.build().into()) };
          next.kind = target_kind.to_owned();
          next.params = Params::default();
          next.params.set(param, ParamValue::Value(converted));
          self.apply(label, vec![Op::ReplaceElement { slide, element: next }], None)
      }

      /// La valeur du paramètre `host(host)` de `e`, si son widget en a un.
      fn host_value(&self, e: &Element, host: HostType) -> Option<serde_json::Value> {
          let param = self.registry.get(&e.kind)?.host_param(host)?;
          match e.params.get(&param.name) {
              Some(ParamValue::Value(v)) => Some(v.clone()),
              _ => None,
          }
      }
  }
  ```

  Remarques :
  - Prendre les noms exacts des sélecteurs et des champs dans `set_chart_kind` (`lib.rs:808-829`), qui fait la même chose : `self.doc.sources.deck()`, `self.registry`, `self.chart_labels`, `self.apply`, `NoSlideSnafu`, `ChartSnafu` et `ResponseSnafu`.
  - Si `Params::default()` n'existe pas, voir `params.rs` pour son constructeur vide.
  - Si un champ est privé au module `lib`, utiliser `pub(crate)`, comme le font les autres `impl Session` dans des modules séparés (`deck_pack.rs`, `merge.rs`).

  Dans `lib.rs` :
  - ajouter `mod tables;` ;
  - à côté du bras `Request::ChartKind` (l. 643), ajouter `Request::ShowAs { label, slide, element, view } => self.show_as(&label, slide, &element, view)?,`.

- [ ] **Step 4 : MCP.**

  Dans `edit_tools.rs`, après `ChartKindArgs` :

  ```rust
  #[derive(Debug, Deserialize, JsonSchema)]
  pub(crate) struct ShowAsArgs {
      pub slide: String,
      pub element: String,
      /// `table` ou `chart` ; les données sont gardées.
      pub view: oku_slides_session::View,
  }
  ```

  et, après `chart_kind` :

  ```rust
      pub(crate) async fn show_as(&self, args: Value) -> CallToolResult {
          let ShowAsArgs { slide, element, view } = match parse(args) {
              Ok(a) => a,
              Err(r) => return r,
          };
          let label = self.localizer.tr("agent-undo-apply");
          self.change(Request::ShowAs { label, slide, element, view }).await
      }
  ```

  - `handler.rs:56` : `"show_as" => self.show_as(args).await,`.
  - `tools.rs:202` : `tool::<crate::edit_tools::ShowAsArgs>("show_as", "Show a chart as a table, or a table as a chart, keeping its data; one undo step."),`.
  - `tools.rs:201` : après la phrase sur `set_data`, ajouter ` set_table_option changes a table's display (header-row, header-column, striped, align: auto|start|center|end per column) as a JSON merge patch; table data is the same dataset as a chart's.`.

- [ ] **Step 5 : lancer les tests et constater qu'ils passent.**

  ```bash
  cargo nextest run -p oku-slides-session -p oku-slides-mcp -p oku-slides-i18n
  cargo clippy -p oku-slides-session -p oku-slides-mcp --all-targets -- -D warnings
  cargo nextest run --workspace --exclude oku-slides-app
  ```

  Résultat attendu : PASS. `wc -l crates/oku-slides-session/src/lib.rs` doit rester sous 1000.

- [ ] **Step 6 : commit.**

  ```bash
  git add crates/oku-slides-session crates/oku-slides-mcp crates/oku-slides-i18n
  git diff --cached --name-only
  git commit -m "feat(session) : requête show_as, un graphique s'affiche en tableau et inversement, en une étape (t-578)"
  ```

---

### Task 6 : UI — insertion et panneau du tableau

**Files:**
- Create :
  - `ui/src/ownPanel.ts` ;
  - `ui/src/ownPanel.test.ts` ;
  - `ui/src/table/tableDeck.ts` ;
  - `ui/src/table/tableDeck.test.ts` ;
  - `ui/src/table/TablePanel.tsx` ;
  - `ui/src/table/TablePanel.render.test.tsx`.
- Modify :
  - `ui/src/slideTools.ts:6` et `ui/src/slideTools.test.ts:7,11` ;
  - `ui/src/newElement.ts` et `ui/src/newElement.test.ts` ;
  - `ui/src/icons.tsx` : `Icon.table` ;
  - `ui/src/TargetPanel.tsx` et `ui/src/TargetPanel.render.test.tsx` ;
  - `ui/src/useTargetSettings.ts:22,38` et `ui/src/useTargetSettings.render.test.tsx` ;
  - `ui/src/chart/ChartDataGrid.tsx` : la prop `newColumnName` ;
  - `ui/src/chart/ChartDataGrid.css` : `.table-align` ;
  - `ui/src/table/cells.ts` : l'en-tête du module ;
  - `ui/e2e/toolbar.cjs:20,24` ;
  - `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 fichiers).

**Interfaces :**
- Consumes : `ChartDataGrid` (props `source`, `onOp`, `firstColumnIsLabel`, `checkboxColumns`), `DataOp`, et les ops `set_table_option` et `set_data` sur le param `data`.
- Produces :
  - dans `ownPanel.ts` : `hasOwnPanel(kind: string | undefined): boolean` ;
  - dans `tableDeck.ts` :
    - `type ColumnAlign` et `COLUMN_ALIGNS` ;
    - `type TableOption` ;
    - `type TableFlag = "header-row" | "header-column" | "striped"` et `TABLE_DEFAULTS` ;
    - `flag(option, key): boolean` ;
    - `tableOption(el): TableOption | null` ;
    - `starterTable(): TableOption` ;
    - `columnAlign(option, i): ColumnAlign` ;
    - `alignPatch(option, i, a): { align: ColumnAlign[] | null }` ;
    - `asColumnAlign(v: string): ColumnAlign` ;
  - le composant `TablePanel({ slide, element, onApply })`, auquel la tâche 7 ajoute `onShowAs` ;
  - dans `ChartDataGrid`, la prop `newColumnName?: (n: number) => string`, dont le défaut est le nom de série actuel.

- [ ] **Step 1 : écrire les tests qui échouent.**

  Fichier `ownPanel.test.ts` :

  ```ts
  import { describe, expect, test } from "bun:test";

  import { hasOwnPanel } from "./ownPanel";

  describe("hasOwnPanel", () => {
    test("hasOwnPanel chart or table returns true", () => {
      expect([hasOwnPanel("chart"), hasOwnPanel("table")]).toEqual([true, true]);
    });

    test("hasOwnPanel other or missing kind returns false", () => {
      expect([hasOwnPanel("text"), hasOwnPanel(undefined)]).toEqual([false, false]);
    });
  });
  ```

  Fichier `table/tableDeck.test.ts` :

  ```ts
  import { describe, expect, test } from "bun:test";

  import { alignPatch, asColumnAlign, columnAlign, flag, starterTable, tableOption, type TableOption } from "./tableDeck";

  const option: TableOption = { dataset: { source: [["A", "B", "C"], [1, 2, 3]] }, align: ["end"] };

  describe("tableDeck", () => {
    test("tableOption of a table element returns its data", () => {
      expect(tableOption({ id: "t", kind: "table", params: { data: option } })?.dataset.source[0]).toEqual(["A", "B", "C"]);
    });

    test("tableOption of a chart returns null", () => {
      expect(tableOption({ id: "g", kind: "chart", params: { option: {} } })).toBeNull();
    });

    test("tableOption with a non-array source returns null", () => {
      expect(tableOption({ id: "t", kind: "table", params: { data: { dataset: { source: "x" } } } })).toBeNull();
    });

    test("flag without value returns the default", () => {
      expect([flag(option, "header-row"), flag(option, "header-column"), flag(option, "striped")]).toEqual([true, false, false]);
    });

    test("flag with a value returns it", () => {
      expect(flag({ ...option, "header-row": false }, "header-row")).toBe(false);
    });

    test("columnAlign without entry is auto", () => {
      expect([columnAlign(option, 0), columnAlign(option, 2)]).toEqual(["end", "auto"]);
    });

    test("alignPatch on one column keeps the others", () => {
      expect(alignPatch(option, 2, "center")).toEqual({ align: ["end", "auto", "center"] });
    });

    test("alignPatch back to auto trims trailing autos to null", () => {
      expect(alignPatch(option, 0, "auto")).toEqual({ align: null });
    });

    test("asColumnAlign of an unknown value returns auto", () => {
      expect([asColumnAlign("center"), asColumnAlign("left")]).toEqual(["center", "auto"]);
    });

    test("starterTable has three rows of the same width", () => {
      const s = starterTable().dataset.source;
      expect([s.length, new Set(s.map((r) => r.length)).size]).toEqual([3, 1]);
    });
  });
  ```

  Fichier `table/TablePanel.render.test.tsx` :

  ```tsx
  // Rendu du panneau de tableau dans un DOM happy-dom.

  import { afterEach, describe, expect, test } from "bun:test";
  import "../test/dom";

  const { act, createElement } = await import("react");
  const { createRoot } = await import("react-dom/client");
  const { TablePanel } = await import("./TablePanel");
  const { expectIconOnly } = await import("../test/iconOnly");

  type Root = ReturnType<typeof createRoot>;
  let mounted: { root: Root; host: HTMLElement } | null = null;
  afterEach(async () => {
    const m = mounted;
    mounted = null;
    if (!m) return;
    await act(async () => m.root.unmount());
    m.host.remove();
  });

  type Applied = { label: string; ops: Record<string, unknown>[] };
  const target = { slide: "s1", element: "t1" };

  async function mount(data: Record<string, unknown> = { dataset: { source: [["A", "B"], ["x", 1]] } }) {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const applied: Applied[] = [];
    const element = { id: "t1", kind: "table" as const, params: { data } };
    const onApply = (label: string, ops: Record<string, unknown>[]) => applied.push({ label, ops });
    await act(async () => {
      root.render(createElement(TablePanel, { slide: "s1", element, onApply }));
    });
    return { host, applied };
  }

  const options = (host: HTMLElement) => [...host.querySelectorAll(".table-options button")] as HTMLButtonElement[];

  describe("TablePanel", () => {
    test("TablePanel option buttons show icons only", async () => {
      const { host } = await mount();
      expect(options(host)).toHaveLength(3);
      for (const b of options(host)) expectIconOnly(b);
    });

    test("TablePanel defaults press header row only", async () => {
      const { host } = await mount();
      expect(options(host).map((b) => b.getAttribute("aria-pressed"))).toEqual(["true", "false", "false"]);
    });

    test("TablePanel striped toggle sends a set_table_option patch to the table", async () => {
      const { host, applied } = await mount();
      await act(async () => options(host)[2].click());
      expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "data", patch: { striped: true } });
    });

    test("TablePanel header row off toggle sends header-row true", async () => {
      const { host, applied } = await mount({ dataset: { source: [["A"], [1]] }, "header-row": false });
      await act(async () => options(host)[0].click());
      expect(applied[0].ops[0]).toMatchObject({ patch: { "header-row": true } });
    });

    test("TablePanel alignment select sends the align patch", async () => {
      const { host, applied } = await mount();
      const select = host.querySelectorAll(".table-align select")[1] as HTMLSelectElement;
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(select, "center");
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
      expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "data", patch: { align: ["auto", "center"] } });
    });

    test("TablePanel add column sends set_data on the data param with a column name", async () => {
      const { host, applied } = await mount();
      const [, addColumn] = host.querySelectorAll(".cdg-actions button");
      await act(async () => (addColumn as HTMLButtonElement).click());
      expect(applied[0].ops[0]).toMatchObject({ op: "set_data", target, param: "data", change: { op: "insert_column", at: 2 } });
      const cells = (applied[0].ops[0].change as { cells: string[] }).cells;
      expect(cells[0]).not.toContain("Series");
    });

    test("TablePanel without data shows the no-data hint", async () => {
      const { host } = await mount({});
      expect(host.querySelector(".a2ui-hint")).not.toBeNull();
    });
  });
  ```

  Dans `TargetPanel.render.test.tsx`, sur le modèle du test du graphique qui s'y trouve (l. 72-79), ajouter `TargetPanel table selected shows TablePanel only` : `.table-panel` présent, `.host-settings` absent.

  Dans `useTargetSettings.render.test.tsx`, sur le modèle de la l. 121 (le cas du graphique), ajouter `useTargetSettings table selected sends no request`.

  Dans `newElement.test.ts`, en suivant le style du fichier :

  ```ts
  test("table : un tableau de départ dans params.data", () => {
    const e = newElement("table", "t1");
    expect([e.kind, (e.params?.data as { dataset: { source: unknown[] } }).dataset.source.length]).toEqual(["table", 3]);
  });
  ```

  Dans `slideTools.test.ts` :
  - l. 7 : `["text", "card", "shape", "chart", "table"]` ;
  - l. 11 : `["card", "shape", "chart", "table"]`.

- [ ] **Step 2 : lancer les tests et constater l'échec.**

  ```bash
  cd ui && bun test src/ownPanel.test.ts src/table/tableDeck.test.ts src/newElement.test.ts src/slideTools.test.ts
  cd ui && bun test src/table/TablePanel.render.test.tsx
  ```

  Résultat attendu : FAIL. Les modules n'existent pas et `ADD_KINDS` ne contient pas encore `table`.

- [ ] **Step 3 : écrire `ownPanel.ts` et `tableDeck.ts`.**

  ```ts
  // Sortes d'élément qui ont leur propre panneau à la place des réglages génériques.

  const OWN_PANEL: readonly string[] = ["chart", "table"];

  /** Vrai pour un graphique ou un tableau : leur panneau remplace les réglages de la cible. */
  export const hasOwnPanel = (kind: string | undefined): boolean => kind !== undefined && OWN_PANEL.includes(kind);
  ```

  ```ts
  // Tableaux dans le deck, côté interface : lecture d'un élément `table` (param `data`).
  // Le cœur Rust valide et applique ; le tableau de départ est nommé ici, dans la
  // langue de l'utilisateur, faute de canonicalisation des tableaux côté session.

  import type { Element } from "../api";
  import type { Cell } from "../chart/chartTypes";
  import { t } from "../i18n";

  export type ColumnAlign = "auto" | "start" | "center" | "end";
  export const COLUMN_ALIGNS: readonly ColumnAlign[] = ["auto", "start", "center", "end"];

  export type TableFlag = "header-row" | "header-column" | "striped";

  /** Miroir de `TableOption` (oku-slides-chart) : ligne 0 = noms de colonnes. */
  export type TableOption = {
    dataset: { source: Cell[][] };
    "header-row"?: boolean;
    "header-column"?: boolean;
    striped?: boolean;
    align?: ColumnAlign[];
  };

  /** Valeurs par défaut des drapeaux, celles de `TableOption` côté Rust. */
  export const TABLE_DEFAULTS: Readonly<Record<TableFlag, boolean>> = { "header-row": true, "header-column": false, striped: false };

  /** Le drapeau `key`, ou sa valeur par défaut. */
  export const flag = (option: TableOption, key: TableFlag): boolean => option[key] ?? TABLE_DEFAULTS[key];

  /** Options d'un élément `table`, ou `null`. */
  export function tableOption(el: Element): TableOption | null {
    const data: unknown = el.params?.data;
    if (el.kind !== "table" || typeof data !== "object" || data === null) return null;
    const source = (data as { dataset?: { source?: unknown } }).dataset?.source;
    return Array.isArray(source) ? (data as TableOption) : null;
  }

  /** Tableau neuf : trois colonnes nommées, deux lignes. */
  export function starterTable(): TableOption {
    const header = [1, 2, 3].map((n) => t("ui-table-column", { n }));
    return { dataset: { source: [header, ["", 0, 0], ["", 0, 0]] } };
  }

  /** Alignement de la colonne `i` ; sans entrée, `auto`. */
  export const columnAlign = (option: TableOption, i: number): ColumnAlign => option.align?.[i] ?? "auto";

  /** Une valeur de `<select>` ramenée à un alignement connu, sinon `auto`. */
  export const asColumnAlign = (v: string): ColumnAlign => COLUMN_ALIGNS.find((a) => a === v) ?? "auto";

  /**
   * Patch `set_table_option` qui aligne la colonne `i`. Un merge patch remplace
   * le tableau entier ; les `auto` de fin sont retirés (`null` : plus d'entrée).
   */
  export function alignPatch(option: TableOption, i: number, a: ColumnAlign): { align: ColumnAlign[] | null } {
    const width = option.dataset.source[0]?.length ?? 0;
    const align = Array.from({ length: width }, (_, j) => (j === i ? a : columnAlign(option, j)));
    while (align.length > 0 && align[align.length - 1] === "auto") align.pop();
    return { align: align.length > 0 ? align : null };
  }
  ```

- [ ] **Step 4 : `ChartDataGrid`, nom des nouvelles colonnes.**
  - Ajouter la prop `/** Nom d'une colonne ajoutée ou collée sans en-tête (\`n\` : son index) ; par défaut « Série n ». */ newColumnName?: (n: number) => string;`.
  - Dans le composant : `newColumnName = seriesName`, avec `const seriesName = (n: number) => t("ui-chart-series", { n });` au niveau du module.
  - Remplacer `t("ui-chart-series", { n: width })` (l. 156) par `newColumnName(width)`.
  - À la l. 255 (`pasteInto`), passer `newColumnName` en argument, comme `firstColumnIsLabel`, et remplacer `t("ui-chart-series", { n: c })` par `newColumnName(c)`.
  - Ajouter `newColumnName` aux dépendances du `useMemo` (l. 109).

- [ ] **Step 5 : écrire `TablePanel.tsx`.**

  ```tsx
  // Panneau d'un tableau sélectionné : options d'affichage, alignement des colonnes
  // et grille de ses données (celle des graphiques). Chaque modification part en op.

  import type { ReactNode } from "react";

  import type { Element, Op } from "../api";
  import { ChartDataGrid } from "../chart/ChartDataGrid";
  import { t } from "../i18n";
  import { stroke } from "../icons";
  import { COLUMN_ALIGNS, type TableFlag, alignPatch, asColumnAlign, columnAlign, flag, tableOption } from "./tableDeck";

  const FLAGS: { key: TableFlag; label: string; icon: ReactNode }[] = [
    { key: "header-row", label: "ui-table-header-row",
      icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 10h16" strokeWidth="3" /></>) },
    { key: "header-column", label: "ui-table-header-column",
      icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M9 5v14" strokeWidth="3" /></>) },
    { key: "striped", label: "ui-table-striped",
      icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 9.5h16M4 14.5h16" /></>) },
  ];

  const tableColumnName = (n: number) => t("ui-table-column", { n: n + 1 });

  export function TablePanel({ slide, element, onApply }: {
    slide: string;
    element: Element;
    onApply: (label: string, ops: Op[]) => void;
  }) {
    const option = tableOption(element);
    if (!option) return <p className="a2ui-hint">{t("ui-table-no-data")}</p>;
    const target = { slide, element: element.id };
    const patch = (label: string, p: Record<string, unknown>) =>
      onApply(label, [{ op: "set_table_option", target, param: "data", patch: p }]);
    const header = option.dataset.source[0] ?? [];
    return (
      <section className="table-panel chart-panel" aria-label={t("ui-table-panel")}>
        <div className="table-options chart-options" role="group" aria-label={t("ui-table-options")}>
          {FLAGS.map(({ key, label, icon }) => {
            const pressed = flag(option, key);
            return (
              <button key={key} aria-label={t(label)} title={t(label)} aria-pressed={pressed} className={pressed ? "on" : ""}
                onClick={() => patch(t("ui-undo-table-option"), { [key]: !pressed })}>
                {icon}
              </button>
            );
          })}
        </div>
        <div className="table-align" role="group" aria-label={t("ui-table-alignment")}>
          {header.map((name, i) => {
            const shown = String(name ?? "").trim() || String(i + 1);
            return (
              <label key={i}>
                <span>{shown}</span>
                <select aria-label={t("ui-table-align-of", { name: shown })} value={columnAlign(option, i)}
                  onChange={(e) => patch(t("ui-undo-table-align"), alignPatch(option, i, asColumnAlign(e.target.value)))}>
                  {COLUMN_ALIGNS.map((a) => <option key={a} value={a}>{t("ui-table-align", { align: a })}</option>)}
                </select>
              </label>
            );
          })}
        </div>
        <ChartDataGrid
          source={option.dataset.source}
          firstColumnIsLabel={flag(option, "header-column")}
          newColumnName={tableColumnName}
          onOp={(change) => onApply(t("ui-undo-table-data"), [{ op: "set_data", target, param: "data", change }])}
        />
      </section>
    );
  }
  ```

  Remarques :
  - `{ [key]: !pressed }` a une clé typée `TableFlag`, donc aucune clé arbitraire ne peut venir des données.
  - Les noms de colonnes sont posés en nœuds texte React, jamais en HTML.

- [ ] **Step 6 : brancher l'insertion et le panneau.**
  - `slideTools.ts:6` : `export const ADD_KINDS = ["text", "card", "shape", "chart", "table"] as const;`.
  - `newElement.ts` : importer `starterTable` depuis `./table/tableDeck`, puis ajouter :

    ```ts
        case "table":
          return { id, kind: "table", params: { data: starterTable() } };
    ```

  - `icons.tsx`, après `chart` : `table: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 10h16M4 14.5h16M10 5v14" /></>),`.
  - `TargetPanel.tsx` : importer `TablePanel`. Après le bras du graphique, ajouter `if (selected?.kind === "table") return <TablePanel slide={view.slide.id} element={selected} onApply={onApply} />;`. Mettre à jour la doc de tête : « le panneau d'un graphique ou d'un tableau sélectionné ».
  - `useTargetSettings.ts:38` : `element?.kind !== "chart"` devient `!hasOwnPanel(element?.kind)`, importé depuis `./ownPanel`. À la l. 22, la doc devient « Un graphique ou un tableau sélectionné ne demande rien ».
  - `ChartDataGrid.css`, avec les règles `.chart-panel`. Les jetons sont ceux qui existent (`--space-1/2/4/6/10`, voir `styles.css:71-75`), et `tableTokens.test.ts` couvre ce fichier.

    ```css
    .table-align { display: grid; grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr)); gap: var(--space-2); }
    .table-align label { display: grid; gap: var(--space-1); font-size: 12px; }
    ```

  - Ne pas toucher aux sélecteurs `.chart-panel` de `styles.css` : `TablePanel` porte déjà la classe `chart-panel`.
  - `ui/src/table/cells.ts`, l. 1-2 : « Cellules d'une grille : collage TSV et conversion d'une saisie, partagés par les tableaux du dock, la grille des graphiques et celle du widget tableau. ».
  - `ui/e2e/toolbar.cjs:20` : `[false, false, false, false, false]` ; l. 24 : `[true, true, true, true, true]`.

- [ ] **Step 7 : i18n UI.** Dans les 20 `ui.ftl` :
  - `ui-add-kind` : la branche `[table] Table` (fr : `[table] Tableau`), après `[chart]` ;
  - `ui-undo-add` : la branche `[table] Add a table` (fr : `[table] Ajouter un tableau`), après `[chart]` ;
  - une section `## Table` juste après la section `## Chart`, au même endroit dans tous les fichiers.

  en :

  ```
  ## Table

  ui-table-panel = Table
  ui-table-options = Table options
  ui-table-header-row = Header row
  ui-table-header-column = First column as header
  ui-table-striped = Striped rows
  ui-table-alignment = Column alignment
  ui-table-align-of = Alignment of { $name }
  ui-table-align = { $align ->
      [start] Start
      [center] Center
      [end] End
     *[auto] Automatic
  }
  ui-table-column = Column { $n }
  ui-table-no-data = This table has no data.
  ui-undo-table-option = Table options
  ui-undo-table-align = Align the column
  ui-undo-table-data = Edit the table
  ```

  fr :

  ```
  ## Table

  ui-table-panel = Tableau
  ui-table-options = Options du tableau
  ui-table-header-row = Ligne d'en-tête
  ui-table-header-column = Première colonne en en-tête
  ui-table-striped = Lignes zébrées
  ui-table-alignment = Alignement des colonnes
  ui-table-align-of = Alignement de { $name }
  ui-table-align = { $align ->
      [start] Début
      [center] Centre
      [end] Fin
     *[auto] Automatique
  }
  ui-table-column = Colonne { $n }
  ui-table-no-data = Ce tableau n'a pas de données.
  ui-undo-table-option = Options du tableau
  ui-undo-table-align = Aligner la colonne
  ui-undo-table-data = Modifier le tableau
  ```

  Traduire ces clés dans les 18 autres locales, dans le même ordre.

- [ ] **Step 8 : lancer les tests et constater qu'ils passent.**

  ```bash
  cd ui && bun run test && bun run typecheck
  cargo nextest run -p oku-slides-i18n
  ```

  Résultat attendu : PASS. `i18n-keys.test.ts`, qui compare l'ordre des clés, `tableTokens.test.ts` et `ChartPanel.render.test.tsx` passent. Ce dernier garde son nom de série par défaut.

- [ ] **Step 9 : commit.**

  ```bash
  git add ui/src ui/e2e/toolbar.cjs crates/oku-slides-i18n
  git diff --cached --name-only
  git commit -m "feat(ui) : insérer un tableau et l'éditer dans son panneau, avec la grille des graphiques (t-578)"
  ```

---

### Task 7 : UI — « Afficher en tableau » / « Afficher en graphique »

**Files:**
- Create : `ui/src/table/ShowAsButton.tsx`.
- Modify :
  - `ui/src/api.ts` : le type `Request` (l. 523) et la méthode `showAs` (l. 672) ;
  - `ui/src/chart/ChartPanel.tsx` et `ui/src/chart/ChartPanel.render.test.tsx` ;
  - `ui/src/table/TablePanel.tsx` et `ui/src/table/TablePanel.render.test.tsx` ;
  - `ui/src/TargetPanel.tsx` : la prop `onShowAs` ;
  - `ui/src/App.tsx` : à côté de `onKind`, l. 885 de `HEAD` ;
  - `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 fichiers).

**Interfaces :**
- Consumes : la requête `show_as` (tâche 5), en JSON `{ cmd: "show_as", label, slide, element, view }`.
- Produces :
  - `api.showAs(label, slide, element, view: "chart" | "table")` ;
  - `ShowAsButton({ view, onShowAs })` ;
  - la prop `onShowAs: (view: "chart" | "table") => void`, sur `ChartPanel`, `TablePanel` et `TargetPanel`.

- [ ] **Step 1 : écrire les tests qui échouent.**

  Dans `ChartPanel.render.test.tsx` :
  - `mount` gagne un second argument facultatif `onShowAs = (_view: string) => {}`, passé à `ChartPanel` ;
  - le test existant `ChartPanel stack option shows an icon only` cible `.chart-stack button` à la place de `.chart-options button` ;
  - ajouter :

    ```tsx
      test("ChartPanel show-as-table button shows an icon and asks for the table view", async () => {
        const views: string[] = [];
        const host = await mount([{ type: "bar" }], (view: string) => views.push(view));
        const button = host.querySelector(".show-as button") as HTMLButtonElement;
        expectIconOnly(button);
        await act(async () => button.click());
        expect(views).toEqual(["table"]);
      });
    ```

  Dans `TablePanel.render.test.tsx`, `mount` gagne de même un second argument `onShowAs`. Ajouter :

  ```tsx
    test("TablePanel show-as-chart button asks for the chart view", async () => {
      const views: string[] = [];
      const { host } = await mount(undefined, (view: string) => views.push(view));
      const button = host.querySelector(".show-as button") as HTMLButtonElement;
      expectIconOnly(button);
      await act(async () => button.click());
      expect(views).toEqual(["chart"]);
    });
  ```

  Dans le corps de `mount`, écrire `data` avec sa valeur par défaut, pour que `undefined` la donne.

- [ ] **Step 2 : lancer les tests et constater l'échec.**

  ```bash
  cd ui && bun test src/chart/ChartPanel.render.test.tsx && bun test src/table/TablePanel.render.test.tsx
  ```

  Résultat attendu : FAIL. Le sélecteur `.chart-stack` et le bouton `.show-as` sont absents.

- [ ] **Step 3 : écrire `ShowAsButton.tsx`.**

  ```tsx
  // Bouton « Afficher en tableau » / « Afficher en graphique » : la session convertit
  // (requête `show_as`), en une étape d'annulation.

  import { t } from "../i18n";
  import { Icon } from "../icons";

  export type View = "chart" | "table";

  export function ShowAsButton({ view, onShowAs }: { view: View; onShowAs: (view: View) => void }) {
    const label = t(view === "table" ? "ui-show-as-table" : "ui-show-as-chart");
    return (
      <div className="show-as chart-options" role="group" aria-label={t("ui-show-as")}>
        <button aria-label={label} title={label} onClick={() => onShowAs(view)}>
          {view === "table" ? Icon.table : Icon.chart}
        </button>
      </div>
    );
  }
  ```

- [ ] **Step 4 : brancher le bouton.**
  - `ChartPanel.tsx` :
    - prop `/** Affiche le graphique en tableau (requête \`show_as\`). */ onShowAs: (view: View) => void` ;
    - le groupe de l'empilement prend `className="chart-stack chart-options"` ;
    - après le groupe `chart-kinds`, ajouter `<ShowAsButton view="table" onShowAs={onShowAs} />`, avec `disabled` si le graphique n'a pas de données. Pour cela, `ShowAsButton` reçoit une prop `disabled?: boolean`, passée à `<button disabled={disabled}>` ; l'ajouter au composant.
  - `TablePanel.tsx` : même prop, et `<ShowAsButton view="chart" onShowAs={onShowAs} />` juste après `div.table-options`.
  - `TargetPanel.tsx` : prop `onShowAs: (view: View) => void`, passée aux deux panneaux.
  - `api.ts` :
    - dans `Request`, après `chart_kind` : `| { cmd: "show_as"; label: string; slide: string; element: string; view: "chart" | "table" }` ;
    - dans `tabApi`, après `chartKind` : `showAs: (label: string, slide: string, element: string, view: "chart" | "table") => call<Snapshot>({ cmd: "show_as", label, slide, element, view }),`.
  - `App.tsx`, là où `TargetPanel` reçoit `onKind` :

    ```tsx
    onShowAs={(v) => view && selectedElement && run(api.showAs(t(v === "table" ? "ui-undo-show-as-table" : "ui-undo-show-as-chart"), view.slide.id, selectedElement.id, v))}
    ```

    `run` affiche déjà l'erreur d'une requête refusée, par exemple un graphique de plus de 20 colonnes ou un tableau dont une colonne de valeurs contient du texte.

- [ ] **Step 5 : i18n.** Dans les 20 `ui.ftl`, à la fin de la section `## Table` ajoutée à la tâche 6 :

  en :

  ```
  ui-show-as = Show as
  ui-show-as-table = Show as a table
  ui-show-as-chart = Show as a chart
  ui-undo-show-as-table = Show as a table
  ui-undo-show-as-chart = Show as a chart
  ```

  fr :

  ```
  ui-show-as = Afficher en
  ui-show-as-table = Afficher en tableau
  ui-show-as-chart = Afficher en graphique
  ui-undo-show-as-table = Afficher en tableau
  ui-undo-show-as-chart = Afficher en graphique
  ```

  Traduire dans les 18 autres locales, dans le même ordre. Les clés `ui-undo-*` restent séparées, comme dans le reste du fichier.

- [ ] **Step 6 : lancer les tests et constater qu'ils passent.**

  ```bash
  cd ui && bun run test && bun run typecheck
  cargo nextest run -p oku-slides-i18n
  ```

  Résultat attendu : PASS.

- [ ] **Step 7 : commit.**

  ```bash
  git add ui/src crates/oku-slides-i18n
  git diff --cached --name-only
  git commit -m "feat(ui) : afficher un graphique en tableau et un tableau en graphique (t-578)"
  ```

---

### Task 8 : Aide, consolidation et vérification complète

**Files:**
- Modify :
  - `docs/aide/en.json` et `docs/aide/fr.json` : l. 146, 1572, 1576 et 1851 ;
  - le paragraphe sur le tableau dans la slide de l'aide qui décrit les éléments, celle de la l. 146.

- [ ] **Step 1 : aide.** Dans les deux langues :
  - l. 146 : « adds a text, card, shape, chart… » devient « adds a text, card, shape, chart, table… » ;
  - l. 1572 : « 13 base widgets » / « 13 widgets de base » devient 14 ;
  - l. 1576 : `table` après `chart` dans la liste des widgets ;
  - l. 1851 : « Host-provided type (chart) » devient « (chart, table) », et en français « (graphique, tableau) ».

  Dans la slide de la l. 146, ajouter un paragraphe après celui qui la contient, au format JSON des paragraphes voisins (`runs`).

  en : « Table: a data table edited in its panel with the same grid as charts. Toggle the header row, the first column as header and striped rows; align each column (automatic puts numbers at the end). A table larger than its box shrinks to fit. “Show as a chart” and “Show as a table” switch between the two, keeping the data, in one undo step. »

  fr : « Tableau : un tableau de données édité dans son panneau avec la même grille que les graphiques. Activez la ligne d'en-tête, la première colonne en en-tête et les lignes zébrées ; alignez chaque colonne (automatique place les nombres à la fin). Un tableau plus grand que son cadre est réduit pour y tenir. « Afficher en graphique » et « Afficher en tableau » passent de l'un à l'autre en gardant les données, en une seule étape d'annulation. »

  ```bash
  cargo nextest run -p oku-slides-session help
  ```

  Résultat attendu : PASS. `help_tests.rs` vérifie la parité des slides entre les deux langues.

- [ ] **Step 2 : consolidation (G10).**

  ```bash
  git grep -n -E "kind === \"chart\"|kind == \"chart\"|HostType::Chart|host: HostType" -- crates ui/src typst
  ```

  Chaque résultat restant doit être propre au graphique : dessin SVG, `chart_kind`, `grow` et canonicalisation. Aucun ne doit exclure le tableau à tort.

  ```bash
  git grep -n "fn chart_param\|fn host_param\|fn host_of" -- crates
  ```

  Résultat attendu : `host_param` dans `widget.rs` et `ops/host.rs` (privé, signature différente), et `chart_param` dans les crates de rendu et de session, qui passent tous les deux par `WidgetSchema::host_param`. Aucun `host_of`.

- [ ] **Step 3 : vérification complète.**

  ```bash
  just check
  just test
  ```

  Résultat attendu : tout passe. Lancer les e2e uniquement si on les demande, sur un port libre (le port 4872 entre en collision entre sessions parallèles) et avec des données isolées.

- [ ] **Step 4 : commit.**

  ```bash
  git add docs/aide
  git diff --cached --name-only
  git commit -m "docs(aide) : le widget tableau et la conversion graphique ↔ tableau (t-578)"
  ```
