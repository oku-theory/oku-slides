# Finitions de la réparation des ids et des séries par colonne (t-677)

**Goal:** solder les finitions relevées par la revue finale de t-652/t-653/t-655.

**Spec:** la tâche Oku t-677 (`oku cli task info --project slider --id t-677`) et le design approuvé en séance (2026-10-09). Plan revu par l'équipe (14 relecteurs) puis vérification ciblée ; constats intégrés.

## Global Constraints

- Pas de rétrocompatibilité (MVP) ; TDD : test rouge d'abord, vu échouer avant le correctif.
- Chaque bug a un test de régression dans `tests/regressions/` de la crate (fichier `issue_0677_<sujet>.rs`, déclaré dans `tests/regressions/main.rs`).
- Toute clé i18n modifiée l'est dans les 20 langues (`crates/oku-slides-i18n/src/locales/*/`), traduite (pas recopiée de l'anglais).
- Commits conventionnels en français, avec portée et référence de sous-tâche, par ex. `fix(sources) : … (t-688)`. Jamais de ligne `Co-Authored-By`.
- `git add` dans sa propre commande, puis `git diff --cached --name-only`, puis `git commit -F <fichier>` seul (le hook bloque les commandes composées).
- Cargo : préfixer par `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target`. Tests Rust : `OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run -p <crate>` ; clippy `-D warnings`, `cargo fmt --check`.
- UI : `cd ui && bun run test && bunx --bun tsc -b` (le node de Homebrew est cassé).
- `crates/oku-slides-core/ARCHITECTURE.md` fait déjà 50 lignes (le maximum) : réécrire des lignes, n'en ajouter aucune.

## Task 1: A : bundle répare les ids avant de normaliser ; ARCHITECTURE du cœur (t-688)

- `crates/oku-slides-core/src/sources.rs`, `SourceBundle::new` : après le garde `too_deep` (inchangé, il borne la récursion), l'ordre devient `crate::canonicalize(&mut deck)` → `crate::ops::normalize_hosts(&registry, &mut deck)` → `crate::canonicalize(&mut deck)`. Le premier passage écarte les copies exactes *après* canonisation des textes et nombres (ce que fait `canonicalize` avant d'appeler `unique_ids`), donc avant que `normalize_hosts` ne pose des ids de lignes/colonnes aléatoires différents sur chaque copie ; le second remet sous forme canonique ce que la normalisation a ajouté. Mettre à jour la doc de `new` en conséquence.
- `restore` n'est pas modifié : son deck vient de `preview.deck_at`, relecture du replica qui appelle déjà `unique_ids` (`replica/read.rs`). Le dire en une ligne dans le commentaire existant de `restore`.
- Régression `crates/oku-slides-core/tests/regressions/issue_0677_bundle_dedups_before_normalizing.rs` (déclarée dans `main.rs`), à partir d'un deck **JSON** (pas de canonisation préalable) :
  1. une slide dont un slot porte deux copies exactes (même id d'élément) d'un **graphique** dont le jeu de données n'a ni ids de lignes ni de colonnes → après `SourceBundle::new`, un seul élément, aucun id en `-2` ;
  2. idem pour un **tableau** ;
  3. deux copies d'un élément `text` (sans hôte) qui ne diffèrent que par un paramètre texte avec blanc de bord (que `canonicalize` rogne) → un seul élément ;
  4. dans chaque cas, la vue (`bundle.deck()` ou équivalent) égale la relecture du replica.
  Les cas 1 et 2 échouent avant le correctif ; le cas 3 passe avant et après (il garde de la régression qu'un simple `unique_ids` aurait introduite).
- `crates/oku-slides-core/ARCHITECTURE.md` : réécrire en place la ligne `SourceBundle` (« `new` refuse un deck trop profond, met sous forme canonique avant et après la normalisation des hôtes… ») et la ligne `canonicalize` (« forme canonique d'un `Deck` (clés triées, slots vides supprimés) ; répare les ids en double — copie exacte écartée, autre occurrence renommée `{id}-n` — comme `apply_all` et la relecture du replica (t-652) »). Toujours 50 lignes au plus.

## Task 2: B : chart-columns aux ids distincts (t-689)

- `crates/oku-slides-chart/src/data_ops.rs` : extraire la fermeture `unique` de `validate_ids` en fonction `pub(crate) fn all_unique(list: &[String]) -> bool` (HashSet, linéaire), utilisée par `validate_ids` et par `validate_table`.
- `crates/oku-slides-chart/src/table.rs`, `validate_table` : `chart_columns` est refusé (même message `chart-table-chart-columns`) s'il dépasse `MAX_TABLE_COLUMNS` (vérifié en premier, avant tout parcours), s'il contient un id invalide (existant) ou un id en double (`all_unique`).
- Texte de `chart-table-chart-columns` (sans argument) dans les 20 langues : en `chart-columns must list valid, distinct column identifiers (1 to 64 letters, digits, - or _)` ; fr `chart-columns doit lister des identifiants de colonnes valides et distincts (1 à 64 lettres, chiffres, - ou _)` ; les 18 autres traduites.
- Mettre à jour la doc du champ `chart_columns` (`table.rs:46`) : ids distincts. Tests dans `table_tests.rs`, sur le modèle des tests existants, toujours avec `"chart": {}` (sans graphique, `tidy` vide la liste) : `["a","a"]` refusé avec ce message ; une liste de `MAX_TABLE_COLUMNS + 1` ids distincts refusée ; témoin positif `["a","b"]` sur un jeu à deux colonnes accepté. Test de `all_unique` dans `data_ops_tests.rs`.
- Régression : la crate chart n'a pas de `tests/regressions/` ; créer `crates/oku-slides-chart/tests/regressions/main.rs` + `issue_0677_chart_columns_duplicates.rs` qui passe par `parse_table` (pub, réexporté), qui atteint `validate_table`.

## Task 3: C : la grille lit NumberLangContext ; plan t-654 (t-690)

- `ui/src/a2ui/numberLang.ts` : y déplacer et exporter `useNumberLang()` (aujourd'hui privé dans `ListTable.tsx:92` : `useContext(NumberLangContext) || locale()`). `ListTable` l'importe ; `ChartDataGrid` l'utilise à la place de `numberLang || locale()`. Mettre à jour le commentaire de tête (la grille et la table de liste lisent ce contexte ; App le fournit).
- Retirer la prop `numberLang` de `ChartDataGridProps`, `ChartPanel`, `TablePanel`, `TargetPanel` (Props et passage) et de l'appel `numberLang={snap.lang}` dans `App.tsx`. Critère de fin : `grep -rn "numberLang" ui/src` ne renvoie plus que `numberLang.ts`, ses importeurs et des usages de `useNumberLang`/`NumberLangContext`.
- Tests à adapter : `ui/src/chart/ChartDataGrid.render.test.tsx`, `ui/src/table/TablePanel.render.test.tsx` (et tout test qui passait `numberLang`) — la langue est fournie par `NumberLangContext.Provider`. Ajouter : sans fournisseur, la grille formate dans la langue de l'UI ; avec `TablePanel` sous un fournisseur `fr`, la grille affiche le format français (preuve que la chaîne sans prop marche).
- Langue invalide : vérifier ce que fait la grille avec une étiquette invalide (par ex. `"zz-!!"`) ; si `Intl.NumberFormat` lève, `useNumberLang` retombe sur `locale()` (test). Si rien ne lève, l'écrire dans le rapport.
- Plan t-654 (`docs/superpowers/plans/2026-10-09-agent-detachable.md`), **Task 9**, description de `AgentWindowFor` (vers « **`AgentWindowFor({ tab, transport, host })`** ») : ajouter que le rendu de la vue est enveloppé dans `<NumberLangContext.Provider value={snap?.lang ?? null}>` (`snap` est l'instantané que la fenêtre relit déjà par `host.api.snapshot()`), sans quoi les tableaux de liste du fil prendraient la langue de l'UI ; et, dans les tests de la Task 9, un cas : avec un instantané `lang: "fr"`, le contexte vu sous `AgentWindowFor` vaut `"fr"`. Modification ciblée de ce fichier volumineux : ne rien réécrire d'autre.

## Task 4: D : proptest, déplacement entre slots (t-691)

Contexte : le layout `content` déclare deux slots, `blocks` (flux) et `elements` (libre, éléments à `frame`) ; `canonicalize` supprime les slots vides ; dans le deck de base `roadmap`, aucune slide n'a deux slots peuplés. Le replica n'émet `mov` que dans une même liste : un déplacement entre slots est un retrait + une insertion.

- `crates/oku-slides-core/tests/convergence.rs` :
  1. Le deck de départ du proptest (là où `s-constat` reçoit déjà ses éléments de test) gagne un élément simple dans le slot `elements` de `s-constat`, avec un `frame` valide, pour que les deux slots existent.
  2. Nouvelle édition `Edit::MoveAcrossSlots(usize, usize)` dans `edit()` : sur une slide dont le layout déclare `blocks` et `elements` et dont au moins l'un est non vide (choisie par `i % n`), prendre l'élément `a % len` d'un slot non vide (le premier non vide dans l'ordre `blocks`, `elements`, alternance selon la parité de `i`) et l'ajouter à la fin de l'autre slot, en le créant s'il est absent ; vers `elements`, poser un `frame` par défaut si l'élément n'en a pas ; vers `blocks`, retirer `frame` et `cell`. Le slot source vidé est supprimé des params. Le sens (source `blocks` ou `elements`) vient de la parité de `a`, pas de `i` (qui choisit la slide). Indices toujours réduits par un modulo non nul.
  3. Test déterministe nommé `concurrent_move_across_slots_and_edit_converge` (avec `exchange` ou l'aide existante à deux pairs) : le pair A déplace un élément de `blocks` vers `elements` (retrait puis ajout cadré dans `elements`) pendant que le pair B le met en gras ; après échange, les deux pairs convergent et l'élément n'existe qu'une fois (pas d'id en double).
- Le proptest de convergence passe (48 cas). `with_cases(48)` écrase `PROPTEST_CASES` : passer temporairement à 1000 cas en local, lancer une fois, noter le résultat dans le rapport, revenir à 48 avant de committer.
- Si le proptest ou le test déterministe révèle une vraie divergence : s'arrêter, réduire le cas, rapporter `BLOCKED` avec le cas minimal — pas de restriction de la stratégie pour faire passer le test.
