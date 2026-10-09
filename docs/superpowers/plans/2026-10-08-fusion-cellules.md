# Fusion fine des éditions de cellules — plan d'implémentation (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** quand deux personnes modifient en même temps les données d'un tableau ou d'un graphique, leurs deux éditions sont gardées. Cela vaut pour les cellules, les lignes et les colonnes.

**Architecture :**
- Les lignes et les colonnes de `dataset.source` reçoivent des identifiants stables (`ids`, `columns`). Le cœur les pose et les garantit à chaque écriture d'un paramètre `host(..)`.
- Le replica Loro stocke `dataset` en conteneurs :
  - les colonnes forment une liste ordonnée d'identifiants ;
  - les lignes, rapprochées par identifiant, portent chacune une carte `id de colonne → cellule`.
- À la relecture, le replica répare un état fusionné qui serait incohérent.
- Pas de rétrocompatibilité : le format passe en v5/v7.

**Tech Stack :** Rust, Loro 1.16.2 (`LoroMap`, `LoroMovableList`), serde, schemars, snafu, speculoos, proptest ; TS pour les types miroirs de l'UI.

**Spec :** `docs/superpowers/specs/2026-10-08-fusion-cellules-design.md` (v2). Ce plan remplace la v1, qui a été relue par l'équipe et dont les retours sont intégrés.

## Global Constraints

- **Tâche et commits** : tâche Oku **t-589**. Commits conventionnels en français, terminés par `(t-589)`. Le commit du changement de format est marqué `!`. **Jamais de `Co-Authored-By`.**
- **Hook de revue** : `oku.review=deferred` est posé pour ce worktree seulement (`--worktree`). Faire `git add` à part, puis `git diff --cached --name-only` juste avant `git commit`.
- **Commits verts** : chaque commit laisse l'espace de travail vert. Avant de commiter, lancer :
  - `cargo nextest run --workspace --exclude oku-slides-app` ;
  - `cargo clippy --workspace --all-targets -- -D warnings` ;
  - `cargo fmt --check` ;
  - si l'UI change : `cd ui && bun run test && bun run typecheck`.
- **Identifiants** :
  - 8 caractères hexadécimaux, `uuid::Uuid::new_v4().simple()`, puis `[..8]` ;
  - tirés à nouveau tant qu'ils sont déjà pris dans la même liste ;
  - format admis : `[A-Za-z0-9_-]{1,64}` ;
  - uniques par liste (`ids`, `columns`).
- **Bornes de données** :
  - tableau : 100 lignes × 20 colonnes, comme aujourd'hui ;
  - lecture du replica : au plus 10 000 lignes et 1 000 colonnes, au-delà c'est tronqué et signalé ;
  - graphique : pas de nouvelle borne d'édition, mais la réconciliation reste linéaire (index par `HashMap`).
- **Pas de panique** : aucun `insert`, `remove` ou `[i]` sur `ids`/`columns` sans vérifier la longueur avant. Pas d'`unwrap()` nu, ni dans le code, ni dans les aides de `tests/*.rs`.
- **Documentation** : la première phrase d'une doc fait moins de 15 mots. Les tests sont nommés `<unité>_<scénario>_<résultat>`.
- **i18n** : toute clé ajoutée va dans les 20 locales, au même endroit, traduite.
- **Taille des fichiers** : `core/ARCHITECTURE.md` fait 57 lignes, au-dessus de la limite de 50. Le replica part dans `crates/oku-slides-core/src/replica/ARCHITECTURE.md`, et le fichier du cœur n'en garde qu'une ligne de renvoi.

---

### Task 1 : ADR-0004 et passage du format en v5/v7

**Files :**
- Create : `docs/adr/0004-donnees-des-hotes-fusionnees-par-cellule.md`.
- Modify :
  - `docs/adr/0002-sources-du-deck-en-crdt-loro.md` (l. 50-53, mention « remplacé en partie par l'ADR-0004 ») ;
  - `docs/adr/README.md` ;
  - `crates/oku-slides-core/src/model.rs:18` (`DECK_VERSION = 5`) ;
  - `crates/oku-slides-core/src/sources.rs:46` (`SOURCES_VERSION = 7`) ;
  - toutes les fixtures et tous les tests qui écrivent `"version": 4` ou un zip v6. Utiliser `git grep -n '"version": 4\|version: 4\|SOURCES_VERSION\|DECK_VERSION'` et migrer chaque occurrence.

- [ ] **Step 1 : écrire les tests qui échouent.** Dans `crates/oku-slides-core/src/model_tests.rs` (ou l'endroit où sont testées les versions, voir `UnsupportedDeckVersion`), ajouter :
  - `deck_v4_is_refused_as_unsupported_version` ;
  - `sources_v6_zip_is_refused`, sur le modèle d'un test existant de refus de version dans `sources_tests.rs`.

  Lancer `cargo nextest run -p oku-slides-core version`. Résultat attendu : échec (FAIL), car les versions 4 et 6 sont encore acceptées.

- [ ] **Step 2 : écrire l'ADR-0004**, avec le skill `common:new-adr` et au format de l'ADR-0002. Contenu :
  - **Contexte** : l'option d'hôte est un `Atom`, donc les éditions concurrentes se perdent.
  - **Décision** :
    - identifiants stables de lignes et de colonnes ;
    - `dataset` en conteneurs (`columns`, `rows`, cellules par clé de colonne) ;
    - réparation à la relecture ;
    - autres clés de l'option en dernière écriture gagnante, clé par clé.
  - **Invariant global** : le cœur garde l'alignement de `ids` et `columns` à chaque écriture, la relecture répare un état fusionné, et la validation (100 × 20) s'applique aux ops. Un tableau qui dépasse ces bornes après fusion est rendu en substitut jusqu'à ce qu'on supprime des lignes ou des colonnes.
  - **Compatibilité** : aucune. v5/v7, et une ancienne version refuse le nouveau format avec son erreur de version.
  - **Limites** :
    - une cellule est un scalaire, en dernière écriture gagnante ;
    - un collage garde les identifiants par position.

  Dans l'ADR-0002, l. 50-53, remplacer la phrase sur `chart.option` par un renvoi à l'ADR-0004. Ajouter la ligne de l'ADR-0004 au README.

- [ ] **Step 3 : passer les versions.** Mettre `DECK_VERSION = 5` et `SOURCES_VERSION = 7`, puis migrer les fixtures (`fixtures/*.json`, `crates/*/tests/**`, `templates/**`) et les snapshots `insta` concernés. Ne rien changer d'autre.

- [ ] **Step 4 : lancer les tests.** Tout le workspace, plus `cd ui && bun run test`, puisque des fixtures e2e ou UI peuvent porter la version. Résultat attendu : PASS.

- [ ] **Step 5 : commit.**

  ```text
  feat! : format des sources en v5/v7 pour la fusion fine des données d'hôte, ADR-0004 (t-589)
  ```

---

### Task 2 : Identifiants de lignes et de colonnes dans le modèle (`oku-slides-chart`)

**Files :**
- Modify :
  - `crates/oku-slides-chart/Cargo.toml` : ajouter `uuid.workspace = true` (il n'y est pas) ;
  - `src/option.rs` (`Dataset`) ;
  - `src/data_ops.rs` et `src/data_ops_tests.rs` ;
  - `src/model.rs` (`normalize` appelle `validate_ids`) et `src/model_tests.rs` ;
  - `src/table.rs` (`validate_table` ; `check_growth` et `table_data`, qui déstructurent `DataOp::Replace { source }` aux l. 108 et 127 et passent à `{ source, .. }` ; `align` tronqué) et `src/table_tests.rs` (l. 106, 119, 139, 155, 170) ;
  - `src/canonical.rs` (l. 106) et `src/canonical_tests.rs` ;
  - `src/convert.rs` (l. 54) et `src/convert_tests.rs` ;
  - `src/table_convert.rs` (l. 29) et `src/table_convert_tests.rs` ;
  - `src/lib.rs` et `ARCHITECTURE.md` ;
  - `crates/oku-slides-i18n/src/locales/*/chart.ftl` ;
  - `crates/oku-slides-core/tests/tables.rs:137` (`Replace`) ;
  - `ui/src/chart/chartTypes.ts` et `ui/src/table/tableDeck.ts` (types miroirs).

**Interfaces :**
- Produces :
  - `Dataset { source: Vec<Vec<Cell>>, #[serde(default, skip_serializing_if = "Vec::is_empty")] ids: Vec<String>, #[serde(default, skip_serializing_if = "Vec::is_empty")] columns: Vec<String> }`. Dériver `Default`.
  - `pub fn ensure_ids(d: &mut Dataset)` : complète ou recale `ids` sur `source.len()` et `columns` sur la largeur de la ligne 0. Elle garde les identifiants valides et uniques déjà en place, et tire les manquants. Elle est idempotente, sans panique, quelle que soit l'incohérence de départ.
  - `DataOp::InsertRow { at, cells, #[serde(default, skip_serializing_if = "Option::is_none")] id: Option<String> }` et `DataOp::InsertColumn { at, cells, id: Option<String> }`, avec les mêmes attributs. La doc de chaque champ `id` dit : « Ne pas fournir : le cœur en tire un. »
  - `DataOp::apply(&self, d: &mut Dataset) -> Result<()>` : **plus d'inverse**. Elle appelle d'abord `ensure_ids(d)`, puis applique l'op en tenant `ids` et `columns` alignés.
  - `pub(crate) fn validate_ids(d: &Dataset) -> Result<()>`.
  - Nouvelles clés i18n : `chart-data-ids-length`, `chart-data-ids-duplicate`, `chart-data-ids-invalid`, `chart-data-id-taken`.

- [ ] **Step 1 : écrire les tests qui échouent.** Dans `data_ops_tests.rs`, avec `ds()` qui porte `ids: ["h","a","b"]` et `columns: ["m","v"]` :
  - `insert_row_without_id_gets_a_fresh_hex_id` : 8 caractères hexadécimaux, distinct des autres ; une boucle de 50 insertions donne 50 identifiants uniques ;
  - `insert_row_with_id_keeps_it` ;
  - `insert_row_with_taken_id_is_refused` : clé `chart-data-id-taken` ;
  - `insert_column_without_id_gets_a_fresh_id`, `insert_column_with_id_keeps_it` ;
  - `remove_row_drops_its_id`, `remove_column_drops_its_id` ;
  - `set_cell_insert_column_remove_column_leave_row_ids_alone` ;
  - `replace_keeps_ids_by_position_and_mints_extra` : d'abord 3 × 2, puis remplacé par 4 × 3. Les 3 premiers `ids` et les 2 premières `columns` sont gardés, et les nouveaux sont neufs ;
  - `replace_with_fewer_rows_drops_extra_ids` ;
  - `apply_with_misaligned_ids_does_not_panic` : `ids` de longueur 1 pour 3 lignes. Chaque op réussit ou rend une erreur, sans panique, et l'état final est aligné ;
  - `ensure_ids_is_idempotent_and_keeps_valid_ids` ;
  - `ensure_ids_replaces_duplicate_or_invalid_ids`.

  Le test existant `apply_every_op_then_inverse_restores_dataset` disparaît, puisque `apply` ne rend plus d'inverse. Il est remplacé par un test d'état par op, comme ci-dessus.

  Dans `table_tests.rs` et `model_tests.rs`, ajouter :
  - `parse_table_ids_of_wrong_length_is_inconsistent`, `parse_table_duplicate_ids_is_inconsistent` ;
  - `parse_table_invalid_id_is_inconsistent`, avec les cas `"a b"`, `""` et un identifiant de 65 caractères, et `parse_table_id_of_64_chars_is_accepted` ;
  - les mêmes cas pour `columns`, et pour `parse_option` d'un graphique ;
  - `parse_table_align_longer_than_columns_is_truncated` (au lieu d'être refusé).

  Les tests de conversion partent d'identifiants qui ne sont pas positionnels, par exemple `["h","zz","qq"]` :
  - `canonicalize_keeps_dataset_ids` ;
  - `canonicalize_with_inline_data_over_dataset_mints_new_ids`, pour canonical.rs l. 106 ;
  - `convert_bar_to_line_keeps_ids` ;
  - `convert_to_scatter_with_changed_row_count_mints_new_ids` ;
  - `chart_to_table_keeps_ids_and_columns`, `table_to_chart_keeps_ids_and_columns`.

- [ ] **Step 2 : lancer et constater l'échec.** `cargo nextest run -p oku-slides-chart`. Résultat attendu : FAIL.

- [ ] **Step 3 : implémenter.**
  - `fn new_id(taken: &[String]) -> String` tire en boucle jusqu'à obtenir un identifiant absent de `taken`.
  - `ensure_ids` :
    - pour chaque position `i < source.len()`, garder `ids[i]` s'il existe, est valide et n'est pas un doublon d'un identifiant gardé plus tôt ;
    - sinon, tirer `new_id` ;
    - tronquer `ids` à `source.len()` ;
    - faire de même pour `columns` et la largeur de la ligne 0 (`source.first().map_or(0, Vec::len)`).
  - `apply` :
    - appeler `ensure_ids`, puis faire les contrôles de bornes existants sur `src` ;
    - insérer ou retirer dans `ids` et `columns` à la même position, en utilisant `Vec::insert`/`remove` seulement après la vérification `at <= len` ou `at < len`, que `ensure_ids` garantit ;
    - `InsertRow` ou `InsertColumn` avec un `id` déjà pris : erreur `chart-data-id-taken` ;
    - `Replace` : garder `ids[..min]` et `columns[..min]`, et tirer le reste.
  - `validate_ids` : vérifier d'abord les longueurs (retour immédiat si elles diffèrent), puis le format, puis l'unicité avec un `HashSet`. `ids` et `columns` vides sont acceptés.
    - Appelée par `normalize`, pour un graphique qui a un dataset, et par `validate_table`.
    - Dans `validate_table`, `align` n'est plus refusé s'il est trop long. `parse_table` le tronque avant de valider. Retirer la clé `chart-table-align` des 20 locales si elle ne sert plus.
  - Conversions :
    - `canonical.rs:106` : un `source` reconstruit donne `ids` et `columns` vides, que `ensure_ids` remplira au cœur ;
    - `convert.rs:54` : garder `ids` si le nombre de lignes est inchangé, `columns` si la largeur l'est, et sinon les laisser vides ;
    - `table_convert.rs:29` : recopier `ids` et `columns`.
  - Types TS :
    - `chartTypes.ts` : `insert_row` gagne `id?: string`, `insert_column` gagne `id?: string`, avec en commentaire « ne pas fournir, le cœur le tire » ;
    - `tableDeck.ts` : `TableOption.dataset` devient `{ source: Cell[][]; ids?: string[]; columns?: string[] }`.
  - `ARCHITECTURE.md` du crate : une ligne sur les identifiants (`ensure_ids`, alignement garanti par le cœur).
  - `chart.ftl` (20 locales), traduits. en :

    ```text
    chart-data-ids-length = dataset.ids needs one id per row and dataset.columns one per column
    chart-data-ids-duplicate = a row or column id is repeated
    chart-data-ids-invalid = a row or column id has 1 to 64 letters, digits, - or _
    chart-data-id-taken = this row or column id is already used
    ```

    fr :

    ```text
    chart-data-ids-length = dataset.ids demande un id par ligne et dataset.columns un par colonne
    chart-data-ids-duplicate = un id de ligne ou de colonne est répété
    chart-data-ids-invalid = un id de ligne ou de colonne a de 1 à 64 lettres, chiffres, - ou _
    chart-data-id-taken = cet id de ligne ou de colonne est déjà pris
    ```

- [ ] **Step 4 : lancer les tests.** Tout le workspace, plus l'UI. Résultat attendu : PASS. Note : `chart_data` et `table_data`, dans `data_ops.rs` et `table.rs`, n'utilisent plus d'inverse. Retirer les variables devenues inutiles.

- [ ] **Step 5 : commit.**

  ```text
  feat(chart) : identifiants stables des lignes et des colonnes, collage qui les garde par position (t-589)
  ```

---

### Task 3 : Le cœur garantit les identifiants à chaque écriture d'hôte

**Files :**
- Modify :
  - `crates/oku-slides-core/src/ops/host.rs` (`host_option`, `check_host_params`, `check_hosts`) ;
  - `crates/oku-slides-core/src/ops/mod.rs` (arms `add_element`, `ReplaceElement` et `AddSlide`, qui gardent les valeurs normalisées) ;
  - `crates/oku-slides-core/tests/tables.rs` et `charts.rs`.

**Interfaces :**
- `host_option(host, subject, v) -> Result<Value, OpError>` : lit, valide, appelle `ensure_ids` sur le dataset s'il y en a un, puis sérialise. C'est la valeur rendue qui est stockée.
- `check_hosts` et `check_host_params` : ils **réécrivent** dans l'élément la valeur normalisée de chaque paramètre d'hôte. Leur signature passe de `&Element` à `&mut Element`, ou ils rendent la liste des réécritures. Chaque appelant est mis à jour.
- `set_data` et `set_option` écrivent la valeur rendue par `chart_data`, `table_data`, `apply_option_patch` ou `patch_table`. Le dataset y est déjà aligné, car `apply` appelle `ensure_ids`. Pour `set_option`, appeler `ensure_ids` avant d'écrire.

- [ ] **Step 1 : écrire les tests qui échouent** (`core/tests/tables.rs`) :
  - `add_element_table_without_ids_stores_aligned_ids_and_columns` ;
  - `replace_element_chart_without_ids_stores_ids` ;
  - `set_param_table_without_ids_stores_ids` ;
  - `add_slide_with_table_without_ids_stores_ids` ;
  - `apply_all_keeps_existing_ids` : un tableau qui a déjà des identifiants, modifié par `set_cell`, garde exactement les mêmes.

- [ ] **Step 2 : lancer et constater l'échec.** `cargo nextest run -p oku-slides-core --test tables`. Résultat attendu : FAIL.

- [ ] **Step 3 : implémenter.** Faire réécrire les valeurs normalisées par `host_option` et ses appelants. Dans les arms d'ajout et de remplacement, la normalisation précède `check_params`, et l'élément inséré est l'élément normalisé.

- [ ] **Step 4 : lancer les tests.** Tout le workspace. Résultat attendu : PASS. Les tests de session (`tests/tables.rs`, `errors.rs`) qui comparent du JSON peuvent maintenant voir `ids` et `columns`. Ajuster leurs assertions pour comparer `source`, sans affaiblir ce qu'elles prouvent.

- [ ] **Step 5 : commit.**

  ```text
  feat(core) : chaque écriture d'un paramètre d'hôte pose les identifiants des lignes et des colonnes (t-589)
  ```

---

### Task 4 : Le replica fusionne les données d'hôte cellule par cellule

**Files :**
- Create :
  - `crates/oku-slides-core/src/replica/host.rs`, avec les fonctions d'écriture `impl Writer`, de lecture et de réparation ;
  - `crates/oku-slides-core/src/replica/host_tests.rs` ;
  - `crates/oku-slides-core/src/replica/ARCHITECTURE.md`.
- Modify :
  - `replica/schema.rs` (`Kind::Host`, `declared_kind`, `fits`, `column_kind`, constantes `COLUMNS`/`ROWS`/`CELLS`) ;
  - `replica/write.rs` (arms `Kind::Host`, `sync_list`, `list_at`, `map_at` et `set_value` passent en `pub(super)`) ;
  - `replica/read.rs` (marqueur sous `dataset` d'un paramètre d'hôte) ;
  - `replica/mod.rs`, qui déclare `mod host;` et, sous `#[cfg(test)] #[path = "host_tests.rs"] mod host_tests;`, le module de test placé à côté de `list_tests` ; à ajouter aussi à la liste des modules dans la doc ;
  - `replica/undo_tests.rs` ;
  - `crates/oku-slides-core/tests/convergence.rs` ;
  - `crates/oku-slides-core/ARCHITECTURE.md`, réduit à une ligne de renvoi vers `replica/ARCHITECTURE.md`.

**Interfaces :**
- `Kind::Host`, documenté ainsi : « `LoroMap` ; `dataset` en colonnes et lignes à identifiant ».
  - `declared_kind` renvoie `ParamType::Host { .. } => Kind::Host`.
  - `fits(Kind::Host, Value::Object(_))` vaut vrai.
  - `column_kind` rabat `Host` sur `Atom`, comme `List` et `Collection`.
- Écriture : `fn sync_host(&self, map: &LoroMap, obj: &Map<String, Value>) -> Res`, une méthode de `Writer` dans `host.rs` qui utilise `self.split`.
- Les formes reconnues sont celles où **toutes** ces conditions tiennent ; toute autre forme part en `Nested(Opaque)` et ne fusionne pas finement :
  - `dataset` est un objet dont les clés sont parmi `{source, ids, columns}` ;
  - `source` est une liste de listes de scalaires ;
  - `ids` et `columns` sont des listes de chaînes alignées.

- [ ] **Step 1 : écrire d'abord les tests de convergence, et les voir échouer sur le code actuel.**

  Dans `tests/convergence.rs`, réutiliser `with_ops` et `exchange` (qui prennent `common::list_registry()`, où figurent `table` et `chart`). Ajouter une aide `table_peers()` qui crée, sur deux pairs issus du même snapshot, le tableau `t` (3 lignes × 2 colonnes, identifiants explicites) par `add_element`. Les éditions passent par `Op::SetData`. Les cas :
  - `concurrent_edits_of_two_cells_in_one_row_are_both_kept` ;
  - `concurrent_edits_of_two_cells_in_one_column_are_both_kept` ;
  - `concurrent_row_inserts_are_both_kept` (5 lignes, identifiants uniques) ;
  - `concurrent_column_inserts_are_both_kept` (4 colonnes) ;
  - `concurrent_row_remove_and_other_row_edit_merge` ;
  - `concurrent_column_insert_and_cell_edit_merge` ;
  - `concurrent_row_insert_and_column_insert_leave_new_row_aligned` : la nouvelle ligne vaut `null` dans la nouvelle colonne, et ses autres valeurs sont à leur place ;
  - `same_row_removed_by_both_then_undone_by_both_has_no_duplicate_id`.

  Chaque test compare le `source` complet attendu, vérifie que `ids` et `columns` sont alignés et uniques, que `parse_table` réussit sur le résultat, et que la fusion donne le même résultat dans les deux sens.

  Lancer `cargo nextest run -p oku-slides-core --test convergence`. Résultat attendu : FAIL sur les cas « both kept », car l'`Atom` en dernière écriture gagnante perd une édition. Consigner cette sortie dans le rapport comme preuve du RED.

  Étendre le proptest (l. 1-136) :
  - en prélude, un `add_element` du tableau `t` avec identifiants ;
  - une stratégie d'éditions `set_cell`, `insert_row`, `remove_row`, `insert_column` et `remove_column`, dont les index sont tirés dans les bornes courantes du pair ;
  - après fusion, les assertions de convergence, `parse_table` qui réussit, et l'alignement de `ids` et `columns`.

- [ ] **Step 2 : écrire les tests unitaires du replica** (`host_tests.rs`, avec `use super::*`, sur le modèle de `list_tests.rs`) :
  - `host_table_round_trips_through_the_replica` et `host_chart_round_trips_through_the_replica`. Le JSON relu est identique, `ids` et `columns` compris. Inclure des cellules `1.0`, `true` et `null` ;
  - `host_dataset_is_stored_as_columns_and_rows` : vérifier la structure Loro ;
  - `set_cell_writes_one_entry_in_the_same_row_container` : comparer `ContainerID` de la ligne avant et après, et vérifier que `len_ops()` augmente de 1 ;
  - `rewriting_an_unchanged_deck_writes_nothing` : `len_ops()` est inchangé ;
  - `row_insert_remove_reorder_keep_surviving_row_containers` ;
  - `column_insert_remove_reorder_keep_cells_by_column_id` ;
  - `dataset_of_unexpected_shape_round_trips_unchanged`, avec trois cas : une ligne qui n'est pas une liste, une clé en trop, des `ids` non alignés ;
  - lecture d'états forgés, construits à la main dans `r.doc` :
    - `forged_duplicate_row_ids_are_repaired_on_read` ;
    - `forged_cell_container_reads_as_null` ;
    - `forged_rows_beyond_cap_are_truncated_and_reported` ;
    - `forged_marker_outside_dataset_is_not_read_as_dataset` ;
    - `forged_deep_cell_respects_max_depth` ;
  - `ragged_merged_rows_read_with_nulls`.

  Dans `undo_tests.rs` :
  - `undo_redo_of_set_cell_on_table_restores_exactly` ;
  - `undo_redo_of_remove_row_restores_id_and_cells` ;
  - `undo_redo_of_insert_column_restores_exactly` ;
  - `undo_redo_of_replace_restores_exactly`.

  Ces tests s'appuient sur le rappel t-263 : chaque `mov` part dans son propre commit (`split`).

- [ ] **Step 3 : implémenter.**
  - **Écriture** (`host.rs`, `impl Writer`) :
    - `sync_host` : les clés obsolètes sont retirées. Pour chaque clé :
      - `dataset` de forme reconnue → `sync_dataset` ;
      - autre objet → `self.sync_map(&map_at(..), o, Ctx::Opaque)` ;
      - toute autre valeur → `set_value`.
    - `sync_dataset(&map_at(map, "dataset")?, ds)` :
      - la carte doit avoir exactement les clés `{columns, rows}`, sinon elle est remplacée, comme le fait `items_at` ;
      - `columns` : rapprochement d'une liste de chaînes par identifiant, avec suppressions, puis insertions, puis `mov` (un `(self.split)()` avant chaque `mov`). Indexer les positions avec une `HashMap`, pas avec `position` ;
      - `rows` : `self.sync_list(&rows, &row_objects, &|m, o| self.sync_row(m, o))`, où `row_objects` vaut `[{"id": ids[i], "cells": {col_id: valeur, …}}]` ;
      - `sync_row` écrit toujours `id` (`set_value`), puis rapproche la carte `cells` par clé : `set_value` pour chaque colonne présente, `delete` pour les clés absentes ;
      - une valeur égale n'écrit rien (`set_value` le garantit déjà).
    - Dans `write.rs` :
      - `sync_fields` : arm `Kind::Host => match value { Value::Object(o) => self.sync_host(&map_at(map, key)?, o)?, v => set_value(map, key, json_to_loro(v))? }` ;
      - boucle des champs déclarés absents : `Kind::Host => {}`, avec le commentaire « pas créé d'avance : la forme du dataset est une donnée » ;
      - `sync_map` : `Ctx::kind` ne rend jamais `Host`. Dans les deux `match`, `Kind::Host` rejoint le bras d'erreur de `Kind::List`, en renommant `ListOutsideParams` en `ContainerOutsideParams` dans `crate::error` (variante et message).
  - **Lecture** (`read.rs` + `host.rs`) :
    - dans `read_params`, pour une carte (paramètre d'hôte lu en `Opaque`), passer le contexte « hôte » à `read_map_at`, par une variante `Ctx::Host` qui se lit comme `Opaque` ;
    - dans `read_map_at`, pour la clé `dataset` en contexte hôte, une carte `{columns: MovableList, rows: MovableList}` est reconstruite par `host::read_dataset` ;
    - `read_dataset` :
      - lit `columns` (chaînes, bornées à 1 000) et les lignes (bornées à 10 000) ;
      - chaque ligne doit être une carte avec un `id` chaîne et un `cells` carte, sinon elle est écartée et signalée par `issue-unreadable-item` ;
      - chaque cellule de valeur passe par `loro_to_json_at(&v, depth + 3)` ; une cellule conteneur ou absente vaut `null` ;
      - ensuite, réparation déterministe des identifiants en double ou invalides (`r{i}`/`c{j}`, plus un suffixe si l'identifiant est pris) ;
      - le résultat est rendu en `{"columns": …, "ids": …, "source": …}`, aux clés triées ;
    - la profondeur est comptée comme ailleurs (`depth.deeper()` à chaque niveau).
  - **Docs** :
    - `replica/ARCHITECTURE.md` (moins de 50 lignes) : modules (schema, read, write, host, rich, undo, authors, history) ; catégories (`Kind`) ; règle « diff du JSON du deck » ; réparation à la relecture ; parades t-188 et t-263 ;
    - `core/ARCHITECTURE.md` : le paragraphe du replica devient une ligne de renvoi, et le fichier repasse sous 50 lignes.

- [ ] **Step 4 : lancer les tests.** Tout le workspace. Résultat attendu : PASS, y compris tous les tests existants du replica et la convergence, qui passe maintenant.

- [ ] **Step 5 : commit.**

  ```text
  feat(core) : le replica fusionne les données des graphiques et tableaux cellule par cellule (t-589)
  ```
