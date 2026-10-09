# Éléments d'une slide dans un `LoroTree` (t-694) — plan d'implémentation

Plan revu par l'équipe (14 relecteurs), rulings intégrés.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** déplacer un élément vers un autre slot, ou dans le slot d'un autre élément, ne perd plus l'édition qu'un pair fait en même temps sur cet élément.

**Architecture :**
- Aujourd'hui, chaque slot est une `LoroMovableList` (`Kind::Collection(Ctx::Element)`). Un déplacement entre slots y devient un retrait suivi d'une insertion, et l'édition concurrente de l'élément déplacé est perdue (`concurrent_move_across_slots_and_edit_converge`, `crates/oku-slides-core/tests/convergence.rs`).
- Désormais, chaque slide porte un `LoroTree` sous sa clé réservée `@elements`, qui contient ses éléments à toutes les profondeurs :
  - la méta d'un nœud est la carte de l'élément, comme aujourd'hui (`id`, `kind`, `params` **sans ses slots**, `frame`, `cell`, `style`) ;
  - la clé réservée `@slot` de la méta est une **carte** { id d'élément du parent (`""` pour la slide) → nom du slot }. Seule l'entrée du parent réel compte ; celles des anciens parents restent et servent de repli ;
  - les racines de l'arbre sont les éléments des slots de la slide, et les enfants d'un nœud sont ceux des slots de cet élément ;
  - l'ordre est l'index fractionnaire de Loro ; un déplacement (entre slots ou entre profondeurs) est un `mov_after` / `mov_before` / `mov`.
- Le JSON du deck ne change pas : les slots restent des tableaux dans `params`, et `DECK_VERSION` reste 5. Seul le replica change :
  - `write` réconcilie l'arbre par id d'élément ;
  - `read` reconstruit les slots ;
  - `schema` classe un paramètre `slot` en `Kind::Slot` ; `Kind::Collection` (sans charge) ne sert plus qu'aux slides.
- Les slides et les items de liste restent des `LoroMovableList`, et la parade t-263 (un commit par `mov`) ne vaut plus que pour eux.
- `SOURCES_VERSION` passe de 7 à 8, sans compatibilité (précédent : ADR-0004). Un marqueur `meta.format = 2` refuse en plus un `deck.loro` v7 relu hors zip (reprise de session) ou reçu par fusion.

**Tech Stack :** Rust, Loro 1.16.2 (`LoroTree`, `LoroMap`, `LoroMovableList`, `UndoManager`), serde_json, snafu, speculoos, proptest.

**Spec :** le design a été approuvé en séance le 2026-10-09 (approche A). Sonde : `crates/oku-slides-core/tests/loro_tree_probe.rs` (verdict GO AVEC PARADES). Décisions de revue (R1 à R13) intégrées au plan. Sous-tâches Oku : t-740 à t-745 (Tasks 1 à 6). Le déplacement entre slides est hors portée (t-746).

## Global Constraints

- **Lieu.** Tout se fait dans le worktree `/Users/matthieu/Projects/slider/.worktrees/694-elements-en-arbre`. Jamais de `cd` vers le dépôt principal.
- **MVP.** Aucune rétrocompatibilité, aucun chemin de lecture de l'ancien format (slots en `LoroMovableList`).
- **TDD.** Le test rouge est écrit et **vu échouer par une assertion** avant le code, puis il passe au vert. Corps provisoires plutôt que `todo!()` : par exemple un `sync_tree` qui n'écrit rien, ou un `read_tree` qui rend une carte vide. Un test vert d'emblée est un test de garde : le rapport de la tâche le nomme comme tel. Avant chaque `git add`, `grep -rn "todo!\|unimplemented!" crates/` doit être vide.
- **Régressions.** Elles vont dans `crates/<crate>/tests/regressions/issue_0694_<sujet>.rs`, déclarées dans le `main.rs` du dossier (ordre numérique).
- **Commits.**
  - Conventionnels, en français, avec la portée et la référence de la sous-tâche, par ex. `feat(replica)! : … (t-741)`. **Jamais de ligne `Co-Authored-By`.**
  - Le hook `oku.review=deferred` est posé pour ce worktree (`git config --worktree --get oku.review`). Chaque commande seule, dans cet ordre :
    1. `git add <fichiers>` ;
    2. `git diff --cached --name-only` (aucun fichier étranger) ;
    3. message écrit dans un fichier hors du dépôt (outil Write, dossier scratchpad), puis `git commit -F <fichier>`.
- **Fin de branche (R5, R12).** La branche est **squashée** sur `main`, c'est obligatoire : les commits intermédiaires (arbre écrit sous `SOURCES_VERSION` 7 jusqu'à la Task 5) ne doivent pas arriver sur `main`. Le message du squash porte ce pied :
  ```text
  BREAKING CHANGE: sources en v8 (éléments d'une slide en LoroTree, meta.format = 2) ; un zip v7 est refusé, une reprise de session v7 est mise de côté (.failed-*).
  ```
- **Cargo (R13).** On garde le dossier partagé : chaque commande est préfixée par `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target`, avec une seule compilation à la fois. En cas de conflit : verrou cargo et lenteur, pas d'erreur. Les tests tournent avec des dossiers de données temporaires :
  - ciblé : `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run -p oku-slides-core -E 'test(<motif>)'` ;
  - avant chaque commit : `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run --workspace --exclude oku-slides-app` ;
  - `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy --workspace --all-targets -- -D warnings` ;
  - `cargo fmt --check`.
- **UI.** Le JSON du deck ne change pas, donc `ui/` n'est pas touché. Si un fichier de `ui/` change malgré tout : `cd ui && bun run test && bunx --bun tsc -b`.
- **i18n.** Une seule clé nouvelle, `issue-slot-repaired`, dans les 20 `crates/oku-slides-i18n/src/locales/*/core.ftl`, à côté de `issue-unreadable-element`, traduite et pas recopiée de l'anglais.
- **Taille.**
  - Fichiers ≤ 1000 lignes. `replica/mod_tests.rs` (824 lignes) ne reçoit aucun test ; les nouveaux vont dans `replica/tree_tests.rs`, et dans un second fichier `replica/tree_read_tests.rs` au-delà de ~600 lignes. Les nouveaux tests de convergence vont dans `tests/tree_convergence.rs`.
  - `replica/ARCHITECTURE.md` et `crates/oku-slides-core/ARCHITECTURE.md` font **déjà 50 lignes** : le texte exact est donné en Task 1 et en Task 5, à vérifier avec `wc -l`.
- **Code.**
  - Pas d'`unwrap`/`expect` en production, et pas de repli silencieux qui masquerait un bogue.
  - Erreurs Snafu : `LoroSnafu` pour toute `LoroError`, plus les variantes nommées ci-dessous.
  - Aucune récursion non bornée sur un replica non fiable : l'écriture est itérative et bornée par `MAX_SLOT_DEPTH`, comme la lecture.
  - Dans les aides de test : `unwrap_or_else(|e| panic!(…))`.
- **Doc.** La première phrase d'une doc fait moins de 15 mots. Les tests sont nommés `<unité>_<scénario>_<résultat>`.

## Valeurs fixées

| Nom | Où | Valeur | Rôle |
|---|---|---|---|
| `ELEMENTS` | `replica/schema.rs` | `"@elements"` | clé de la carte d'une slide qui porte son `LoroTree` |
| `SLOT` | `replica/schema.rs` | `"@slot"` | clé de la méta d'un nœud : carte { id du parent → slot } |
| `ROOT_OWNER` | `replica/schema.rs` | `""` | clé de `@slot` pour un élément d'un slot de la slide |
| `ORPHANS` | `replica/schema.rs` | `"orphans"` | slot de repli d'un élément sans slot lisible |
| `Kind::Slot` | `replica/schema.rs` | variante | paramètre `slot` (déclaré, ou tableau d'éléments), nom non vide |
| `Kind::Collection` | `replica/schema.rs` | variante sans charge | `slides` du deck, et rien d'autre |
| `FORMAT` | `replica/mod.rs` | `"format"` | clé de la carte `meta` |
| `REPLICA_FORMAT` | `replica/mod.rs` | `2_i64` | 2 = éléments en arbre ; absent ou autre = refusé |
| `SOURCES_VERSION` | `sources.rs` | `8` | était 7 |
| `DECK_VERSION` | `model.rs` | `5` | **inchangé** |
| fractionnaire | `tree_at` | `enable_fractional_index(0)` | actif par défaut, explicité (sonde, parade 4) ; appel peu coûteux, répété à chaque obtention |

Variantes d'erreur ajoutées à `ReplicaInner`, dans `crates/oku-slides-core/src/error.rs` (vérifié sur `main` après réintégration) :

| Variante | `display` | `ReplicaErrorKind` |
|---|---|---|
| `ElementsNotATree { backtrace }` | `"slide elements are not a tree"` | `Corrupted` |
| `TreeOrder { backtrace }` | `"element placed before its owner"` | `Write` |
| `CollectionInParams { backtrace }` | `"collection inside the widget params"` | `Write` |
| `SlotsTooDeep { max: usize, backtrace }` | `"elements nested deeper than {max} slots"` | `Write` |
| `UnsupportedReplicaFormat { found: String, backtrace }` | `"unsupported replica format: {found}"` | `UnsupportedVersion` |

Aucune n'ajoute de clé i18n : `ReplicaError::localize` passe par le kind.

**Collision des clés réservées (vérifiée).** La réconciliation écrit `serde_json::to_value(deck)`. Ni `Slide` (`id`, `kind`, `params`, `name`, `description`, `notes`) ni `Element` (`id`, `kind`, `frame`, `cell`, `style`, `params`) n'a de `#[serde(flatten)]` ou de champ libre. Les noms de paramètres vivent un cran plus bas, dans la carte `params`. Aucune clé de premier niveau d'une slide ou d'un élément ne commence donc par `@`. Deux tests le fixent (Task 2).

## Règles de réconciliation et de lecture (référence des Tasks 2 et 3)

**Slot résolu d'un nœud `n` sous le parent réel `P`** (`resolved_slot`), d'id `pid` (`ROOT_OWNER` pour la racine). C'est la même règle à la lecture et à l'écriture :
1. `@slot[pid]`, si c'est une chaîne non vide ;
2. sinon, le slot résolu par la règle 1 du premier frère qui en a un, dans l'ordre des enfants de Loro ;
3. sinon, la première valeur valide de la carte `@slot`, dans l'ordre des clés (slot « inscrit » pour un autre parent) ;
4. sinon `ORPHANS`.

Les cas 2 à 4 émettent une `Issue` `issue-slot-repaired` (slide, élément, argument `slot`), une par nœud réparé, comme `issue-unreadable-element` aujourd'hui. Un élément n'est **jamais écarté** pour une raison de slot (R3). Un nom de slot que le parent ne déclare pas est gardé tel quel : la lecture se passe du registre, et `validate` signale `issue-unknown-param`.

**`ordered_children(tree, P, pid)`.** Les enfants vivants de `P` (`tree.children(P)`), chacun avec son slot résolu. Les slots résolus sont calculés **une seule fois** par parent, avant tout `mov` sous ce parent : sinon, la règle 2 d'un nœud réparé pourrait changer au fil des déplacements. Le regroupement par slot est stable, dans un `BTreeMap<String, Vec<Child>>` : un seul passage, sans recherche linéaire. La lecture et l'index courant de l'écriture l'utilisent tous les deux.

**Ordre des occurrences = ordre de `canonical::unique_params`** (F3 ; ce n'est **pas** un préordre). `unique_params` traite d'abord toute la liste d'un slot, puis descend dans chacun de ses éléments. L'ordre `uorder(P)` est donc :

```text
pour chaque slot S de P, par nom :
    pour chaque élément e de S, dans l'ordre : compter (id(e), n)
    pour chaque élément e de S, dans l'ordre : uorder(e)
```

`current` (arbre) et `wanted` (JSON) sont construits dans cet ordre, de façon itérative (pile explicite de cadres `(parent, slot, phase)`). Une même clé `(id, n)` désigne alors le nœud que `unique_ids` garde sous le nom `id` à la relecture. Exemple : avec `blocks = [pile{children:[x]}, x]`, `(x, 0)` est le `x` de `blocks`, et l'enfant de `pile` est `(x, 1)`, relu `x-2`. Le nœud relu sous un nom réparé (`x-2`) n'a pas de clé `(x-2, n)` dans l'arbre : le premier `apply` du deck relu le recrée une fois sous ce nom (nouveau `TreeID`, exception F1, comme les listes avec t-652), puis le second n'écrit plus rien.

**Écriture d'une slide** (`Writer::sync_map`, `Ctx::Slide`) :
1. Ses `params` sont réconciliés comme aujourd'hui, **sauf** les paramètres `Kind::Slot`, absents de la carte `params` (une entrée restée sous ce nom est supprimée).
2. L'arbre est obtenu par `tree_at(map, ELEMENTS)`. Il est créé d'avance, même vide (`Ctx::trees()`, parade t-188). S'il porte un conteneur d'un autre type, c'est l'erreur `ElementsNotATree`, sans écrasement.
3. `Writer::sync_tree(&tree, &slots_of(params, schema))`.

**`Writer::sync_tree(tree, roots)`**, sans aucun `split`.

*Temps 0 — index*, sans aucune écriture.
- `current = live_index(tree)` : les nœuds vivants à une profondeur `< MAX_SLOT_DEPTH` (racines = 0), dans l'ordre `uorder`. Plus profond, un nœud n'est ni indexé, ni lu, ni touché, ni supprimé un par un (R7b). Chaque entrée `Live { key: (String, usize), node: TreeID, parent: Option<usize>, end: usize }` porte :
  - `key` : la clé `(id, occurrence)` ;
  - `parent` : l'index de son parent dans `current` (`None` pour une racine) ;
  - `end` : la fin de son sous-arbre, au sens d'un intervalle d'index dans un second ordre préordre tenu à côté (`pre: Vec<usize>`), qui sert au saut du temps 1.
- `wanted = wanted_index(self.registry, roots)?` : les éléments voulus du JSON, dans l'ordre `uorder`. Une entrée `Wanted<'a> { key: (String, usize), owner: Option<usize>, slot: &'a str, depth: usize, obj: &'a Map<String, Value> }`, où `owner` est l'index du propriétaire dans `wanted` (`None` = la slide). Le schéma d'un élément vient de `self.registry.get(kind)` pour `slots_of`.
- **Borne (F4).** Un élément voulu de `depth >= MAX_SLOT_DEPTH` donne l'erreur `SlotsTooDeep { max: MAX_SLOT_DEPTH }`, rendue **avant toute écriture dans cette slide**. Sans elle, ces nœuds, jamais indexés par `current`, seraient recréés à chaque `apply`.

*Temps 1 — suppressions (R2).*
- On calcule `has_wanted` pour chaque nœud de `current` : son sous-arbre contient-il une clé voulue ? Un passage du préordre `pre` à l'envers suffit.
- On parcourt `pre`. Un nœud non voulu sans descendant voulu est supprimé par `tree.delete(n)`, puis on saute son sous-arbre (`end`), sans jamais appeler `is_node_deleted` sur ses descendants : coût linéaire.
- Un nœud non voulu qui a des descendants voulus va dans `deferred`, qu'on traite au temps 3.

*Temps 2 — placement, par (parent, slot).* Pour chaque parent voulu `P` (la racine, puis chaque entrée de `wanted` dans l'ordre `uorder` ; un propriétaire y précède toujours ses éléments), d'id `pid` :
- `node(P)` vient de `placed[owner]`. Si l'index manque, c'est l'erreur `TreeOrder`, sans repli.
- `kids` : une copie locale de `tree.children(P)`, tenue à jour pendant le traitement de `P`, après **chaque** `mov` **et chaque** `create_at`.
- Les slots résolus des enfants de `P` sont calculés une fois, au début de `P` (`ordered_children`).
- Pour chaque slot `S` de `P` (dans l'ordre des noms), avec `W = [w1..wk]` les voulus de `(P, S)` :
  1. `C` : les enfants actuels de `P`, dans l'ordre de `kids`, dont la clé est voulue **sous `(P, S)`** et dont le slot résolu vaut `S`. Les nœuds voulus ailleurs et les autres slots sont ignorés.
  2. **Stables (F2)** : on prend la suite des rangs dans `W` des nœuds de `C`. Les nœuds d'une plus longue sous-suite croissante (`lis(ranks: &[usize]) -> Vec<bool>`, O(k log k), déterministe : à longueur égale, la sous-suite qui finit au plus petit rang) sont **stables** et ne bougent pas.
  3. On parcourt `W` avec un prédécesseur `pred` (initialement `None`) :
     - **nœud stable `n`** : aucun `mov` ;
     - **nœud connu non stable** :
       - `pred = Some(p)` : `tree.mov_after(n, p)` ;
       - sinon, s'il existe un nœud stable plus loin dans `W` : `tree.mov_before(n, premier stable suivant)` ;
       - sinon : `tree.mov(n, P)` (en fin de parent) ;
     - **nœud inconnu** : `n = tree.create_at(P, idx)`, où `idx` est la position dans `kids` qui suit `pred`, ou celle du premier stable suivant, ou `kids.len()`. La méta reçoit aussitôt `@slot` (carte vide, `map_at`) ;
     - dans tous les cas : `set_value(&map_at(&meta, SLOT)?, pid, S)` (aucune op si l'entrée vaut déjà `S` ; les entrées d'autres parents ne sont pas effacées), puis `self.sync_map(&meta, w.obj, Ctx::Element)?`, `placed.push(n)` et `pred = Some(n)`.
  - Loro ne produit aucune op pour un `mov` vers la position où le nœud est déjà (`mov_with_txn`), mais la LIS évite surtout de toucher les nœuds qu'un pair déplace peut-être en parallèle. Exemple : `C = [a, b, c]`, `W = [b, c, a]` donne un seul `mov_after(a, c)`.
- Les slots de `P` sont indépendants : un ordre brut qui entrelace `blocks` et `elements` après une fusion n'entraîne aucune écriture.
- Il n'y a pas de cycle local : `P` est placé avant ses enfants, sa chaîne d'ancêtres n'est faite que de nœuds placés, et `n` n'en fait pas partie.

*Temps 3.* Chaque nœud de `deferred`, en préordre, est supprimé s'il n'est pas déjà supprimé. `is_node_deleted` ne vise que ces nœuds rares. Le temps 3 est complet : un nœud voulu sous un différé en sort au temps 2, car sa chaîne de propriétaires voulus remonte à la racine sans passer par le différé.

**Invariant (F1).** Rien n'est écrit quand rien ne change : `apply(&replica.deck().0)` laisse `len_ops` inchangé, même après une fusion. **Exception** : une réparation faite à la relecture s'écrit une fois, au premier `apply` du deck relu. C'est le cas des ids en double (t-652 : copie écartée ou renommée `{id}-{n}`) et de `issue-slot-repaired`. Le second `apply` n'écrit plus rien. Les tests déterministes sans annulation ni doublon assertent l'invariant strict dès le premier `apply` ; le proptest (avec `Edit::Undo`) l'asserte sur le second.

**Lecture d'une slide** (`Reader::read_map_at`, `Ctx::Slide`) :
- après la boucle des clés, un `Container::Tree` sous `ELEMENTS` est lu par `read_tree`, puis versé dans `out["params"]` par `merge_slots` (créé au besoin ; un slot l'emporte sur un paramètre de même nom), puis `sort_keys()` sur `params` et sur `out` ;
- un `ELEMENTS` d'un autre type est ignoré, avec une `Issue` `issue-unreadable-element` (élément `None`, la slide renseignée par `read_slides`).

**`Reader::read_tree(tree, parent, pid, depth)`.** `depth.slots` est la profondeur des nœuds lus (0 pour les racines).
- On prend les enfants par `ordered_children`.
- Méta d'un nœud : `read_map_at(meta, Ctx::Element, levels + 2)`, qui saute les clés réservées. Une méta illisible (`get_meta` en erreur) écarte le nœud avec `issue-unreadable-element`.
- On désérialise d'abord l'élément **sans ses enfants**. S'il ne se lit pas en `Element`, on l'écarte avec tout son sous-arbre (`issue-unreadable-element` avec son id), sans lire ses enfants, comme aujourd'hui pour un élément illisible.
- Ses enfants : s'il en a et que `depth.slots + 1 >= MAX_SLOT_DEPTH`, on émet une `issue-slot-too-deep` par nœud (aujourd'hui, une par liste ; écrit dans l'ADR) et on ne les lit pas. Sinon, `read_tree(tree, Node(n), id, depth.slots + 1)`, versé par `merge_slots` puis `sort_keys()`.
- Ids en double (suppression défaite, t-652) : rien ne change, `canonical::unique_ids` les traite dans `materialize`. Une copie exacte n'est écartée que **dans le même slot**, et une occurrence dans un autre slot est renommée `{id}-{n}`.

---

## Task 1: ADR-0005 et ARCHITECTURE du replica (t-740)

**Files :**
- Create : `docs/adr/0005-elements-d-une-slide-en-arbre-loro.md`.
- Modify :
  - `docs/adr/README.md` ;
  - `docs/adr/0002-sources-du-deck-en-crdt-loro.md` (paragraphe « Quatre catégories de champs », l. 42-46, et bloc t-263, l. 64-66) ;
  - `crates/oku-slides-core/src/replica/ARCHITECTURE.md`.

- [ ] **Step 1 : écrire l'ADR-0005**, au format de l'ADR-0004 (skill `common:new-adr`). En-tête : statut accepté, date 2026-10-09, tâches t-694 (t-740 à t-745), « Remplace en partie : ADR-0002 (slots en `LoroMovableList`) », et un renvoi au rapport de sonde.
  - **Contexte** :
    - un slot est une `LoroMovableList`, donc changer de slot = un retrait et une insertion, et l'édition concurrente de l'élément déplacé est perdue ;
    - plusieurs `mov` d'un même commit se défont mal (t-263), d'où un commit par `mov`.
  - **Décision** :
    - un `LoroTree` par slide sous `@elements` ;
    - la méta d'un nœud = la carte de l'élément ;
    - `@slot` = une carte { id du parent (`""` : la slide) → slot } : une entrée par parent, jamais effacée, et seule celle du parent réel compte ;
    - ordre fractionnaire (jitter 0) ;
    - placement par ordre relatif dans chaque (parent, slot), par `mov_after`/`mov_before` ;
    - réconciliation par id d'élément ;
    - JSON du deck inchangé ;
    - identité = l'`id` de la méta, jamais le `TreeID` ;
    - slides et items de liste restent en `LoroMovableList`.
  - **Réparation à la relecture** : les quatre règles du slot résolu, avec `issue-slot-repaired`, sans jamais écarter un élément pour son slot ; ids en double par `unique_ids` (t-652 : copie exacte écartée dans un même slot, renommée d'un slot à l'autre) ; profondeur bornée par `MAX_SLOT_DEPTH` à la lecture **et** à l'écriture (un deck JSON plus profond est refusé par `SlotsTooDeep`), et `issue-slot-too-deep` émise une fois par nœud. Un élément rangé dans `orphans`, ou dans un slot que son parent ne déclare pas, est gardé dans le JSON mais **n'est pas rendu** : la mise en page ne lit que les slots déclarés. `validate` signale `issue-unknown-param`, et l'`apply` suivant inscrit la réparation (`@slot[pid] = "orphans"`) ; une réparation à la relecture s'écrit ainsi une fois, et l'invariant « rien n'est écrit » vaut à partir du second `apply`.
  - **Compatibilité** :
    - aucune. `SOURCES_VERSION` passe de 7 à 8 ;
    - le marqueur `meta.format = 2` est vérifié à l'import et à la fusion. Un `deck.loro` de reprise v7 est refusé (`replica-unsupported-version`) et la session met son dossier de côté (`.failed-*`) : les modifications non enregistrées d'une session reprise sont gardées, pas relues, au premier lancement après la mise à jour ;
    - `DECK_VERSION` ne change pas (5) ;
    - règle : changer `REPLICA_FORMAT` impose de changer `SOURCES_VERSION`.
  - **Conséquences** :
    - un déplacement entre slots ou entre profondeurs garde les éditions concurrentes de l'élément ;
    - la parade t-263 disparaît pour les éléments : l'arbre s'écrit en un commit. Elle reste pour les slides et les items.
  - **Limites** :
    - un élément déplacé sous un élément supprimé en parallèle disparaît avec lui (sonde, point 3) ;
    - deux imbrications croisées qui formeraient un cycle : le premier `mov` dans l'ordre (lamport, pair) gagne, et le perdant reste visible à son ancienne place (son entrée `@slot` pour l'ancien parent est intacte) ;
    - défaire une suppression recrée un nœud neuf, avec un autre `TreeID`. Loro ne ressuscite jamais un nœud supprimé : face à un déplacement concurrent, seule la copie reste et l'édition du pair est perdue. Avec les listes (t-652), les deux versions restaient (copie renommée `c2-2`). Suite : t-747 ;
    - **déplacer un élément d'une slide à une autre perd toujours l'édition concurrente** (un arbre par slide) : tâche t-746 ;
    - avec un jitter de 0, deux insertions concurrentes au même endroit reçoivent le même fractionnaire. Une insertion entre elles fait émettre par Loro un `Rearrange`, c'est-à-dire des `mov` implicites de frères, qui peuvent ramener un frère qu'un pair déplaçait en parallèle ;
    - Loro est figé par `Cargo.lock` (1.16.2) : pas de `cargo update -p loro` sans relancer la sonde (`tests/loro_tree_probe.rs`).
- [ ] **Step 2 : ADR-0002.**
  - Dans « Quatre catégories de champs » (l. 42-46), remplacer « collections ordonnées à `id` (`slides`, `blocks`, `elements`, `children`) → `LoroMovableList` de `LoroMap` » par « collections ordonnées à `id` : `slides` → `LoroMovableList` de `LoroMap` ; les slots (`blocks`, `elements`, `children`) → nœuds d'un `LoroTree` par slide, voir l'[ADR-0005](0005-elements-d-une-slide-en-arbre-loro.md) ».
  - L. 66, après « (t-263) », ajouter : « ; plus nécessaire pour les éléments depuis l'ADR-0005 ».
  - Dans le README, ajouter la ligne `| [0005](0005-elements-d-une-slide-en-arbre-loro.md) | Les éléments d'une slide en arbre Loro | accepté |`.
- [ ] **Step 3 : `replica/ARCHITECTURE.md`.** Le fichier fait 50 lignes : le remplacer **entièrement** par le texte ci-dessous (46 lignes). Ce qui est condensé :
  - `host` et `rich` sur une ligne ;
  - `Prose` et `Rich` sur une ligne ;
  - `Host` de 4 à 2 lignes ;
  - « Diff » et « Lecture sans registre » réécrites sur 2 lignes chacune ;
  - t-188 et t-263 sur une ligne chacune.

  Vérifier `wc -l` ≤ 50.

  ```markdown
  # replica

  Le deck dans un `LoroDoc` (CRDT) : historique complet, fusion, annuler.
  `DeckReplica` fait foi ; le `Deck` n'en est qu'une vue.

  ## Modules

  - `mod.rs` : `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`, annuler).
  - `schema` : catégorie (`Kind`) de chaque champ, selon son contexte (`Ctx`) et le registre.
  - `write` : `Writer`, réconciliation des conteneurs vers un `Deck` JSON ; `tree` : arbre des éléments.
  - `read` : lecture totale, sans registre ; l'illisible est écarté et signalé (`Issue`).
  - `host` : option `host(chart|table)` ; `rich` : paragraphes dans un `LoroText` à marques.
  - `undo` : étapes d'annulation, groupes, clés de fusion.
  - `authors` : nom de chaque pair (racine `peers`), `MergeReport`.
  - `history` : une `Version` par édition, aperçu d'une version passée.

  ## Catégories (`Kind`)

  - `Prose` : `LoroText`, fusion au caractère. `Rich` : `LoroMap { rich: LoroText }` à marques.
  - `Collection` : `LoroMovableList` de `LoroMap` à `id` (slides).
  - `Slot` : nœuds du `LoroTree` de la slide (`@elements`) ; méta = l'élément sans ses slots,
    `@slot` = carte { id du parent (`""` : la slide) → slot } ; ordre fractionnaire (ADR-0005).
  - `List` : `LoroMap { list: LoroMovableList }`, items d'un paramètre `list`.
  - `Host` : `LoroMap` ; `dataset` en `{columns, rows}`, `rows` de `{id, cells}`, `cells` par
    colonne ; autres clés LWW. Un `dataset` d'une autre forme s'écrit opaque, sans perte.
  - `Atom` : valeur JSON entière, LWW. `Nested` : objet en `LoroMap`, sinon LWW.

  ## Règles

  - **Diff du JSON du deck** : `apply` amène chaque conteneur à l'état du `Deck` sérialisé ;
    rien n'est écrit quand rien ne change (arbre : ordre comparé par parent et slot).
  - **Lecture sans registre** : le type se lit dans le conteneur ; `{columns, rows}` ne se
    lit que sous `dataset` d'un paramètre à valeur de carte (seul un hôte s'y écrit).
  - **Réparation à la relecture** : colonne en double, et ligne en double aux mêmes
    cellules, écartées ; cellule absente lue `null` ; autre identifiant en double ou invalide
    remplacé (`r{i}`/`c{j}`, puis `-{n}`) ; bornes de 10 000 lignes, 1 000 colonnes et
    1 000 000 cellules (`issue-data-truncated`), au-delà `Opaque` ; profondeur bornée par
    `MAX_DEPTH`, arbre par `MAX_SLOT_DEPTH` (écriture comprise). Slides, éléments d'une slide
    (tout l'arbre) ou items au même id (t-652) : copie exacte d'un voisin écartée, autre
    occurrence renommée `{id}-{n}` ; id vide laissé. Sans `@slot` pour son parent : slot d'un
    frère, sinon slot inscrit, sinon `orphans` (`issue-slot-repaired`) ; jamais écarté.
  - **Parade t-188** : conteneurs déclarés et arbre d'une slide créés d'avance, vides.
  - **Parade t-263** : un commit par `mov` de slide ou d'item (`split`) ; l'arbre, en un seul.
  - **Format** : `meta.format = 2`, vérifié à l'import et à la fusion ; changer ce format
    impose de changer `SOURCES_VERSION`.
  - **Clés triées** à la lecture, comme la forme canonique.
  ```
- [ ] **Step 4 : commit.** C'est de la doc seule, pas de tests à lancer.

  ```text
  docs(adr) : ADR-0005, éléments d'une slide en arbre Loro (t-740)
  ```

---

## Task 2: écriture et lecture de l'arbre, catégorie `Slot` (t-741)

L'écriture et la lecture sont couplées par l'aller-retour : chaque test du replica relit ce qu'il écrit. Elles forment donc **une seule tâche et un seul commit**, avec un jalon interne (Step 4). Le repli des slots (R3, R4) en fait partie : aucun élément n'est perdu à aucun commit.

**Files :**
- Create :
  - `crates/oku-slides-core/src/replica/tree.rs` ;
  - `crates/oku-slides-core/src/replica/tree_tests.rs` ;
  - `crates/oku-slides-core/tests/regressions/issue_0694_move_across_slots_keeps_concurrent_edit.rs`.
- Modify :
  - `src/replica/schema.rs`, `src/replica/write.rs`, `src/replica/read.rs` ;
  - `src/replica/mod.rs` (`mod tree;`, doc de module) ;
  - `src/error.rs` (`ElementsNotATree`, `TreeOrder`, `CollectionInParams`, `SlotsTooDeep` et leur kind) ;
  - `crates/oku-slides-i18n/src/locales/*/core.ftl` (20 fichiers : `issue-slot-repaired`) ;
  - aides de tests qui naviguent dans les conteneurs :
    - `src/replica/host_tests.rs` (`host_map`, l. 106-114) ;
    - `src/replica/mod_tests.rs` (`block_params`, l. 50-70 ; `read_deeply_nested_slots_stops_at_the_depth_bound`, l. 673-698) ;
    - `src/replica/list_tests.rs` (`items_list`, l. 97-104) ;
    - `src/replica/read_tests.rs` (`put`, `read`, l. 13-41) ;
  - `tests/convergence.rs` (`concurrent_move_across_slots_and_edit_converge`, l. 630-667) ;
  - `tests/regressions/main.rs`.

**Consommateurs recensés** (grep fait) :
- `Collection(` : `schema.rs` l. 43, 81, 119, 130, 142, 167 ; `write.rs` l. 142, 153, 213, 223 ; `read.rs` l. 148. Après la tâche, `Kind::Collection` (sans charge) ne vient que de `(Deck, "slides")`.
- Lecture des slots en `MovableList` dans `read.rs` :
  - `read_params` **reste** : il sert aux items et aux hôtes. Seules ses deux branches `ValueOrContainer::Container(Container::MovableList(l))` partent (l. 213-225 du worktree) ;
  - `read_list` (l. 258-286) perd sa branche `Ctx::Element` et devient `read_slides(&mut self, list, depth)`.
- `MAX_DEPTH`/`MAX_SLOT_DEPTH` : `read.rs` l. 17, 25, 78, 180, 213. `Depth.slots` sert désormais à `read_tree`.
- `unique_ids` : `read.rs` `materialize`, l. 324, inchangé.
- `tests/untrusted.rs` `from_snapshot_with_malformed_container_state_returns_corrupted` (l. 69-95) : les états de conteneurs changent (arbres). Le lancer ; il doit rester vert (« plus de la moitié des états tronqués vus »), ce qui exige que `read_tree` lise la méta de chaque nœud lisible.
- `tests/regressions/issue_0652_undo_remove_vs_peer_move.rs` : ses cas « élément » doivent rester verts. Lire l'assertion qui échoue avant de toucher à quoi que ce soit.
- `crates/oku-slides-session/src/files.rs` l. 53-66 : `issue-slot-repaired` n'est **pas** une perte, il ne doit pas correspondre au motif `issue-unreadable-*` (vérifier).
- Aucune fixture binaire (`*.loro`, `*.slider`, zip) n'est versionnée (`git ls-files`).

**Interfaces :**

`schema.rs` :
```rust
/// Clé de la carte d'une slide qui porte l'arbre de ses éléments.
pub(super) const ELEMENTS: &str = "@elements";
/// Clé de la méta d'un nœud : slot par id de parent.
pub(super) const SLOT: &str = "@slot";
/// Clé de `@slot` d'un élément d'un slot de la slide.
pub(super) const ROOT_OWNER: &str = "";
/// Slot de repli d'un élément sans slot lisible.
pub(super) const ORPHANS: &str = "orphans";

pub(super) enum Kind {
    Prose, Rich,
    /// `LoroMovableList` de `LoroMap` à `id` : les slides.
    Collection,
    /// Éléments d'un slot : nœuds de l'arbre de la slide.
    Slot,
    List, Atom, Nested(Ctx), Host,
}

impl Ctx {
    /// Arbres créés dès la création de l'objet : `Slide` → `&[ELEMENTS]`, autres → `&[]`.
    pub(super) fn trees(self) -> &'static [&'static str];
    /// Clés du replica, jamais lues ni écrites comme des champs : `Slide` → `&[ELEMENTS]`, `Element` → `&[SLOT]`.
    pub(super) fn reserved(self) -> &'static [&'static str];
}
```
- `Ctx::kind` : `(Deck, "slides") => Kind::Collection`.
- `declared_kind` : `ParamType::Slot { .. } => Kind::Slot`.
- `shape_kind` : `Value::Array(a) if is_elements(a) => Kind::Slot`.
- `fits` : `(Kind::Slot, Value::Array(a)) => a.is_empty() || is_elements(a)`.
- `param_kind` : un **nom vide** n'est jamais `Kind::Slot`. Si la catégorie trouvée est `Slot` et que `name.is_empty()`, rendre `Kind::Atom` (R7c).
- `column_kind` : `Kind::Collection | Kind::Slot | Kind::List | Kind::Host => Kind::Atom`.
- `declared` filtre aussi `Kind::Slot`.

`tree.rs` (nouveau, ~250 lignes, doc de module : « L'arbre des éléments d'une slide : placement par id, ordre des enfants. ») :
```rust
/// L'arbre de `map[key]`, créé au besoin ; erreur si la clé porte autre chose.
pub(super) fn tree_at(map: &LoroMap, key: &str) -> Result<LoroTree, ReplicaInner>; // ElementsNotATree
/// Le slot de `@slot[owner]` : chaîne non vide, sinon `None`.
pub(super) fn slot_for(meta: &LoroMap, owner: &str) -> Option<String>;
/// Première valeur valide de `@slot`, par ordre des clés (règle 3).
pub(super) fn any_slot(meta: &LoroMap) -> Option<String>;
/// Un enfant, son slot résolu, et si ce slot a été réparé (règles 2 à 4).
pub(super) struct Child { pub(super) node: TreeID, pub(super) slot: String, pub(super) repaired: bool }
/// Enfants vivants de `parent` (d'id `pid`), groupés (stable) par slot résolu.
pub(super) fn ordered_children(tree: &LoroTree, parent: TreeParentId, pid: &str) -> BTreeMap<String, Vec<Child>>;
/// Les slots de `params` (catégorie `Kind::Slot`), triés par nom.
pub(super) fn slots_of<'v>(params: Option<&'v Map<String, Value>>, schema: Option<&[ParamSchema]>) -> Vec<(&'v str, &'v [Value])>;
/// Nœuds vivants de profondeur < `MAX_SLOT_DEPTH`, dans l'ordre `uorder`, et leur préordre.
fn live_index(tree: &LoroTree) -> (Vec<Live>, Vec<usize>);
/// Éléments voulus dans l'ordre `uorder` ; `SlotsTooDeep` au-delà de la borne.
fn wanted_index<'a>(registry: &WidgetRegistry, roots: &[(&'a str, &'a [Value])]) -> Result<Vec<Wanted<'a>>, ReplicaInner>;
/// Les positions d'une plus longue sous-suite croissante de `ranks` (stables).
fn lis(ranks: &[usize]) -> Vec<bool>;
impl Writer<'_> {
    /// Amène l'arbre `tree` aux slots `roots` de la slide (voir les règles).
    pub(super) fn sync_tree(&self, tree: &LoroTree, roots: &[(&str, &[Value])]) -> Res;
}
#[cfg(test)]
/// Le nœud vivant d'id `id` de la slide `slide`, et son arbre (tests).
pub(super) fn node_of(doc: &LoroDoc, slide: &str, id: &str) -> (LoroTree, TreeID);
#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;
```
- `ordered_children` : `tree.children(parent).unwrap_or_default()`. Une méta illisible donne un `Child` à réparer. `slots_of` utilise une petite fonction locale `elements(value) -> &[Value]`, sans exposer `write::objects`.
- `node_of` (tests seulement) panique avec un message si le nœud est absent.

`write.rs` :
- `keyed` devient `pub(super)`.
- `Writer::sync_map` :
  - `stale` exclut `ctx.reserved()` ;
  - `for key in ctx.trees() { tree_at(map, key)?; }` à côté de `texts()`/`lists()` ;
  - `Kind::Collection => self.sync_list(&list_at(map, key)?, objects(value), &|m, o| self.sync_map(m, o, Ctx::Slide))?` ;
  - branche `owner` : après `sync_params`, si `ctx == Ctx::Slide`, appeler `self.sync_tree(&tree_at(map, ELEMENTS)?, &slots_of(params, schema))?`. Pour `Ctx::Element`, rien de plus ;
  - `Kind::Slot` rejoint `Kind::List | Kind::Host => ContainerOutsideParamsSnafu.fail()` dans les deux `match` (jamais rendu par `Ctx::kind`).
- `Writer::sync_fields` :
  - boucle `declared` : `Kind::Slot` n'a pas de conteneur créé d'avance ;
  - écriture : `Kind::Slot => if map.get(key).is_some() { map.delete(key).context(LoroSnafu)? }` (« écrit dans l'arbre de la slide, `tree.rs` ») ;
  - `Kind::Collection => return CollectionInParamsSnafu.fail()`.
- Doc de `reorder` : « slides et items de liste ; les éléments sont dans un arbre, voir `tree.rs` ».

`read.rs` :
```rust
impl Reader {
    /// Les slots des enfants de `parent`, par nom (voir les règles de lecture).
    fn read_tree(&mut self, tree: &LoroTree, parent: TreeParentId, pid: &str, depth: Depth) -> Map<String, Value>;
    /// Les slides de la liste `slides` (ex-`read_list`).
    fn read_slides(&mut self, list: &LoroMovableList, depth: Depth) -> Vec<Value>;
}
/// Verse `slots` dans `obj["params"]`, créé au besoin, puis trie ses clés.
fn merge_slots(obj: &mut Map<String, Value>, slots: Map<String, Value>);
```
- `read_map_at` saute `ctx.reserved()`. Pour `Ctx::Slide`, après la boucle : `read_tree(&t, TreeParentId::Root, ROOT_OWNER, Depth { levels: depth.levels + 1, slots: 0 })`, `merge_slots`, puis `out.sort_keys()`.
- Les `Issue` de réparation : `Issue { slide: None, element: Some(id), message: Message::new("issue-slot-repaired").arg("slot", slot) }`. La slide est renseignée par `read_slides`.

i18n, `issue-slot-repaired` (argument `slot`) :
- fr : `élément sans slot connu, rangé dans { $slot }` ;
- en : `element without a known slot, placed in { $slot }` ;
- les 18 autres traduites.

- [ ] **Step 1 : régression et convergence, rouges.**
  - `tests/regressions/issue_0694_move_across_slots_keeps_concurrent_edit.rs` (déclaré dans `main.rs` après `issue_0677_…`), test `move_across_slots_keeps_the_concurrent_bold` :
    - registre `common::shared()`, deck `roadmap`, et deux pairs issus d'un même snapshot ;
    - A retire `b-points` de `blocks` de `s-constat` et l'ajoute à `elements` avec un `frame` `{x:0,y:0,w:400,h:200}` (ops `remove_element` puis `add_element`) ;
    - B met en gras le premier run du `body` de `b-points`, **visé par id** (`slide.locate("b-points")`, pas `toggle_bold`). On vérifie d'abord que ce run n'était pas gras dans le deck de départ et qu'il l'est dans le deck de B ;
    - après un `merge` croisé : même deck relu par les deux pairs, aucune `Issue` ;
    - `b-points` est présent une seule fois, dans `elements`, avec le `frame` de A **et** le gras de B.
  - `tests/convergence.rs`, `concurrent_move_across_slots_and_edit_converge` :
    - remplacer le commentaire « Le gras posé par B est perdu… suivi : t-694 » par « Le déplacement entre slots est un `mov` de l'arbre : le gras posé par B est gardé (t-694). » ;
    - vérifier que `toggle_bold` a basculé `b-points` dans le deck de B (`first_run_bold(e: &Element) -> bool`, nouvelle aide) ;
    - asserter que `found[0].1` est gras et porte `DEFAULT_FRAME`.
  - Lancer : les deux échouent sur l'assertion du gras. Noter la sortie.
- [ ] **Step 2 : squelettes compilables.** Écrire `tree.rs` avec les signatures. `sync_tree` rend `Ok(())` sans rien écrire, `read_tree` rend une carte vide, `slots_of` rend `vec![]` et `lis` rend `vec![false; n]` ; `schema.rs` est modifié. Les tests du replica échouent alors par **assertion** (éléments absents), pas par panique.
- [ ] **Step 3 : tests unitaires rouges** dans `src/replica/tree_tests.rs`.

  Aides :
  - `registry()` = `crate::test_registry::shared()`, ainsi que `roadmap()` et `with_ops(deck, ops)` comme dans `mod_tests.rs` ;
  - `two_slots()` = `roadmap` + un texte cadré `n` dans `elements` de `s-constat` ;
  - `with_pile()` = `roadmap` + un `stack` `pile` dans `blocks` de `s-constat` ;
  - `ops_of(r)` = `r.len_ops()`.

  Tests :
  - `reserved_keys_never_collide_with_model_fields` (garde, vert d'emblée) : un `Element` dont `frame`, `cell`, `style` et `params` sont non vides, et une `Slide` avec tous ses champs, sont sérialisés. Aucune clé de premier niveau ne commence par `@`.
  - `param_named_like_a_reserved_key_stays_in_params` (garde) : un paramètre de texte nommé `@slot` sur `b-points` revient de l'aller-retour à l'identique, et la carte `@slot` de la méta est intacte.
  - `reconcile_slide_elements_live_in_the_slide_tree` : après `DeckReplica::new(roadmap)`, la carte de `s-constat` porte un `Container::Tree` sous `@elements`, sa carte `params` n'a pas de clé `blocks`, et la méta de `b-points` a `@slot == { "": "blocks" }`.
  - `round_trip_elements_in_two_slots_is_identical` : `two_slots()` → `new` → `snapshot` → `from_snapshot` donne un deck égal et aucune `Issue`.
  - `round_trip_nested_children_is_identical` : `with_pile()` + deux textes dans `pile.children`. La relecture est égale, et les clés de `params` de la slide et de `pile` sont triées.
  - `apply_move_across_slots_keeps_the_tree_node` : sur `two_slots()`, `b-points` va de `blocks` à `elements`. Son `TreeID` ne change pas (`node_of`), et `@slot[""] == "elements"`.
  - `apply_move_into_a_child_slot_keeps_the_tree_node` : sur `with_pile()`, `b-points` va dans `pile.children`. Même `TreeID`, son parent devient `pile`, `@slot["pile"] == "children"` et `@slot[""] == "blocks"` (entrée gardée).
  - `apply_reorder_of_elements_is_one_change` : une slide `s` porte 4 textes `a..d` dans `blocks`, et on applique l'ordre inverse. `len_changes` augmente d'exactement autant qu'un `apply` de référence qui ne change qu'un titre (un change), et l'ordre relu est l'inverse exact. Rouge sur l'ancien code (un commit par `mov`).
  - `moving_the_first_of_three_to_the_end_is_one_move` (F2) : avec `[a, b, c]` dans `blocks`, l'ordre `[b, c, a]` augmente `len_ops` de **exactement 1** (un seul `mov_after(a, c)` ; `@slot` inchangé).
  - `lis_marks_a_longest_increasing_subsequence` : `lis(&[2, 0, 1]) == [false, true, true]`, `lis(&[]) == []`, `lis(&[3, 2, 1, 0])` a exactement un `true`.
  - `nested_duplicate_keeps_the_outer_tree_id` (F3) : forger `blocks = [pile{children:[x "A"]}, x "B"]` (deux nœuds `x`, textes différents). Relire : `x` (« B ») est dans `blocks`, `x-2` (« A ») sous `pile`. Après un `apply` du deck relu, le `TreeID` du `x` de `blocks` n'a pas changé, et l'enfant de `pile` est relu `x-2` avec son contenu « A ». Le premier `apply` le recrée une fois (exception F1), le second n'écrit plus rien. Une relecture redonne le même deck.
  - `apply_of_a_too_deep_deck_twice_does_not_grow` (F4) : `r = DeckReplica::new(canvas_chain(MAX_SLOT_DEPTH))` (aide copiée de `src/sources_tests.rs:410`). Deux `apply(&canvas_chain(MAX_SLOT_DEPTH + 1))` rendent chacun une erreur de kind `ReplicaErrorKind::Write` (`SlotsTooDeep`), et `len_ops` est le même après le premier et après le second.
  - `removing_the_first_of_three_writes_one_delete` : avec `[a, b, c]` dans `blocks`, retirer `a` augmente `len_ops` de **exactement 1**.
  - `reapply_after_a_move_across_slots_writes_nothing` : `apply(&r.deck().0)` laisse `len_ops` inchangé.
  - `reapply_after_concurrent_inserts_in_two_slots_writes_nothing` : sur `two_slots()`, A insère en tête de `elements` et B en tête de `blocks`. Après la fusion croisée, `apply(&deck())` sur chaque pair laisse `len_ops` inchangé, et un second échange ne change rien.
  - `concurrent_first_elements_on_a_new_slide_are_both_kept` : le deck initial porte une slide `content` vide, `s-neuve`. A ajoute `e1` et B ajoute `e2` dans `blocks`. Après la fusion croisée, les deux sont présents, dans le même ordre chez les deux pairs.
  - `removing_an_owner_removes_its_subtree` : `with_pile()` + deux enfants, puis on retire `pile`. Aucun nœud vivant ne porte `pile` ni ses enfants, et la relecture est égale.
  - `removing_an_owner_while_moving_its_child_out_keeps_the_child` : dans le même `apply`, `pile` est retiré et son enfant `k` passe dans `blocks`. `k` est relu dans `blocks` (temps 3 « différé »).
  - `slots_of_takes_declared_and_shaped_slots_sorted` : `slots_of` sur `{ "blocks": [élément], "title": "x", "zzz": [élément], "": [élément] }` avec le schéma de `content` rend `[("blocks", …), ("zzz", …)]`.
  - `read_element_without_owner_entry_joins_the_first_sibling_slot` : on forge sous la racine de `s-constat`, **avant** `b-points` dans l'ordre de l'arbre, un nœud `{id: "x", kind: "text", @slot: {}}` ; `n` est déjà dans `elements`. `x` est relu dans `blocks` (premier frère résolu dans l'ordre des enfants : `b-points`), avec une `issue-slot-repaired`.
  - `read_element_with_only_another_owner_entry_keeps_that_slot` : un nœud racine seul sur une slide vide, avec `@slot = {"pile": "children"}`, est relu dans `params.children` de la slide, avec une `issue-slot-repaired`.
  - `read_element_without_any_slot_goes_to_orphans` : un nœud racine seul, avec `@slot` absent ou une valeur non chaîne, est relu dans `params.orphans`, avec une `issue-slot-repaired`. Il **n'est pas écarté**.
  - `read_unknown_slot_name_is_kept_as_a_param` (garde, vert dès que la lecture de l'arbre existe) : avec `@slot = {"": "zzz"}`, `params.zzz` porte l'élément, et `validate` rend `issue-unknown-param`.
  - `read_slot_clashing_with_a_param_keeps_the_elements` : une valeur `"texte"` est écrite sous `params.blocks` de la carte de `s-constat`. À la relecture, `blocks` est le slot, et un `apply` du deck relu supprime la valeur résiduelle de la carte `params`.
  - `read_unreadable_element_drops_its_subtree_only` : un nœud forgé `{id: "x", kind: 42, @slot: {"": "blocks"}}` a un enfant valide. `x` et son enfant sont absents, `b-points` est présent, et une seule `Issue` est émise (`issue-unreadable-element`, élément `x`), aucune pour l'enfant.

  Aide de forge : `fn forge(r: &DeckReplica, slide: &str, parent: Option<&str>, meta: &[(&str, LoroValue)], slot: Option<&[(&str, &str)]>) -> TreeID`. Elle crée le nœud, écrit la méta, puis la carte `@slot` si `slot` est `Some`, et appelle `r.doc.commit()`. La relecture passe par `DeckReplica::from_snapshot(&r.snapshot()…)`. Pour placer le nœud avant un frère, utiliser `tree.mov_before`.

  Lancer : rouges par assertion, sauf les gardes nommées comme telles.
- [ ] **Step 4 : implémenter** `schema.rs`, `tree.rs`, `write.rs`, `read.rs` et `error.rs`, ainsi que la clé i18n, selon les interfaces et les règles. **Jalon** : lancer `-E 'test(tree_) | test(issue_0694) | test(concurrent_move_across_slots)'`. Ces tests doivent passer avant de migrer les aides (les autres tests du replica peuvent encore échouer à cause des aides).
- [ ] **Step 5 : migrer les aides de tests :**
  - `host_tests.rs` `host_map(r, id, param)` : `let (tree, n) = tree::node_of(&r.doc, "s", id);` puis `as_map(tree.get_meta(n)…)` → `params` → `param`. La slide de ces tests a l'id `"s"` (`Deck::from_json(r#"{"slides":[{"id":"s",…`).
  - `mod_tests.rs` `block_params(r, id)` : par `node_of(&r.doc, "s-constat", id)`.
  - `list_tests.rs` `items_list(r)` : par `node_of(&r.doc, "s-constat", "f")`.
  - `mod_tests.rs` `read_deeply_nested_slots_stops_at_the_depth_bound` : on forge la chaîne dans l'arbre. À partir du nœud de `pile-0`, 199 fois `tree.create(TreeParentId::Node(prev))`, avec la méta `id = pile-{i}`, `kind = "stack"` et `@slot = {"pile-{i-1}": "children"}`. Les assertions ne changent pas.
  - `read_tests.rs` : `put` traite le tableau `slides` par `fn put_slide(list: &LoroMovableList, i: usize, slide: &Map<String, Value>)`. Un `params` qui est un tableau non vide d'objets portant tous `kind` devient des nœuds de l'arbre `@elements` (`tree_at`), par `fn put_nodes(tree: &LoroTree, parent: TreeParentId, owner: &str, slot: &str, elements: &[Value])` : méta par `put`, `@slot = {owner: slot}`, et récursion sur les tableaux d'éléments de leurs `params`. Le reste se fait comme avant.
    - Test d'accord `put_slide_matches_the_writer` : pour `roadmap`, `read` de la forge et `DeckReplica::new(roadmap).deck()` sont égaux.
- [ ] **Step 6 : lancer** tout `oku-slides-core`, `tests/untrusted.rs` et `issue_0652` compris, puis la commande d'avant commit. Attendu : PASS, proptest de convergence (48 cas) compris. Si un test du replica échoue sur un écart de lecture, chercher d'abord une aide qui navigue encore dans `params.blocks`.
- [ ] **Step 7 : commit.**

  ```text
  feat(replica)! : les éléments d'une slide dans un LoroTree, un déplacement entre slots garde les éditions concurrentes (t-741)
  ```

---

## Task 3: doublons, profondeur et durcissement de l'arbre (t-742)

La Task 2 livre déjà le code de durcissement : `live_index` borné, `wanted_index` borné (`SlotsTooDeep`), `tree_at` → `ElementsNotATree`, nom vide jamais `Kind::Slot`, ordre `uorder`. Les tests ci-dessous sont donc des **gardes**, attendus verts. Si l'un est rouge, c'est un défaut de la Task 2 : le corriger dans `tree.rs` ou `schema.rs`, et dire dans le rapport ce qui manquait.

**Files :**
- Modify : `src/replica/tree_tests.rs` (ou `tree_read_tests.rs`) ; `src/replica/mod.rs` (doc de `guarded`, paragraphe « Reste hors garde ») ; `src/replica/tree.rs` et `src/replica/schema.rs` seulement si une garde est rouge.
- Create : `crates/oku-slides-core/tests/regressions/issue_0694_undo_remove_vs_peer_move_in_tree.rs`.

- [ ] **Step 1 : tests.**
  - `read_exact_copies_in_one_slot_keep_one` : on forge deux nœuds de méta égale `{id: "x", kind: "text"}` dans `blocks` de `s-constat`. Un seul `x` est relu, et `validate` ne rend pas `issue-duplicate-element-id`.
  - `read_copies_in_two_slots_rename_the_second` : la même méta dans `blocks`, et dans `elements` avec `frame`, donne `x` dans `blocks` et `x-2` dans `elements` (copie d'un autre slot renommée : `unique_ids` tient ses copies par slot).
  - `read_diverged_copies_rename_the_later_occurrence` : avec deux métas différentes dans `blocks`, la deuxième dans l'ordre des enfants devient `x-2`.
  - `duplicate_ids_round_trip_keeps_the_edited_node` : on forge `x` deux fois, la deuxième avec un texte différent. Après une relecture, un `apply` du deck relu et une nouvelle relecture, le deck est identique, et le texte de la deuxième occurrence est présent sous `x-2`. Cela vérifie que l'ordre d'occurrence est le même à l'écriture et dans `unique_ids`.
  - `apply_on_a_forged_deep_chain_leaves_unread_nodes_untouched` : on forge une chaîne de 200 `stack` (comme `read_deeply_nested_slots_stops_at_the_depth_bound`), puis on fait `apply(&r.deck().0)` dans un fil à pile de 256 Kio. Pas de débordement, pas d'erreur, et les nœuds de profondeur ≥ `MAX_SLOT_DEPTH` existent toujours (`!is_node_deleted`). Garde de la borne de `live_index`.
  - `apply_with_elements_of_another_type_fails_without_overwriting` : on remplace `@elements` de `s-constat` par une `LoroMap`, puis on fait un `apply` d'un titre changé. On obtient une erreur `ReplicaErrorKind::Corrupted` (`ElementsNotATree`), et la carte forgée est toujours là. À la lecture, une `issue-unreadable-element` est émise et aucun élément n'est relu pour cette slide.
  - `empty_param_name_with_elements_round_trips` : un élément porte `params: {"": [élément]}`. L'aller-retour est l'identité, sans nœud créé pour lui.
  - Régression `issue_0694_undo_remove_vs_peer_move_in_tree.rs`, sur le modèle de `issue_0652_undo_remove_vs_peer_move.rs` (`undo_vs_move`). A retire `b-points`, puis annule ; B le déplace vers `elements` et le met en gras ; ils échangent, réécrivent puis échangent encore. Les ids sont uniques, les deux pairs convergent, et un `b-points` (ou `b-points-2`) gras est présent. Un second cas fait de même avec `pile` et ses deux enfants (ids uniques à toutes les profondeurs).
  - Lancer : tous des gardes, attendus verts.
- [ ] **Step 2 : doc.** Dans la doc de `guarded` (`mod.rs`), ajouter : « les nœuds de l'arbre au-delà de `MAX_SLOT_DEPTH` ne sont ni lus ni écrits (`live_index`, `wanted_index`) ».
- [ ] **Step 3 : lancer** `-E 'test(tree_) | test(issue_0694) | test(issue_0652)'`, puis la commande d'avant commit.
- [ ] **Step 4 : commit.**

  ```text
  test(replica) : doublons d'ids et durcissement de l'arbre des éléments (t-742)
  ```

---

## Task 4: annuler, historique et aperçu ; parade t-263 limitée aux listes (t-743)

**Sonde d'abord** (R8e). Les tests du Step 1 couvrent des combinaisons que la sonde n'a pas mesurées : arbre dans une carte de slide, elle-même dans une `LoroMovableList`. Si l'un d'eux est rouge, **s'arrêter** et rapporter `BLOCKED` avec le cas : aucune parade improvisée. Les tests du Step 2 sont des gardes.

**Files :**
- Modify :
  - `src/replica/undo_tests.rs` (380 lignes ; ajout après `undo_redo_of_reordered_rows_and_columns_restores_exactly`) ;
  - `src/replica/history_tests.rs` (282 lignes) ;
  - `crates/oku-slides-core/tests/regressions/main.rs` ;
  - docs : `src/replica/undo.rs` (doc de module, l. 4-9) ; `src/replica/history.rs` (doc de module, l. 8-11, et commentaire l. 78) ; `src/replica/write.rs` (doc de `reorder`, si la Task 2 ne l'a pas faite).
- Create : `crates/oku-slides-core/tests/regressions/issue_0694_undo_tree_moves_keep_elements.rs`.

**Consommateurs recensés :** `split`/`PART` dans `mod.rs` (`apply`, `apply_silent`, `restore`, `part_message`), `history.rs` (`build`) et `write.rs` (`reorder`). Pas de changement de code : `Writer::split` reste pour `sync_list` (slides, items), et `sync_tree` ne l'appelle jamais. La doc de `restore` (`mod.rs` ~l. 378-388) parle déjà de la liste `slides` : on n'y touche pas.

- [ ] **Step 1 : sonde sur le vrai replica** (`undo_tests.rs`) :
  - `undo_redo_of_a_slide_removal_restores_its_element_tree` : on supprime `s-constat` (après y avoir mis une `pile` à deux enfants et un texte riche). `assert_undo_redo_exact`, et la carte relue porte un `@elements` de type arbre.
  - `undo_redo_of_removing_an_owner_with_children_restores_exactly` : on retire `pile` et ses enfants.
  - `undo_redo_of_a_slide_move_with_an_element_move_restores_exactly` : un seul `apply` déplace `s-constat` en tête (`mov` de liste, `split`) **et** passe `b-points` de `blocks` à `elements` (`mov` d'arbre).
- [ ] **Step 2 : gardes.**
  - `undo_tests.rs` (deck `two_slots` reconstruit localement) :
    - `undo_redo_of_a_move_across_slots_restores_exactly` : `assert_undo_redo_exact(&before, &after)` ;
    - `undo_redo_of_a_move_into_a_child_slot_restores_exactly` ;
    - `undo_redo_of_a_full_element_reversal_restores_exactly` : 8 textes dans `blocks`, dans l'ordre inverse (le cas t-263, pour les éléments) ;
    - `undo_move_across_slots_keeps_a_remote_edit` : A déplace `b-points` vers `elements` et B le met en gras. A fusionne B puis annule : `b-points` revient dans `blocks`, **gras**. A rétablit : `b-points` est dans `elements`, gras. B fusionne A : les decks sont égaux ;
    - `undo_a_stale_move_after_the_peer_moved_it_again_converges` : A déplace `b-points` vers `elements`, B le met dans `pile.children`. A fusionne puis annule, et les deux pairs échangent : convergence, `b-points` présent une fois, dans un slot déclaré par son parent réel.
  - Régression `issue_0694_undo_tree_moves_keep_elements.rs`, sur le modèle de `issue_0263_undo_reorder_loses_slides.rs` (même `Lcg`, graines `[1, 7, 42, 2026]`, 6 tours) :
    - point de départ : `roadmap` + `pile` dans `blocks` de `s-constat` + `n` dans `elements` ;
    - une édition au hasard, sur `s-constat` : permutation des éléments de chaque slot, passage d'un élément d'un slot à l'autre (cadre ajouté ou retiré), imbrication d'un texte dans `pile.children` ou sortie, suppression d'un élément non `pile` et ajout d'une copie au nouvel id `e-copie-{round}` ;
    - on annule tout, puis on rétablit tout ; chaque état relu est égal à l'état appliqué, ids conservés, aucune `Issue`. Si ce test est rouge, la parade t-263 doit revenir pour l'arbre : `BLOCKED`.
  - `history_tests.rs` :
    - `a_move_across_slots_is_one_version` : `history()` gagne exactement une version, et `len_changes` augmente de 1 ;
    - `deck_at_before_a_move_across_slots_shows_the_old_slot` ;
    - `restore_across_a_move_across_slots_brings_back_the_old_slot` : `restore_and_undo(&mut b, &h, 0)`.
- [ ] **Step 3 : docs.**
  - `undo.rs` : « Loro 1.16.2 défait mal un item qui porte plusieurs `mov` sur une `LoroMovableList` de conteneurs (t-263) : la réconciliation committe donc avant chaque `mov` des slides et des items de liste. L'arbre des éléments d'une slide n'en a pas besoin (ADR-0005). »
  - `history.rs` : « une édition qui déplace des slides ou des items de liste fait un commit par `mov` ».
- [ ] **Step 4 : lancer** `-E 'test(undo_) | test(history) | test(restore) | test(deck_at) | test(issue_0694)'`, puis la commande d'avant commit.
- [ ] **Step 5 : commit.**

  ```text
  test(replica) : annuler, historique et aperçu d'un déplacement d'élément dans l'arbre ; t-263 limitée aux listes (t-743)
  ```

---

## Task 5: format v8 des sources et marqueur de format du replica (t-744)

Sans marqueur, un `deck.loro` v7 relu hors zip se lirait **sans aucun élément**, sans erreur, et la prochaine écriture effacerait ses slots. C'est le cas d'une reprise de session : `crates/oku-slides-session/src/resume.rs` `read_doc`, l. 434-449, appelle `DeckReplica::from_snapshot`. Avec le marqueur, ce fichier est refusé, puis `Session::resume` (l. 555-575) émet `warn!("reprise impossible")` avec le texte de l'erreur (« unsupported replica format: none ») et met le dossier de côté par `quarantine` (`.failed-<secondes>`).

**Files :**
- Modify :
  - `crates/oku-slides-core/src/replica/mod.rs` (`create`, `import_snapshot`, `merge_inner`, constantes) ;
  - `crates/oku-slides-core/src/error.rs` (`UnsupportedReplicaFormat`, son kind, et la doc de `ReplicaErrorKind::UnsupportedVersion` : « Deck d'une autre version (`DECK_VERSION`), ou replica d'un autre format (`meta.format`). ») ;
  - `crates/oku-slides-core/src/sources.rs` (`SOURCES_VERSION = 8` ; doc de module : l. 1 « version 6 » → « version 8 », et « Un zip d'une version antérieure (1 : `deck.json` ; 2 à 5) » → « (1 : `deck.json` ; 2 à 7) ») ;
  - `crates/oku-slides-core/src/sources_tests.rs` (`MANIFEST` l. 27 en version 8 ; nouveau test ; doc de `sources_v6_zip_is_refused` : « Sources v6 : refusées. ») ;
  - `crates/oku-slides-core/tests/zip_limits.rs` (l. 56 : version 8) ;
  - `crates/oku-slides-core/src/replica/tree_tests.rs` (tests du marqueur) ;
  - `crates/oku-slides-core/ARCHITECTURE.md` l. 27 : « Zip v7 » → « Zip v8 », remplacement sur place, 50 lignes au plus ;
  - `crates/oku-slides-pdf/ARCHITECTURE.md` l. 22 (« Format du zip (version 5) ») et l. 25 (`version: 5`) → 8 ;
  - `crates/oku-slides-pdf/src/lib.rs` l. 18 et l. 21 → 8 ;
  - `docs/aide/fr.json` et `docs/aide/en.json`, l. 2136 (titre « … (version 5) »), l. 2147 (`version: 5`) et l. 2418 (« zip version 5 ») → 8. **Ne pas toucher** l. 2287 (« Le deck JSON (version 5) », c'est `DECK_VERSION`), ni le texte « FUSION SANS CONFLIT », que cherche `crates/oku-slides-session/src/help_tests.rs:301` ;
  - `crates/oku-slides-session/Cargo.toml` : `loro.workspace = true` en `[dev-dependencies]` (dépendance déjà dans le workspace ; nécessaire pour forger un replica v7) ;
  - `crates/oku-slides-session/src/resume_tests.rs` ;
  - `crates/oku-slides-session/tests/regressions/main.rs`.
- Create : `crates/oku-slides-session/tests/regressions/issue_0694_reprise_v7_mise_de_cote.rs`.

**Consommateurs recensés :**
- `SOURCES_VERSION` : `sources.rs` l. 46, 450, 575-576.
- Manifestes en dur : `sources_tests.rs` l. 27 (version courante) et l. 72, 85, 98, 110 (versions refusées, inchangées) ; `zip_limits.rs` l. 56.
- `tests/untrusted.rs` l. 69-95 : le contrôle `deck_uid` reste **avant** le contrôle du format, si bien qu'une `meta` tronquée donne toujours `Corrupted`. À relancer.

**Interfaces :**
```rust
// replica/mod.rs
/// Clé de `meta` : format du replica.
const FORMAT: &str = "format";
/// 2 : éléments d'une slide en `LoroTree` (ADR-0005). Le changer impose de changer `SOURCES_VERSION`.
const REPLICA_FORMAT: i64 = 2;
/// `meta.format`, s'il est un entier (`LoroValue::I64` seulement).
fn format_of(doc: &LoroDoc) -> Option<i64>;
/// Refuse un document d'un autre format.
fn check_format(doc: &LoroDoc) -> Result<(), ReplicaInner>; // UnsupportedReplicaFormat { found: "none" | "<n>" }
```
- `create` : `doc.get_map(META).insert(FORMAT, REPLICA_FORMAT)` juste après `DECK_UID`, dans le même commit initial.
- `import_snapshot`, dans `guarded`, après le contrôle `deck_uid` et **avant** `read_deck` : `check_format(doc)?`.
- `merge_inner`, dans la garde d'essai, après `doc.import(&updates)` et avant `read_deck` : `check_format(doc)?` (R7a). Un pair qui réécrit le format est refusé, et `self` reste intact.

- [ ] **Step 1 : tests rouges.**
  - `sources_tests.rs` : `sources_v7_zip_is_refused`, sur le modèle de `sources_v6_zip_is_refused` : le manifeste `{"format":"oku-slides-sources","version":7}` donne `SourcesErrorKind::NotSources`.
  - `tree_tests.rs` :
    - `new_replica_carries_the_replica_format` : `meta.format == 2` ;
    - `from_snapshot_without_replica_format_is_refused` : un replica neuf, puis `r.doc.get_map("meta").delete("format")`, un commit et un snapshot. `from_snapshot` échoue avec le kind `UnsupportedVersion` ;
    - `from_snapshot_of_another_replica_format_is_refused` : `format = 3`, même résultat ;
    - `from_snapshot_of_a_v7_shaped_replica_is_refused` : un `LoroDoc` construit à la main au format v7 : `meta.deck_uid`, pas de `format`, `deck.slides` en `LoroMovableList`, et une slide dont `params.blocks` est une `LoroMovableList` d'éléments. `from_snapshot` donne `UnsupportedVersion` ;
    - `merge_of_a_peer_that_rewrites_the_format_is_refused` : deux pairs ; B écrit `meta.format = 3` et committe. `a.merge(&b)` échoue avec `UnsupportedVersion`, et `a` garde son deck et `meta.format == 2`.
  - `resume_tests.rs` : `read_doc_of_a_replica_without_format_returns_unsupported_version`. Le snapshot est écrit comme dans `read_doc_with_corrupt_deck_loro_returns_error` ; on remplace ensuite `deck.loro` par le même snapshot sans `meta.format`, forgé avec `loro::LoroDoc` (import, `delete("format")`, commit, `export(ExportMode::Snapshot)`). Le résultat est `Err`, et son texte contient « unsupported replica format ».
  - Régression session `issue_0694_reprise_v7_mise_de_cote.rs`, sur le modèle de `resume_with_unreadable_state_moves_old_data_aside_instead_of_overwriting` (`tests/session.rs:601`), avec les aides de `tests/common` (`data_dir`, `session_in`) :
    - une session écrit une reprise, avec une édition puis `flush` (même chemin que `issue_0224_agent_edits_resume.rs`) ;
    - le `deck.loro` de la reprise est réécrit sans `meta.format` ;
    - une nouvelle session : `resume(dir)` vaut `false`, il existe un seul dossier `resume.failed-*` dont le `deck/deck.loro` est égal aux octets forgés, et rien n'est écrit dans `resume`.
  - Lancer : rouges.
- [ ] **Step 2 : implémenter** ; passer `MANIFEST` et `zip_limits.rs` en version 8 ; faire toutes les mises à jour de doc de la liste **Files**.
- [ ] **Step 3 : lancer** la commande d'avant commit, sur tout le workspace : `oku-slides-session`, `oku-slides-cli` et `oku-slides-render` relisent des zips qu'ils écrivent eux-mêmes, et `help_tests` vérifie l'aide.
- [ ] **Step 4 : commit.**

  ```text
  feat(sources)! : sources en v8 et format du replica marqué, l'ancien replica est refusé à l'import comme à la fusion (t-744)
  ```

---

## Task 6: convergence des déplacements dans l'arbre (t-745)

Ces tests sont des gardes du comportement livré par les Tasks 2 et 3 : le rouge de t-694 a été vu en Task 2 avec la régression. Ils sont attendus verts. S'ils révèlent une divergence, s'arrêter, réduire le cas et rapporter `BLOCKED` avec le cas minimal. On ne restreint jamais une stratégie pour faire passer un test.

**Files :**
- Create : `crates/oku-slides-core/tests/tree_convergence.rs` (nouveaux tests déterministes, avec leurs aides : `convergence.rs` fait 667 lignes).
- Modify :
  - `crates/oku-slides-core/tests/convergence.rs` (proptest seulement) ;
  - `crates/oku-slides-core/src/replica/tree_tests.rs` (cycle déterministe : il faut fixer les pairs, ce qu'on ne peut faire que dans la crate).

**Aides de `tree_convergence.rs`** (copiées et adaptées de `convergence.rs` : `op`, `ok`, `with_ops`, `peers`) :
- `fn with_pile(deck: Deck, id: &str) -> Deck` : ajoute un `stack` d'id `id` à la fin de `blocks` de `s-constat` (op `add_element`, `common::registry()`) ;
- `fn nest(deck: &Deck, id: &str, owner: &str) -> Vec<Op>` : `remove_element` de `id`, puis `add_element` de sa copie (prise dans `deck`, sans `frame` ni `cell`) dans le slot `children` de `owner` ;
- `fn located<'a>(deck: &'a Deck, id: &str) -> Vec<Located<'a>>` : les occurrences de `id` dans `s-constat` (`slide.walk()`) ;
- `fn first_run_bold(e: &Element) -> bool`.

- [ ] **Step 1 : tests.**
  - `tree_convergence.rs` :
    - `concurrent_move_into_a_child_slot_and_edit_converge` : sur `with_pile(roadmap, "pile")`, A fait `nest(…, "b-points", "pile")` et B met `b-points` en gras (visé par id ; on vérifie le basculement chez B). Après la fusion croisée : mêmes decks, aucune `Issue` de lecture ; `b-points` présent une fois, avec `owner == Some("pile")`, `slot == "children"` et `first_run_bold`.
    - `concurrent_moves_of_one_element_to_two_places_agree` : A passe `b-points` dans `elements` (cadré), B fait `nest(…, "b-points", "pile")`. Après la fusion : mêmes decks, `b-points` une fois, et le couple est cohérent : `(None, "elements")` ou `(Some("pile"), "children")`. `validate` ne rend pas `issue-unknown-param`.
    - `concurrent_nest_and_reorder_keep_the_element_in_a_declared_slot` (R1) : le deck de départ est `with_pile(roadmap, "pile")`, plus un élément `{id: "x", kind: "text", params: {body: [{runs: [{t: "Texte x"}]}]}}` ajouté à la fin de `blocks` de `s-constat` (op `add_element`). Sur `[b-points, pile, x]` dans `blocks`, A fait `nest(…, "x", "pile")` et B fait passer `x` en tête de `blocks`. Quel que soit le gagnant, `x` est présent une fois, dans `children` s'il est sous `pile`, sinon dans `blocks`. Pas d'`issue-unknown-param`.
    - `move_under_a_concurrently_removed_owner_is_a_documented_loss` : A retire `pile` ; B fait `nest(…, "b-points", "pile")`. Après la fusion : mêmes decks, aucune `Issue` de lecture, ni `pile` ni `b-points`. Puis chaque pair fait un `apply` du deck relu et ils échangent encore : rien ne change, pas d'erreur, pas de panique. Commentaire : « Comportement documenté (ADR-0005, Limites ; sonde, point 3) ».
  - `tree_tests.rs`, `concurrent_crossed_nesting_moves_converge_without_cycle` (R8j) :
    - deux pairs issus d'un même snapshot, `a.doc.set_peer_id(1)` et `b.doc.set_peer_id(2)` ;
    - deck avec `pile-a` et `pile-b` dans `blocks` de `s-constat`, `pile-a` avant `pile-b` ;
    - A met `pile-b` dans `pile-a.children`, B met `pile-a` dans `pile-b.children`, par deux `apply` de même forme ;
    - attendu : le `mov` de A (pair 1, même lamport) gagne. `pile-b` est dans `pile-a.children`, et `pile-a` reste **visible** à sa place, dans `blocks`, à son index d'origine, grâce à son entrée `@slot[""]` intacte. Mêmes decks, aucune `Issue` de lecture ni de `validate` ;
    - puis `apply(deck())` sur chaque pair laisse `len_ops` inchangé ;
    - si les lamports diffèrent malgré tout (forme des `apply`), écrire l'attendu selon la règle (premier `mov` dans l'ordre (lamport, pair)), lu sur les changes, et le dire dans le rapport.
  - `convergence.rs`, proptest `peers_editing_in_parallel_converge` (R8a) :
    - le deck de départ gagne deux `stack`, `pile-1` et `pile-2`, dans `blocks` de `s-constat` (`with_pile`) ;
    - nouvelles éditions dans `Edit` et dans `edit()` :
      - `Edit::Nest(usize, usize, usize)` : sur `s-constat`, déplace l'élément `a` (modulo, toutes profondeurs, par `walk`) dans `children` de la pile `b` (modulo les `stack` de la slide). L'édition n'a pas lieu si l'élément est la pile elle-même ou l'un de ses ancêtres (cycle local), ou s'il n'y a pas de pile. Les piles peuvent s'imbriquer : des cycles concurrents sont alors possibles ;
      - `Edit::RemoveOwner(usize)` : retire la pile `i` (modulo), avec ses enfants ; rien s'il n'y a pas de pile.
    - aides récursives `detach(params: &mut Params, id: &str) -> Option<Element>` et `attach(params: &mut Params, owner: &str, el: Element) -> bool`, sur `slots_mut` (profondeur bornée par le deck) ;
    - nouvel invariant après la convergence (R2, F1) : pour chaque pair, un premier `apply(&first, "relire", None)`, qui peut écrire une réparation t-652 ; puis un **second** `apply(&first, "relire", None)` qui laisse `len_ops` inchangé (`prop_assert_eq!`). C'est l'exception de la règle « Invariant » ;
    - l'assertion existante `issues.is_empty()` (« lecture partielle », `convergence.rs` l. 316) ignore `issue-slot-repaired`. Ce n'est pas une perte, comme dans `files.rs::is_unreadable` ; une annulation qui défait une entrée `@slot` face à un `mov` concurrent peut la produire. Filtrer par `i.message.id() != "issue-slot-repaired"` ;
    - passer temporairement `with_cases(48)` à `with_cases(1000)` et lancer une fois `-E 'test(peers_editing_in_parallel_converge)'`. Noter le résultat dans le rapport, mettre tout cas minimal trouvé en test fixe dans `tree_convergence.rs`, puis **revenir à 48** avant de committer.
- [ ] **Step 2 : lancer** `-E 'test(concurrent_) | test(move_under) | test(peers_editing)'`, puis la commande d'avant commit.
- [ ] **Step 3 : commit.**

  ```text
  test(replica) : convergence des déplacements imbriqués, croisés et concurrents, et relecture sans écriture au proptest (t-745)
  ```

---

## Risques et parades

| Risque | Parade |
|---|---|
| Un `deck.loro` v7 relu hors zip (reprise) se lit sans éléments, puis la prochaine écriture efface ses slots. | Marqueur `meta.format = 2`, vérifié à l'import et à la fusion (Task 5) ; la reprise est mise de côté (`.failed-*`), testée côté session. Le zip est gardé par `SOURCES_VERSION = 8`. |
| Un pair réécrit `meta.format` et rend le replica local illisible à la reprise suivante. | Contrôle dans la garde d'essai de `merge` : la fusion est refusée et `self` reste intact (Task 5). |
| Le `mov` et le slot sont deux registres : un `mov` concurrent peut placer un nœud chez un parent dont le slot n'est pas inscrit. | `@slot` est une carte par parent : seule l'entrée du parent réel compte, les anciennes servent de repli. Pas d'appariement à garantir (R1). Tests : `concurrent_moves_of_one_element_to_two_places_agree`, `concurrent_nest_and_reorder…`, cycle. |
| `mov` parasites : une suppression ou un entrelacement après fusion ferait déplacer des frères, et pourrait annuler un déplacement concurrent. | Suppressions d'abord ; ordre comparé par (parent, slot) entre les seuls nœuds voulus ; nœuds d'une plus longue sous-suite croissante (LIS) laissés en place, les autres placés par `mov_after`/`mov_before` (R2, F2). Tests : un retrait = une op ; premier → dernier = un `mov` ; `apply(deck())` après fusion n'écrit rien ; au proptest, le second `apply` n'écrit rien (F1). |
| Un `LoroTree` **enfant d'une carte de slide**, elle-même dans une `LoroMovableList`, se comporte autrement que l'arbre racine de la sonde. | Tests sur le vrai replica (Tasks 2 et 4), sonde d'abord en Task 4 (suppression de slide, propriétaire à enfants, `mov` de slide et d'arbre mêlés) ; `BLOCKED` si rouge. |
| La parade t-263 est retirée pour l'arbre sans preuve suffisante. | Régression pseudo-aléatoire `issue_0694_undo_tree_moves_keep_elements` (Task 4) ; si elle est rouge, la parade revient, `BLOCKED`. |
| Déplacement sous un élément supprimé en parallèle : l'élément disparaît. | Les slots ne sont jamais des nœuds, et les racines sont les éléments de la slide : seuls les éléments porteurs de slots (`stack`, `canvas`) sont exposés. C'est documenté (ADR-0005) et testé, avec une seconde écriture qui reste stable. |
| Perdant d'un cycle invisible. | L'entrée `@slot` de l'ancien parent est intacte, donc le perdant reste visible à sa place (test déterministe à pairs fixés). |
| Défaire une suppression recrée un nœud : doublon d'id. | Identité = `id` de la méta ; `unique_ids` (t-652) sur tout l'arbre ; tests séparés « même slot » et « deux slots » ; régression `issue_0694_undo_remove_vs_peer_move_in_tree`. |
| Écriture hors garde sur des nœuds jamais lus (forge profonde), ou coût quadratique des suppressions. | L'écriture est bornée à `MAX_SLOT_DEPTH` comme la lecture. Les suppressions sautent le sous-arbre supprimé, sans `is_node_deleted` sauf pour les différés. Test hostile de 200 niveaux, puis `apply` sous une pile de 256 Kio (Task 3). |
| `@elements` forgé d'un autre type, écrasé en silence. | Erreur `ElementsNotATree` à l'écriture, `Issue` à la lecture (Task 3). |
| Un nom de paramètre vide classé en slot casse l'aller-retour. | Jamais `Kind::Slot` pour un nom vide (Task 2) ; test d'aller-retour (Task 3). |
| Un slot qui masque un paramètre de même nom. | Le slot l'emporte à la lecture, et l'écriture suivante supprime la valeur résiduelle (test). Un pair a déjà le pouvoir d'éditer ce paramètre : pas d'`Issue` dédiée. |
| Invariant de placement cassé (propriétaire non placé). | Erreur `TreeOrder`, jamais de repli silencieux vers la racine. |
| Appariement des ids en double différent entre lecture et écriture : le nœud d'un pair perd son identité. | `current` et `wanted` sont construits dans l'ordre `uorder` de `unique_params`, pas en préordre (F3) ; tests `nested_duplicate_keeps_the_outer_tree_id` et `duplicate_ids_round_trip_keeps_the_edited_node`. |
| Un `Deck` JSON plus profond que la borne recréerait ses nœuds profonds à chaque `apply`. | `wanted_index` refuse par `SlotsTooDeep` avant toute écriture dans la slide (F4) ; test `apply_of_a_too_deep_deck_twice_does_not_grow`. |
| `Rearrange` implicite de Loro (jitter 0) : des `mov` de frères non demandés. | Accepté et documenté (ADR-0005, Limites) ; Loro figé par `Cargo.lock`. |
| Avalanche d'`Issue` sur un replica forgé large. | Une `Issue` par nœud réparé ou illisible, comme aujourd'hui pour les éléments ; `issue-slot-too-deep` une par nœud. La taille est bornée par `max_replica_bytes`. |
| Commits intermédiaires incohérents (arbre sous `SOURCES_VERSION` 7). | Squash obligatoire sur `main`, avec un pied `BREAKING CHANGE` (R5, R12). |
| Dossier cargo partagé. | Une seule compilation à la fois ; un conflit ne cause qu'un verrou et de la lenteur (R13). |
| Fichiers trop longs. | Nouveaux tests dans `tree_tests.rs`, `tree_read_tests.rs` et `tests/tree_convergence.rs` ; `mod_tests.rs` (824 lignes) ne grandit pas ; ARCHITECTURE donnée en entier et comptée avec `wc -l`. |
| Proptest qui diverge. | `BLOCKED` avec le cas minimal, mis en test fixe ; jamais de stratégie restreinte. |
