# Deck en un seul `LoroTree`, suppression douce, `move_element`, `Ids::fresh` linéaire (t-693, t-746, t-747) — plan d'implémentation

Plan revu par l'équipe (14 relecteurs), rulings R1 à R13 intégrés.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :**
- déplacer un élément d'une slide à une autre garde l'édition qu'un pair fait en même temps sur cet élément (t-746). Le déplacement vient de l'op `move_element`, que l'agent appelle et que l'UI emploie pour un couper-coller ;
- annuler une suppression rend le **même** nœud, avec les déplacements et les éditions qu'un pair y a faits entre-temps (t-747) ;
- la réparation des ids en double (`Ids::fresh`) devient linéaire (t-693).

**Architecture :**
- Aujourd'hui (ADR-0005), chaque slide porte son propre `LoroTree` sous `@elements`, et les slides sont une `LoroMovableList` :
  - passer un élément d'une slide à l'autre est une copie suivie d'une suppression, et l'édition concurrente est perdue. Aucun geste ne le fait d'ailleurs en un seul `apply` : l'UI coupe (`remove_element`, un `apply`) puis colle une copie aux ids renouvelés (`ui/src/App.tsx` l. 213-232, `ui/src/deck.ts` l. 242-249) ;
  - retirer un élément appelle `tree.delete`. Loro ne ressuscite jamais un nœud supprimé : défaire le retrait recrée une copie, et le `mov` ou l'édition concurrente d'un pair est perdue ;
  - retirer une slide la supprime de la liste. Défaire la recrée, et un pair qui la déplaçait garde l'original : deux slides (`s-toc`, `s-toc-2`, t-652).
- Désormais, **un seul `LoroTree`** porte tout le deck, sous la clé `slides` de la carte `deck` :
  - les racines sont les slides, et **seulement** elles. La méta d'une racine est la carte de la slide (sans ses slots), plus `@slide = true`. L'ordre des slides est l'index fractionnaire de Loro ;
  - les éléments sont les descendants d'une slide, comme dans ADR-0005. À **tous** les niveaux, la clé de `@slot` est le `TreeID` du parent, écrit `counter@peer` (`TreeID::to_string`) et relu par `TreeID::try_from` ;
  - un réordonnancement des slides et un déplacement d'élément d'une slide à l'autre sont des `mov` ;
  - la parade t-263 (un commit par `mov`) ne vaut plus que pour les `LoroMovableList` restantes : items de liste, colonnes et lignes d'un `dataset`.
- **Suppression douce.** Retirer un nœud (slide ou élément) n'appelle plus `tree.delete`. Le nœud reçoit une marque `@deleted/<peer id>` = `true` dans sa méta :
  - une clé par pair, à plat. Le nœud est supprimé tant qu'il en garde au moins une ;
  - la lecture ignore un nœud marqué et tout son sous-arbre ;
  - l'annulation retire la seule clé du pair qui annule : c'est le même nœud qui revient, et la suppression concurrente d'un autre pair tient ;
  - la réconciliation ne retire jamais une marque. Un id réajouté crée un nœud neuf : on ne ranime jamais un nœud par son id.
- **Entrée stricte.** À l'import et à la fusion, `check_trees` refuse (`Corrupted`) un arbre de deck mal formé :
  - un `@slide` hors racine ;
  - une clé `@deleted…` qui n'est pas `@deleted/<u64>` = `true` ;
  - un `@slot` qui n'est pas une carte fusionnable.

  Ce contrôle ne garde que ce que le code, annuler et rétablir compris, ne produit jamais. Une racine sans `@slide`, que Loro crée lui-même dans un cas d'annulation (R0), est acceptée et ignorée à la lecture.
- **`move_element`** : une op cœur (slide et élément source, slot et position cibles, même slide admise). `apply_all_moving` rend la liste des déplacements, que la session passe à `DeckReplica::apply_moving` : le nœud déplacé est apparié à coup sûr, même si son id existe dans d'autres slides.
- Le JSON du deck ne change pas (`DECK_VERSION` = 5). `REPLICA_FORMAT` passe de 2 à 3 et `SOURCES_VERSION` de 8 à 9, sans compatibilité.
- `Ids::fresh` (`canonical.rs`) mémorise le prochain `n` à essayer pour chaque id de base. Le résultat ne change pas.

**Tech Stack :** Rust, Loro 1.16.2 (`LoroTree`, `LoroMap`, `LoroMovableList`, `UndoManager`, `ensure_mergeable_map`, `ContainerID::is_mergeable`), serde_json, snafu, schemars, speculoos, proptest ; UI : React, TypeScript, Vitest (`bun run test`).

**Spec :** design approuvé le 2026-10-10 ; portée élargie par l'utilisateur à l'op `move_element` et au couper-coller (revue du plan). Sonde : `crates/oku-slides-core/tests/loro_deck_tree_probe.rs` (commit `dfb88930`, 14 tests verts). Verdicts : suppression douce GO AVEC PARADES (une marque par pair, propagation au sous-arbre à la lecture) ; arbre de deck GO. Les points de la sonde sont cités par le nom de leur test (`p1_…` à `p9_…`). Tâches Oku : voir l'en-tête de chaque tâche ; compaction des nœuds marqués : t-770.

## Global Constraints

- **Lieu.** Tout se fait dans le worktree `/Users/matthieu/Projects/slider/.worktrees/746-deck-en-arbre`. Jamais de `cd` vers le dépôt principal.
- **MVP.** Aucune rétrocompatibilité, aucun chemin de lecture de l'ancien format (arbre par slide, slides en `LoroMovableList`).
- **TDD.**
  - Le test rouge est écrit et **vu échouer** avant le code, puis il passe au vert. Il échoue par une assertion, ou par le `panic!` explicite d'une aide de navigation (« arbre du deck attendu »). Jamais par une erreur de compilation : la cible de test de la lib compile à **chaque** étape.
  - Corps provisoires plutôt que `todo!()` : par exemple un `sync_deck` qui n'écrit rien, ou un `read_deck_tree` qui rend `vec![]`.
  - Un test vert d'emblée est un test de garde : le rapport de la tâche le nomme comme tel.
  - Avant chaque `git add`, `grep -rn "todo!\|unimplemented!" crates/` doit être vide.
- **Régressions.** Elles vont dans `crates/<crate>/tests/regressions/issue_<tâche sur 4 chiffres>_<sujet>.rs`, déclarées dans le `main.rs` du dossier, **dans l'ordre numérique** : un `issue_0746_…` ajouté plus tard se range avant `issue_0747_…`.
- **Commits.**
  - Conventionnels, en français, avec la portée et la référence (t-693, t-746 ou t-747). **Pas de `!`** dans le sujet des commits intermédiaires : la rupture n'est déclarée que par le squash. **Jamais de ligne `Co-Authored-By`.**
  - Le hook `oku.review=deferred` est posé pour ce worktree (`git config --worktree --get oku.review`). Chaque commande seule, dans cet ordre :
    1. `git add <fichiers>` ;
    2. `git diff --cached --name-only` (aucun fichier étranger) ;
    3. message écrit dans un fichier hors du dépôt (outil Write, dossier scratchpad), puis `git commit -F <fichier>`.
- **Fin de branche.** La branche est **squashée** sur `main`, c'est obligatoire : les commits intermédiaires (arbre de deck écrit sous `REPLICA_FORMAT` 2 et `SOURCES_VERSION` 8 jusqu'à la Task 8) ne doivent pas arriver sur `main`. Le pied `BREAKING CHANGE:` porte sur le message du squash, et sur lui seul :
  ```text
  BREAKING CHANGE: sources en v9 (deck en un seul LoroTree, suppression douce, meta.format = 3) ; un zip v8 est refusé, une reprise de session v8 est mise de côté (.failed-*). Couper un élément ne le retire plus : le coller le déplace (op move_element).
  ```
- **Cargo** (décision de l'utilisateur). On garde le dossier partagé : chaque commande est préfixée par `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target`, avec une seule compilation à la fois. Si un résultat de test paraît incohérent, relancer la suite en séquence avant de conclure. Les tests tournent avec des dossiers de données temporaires :
  - ciblé : `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run -p oku-slides-core -E 'test(<motif>)'` ;
  - avant chaque commit : `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run --workspace --exclude oku-slides-app` ;
  - `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy --workspace --all-targets -- -D warnings` ;
  - `cargo fmt --check`.
- **Tests instables connus, hors tâche.** `oku-slides-agent` `stack::a_prompt_queued_before_tab_closed_never_reaches_the_agent` et `oku-slides-cli` `issue_0181_serve_obeys_foreign_origins::serve_refuses_requests_from_another_origin`. S'ils échouent, les relancer seuls. S'ils passent seuls, les citer dans le rapport sans y toucher.
- **UI.** Seule la Task 7 touche `ui/`. Elle lance `cd ui && bun run test && bunx --bun tsc -b`. Une autre tâche qui toucherait `ui/` le lance aussi.
- **i18n.** Aucune clé nouvelle. La Task 7 réutilise `ui-undo-move` (« Déplacer »), qui existait déjà dans les 20 `crates/oku-slides-i18n/src/locales/*/ui.ftl`, et retire `ui-undo-cut`, qui n'est plus référencée. Aucune autre : `ReplicaError::localize` passe par le kind, et `move_element` réutilise les erreurs d'op existantes.
- **Taille.**
  - Fichiers ≤ 1000 lignes. `replica/tree_tests.rs` (818 lignes) et `replica/mod_tests.rs` (812) ne reçoivent **aucun** test nouveau : ils ne font que migrer.
  - Les nouveaux tests vont dans `replica/soft_delete_tests.rs` (Task 3, déclaré dans `tree.rs`, à côté de `tree_read_tests`) et `replica/tree/deck_tree_tests.rs` (Task 4, déclaré dans `tree/deck.rs` par `#[path = "deck_tree_tests.rs"]`, relatif au dossier `tree/`). Les deux sont des descendants du module `tree` : ils voient les aides `pub(super)` de `tree_tests`, et `deck_tree_tests` voit en plus les items privés de `tree::deck` (`pair`, `Pairing`, `live_slides`, champs de `WantedSlide`).
  - `tests/convergence.rs` (794 lignes) ne reçoit que le proptest. Les tests déterministes vont dans `tests/tree_convergence.rs` (197).
  - `tests/ops.rs` (896) ne reçoit rien : les tests de `move_element` vont dans `tests/move_element.rs`.
  - `src/ops/mod.rs` (792) : `move_element` va dans `src/ops/moving.rs`.
  - `replica/tree.rs` (472) : si `wc -l` dépasse 500 à la fin de la Task 4, `lis`, `arrange` et `place` sortent dans `replica/tree/place.rs`.
  - `replica/ARCHITECTURE.md` et `crates/oku-slides-core/ARCHITECTURE.md` : 50 lignes au plus, vérifiées par `wc -l`.
- **Code.**
  - Pas d'`unwrap`/`expect` en production, et pas de repli silencieux qui masquerait un bogue.
  - Pas d'indexation directe sur des longueurs venues du replica (`pair`, `sync_deck`, `placed`) : `.get()`, puis `TreeOrderSnafu` si l'invariant casse.
  - Erreurs Snafu : `LoroSnafu` pour toute `LoroError`. Les variantes sont dans `crates/oku-slides-core/src/error/mod.rs` (et non `error.rs`).
  - Aucune récursion non bornée sur un replica non fiable : lecture, écriture et contrôle d'entrée sont itératifs. Les éléments sont bornés par `MAX_SLOT_DEPTH` (16).
  - `LoroMap::get_or_create_container` est déprécié en 1.16.2 : jamais utilisé. `@slot` est créé par `ensure_mergeable_map` (Task 3).
  - Aides de test : `unwrap_or_else(|e| panic!(…))`. Les aides qui vérifient un état (`visible_ids`, `slide_ids`, `marks`) lisent l'arbre **brut** (`tree.roots()`, `tree.children`, `get_meta`, clés de méta), jamais par `ordered_children` ni `slide_order` : un oracle ne réutilise pas le code qu'il vérifie. Leurs parcours sont itératifs.
- **Doc.** La première phrase d'une doc fait moins de 15 mots. Les tests sont nommés `<unité>_<scénario>_<résultat>`. Aucun document versionné ne renvoie vers le dossier de travail non versionné. Tout numéro de ligne cité dans un commentaire ou une doc est vérifié au moment de l'écrire.

## Leçons de t-694 à garder

Elles sont dans le code actuel ; aucune tâche ne doit les perdre.
- **Loro saute un `insert` de même valeur.** `set_value` (`write.rs` l. 53-58) ne réécrit pas une valeur égale. `mark` n'écrit que sur un nœud visible, donc jamais sur sa propre marque : un `insert` sauté laisserait l'annulation sans rien à défaire.
- **Placement minimal par LIS** (`lis`, `place`, `tree.rs` l. 108-131 et 363-427) : les nœuds d'une plus longue sous-suite déjà ordonnée ne bougent pas. Cela vaut aussi pour les slides à la racine.
- **Un `apply` d'un deck inchangé n'écrit rien** (F1). Les seules exceptions sont listées en R7.
- **Aucun élément n'est écarté** pour une raison de slot.
- **Profondeur bornée** en lecture (`read_tree`) et en écriture (`live_index`, `wanted_index`, qui rend `SlotsTooDeep` avant toute écriture).
- **Conteneurs et marques du mauvais type refusés à l'entrée** (`check_trees` dans les closures de `guarded`, `mod.rs` l. 212-218 et 461-466), jamais écrasés en silence.
- **ARCHITECTURE ≤ 50 lignes, committée avec le code qu'elle décrit** ; **numéros de ligne vérifiés**.

## Valeurs fixées

| Nom | Où | Valeur | Rôle |
|---|---|---|---|
| `SLIDES` | `replica/schema.rs` | `"slides"` | clé de la carte `deck` qui porte le `LoroTree` du deck |
| `SLIDE` | `replica/schema.rs` | `"@slide"` | méta d'une racine : seule `LoroValue::Bool(true)` en fait une slide ; une racine d'une autre valeur est acceptée et ignorée (R0, « Accepté ») ; interdit hors racine, quelle que soit la valeur |
| `SLOT` | `replica/schema.rs` | `"@slot"` | **carte fusionnable** { `TreeID` du parent en `counter@peer` → slot } |
| `DELETED` | `replica/schema.rs` | `"@deleted"` | préfixe réservé. Seule forme admise : `@deleted/<u64 décimal canonique>` = `true` |
| `mark_key(peer)` | `replica/schema.rs` | `format!("{DELETED}/{peer}")` | la marque de ce pair (décimal, comme dans `TreeID::to_string`) |
| `ORPHANS` | `replica/schema.rs` | `"orphans"` | **inchangé** |
| `ELEMENTS`, `ROOT_OWNER`, `Ctx::lists` | `replica/schema.rs` | — | **retirés** en fin de Task 4, quand plus rien ne les importe |
| `Kind::Slides` | `replica/schema.rs` | variante sans charge | **remplace** `Kind::Collection` : `slides` du deck, et rien d'autre |
| `REPLICA_FORMAT` | `replica/mod.rs` | `3_i64` | était 2 ; 3 = deck en un seul arbre, suppression douce |
| `SOURCES_VERSION` | `sources.rs` | `9` | était 8 |
| `DECK_VERSION` | `model.rs` | `5` | **inchangé** |
| fractionnaire | `tree_at` | `enable_fractional_index(0)` | inchangé, appliqué à l'arbre du deck |
| `Ids::next` | `canonical.rs` | `HashMap<String, u64>` | prochain `n` à essayer par id de base (t-693) |
| `Moved` | `ops/moving.rs`, réexporté | `{ element, from, to }` | un déplacement entre slides fait par `move_element` |

Variante d'erreur de `ReplicaInner` (`crates/oku-slides-core/src/error/mod.rs`, l. 384-419 ; kinds l. 421-444) :

| Variante | `display` | `ReplicaErrorKind` |
|---|---|---|
| `MalformedDeckTree { what: &'static str, backtrace }` (remplace `ElementsNotATree`, l. 412 et 427) | `"malformed deck tree: {what}"` | `Corrupted` |

`what` vaut l'une de ces chaînes fixes : `"slides are not a tree"`, `"root without @slide"`, `"@slide outside the root"`, `"malformed deletion mark"`, `"@slot is not a mergeable map"`. Aucune clé i18n nouvelle : la variante passe par le kind.

**Collision des clés réservées.** Ni `Slide` ni `Element` n'a de clé de premier niveau commençant par `@` (`reserved_keys_never_collide_with_model_fields`, `tree_tests.rs`). Les clés réservées d'une méta sont donc `@slot`, `@slide` et toute clé de préfixe `@deleted`. Un paramètre de ce nom vit un cran plus bas, dans `params`. Le test `param_named_like_a_reserved_key_stays_in_params` est étendu à `@slide` et `@deleted/7` (Task 3).

## Règles de réconciliation et de lecture (référence des Tasks 3, 4 et 6)

**R1. Marque et visibilité.**
- Un nœud est **marqué** si sa méta porte au moins une clé `@deleted/<u64>` de valeur `true` (`is_marked`, calculé **une fois** par nœud). Toute autre clé de préfixe `@deleted`, et toute autre valeur, est refusée à l'entrée (R0). La lecture n'en voit donc jamais.
- La lecture et l'index de l'écriture ne descendent **jamais** sous un nœud marqué : un nœud vivant sous un parent marqué est caché avec lui (sonde, `p3_move_under_soft_deleted_parent_reappears_on_undo`).
- `tree.delete` n'est plus jamais appelé par la réconciliation. Seul l'`UndoManager` supprime encore un nœud pour de bon : il défait un `create` par un `delete` (défaire un ajout, R10 de la revue ; ADR-0006, Limites).
- La réconciliation pose `mark_key(doc.peer_id())` une seule fois par nœud, sur un nœud visible. Elle ne retire jamais une marque ; seule l'annulation le fait (`p4_redo_restores_the_mark`).
- Limite de confiance : un pair peut supprimer n'importe quel nœud par une marque, comme il le pouvait par `tree.delete`. Une marque posée par un pair absent ne s'annule pas (ADR-0006).
- **Nœud vide.** Défaire un `mov` dont la cible a été supprimée par Loro (un ajout défait par un pair) fait créer par Loro un nœud **neuf à méta vide** sous l'ancien parent (loro-internal 1.16.2, `handler.rs` l. 1376-1400). Un nœud dont la méta n'a aucune clé est ignoré avec son sous-arbre, à la lecture comme à l'index, **sans `Issue`**. Il n'est ni lu ni écrit, et il ne fournit pas de slot hérité (R2). Pas d'`Issue` : ce nœud naît d'un chemin normal (deux annulations croisées) et ne porte aucune donnée ; signaler chaque occurrence alarmerait l'utilisateur pour rien.

**R0. Contrôle d'entrée (`check_trees`, import et fusion, `Corrupted`).** Un seul parcours itératif de `tree.nodes()`, nœuds supprimés par Loro exclus. Coût linéaire en nœuds et en clés de méta. `deck.slides` absent : accepté (`Ok`, comme aujourd'hui). Le contrôle ne refuse que ce qu'aucun chemin du code ne produit, annuler et rétablir compris. Chaque refus est justifié :
- `deck.slides` d'un autre type qu'un `LoroTree`. *Jamais produit :* l'arbre est créé au commit initial, avant l'`UndoManager`, donc aucune annulation ne le touche ;
- un nœud non racine qui porte `@slide`, quelle que soit sa valeur : une slide sous un élément ou sous une autre slide. *Jamais produit :* la réconciliation ne place une slide qu'à la racine. Annuler un `mov` de slide la rend à la racine. Le nœud que Loro recrée en défaisant un `mov` a une méta vide, sans `@slide`. Seul un replica forgé y arrive (la sonde par un `mov` direct, `p7_cross_slide_cycle_converges_without_panic`) ;
- une clé de préfixe `@deleted` qui n'est pas `@deleted/<n>`, où `n` est un `u64` dont `n.to_string()` redonne la chaîne, ou dont la valeur n'est pas `Bool(true)`. *Jamais produit :* la réconciliation écrit `mark_key(peer)` = `true`. Annuler supprime la clé (absente, filtrée par `keys()`), et rétablir réécrit `true` sous la même clé ;
- un `@slot` présent qui n'est pas une `LoroMap` dont `id().is_mergeable()` est vrai. *Jamais produit :* `@slot` n'est créé que par `ensure_mergeable_map`. Annuler sa création retire la clé, et rétablir sous un nœud recréé repose un marqueur fusionnable (`handler.rs` l. 1116-1121).

**Accepté, et non refusé :** une racine sans `@slide = true`. Loro en crée une avec une méta vide (R1, « Nœud vide ») quand A défait le déplacement d'une slide dont B a défait l'ajout. Elle est ignorée avec son sous-arbre par `slide_order` et `live_slides`. Une méta vide passe sans `Issue`. Une méta non vide sans `@slide = true` (forgée) émet `issue-unreadable-slide`, signal utile au diagnostic, puisque le code ne la produit jamais.

**R2. Slot résolu**, comme ADR-0005. `pid` est le `TreeID` du parent en chaîne, **à tous les niveaux** : slide, élément, élément imbriqué. Les trois sites qui passent aujourd'hui l'id d'élément changent : `live_index` (`tree.rs` l. 203), `place` (`tree.rs` l. 347, `wanted[p].key.0` ne sert plus de `pid`) et `read_tree` (`read.rs` l. 221). Les règles :
1. `@slot[pid]`, si c'est une chaîne non vide ;
2. sinon, le slot résolu par la règle 1 du premier frère **visible** qui en a un, dans l'ordre des enfants de Loro. Les frères marqués sont écartés **avant** ce calcul ;
3. sinon, les clés **canoniques** de `@slot` : celles que `TreeID::try_from` relit et que `to_string()` réécrit à l'identique, avec une valeur chaîne non vide. Une clé illisible ou non canonique est ignorée, sans deviner (sonde, `p9_tree_id_string_key_is_stable_across_snapshot_and_peers`) :
   - 3a. on ne retient que les clés canoniques dont le nœud est **absent** de l'arbre ou **supprimé** par Loro. Une clé dont le nœud est vivant mais n'est pas le parent réel est une entrée périmée, laissée par un ancien parent : elle est ignorée ici. Si les clés retenues existent et donnent **toutes le même slot**, ce slot est pris **sans `Issue`**. C'est le cas d'un parent recréé par « rétablir » sous un nouveau `TreeID` ;
   - 3b. sinon (aucune clé de nœud disparu, ou des slots différents), la première clé canonique dans l'ordre des clés, vivantes comprises, donne le slot, et `issue-slot-repaired` est émise ;
4. sinon `ORPHANS`.

Les cas 2, 3b et 4 émettent `issue-slot-repaired`. `ordered_children` ignore les enfants marqués. En Task 3, le `pid` est encore l'id d'élément ou `""` ; les règles 3a et 3b n'y existent pas encore (la règle 3 reste celle d'ADR-0005).

**R3. `@slot` fusionnable (parade t-188).** `slot_map_at(meta)` :
- `@slot` absent : `meta.ensure_mergeable_map(SLOT)` ;
- `@slot` est une `LoroMap` et `id().is_mergeable()` : on la rend ;
- autre chose : `MalformedDeckTree`. Ce cas est inatteignable après R0 ; c'est une défense.

Deux pairs qui créent `@slot` en concurrence sur un même nœud gardent leurs deux entrées. C'est le cas d'un nœud sans `@slot`, réparé par l'un et déplacé par l'autre. Il n'y a pas de réparation d'un `@slot` forgé : il est refusé à l'entrée.

**R4. Ordre des slides.** `slide_order(tree)` : les racines non marquées qui portent `@slide = true`, dans l'ordre de Loro. Une racine sans `@slide = true` est ignorée avec son sous-arbre (R0, « Accepté »). Une slide hors racine est refusée à l'entrée. La lecture et l'écriture partagent cette fonction ; la lecture seule émet l'`issue-unreadable-slide` d'une racine non vide sans `@slide` (fonction voisine `stray_roots`, non utilisée par l'écriture).

**R5. Appariement (`pair`).** C'est l'identité d'un objet : l'`id` de la méta, jamais le `TreeID`.
1. **Slides** : clés `keyed(ids)` de `slide_order` contre celles du JSON. Une clé voulue sans slide vivante sera créée.
2. **Éléments, passe 1** : dans chaque slide voulue appariée à une slide vivante `L`, un élément voulu de clé `(id, n)` prend l'élément vivant de `L` de même clé. Les clés sont propres à chaque slide, dans l'ordre `uorder` d'ADR-0005.
3. **Éléments, passe 2 (d'une slide à l'autre)**. Une *racine déplacée* est un élément voulu non apparié dont le propriétaire voulu est apparié, ou est la slide. Elle prend un élément vivant non apparié d'une autre slide :
   - **déplacement annoncé** (à partir de la Task 6) : `moves` (de `move_element`) contient `(id, from, to)`. La slide voulue `to` est appariée ; la slide vivante `from` n'est pas `to`, et peut être retirée dans la même transaction : l'annonce fait foi (revue finale). L'élément vivant est la première occurrence de `id` dans `from` (ordre `uorder`), non appariée ;
   - **sinon, déplacement déduit** (R7 de la revue), seulement si toutes ces conditions tiennent : l'`id` figure dans **une seule** slide vivante (éléments visibles) et dans **une seule** slide voulue ; ces deux slides sont appariées et différentes ; les `kind` sont égaux ;
   - sinon, rien : retrait et création.
4. **Descendants d'une racine déplacée** : ils sont appariés par clé `(id, n)` calculée dans le sous-arbre (ordre `uorder` du sous-arbre), des deux côtés.
5. Hors déplacement annoncé, un élément d'une slide retirée, ou voulu dans une slide nouvelle, n'est jamais apparié d'une slide à l'autre : il est créé à neuf.
6. Un nœud marqué n'est jamais indexé, donc jamais apparié : un id réajouté crée un nœud neuf.
7. Coût : une `HashMap<&str, …>` par passe, un seul parcours de chaque côté. Pas de balayage quadratique.

**R6. Écriture du deck** (`Writer::sync_deck(tree, wanted)` en Task 4 ; le paramètre `moves` s'ajoute en Task 6), en un seul commit (pas de `split`).
- *Temps 0, index sans écriture.*
  - `wanted = wanted_slides(registry, slides)?` est calculé au début de `sync_map(Ctx::Deck)`, **avant toute écriture dans le deck**. Il rend `SlotsTooDeep` au-delà de la borne. `sync_deck` est appelé une seule fois, avec ce `Vec`, sans `Option`.
  - Puis `live = live_slides(tree)` (avec `live_index(tree, slide)` par slide) et `pairing = pair(&live, &wanted)` (`pair(&live, &wanted, moves)` dès la Task 6).
- *Temps 1, marques.*
  - Une slide vivante non appariée reçoit une marque (une op), et son sous-arbre n'est pas visité, sauf sous un élément apparié ailleurs (déplacement annoncé) : ses descendants non voulus y sont marqués ou différés comme dans une slide gardée, car ils sortent avec lui.
  - Dans une slide appariée, le temps 1 d'ADR-0005 s'applique (`has_wanted`, saut du sous-arbre, `deferred`), avec « voulu » = apparié, quelle que soit la slide, et `mark` au lieu de `tree.delete`.
- *Temps 2, placement.*
  - Les slides d'abord, sous `TreeParentId::Root`, par `arrange` : LIS sur les racines appariées, puis `mov_after`, `mov_before` ou `mov` ; ou `create_at(Root, idx)` puis `@slide = true`.
  - Puis chaque slide voulue, dans l'ordre :
    1. `sync_map(&meta, obj, Ctx::Slide)` : champs et `params`, sans les slots ;
    2. `place_elements(tree, node, &ws.elements, &pairing.elements[i])` : le temps 2 d'ADR-0005, avec `parent = Node(slide)`, `pid = slide.to_string()` et « connu » = apparié.
  - Un élément apparié dans une autre slide n'est pas parmi les enfants : il est déplacé. L'entrée `@slot` de l'ancienne slide reste et sert de repli.
- *Temps 3.* Chaque nœud de `deferred` (toutes slides) reçoit une marque. Aucun n'est déjà marqué : un différé a un descendant voulu, donc aucun de ses ancêtres n'est marqué au temps 1.
- *Pas de cycle local* (argument d'ADR-0005, étendu) :
  - les slides sont placées à la racine avant tout élément, et un propriétaire est placé avant ses éléments ;
  - la chaîne d'ancêtres d'un parent n'est donc faite que de nœuds appariés à ses propriétaires voulus, de sa slide et de la racine ;
  - l'appariement est injectif, donc le nœud déplacé n'en fait jamais partie.

**R7. Invariant F1.** `apply(&replica.deck().0)` laisse `len_ops` inchangé. Exceptions, écrites une seule fois au premier `apply` du deck relu, puis plus rien :
- renommage t-652 ;
- slot réparé (R2, cas 2, 3 et 4).

**R8. Lecture du deck.**
- `read_map_at(Ctx::Deck)` :
  - `(Kind::Slides, Container::Tree(t))` donne `read_deck_tree(&t, depth.deeper())` ;
  - `(Kind::Slides, _) => continue` est un bras explicite. Un autre conteneur est refusé à l'entrée (R0) ; la lecture seule le saute, et `materialize` pose `slides: []`.
- `read_deck_tree` (nom nouveau : `read_slides`, qui lit la liste, existe jusqu'au branchement de la Task 4) : pour chaque nœud de `slide_order`, `read_map_at(&meta, Ctx::Slide, depth)`, qui saute les clés réservées. Puis `read_tree(tree, &kids, Depth { levels: depth.levels + 1, slots: 0 })`, versé par `merge_slots`, puis `sort_keys()`. L'id de la slide est reporté dans les `Issue` émises pour elle. `get_meta` d'un nœud listé ne peut pas échouer : pas de branche « méta illisible ».
- **Une seule signature**, posée en Task 3 : `fn read_tree(&mut self, tree: &LoroTree, kids: &Children, depth: Depth) -> Map<String, Value>`. L'appelant calcule `kids = ordered_children(tree, parent, &pid)` (marqués et nœuds vides exclus). Pour chaque enfant, `read_tree` calcule lui-même les `Children` de cet enfant, avec son `pid` : l'id d'élément en Task 3, `child.to_string()` dès la Task 4. Il s'en sert pour le compte d'`issue-slot-too-deep` (plus de `children_num`) et pour l'appel récursif.
- Ids en double : `canonical::unique_ids` dans `materialize`, inchangé. Ils restent possibles : un id réajouté, puis l'annulation de l'ancien retrait, donnent deux nœuds au même id (Task 3, test dédié).

---

## Task 1: ADR-0006, ADR-0005 et ADR-0002 (t-746, t-747, t-759)

`replica/ARCHITECTURE.md` n'est **pas** touché ici : il est committé avec le code qu'il décrit, en Task 4.

**Files :**
- Create : `docs/adr/0006-deck-en-un-seul-arbre-suppression-douce.md`.
- Modify :
  - `docs/adr/0005-elements-d-une-slide-en-arbre-loro.md` (en-tête l. 3-8 ; Limites l. 87-88, 92-100 et 105-106) ;
  - `docs/adr/0002-sources-du-deck-en-crdt-loro.md` (en-tête ; l. 44-46, l. 67, l. 84-85) ;
  - `docs/adr/README.md`.

- [ ] **Step 1 : écrire l'ADR-0006 « Le deck en un seul arbre Loro, suppression douce »**, au format de l'ADR-0005 (skill `common:new-adr`). En-tête :
  - statut accepté, date 2026-10-10, tâches t-746 et t-747 ;
  - « Remplace en partie : ADR-0005 (un arbre par slide, `@slot` par id de parent, suppression par `tree.delete`) et ADR-0002 (slides en `LoroMovableList`) » ;
  - sonde : `crates/oku-slides-core/tests/loro_deck_tree_probe.rs`.

  Contenu :
  - **Contexte** :
    - un arbre par slide : passer d'une slide à l'autre est une copie suivie d'une suppression, et l'édition concurrente est perdue. L'UI ne déplaçait d'ailleurs jamais un élément d'une slide à l'autre : couper retirait l'élément, coller posait une copie aux ids renouvelés ;
    - `tree.delete` : Loro ne ressuscite jamais un nœud. Défaire un retrait recrée une copie, et le `mov` ou l'édition concurrente d'un pair est perdue ;
    - slides en `LoroMovableList` : défaire un retrait face au `mov` d'un pair laisse deux slides (t-652), et la parade t-263 impose un commit par `mov`.
  - **Décision** :
    - un seul `LoroTree` sous `deck.slides`. Les racines, et elles seules, sont les slides : méta = la slide sans ses slots, plus `@slide = true`. Les descendants sont les éléments. Ordre fractionnaire, jitter 0 ;
    - `@slot` : carte **fusionnable** (`ensure_mergeable_map`) { `TreeID` du parent en `counter@peer` → slot }, à tous les niveaux ;
    - suppression douce (R1) : une marque `@deleted/<pair>` = `true` par pair, et non un booléen. Avec un booléen, l'annulation de A effacerait la suppression concurrente de B (sonde, `p1_concurrent_double_soft_delete_then_one_undo` contre `p1_per_peer_mark_keys_survive_a_concurrent_undo`). La réconciliation ne retire jamais une marque, et un id réajouté crée un nœud neuf ;
    - entrée stricte (R0) : `@slide` hors racine, marques et `@slot` mal formés refusés à l'import et à la fusion. Seul ce qu'aucun chemin du code ne produit, annuler et rétablir compris, est refusé ; chaque refus porte sa justification. Une racine sans `@slide` (nœud vide que Loro crée en défaisant un `mov`) est acceptée et ignorée ;
    - appariement (R5) : slides, puis éléments dans leur slide, puis d'une slide à l'autre, **seulement** pour un déplacement annoncé par `move_element`, ou déduit sans ambiguïté (id unique des deux côtés, même `kind`) ;
    - **chemins qui produisent un déplacement entre slides** : l'op `move_element` (outil `apply` de l'agent, couper-coller de l'UI), et un `apply` qui retire un id d'une slide et l'ajoute dans une autre, sous les conditions du déplacement déduit ;
    - la parade t-263 ne reste que pour les listes (items, `dataset`).
  - **Lecture** : R1 (nœud vide ignoré sans `Issue`), R2 (clés relues par `TreeID::try_from` et canoniques ; réparation silencieuse si les clés des parents disparus donnent toutes le même slot, les entrées de parents vivants mais périmés étant ignorées), R4.
  - **Compatibilité** : aucune. `REPLICA_FORMAT` passe de 2 à 3 et `SOURCES_VERSION` de 8 à 9. Un `deck.loro` v8 relu hors zip est refusé (`replica-unsupported-version`), et la reprise de session est mise de côté (`.failed-*`). Le JSON du deck ne change pas ; le comportement de « couper » change (Task 7).
  - **Conséquences** :
    - un déplacement entre slides, ou entre slots, garde les éditions concurrentes ;
    - annuler un retrait rend le même nœud, avec le `mov` et l'édition d'un pair, même reçus entre le retrait et l'annulation (`p1_undo_soft_delete_after_receiving_move_does_not_move_back`) ;
    - un enfant qu'un pair a déplacé sous un parent retiré revient avec lui à l'annulation (`p3_…`) ;
    - réordonner les slides se fait en un commit ;
    - coût mesuré par la sonde (`p8_cost_single_tree_vs_tree_per_slide`, deck de 40 × 20, une exécution debug et une release) : construction 6-7 ms contre 4-15 ms, lecture 0,4-1,0 ms contre 0,6-1,3 ms, snapshot après 200 passages d'environ 100 Ko contre 120 Ko. Ces chiffres sont indicatifs et ne couvrent pas les nœuds marqués.
  - **Limites** :
    - les nœuds marqués ne sont jamais libérés. Le snapshot grandit, et la lecture comme chaque `apply` coûtent O(vivants + marqués), car `children()` rend aussi les marqués. `max_replica_bytes` n'est vérifié qu'à l'import : un deck très retouché peut un jour être refusé à la reprise. Compaction : t-770 ;
    - une marque d'une session close ne s'annule plus (la pile d'annulation repart de zéro à la reprise) ;
    - défaire un **ajout** supprime le nœud pour de bon, car Loro inverse `create` par `delete` : un sous-arbre qu'un pair a déplacé dessous disparaît avec lui. Aucun réglage simple ne l'évite : l'inverse d'un `create` appartient à l'`UndoManager`. Suivi : t-776 ;
    - un élément déplacé sous un élément, ou dans une slide, retiré en parallèle est caché avec lui ; l'annulation du retrait le rend ;
    - sans `move_element`, un id qui figure dans plusieurs slides n'est jamais apparié d'une slide à l'autre : son édition concurrente suit la règle d'avant (perdue) ;
    - retirer un id d'une slide et l'ajouter dans une autre, dans un même `apply`, vaut déplacement quand le déplacement est déduit (R5) : l'édition concurrente du pair suit le nœud ;
    - un id réajouté puis l'annulation de l'ancien retrait laissent deux nœuds au même id, réparés à la lecture (t-652) ;
    - jitter 0 et `Rearrange` implicite touchent désormais aussi **l'ordre des slides** : deux insertions concurrentes de slides au même rang, puis une insertion entre elles, peuvent ramener une slide qu'un pair déplaçait (test en Task 9) ;
    - un replica forgé dont l'arbre est mal formé (R0) est refusé en bloc, à l'import comme à la fusion ;
    - **limite de confiance** : un pair peut supprimer n'importe quel nœud, slide ou élément, par une marque `@deleted/<pair>`, comme il le pouvait par `tree.delete`. La marque est bien formée, donc acceptée. Seule l'annulation du pair qui l'a posée la retire : une marque d'un pair absent, ou d'une session close, cache le nœud pour de bon, sans `Issue` ;
    - un nœud vide que Loro crée en défaisant un `mov` (cible supprimée entre-temps) reste dans l'arbre, ignoré, et s'ajoute aux nœuds morts ;
    - Loro reste figé par `Cargo.lock` (1.16.2). Pas de `cargo update -p loro` sans relancer `tests/loro_tree_probe.rs` et `tests/loro_deck_tree_probe.rs`.
- [ ] **Step 2 : ADR-0005.**
  - En-tête : une ligne « **Remplacée en partie par :** [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md) (un arbre pour tout le deck, `@slot` par `TreeID`, suppression douce) ». C'est la même forme que la ligne « Remplace en partie » ; le statut reste « accepté ».
  - Limites :
    - l. 87-88 (« Un élément déplacé sous un élément supprimé en parallèle disparaît avec lui ») devient « … est caché avec lui ; l'annulation du retrait le rend (ADR-0006) » ;
    - retirer la puce « Défaire une suppression recrée un nœud neuf… Suite : t-747 » (l. 92-98) et la puce « Déplacer un élément d'une slide à une autre perd toujours l'édition concurrente… Suite : t-746 » (l. 99-100), désormais levées ;
    - l. 105-106 : « … sans relancer les sondes (`tests/loro_tree_probe.rs`, `tests/loro_deck_tree_probe.rs`) ».
- [ ] **Step 3 : ADR-0002 et README.**
  - En-tête d'ADR-0002 : « **Remplacée en partie par :** ADR-0004, ADR-0005, ADR-0006 », même forme, statut « accepté ».
  - L. 44-46 : « collections ordonnées à `id` : `slides` et slots (`blocks`, `elements`, `children`) → nœuds d'un seul `LoroTree` (slides en racine), voir l'[ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md) ».
  - L. 67 : « ; plus nécessaire pour l'arbre du deck depuis l'ADR-0006 (reste pour les listes d'items et de données) ».
  - L. 84-85 : « Pas de `revert_to` : il perdait des slides déplacées dans la `LoroMovableList` (`mov`), ou échouait » devient « Pas de `revert_to` (constaté sur les `LoroMovableList` de conteneurs, qui portent encore les items de liste) ».
  - README : la ligne 0006 est ajoutée, `| [0006](0006-deck-en-un-seul-arbre-suppression-douce.md) | Le deck en un seul arbre Loro, suppression douce | accepté |`. Les statuts des lignes 0002 et 0005 restent « accepté » (convention unique : le remplacement partiel se lit dans l'en-tête de l'ADR).
- [ ] **Step 4 : commit.** C'est de la doc seule, pas de tests à lancer.

  ```text
  docs(adr) : ADR-0006, le deck en un seul arbre Loro et la suppression douce (t-746, t-747)
  ```

---

## Task 2: `Ids::fresh` en coût linéaire (t-693, t-760)

**Files :**
- Modify : `crates/oku-slides-core/src/canonical.rs` (`Ids`, l. 80-104) ; `crates/oku-slides-core/src/canonical_tests.rs` (493 lignes, ajouts en fin de fichier) ; `crates/oku-slides-core/tests/regressions/main.rs`.
- Create : `crates/oku-slides-core/tests/regressions/issue_0693_unique_ids_keeps_its_names.rs`, déclaré dans `main.rs` entre `issue_0677_…` et `issue_0694_…` (ordre numérique).

**Interface :**
```rust
struct Ids {
    taken: HashSet<String>,
    kept: HashSet<String>,
    /// Prochain `n` à essayer par id de base : tous les plus petits sont pris.
    next: HashMap<String, u64>,
    /// Candidats essayés par `fresh` (tests : coût linéaire, t-693).
    #[cfg(test)]
    tries: usize,
}

impl Ids {
    /// `{id}-{n}`, au premier `n ≥ 2` libre ; désormais pris.
    ///
    /// Reprend au dernier `n` essayé pour `id` : `taken` ne fait que grandir,
    /// donc les `n` déjà essayés restent pris. Même résultat qu'une recherche
    /// depuis 2, en coût linéaire sur toute la réparation.
    fn fresh(&mut self, id: &str) -> String;
}
```
- Corps : `let n = self.next.entry(id.to_owned()).or_insert(2);`, puis une boucle qui forme `{id}-{n}`, incrémente `*n` (et `tries` sous `#[cfg(test)]`), et rend le candidat dès que `taken.insert` réussit.
- `Ids::new` initialise `next: HashMap::new()` et, sous `#[cfg(test)]`, `tries: 0` dans le littéral.

- [ ] **Step 1 : tests rouges** dans `canonical_tests.rs`. Ajouter d'abord le seul champ `tries` (son initialisation et son incrément dans la boucle actuelle), sans toucher l'algorithme.
  - `ids_fresh_with_two_thousand_suffixes_taken_is_linear` :
    - `let mut ids = Ids::new((2..=2_000).map(|n| format!("x-{n}")).chain(["x".to_owned()]));`
    - 2 000 appels à `ids.fresh("x")` ;
    - assertions : les noms rendus valent exactement `x-2001` … `x-4000`, dans l'ordre, et `ids.tries == 3_999` (1 999 suffixes pris sautés, plus 2 000 succès) ;
    - rouge attendu : environ 6·10⁶ essais, par assertion, en moins d'une seconde.
  - `ids_fresh_bases_do_not_interfere` : `Ids::new(["x", "x-3"])`, puis dans l'ordre `fresh("x")`, `fresh("x-2")`, `fresh("x")`, `fresh("x-2")`. Rendus attendus : `x-2`, `x-2-2`, `x-4` (car `x-3` est pris), `x-2-3`. Garde.
  - `unique_ids_after_a_taken_suffix_keeps_the_same_names` (garde) : une slide dont `blocks` porte, dans l'ordre, `x` (« a »), `x` (« b »), `x-2` (« c »), `x` (« d »), quatre textes différents. Après `unique_ids`, les ids valent `["x", "x-3", "x-2", "x-4"]`. Le test fige le résultat de l'ancien algorithme.
  - Lancer `-E 'test(ids_fresh) | test(unique_ids_after)'` : le premier est rouge par assertion, les autres sont verts.
- [ ] **Step 2 : régression** `issue_0693_unique_ids_keeps_its_names.rs`, test `canonicalize_of_five_thousand_copies_names_them_in_order`. Il passe par l'API publique (`oku_slides_core::canonicalize`, car `unique_ids` est `pub(crate)`). Une slide porte 5 000 textes d'id `x`, de contenus différents. Après `canonicalize`, les ids valent `x, x-2, …, x-5000`. C'est une garde de résultat. La linéarité est prouvée par le compteur du test unitaire, qu'une régression externe ne peut pas lire ; l'écart à G16 est écrit en tête du fichier.
- [ ] **Step 3 : implémenter** `next` comme dans l'interface.
- [ ] **Step 4 : lancer** `-E 'test(canonical) | test(issue_0693)'`, puis la commande d'avant commit.
- [ ] **Step 5 : commit.**

  ```text
  perf(canonical) : Ids::fresh reprend au dernier suffixe essayé, coût linéaire (t-693)
  ```

---

## Task 3: suppression douce dans l'arbre des éléments (t-747, t-761)

L'arbre reste un arbre par slide (ADR-0005), et les slides restent dans leur `LoroMovableList`. Cette tâche ne change que :
- la suppression d'un élément ;
- la lecture des nœuds marqués ;
- la création de `@slot` ;
- le contrôle d'entrée des marques et de `@slot`.

La Task 4 étend tout cela aux slides. Les aides de `soft_delete_tests` ne passent que par les aides partagées (`forge`, `node_of`, `owner_key`), jamais par un accès direct à `slides` ou à `@elements`. Ainsi, la Task 4 n'a qu'un endroit à migrer.

**Files :**
- Create :
  - `crates/oku-slides-core/src/replica/soft_delete_tests.rs` (déclaré dans `tree.rs`, après `tree_read_tests`) ;
  - `crates/oku-slides-core/tests/regressions/issue_0747_undo_remove_keeps_the_peer_move_and_edit.rs` (déclaré dans `main.rs` après `issue_0694_…`).
- Modify :
  - `src/replica/schema.rs` : `DELETED`, `mark_key`, `Ctx::reserved` devient un prédicat (l. 133-140) ;
  - `src/replica/write.rs` : `Writer` l. 112-116, champ `mark_key` ; filtre `stale` l. 142-147 ;
  - `src/replica/read.rs` : `read_map_at` l. 127 ; `read_tree` l. 194-235 (signature avec `Children`, compte des enfants) ;
  - `src/replica/tree.rs` : `is_marked`, `ordered_children` l. 70-86, `slot_map_at`, `Writer::mark`, `sync_tree` l. 328 et 354-359, `place` l. 415 et 420, `node_of` l. 440-464 ;
  - `src/replica/mod.rs` : construction du `Writer` (l. 341), `check_trees` (l. 116-132 : marques et `@slot` de chaque nœud), doc de `guarded` (l. 153-158) ;
  - `src/error/mod.rs` : `ElementsNotATree` devient `MalformedDeckTree { what }` (l. 412, 427) ;
  - aides de test qui créent `@slot` par `insert_container`, à passer à `ensure_mergeable_map` comme la production :
    - `src/replica/tree_tests.rs` `forge` (l. 148) ;
    - `src/replica/mod_tests.rs` `read_deeply_nested_slots_stops_at_the_depth_bound` (l. 668) ;
    - `src/replica/read_tests.rs` `put_nodes` (l. 75) ;
  - `src/replica/tree_tests.rs` :
    - au Step 2 : `add` (l. 52), `remove` (l. 60) et `with_pile` (l. 72) passent en `pub(super)`, et trois aides partagées sont créées ici, en `pub(super)`, pour `soft_delete_tests` puis `deck_tree_tests` :
      - `owner_key(r, slide, owner: Option<&str>) -> String` : en Task 3, `""` ou l'id d'élément ; la Task 4 ne change que son corps ;
      - `marks(tree, node) -> Vec<String>` : les clés de préfixe `@deleted`, triées, lues brutes ;
      - `fixed_peers(deck, a, b) -> (DeckReplica, DeckReplica)` : deux pairs issus d'un même snapshot, par `set_peer_id` ;
    - `removing_an_owner_removes_its_subtree` (l. 529-549) et `empty_param_name_with_elements_round_trips` (l. 679-686) comptent les nœuds vivants par une aide `visible_ids(tree, root) -> Vec<String>` : parcours brut et itératif (`tree.children`, `get_meta`, clés `@deleted/` lues directement) ;
    - `removing_the_first_of_three_writes_one_delete` (l. 435) devient `…_writes_one_mark` ;
    - `apply_on_a_forged_deep_chain_leaves_unread_nodes_untouched` (l. 590-617) : `!is_marked` en plus de `!is_node_deleted` ;
    - `reserved_keys_never_collide_with_model_fields` (l. 209) et `param_named_like_a_reserved_key_stays_in_params` (l. 229) sont étendus à `@slide` et `@deleted/7` ;
    - les trois tests « `@elements` d'un autre type » (l. 619-671) : kind `Corrupted` inchangé, variante renommée ;
  - `src/replica/tree_read_tests.rs` l. 39-48, `read_element_without_any_slot_goes_to_orphans` : le nœud `y` à `@slot = I64(3)` est retiré du test. R0 le refuse désormais à l'entrée, et `from_snapshot_with_a_non_mergeable_slot_is_refused` couvre ce cas. `x` (sans `@slot`) reste, relu dans `orphans`. Les l. 30-37 (`read_element_with_only_another_owner_entry_keeps_that_slot`, clé `"pile"`) ne changent pas en Task 3 : les clés sont encore des ids ;
  - `tests/regressions/issue_0652_undo_remove_vs_peer_move.rs` (cas élément, l. 130-154) ;
  - `tests/regressions/issue_0694_undo_remove_vs_peer_move_in_tree.rs` (l. 108-154) ;
  - `tests/tree_convergence.rs` : `move_under_a_concurrently_removed_owner_is_a_documented_loss` (l. 180-197) devient `move_under_a_concurrently_removed_owner_is_hidden_until_undo`, avec son commentaire ;
  - `tests/untrusted.rs` (marques hostiles) ;
  - `tests/regressions/main.rs`.

**Consommateurs recensés** (grep fait) :
- `tree.delete` : `tree.rs` l. 328 et 357, rien d'autre en production.
- `reserved()` : `write.rs` l. 142-146, `read.rs` l. 127.
- `map_at(…, SLOT)` : `tree.rs` l. 415 et 420.
- `ElementsNotATree` : `error/mod.rs` l. 412, 427 ; `mod.rs` l. 31, 128 ; `tree.rs` l. 17, 26.
- `is_node_deleted` dans les tests : `tree_tests.rs` l. 544, 609 et 683.

**Interfaces :**

`schema.rs` :
```rust
/// Préfixe réservé des marques de suppression : `@deleted/<pair>`.
pub(super) const DELETED: &str = "@deleted";
/// La marque de suppression du pair `peer`.
pub(super) fn mark_key(peer: u64) -> String; // format!("{DELETED}/{peer}")
/// Le pair d'une marque bien formée, `None` sinon (forme canonique exigée).
pub(super) fn mark_peer(key: &str) -> Option<u64>;
impl Ctx {
    /// Vrai pour une clé du replica, jamais lue ni écrite comme un champ.
    /// `Slide` : `ELEMENTS` ; `Element` : `SLOT` ou préfixe `DELETED`.
    pub(super) fn reserved(self, key: &str) -> bool;
}
```

`tree.rs` :
```rust
/// Vrai si la méta porte une marque de suppression (R1).
pub(super) fn is_marked(meta: &LoroMap) -> bool;
/// La carte `@slot` fusionnable d'une méta, créée au besoin (R3).
pub(super) fn slot_map_at(meta: &LoroMap) -> Result<LoroMap, ReplicaInner>;
/// Les enfants visibles d'un parent, groupés par slot résolu.
pub(super) struct Children { pub(super) slots: BTreeMap<String, Vec<Child>> }
impl Writer<'_> {
    /// Marque `node` retiré par ce pair (R1), sans `tree.delete`.
    fn mark(&self, tree: &LoroTree, node: TreeID) -> Res; // set_value(&meta, &self.mark_key, true)
}
```
- `ordered_children` lit la méta de chaque enfant une fois. Il écarte les marqués et les nœuds vides (méta sans clé, R1), puis calcule `own` et le frère de repli sur les seuls restants (R2, règle 2).
- `sync_tree` : au temps 1, `tree.delete(l.node)` devient `self.mark(tree, l.node)`. Au temps 3, plus d'`is_node_deleted` : chaque différé reçoit `self.mark`.
- `place` : `map_at(&meta, SLOT)` devient `slot_map_at(&meta)`, à la création comme pour l'écriture de l'entrée.
- `node_of` (tests) : parcours brut et itératif, sans borne de profondeur (`forge_chain`, `tree_tests.rs` l. 573-587, cherche au-delà de `MAX_SLOT_DEPTH`). Il saute les nœuds marqués et leurs sous-arbres.

`write.rs` :
```rust
pub(super) struct Writer<'a> {
    pub(super) registry: &'a WidgetRegistry,
    pub(super) split: Split<'a>,
    /// Clé de la marque de suppression de ce pair (`mark_key`).
    pub(super) mark_key: String,
}
```
- `mod.rs` l. 341 : `write::Writer { registry, split, mark_key: schema::mark_key(doc.peer_id()) }`.

`mod.rs` : `check_trees` parcourt, pour chaque slide, `tree.nodes()` de son arbre (nœuds supprimés par Loro exclus). Il refuse une marque mal formée (R0, `mark_peer` ou valeur) et un `@slot` non fusionnable. En Task 4, il passe à l'arbre du deck.

`read.rs` :
- `read_map_at` : `if ctx.reserved(&key) { continue; }`.
- `read_tree(&mut self, tree: &LoroTree, kids: &Children, depth: Depth) -> Map<String, Value>` : la signature unique de R8. L'appelant (`read_map_at`, `Ctx::Slide`) calcule `ordered_children(&t, TreeParentId::Root, ROOT_OWNER)`. Pour chaque enfant, `read_tree` calcule les `Children` de cet enfant avec son id d'élément comme `pid`, compte ses enfants pour `issue-slot-too-deep`, et s'appelle avec eux.

- [ ] **Step 1 : régressions rouges.**
  - `issue_0652_undo_remove_vs_peer_move.rs` :
    - renommer `undo_remove_while_peer_moves_and_edits_element_keeps_the_undone_copy` en `undo_remove_while_peer_moves_and_edits_element_keeps_the_peer_edit` ;
    - remplacer le commentaire « Limite acceptée (t-694, suivi t-747) … » par « Suppression douce (t-747, ADR-0006) : annuler retire la marque, et le même nœud revient avec le `mov` et l'édition de B. » ;
    - asserter `block_ids(&merged) == ["c2", "c1", "c3"]` et le `kicker` de `c2` égal à `"Pilier B"`.
  - `issue_0694_undo_remove_vs_peer_move_in_tree.rs` :
    - renommer `undo_remove_while_peer_moves_and_bolds_element_across_slots_keeps_one_unique_copy` en `…_keeps_the_peer_move_and_bold` ;
    - asserter que `b-points` (id exact) est présent une fois, dans `elements` de `s-constat`, avec le cadre de `framed`, et gras (`body[0].runs[0].bold`) ;
    - retirer le commentaire « Limite acceptée (t-747…) » ;
    - dans `undo_remove_while_peer_moves_owner_with_children_keeps_ids_unique_at_every_depth`, asserter en plus que `pile` est dans `elements` et que ses `children` valent `["k1", "k2"]`, avec `k1` gras.
  - `issue_0747_undo_remove_keeps_the_peer_move_and_edit.rs` (aides recopiées de `issue_0694_undo_remove_vs_peer_move_in_tree.rs` : `peers`, `exchange`, `apply`, `with_ops`, `remove`, `add`, `find`) :
    - `concurrent_removals_then_one_undo_keep_the_element_removed` (parade de `p1_concurrent_double_soft_delete_then_one_undo`). Trois séquences :
      1. A retire `b-points`, B le retire, échange, A annule, échange ;
      2. A retire puis annule, B retire, échange ;
      3. B retire, échange, A retire, échange, A annule, échange.

      Dans les trois : `b-points` absent chez A et chez B, decks égaux, aucune `Issue`, puis `apply(deck())` sans écriture sur les deux pairs.
    - `undo_remove_after_receiving_the_peer_move_keeps_the_move` (`p1_undo_soft_delete_after_receiving_move_does_not_move_back`) : A retire `b-points` ; B le passe dans `elements` (cadré) et le met en gras ; A fusionne B, **puis** annule ; échange. `b-points` est présent une fois, dans `elements`, gras, chez les deux.
  - Lancer `-E 'test(issue_0652) | test(issue_0694_undo_remove) | test(issue_0747)'` : rouges par assertion (copie recréée, gras ou `kicker` perdus, retrait de B défait). Noter la sortie.
- [ ] **Step 2 : squelettes.**
  - `DELETED`, `mark_key`, `mark_peer`, `is_marked` (provisoire : `false`), `slot_map_at` (provisoire : `map_at(meta, SLOT)`), `Writer::mark` (provisoire : `tree.delete`), champ `mark_key`, `reserved` en prédicat **avec le préfixe `DELETED` dès ce pas**, `MalformedDeckTree`.
  - Les passages en `pub(super)` et les trois aides partagées de `tree_tests` (**Files**).
  - Tout compile, et les tests existants restent verts.
- [ ] **Step 3 : tests unitaires rouges** dans `soft_delete_tests.rs`.

  Aides reprises de `tree_tests` (`pub(super)`) : `new`, `apply`, `reread`, `exchange`, `forge`, `node_of`, `owner_key`, `marks`, `fixed_peers`, `with_ops`, `roadmap`, `two_slots`, `with_pile`, `add`, `remove`, `text`, `ids_in`, `issue_ids`. Les clés de `@slot` forgées passent toujours par `owner_key`. On ajoute, localement :
  - `fn with_pile_and_children() -> Deck` : `pile` dans `blocks` de `s-constat`, avec `k1` et `k2` dans `children`.

  Tests :
  - `removing_an_element_marks_its_node_without_deleting_it` : on prend le `TreeID` de `b-points`, puis on applique son retrait. `!tree.is_node_deleted(n)`, `marks == [mark_key(r.peer_id())]`, la valeur vaut `true`, et le deck relu n'a plus `b-points`. Rouge : le nœud est supprimé.
  - `undo_of_a_removal_unmarks_the_same_node` : retrait, puis `undo`. `node_of(…, "b-points").1 == n` (même `TreeID`), `marks` est vide, et le deck égale le deck de départ. Rouge : nouveau `TreeID`.
  - `redo_of_a_removal_marks_the_node_again` : retrait, `undo`, `redo`. Même `n`, une marque, `b-points` absent.
  - `removing_an_owner_writes_one_mark` (garde) : sur `with_pile_and_children()`, retirer `pile` augmente `len_ops` d'exactement 1, et `k1`, `k2` n'ont aucune marque (sous-arbre sauté).
  - `read_hides_the_subtree_of_a_marked_owner` : sur `with_pile_and_children()`, on forge `@deleted/9 = true` dans la méta de `pile`, puis commit. La relecture n'a ni `pile` ni `k1` ni `k2`, et aucune `Issue`. Rouge : `pile` reste.
  - `marked_sibling_does_not_give_its_slot` (R2, règle 2) : sous la racine de `s-title` (slide sans élément), on forge dans cet ordre :
    - `y`, marqué, avec `@slot[owner_key(s-title)] = "elements"` ;
    - `x`, avec une carte `@slot` vide ;
    - `z`, avec `@slot[owner_key(s-title)] = "blocks"`.

    `x` n'hérite pas de `elements` (frère marqué) mais de `blocks` (premier frère visible), avec `issue-slot-repaired`. Rouge sur l'ancien code : `y` est lu et donne `elements`.
  - `reapply_with_marked_nodes_writes_nothing` (garde, F1) : après le retrait de `pile`, `apply(&r.deck().0)` laisse `len_ops` inchangé.
  - `readding_a_removed_id_creates_a_new_node` : retrait de `b-points`, puis `apply` du deck de départ (sans `undo`). Le `b-points` relu a un autre `TreeID`. L'ancien nœud est intact (`!is_node_deleted`) et garde la même marque. Un second `apply` n'écrit rien. Rouge : l'ancien nœud est supprimé.
  - `undo_of_a_removal_after_the_id_was_readded_reads_unique_ids` : A retire `b-points` ; B reçoit, puis réajoute un `b-points` de texte « neuf » ; A reçoit, puis annule ; échange. Les deux nœuds vivent. La relecture rend `b-points` (premier dans l'ordre des enfants) et `b-points-2`, sans `Issue`. Le premier `apply` du deck relu écrit le renommage, le second rien. Le nom de chacun est asserté d'après l'ordre des enfants lu brut.
  - `undo_of_an_owner_removal_brings_back_a_child_moved_in_by_the_peer` (`p3_…`) :
    - sur `with_pile_and_children()`, A retire `pile` ; B passe `b-points` dans `pile.children` ; échange ;
    - ni `pile` ni `b-points` ne sont visibles, aucune `Issue` ;
    - A annule, échange : `pile` est dans `blocks`, et ses `children` valent `["k1", "k2", "b-points"]` (ordre de B), chez les deux ;
    - rouge : `pile` est recréé sans `b-points`.
  - `undo_of_an_add_deletes_the_node_and_a_child_moved_in_by_the_peer` (documente la limite R1 ; garde) :
    - A ajoute `pile` ; échange ; B passe `b-points` dans `pile.children` ; échange ;
    - A annule son ajout ; échange ;
    - `pile` et `b-points` sont absents chez les deux, decks égaux, et `tree.is_node_deleted(pile)` est vrai ;
    - commentaire : « Limite documentée (ADR-0006) : défaire un ajout est un `delete` de Loro. »
  - `undo_of_a_move_after_the_peer_undid_the_add_leaves_an_ignored_empty_node` (R1, « Nœud vide ») :
    - `fixed_peers(1, 2)`. B ajoute `e` (texte) dans `blocks` de `s-constat` ; échange ;
    - A passe `e` dans `elements` (cadré) ; B défait son ajout (`tree.delete` de Loro) ; échange ;
    - A défait son déplacement. Loro crée sous la racine de `s-constat` un nœud neuf à méta vide ; échange ;
    - attendu chez les deux : decks égaux, sans `e`, aucune `Issue`. `from_snapshot(snapshot())` réussit, `merge` dans les deux sens réussit, et `apply(deck())` n'écrit rien. Le test vérifie, en lisant l'arbre brut, qu'un nœud à méta vide existe bien : sinon le scénario ne reproduit pas le cas, et le test le dit ;
    - rouge : le nœud vide est relu `issue-unreadable-element`.
  - `concurrent_slot_entries_on_a_forged_node_are_both_kept` (garde t-188 de `@slot`, R3) :
    - deck `with_pile()`. On forge `x` (texte) sous la racine de `s-constat`, **sans** `@slot` ;
    - `fixed_peers` dans les deux ordres, (1, 2) puis (2, 1) ;
    - A relit (`issue-slot-repaired`) et applique le deck relu : il crée `@slot` et écrit l'entrée de la slide. B passe `x` dans `pile.children` : il crée `@slot` et écrit l'entrée de `pile` ;
    - après l'échange : `x` est dans `pile.children` chez les deux, `@slot` porte les deux entrées, et aucune `Issue` ;
    - rouge attendu dans au moins un ordre : avec `map_at`, une des deux cartes ordinaires gagne la clé `@slot` (LWW), et `x` est réparé dans un slot non déclaré.
  - `from_snapshot_with_a_malformed_mark_is_refused` : `Corrupted` pour chacune de ces marques forgées :
    - `@deleted/9 = false` ;
    - `@deleted/abc = true` ;
    - `@deleted/09 = true` ;
    - `@deleted = true` ;
    - `@deletedx = true`.
  - `from_snapshot_with_a_non_mergeable_slot_is_refused` : `Corrupted` pour un `@slot` forgé en `LoroMap` ordinaire (`insert_container`), en valeur `"texte"`, et en `LoroText`.
  - `merge_of_a_peer_with_a_malformed_mark_is_refused_and_keeps_self` : B forge `@deleted/9 = false`. `a.merge(&b)` donne `Corrupted`, et `a` garde son deck.
  - `merge_of_a_peer_with_a_non_mergeable_slot_is_refused_and_keeps_self` : idem, avec un `@slot` en `LoroMap` ordinaire.

  Lancer : rouges par assertion, sauf les gardes nommées comme telles.
- [ ] **Step 4 : implémenter** selon les interfaces, R0 (marques et `@slot`), R1 et R3. **Jalon** : `-E 'test(soft_delete) | test(issue_0652) | test(issue_0694) | test(issue_0747)'` passe.
- [ ] **Step 5 : migrer les aides de test** de la liste **Files**. Le test de `tree_convergence.rs` devient :
  - `move_under_a_concurrently_removed_owner_is_hidden_until_undo` ;
  - même scénario, plus l'annulation de A et un échange. `pile` revient avec `b-points` dans `children` ;
  - commentaire : « ADR-0006 : l'élément est caché avec le propriétaire retiré ; l'annulation du retrait le rend. »
- [ ] **Step 6 : tests hostiles** dans `tests/untrusted.rs` :
  - `from_snapshot_with_many_marks_on_one_node_reads_in_linear_time` : un nœud porte 100 000 clés `@deleted/<n>` = `true`. L'import réussit, le nœud est caché, et la lecture s'exécute dans un fil à pile de 256 Kio.
  - `from_snapshot_with_a_huge_slot_map_reads` : un `@slot` porte 100 000 entrées, sans entrée pour le parent réel. La règle 3 s'applique une fois, et tout tient dans la même pile.
  - Ce sont des gardes de coût : pas de mesure de temps, seulement l'absence de débordement et de panique.
- [ ] **Step 7 : lancer** tout `oku-slides-core`, `tests/untrusted.rs` compris, puis la commande d'avant commit.
  - Lancer aussi **une fois en local** le proptest à 1000 cas : `with_cases(48)` passe temporairement à `with_cases(1000)`, puis `-E 'test(peers_editing_in_parallel_converge)'`. Son `Edit::Undo` exerce désormais l'annulation d'une marque.
  - Noter le résultat dans le rapport, puis **revenir à 48**.
- [ ] **Step 8 : commit.**

  ```text
  feat(replica) : suppression douce des éléments, une marque par pair ; annuler un retrait rend le même nœud avec le déplacement et l'édition du pair (t-747)
  ```

---

## Task 4: un seul arbre pour le deck, slides comprises, `@slot` par `TreeID` (t-746, t-762)

**Ordre imposé.** La cible de test de la lib compile à chaque étape :
1. les squelettes s'ajoutent à côté de l'ancien code, sans le brancher ;
2. les tests nouveaux s'écrivent ;
3. on branche ;
4. on migre les aides ;
5. on retire les constantes mortes (`ELEMENTS`, `ROOT_OWNER`, `Ctx::lists`) quand plus rien ne les importe.

`tree_at` n'est jamais retiré : `deck_tree_at` l'enveloppe, et `read_tests.rs` (l. 8 et 82) l'importe jusqu'à sa migration au Step 6. `Moved` n'apparaît pas dans cette tâche (Task 6).

La preuve que la parade t-263 peut partir est **dans cette tâche** (Step 8). Si elle est rouge à cause de Loro : `BLOCKED` avec le cas minimal, sans repli silencieux.

**Files :**
- Create :
  - `crates/oku-slides-core/src/replica/tree/deck.rs` : sous-module de `tree` (`mod deck;` dans `tree.rs`), ~350 lignes. Doc de module : « L'arbre du deck : slides en racine, appariement d'une slide à l'autre. » En sous-module, il voit les champs privés de `Live` et `Wanted` sans les ouvrir au reste de `replica/` ;
  - `crates/oku-slides-core/src/replica/tree/deck_tree_tests.rs`, déclaré dans `tree/deck.rs` par `#[cfg(test)] #[path = "deck_tree_tests.rs"] mod deck_tree_tests;`. Il voit les items privés de `tree::deck` et les aides `pub(super)` de `tree_tests` (`use super::super::tree_tests::*;`) ;
  - `crates/oku-slides-core/tests/regressions/issue_0746_move_between_slides_keeps_concurrent_edit.rs`, déclaré avant `issue_0747_…` ;
  - `crates/oku-slides-core/tests/regressions/issue_0746_undo_deck_tree_moves_keep_slides_and_elements.rs`.
- Modify :
  - `src/replica/schema.rs` :
    - `Kind::Collection` (l. 42-43) devient `Kind::Slides` ;
    - ajout de `SLIDES` et `SLIDE` ;
    - `Ctx::kind` l. 95, `trees` l. 125-131, `reserved`, `column_kind` l. 204 ;
    - retrait d'`ELEMENTS` (l. 57-58), de `ROOT_OWNER` (l. 63-64) et de `Ctx::lists` (l. 117-123) **au Step 7 seulement** ;
  - `src/replica/tree.rs` :
    - doc de module l. 1-7 ;
    - `any_slot` (R2, règles 3a et 3b) ;
    - `live_index(tree, slide)` ;
    - découpe de `sync_tree` en `mark_elements` et `place_elements` ;
    - `arrange` ;
    - `node_of` (l. 440-464, au Step 5), `pid` en `TreeID` aux l. 202-203 et 344-347 ;
    - `tree_at` reste, générique ; `deck_tree_at(map)` (dans `tree/deck.rs`) l'appelle avec `SLIDES` et rend `MalformedDeckTree { what: "slides are not a tree" }` pour un autre type ;
    - `ordered_children` rend `Children`, et écarte aussi les nœuds vides et les enfants à `@slide` (ces derniers sont refusés à l'entrée : défense) ;
  - `src/replica/write.rs` :
    - imports l. 9-11 ;
    - `sync_map` : l. 126-131 (index calculé pour `Ctx::Deck`) ; l. 136-141 (plus de `lists()`) ; l. 155 et 166-168 (`Kind::Slides`) ; l. 182-186 (plus d'arbre pour `Ctx::Slide`) ;
    - `sync_fields` l. 232 et 247 ;
    - doc de `reorder` l. 311-314 : « Sert aux items de liste et aux identifiants d'un `dataset` ; slides et éléments sont dans l'arbre du deck (`tree/deck.rs`). » Elle n'est **que** dans cette tâche ;
  - `src/replica/read.rs` :
    - imports l. 15-16 ;
    - branche `Kind::Collection` l. 152-155 (et bras explicite `(Kind::Slides, _) => continue`) ;
    - bloc `Ctx::Slide` l. 169-182, retiré ;
    - `read_tree` l. 194-235 : même signature (R8), `pid` = `child.to_string()` au lieu de l'id (l. 221) ;
    - `read_slides` l. 300-315 (liste) : retiré au Step 5, remplacé par `read_deck_tree`, ajouté au Step 2 sous ce nom nouveau ;
  - `src/replica/mod.rs` :
    - doc de module l. 6-9, imports l. 30-38 ;
    - `check_trees` l. 116-132 : R0 complet sur l'arbre du deck ;
    - doc de `guarded` l. 153-158 : l'arbre unique, le contrôle d'entrée, et « les nœuds au-delà de `MAX_SLOT_DEPTH` ne sont ni lus ni écrits » ;
  - `src/canonical.rs` : doc de `unique_ids` l. 65-68 ;
  - `src/replica/ARCHITECTURE.md` (Step 9) ;
  - aides et tests à migrer (Step 6). Recensement complet :
    - `tree_tests.rs` :
      - l. 9 : import `ELEMENTS` ;
      - `slide_map` l. 101-110, `slide_tree` l. 112-118, `forge` l. 127-155, `root_ids` l. 171-176 ;
      - l. 245 : `reconcile_slide_elements_live_in_the_slide_tree` lit `ELEMENTS` ;
      - l. 372 : `root_x` de `nested_duplicate_keeps_the_outer_tree_id` lit les racines ;
      - l. 590-617 : `apply_on_a_forged_deep_chain_leaves_unread_nodes_untouched`. Il compte par `visible_ids` sous le nœud de `s-constat` (201 = 200 + `b-points`) et garde le fil de 256 Kio ;
      - assertions `slots(…)` l. 238, 248, 295, 314, 347 ;
      - tests `@elements` l. 619-671, **retirés** : remplacés dans `deck_tree_tests.rs` ;
      - `empty_param_name_with_elements_round_trips` l. 675-692 ;
      - tests du format l. 719-772 : inchangés ici (Task 8) ;
    - `tree_read_tests.rs` :
      - l. 8 : import `ELEMENTS` ;
      - l. 30-37, `read_element_with_only_another_owner_entry_keeps_that_slot` : la clé `"pile"` devient le `TreeID` du nœud **vivant** `b-points` (`node_of(&r.doc, "s-constat", "b-points").1.to_string()`). C'est le cas 3b : `x` est relu dans `children` de `s-title`, avec `issue-slot-repaired`, comme l'asserte le test. Un second test, `read_element_with_only_an_absent_parent_entry_keeps_that_slot_silently`, prend la clé `"7@1"` (nœud absent, cas 3a) : `children`, et **aucune** `Issue` ;
      - l. 39-48, `read_element_without_any_slot_goes_to_orphans` : déjà réduit à `x` en Task 3 ; il ne change pas ;
      - l. 96-102 : `blocks_slot` et `elements_slot` passent par `owner_key` ;
      - l. 157 et 172 : `children(Root)[0]` devient le premier enfant du nœud de `s` ;
      - l. 178-189 : `read_slide_with_elements_of_another_type_reads_none_and_reports` devient `read_slides_of_another_type_reads_no_slide` ;
    - `read_tests.rs` : `put_params`, `put_nodes`, `put_slide` et `read` (l. 43-102 ; `ELEMENTS` et `ROOT_OWNER` l. 82-83) ;
    - `soft_delete_tests.rs` : ses clés `@slot` passent déjà par `owner_key` (Task 3) ; rien à changer, sinon vérifier ;
    - `mod_tests.rs` :
      - l. 669 : `slot.insert(&prev.to_string(), "children")` ;
      - l. 470-482 : `apply_reorder_marks_intermediate_commits_as_parts` est refait sur des items de liste et déplacé dans `list_tests.rs` (`apply_reorder_of_items_marks_intermediate_commits_as_parts` : on permute les items `a`, `b`, `c` de `f`, avec deux commits `oku-part:` puis le dernier) ;
    - `history_tests.rs` l. 222-238 : `a_reorder_of_several_slides_is_one_version` est réécrit sur place. `len_changes` augmente d'exactement 1, car un réordonnancement de slides n'a plus de commit intermédiaire ;
    - `undo_tests.rs` l. 460 : commentaire « `node_of` exige l'arbre du deck » ;
    - `tests/regressions/issue_0263_undo_reorder_loses_slides.rs` l. 1-4 : le commentaire dit que les slides sont désormais dans l'arbre et que le test reste une garde ;
    - `tests/regressions/issue_0652_undo_remove_vs_peer_move.rs` (cas slide, l. 86-103) ;
  - `tests/regressions/main.rs`.

**Consommateurs recensés** (grep fait) :
- `Kind::Collection` : `schema.rs` l. 43, 95, 204 ; `write.rs` l. 155, 166, 232, 247 ; `read.rs` l. 153.
- `ELEMENTS` : `schema.rs` l. 58, 128, 136 ; `write.rs` l. 9, 185 ; `read.rs` l. 15, 170 ; `mod.rs` l. 38, 127 ; `tree.rs` l. 3, 454 ; tests listés ci-dessus.
- `ROOT_OWNER` : `tree.rs` l. 15, 202, 345 ; `read.rs` l. 15, 173 ; `read_tests.rs` l. 83.
- `"slides"` en `LoroMovableList` : `mod.rs` l. 121, `tree.rs` l. 444, `tree_tests.rs` l. 103, `read_tests.rs` l. 92.
- `split` : il ne sert plus qu'à `sync_list` pour les items (`write.rs` l. 250) et à `reorder` pour les lignes et colonnes d'un `dataset` (`host.rs` l. 138 et 146).
- Hors `oku-slides-core` : aucun accès direct aux conteneurs (grep de `MovableList`, `@elements` et `get_map("deck")`).

**Interfaces :**

`schema.rs` :
```rust
/// Clé de la carte `deck` qui porte l'arbre du deck.
pub(super) const SLIDES: &str = "slides";
/// Clé de la méta d'une racine : `true`, la marque d'une slide.
pub(super) const SLIDE: &str = "@slide";
pub(super) enum Kind {
    Prose, Rich,
    /// Un `LoroTree` : slides en racine, éléments en descendants (ADR-0006).
    Slides,
    /// Éléments d'un slot : nœuds de l'arbre du deck.
    Slot,
    List, Atom, Nested(Ctx), Host,
}
```
- `Ctx::kind` : `(Deck, "slides") => Kind::Slides`.
- `Ctx::trees` : `Deck => &[SLIDES]`, les autres `&[]`.
- `Ctx::reserved(key)` : pour `Slide | Element`, vrai si `key == SLOT || key == SLIDE || key.starts_with(DELETED)`. Pour les autres contextes, faux.

`tree/deck.rs` :
```rust
/// Les slides visibles : racines non marquées, dans l'ordre de Loro (R4).
pub(in crate::replica) fn slide_order(tree: &LoroTree) -> Vec<TreeID>;
/// L'arbre du deck sous `map[SLIDES]`, créé au besoin.
pub(in crate::replica) fn deck_tree_at(map: &LoroMap) -> Result<LoroTree, ReplicaInner>;
/// Une slide voulue et ses éléments voulus (ordre `uorder`).
pub(in crate::replica) struct WantedSlide<'a> { key: Key, obj: &'a Map<String, Value>, elements: Vec<Wanted<'a>> }
/// Les slides voulues du deck JSON ; `SlotsTooDeep` avant toute écriture.
pub(in crate::replica) fn wanted_slides<'a>(registry: &WidgetRegistry, slides: &'a [Value]) -> Result<Vec<WantedSlide<'a>>, ReplicaInner>;
/// Une slide de l'arbre, et l'index de ses éléments.
struct LiveSlide { key: Key, node: TreeID, elements: Vec<Live>, pre: Vec<usize> }
fn live_slides(tree: &LoroTree) -> Vec<LiveSlide>;
/// Le nœud apparié à chaque slide voulue et à chaque élément voulu (R5).
struct Pairing { slides: Vec<Option<TreeID>>, elements: Vec<Vec<Option<TreeID>>> }
fn pair(live: &[LiveSlide], wanted: &[WantedSlide<'_>]) -> Pairing;
impl Writer<'_> {
    /// Amène l'arbre du deck aux slides voulues (R6).
    pub(in crate::replica) fn sync_deck(&self, tree: &LoroTree, wanted: Vec<WantedSlide<'_>>) -> Res;
}
/// Les racines non vides sans `@slide = true` (lecture seule : `issue-unreadable-slide`).
pub(in crate::replica) fn stray_roots(tree: &LoroTree) -> Vec<TreeID>;
#[cfg(test)]
/// Nœuds dont la méta a été lue depuis le dernier `reset_visited` (tests de coût).
pub(in crate::replica) fn visited() -> usize;
#[cfg(test)]
pub(in crate::replica) fn reset_visited();
```
- Ici, la passe 2 ne fait que le déplacement déduit (R5, point 3, second cas). `moves` s'ajoute aux deux signatures en Task 6.
- Le compteur de coût : un `thread_local!` `Cell<usize>` sous `#[cfg(test)]`, incrémenté à chaque méta lue par `ordered_children`, `slide_order`, `stray_roots` et `live_slides`. Hors test, il n'existe pas.
- `pairing.elements.len() == wanted.len()` est un invariant testé (`pair_has_one_entry_per_wanted_slide`).

`tree.rs` :
```rust
/// Nœuds éléments visibles de `slide`, en ordre `uorder`. Bornés par `MAX_SLOT_DEPTH` ; rend aussi leur préordre.
pub(super) fn live_index(tree: &LoroTree, slide: TreeID) -> (Vec<Live>, Vec<usize>);
/// Place les nœuds `list` sous `parent`, dans cet ordre.
/// `None` : à créer. `present(n)` : `n` est déjà dans ce groupe. Une plus longue
/// sous-suite ordonnée ne bouge pas. Rend chaque nœud, et s'il vient d'être créé.
pub(super) fn arrange(tree: &LoroTree, parent: TreeParentId, kids: &mut Vec<TreeID>, list: &[Option<TreeID>], present: &dyn Fn(TreeID) -> bool) -> Result<Vec<(TreeID, bool)>, ReplicaInner>;
impl Writer<'_> {
    /// Temps 1 d'une slide : marque les non voulus. Rend les différés.
    pub(super) fn mark_elements(&self, tree: &LoroTree, live: &[Live], pre: &[usize], wanted: &HashSet<TreeID>) -> Result<Vec<TreeID>, ReplicaInner>;
    /// Temps 2 d'une slide : place ses éléments voulus, appariés par `paired`.
    pub(super) fn place_elements(&self, tree: &LoroTree, slide: TreeID, wanted: &[Wanted<'_>], paired: &[Option<TreeID>]) -> Res;
}
```
- `place` garde son découpage par (parent, slot) et appelle `arrange`. Dans `arrange`, on prend les enfants de `kids` pour lesquels `present` est vrai et qui ont un rang dans `list`. Leurs rangs passent par `lis`, puis on parcourt `list` comme au temps 2 d'ADR-0005.
- Dans `place`, `present(n)` signifie : le slot résolu de `n` est `S`. Le rang d'un nœud vient de `paired`, par la table inverse `HashMap<TreeID, usize>`.
- Un nœud créé reçoit aussitôt `slot_map_at(&meta)`.
- Le `pid` d'un parent élément est `node.to_string()`, à tous les niveaux.

`write.rs` :
- `sync_map`, au début : `let wanted = match ctx { Ctx::Deck => wanted_slides(self.registry, elements(obj.get(SLIDES)…))?, _ => Vec::new() };`.
- `for key in ctx.trees() { deck_tree_at(map)?; }`.
- Clé présente : `Kind::Slides => self.sync_deck(&deck_tree_at(map)?, std::mem::take(&mut wanted))?`. Clé périmée : `Kind::Slides => self.sync_deck(&deck_tree_at(map)?, Vec::new())?` (tout est marqué). `slides` est toujours présent dans un `Deck` sérialisé : un seul appel par `apply`.
- Branche `owner` : `sync_params` seul, aussi pour `Ctx::Slide`.
- `sync_fields` : `Kind::Slides => return CollectionInParamsSnafu.fail()`.

`read.rs` :
```rust
impl Reader {
    /// Les slides de l'arbre du deck, dans l'ordre de `slide_order` (R8).
    fn read_deck_tree(&mut self, tree: &LoroTree, depth: Depth) -> Vec<Value>;
}
```

`mod.rs` : `check_trees(doc)` applique R0 à l'arbre du deck, et `MalformedDeckTreeSnafu { what }` en cas de refus.

- [ ] **Step 1 : régressions rouges.**
  - `issue_0746_move_between_slides_keeps_concurrent_edit.rs`, test `move_between_slides_keeps_the_concurrent_bold` :
    - registre `common::shared()`, deck `roadmap`, deux pairs issus d'un même snapshot ;
    - A retire `b-points` de `s-constat` et l'ajoute en tête de `blocks` de `s-priorites` (`remove_element` puis `add_element`, `index: 0`, dans un même `apply`). `b-points` ne figure que dans ces slides : déplacement déduit ;
    - B met en gras le premier run du `body` de `b-points`, visé par id ; on vérifie le basculement chez B ;
    - après la fusion croisée : même deck chez les deux, aucune `Issue` ;
    - `blocks` de `s-priorites` vaut `["b-points", "c1", "c2", "c3"]`, `b-points` est gras, et `s-constat` ne l'a plus.
  - `issue_0652_undo_remove_vs_peer_move.rs` : renommer `undo_remove_while_peer_moves_and_edits_slide_keeps_both_versions_and_converges` en `undo_remove_while_peer_moves_and_edits_slide_keeps_one_slide_with_the_peer_edit` :
    - ids attendus : `["s-toc", "s-title", "s-ch1", "s-constat", "s-priorites", "s-ch2", "s-libre", "s-fin"]` ;
    - `merged.slides[0].notes` finit par `" (B)"` ;
    - commentaire : « Suppression douce (t-747) dans l'arbre du deck (t-746) : plus de `s-toc-2`. » ;
    - `undo_remove_while_peer_moves_slide_keeps_ids_unique` reste une garde.
  - Lancer : rouges par assertion (gras perdu ; `s-toc-2` présent).
- [ ] **Step 2 : squelettes, non branchés.**
  - `tree/deck.rs` avec les signatures : `sync_deck` rend `Ok(())` sans rien écrire, `slide_order` rend `vec![]`, `pair` rend des `None`.
  - `SLIDES`, `SLIDE` et `Kind::Slides` (renommage de `Collection`, vérifié par le compilateur) ; `Reader::read_deck_tree` (rend `vec![]`), à côté de `read_slides`, qui reste ; `deck_tree_at`, `stray_roots`, `visited`, `reset_visited` ; `mod deck;` dans `tree.rs` et la déclaration de `deck_tree_tests` (fichier vide) dans `tree/deck.rs`.
  - `ELEMENTS`, `ROOT_OWNER`, `Ctx::lists` et `tree_at` restent. L'écriture et la lecture suivent encore l'arbre par slide. Les items nouveaux, inutilisés, n'émettent que des avertissements `dead_code` : seul `clippy -D warnings` les refuserait, et il ne tourne qu'avant le commit.
  - Vérifier : `cargo check -p oku-slides-core --all-targets` passe, et tous les tests existants restent verts.
- [ ] **Step 3 : aides de navigation du nouvel arbre**, dans `tree_tests.rs` (`pub(super)`), lues brutes, pour qu'`owner_key` et les tests de la Task 3 s'en servent aussi :
  - `fn deck_tree(r: &DeckReplica) -> LoroTree` : panique « arbre du deck attendu » si `deck.slides` n'est pas un arbre ;
  - `fn slide_node(r: &DeckReplica, slide: &str) -> TreeID` : racine dont la méta a `id == slide` ;
  - `fn slide_ids(r: &DeckReplica) -> Vec<String>` : `tree.roots()` dans l'ordre, marquées et sans `@slide` exclues ;
  - `marks` et `owner_key` viennent de la Task 3. Le corps d'`owner_key` change au Step 5, avec `node_of` : `""` donne le `TreeID` du nœud de la slide, un id d'élément le `TreeID` de son nœud.
- [ ] **Step 4 : tests unitaires rouges** dans `deck_tree_tests.rs`. Ils échouent par assertion, ou par le `panic!` explicite d'une aide (« arbre du deck attendu ») tant que l'écriture n'est pas branchée.
  - `slides_are_root_nodes_of_the_deck_tree` : sur `roadmap`, `deck.slides` est un `Container::Tree` à 8 racines, dont chaque méta porte `@slide == true`. Le parent de `b-points` est le nœud de `s-constat`, et `@slot(b-points) == { "<TreeID de s-constat>": "blocks" }`.
  - `nested_slot_keys_are_tree_ids` : `with_pile()` plus `k` dans `pile.children`. `@slot(k) == { "<TreeID de pile>": "children" }`.
  - `round_trip_of_the_roadmap_is_identical` : `new`, `snapshot`, puis `from_snapshot` rendent le même deck, sans `Issue`.
  - `apply_move_between_slides_keeps_the_tree_node` :
    - `b-points` passe en tête de `blocks` de `s-priorites` ;
    - même `TreeID`, son parent est le nœud de `s-priorites`, et `@slot` porte les deux clés (ancienne et nouvelle slide) ;
    - puis `b-points` revient dans `s-constat` : même `TreeID`, toujours deux entrées dans `@slot`. Un `apply(deck())` ensuite n'écrit rien.
  - `apply_reorder_of_slides_is_one_change_and_keeps_the_nodes` : on inverse les 8 slides. `len_changes` augmente de **1** exactement, chaque slide garde son `TreeID`, et l'ordre relu est l'inverse exact.
  - `moving_the_first_slide_to_the_end_is_one_move` (garde) : `len_ops` + 1.
  - `removing_a_slide_marks_only_its_node` : retirer `s-constat` augmente `len_ops` d'exactement 1. Son nœud n'est pas supprimé et porte une marque ; `b-points` n'en porte aucune.
  - `undo_of_a_slide_removal_unmarks_the_same_node` : même `TreeID` pour `s-constat` et `b-points`, aucune marque, deck de départ.
  - `reapply_after_a_move_between_slides_writes_nothing` (F1).
  - `same_element_id_in_two_slides_keeps_each_node` (passe 1) : deux slides `a` et `b`, chacune avec un texte `t` de contenu différent. On permute les slides : chaque `t` garde son `TreeID` et son contenu.
  - `element_ids_repeated_across_slides_are_not_paired_between_slides` (R7 de la revue) :
    - `body` est dans `a` et dans `b` ;
    - dans un même `apply`, on retire le `body` de `a` et on ajoute un `body` neuf dans `c` ;
    - le neuf a un **autre** `TreeID`, et l'ancien de `a` est marqué.
  - `element_moved_with_another_kind_is_not_paired_between_slides` : `x` (texte), seul dans `a`, est retiré ; un `x` de `kind` `stack` est ajouté dans `b`. Autre `TreeID`.
  - `element_of_a_removed_slide_is_created_anew_elsewhere` (R5, point 5) : dans un même `apply`, on retire `s-constat` et on ajoute `b-points` (même contenu) en tête de `blocks` de `s-priorites`. Le `b-points` relu a un **autre** `TreeID`, et l'ancien reste sous `s-constat`, marquée.
  - `moved_owner_keeps_its_children_nodes` (R5, point 4) : `pile` (deux enfants `k1`, `k2`) passe de `s-constat` à `s-priorites`. `pile`, `k1` et `k2` gardent leur `TreeID`.
  Les tests de la règle 3 forgent sous `s-title`, qui n'a aucun élément : aucun frère ne fournit la règle 2.
  - `read_rule_three_ignores_keys_that_are_not_canonical_tree_ids` : `x` sous `s-title`, avec `@slot = { "12": "zzz", "+7@1": "zzz", "7@1": "elements" }`. Seule `7@1` est canonique ; elle désigne un nœud absent : cas 3a, `x` est relu dans `elements`, sans `Issue`. Avec `@slot = { "x@1": "blocks" }` seul, aucune clé canonique : `x` va dans `orphans`, avec `issue-slot-repaired`.
  - `read_rule_three_with_a_live_other_parent_reports_the_repair` (3b) : deck `with_pile()`. `x` sous `s-title` porte la seule entrée `{ "<TreeID de pile, vivant dans s-constat>": "children" }`. `x` est relu dans `children` de `s-title`, avec `issue-slot-repaired`.
  - `read_rule_three_with_absent_parents_of_two_slots_reports_the_repair` (3b) : `x` sous `s-title`, avec `{ "7@1": "blocks", "8@1": "elements" }` (deux nœuds absents, deux slots). `x` est relu dans `blocks` (première clé), avec `issue-slot-repaired`.
  - `slide_outside_the_root_is_refused` (R0) : on forge `tree.mov(slide_node("s-title"), node_of("b-points"))`. `from_snapshot` donne `Corrupted`, et `merge` d'un pair qui le fait aussi, `self` intact. Même résultat pour une slide sous une autre slide.
  - `root_without_slide_mark_is_ignored_and_accepted` : on retire `@slide` d'une racine (`s-title`), ou on le met à `"true"` (chaîne). `from_snapshot` et `merge` réussissent ; `s-title` n'est pas relue, avec une `issue-unreadable-slide`. Une racine à méta vide, forgée par `tree.create(Root)`, est ignorée sans `Issue`. Un `apply(deck())` ensuite n'écrit rien et ne touche pas ces racines.
  - `undo_of_a_slide_move_after_the_peer_undid_its_creation_is_accepted` (R0, « Accepté ») :
    - `fixed_peers(1, 2)`. B ajoute la slide `s-neuve` ; échange ;
    - A la déplace en tête ; B défait son ajout ; échange ;
    - A défait son déplacement : Loro crée une racine à méta vide ; échange ;
    - attendu chez les deux : `from_snapshot(snapshot())` et `merge` dans les deux sens réussissent, decks égaux, sans `s-neuve`, aucune `Issue`, et `apply(deck())` n'écrit rien ;
    - le test vérifie, en lisant l'arbre brut, qu'une racine à méta vide existe bien.
  - `slides_of_another_type_are_refused` : `deck.slides` forgé en `LoroMovableList`. `from_snapshot` donne `Corrupted`, `merge` aussi, et `apply` donne `Corrupted` sans écraser la liste forgée.
  - `too_deep_slide_leaves_the_whole_deck_untouched` (atomicité du temps 0) : un seul `apply` renomme `s-title`, réordonne deux slides, retire `s-fin` et rend `s-constat` trop profonde (`canvas_chain(MAX_SLOT_DEPTH + 1)`). Il rend `SlotsTooDeep`, et `len_ops` et le deck relu sont inchangés.
  - `pair_has_one_entry_per_wanted_slide` : sur un deck où des slides sont ajoutées et retirées, `pairing.elements.len() == wanted.len()`, et chaque `elements[i].len() == wanted[i].elements.len()`.
  - `concurrent_crossed_nesting_across_slides_converges_without_cycle` :
    - `fixed_peers` (1, 2), puis (2, 1) ; `pile-a` dans `blocks` de `s-constat`, `pile-b` dans `blocks` de `s-priorites` ;
    - A met `pile-b` dans `pile-a.children` ; B l'inverse. Les deux `apply` ont la même forme (un commit, mêmes ops), donc le même lamport ;
    - attendu dans les deux ordres : le `mov` du pair d'id **1** gagne (à lamport égal, le plus petit pair d'abord, `p7_…`). Le perdant reste visible dans `blocks` de sa slide, grâce à son entrée `@slot` intacte ;
    - mêmes decks, aucune `Issue`, puis `apply(deck())` sans écriture.
  - `marked_slides_cost_stays_linear` (R9), **par compteur**, sans mesure de temps :
    - un deck de 10 001 slides, dont on retire toutes sauf une : 10 000 racines marquées ;
    - `reset_visited()`, puis `deck()` : `visited() <= 2 * nœuds`, où `nœuds` est le nombre de nœuds de l'arbre lu brut ;
    - `reset_visited()`, puis `apply(&deck().0)` : `visited() <= 4 * nœuds`, et `len_ops` est inchangé ;
    - même mesure avec 10 000 éléments marqués sous une seule slide.
    Un coût quadratique donnerait de l'ordre de 10⁸ visites.
  - `deep_element_chain_in_the_deck_tree_reads_and_applies_in_a_small_stack` : chaîne forgée de 10 000 `stack` sous `s-constat`, puis `from_snapshot`, `deck()` et `apply(deck())` dans un fil à pile de 256 Kio. Pas de débordement, nœuds profonds intacts.
- [ ] **Step 5 : implémenter et brancher** `schema.rs`, `tree.rs`, `tree/deck.rs`, `write.rs`, `read.rs`, `mod.rs` et `error/mod.rs`, selon les interfaces et R0 à R8.
  - L'ancien code (lecture et écriture de `@elements`, `Reader::read_slides` de la liste) est retiré.
  - `node_of` (`tree.rs`, `#[cfg(test)]`) et le corps d'`owner_key` passent au nouvel arbre dans ce même pas.
  - Les constantes `ELEMENTS`, `ROOT_OWNER`, `Ctx::lists` et `tree_at` restent, encore importées par les tests.
  - **Jalon** : `-E 'test(deck_tree) | test(issue_0746) | test(issue_0652)'` passe. Les tests aux aides non migrées (`slide_map`, `slide_tree`, `forge`, `put_slide`), y compris `soft_delete_tests`, échouent par panique d'aide ou par assertion, mais compilent.

  **Contrôle de compilation, pas par pas** (imports réels relevés par grep) :
  - Step 2 : rien n'est retiré. Les seuls ajouts sont des items nouveaux et un renommage (`Kind::Collection`, utilisé seulement par `schema.rs`, `write.rs` et `read.rs`).
  - Step 5 : sont retirés `Reader::read_slides` (appelé seulement par `read.rs` l. 154) et le bloc `ELEMENTS` de `read.rs` (l. 169-182). `ElementsNotATree` est déjà renommé en Task 3. Les tests importent encore `ELEMENTS` (`tree_tests.rs` l. 9, `tree_read_tests.rs` l. 8), `tree_at` et `ROOT_OWNER` (`read_tests.rs` l. 8, 82-83) : ils restent définis.
  - Step 6 : les imports de test passent à `SLIDES`, `deck_tree_at`, `owner_key`.
  - Step 7 : `git grep` vide, puis retrait.
- [ ] **Step 6 : migrer les aides et tests** du recensement (**Files**) :
  - `tree_tests.rs` :
    - `slide_map(r, slide)` rend la méta du nœud de la slide ;
    - `slide_tree(r, _)` rend l'arbre du deck ;
    - `root_ids(r, slide)` lit les enfants du nœud de la slide ;
    - `forge` : `parent: None` donne le nœud de la slide. Une clé de `slot` vaut `""` → `TreeID` de la slide, un id d'élément existant → son `TreeID`, toute autre chaîne est gardée telle quelle ;
    - `slots(…) == json!({ "": … })` passe par `owner_key`.
  - `read_tests.rs` :
    - `read` crée l'arbre `deck_tree_at(&map)`. `put_slide(tree, slide)` fait `create(Root)`, pose `@slide = true`, puis `put_params(&meta, slide, tree, Node(n), &n.to_string())` ;
    - `put_nodes` prend le `TreeID` du parent comme clé de `@slot`, par `ensure_mergeable_map` ;
    - `put_slide_matches_the_writer` compare aussi la forme brute des deux arbres : clés de méta de chaque nœud, `@slot` en JSON avec les `TreeID` remplacés par l'id de leur nœud.
  - Le reste comme listé.
- [ ] **Step 7 : retirer** `ELEMENTS`, `ROOT_OWNER` et `Ctx::lists`. Avant : `git grep -n "ELEMENTS\|ROOT_OWNER\|\.lists()" crates/oku-slides-core/src/replica` ne rend que leurs définitions. Attention : `validate.rs` l. 234 appelle `params.lists()`, une autre méthode. Lancer tout `oku-slides-core`, `soft_delete_tests` compris.
- [ ] **Step 8 : preuve t-263 (condition de la tâche).** Dans `undo_tests.rs`, avec `assert_undo_redo_exact` :
  - `undo_redo_of_a_slide_reversal_restores_exactly` : 8 slides, dans l'ordre inverse ;
  - `undo_redo_of_a_move_between_slides_restores_exactly` ;
  - `undo_redo_of_a_slide_removal_an_element_move_and_a_reorder_in_one_apply_restores_exactly` : retrait de `s-ch1`, passage de `b-points` dans `s-priorites` et `s-libre` en tête ;
  - `undo_redo_of_adding_an_owner_with_children_restores_exactly` : ajout de `pile` (enfants `k1`, `k2`) à `s-constat`, annuler, rétablir. Le deck est exact et **aucune `Issue`** n'apparaît. Si « rétablir » recrée les nœuds sous d'autres `TreeID`, la règle 3a rend `k1`, `k2` à `children` sans `Issue`, et le premier `apply` écrit l'entrée `@slot` du nouveau parent. Le test l'asserte : `issues.is_empty()`, et le second `apply` n'écrit rien ;
  - `undo_redo_of_adding_a_slide_with_elements_restores_exactly` : idem pour une slide neuve à deux éléments ;
  - `undo_redo_of_moving_an_element_into_a_new_owner_restores_exactly` :
    - dans un même `apply`, A ajoute `pile` à `s-constat` et y passe `b-points` (`children`). Annuler, rétablir ;
    - après « rétablir », `b-points` porte deux entrées : celle de `s-constat` (nœud vivant mais pas parent : entrée périmée, ignorée par 3a) et celle de l'ancien `pile` (nœud supprimé, `children`). C'est le cas 3a ;
    - attendu : deck exact (`b-points` dans `pile.children`), **aucune `Issue`**, et le second `apply` n'écrit rien.

  Après chaque « rétablir » de ces tests, on asserte aussi que `from_snapshot(snapshot())` réussit et qu'une fusion avec un pair issu du snapshot de départ est acceptée : R0 ne refuse rien de ce qu'annuler et rétablir produisent.

  Régression `issue_0746_undo_deck_tree_moves_keep_slides_and_elements.rs`, sur le modèle de `issue_0694_undo_tree_moves_keep_elements.rs` (même `Lcg`, graines `[1, 7, 42, 2026]`, 6 tours) :
  - départ : `roadmap` + `pile` dans `blocks` de `s-constat` ;
  - une édition au hasard parmi :
    - permutation des slides ;
    - passage d'un élément d'une slide de contenu (`s-constat`, `s-priorites`, `s-libre`) à une autre, cadre ajouté ou retiré selon le slot ;
    - retrait d'une slide autre que `s-constat` ;
    - retrait d'un élément non `pile`, puis ajout d'une copie au nouvel id `e-copie-{round}` ;
    - ajout d'un propriétaire `pile-{round}` à deux enfants ;
  - on annule tout, puis on rétablit tout. Chaque état relu est égal à l'état appliqué, ids conservés, aucune `Issue`.

  Si l'un de ces tests est rouge à cause de Loro : **s'arrêter**, réduire le cas, rapporter `BLOCKED`. Ne pas remettre `split` pour l'arbre sans décision.
- [ ] **Step 9 : `replica/ARCHITECTURE.md`**, remplacé **entièrement** par le texte ci-dessous (50 lignes). La ligne « Format » garde `meta.format = 2` jusqu'à la Task 8. Vérifier `wc -l` = 50.

  ```markdown
  # replica

  Le deck dans un `LoroDoc` (CRDT) : historique complet, fusion, annuler.
  `DeckReplica` fait foi ; le `Deck` n'en est qu'une vue.

  ## Modules

  - `mod.rs` : `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`, annuler).
  - `schema` : catégorie (`Kind`) de chaque champ, selon son contexte (`Ctx`) et le registre.
  - `write` : `Writer`, réconciliation des conteneurs vers un `Deck` JSON.
  - `tree` : éléments, placement par (parent, slot) ; `tree/deck` : slides, appariement.
  - `read` : lecture totale, sans registre ; l'illisible est écarté et signalé (`Issue`).
  - `host` : option `host(chart|table)` ; `rich` : paragraphes dans un `LoroText` à marques.
  - `undo` : étapes d'annulation, groupes, clés de fusion ; `authors` : pairs, `MergeReport`.
  - `history` : une `Version` par édition, aperçu d'une version passée.

  ## Catégories (`Kind`)

  - `Prose` : `LoroText`, fusion au caractère. `Rich` : `LoroMap { rich: LoroText }` à marques.
  - `Slides` : un seul `LoroTree` (`slides`) ; racines = slides (méta = la slide, `@slide`),
    descendants = éléments ; ordre fractionnaire (ADR-0006).
  - `Slot` : nœuds éléments ; méta = l'élément sans ses slots, `@slot` = carte fusionnable
    { `TreeID` du parent (`counter@peer`) → slot }.
  - `List` : `LoroMap { list: LoroMovableList }`, items d'un paramètre `list`.
  - `Host` : `LoroMap` ; `dataset` en `{columns, rows}`, `rows` de `{id, cells}`, `cells` par
    colonne ; autres clés LWW. Un `dataset` d'une autre forme s'écrit opaque, sans perte.
  - `Atom` : valeur JSON entière, LWW. `Nested` : objet en `LoroMap`, sinon LWW.

  ## Règles

  - **Diff du JSON du deck** : `apply` amène chaque conteneur à l'état du `Deck` sérialisé ;
    rien n'est écrit quand rien ne change (arbre : ordre comparé par parent et slot).
  - **Suppression douce** : un nœud retiré reçoit `@deleted/<pair>` (jamais `tree.delete`) et se
    cache avec son sous-arbre ; annuler retire la marque ; un id réajouté crée un nœud neuf.
  - **Entrée stricte** : `@slide` hors racine, marque ou `@slot` mal formés : refus (jamais produits).
  - **Lecture sans registre** : le type se lit dans le conteneur ; `{columns, rows}` ne se
    lit que sous `dataset` d'un paramètre à valeur de carte ; clés triées, comme la forme canonique.
  - **Réparation à la relecture** : colonne en double, et ligne en double aux mêmes
    cellules, écartées ; cellule absente lue `null` ; autre identifiant en double ou invalide
    remplacé (`r{i}`/`c{j}`, puis `-{n}`) ; bornes de 10 000 lignes, 1 000 colonnes et
    1 000 000 cellules (`issue-data-truncated`), au-delà `Opaque` ; profondeur bornée par
    `MAX_DEPTH`, éléments par `MAX_SLOT_DEPTH` (écriture comprise). Slides, éléments d'une
    slide ou items au même id (t-652) : copie exacte d'un voisin écartée, autre occurrence
    renommée `{id}-{n}` ; id vide laissé. Sans `@slot` pour son parent : slot d'un frère,
    sinon slot inscrit (silencieux si les parents disparus s'accordent), sinon `orphans`
    (`issue-slot-repaired`) ; jamais écarté. Nœud vide (né d'une annulation) : ignoré.
  - **Parade t-188** : conteneurs déclarés et arbre du deck créés d'avance ; `@slot` fusionnable.
  - **Parade t-263** : un commit par `mov` d'une liste (items, `dataset`) ; l'arbre, sans.
  - **Format** : `meta.format = 2`, vérifié à l'import et à la fusion ; changer ce format
    impose de changer `SOURCES_VERSION`.
  ```
- [ ] **Step 10 : lancer** tout `oku-slides-core` : `tests/untrusted.rs`, `issue_0263_undo_reorder_loses_slides` (garde), `issue_0694_*`. Puis la commande d'avant commit. Lancer **une fois en local** le proptest à 1000 cas (comme en Task 3), noter le résultat, revenir à 48. Si `wc -l src/replica/tree.rs` > 500, sortir `lis`, `arrange` et `place` dans `src/replica/tree/place.rs`.
- [ ] **Step 11 : commit.**

  ```text
  feat(replica) : le deck dans un seul LoroTree, slides en racine ; un déplacement d'élément entre slides garde les éditions concurrentes (t-746)
  ```

---

## Task 5: annuler, historique et aperçu dans l'arbre du deck (t-746, t-747, t-763)

La preuve t-263 est déjà faite (Task 4, Step 8). Cette tâche ajoute des gardes d'annulation entre pairs, l'historique et l'aperçu, et met les docs à jour.

**Files :**
- Modify :
  - `src/replica/undo_tests.rs` (ajouts après les tests de la Task 4) ;
  - `src/replica/history_tests.rs` ;
  - docs :
    - `src/replica/undo.rs` (doc de module, l. 4-7) ;
    - `src/replica/history.rs` (doc de module, l. 8-12 ; commentaire l. 79-80) ;
    - `src/replica/mod.rs` (doc de `restore`, l. 426-428 ; doc de `PART`, l. 49-50).

**Consommateurs recensés :** `split` et `PART` dans `mod.rs` (`apply`, `apply_silent`, `restore`, `part_message`), `history.rs` (`build`), `write.rs` (`reorder`) et `host.rs` (`sync_ids`). Pas de changement de code.

- [ ] **Step 1 : gardes.**
  - `undo_tests.rs` :
    - `undo_move_between_slides_keeps_a_remote_edit` :
      - A passe `b-points` dans `s-priorites`, B le met en gras ;
      - A fusionne B puis annule : `b-points` revient dans `s-constat`, **gras** ;
      - A rétablit : il est dans `s-priorites`, gras ;
      - B fusionne A : decks égaux ;
    - `undo_of_a_slide_removal_keeps_a_remote_move_of_one_of_its_elements` :
      - A retire `s-constat` ; B passe `b-points` dans `s-priorites`. Après l'échange, `b-points` est dans `s-priorites`, car le `mov` de B l'a sorti de la slide retirée ;
      - A annule, échange : `s-constat` revient, sans `b-points`, et `b-points` reste dans `s-priorites` chez les deux.
  - `history_tests.rs` :
    - `a_move_between_slides_is_one_version` : `history()` gagne exactement une version, et `len_changes` augmente de 1 ;
    - `deck_at_before_a_slide_removal_shows_the_slide` ;
    - `restore_before_a_slide_removal_brings_it_back_as_a_new_node` : `restore_and_undo(&mut b, &h, 0)`. Le deck restauré est égal à la version, la slide a un nouveau `TreeID`, et l'ancien nœud garde sa marque (R5, point 6 : jamais ranimée par son id).
  - Le réordonnancement de slides en une version est déjà couvert (Task 4, `history_tests.rs` réécrit) : pas de doublon.
- [ ] **Step 2 : docs.**
  - `undo.rs` l. 4-7 : « Loro 1.16.2 défait mal un item qui porte plusieurs `mov` sur une `LoroMovableList` de conteneurs (t-263). La réconciliation committe donc avant chaque `mov` d'une liste (items, lignes et colonnes d'un `dataset`). L'arbre du deck n'en a pas besoin (ADR-0006). »
  - `history.rs` l. 8-12 : « une édition qui déplace des items de liste ou des lignes de données fait un commit par `mov` (t-263) … Les `mov` de l'arbre du deck n'ajoutent aucun commit (ADR-0006). » Le commentaire des l. 79-80 reste, s'il est encore juste.
  - `mod.rs` l. 426-428 (`restore`) : « Pas de `revert_to` : sur une `LoroMovableList` de conteneurs (items de liste), il perd des items ou échoue (`ContainerDeleted`) dès qu'un déplacement suit la version visée. »
  - `mod.rs` l. 49-50 (`PART`) : « … (avant un `mov` de liste) … ».
- [ ] **Step 3 : lancer** `-E 'test(undo_) | test(history) | test(restore) | test(deck_at)'`, puis la commande d'avant commit.
- [ ] **Step 4 : commit.**

  ```text
  test(replica) : annuler entre pairs, historique et aperçu dans l'arbre du deck (t-746, t-747)
  ```

---

## Task 6: op `move_element`, cœur et agent (t-777)

Un déplacement explicite : la réconciliation apparie le nœud déplacé à coup sûr, même si son id existe dans d'autres slides (R5, déplacement annoncé). L'agent l'obtient par l'outil `apply`, dont le schéma vient des ops (`ops_schema`).

**Files :**
- Create :
  - `crates/oku-slides-core/src/ops/moving.rs` (~120 lignes ; doc : « L'op `move_element` : un élément change de slot, de slide ou de place. ») ;
  - `crates/oku-slides-core/tests/move_element.rs`.
- Modify :
  - `crates/oku-slides-core/src/ops/mod.rs` :
    - variante `Op::MoveElement`, après `RemoveElement` (l. 180-183) ;
    - son bras dans `apply` (près de l. 580) ;
    - `apply_all` (l. 778-788) délègue à `apply_all_moving` ;
  - `crates/oku-slides-core/src/lib.rs` (l. 49-51 : `apply_all_moving`, `Moved`) ;
  - `crates/oku-slides-core/src/replica/mod.rs` : `apply_moving` ; `apply` délègue avec `&[]`. `reconcile_doc` (l. 327-342) prend `moves: &[Moved]` et le donne au `Writer`. `apply_silent` (l. 315-324) et l'essai de `restore` (l. 438) lui passent `&[]` ;
  - `crates/oku-slides-core/src/replica/tree/deck.rs` : `pair(live, wanted, moves)` et `sync_deck(tree, wanted, moves)` prennent ici leur paramètre `moves` ;
  - `crates/oku-slides-core/src/replica/write.rs` : `Writer` porte `moves: &'a [Moved]`, passés à `sync_deck` ;
  - `crates/oku-slides-core/src/sources.rs` : `SourceBundle::apply_moving` (près de `apply`, l. 275-288), et `apply` délègue avec `&[]` ;
  - `crates/oku-slides-session/src/deck_widgets.rs` : `apply` (l. 189-240) appelle `apply_all_moving`, et `commit` (l. 250-256) prend `moves: &[Moved]` et appelle `sources.apply_moving` ;
  - `crates/oku-slides-mcp/src/tools.rs` l. 235 : description de l'outil `apply` ;
  - `crates/oku-slides-mcp/tests/edit.rs` (l. 47-56 et nouveau test) ;
  - `crates/oku-slides-core/ARCHITECTURE.md` : rien, la règle « Une nouvelle opération : `Op` + `ops_schema` » (l. 45) couvre le cas.

**Interfaces :**
```rust
// ops/mod.rs
/// Déplace un élément, avec ses enfants, vers un slot d'une slide ou d'un élément.
/// Même slide admise. Entrer dans un slot `flow` retire `frame` et `cell`.
MoveElement {
    slide: String,
    element: String,
    to: SlotRef,
    /// Position dans le slot cible ; absent = à la fin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    index: Option<usize>,
},

// ops/moving.rs
/// Un élément passé d'une slide à une autre par `move_element`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved { pub element: String, pub from: String, pub to: String }
/// Applique `move_element` : retrait, contrôles d'`add_element`, insertion.
pub(super) fn move_element(deck: &mut Deck, registry: &WidgetRegistry, slide: String, element: String, to: SlotRef, index: Option<usize>) -> Result<(), OpError>;

// ops/mod.rs
/// Comme `apply_all`, et rend les déplacements entre slides de la transaction.
pub fn apply_all_moving(deck: &mut Deck, ops: Vec<Op>, registry: &WidgetRegistry) -> Result<Vec<Moved>, TransactionError>;

// replica/mod.rs
/// Comme `apply`, en gardant le nœud des éléments de `moves` (ADR-0006).
pub fn apply_moving(&mut self, deck: &Deck, label: &str, merge: Option<&str>, moves: &[Moved]) -> Result<(), ReplicaError>;

// sources.rs
pub fn apply_moving(&mut self, deck: Deck, label: &str, merge: Option<&str>, moves: &[Moved]) -> Result<(), SourcesError>;
```
- `move_element` travaille sur une copie de la slide source :
  1. retire l'élément et son sous-arbre (`remove_from`), sinon `ElementNotFound` ;
  2. applique au sous-arbre les contrôles d'`add_element` (l. 717-772) : slot déclaré, `accepts`, catégorie, `check_ids` dans la slide cible (sans l'élément retiré si c'est la même slide), `check_depth` et `check_params`. Pour cela, `add_element` est découpé en `check_add` + `insert_at`, réutilisés ;
  3. si le slot cible est en mode `Flow` (`containing_slot` / `SlotMode::Flow`), retire `frame` et `cell` ;
  4. insère.

  Le propriétaire cible introuvable après retrait (déplacement dans son propre sous-arbre) donne `ElementNotFound` : pas de variante nouvelle, pas de clé i18n.
- `Op::apply` rend `()` : `apply_all_moving` lit le `Moved` dans l'op (`Op::MoveElement { slide, element, to, .. }`, avec `to.owner.slide`) **avant** de l'appliquer, et ne le garde que si l'op réussit.
- `apply_all_moving` compose les déplacements d'un même élément dans la transaction : `(e, A→B)` puis `(e, B→C)` donnent `(e, A→C)`. Un retour à la slide d'origine n'est pas un déplacement entre slides. Un élément retiré plus loin dans la transaction perd son entrée.
- `pair` : une entrée de `moves` qui ne se résout pas (slide absente, élément introuvable, `kind` différent) est ignorée, et le déplacement déduit s'applique.

- [ ] **Step 1 : tests rouges** dans `tests/move_element.rs` :
  - `move_element_between_slides_keeps_the_id_and_the_subtree` : `pile` (avec `k1`, `k2`) passe de `s-constat` à `blocks` de `s-priorites`, en position 0. Les ids sont gardés, et la source ne l'a plus.
  - `move_element_within_a_slide_reorders` : `c3` passe en tête de `blocks` de `s-priorites`.
  - `move_element_into_a_flow_slot_drops_frame_and_cell` : `e-texte` (`s-libre.elements`) passe dans `blocks` de `s-constat` : plus de `frame` ni de `cell`.
  - `move_element_into_its_own_subtree_is_refused` : `pile` dans `pile.children` donne `ElementNotFound`, et le deck est inchangé (atomique).
  - `move_element_onto_a_taken_id_is_refused` : la slide cible a déjà l'id ; `DuplicateId`.
  - `move_element_to_an_unknown_slot_is_refused` : `UnknownParam` ; un slot qui refuse le `kind` donne `SlotRejects`.
  - `apply_all_moving_composes_moves_of_one_element` : A→B puis B→C donnent `[Moved { e, A, C }]` ; A→B puis B→A ne donnent rien ; un déplacement suivi d'un `remove_element` ne donne rien.
  - `move_element_from_or_to_an_unknown_slide_is_refused` : slide source inconnue, puis slide cible inconnue ; `SlideNotFound`, deck inchangé.
  - `move_element_too_deep_is_refused` : déplacer une chaîne de `MAX_SLOT_DEPTH - 1` piles sous un élément déjà imbriqué ; le refus vient de `check_depth`, deck inchangé.
  - `move_element_to_an_index_out_of_range_is_refused` : `index` supérieur à la longueur du slot cible ; `IndexOutOfRange`, deck inchangé.
  - `ops_schema_lists_move_element` : le schéma contient `"move_element"`.
- [ ] **Step 2 : tests rouges du replica** (`deck_tree_tests.rs`) :
  - `announced_move_keeps_the_node_even_with_a_repeated_id` : `title` est un texte présent dans `a`, `b` et `c`. `apply_all_moving` d'un `move_element` de `title` de `a` vers `d` (qui n'en a pas), puis `apply_moving`. Le nœud garde son `TreeID`. Contre-épreuve : le même deck appliqué par `apply` (sans `moves`) crée un nœud neuf (R7).
  - `stale_move_hint_is_ignored` : `moves` nomme une slide absente. Même résultat qu'`apply`, sans erreur.
- [ ] **Step 3 : régression session et agent.**
  - `crates/oku-slides-mcp/tests/edit.rs` :
    - `apply_schema_accepts_every_op_kind_in_tool_list` : ajouter `"move_element"` à la liste ;
    - `apply_move_element_through_mcp_keeps_the_concurrent_edit` : par l'outil `apply`, l'agent déplace un élément (id répété dans le deck de test) vers une autre slide. Une seconde session, issue du même snapshot et qui l'a mis en gras, fusionne ensuite. L'élément est déplacé et gras. Si le harnais MCP ne permet pas la seconde session, asserter au moins que le `TreeID` est gardé, par `SourceBundle`, dans un test de `oku-slides-session`.
  - `crates/oku-slides-core/tests/regressions/issue_0746_move_between_slides_keeps_concurrent_edit.rs` : ajouter `move_element_with_a_repeated_id_keeps_the_concurrent_bold`, comme le test du Step 1 de la Task 4, avec un id présent dans trois slides et un `move_element`.
- [ ] **Step 4 : implémenter** selon les interfaces.
  - La description de l'outil `apply` (`tools.rs` l. 235) gagne cette phrase après `add_element` : « move_element moves an element with its children to a slot of the same or another slide (slide, element, to {owner, param}, index?), keeping its id and any concurrent edit; prefer it to remove_element + add_element. »
  - Le texte d'`agent-instructions-widgets` (« jamais en réinsérant ou en déplaçant ») reste juste : pas de changement i18n.
- [ ] **Step 5 : lancer** `-E 'test(move_element) | test(announced_move) | test(stale_move) | test(issue_0746)'` sur le core, `-p oku-slides-mcp -E 'test(apply)'`, puis la commande d'avant commit.
- [ ] **Step 6 : commit.**

  ```text
  feat(ops) : op move_element, exposée à l'agent ; la réconciliation garde le nœud déplacé entre slides (t-777)
  ```

---

## Task 7: couper-coller qui déplace un élément, côté UI (t-778)

**Choix de comportement**, à confirmer par l'utilisateur avant d'exécuter cette tâche. Il suit les tableurs.
- Aujourd'hui, ⌘X retire l'élément tout de suite (`remove_element`, un `apply`). Après un retrait, aucun collage ne peut garder le nœud, car on ne ranime jamais un nœud par son id (R5).
- Désormais, ⌘X **marque** l'élément « à déplacer » sans le retirer : contour en pointillés et opacité réduite.
- ⌘V, sur cette slide ou une autre, envoie un `move_element` (une étape d'annulation, « Déplacer »), puis le presse-papiers redevient une copie de l'élément déplacé.
- Échap, un nouveau ⌘C ou ⌘X, ou la disparition de l'élément, abandonne la coupe.
- Si l'hôte refuse le déplacement (élément retiré entre-temps par l'agent ou un pair), le collage retombe sur une copie aux ids renouvelés (`pasteOps`), comme aujourd'hui.
- Le collage d'une copie (⌘C puis ⌘V) ne change pas.

**Files :**
- Modify :
  - `ui/src/deck.ts` : `moveOps`, à côté de `pasteOps` (l. 242-264) ;
  - `ui/src/deck.test.ts` (401 lignes) : `describe("moveOps")` ;
  - `ui/src/App.tsx` (l. 196-240 : `clip` l. 200, `copy`, `cut` l. 213-217, `paste` l. 218-232 ; l. 492 : Échap ; l. 740 : prop de `Stage`) ;
  - `ui/src/Stage.tsx` (prop `cut: string | null`, classe sur la boîte de l'élément) et `ui/src/Stage.render.test.tsx` ;
  - la feuille de style de `Stage` (classe `cut`) ;
  - `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 fichiers : `ui-undo-cut` retirée ; `ui-undo-move` existe déjà).

**Interfaces :**
```ts
/** Ops qui déplacent `source` (de la slide `from`) sur la slide de `view`. Même placement que `pasteOps`, ids gardés. */
export function moveOps(view: SlideView, source: Element, from: string): { id: string; ops: Op[] };
```
- Slot cible et placement comme `pasteOps` : `blocks` si la slide en a, ou sans grille ni cadre ; sinon `elements`.
- Ops :
  - `[{ op: "move_element", slide: from, element: source.id, to: { owner: { slide: view.slide.id }, param } }]` ;
  - puis, pour `elements` : `set_cell` vers `freeCell(…)` sur une grille, ou rien hors grille (le cadre est gardé, sans décalage) ;
  - le slot `blocks` retire `frame` et `cell` côté cœur.
- `clip` : `useRef<{ element: Element; text: string; cut?: { slide: string } } | null>`.
- i18n : `ui-undo-move` existe déjà (fr « Déplacer », en « Move ») ; `ui-undo-cut` est retirée.

- [ ] **Step 1 : tests rouges.**
  - `deck.test.ts` :
    - `moveOps keeps the ids and targets blocks on a block slide` ;
    - `moveOps places on a free cell on a grid slide` ;
    - `moveOps keeps the frame on a free slide without grid` ;
    - `moveOps within the same slide moves, never copies`.
  - `Stage.render.test.tsx` : `marks the cut element` (la boîte de l'élément `cut` porte la classe).
  - Lancer `cd ui && bun run test` : rouges.
- [ ] **Step 2 : implémenter** `moveOps`, la prop `cut` de `Stage`, la classe, et dans `App.tsx` :
  - `cut` : copie, puis `clip.current.cut = { slide }`, puis `setCut(id)`. Plus d'`apply` ;
  - `paste` : si `clip.current.cut`, alors `apply(t("ui-undo-move"), moveOps(…).ops)`. Si l'hôte refuse, c'est `pasteOps`, seulement si l'élément n'est plus sur la slide `from` (`copyAfterRefusedMove`, revue finale) ; sinon l'erreur de l'hôte reste seule. Ensuite `cut` est vidé et l'élément sélectionné ;
  - Échap, `copy`, ou un `cut` sur un autre élément vident la coupe.
- [ ] **Step 3 : i18n**, 20 fichiers `ui.ftl`.
- [ ] **Step 4 : lancer** `cd ui && bun run test && bunx --bun tsc -b`, puis la commande d'avant commit (i18n compris).
- [ ] **Step 5 : commit.**

  ```text
  feat(ui) : couper puis coller déplace l'élément (move_element), sur la slide ou une autre, sans perdre ses éditions (t-778)
  ```

---

## Task 8: format v9 des sources et format 3 du replica (t-746, t-764)

Sans changement de format, un `deck.loro` v8 relu hors zip (reprise de session) serait refusé comme `Corrupted` par `check_trees`, et non comme une autre version. Avec le format 3, il est refusé par `check_format` (`UnsupportedVersion`), qui passe avant `check_trees` (`mod.rs` l. 215-216 et 463-464). La session le met alors de côté (`.failed-*`) et avertit l'utilisateur (`warn!("reprise impossible")` de `Session::resume`, comme pour le v7 en t-694).

**Files :**
- Modify :
  - `crates/oku-slides-core/src/replica/mod.rs` : `REPLICA_FORMAT = 3` et sa doc, l. 46-47 : « 3 : le deck en un seul `LoroTree`, suppression douce (ADR-0006). Le changer impose de changer `SOURCES_VERSION`. » ;
  - `crates/oku-slides-core/src/replica/ARCHITECTURE.md` : ligne « Format », `meta.format = 3` (sur place, 50 lignes) ;
  - `crates/oku-slides-core/src/sources.rs` : `SOURCES_VERSION = 9` (l. 46) ; doc l. 1 « version 8 » → « version 9 » ; l. 16 « 2 à 7 » → « 2 à 8 » ;
  - `crates/oku-slides-core/src/sources_tests.rs` : `MANIFEST` l. 27 en version 9 ; doc de `sources_v7_zip_is_refused` l. 850 : « Sources v7 : refusées. » ; nouveau test ;
  - `crates/oku-slides-core/tests/zip_limits.rs` l. 56 : version 9 ;
  - `crates/oku-slides-core/src/replica/tree_tests.rs`, tests du format, l. 719-772 :
    - `new_replica_carries_the_replica_format` : `I64(3)` ;
    - `from_snapshot_of_another_replica_format_is_refused` : `format = 4` ;
    - `merge_of_a_peer_that_rewrites_the_format_is_refused` : B écrit `4`, et A garde `I64(3)` ;
    - `format_of_meta` et `meta_of` passent en `pub(super)` ;
  - `crates/oku-slides-core/src/replica/deck_tree_tests.rs` (nouveaux tests du format) ;
  - `crates/oku-slides-core/ARCHITECTURE.md` l. 27 : « Zip v8 » → « Zip v9 », sur place, 50 lignes au plus ;
  - `crates/oku-slides-pdf/ARCHITECTURE.md` l. 22 et l. 25 → 9 ; `crates/oku-slides-pdf/src/lib.rs` l. 18 et l. 21 → 9 ;
  - `docs/aide/fr.json` et `docs/aide/en.json` :
    - l. 2152 (titre « … (version 8) »), l. 2163 (`version: 8`) et l. 2434 (« zip version 8 ») → 9 ;
    - slide `s-format-loro`, puce « Quatre sortes de champs » (fr l. 2283-2287, en l. 2283-2287), réécrite :
      - fr : « listes ordonnées à id (items, données), prose, prose riche à marques, et le reste en dernier écrivain gagne. Les slides et leurs éléments forment un seul arbre : déplacer un élément d'un slot ou d'une slide à l'autre garde les éditions concurrentes, et annuler un retrait rend le même élément. » ;
      - en : « ordered lists with ids (items, data), prose, rich prose with marks, and the rest last-writer-wins. Slides and their elements form a single tree: moving an element between slots or slides keeps concurrent edits, and undoing a removal brings back the same element. » ;
    - **ne pas toucher** « Le deck JSON (version 5) » (`DECK_VERSION`), ni « FUSION SANS CONFLIT » (fr) et « canonical read-back » (en), que cherche `crates/oku-slides-session/src/help_tests.rs` (l. 301 et 320) ;
  - `crates/oku-slides-session/src/resume_tests.rs` (après `read_doc_of_a_replica_without_format_returns_unsupported_version`, l. 76-90) ;
  - `crates/oku-slides-session/tests/regressions/main.rs`.
- Create : `crates/oku-slides-session/tests/regressions/issue_0746_reprise_v8_mise_de_cote.rs`.

**Consommateurs recensés :**
- `SOURCES_VERSION` : `sources.rs` l. 46, 450, 575-576.
- Manifestes en dur : `sources_tests.rs` l. 27 (version courante), l. 72, 85, 98, 110 et 853 (versions refusées, inchangées) ; `zip_limits.rs` l. 56.
- `REPLICA_FORMAT` : `mod.rs` l. 47, 109 et 181.
- `issue_0694_reprise_v7_mise_de_cote.rs` (session) : il retire `meta.format`, et reste valable tel quel (garde).

- [ ] **Step 1 : tests rouges.**
  - `sources_tests.rs`, `sources_v8_zip_is_refused`, sur le modèle de `sources_v7_zip_is_refused` : `{"format":"oku-slides-sources","version":8}` donne `SourcesErrorKind::NotSources`.
  - `deck_tree_tests.rs` :
    - `from_snapshot_of_a_format_2_replica_is_refused` : `meta.format = 2` donne `UnsupportedVersion` ;
    - `from_snapshot_of_a_v8_shaped_replica_is_refused` : un `LoroDoc` construit à la main au format v8 donne `UnsupportedVersion`, et non `Corrupted`. Sa forme : `meta.deck_uid`, `meta.format = 2`, `deck.version = 5`, `deck.slides` en `LoroMovableList`, une slide dont `@elements` est un `LoroTree` à un élément `t`. La fabrique `v8_shaped_snapshot() -> Vec<u8>` est `pub(crate)` sous `#[cfg(any(test, feature = "test-support"))]` si la session en a besoin ; sinon, elle est recopiée côté session ;
    - `merge_of_a_format_2_peer_is_refused` : B réécrit `meta.format = 2`. `a.merge(&b)` donne `UnsupportedVersion`, et `a` garde son deck et `format == 3`.
  - `tree_tests.rs` : les trois valeurs ci-dessus (`3`, `4`, `4`).
  - `resume_tests.rs`, `read_doc_of_a_format_2_replica_returns_unsupported_version` : même forme que le test de la l. 78, avec `insert("format", 2_i64)` au lieu de `delete("format")`. Le texte de l'erreur contient « unsupported replica format ».
  - Régression session `issue_0746_reprise_v8_mise_de_cote.rs`, copie de `issue_0694_reprise_v7_mise_de_cote.rs`, sur un `deck.loro` **à la forme v8** (slides en `LoroMovableList`, `@elements` par slide, `meta.format = 2`, construit avec `loro::LoroDoc` comme la fabrique du core) qui remplace celui de la reprise. Mêmes assertions : `resume` vaut `false`, un seul `resume.failed-*` aux octets forgés, et `resume` n'existe plus.
  - Lancer : rouges.
- [ ] **Step 2 : implémenter** ; passer `MANIFEST` et `zip_limits.rs` en version 9 ; faire toutes les mises à jour de doc de la liste **Files**. Vérifier `wc -l` ≤ 50 pour les deux ARCHITECTURE.
- [ ] **Step 3 : lancer** la commande d'avant commit, sur tout le workspace : `oku-slides-session`, `oku-slides-cli` et `oku-slides-render` relisent des zips qu'ils écrivent, et `help_tests` vérifie l'aide.
- [ ] **Step 4 : commit.**

  ```text
  feat(sources) : sources en v9 et replica au format 3, un replica v8 est refusé à l'import comme à la fusion (t-746)
  ```

---

## Task 9: convergence et proptest (t-746, t-747, t-765)

Ces tests sont des gardes du comportement livré par les Tasks 3, 4 et 6 : leur rouge a été vu avec les régressions. Ils sont attendus verts. S'ils révèlent une divergence, s'arrêter, réduire le cas et rapporter `BLOCKED` avec le cas minimal. On ne restreint jamais une stratégie pour faire passer un test.

**Files :**
- Modify :
  - `crates/oku-slides-core/tests/tree_convergence.rs` (tests déterministes et leurs aides) ;
  - `crates/oku-slides-core/tests/convergence.rs` (proptest seulement : `Edit` l. 17-47, `edit()` l. 55-77, `apply` l. 313-379, boucle du proptest l. 394-406, assertions l. 422-441).

**Aides de `tree_convergence.rs`** (en plus de `nest`, `located`, `first_run_bold`, `exchange`, `place`) :
- `fn to_slide(deck: &Deck, id: &str, slide: &str, slot: &str, at: Option<usize>) -> Vec<Op>` : un `move_element` de `id` vers `slot` de `slide` ;
- `fn slide_of(deck: &Deck, id: &str) -> Vec<&str>` : les slides où `id` apparaît.

- [ ] **Step 1 : tests.**
  - `tree_convergence.rs` :
    - `concurrent_move_between_slides_and_nest_agree` : A fait `to_slide(…, "b-points", "s-priorites", "blocks", Some(0))` ; B fait `nest(…, "b-points", "pile")` (deck `with_pile(roadmap, "pile")`). Après la fusion : mêmes decks, `b-points` une fois, et le couple est cohérent : `("s-priorites", None, "blocks")` ou `("s-constat", Some("pile"), "children")`. Pas d'`issue-unknown-param`.
    - `concurrent_moves_of_one_element_to_two_slides_agree` : A l'envoie dans `blocks` de `s-priorites`, B dans `elements` de `s-libre`. Une seule occurrence, dans un slot que sa slide déclare.
    - `move_into_a_concurrently_removed_slide_hides_the_element_until_undo` :
      - A retire `s-priorites`, B y envoie `b-points` ;
      - après la fusion : ni `s-priorites` ni `b-points`, aucune `Issue` ;
      - A annule, échange : `s-priorites` revient avec `blocks == ["b-points", "c1", "c2", "c3"]`, chez les deux.
    - `concurrent_removals_of_a_slide_then_one_undo_keep_it_removed` : parade par pair, pour une slide. A et B retirent `s-ch2`, A annule. Dans les trois séquences de la Task 3, `s-ch2` reste absente chez les deux, et `apply(deck())` n'écrit rien.
    - `undo_of_an_element_removal_keeps_the_peer_move_to_another_slide` : A retire `b-points` ; B l'envoie dans `s-priorites` et le met en gras ; A fusionne puis annule ; échange. `b-points` est dans `s-priorites`, gras, une fois.
    - `concurrent_slide_inserts_at_one_place_and_a_reorder_keep_every_slide` (jitter 0 sur les slides) :
      - trois pairs : A et B ajoutent chacun une slide en position 2 ; C déplace `s-fin` en tête ;
      - échange complet, puis A ajoute une slide entre les deux nouvelles ; échange ;
      - toutes les slides sont présentes une fois, decks égaux, et `s-fin` est en tête. Si `s-fin` ne l'est pas (`Rearrange`), le test asserte la convergence et l'absence de perte, et la limite d'ADR-0006 est confirmée dans le rapport.
  - `convergence.rs`, proptest `peers_editing_in_parallel_converge` :
    - nouvelles éditions dans `Edit` et dans `edit()` :
      - `Edit::MoveToSlide(usize, usize, usize)` : l'élément `a` passe en position `at` (modulo) du premier slot de la slide `content` `b` (modulo), autre que la sienne, par l'op `move_element` (`apply_all`) :
        - `a` est pris modulo, à toutes profondeurs, parmi les éléments des slides `content` (`walk`) ;
        - rien s'il n'y a qu'une slide `content`, ou si l'op est refusée ;
      - `Edit::RemoveElementThenUndo(usize)` : retire l'élément `i` (modulo, `busy_slot`), applique, puis `replica.undo()`. Traité dans la boucle, comme `Edit::Undo` ;
      - `Edit::DeleteThenUndo(usize)` : idem pour la slide `i` ;
      - dans `apply` (l. 313), `RemoveElementThenUndo` et `DeleteThenUndo` ne font rien : la boucle les traite avant ;
    - **oracles** (R11 de la revue), en plus de la convergence :
      1. dans la boucle, après chaque `RemoveElementThenUndo`, `DeleteThenUndo` et `Edit::Undo` qui suit sa propre édition, le deck relu est égal au deck d'avant l'édition (annulation exacte en local) ;
      2. tout id du deck final appartient à l'ensemble des ids jamais créés : ids de départ, `p{peer}-{n}`, `pile-*` et les ids qu'introduit `Edit::Replace`. Un id `x-2` trahit une réparation t-652. Le seul cas admis est un id réajouté face à l'annulation de son retrait (ADR-0006, Limites), que la stratégie ne génère pas : l'oracle est strict ;
      3. `issue-slot-repaired` n'est plus filtré (l. 425) : `issues` doit être vide ;
      4. survie d'une édition : si un pair a mis en gras un élément (`Edit::Bold`) que personne ne retire (aucun `RemoveElement`, `RemoveOwner`, `Delete` sur sa slide, `Replace`), le gras est présent à la fin ;
    - les invariants existants restent : pas d'id en double, données de table cohérentes, second `apply` sans écriture ;
    - passer temporairement `with_cases(48)` à `with_cases(1000)` et lancer une fois `-E 'test(peers_editing_in_parallel_converge)'`. Noter le résultat dans le rapport. Tout cas minimal trouvé devient un test fixe dans `tree_convergence.rs`. **Revenir à 48** avant de committer.
- [ ] **Step 2 : lancer** `-E 'test(concurrent_) | test(move_into) | test(undo_of) | test(peers_editing)'`, puis la commande d'avant commit.
- [ ] **Step 3 : commit.**

  ```text
  test(replica) : convergence des déplacements entre slides et des retraits annulés, oracles du proptest (t-746, t-747)
  ```

---

## Risques et parades

| Risque | Parade |
|---|---|
| Un booléen unique de suppression : l'annulation de A efface le retrait concurrent de B. | Une marque par pair, `@deleted/<pair>` = `true`. Supprimé tant qu'il en reste une (R1). Tests `concurrent_removals_then_one_undo_keep_the_element_removed` (Task 3) et `…_of_a_slide_…` (Task 9). |
| Marque, `@slide` ou `@slot` forgés : nœud caché ou glissé hors de sa place. | Entrée stricte (R0) : valeurs et formes exactes, refus `Corrupted` à l'import et à la fusion, `self` intact. Tests de refus (Tasks 3 et 4). Limite de confiance documentée : un pair peut supprimer. |
| Une slide hors racine (sous un élément ou une slide). | Refusée à l'entrée (R0, R5 de la revue). La réconciliation ne la produit jamais. Test `slide_outside_the_root_is_refused`. |
| Un nœud vivant sous un parent marqué : la lecture le montrerait, ou l'écriture le toucherait. | La lecture et `live_index` ne descendent jamais sous un nœud marqué (R1). Un frère marqué ne donne pas son slot (R2). Tests du point 3 de la sonde et `marked_sibling_does_not_give_its_slot`. |
| La réconciliation ranime un nœud marqué par son id. | Un nœud marqué n'est jamais indexé, donc jamais apparié (R5, point 6). Tests `readding_a_removed_id_creates_a_new_node`, `restore_before_a_slide_removal_brings_it_back_as_a_new_node`. |
| Un id réajouté, puis l'annulation de l'ancien retrait : deux nœuds au même id. | Réparé à la lecture par `unique_ids` (t-652), écrit une fois. Test `undo_of_a_removal_after_the_id_was_readded_reads_unique_ids` ; doc de `unique_ids` et ADR-0006 corrigées. |
| `mark` sur sa propre marque : Loro saute l'`insert`, et l'annulation n'a rien à défaire. | `mark` ne vise qu'un nœud visible (R1). Au temps 3, un différé n'est jamais déjà marqué (R6). |
| Croissance des nœuds marqués : snapshot, lecture et `apply` en O(vivants + marqués) ; `max_replica_bytes` n'est vérifié qu'à l'import. | Documenté (ADR-0006, Limites) ; compaction : t-770. Test de coût `marked_slides_cost_stays_linear` (Task 4) et tests hostiles à 100 000 clés (Task 3). |
| Défaire un ajout : `delete` de Loro, un sous-arbre déplacé dessous par un pair disparaît. | Limite documentée, test `undo_of_an_add_deletes_the_node_and_a_child_moved_in_by_the_peer`, tâche backlog t-776, citée par l'ADR-0006. |
| Rétablir un ajout recrée les nœuds sous d'autres `TreeID` : les clés `@slot` des enfants désignent l'ancien parent. | Règle 3a : les entrées de parents vivants non parents sont périmées et ignorées ; en silence si les clés de nœuds disparus donnent toutes le même slot, sinon 3b avec `Issue`. Écrite au premier `apply`. Tests `undo_redo_of_adding_an_owner_with_children_restores_exactly`, `…_a_slide_with_elements_…` et `undo_redo_of_moving_an_element_into_a_new_owner_restores_exactly` (Task 4, Step 8). |
| Loro crée un nœud à méta vide en défaisant un `mov` dont la cible a été supprimée : un replica légitime serait refusé, ou lu avec une `Issue`. | Racine sans `@slide` acceptée et ignorée ; nœud vide ignoré sans `Issue` (R0, R1). Tests à deux pairs `undo_of_a_slide_move_after_the_peer_undid_its_creation_is_accepted` et `undo_of_a_move_after_the_peer_undid_the_add_leaves_an_ignored_empty_node`. |
| Appariement par id entre slides : un `body` retiré ici et un `body` neuf ailleurs partagent un nœud, et l'édition d'un pair fuit. | Déplacement déduit seulement pour un id unique des deux côtés, entre deux slides gardées, de même `kind` (R5, R7 de la revue). Sinon `move_element`. Tests `element_ids_repeated_across_slides_are_not_paired_between_slides`, `element_moved_with_another_kind_…`. |
| Contenu d'une slide retirée recyclé dans une slide neuve. | Pas d'appariement depuis ou vers une slide non appariée (R5, point 5). Test `element_of_a_removed_slide_is_created_anew_elsewhere`. |
| Couper-coller ne garde pas le nœud : le retrait immédiat interdit toute reprise. | Couper marque, coller envoie `move_element` (Task 7) ; `move_element` passe l'appariement par `moves` (Task 6). Choix de comportement à confirmer par l'utilisateur. |
| `mov` qui créerait un cycle lors d'un déplacement entre slides. | Slides placées d'abord, propriétaires avant leurs éléments, appariement injectif (R6) ; `move_element` refuse un déplacement dans son propre sous-arbre. Test `concurrent_crossed_nesting_across_slides_converges_without_cycle`, à pairs fixés. |
| Création concurrente de `@slot` : une carte ordinaire perd les entrées de l'autre pair (t-188). | `ensure_mergeable_map` (R3). Test `concurrent_slot_entries_on_a_forged_node_are_both_kept`. |
| Clé `@slot` illisible ou non canonique prise pour un parent. | Règle 3 limitée aux clés `TreeID` canoniques (R2). Test `read_rule_three_ignores_keys_that_are_not_canonical_tree_ids`. |
| La parade t-263 est retirée pour les slides sans preuve sur le vrai replica. | Preuve **dans** la Task 4 (Step 8), régression pseudo-aléatoire comprise ; `BLOCKED` si rouge. `issue_0263_undo_reorder_loses_slides` reste une garde. |
| Jitter 0 sur les slides : `Rearrange` ramène une slide qu'un pair déplaçait. | Documenté (ADR-0006) ; test déterministe à trois pairs (Task 9). |
| Un `deck.loro` v8 relu hors zip : refusé comme corrompu, ou lu à vide. | `REPLICA_FORMAT = 3` contrôlé avant `check_trees`, à l'import et à la fusion (Task 8). La reprise est mise de côté, testée côté session sur un vrai replica à la forme v8. |
| `apply` qui écrit sans changement (marques, `@slide`, `@slot`). | `set_value` ne réécrit pas une valeur égale. F1 est asserté sur chaque test déterministe, et au proptest sur le second `apply` (R7). |
| Deck trop profond : deck à moitié écrit. | `wanted_slides` rend `SlotsTooDeep` avant toute écriture dans le deck (R6, temps 0). Test `too_deep_slide_leaves_the_whole_deck_untouched`. |
| Panique par index sur un replica hostile. | Pas d'indexation directe dans `pair` et `sync_deck` : `.get()` puis `TreeOrderSnafu`. Test d'invariant `pair_has_one_entry_per_wanted_slide`. |
| Task 4 qui ne compile pas en cours de route. | Ordre imposé : squelettes non branchés, aides, branchement, migration, retrait des constantes ; `cargo check --all-targets` au Step 2. |
| Oracles de test qui réutilisent le code vérifié. | Aides de vérification en lecture brute (Global Constraints). |
| Commits intermédiaires incohérents (arbre de deck sous le format 2). | Squash obligatoire sur `main`, pied `BREAKING CHANGE` sur le seul squash, pas de `!` dans les sujets intermédiaires. |
| Coût du test t-693 rouge (quadratique). | Rouge réduit à 2 000 suffixes (moins d'une seconde) ; compteur `tries` (`#[cfg(test)]`), assertion exacte. |
| Fichiers trop longs. | Nouveaux tests dans `soft_delete_tests.rs`, `deck_tree_tests.rs`, `tests/move_element.rs` et `tests/tree_convergence.rs` ; `tree/deck.rs`, `ops/moving.rs`, et `tree/place.rs` au besoin. |
| Dossier cargo partagé. | Une seule compilation à la fois ; un résultat incohérent est relancé en séquence avant de conclure. |
| Proptest qui diverge. | `BLOCKED` avec le cas minimal, mis en test fixe ; jamais de stratégie restreinte. 1000 cas en local aux Tasks 3, 4 et 9. |
