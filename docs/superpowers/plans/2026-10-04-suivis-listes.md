# Suivis du paramètre liste (t-527 à t-535) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking. Tasks run strictly in order, one at a time (they share files).

**Tâches oku :** parent t-536 (« Suivis du paramètre liste », plan résumé dans son corps) ; chaque Task correspond à une tâche existante liée à t-536 : Task 1 = t-532, Task 2 = t-527, Task 3 = t-531, Task 4 = t-528, Task 5 = t-529, Task 6 = t-530, Task 7 = t-533, Task 8 = t-534, Task 9 = t-535. Corps d'une tâche : `oku cli task info --project slider --id t-NNN`.

**Goal:** solder les suivis de t-494 (paramètre `list`) : retrait de la frise de base, validation, nettoyage, bornes, tests, poignées, finitions, relecture des traductions.

**Architecture:** corrections ciblées dans le code livré par t-494 (cœur `ops/list.rs`, `validate.rs`, replica ; session `items.rs`, `widget_examples.rs`, `widget_params.rs`, `widget_preview.rs` ; UI `ItemLayer.tsx`, `ListTable.tsx`, `Stage.tsx`, `stageItems.ts`, `WidgetTab.tsx` ; Typst `widgets.typ`).

**Tech Stack:** Rust (snafu, serde, Loro, proptest), Typst, TypeScript/React (bun test + happy-dom), Playwright, Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-10-04-parametre-liste-design.md` (amendements A1–A11) et le corps de chaque tâche oku.

## Global Constraints

- Chaque commit se termine par `(t-NNN)` de sa tâche. Le contrôleur marque les tâches `done`.
- Commits : `git add` séparé, `git diff --cached --name-only` juste avant `git commit` ; pas de ligne `Co-Authored-By` ; format `type(scope) : description (t-NNN)`.
- TDD : test rouge (garder la preuve), puis vert. Tests côte à côte, speculoos/rstest, noms `<unité>_<cas>_<résultat>`. Bug corrigé → test de régression dans `tests/regressions/issue_<NNNN>_<desc>.rs` (côté Rust) nommé par la tâche (ex. `issue_0527_…`), ou test de rendu nommé côté UI.
- Une tâche sans nouveau comportement (refactor, docs, traductions) le dit dans son rapport : pas de RED attendu, tests existants verts.
- Pas de `.unwrap()`/`.expect()` hors tests ; erreurs Snafu ; tout texte visible par Fluent dans les 20 langues (`every_locale_has_exactly_the_reference_keys`) ; toute clé `ui-…` utilisée (`ui/src/i18n-keys.test.ts`) ; une tâche qui ajoute une clé l'ajoute aux 20 langues.
- Fichiers ≤ 500 lignes visés, 1000 max : `Stage.tsx` est à 500 (logique nouvelle dans `stageItems.ts`), `widget_preview.rs` à 600 (code nouveau dans un module à part), `ListTable.tsx` à ~400 (extraire un hook si besoin).
- App ou e2e : toujours `export OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d)` avant (`just e2e` ou serveur lancé à la main) ; arrêter tout serveur lancé.
- Décisions de l'utilisateur (2026-10-04) : liste vidée d'un cas d'exemple = liste vide `[]` gardée, par tous les chemins (t-528) ; bornes de taille des listes dans le **cœur**, pour tous les chemins (t-528) ; liste de slide active quand aucun élément n'est sélectionné (t-530) ; la frise est un widget de pack, la frise de base est retirée — pas de migration des decks qui auraient un élément `frise` de base : ils reçoivent l'issue de widget inconnu (règle MVP) (t-532).
- Vérifications avant commit : `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, nextest des crates touchés ; `cd ui && bun run test && bun run typecheck` si l'UI change ; `just e2e` (env isolé) si la tâche touche un script e2e ou un parcours e2e.

---

### Task 1: Retirer la frise de la bibliothèque de base (t-532)

**Files:** `typst/oku-slides/src/widgets.typ` (`_frise`, `registry`) ; `fixtures/widgets/base-registry.json` ; `crates/oku-slides-render/tests/snapshots/widgets__base_registry.snap` ; `crates/oku-slides-render/tests/{lists.rs,widgets.rs}` (`widgets.rs:101` liste des noms de base) ; session : `src/lib_tests_support.rs` (`add_frise`), `src/fields_tests.rs`, `src/items_tests.rs`, et tout test de session qui ajoute un élément `frise` (`rg -n '"frise"' crates/oku-slides-session`) ; `ui/e2e/lists.cjs` (et `ui/package.json` si l'ordre des scripts change) ; `typst/oku-slides/README.md` (phrase « Le widget de base `frise` … en est l'exemple complet ») ; `crates/oku-slides-i18n/src/locales/fr/agent.ftl` seulement si sa mention désigne le widget de base.

- [ ] Relevé de départ (dans le rapport) : chaque test qui utilise la frise de base et ce qu'il vérifie (hauteur d'une frise vide, champ vide mesuré, boîtes d'item, ListView len 0/4, focus après insertion, ids d0…d3 amorcés).
- [ ] Retirer `_frise` et son entrée du registre ; régénérer la fixture et le snapshot ; vérifier dans le diff que **seule** l'entrée `frise` disparaît, et retirer `frise` de la liste de `widgets.rs:101`.
- [ ] Un **widget de pack de test** à liste reprend la frise (liste `etapes`, colonnes `date` plain et `texte` plain défaut « Nouvelle étape », `item-label: "étape"`, `max: 8`, quatre étapes par défaut, rendu en colonnes avec `list-item`, champs vides `field(…, empty: true)`, frise vide à hauteur d'une rangée). Sa source Typst vit **une fois** dans `fixtures/widgets/frise-test.typ` ; helpers : `crates/oku-slides-render/tests/common/mod.rs` (pack de rendu) et `crates/oku-slides-session/src/test_support.rs` (pack de session, à côté de `add_version`). Le pack `frise2` de `widget_params_tests.rs` passe sur ce même helper. Les tâches suivantes réutilisent ces helpers.
- [ ] Tous les comportements du relevé restent couverts, sur le widget de pack.
- [ ] Tests UI qui utilisent `"frise"` comme simple chaîne de vue (`Stage.render.test.tsx`, `ItemLayer.render.test.tsx`, `deck.test.ts`, `useHostSettings.render.test.tsx`) : laissés tels quels (pas de registre) — le dire dans le rapport.
- [ ] `ui/e2e/lists.cjs` : `set_widget` exige le mode modèles, qui retire les slides du deck. Réorganiser : entrer en mode modèles, créer le widget de pack de test (`set_widget`, comme `lists-dock.cjs`/`widgets.cjs`), en sortir, puis `add_element … defaults: true` de ce widget sur `s-constat` ; **assert** que les 4 items sont présents avant les assertions existantes ; mettre à jour le commentaire d'en-tête et, si besoin, l'ordre dans `ui/package.json`.
- [ ] Vérification finale : `rg -n '\bfrise\b' crates ui fixtures typst docs` — ne restent que le widget de pack de test, les chaînes de vue des tests UI, les exemples de pack de la doc, et les docs historiques `docs/superpowers/*` (laissées telles quelles) ; lister les occurrences restantes dans le rapport.
- [ ] `just test` et `just e2e` (env isolé) verts. Commit : `refactor(typst) : frise retirée de la bibliothèque de base, tests sur un widget de pack (t-532)`.

### Task 2: Valider les colonnes rich des listes (t-527)

**Files:** `crates/oku-slides-core/src/validate.rs` (fn `rich`, ~l.221), `validate_tests.rs`, `crates/oku-slides-core/tests/regressions/issue_0527_rich_list_links.rs` (nouveau), `crates/oku-slides-render/tests/lists.rs`.

- [ ] `validate::rich` parcourt aussi chaque item de chaque liste (`Params::lists`) et ses colonnes rich ; même liste blanche (`is_safe_link`), même contrôle des pastilles (`highlight == 0`), issue portée par le même objet (élément ou slide propriétaire de la liste).
- [ ] Tests (assertions sur la clé **et** l'objet signalé) : `javascript:`, `data:`, `vbscript:`, `" javascript:"` avec espace, schéma en majuscules → `issue-unsupported-link` ; lien sûr → aucune ; pastille `highlight == 0` dans une cellule → issue ; lien dans le 2e item et dans une 2e colonne rich (prouve le parcours) ; liste portée par une slide et par un élément ; liste vide ou cellule rich vide → aucune issue.
- [ ] Test de rendu : un lien non sûr dans une cellule rich d'une liste n'est pas rendu comme lien (même garde que `content.typ`).
- [ ] Commit : `fix(core) : liens et pastilles des colonnes rich des listes validés (t-527)`.

### Task 3: Surface publique et doublons des listes (t-531)

**Files:** `crates/oku-slides-core/src/{params.rs,params_tests.rs,lib.rs,widget.rs,fields.rs,test_registry.rs,validate_tests.rs}`, `crates/oku-slides-core/tests/{common/mod.rs,lists.rs}`, `crates/oku-slides-core/src/replica/{write.rs,schema.rs}`, `crates/oku-slides-session/src/items.rs`.

- [ ] `Params::list_mut` : retiré (seul usage : `params_tests.rs:127`, test à retirer ou réécrire). `Params::lists` : **gardé** (Task 2 lui donne un usage réel).
- [ ] `RESERVED_COLUMNS`, `is_column_type` : `pub(crate)`, retirés des réexports de `lib.rs:55-56` (aucun usage hors crate ; `cargo build --workspace --all-targets` le confirme, tests d'intégration compris).
- [ ] Prédicat unique « colonne texte » : `pub fn is_text(ty: &ParamType) -> bool` (plain ou rich) dans `crates/oku-slides-core/src/widget.rs`, réexporté par `lib.rs` ; utilisé par `core/fields.rs` (~l.44) et `session/items.rs` (`Shape::of`, ~l.64) à la place de leurs copies. `widget_params::column()` (ParamType → ColumnKind de l'A2UI) reste : autre responsabilité.
- [ ] Registre de test « liste » : une seule construction côté tests unitaires (`test_registry::with_list`, qui gagne le widget `bornes` de `validate_tests.rs`) et une seule côté tests d'intégration (`tests/common/mod.rs`), utilisée par `tests/lists.rs` ; plus de copie locale.
- [ ] `replica/write.rs` `sync_map` (~l.143, 155) : les bras `Kind::List` inatteignables (hors `Params`, `Ctx::kind` ne rend jamais `List`) ne sont **pas** fusionnés dans `Atom` : bras explicite qui renvoie une erreur Snafu de réplica (« liste hors des paramètres »), avec un test qui l'atteint si c'est faisable, sinon un commentaire qui renvoie à `Ctx::kind`.
- [ ] Refactor : pas de nouveau comportement, tests existants verts (round-trip replica des listes compris). Commit : `refactor(core) : surface publique des listes réduite, doublons retirés (t-531)`.

### Task 4: Bornes de taille des listes, cœur et cas d'exemple (t-528)

**Files:** `crates/oku-slides-core/src/ops/{list.rs,list_tests.rs,value.rs}`, `crates/oku-slides-core/tests/lists.rs`, `crates/oku-slides-core/src/error.rs` si une erreur nouvelle est nécessaire (+ 20 `core.ftl`) ; `crates/oku-slides-session/src/{widget_examples.rs,widget_params.rs}` et leurs tests ; `crates/oku-slides-mcp/src/widget_tools.rs` (doc de `set_widget_trial`) ; `crates/oku-slides-i18n/src/locales/*/agent.ftl` si les consignes décrivent l'effacement par `[]`.

- [ ] **Cœur** : bornes de liste dans `ops/list.rs`, appliquées par `item_values` (insert, `set_param` d'une liste, défauts) et `ListOp::SetField` : cellule plain ≤ `PLAIN_MAX` (4096), cellule rich ≤ `RICH_PARAGRAPHS_MAX` (64) paragraphes et ≤ `RICH_BYTES_MAX` (16384) octets, id d'item ≤ 64 caractères, nombre d'items ≤ `max` du schéma, ou ≤ `LIST_ITEMS_MAX = 256` quand `max` est absent. Les constantes déménagent de `session/widget_params.rs` vers le cœur (une seule définition, la session les importe) ; erreur : `InvalidValue` avec une raison claire (réutiliser), sauf besoin réel d'un nouveau type.
- [ ] **Cas d'exemple** — les trois chemins qui écrivent une liste : `set_example_values` (UI, `set_example_value`), `set_example_values_by_agent` (outil MCP `set_widget_trial`), `edit_example_list`. Tous passent par les bornes du cœur (une seule mise en œuvre).
- [ ] Liste vidée (décision utilisateur) : `[]` est **gardée** comme valeur du cas par les trois chemins : ajouter un bras `List` à `emptied()` (`widget_examples.rs` ~l.277) pour que `[]` ne soit pas lu comme « effacer », **avant** le filtre `same_as(neutral…)` ; `null` (et l'aperçu neutre) efface toujours. Mettre à jour la doc de `set_widget_trial` (`widget_tools.rs:45-46`), qui dit aujourd'hui que `[]` efface, et les consignes agent si elles le disent. Avec `min > 0`, `[]` est refusé par les trois chemins (même règle que `Remove` sous `min`).
- [ ] `edit_example_list` : une liste stockée illisible renvoie une erreur (fin du `unwrap_or_default()`, ~l.226), au lieu d'être remplacée par `[]`.
- [ ] Tests : cœur — limite exacte acceptée et +1 refusée pour une cellule plain, une cellule rich en paragraphes et en octets, un id, le nombre d'items (avec et sans `max`) ; un refus laisse le deck inchangé. Session — les trois chemins refusent une cellule trop longue et laissent la valeur stockée inchangée ; `[]` gardé, `null` efface, `[]` après effacement gardé ; vider par `edit_example_list` et par `set_example_value([])` donne la **même valeur stockée** (min 0) ; `min > 0` + `[]` refusé ; liste stockée illisible → erreur.
- [ ] Commits (au moins deux) : `fix(core) : listes bornées en taille dans le cœur (t-528)` puis `fix(session) : listes des cas d'exemple bornées, liste vidée gardée vide (t-528)`.

### Task 5: Proptest canonique avec des listes (t-529)

**Files:** `crates/oku-slides-core/src/canonical_tests.rs` (stratégie de `canonical_deck_reads_back_unchanged`), `crates/oku-slides-core/src/test_registry.rs` si un widget de liste manque.

- [ ] La stratégie génère des paramètres liste déclarés par le registre de la proptest (`test_registry::with_list`, plus une liste aux colonnes choice/number/flag/asset si besoin) : items à ids uniques, colonnes plain/rich/flag/number, valeurs vides comprises, listes vides, et **listes absentes** (paramètre omis).
- [ ] Garde-fou contre une stratégie dégénérée : un test (graine fixe ou comptage) prouve que des listes non vides sont réellement générées.
- [ ] Graine de régression proptest gardée (fichier `proptest-regressions` du crate, comme les autres proptests s'il y en a).
- [ ] Écart trouvé : s'il se corrige en moins d'une heure et dans le périmètre des listes, le corriger (TDD) avec un test `tests/regressions/issue_0529_<desc>.rs` ; sinon, s'arrêter et le signaler (DONE_WITH_CONCERNS) pour une tâche à part. Commit : `test(core) : la relecture canonique couvre les listes (t-529)`.

### Task 6: Poignées d'une liste de slide sans sélection (t-530)

**Files:** `ui/src/stageItems.ts` (+ test), `ui/src/Stage.tsx` (~l.126, ~l.438 : calcul de `active`), `ui/src/ItemLayer.tsx`, `ui/src/ItemLayer.render.test.tsx`, `ui/src/Stage.render.test.tsx`.

- [ ] Règle (décision utilisateur) dans une fonction pure de `stageItems.ts`, p. ex. `activeOwner(view, selected, editingOwner): string | null` : propriétaire du champ en édition s'il y en a un ; sinon l'élément sélectionné ; sinon **l'id de la slide** (`view.slide.id`) — la liste de slide est alors active. `Stage.tsx` l'appelle aux deux endroits (items et liste vide) ; la surbrillance des champs (`hot`, ~l.407) reste inchangée.
- [ ] Plusieurs listes de slide : toutes montrées. Mode lecture seule : aucune poignée (comme aujourd'hui).
- [ ] Tests : `activeOwner` (édition > sélection > slide) ; rendu — slide à liste sans sélection → poignées de la liste de slide visibles ; élément sélectionné → poignées de la slide cachées, celles de l'élément montrées ; édition d'un champ de la liste de slide alors qu'un élément était sélectionné → poignées de la slide ; liste d'élément sans sélection → cachée ; lecture seule → rien ; « + » de la liste de slide envoie `edit_list` avec `target: { slide }` (sans `element`).
- [ ] Commit : `fix(ui) : poignées d'une liste de slide sans sélection (t-530)`.

### Task 7: Poignées de liste sur l'aperçu du widget (t-533)

**Files:** session : nouveau module `crates/oku-slides-session/src/widget_preview_items.rs` (items et lists de la vue d'aperçu), `widget_preview.rs` (~l.413-415 : remplacer `items: Vec::new()` et son commentaire), `items.rs` (fonctions `item_views` / `list_views` réutilisées, pas copiées) ; UI : `ui/src/api.ts` (`WidgetPreview` gagne `items`, `lists`), `ui/src/ItemLayer.tsx`, `ui/src/WidgetTab.tsx`, tests de rendu ; `ui/e2e/lists-dock.cjs`.

- [ ] Vérifier d'abord (rapport) que le rendu de l'aperçu produit des `ItemBox` (`layout.items_on`) ; la vue d'aperçu calcule `items` et `lists` avec `items::item_views` / `items::list_views` sur une slide synthétique qui porte les valeurs du cas affiché.
- [ ] `ItemLayer` reçoit ses ops par une prop `onListOp(owner, list, change: ListOp, label)` au lieu de construire des ops de deck ; `Stage` l'adapte en `edit_list` (comportement inchangé sur la scène) ; `WidgetTab` l'adapte en `api.editExampleList(widget, example, list, change)` sur le **cas affiché**. Les ids d'insertion restent générés par le client (`newId("i")`).
- [ ] Après insertion sur l'aperçu, le premier champ texte du nouvel item s'ouvre (même mécanisme d'attente de clé que la scène, `editKeyWhenShown`), y compris quand le champ n'apparaît qu'au rendu suivant. Une liste vide (`[]`, décision t-528) montre le « + » centré.
- [ ] Le chemin d'aperçu ne modifie jamais le deck (garde `in_templates()` et `!is_common()` conservées) : test.
- [ ] Tests : session — vue d'aperçu d'un widget de pack de test à liste : ids d'items et noms de listes exacts, boîtes égales à celles que calcule `items.rs` pour le même rendu ; liste vide → `lists` len 0 ; rendu `WidgetTab` — « + », « × », ⋮⋮ sur l'aperçu → `editExampleList` (insert/remove/move) sur le cas affiché, aucune op de deck ; `max` atteint → pas de « + » ; focus du nouvel item ; e2e `lists-dock.cjs` : un ajout par « + » sur l'aperçu, avec assertion du nombre d'items. Commit : `feat(ui) : poignées des items de liste sur l'aperçu du widget (t-533)`.

### Task 8: Finitions du tableau et des poignées (t-534)

**Files:** `ui/src/a2ui/ListTable.tsx` (+ `.css`, hook extrait si > 500 lignes), `ui/src/ItemLayer.tsx`, `ui/src/stageItems.ts`, `docs/aide/{fr,en}.json` (diapo des raccourcis d'édition, clé de la ligne Cmd/Ctrl+Alt+flèche ajoutée par t-494), tests de rendu. Un commit par groupe, tous `(t-534)` : ListTable, ItemLayer, aide.

- [ ] ListTable — brouillon refusé : quand le cœur refuse une saisie (erreur de `run`), la cellule revient à la valeur des lignes. Test : enregistrement simulé qui échoue → valeur du DOM égale à la valeur d'origine.
- [ ] ListTable — Entrée sur la dernière ligne : l'insertion attend la fin de l'enregistrement de la cellule (`onChange` rend une promesse de succès) ; pas d'insertion si l'enregistrement est refusé ou si la saisie est invalide. Test : enregistrement différé → insert non appelé avant sa résolution ; refus → aucun insert.
- [ ] ListTable — popover rich ancré à sa cellule : positionné sous la cellule (ancre DOM de la cellule), contenu dans le dock (ne déborde pas à droite). Test de rendu : le popover est rendu dans le conteneur de la cellule (ou porte son ancre) ; contrôle visuel de la position par l'e2e `lists-dock.cjs` (boîte du popover sous la cellule).
- [ ] ItemLayer — repère et index de dépôt calculés sur le même ensemble (groupe sans l'item glissé). Test avec des items de tailles inégales : repère affiché = index appliqué.
- [ ] ItemLayer — « + » au bord : position bornée à l'intérieur de la page (`[0, largeur − taille du bouton]`, idem en hauteur). Tests : bord gauche/haut et bord droit/bas.
- [ ] Aide — le raccourci affiche ⌘ et Ctrl (FR, EN). Pas de test.
- [ ] `just e2e` (env isolé) vert.

### Task 9: Relecture des traductions des listes (t-535)

**Files:** `crates/oku-slides-i18n/src/locales/<lang>/{core,ui,agent}.ftl` pour `ar bn de es hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant` ; `crates/oku-slides-i18n/src/lib_tests.rs`.

- [ ] Test automatique (d'abord, RED si un écart existe) : pour les clés ci-dessous, chaque langue a exactement les mêmes variables Fluent que l'anglais ; et la phrase des listes de `agent-instructions-widgets` contient dans chaque langue les jetons `p.list`, `list-item`, `edit_list`, `defaults`, `set_param`.
- [ ] Relire contre l'anglais et le français : `widget-fault-empty-list`, `widget-fault-column-type`, `widget-fault-reserved-name`, `widget-fault-list-bounds`, `op-item-not-found`, `issue-empty-item-id`, `issue-duplicate-item-id`, `issue-list-bounds`, `issue-unreadable-item`, `ui-list-*`, `ui-undo-list-*`, et la seule phrase des listes de `agent-instructions-widgets` (le reste de la ligne n'est pas modifié). Corriger sens, registre, terminologie cohérente avec les clés voisines du même fichier ; jetons de code et variables inchangés.
- [ ] Relecture experte, non native : le rapport et le message de commit listent les langues effectivement corrigées. Commit : `fix(i18n) : traductions des listes relues (<langues>) (t-535)`.
