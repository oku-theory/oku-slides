# Arbre du deck : défaire un ajout, compaction, profondeur — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** défaire un ajout cache le nœud au lieu de le supprimer (t-776) ; à
l'ouverture, les nœuds morts depuis plus de 30 jours d'activité sont vraiment
supprimés (t-770) ; un replica forgé très profond est refusé sans faire
tomber le processus (t-809).

**Architecture :** le `Writer` crée les nœuds par `create_node` : dans une
étape annulable, un nœud naît marqué dans un commit silencieux et sa marque
est retirée dans l'étape. La marque porte sa date ; `replica/compact.rs`
compacte à l'ouverture. `replica/entry.rs` rejoue les opérations d'un
snapshot reçu dans un document neuf (l'état reçu n'est jamais lu), puis borne
ce document : un seul arbre, 1024 niveaux au plus.

**Tech Stack :** Rust, Loro 1.16.2 (figé), snafu, speculoos, nextest, `time`.

**Spec :** `docs/superpowers/specs/2026-10-10-arbre-deck-robustesse-design.md`

## Global Constraints

- Loro reste en 1.16.2 : aucun `cargo update` qui touche `loro*` (les constats de la sonde en dépendent).
- `MAX_TREE_DEPTH = 1024` (racine = 1), sur **tous** les nœuds, supprimés par Loro compris.
- `REPLICA_FORMAT` 3 → 4, `SOURCES_VERSION` 9 → 10, sans compatibilité (Task 2).
- `compact_after_days = 30` dans `[sources]` (Task 4) ; 0 : jamais.
- Jamais `get_deep_value` sur un document reçu (il récurse sans borne).
- Pas de `.unwrap()`/`.expect()` hors tests ; erreurs snafu de `crates/oku-slides-core/src/error/mod.rs`.
- Première phrase de doc < 15 mots ; `//!` sur tout nouveau module ; tests `<unité>_<scénario>_<attendu>`, speculoos.
- Toute commande cargo : `$CARGO` =
  `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo`
  (cible partagée ; un échec dans un test que le diff ne touche pas est un artefact périmé tant
  qu'une relance ne prouve pas le contraire).
- Tout lancement de l'app ou de l'e2e : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` vers un dossier temporaire neuf.
- Commits : conventionnels, en français, `(t-N)`, message dans un fichier, `git add` dans une
  commande à part, `git commit -F <fichier>`. Pas de ligne `Co-Authored-By`.
- Branche `809-fix/arbre-deck-robustesse`, worktree `.worktrees/arbre-deck-robustesse`.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/replica/write.rs` | `Writer::born`, `Writer::now`, `create_node`, `unborn` |
| `crates/oku-slides-core/src/replica/tree.rs`, `tree/place.rs`, `tree/deck.rs` | création par `create_node`, marque datée, `deck_tree_of` |
| `crates/oku-slides-core/src/replica/compact.rs` (créé), `compact_tests.rs` (créé) | compaction |
| `crates/oku-slides-core/src/replica/entry.rs` (créé), `entry_tests.rs` (créé) | ouverture par rejeu, bornes d'entrée, `guarded` |
| `crates/oku-slides-core/src/replica/mod.rs` | `ReplicaLimits`, ouverture, `check_trees` |
| `crates/oku-slides-core/src/replica/history.rs` | version passée par rejeu ; `oku-compact` sans cran |
| `crates/oku-slides-core/src/replica/authors.rs` | rapport de fusion sans parties ni compaction |
| `crates/oku-slides-core/src/replica/undo.rs` | accesseur de test `items` |
| `crates/oku-slides-core/src/sources.rs` | `SourceLimits::compact_after_secs`, `replica()`, `SOURCES_VERSION` |
| `crates/oku-slides-session/src/resume.rs`, `files.rs` ; `crates/oku-slides-cli/src/main.rs` | réglage passé |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml`, `config_tests.rs` | `compact_after_days` |
| `crates/oku-slides-core/tests/loro_depth_probe.rs` (créé), `tests/regressions/issue_0776_*.rs`, `issue_0809_*.rs` (créés) | sonde, régressions |
| `docs/adr/0006-…md`, `docs/adr/0007-compaction-des-noeuds-morts.md` (créé), `docs/adr/README.md`, `crates/oku-slides-core/src/replica/ARCHITECTURE.md` | décisions |

---

## Lot 1 — t-776

### Task 1 : Naître marqué — défaire un ajout cache le nœud

**Files :**
- Modify : `crates/oku-slides-core/src/replica/write.rs`, `crates/oku-slides-core/src/replica/mod.rs`
  (`apply_moving`, `reconcile_doc`, `create`, `apply_silent`, `restore`),
  `crates/oku-slides-core/src/replica/tree/place.rs`, `crates/oku-slides-core/src/replica/tree/deck.rs:429-437`,
  `crates/oku-slides-core/src/replica/tree.rs` (`mark` en `pub(super)`, docs),
  `crates/oku-slides-core/src/replica/undo.rs`, `crates/oku-slides-core/src/replica/authors.rs`,
  `crates/oku-slides-core/src/replica/soft_delete_tests.rs`, `crates/oku-slides-core/tests/regressions/main.rs`,
  `docs/adr/0006-deck-en-un-seul-arbre-suppression-douce.md`
- Create : `crates/oku-slides-core/tests/regressions/issue_0776_undo_add_keeps_moved_subtree.rs`

**Interfaces :**
- Produces :
  - `Writer { …, born: Option<Split<'a>> }`
  - `Writer::create_node(&self, tree: &LoroTree, parent: TreeParentId, at: usize) -> Result<TreeID, ReplicaInner>`
  - `Writer::unborn(&self, meta: &LoroMap) -> Res`
  - `arrange(tree, parent, kids, list, present, create: &dyn Fn(TreeParentId, usize) -> Result<TreeID, ReplicaInner>)`
  - `reconcile_doc(doc, deck, registry, split, born: Option<write::Split<'_>>, moves)`
  - `Undo::items(&self) -> usize` (`#[cfg(test)] pub(super)`)
  - `authors::report_since` ne compte plus les changes dont le message commence par `PART`.

- [ ] **Step 1 : régression** (`tests/regressions/issue_0776_undo_add_keeps_moved_subtree.rs`)

```rust
//! t-776 : B crée une slide, A y déplace un élément existant
//! (`move_element`), B annule sa création. Loro inversait la création par
//! `tree.delete` : l'élément de A partait avec la slide, et le ⌘Z de A ne le
//! rendait pas. La création naît marquée : annuler remet la marque, la slide et
//! l'élément sont cachés, et le ⌘Z de A ou le rétablir de B les rend.

use std::sync::Arc;

use loro::{Container, LoroDoc, LoroValue, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, Op, WidgetRegistry, apply_all, apply_all_moving};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn peers(deck: &Deck, registry: &Arc<WidgetRegistry>) -> (DeckReplica, DeckReplica) {
    let r = DeckReplica::new(deck, registry.clone()).unwrap_or_else(|e| panic!("replica : {e}"));
    let snap = r.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    (open(), open())
}

fn exchange(a: &mut DeckReplica, b: &mut DeckReplica) {
    a.merge(b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(a).unwrap_or_else(|e| panic!("merge : {e}"));
}

fn undo(r: &mut DeckReplica) {
    assert_that!(r.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
}

/// Les slides de `r` qui portent l'élément `id`.
fn where_is(r: &DeckReplica, id: &str) -> Vec<String> {
    r.deck().0.slides.iter().filter(|s| s.locate(id).is_some()).map(|s| s.id.clone()).collect()
}

fn has_slide(r: &DeckReplica, id: &str) -> bool {
    r.deck().0.slide(id).is_some()
}

/// Vrai si une racine de l'arbre du deck de `r`, lue brute, porte l'`id` `id` (marquée ou non).
fn slide_node_survives(r: &DeckReplica, id: &str) -> bool {
    let doc = LoroDoc::new();
    doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    tree.roots().into_iter().any(|n| {
        tree.get_meta(n).is_ok_and(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id))
    })
}

/// B ajoute `s-neuve` ; A y déplace `b-points` (de `s-constat`) ; échanges.
fn scenario() -> (DeckReplica, DeckReplica) {
    let registry = common::shared();
    let (mut a, mut b) = peers(&roadmap(), &registry);
    let mut added = b.deck().0;
    let slide = json!({ "id": "s-neuve", "kind": "content", "params": { "title": "Neuve" } });
    apply_all(&mut added, vec![op(json!({ "op": "add_slide", "slide": slide }))], &registry)
        .unwrap_or_else(|e| panic!("{e:?}"));
    b.apply(&added, "Ajouter une slide", None).unwrap_or_else(|e| panic!("{e}"));
    exchange(&mut a, &mut b);
    let mut moved = a.deck().0;
    let mv = op(json!({ "op": "move_element", "slide": "s-constat", "element": "b-points",
        "to": { "owner": { "slide": "s-neuve" }, "param": "blocks" }, "index": 0 }));
    let moves = apply_all_moving(&mut moved, vec![mv], &registry).unwrap_or_else(|e| panic!("{e:?}"));
    a.apply_moving(&moved, "Déplacer", None, &moves).unwrap_or_else(|e| panic!("{e}"));
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(where_is(r, "b-points")).named("précondition").is_equal_to(vec!["s-neuve".to_owned()]);
    }
    (a, b)
}

#[test]
fn undo_of_a_slide_creation_hides_the_slide_without_deleting_its_node() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(has_slide(r, "s-neuve")).is_false();
        assert_that!(where_is(r, "b-points")).is_empty();
        assert_that!(slide_node_survives(r, "s-neuve")).is_true();
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
}

#[test]
fn undo_by_the_peer_after_the_creation_undo_brings_the_element_back() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    undo(&mut a);
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        assert_that!(where_is(r, "b-points")).is_equal_to(vec!["s-constat".to_owned()]);
    }
}

#[test]
fn redo_of_the_creation_brings_the_slide_back_with_the_element() {
    let (mut a, mut b) = scenario();
    undo(&mut b);
    exchange(&mut a, &mut b);
    assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
    exchange(&mut a, &mut b);
    assert_that!(where_is(&a, "b-points")).is_equal_to(vec!["s-neuve".to_owned()]);
    assert_that!(b.deck().0).is_equal_to(a.deck().0);
}
```

(`blocks` est omis de la slide : un tableau vide se lirait `ParamValue::Rich`.) Ajouter
`mod issue_0776_undo_add_keeps_moved_subtree;` à `tests/regressions/main.rs`.

- [ ] **Step 2 : vérifier l'échec** — `$CARGO nextest run -p oku-slides-core --test regressions issue_0776`
Expected : `undo_of_a_slide_creation…` (le nœud est supprimé par Loro) et
`undo_by_the_peer…` échouent. `redo_of_the_creation…` peut passer avant le
correctif (Loro recrée le même nœud au rétablir) : c'est une garde.

- [ ] **Step 3 : tests unitaires** (`soft_delete_tests.rs`)

Remplacer `undo_of_an_add_deletes_the_node_and_a_child_moved_in_by_the_peer` (l. 236-257) par :

```rust
/// t-776 : défaire un ajout marque le nœud ; l'enfant que le pair y a déplacé
/// est caché avec lui, et rétablir rend le même nœud avec l'enfant.
#[test]
fn undo_of_an_add_marks_the_node_and_hides_a_child_moved_in_by_the_peer() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let added = with_ops(&deck, vec![add(None, "blocks", None, json!({ "id": "pile", "kind": "stack" }))]);
    apply(&mut a, &added);
    exchange(&mut a, &mut b);
    let (_, pile) = node_of(&a.doc, "s-constat", "pile");
    let points = element(&deck, "b-points");
    apply(&mut b, &with_ops(&added, vec![remove("b-points"), add(Some("pile"), "children", None, points)]));
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    for r in [&a, &b] {
        let read = r.deck().0;
        assert_that!(has(&read, "pile")).is_false();
        assert_that!(has(&read, "b-points")).is_false();
        let tree = slide_tree(r, "s-constat");
        assert_that!(ok(tree.is_node_deleted(&pile))).is_false();
        assert_that!(marks(&tree, pile)).is_equal_to(vec![mark_key(1)]);
    }
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(ok(a.redo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(node_of(&a.doc, "s-constat", "pile").1).is_equal_to(pile);
    assert_that!(ids_in(&a.deck().0, "s-constat", Some("pile"), "children")).is_equal_to(vec!["b-points".to_owned()]);
}
```

Remplacer `undo_of_a_move_after_the_peer_undid_the_add_leaves_an_ignored_empty_node`
(l. 259-296) par :

```rust
/// t-776 : défaire un `mov` après l'annulation de l'ajout par le pair ne crée
/// plus de nœud vide : le nœud, marqué par le pair, reçoit le `mov` défait.
#[test]
fn undo_of_a_move_after_the_peer_undid_the_add_leaves_no_empty_node() {
    let deck = roadmap();
    let (mut a, mut b) = fixed_peers(&deck, 1, 2);
    let added = with_ops(&deck, vec![add(None, "blocks", Some(0), text("e", "Éphémère"))]);
    apply(&mut b, &added);
    exchange(&mut a, &mut b);
    let (_, node) = node_of(&a.doc, "s-constat", "e");
    let mut e = element(&added, "e");
    e["frame"] = json!({ "x": 0, "y": 0, "w": 400, "h": 200 });
    apply(&mut a, &with_ops(&added, vec![remove("e"), add(None, "elements", None, e)]));
    assert_that!(ok(b.undo())).is_true();
    exchange(&mut a, &mut b);
    assert_that!(ok(a.undo())).is_true();
    exchange(&mut a, &mut b);
    let tree = slide_tree(&a, "s-constat");
    assert_that!(ok(tree.is_node_deleted(&node))).named("nœud de e").is_false();
    assert_that!(marks(&tree, node)).is_equal_to(vec![mark_key(2)]);
    let empty = tree.nodes().into_iter().filter(|n| !ok(tree.is_node_deleted(n))).any(|n| ok(tree.get_meta(n)).is_empty());
    assert_that!(empty).is_false();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(has(&read, "e")).is_false();
    assert_that!(b.deck()).is_equal_to((read, Vec::new()));
    ok(DeckReplica::from_snapshot(&ok(a.snapshot()), 1 << 24, registry()));
    exchange(&mut a, &mut b);
    for r in [&mut a, &mut b] {
        let before = ops_of(r);
        let read = r.deck().0;
        apply(r, &read);
        assert_that!(ops_of(r)).is_equal_to(before);
    }
}
```

Ajouter :

```rust
/// t-776 : un collage se défait d'un bloc (nœuds marqués, pas supprimés), et
/// rétablir rend les mêmes nœuds.
#[test]
fn undo_of_a_paste_marks_every_pasted_node_and_redo_restores_the_same_nodes() {
    let deck = roadmap();
    let mut r = new(&deck);
    let ids: Vec<String> = (0..5).map(|i| format!("p{i}")).collect();
    let pasted = with_ops(&deck, ids.iter().map(|id| add(None, "blocks", None, text(id, "Collé"))).collect());
    apply(&mut r, &pasted);
    let before: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-constat", id).1).collect();
    assert_that!(ok(r.undo())).is_true();
    let tree = deck_tree(&r);
    for n in &before {
        assert_that!(ok(tree.is_node_deleted(n))).is_false();
        assert_that!(marks(&tree, *n)).is_equal_to(vec![mark_key(r.peer_id())]);
    }
    assert_that!(r.can_undo()).is_false();
    assert_that!(ok(r.redo())).is_true();
    let after: Vec<TreeID> = ids.iter().map(|id| node_of(&r.doc, "s-constat", id).1).collect();
    assert_that!(after).is_equal_to(before);
}

/// t-776 : `apply_silent` crée sans commit de naissance ; `apply`, avec un par nœud créé.
#[test]
fn apply_silent_adds_one_change_and_apply_one_more_per_created_node() {
    let deck = roadmap();
    let three = |prefix: &str| with_ops(&deck, (0..3).map(|i| add(None, "blocks", None, text(&format!("{prefix}{i}"), "x"))).collect());
    let mut r = new(&deck);
    let before = r.len_changes();
    ok(r.apply_silent(&three("s")));
    assert_that!(r.len_changes() - before).is_equal_to(1);
    let mut r = new(&deck);
    let before = r.len_changes();
    apply(&mut r, &three("a"));
    // Par nœud : la partie qui précède (si non vide) et le commit de naissance ; puis le commit final.
    assert_that!(r.len_changes() - before).is_greater_than_or_equal_to(4);
}
```

Run : `$CARGO nextest run -p oku-slides-core soft_delete`
Expected : FAIL pour `undo_of_an_add_marks…`, `undo_of_a_move…no_empty_node`, `undo_of_a_paste…`,
`apply_silent_adds…` (le nœud est supprimé par Loro, pas marqué ; aucun commit de naissance).

- [ ] **Step 4 : implémenter**

`write.rs`, struct `Writer` :

```rust
    /// Commit silencieux où un nœud naît marqué (t-776) ; `None` : création directe.
    ///
    /// Dans une étape annulable, la marque est retirée dans l'étape : défaire
    /// l'ajout la remet au lieu d'un `tree.delete` de Loro.
    pub(super) born: Option<Split<'a>>,
```

`impl Writer<'_>` :

```rust
    /// Crée un nœud sous `parent`, en `at` ; avec `born`, il naît marqué (t-776).
    ///
    /// Ce qui précède part d'abord dans une partie annulable (`split`) : le
    /// commit silencieux ne porte que la création et sa marque.
    pub(super) fn create_node(&self, tree: &LoroTree, parent: TreeParentId, at: usize) -> Result<TreeID, ReplicaInner> {
        let Some(born) = self.born else { return tree.create_at(parent, at).context(LoroSnafu) };
        (self.split)();
        let n = tree.create_at(parent, at).context(LoroSnafu)?;
        self.mark(tree, n)?;
        born();
        Ok(n)
    }

    /// Retire la marque de naissance d'un nœud que [`Self::create_node`] vient de créer.
    pub(super) fn unborn(&self, meta: &LoroMap) -> Res {
        if self.born.is_some() {
            meta.delete(&self.mark_key).context(LoroSnafu)?;
        }
        Ok(())
    }
```

(`Writer::mark`, dans `tree.rs`, passe en `pub(super)`. Imports de `write.rs` : `LoroTree`,
`TreeID`, `TreeParentId`, `LoroSnafu` s'il manque.)

`place.rs`, `arrange` : dernier paramètre `create: &dyn Fn(TreeParentId, usize) -> Result<TreeID, ReplicaInner>` ;
la branche `(None, None)` fait `let n = create(parent, at)?;`. Doc : « `create` crée un nœud
manquant (directement, ou né marqué). » `place` :

```rust
            let arranged = arrange(tree, tp, &mut kids, &nodes, &present, &|p, at| self.create_node(tree, p, at))?;
            for (&w, (n, created)) in list.iter().zip(arranged) {
                let meta = tree.get_meta(n).context(LoroSnafu)?;
                if created {
                    self.unborn(&meta)?;
                }
                set_value(&slot_map_at(&meta)?, &pid, LoroValue::from(slot))?;
                self.sync_map(&meta, wanted.get(w).context(TreeOrderSnafu)?.obj, Ctx::Element)?;
                *placed.get_mut(w).context(TreeOrderSnafu)? = Some(n);
            }
```

`deck.rs`, `sync_deck` (l. 431-437) :

```rust
        let placed = arrange(tree, TreeParentId::Root, &mut roots, &pairing.slides, &|n| shown.contains(&n), &|p, at| {
            self.create_node(tree, p, at)
        })?;
        // … `slots` inchangé …
        for (i, ws) in wanted.iter().enumerate() {
            let (node, created) = placed.get(i).copied().context(TreeOrderSnafu)?;
            let meta = tree.get_meta(node).context(LoroSnafu)?;
            if created {
                self.unborn(&meta)?;
            }
            set_value(&meta, SLIDE, LoroValue::Bool(true))?;
            // … suite inchangée
```

`mod.rs` :
- `reconcile_doc(doc, deck, registry, split, born: Option<write::Split<'_>>, moves)` ; construction :
  `write::Writer { registry, split, born, mark_key: schema::mark_key(doc.peer_id()), moves }`.
- `create` : `…, &|| (), None, &[]` ; `apply_silent` : `…, &commit, None, &[]`.
- essai de `restore` (même chemin que l'`apply` qui suit) : `…, &|| doc.commit(), Some(&|| doc.commit()), &[]`.
- `apply_moving` :

```rust
        let doc = &self.doc;
        let split = || {
            doc.set_next_commit_message(&part_message(label));
            doc.commit();
        };
        let born = || {
            doc.set_next_commit_message(&part_message(label));
            doc.set_next_commit_origin(undo::SILENT);
            doc.commit();
        };
        Self::reconcile_doc(doc, deck, &self.registry, &split, Some(&born), moves)?;
```

`undo.rs` :

```rust
    /// Items Loro de la pile d'annulation (tests de coût).
    #[cfg(test)]
    pub(super) fn items(&self) -> usize {
        self.manager.undo_count()
    }
```

`authors.rs`, `report_since` : ne compter un change (ni son pair) que si son message ne commence
pas par `super::PART` :

```rust
        let part = meta.message.as_deref().is_some_and(|m| m.starts_with(super::PART));
        if !before.includes_id(meta.id) && !part {
            changes += 1;
            peers.insert(meta.id.peer);
        }
```

Doc de `report_since` : « Les parties d'une édition (t-263, t-776) ne comptent pas : une édition
compte une fois. » Doc du module `tree.rs` et de `Writer::mark` : « Une marque est aussi posée à
la naissance d'un nœud dans une étape annulable, puis retirée dans l'étape : c'est la seule marque
que la réconciliation retire (t-776). »

- [ ] **Step 5 : vérifier** — `$CARGO nextest run -p oku-slides-core -p oku-slides-session`
Expected : PASS. Un test qui compte les changes, les items ou les modifications d'un rapport de
fusion peut bouger : ajuster l'attendu s'il comptait des parties ou des nœuds créés, et le dire
dans le rapport ; tout autre écart est un bogue.

- [ ] **Step 6 : mesurer** (`soft_delete_tests.rs`)

```rust
/// Mesure t-776 : coût d'un collage, à plat et imbriqué.
#[test]
#[ignore = "mesure"]
fn measure_paste_cost_flat_and_nested() {
    for n in [50_usize, 500] {
        for nested in [false, true] {
            let deck = with_pile();
            let mut r = new(&deck);
            let element = |i: usize| {
                if nested {
                    json!({ "id": format!("q{i}"), "kind": "stack",
                        "params": { "children": [text(&format!("q{i}a"), "a"), text(&format!("q{i}b"), "b")] } })
                } else {
                    text(&format!("q{i}"), "Collé")
                }
            };
            let pasted = with_ops(&deck, (0..n).map(|i| add(None, "blocks", None, element(i))).collect());
            let items = r.undo.items();
            let t = std::time::Instant::now();
            apply(&mut r, &pasted);
            let apply_time = t.elapsed();
            let step = r.undo.items() - items;
            let t = std::time::Instant::now();
            assert_that!(ok(r.undo())).is_true();
            println!("collage n={n} imbriqué={nested} : apply {apply_time:?}, ⌘Z {:?}, items {step}", t.elapsed());
        }
    }
}
```

Run : `$CARGO nextest run -p oku-slides-core --release --run-ignored only --no-capture measure_paste_cost`.
Si une étape dépasse 64 items, relever `ITEMS_PER_STEP` à la puissance de 2 au-dessus du plus
grand nombre mesuré (n = 500) et l'expliquer dans son doc. **Si un ⌘Z de n = 500 dépasse 100 ms,
s'arrêter et le signaler** (décision utilisateur). Noter les nombres dans le rapport.

- [ ] **Step 7 : ADR-0006**

**Décision → Suppression douce** : ajouter « Une création, dans une étape annulable, naît marquée
dans un commit silencieux, puis la marque est retirée dans l'étape : défaire l'ajout remet la
marque (t-776). » Remplacer « La réconciliation ne retire jamais une marque » par « La
réconciliation ne retire jamais la marque d'un retrait ». **Limites** : supprimer « Défaire un
ajout supprime le nœud pour de bon » et « Régression visible, acceptée » ; la puce « Un nœud vide
que Loro crée en défaisant un `mov`… » devient « Un nœud vide naît d'un `mov` défait dont la
cible a été supprimée par Loro, ce qu'aucun chemin du code ne fait plus. » ; ajouter « Défaire un
ajout cache aussi ce qu'un pair a déplacé dessous ; le ⌘Z du pair, ou le rétablir, le rend. Un
nœud créé dans une étape coûte un change Loro de plus (mesure : <nombres du Step 6>). Le rapport
de fusion ne compte pas les parties. » Ajouter t-776 à **Tâches**.

- [ ] **Step 8 : commit** — `fix(replica) : défaire un ajout cache le nœud au lieu de le supprimer, la création naît marquée (t-776)`

---

## Lot 2 — t-770

### Task 2 : Marque datée et format 4

**Files :**
- Modify : `crates/oku-slides-core/src/replica/tree.rs` (`is_mark`, `mark_time`, docs, `mark`),
  `crates/oku-slides-core/src/replica/write.rs` (`Writer::now`), `crates/oku-slides-core/src/replica/mod.rs`
  (`REPLICA_FORMAT`, `reconcile_doc`, doc de `check_trees`), `crates/oku-slides-core/src/sources.rs` (l. 1, l. 47),
  `crates/oku-slides-core/src/sources_tests.rs` (l. 27 et test v9), `crates/oku-slides-core/tests/zip_limits.rs:56`,
  `crates/oku-slides-core/tests/untrusted.rs`, `crates/oku-slides-pdf/src/lib.rs:18,21`,
  `crates/oku-slides-pdf/ARCHITECTURE.md:22,25`, `docs/aide/fr.json` et `docs/aide/en.json` (l. 2174, 2185, 2456),
  `docs/adr/0006-…md:35-36`, `crates/oku-slides-core/src/replica/ARCHITECTURE.md`,
  `crates/oku-slides-session/src/resume_tests.rs`, `crates/oku-slides-core/src/replica/soft_delete_tests.rs`

**Interfaces :**
- Produces : `Writer { …, now: i64 }` ; une marque vaut `LoroValue::I64(now)` ;
  `pub(super) fn mark_time(meta: &LoroMap, key: &str) -> Option<i64>` (`tree.rs`) ; `is_mark` n'accepte qu'un `I64`.

- [ ] **Step 1 : tests qui échouent**

`soft_delete_tests.rs` :

```rust
/// t-770 : une marque de retrait porte l'heure Unix de sa pose.
#[test]
fn mark_of_a_removal_carries_its_unix_time() {
    let deck = roadmap();
    let mut r = new(&deck);
    let (_, points) = node_of(&r.doc, "s-constat", "b-points");
    apply(&mut r, &without(&deck, "b-points"));
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match ok(deck_tree(&r).get_meta(points)).get(&mark_key(r.peer_id())) {
        Some(ValueOrContainer::Value(LoroValue::I64(t))) => assert_that!((now - t).abs()).is_less_than(60),
        other => panic!("marque datée attendue : {other:?}"),
    }
}

/// t-770 : défaire un ajout remet la marque de naissance, datée de la création.
#[test]
fn undo_of_an_add_restores_a_birth_mark_dated_at_creation() {
    let deck = roadmap();
    let mut r = new(&deck);
    apply(&mut r, &with_ops(&deck, vec![add(None, "blocks", None, text("n1", "Neuf"))]));
    let (tree, n) = node_of(&r.doc, "s-constat", "n1");
    assert_that!(ok(r.undo())).is_true();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match ok(tree.get_meta(n)).get(&mark_key(r.peer_id())) {
        Some(ValueOrContainer::Value(LoroValue::I64(t))) => assert_that!((now - t).abs()).is_less_than(60),
        other => panic!("marque de naissance datée attendue : {other:?}"),
    }
}
```

`tests/untrusted.rs` : dans `from_snapshot_with_many_marks_on_one_node_hides_it`, la marque forgée
devient `meta.insert(&format!("@deleted/{peer}"), 1_700_000_000_i64)` ; ajouter :

```rust
/// t-770 : une marque qui n'est pas une date est refusée à l'entrée.
#[test]
fn from_snapshot_with_a_non_integer_mark_returns_corrupted() {
    for value in [LoroValue::Bool(true), LoroValue::from("hier"), LoroValue::Double(1.5)] {
        let forged_value = value.clone();
        let bytes = forged("s-constat", move |tree, slide| {
            let points = tree
                .children(slide)
                .unwrap_or_default()
                .into_iter()
                .find(|n| tree.get_meta(*n).is_ok_and(|m| has_id(&m, "b-points")))
                .unwrap_or_else(|| panic!("b-points absent"));
            let meta = tree.get_meta(points).unwrap_or_else(|e| panic!("méta : {e}"));
            meta.insert("@deleted/7", forged_value).unwrap_or_else(|e| panic!("marque : {e}"));
        });
        let kind = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).err().map(|e| e.kind());
        assert_that!(kind).named(&format!("{value:?}")).is_equal_to(Some(ReplicaErrorKind::Corrupted));
    }
}
```

`sources_tests.rs` : `MANIFEST` passe à `"version":10` ; ajouter :

```rust
/// Sources v9 : refusées (marques de suppression non datées, replica au format 3).
#[test]
fn sources_v9_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":9}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}
```

`resume_tests.rs` :

```rust
/// Un replica au format 3 (marques non datées) est refusé.
#[test]
fn read_doc_of_a_format_3_replica_returns_unsupported_version() {
    let r = resume();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&document(), &mut Vec::new()).unwrap()).unwrap();
    let path = r.dir.join(DECK_DIR).join("deck.loro");
    let doc = loro::LoroDoc::new();
    doc.import(&std::fs::read(&path).unwrap()).unwrap();
    doc.get_map("meta").insert("format", 3_i64).unwrap();
    doc.commit();
    std::fs::write(&path, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    let err = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).err().expect("replica format 3 refusé");
    assert_that!(oku_slides_platform::errors::one_line(&err)).contains("unsupported replica format");
    std::fs::remove_dir_all(&r.dir).ok();
}
```

Run : `$CARGO nextest run -p oku-slides-core -p oku-slides-session mark format sources_v9`
Expected : FAIL.

- [ ] **Step 2 : implémenter**

`tree.rs` :

```rust
/// Vrai si `key` est une marque de suppression bien formée : une date (R1).
pub(super) fn is_mark(meta: &LoroMap, key: &str) -> bool {
    mark_time(meta, key).is_some()
}

/// L'heure Unix (s) de la marque `key`, si c'en est une bien formée (t-770).
pub(super) fn mark_time(meta: &LoroMap, key: &str) -> Option<i64> {
    mark_peer(key)?;
    match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::I64(t))) => Some(t),
        _ => None,
    }
}
```

Doc de `is_marked` : « Une marque est une clé `@deleted/<pair>` dont la valeur est l'heure Unix de
sa pose ; le contrôle d'entrée refuse toute autre forme. » `Writer::mark` écrit
`LoroValue::I64(self.now)`. `Writer` gagne `pub(super) now: i64` (doc « Heure Unix (s) des marques
de cette réconciliation (t-770). ») ; `reconcile_doc` passe
`now: time::OffsetDateTime::now_utc().unix_timestamp()`. `REPLICA_FORMAT = 4` (doc : « 4 :
marques datées, t-770 »), `SOURCES_VERSION = 10` (idem).

Versions et formes écrites en dur :
- `crates/oku-slides-core/src/sources.rs:1` (« version 9 » → 10) ;
- `crates/oku-slides-core/tests/zip_limits.rs:56` (`"version":9` → 10) ;
- `crates/oku-slides-pdf/src/lib.rs:18` et `:21`, `crates/oku-slides-pdf/ARCHITECTURE.md:22` et `:25` (9 → 10) ;
- `docs/aide/fr.json` et `docs/aide/en.json`, l. 2174, 2185, 2456 (« version 9 » → 10 ; vérifier
  chaque occurrence avec `grep -n '"version\|version 9\|version: 9' docs/aide/*.json`) ;
- `docs/adr/0006-…md:35-36` (la marque `= true` devient « = l'heure Unix de sa pose (t-770) ») ;
- `crates/oku-slides-core/src/replica/ARCHITECTURE.md` (`meta.format = 3` → 4 ; la marque vaut l'heure de sa pose).

Tests qui forgent ou lisent une marque : `grep -rn 'Bool(true)\|"@deleted\|insert(&mark_key\|, true)' crates/oku-slides-core/src crates/oku-slides-core/tests`
— une marque forgée devient un `i64` (`1_700_000_000_i64`), une marque lue est un `I64`. **Ne pas
toucher** `SLIDE` = `Bool(true)` (`tree/deck.rs:24`, `:437`).

- [ ] **Step 3 : vérifier** — `$CARGO nextest run -p oku-slides-core -p oku-slides-session -p oku-slides-pdf`
Expected : PASS.

- [ ] **Step 4 : commit** — `feat(replica)! : la marque de suppression porte sa date, format 4 (t-770)`

### Task 3 : Compaction à l'ouverture

**Files :**
- Create : `crates/oku-slides-core/src/replica/compact.rs`, `crates/oku-slides-core/src/replica/compact_tests.rs`,
  `docs/adr/0007-compaction-des-noeuds-morts.md`
- Modify : `crates/oku-slides-core/src/replica/mod.rs` (`ReplicaLimits`, `from_snapshot`, `import_snapshot`,
  `mod compact;`), `crates/oku-slides-core/src/lib.rs`, `crates/oku-slides-core/src/replica/tree/deck.rs`
  (`deck_tree_of`), `crates/oku-slides-core/src/replica/tree.rs` (réexport, `find_node`),
  `crates/oku-slides-core/src/replica/history.rs` (`build`), `crates/oku-slides-core/src/replica/authors.rs`,
  `docs/adr/0006-…md`, `docs/adr/README.md`

**Interfaces :**
- Consumes : `mark_time` (Task 2), `undo::SILENT`, `guarded`.
- Produces :
  - `pub struct ReplicaLimits { pub max_bytes: usize, pub compact_after_secs: i64 }` ; `impl From<usize>` (sans compaction) ; réexporté par `lib.rs`.
  - `DeckReplica::from_snapshot(bytes: &[u8], limits: impl Into<ReplicaLimits>, registry) -> Result<(Self, DeckRead), ReplicaError>`.
  - `fn import_snapshot(bytes, limits: ReplicaLimits, registry, now: i64) -> Result<(Self, DeckRead), ReplicaInner>` (privé).
  - `compact.rs` : `pub(super) const COMPACT: &str = "oku-compact";`,
    `pub(super) fn compact_doc(doc: &LoroDoc, now: i64, after_secs: i64) -> Result<usize, ReplicaInner>`,
    `fn newest_change(doc: &LoroDoc) -> Option<i64>`, `fn is_old(t: i64, limit: i64) -> bool`.
  - `tree::deck_tree_of(doc: &LoroDoc) -> Option<LoroTree>` (dans `tree/deck.rs`, à côté de `deck_tree_at`).

- [ ] **Step 1 : `deck_tree_of`** (`tree/deck.rs`, à côté de `deck_tree_at`)

```rust
/// L'arbre du deck de `doc`, s'il existe ; jamais créé (lecture).
///
/// [`deck_tree_at`] le crée au besoin, pour écrire.
pub(in crate::replica) fn deck_tree_of(doc: &LoroDoc) -> Option<LoroTree> {
    match doc.get_map(crate::replica::ROOT).get(SLIDES) {
        Some(ValueOrContainer::Container(Container::Tree(t))) => Some(t),
        _ => None,
    }
}
```

Ajouter `deck_tree_of` au `pub(super) use deck::{…}` de `tree.rs:31` ; `find_node` (`tree.rs`)
l'utilise au lieu de sa lecture locale.

- [ ] **Step 2 : tests qui échouent** (`compact_tests.rs`)

```rust
//! Compaction des nœuds morts à l'ouverture (t-770).

use std::sync::Arc;

use loro::{TreeID, TreeParentId};
use speculoos::prelude::*;

use super::super::tree::{deck_tree_of, node_of};
use super::super::{DeckReplica, ReplicaLimits};
use super::*;
use crate::{Deck, MergeReport};

const DAY: i64 = 86_400;
const DELAY: i64 = 30 * DAY;

fn registry() -> Arc<crate::WidgetRegistry> {
    crate::test_registry::shared()
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// Un replica de `roadmap` sans `b-points` (retiré, donc marqué maintenant), et le `TreeID` de `b-points`.
fn with_points_removed() -> (DeckReplica, TreeID) {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, registry()).unwrap_or_else(|e| panic!("{e}"));
    let (_, points) = node_of(&r.doc, "s-constat", "b-points");
    let mut without = deck.clone();
    let remove = serde_json::json!({ "op": "remove_element", "slide": "s-constat", "element": "b-points" });
    crate::apply_all(&mut without, vec![serde_json::from_value(remove).unwrap_or_else(|e| panic!("{e}"))], &registry())
        .unwrap_or_else(|e| panic!("{e:?}"));
    r.apply(&without, "Retirer", None).unwrap_or_else(|e| panic!("{e}"));
    (r, points)
}

/// Pose sur `n` la marque `@deleted/<peer>` datée `at`, en un commit (daté maintenant par Loro).
fn forge_mark(r: &DeckReplica, n: TreeID, peer: u64, at: i64) {
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    tree.get_meta(n).unwrap_or_else(|e| panic!("{e}")).insert(&format!("@deleted/{peer}"), at).unwrap_or_else(|e| panic!("{e}"));
    r.doc.commit();
}

/// Un change daté `at` (dans le futur) : le document a eu de l'activité jusque-là.
fn activity_at(r: &DeckReplica, at: i64) {
    r.doc.get_map("meta").insert("activité", at).unwrap_or_else(|e| panic!("{e}"));
    r.doc.set_next_commit_timestamp(at);
    r.doc.commit();
}

/// Rouvre `r` comme à l'ouverture d'un fichier, à l'heure `at`, avec le délai `after`.
fn reopen(r: &DeckReplica, at: i64, after: i64) -> DeckReplica {
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    let limits = ReplicaLimits { max_bytes: 1 << 24, compact_after_secs: after };
    DeckReplica::import_snapshot(&bytes, limits, registry(), at).unwrap_or_else(|e| panic!("{e}")).0
}

fn deleted(r: &DeckReplica, n: TreeID) -> bool {
    deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre")).is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn open_compacts_a_node_with_a_mark_older_than_the_delay() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let reopened = reopen(&r, now(), DELAY);
    assert_that!(deleted(&reopened, points)).is_true();
    assert_that!(reopened.deck().0).is_equal_to(r.deck().0);
}

#[test]
fn open_keeps_a_node_whose_marks_are_all_recent() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 10 * DAY);
    assert_that!(deleted(&reopen(&r, now(), DELAY), points)).is_false();
}

#[test]
fn open_compacts_after_thirty_days_of_document_activity() {
    let (r, points) = with_points_removed();
    // La marque du retrait date de maintenant ; 31 jours d'activité la rendent ancienne.
    activity_at(&r, now() + 31 * DAY);
    assert_that!(deleted(&reopen(&r, now() + 31 * DAY, DELAY), points)).is_true();
}

#[test]
fn open_with_a_clock_far_ahead_is_bounded_by_the_newest_change() {
    let (r, points) = with_points_removed();
    // Horloge locale à +10 ans, sans activité : la limite reste « change le plus récent − 30 j ».
    assert_that!(deleted(&reopen(&r, now() + 3650 * DAY, DELAY), points)).is_false();
}

#[test]
fn compact_doc_compacts_a_mark_exactly_at_the_limit_and_keeps_one_a_second_later() {
    let (r, points) = with_points_removed();
    let newest = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    let limit = newest - DELAY;
    forge_mark(&r, points, 7, limit + 1);
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    // Les marques du retrait (maintenant) et forgée (limit + 1) sont récentes.
    let at = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    assert_that!(compact_doc(&r.doc, at, at - limit).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    forge_mark(&r, points, 8, limit);
    let at = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    assert_that!(compact_doc(&r.doc, at, at - limit).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    assert_that!(tree.is_node_deleted(&points).unwrap_or_else(|e| panic!("{e}"))).is_true();
}

#[test]
fn is_old_never_holds_for_a_null_or_negative_date() {
    for t in [0, -1, i64::MIN] {
        assert_that!(is_old(t, i64::MAX)).named(&t.to_string()).is_false();
    }
    assert_that!(is_old(1, 1)).is_true();
    assert_that!(is_old(2, 1)).is_false();
}

#[test]
fn compact_doc_compacts_an_empty_node_after_thirty_days_of_activity() {
    let r = DeckReplica::new(&roadmap(), registry()).unwrap_or_else(|e| panic!("{e}"));
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    let empty = tree.create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    r.doc.commit();
    assert_that!(compact_doc(&r.doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    activity_at(&r, now() + 31 * DAY);
    assert_that!(compact_doc(&r.doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    assert_that!(tree.is_node_deleted(&empty).unwrap_or_else(|e| panic!("{e}"))).is_true();
}

#[test]
fn compact_doc_counts_a_dead_parent_and_its_dead_child_once() {
    let (r, points) = with_points_removed();
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    let Some(TreeParentId::Node(slide)) = tree.parent(points) else { panic!("slide de b-points") };
    let parent = tree.create(TreeParentId::Node(slide)).unwrap_or_else(|e| panic!("{e}"));
    let child = tree.create(TreeParentId::Node(parent)).unwrap_or_else(|e| panic!("{e}"));
    for n in [child, parent] {
        let meta = tree.get_meta(n).unwrap_or_else(|e| panic!("{e}"));
        meta.insert("id", "x").unwrap_or_else(|e| panic!("{e}"));
        meta.insert("@deleted/9", now() - 40 * DAY).unwrap_or_else(|e| panic!("{e}"));
    }
    r.doc.commit();
    assert_that!(compact_doc(&r.doc, now(), DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    for n in [parent, child] {
        assert_that!(tree.is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"))).is_true();
    }
    assert_that!(tree.is_node_deleted(&points).unwrap_or_else(|e| panic!("{e}"))).is_false();
}

#[test]
fn compact_doc_with_nothing_to_compact_makes_no_commit() {
    let r = DeckReplica::new(&roadmap(), registry()).unwrap_or_else(|e| panic!("{e}"));
    let before = r.doc.oplog_vv();
    assert_that!(compact_doc(&r.doc, now(), DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    assert_that!(r.doc.oplog_vv()).is_equal_to(before);
}

#[test]
fn compact_doc_with_a_zero_delay_compacts_nothing() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, 1);
    assert_that!(compact_doc(&r.doc, now(), 0).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    assert_that!(deleted(&r, points)).is_false();
}

#[test]
fn open_compaction_is_not_undoable_and_leaves_no_history_notch() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let versions = r.history().len();
    let mut reopened = reopen(&r, now(), DELAY);
    assert_that!(deleted(&reopened, points)).named("précondition").is_true();
    assert_that!(reopened.can_undo()).is_false();
    assert_that!(reopened.history().len()).is_equal_to(versions);
    // Une édition après l'ouverture se défait sans ramener le nœud compacté.
    let mut renamed = reopened.deck().0;
    renamed.slides[0].name = Some("renommée".to_owned());
    reopened.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(reopened.undo().unwrap_or_else(|e| panic!("{e}"))).is_true();
    assert_that!(deleted(&reopened, points)).is_true();
    assert_that!(reopened.deck().0).is_equal_to(r.deck().0);
}

#[test]
fn compacted_and_uncompacted_copies_converge() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let mut a = reopen(&r, now(), DELAY);
    let mut b = reopen(&r, now(), 0);
    assert_that!(deleted(&a, points)).named("précondition a").is_true();
    assert_that!(deleted(&b, points)).named("précondition b").is_false();
    let mut renamed = b.deck().0;
    renamed.slides[0].name = Some("renommée".to_owned());
    b.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
    a.merge(&b).unwrap_or_else(|e| panic!("{e}"));
    b.merge(&a).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(deleted(&b, points)).is_true();
}

#[test]
fn merge_report_does_not_count_the_compaction() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let mut a = reopen(&r, now(), 0);
    let b = reopen(&r, now(), DELAY);
    assert_that!(deleted(&b, points)).named("précondition").is_true();
    let report = a.merge(&b).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(report).is_equal_to(MergeReport { changes: 0, authors: Vec::new(), anonymous: 0 });
}
```

(Si `MergeReport` n'est pas `PartialEq`, comparer ses trois champs un à un. Si `Slide::name`
n'est pas une `Option<String>`, modifier un autre champ simple de la slide.)
Ajouter `#[cfg(test)] #[path = "compact_tests.rs"] mod compact_tests;` dans `compact.rs`.
Run : `$CARGO nextest run -p oku-slides-core compact open_ merge_report is_old`
Expected : FAIL (compilation).

- [ ] **Step 3 : `compact.rs`**

```rust
//! Compaction des nœuds morts de l'arbre du deck, à l'ouverture (t-770, ADR-0007).
//!
//! Avec `limit = min(now, change le plus récent) − délai`, un nœud qui porte
//! une marque datée de `]0, limit]` (une seule suffit : seul son pair pourrait
//! la retirer, et sa fenêtre est passée), ou un nœud vide daté de `]0, limit]`,
//! est supprimé par `tree.delete` avec son sous-arbre, déjà caché. Une marque
//! n'est donc compactée qu'après un délai d'activité du document. Commit
//! silencieux, de message [`COMPACT`] ; Loro le date comme le change le plus
//! récent.

use loro::{LoroDoc, LoroTree, TreeID, TreeParentId};
use snafu::ResultExt;

use super::tree::{deck_tree_of, mark_time};
use super::undo;
use crate::error::{LoroSnafu, ReplicaInner};

/// Message du commit de compaction : hors historique et hors rapport de fusion.
pub(super) const COMPACT: &str = "oku-compact";

/// Supprime les nœuds morts depuis `after_secs` ; rend leur nombre.
///
/// Un nœud qu'un ancêtre compacté emporte n'est pas compté. `after_secs <= 0` :
/// rien. Un historique illisible ne compacte rien.
pub(super) fn compact_doc(doc: &LoroDoc, now: i64, after_secs: i64) -> Result<usize, ReplicaInner> {
    if after_secs <= 0 {
        return Ok(0);
    }
    let (Some(tree), Some(newest)) = (deck_tree_of(doc), newest_change(doc)) else { return Ok(0) };
    let limit = now.min(newest).saturating_sub(after_secs);
    let mut dead: Vec<TreeID> = Vec::new();
    for n in tree.nodes() {
        // Profondeur bornée à l'entrée : `is_node_deleted` est sûr ici.
        if !tree.is_node_deleted(&n).context(LoroSnafu)? && is_dead(doc, &tree, n, limit)? {
            dead.push(n);
        }
    }
    dead.sort_unstable();
    let top: Vec<TreeID> = dead.iter().copied().filter(|n| !has_dead_ancestor(&tree, *n, &dead)).collect();
    for n in &top {
        tree.delete(*n).context(LoroSnafu)?;
    }
    if !top.is_empty() {
        doc.set_next_commit_message(COMPACT);
        doc.set_next_commit_origin(undo::SILENT);
        doc.commit();
    }
    Ok(top.len())
}

/// Vrai si `t` est une date ancienne : positive et au plus `limit`.
fn is_old(t: i64, limit: i64) -> bool {
    t > 0 && t <= limit
}

/// Vrai si `n` est mort avant `limit` (une marque, ou nœud vide daté).
fn is_dead(doc: &LoroDoc, tree: &LoroTree, n: TreeID, limit: i64) -> Result<bool, ReplicaInner> {
    let meta = tree.get_meta(n).context(LoroSnafu)?;
    if meta.is_empty() {
        let dated = tree.get_last_move_id(&n).and_then(|id| doc.get_change(id)).map(|c| c.timestamp);
        return Ok(dated.is_some_and(|t| is_old(t, limit)));
    }
    Ok(meta.keys().any(|k| mark_time(&meta, &k).is_some_and(|t| is_old(t, limit))))
}

/// Vrai si un ancêtre de `n` est dans `dead` (trié).
fn has_dead_ancestor(tree: &LoroTree, n: TreeID, dead: &[TreeID]) -> bool {
    let mut at = n;
    while let Some(TreeParentId::Node(p)) = tree.parent(at) {
        if dead.binary_search(&p).is_ok() {
            return true;
        }
        at = p;
    }
    false
}

/// L'horodatage positif le plus récent des changes de `doc`.
///
/// `None` si aucun n'est daté, ou si l'historique ne se parcourt pas.
fn newest_change(doc: &LoroDoc) -> Option<i64> {
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let mut newest: Option<i64> = None;
    doc.travel_change_ancestors(&heads, &mut |m| {
        if m.timestamp > 0 {
            newest = Some(newest.map_or(m.timestamp, |n| n.max(m.timestamp)));
        }
        std::ops::ControlFlow::Continue(())
    })
    .ok()?;
    newest
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod compact_tests;
```

- [ ] **Step 4 : `ReplicaLimits` et l'ouverture** (`mod.rs`)

```rust
/// Limites d'ouverture d'un replica relu d'un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaLimits {
    /// Taille maximale du snapshot (réglage `[sources] max_replica_mib`).
    pub max_bytes: usize,
    /// Délai de compaction des nœuds morts, en secondes ; 0 : jamais (t-770).
    pub compact_after_secs: i64,
}

impl From<usize> for ReplicaLimits {
    /// Une limite d'octets seule, sans compaction : tests et outils.
    fn from(max_bytes: usize) -> Self {
        Self { max_bytes, compact_after_secs: 0 }
    }
}
```

`from_snapshot(bytes: &[u8], limits: impl Into<ReplicaLimits>, registry)` appelle
`Self::import_snapshot(bytes, limits.into(), registry, time::OffsetDateTime::now_utc().unix_timestamp())`.
`import_snapshot(bytes, limits: ReplicaLimits, registry, now: i64)` : `max` devient
`limits.max_bytes` ; dans la fermeture de `guarded`, après `read_deck` :

```rust
            let read = read_deck(doc)?;
            // Sous la garde : la compaction lit les changes de Loro, non fiables.
            compact::compact_doc(doc, now, limits.compact_after_secs)?;
            Ok(read)
```

Doc de `from_snapshot` : « Relit un snapshot, puis compacte ses nœuds morts (t-770). » (corps :
octets non fiables, première lecture et compaction sous la garde ; un `usize` vaut une limite
d'octets sans compaction). `lib.rs` : `ReplicaLimits` dans `pub use replica::{…}` et dans le `//!`.

`history.rs`, `build` : après `let message = …;`, `if message == super::compact::COMPACT { continue; }`.
Doc du module : « Le commit de compaction (t-770) n'a pas de cran. »
`authors.rs`, `report_since` : la condition de Task 1 devient
`let skipped = meta.message.as_deref().is_some_and(|m| m.starts_with(super::PART) || m == super::compact::COMPACT);`.

- [ ] **Step 5 : vérifier** — `$CARGO nextest run -p oku-slides-core`
Expected : PASS.

- [ ] **Step 6 : ADR-0007** (`docs/adr/0007-compaction-des-noeuds-morts.md`), sur le modèle d'ADR-0006

- **Statut :** accepté ; **Date :** 2026-10-11 ; **Tâches :** t-770 ; **Remplace en partie :**
  ADR-0006 (« jamais `tree.delete` »).
- **Contexte :** les nœuds marqués s'accumulent ; lecture et `apply` en O(vivants + marqués).
- **Décision :** marque datée (`i64`, horloge de l'auteur ; Loro n'expose que le dernier auteur
  d'une clé) ; `limit = min(now, change le plus récent) − compact_after_days` ; compacté : un nœud
  à marque datée de `]0, limit]`, ou vide daté de `]0, limit]` ; une marque n'est donc compactée
  qu'après 30 jours d'**activité** du document ; à l'ouverture, sous la garde, avant la pile
  d'annulation ; commit silencieux `oku-compact` (Loro le date comme le change le plus récent), ni
  cran d'historique ni compte dans le rapport de fusion ; historique gardé entier.
- **Conséquences :** mesures de Task 5.
- **Limites :** une autre copie ouverte plus longtemps, qui annule alors sa marque ou le `mov`
  d'un élément sous ce nœud, perd ce nœud ; une marque remise par ⌘Z ou rétablir garde sa date
  d'origine ; une horloge d'auteur en retard raccourcit la fenêtre ; un fichier forgé peut faire
  compacter un nœud qu'il pouvait déjà cacher, ou porter un message `oku-compact` qui cache un
  change de l'historique et du rapport (son contenu reste visible dans les versions suivantes) ;
  le snapshot ne rétrécit pas.

ADR-0006 : en-tête « **Remplacé en partie par :** ADR-0007 (compaction) » ; **Limites**, « Les nœuds
marqués ne sont jamais libérés… » devient « Les nœuds marqués restent jusqu'à `compact_after_days`
d'activité (ADR-0007). » ; la puce du nœud vide (Task 1) devient « Un nœud vide naît d'un `mov`
défait dont la cible a été compactée (ADR-0007). » `docs/adr/README.md` : ligne d'ADR-0007.

- [ ] **Step 7 : commit** — `feat(replica) : compaction des nœuds morts à l'ouverture (t-770)`

### Task 4 : Réglage `compact_after_days` jusqu'aux lecteurs de fichiers

**Files :**
- Modify : `crates/oku-slides-platform/src/config.default.toml`, `crates/oku-slides-platform/src/config.rs`,
  `crates/oku-slides-platform/src/config_tests.rs`, `crates/oku-slides-core/src/sources.rs` (`SourceLimits`, `from_zip`),
  `crates/oku-slides-core/src/sources_tests.rs`, `crates/oku-slides-session/src/files.rs:111`,
  `crates/oku-slides-session/src/resume.rs` (`read_doc`, `restore` l. 631-635), `crates/oku-slides-session/src/resume_tests.rs`,
  `crates/oku-slides-cli/src/main.rs:314`, `docs/aide/fr.json`, `docs/aide/en.json`, littéraux `SourceLimits {`

**Interfaces :**
- Consumes : `ReplicaLimits` (Task 3).
- Produces : `SourcesSettings::compact_after_days: u32`, `SourcesSettings::compact_after_secs(&self) -> i64` ;
  `SourceLimits { replica_bytes, total_bytes, compact_after_secs: i64 }`, `SourceLimits::replica(&self) -> ReplicaLimits` ;
  `Resume::read_doc(&self, sub, defaults, limits: impl Into<ReplicaLimits>, registry)`.

- [ ] **Step 1 : tests qui échouent**

`config_tests.rs` (à côté des tests de `max_replica_bytes`) :

```rust
#[test]
fn compact_after_secs_by_default_is_thirty_days() {
    let settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    assert_that!(settings.sources.compact_after_secs()).is_equal_to(30 * 86_400);
}
```

(Si `Settings::defaults()` ne rend pas un `Result`, retirer le `unwrap_or_else`.)

`sources_tests.rs` :

```rust
/// Le replica de `roadmap` dont la première slide porte une marque datée de 1 (1970).
fn deck_loro_with_an_old_mark() -> (Vec<u8>, loro::TreeID) {
    let r = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&r.snapshot().unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = doc.get_map("deck").get("slides")
    else {
        panic!("arbre du deck attendu")
    };
    let slide = tree.roots()[0];
    tree.get_meta(slide).unwrap().insert("@deleted/7", 1_i64).unwrap();
    doc.commit();
    (doc.export(loro::ExportMode::Snapshot).unwrap(), slide)
}

fn deleted_in(bundle: &SourceBundle, node: loro::TreeID) -> bool {
    let doc = loro::LoroDoc::new();
    doc.import(&bundle.replica().snapshot().unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = doc.get_map("deck").get("slides")
    else {
        panic!("arbre du deck attendu")
    };
    tree.is_node_deleted(&node).unwrap()
}

#[test]
fn from_zip_compacts_a_node_marked_before_the_delay() {
    let (loro, slide) = deck_loro_with_an_old_mark();
    let bytes = zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]);
    let limits = SourceLimits { compact_after_secs: 5 * 86_400, ..MAX };
    let (bundle, _) = SourceBundle::from_zip(&bytes, limits, crate::test_registry::shared()).unwrap();
    assert_that!(deleted_in(&bundle, slide)).is_true();
}

#[test]
fn from_zip_with_a_zero_delay_compacts_nothing() {
    let (loro, slide) = deck_loro_with_an_old_mark();
    let bytes = zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]);
    let (bundle, _) = SourceBundle::from_zip(&bytes, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(deleted_in(&bundle, slide)).is_false();
}
```

`resume_tests.rs` :

```rust
/// La reprise compacte un nœud marqué avant le délai (t-770).
#[test]
fn read_doc_compacts_a_node_marked_before_the_delay() {
    let r = resume();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&document(), &mut Vec::new()).unwrap()).unwrap();
    let path = r.dir.join(DECK_DIR).join("deck.loro");
    let doc = loro::LoroDoc::new();
    doc.import(&std::fs::read(&path).unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = doc.get_map("deck").get("slides")
    else {
        panic!("arbre du deck attendu")
    };
    let slide = tree.roots()[0];
    tree.get_meta(slide).unwrap().insert("@deleted/7", 1_i64).unwrap();
    doc.commit();
    std::fs::write(&path, doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    let limits = oku_slides_core::ReplicaLimits { max_bytes: 1 << 24, compact_after_secs: 5 * 86_400 };
    let back = r.read_doc(DECK_DIR, &defaults(), limits, &registry()).unwrap();
    let raw = loro::LoroDoc::new();
    raw.import(&back.sources.replica().snapshot().unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = raw.get_map("deck").get("slides")
    else {
        panic!("arbre du deck attendu")
    };
    assert_that!(tree.is_node_deleted(&slide).unwrap()).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}
```

(Si le deck de `document()` n'a aucune slide, en ajouter une par `with_ops` comme ailleurs dans le fichier.)
Run : `$CARGO nextest run -p oku-slides-core -p oku-slides-session -p oku-slides-platform compact`
Expected : FAIL (compilation).

- [ ] **Step 2 : implémenter**

`config.default.toml`, `[sources]`, après `max_sources_mib` :

```toml
# Délai après lequel un élément ou une slide supprimés sont vraiment effacés à
# l'ouverture (jours d'activité du deck) : passé ce délai, une autre copie du
# deck ne peut plus annuler cette suppression. 0 : jamais.
compact_after_days = 30
```

`config.rs` : `const SECS_PER_DAY: i64 = 86_400;` ; `SourcesSettings` :

```rust
    /// Délai de compaction des nœuds supprimés (jours) ; 0 : jamais.
    pub compact_after_days: u32,
```

```rust
    /// [`Self::compact_after_days`] en secondes.
    pub fn compact_after_secs(&self) -> i64 {
        i64::from(self.compact_after_days).saturating_mul(SECS_PER_DAY)
    }
```

`sources.rs`, `SourceLimits` :

```rust
    /// Délai de compaction, en secondes (réglage `[sources] compact_after_days`) ; 0 : jamais.
    pub compact_after_secs: i64,
```

```rust
impl SourceLimits {
    /// Les limites d'ouverture du replica.
    pub fn replica(&self) -> ReplicaLimits {
        ReplicaLimits { max_bytes: self.replica_bytes, compact_after_secs: self.compact_after_secs }
    }
}
```

`from_zip` : `DeckReplica::from_snapshot(&data, limits.replica(), registry)` (`max` reste pour
`TooLarge`). `files.rs::source_limits` et `cli/src/main.rs` (l. 314) :
`compact_after_secs: settings.sources.compact_after_secs()` (`platform.settings…` dans la CLI).
`resume.rs::read_doc` : `max_replica_bytes: usize` devient `limits: impl Into<ReplicaLimits>` ;
`let limits = limits.into();` ; la lecture bornée utilise `limits.max_bytes` ;
`DeckReplica::from_snapshot(&bytes, limits, registry.clone())`. Doc : « …au plus
`limits.max_bytes`, compacté (`limits.compact_after_secs`)… ». `restore` (l. 631-635) passe
`self.source_limits.replica()` aux deux appels. Les appels de test de `read_doc` qui passent un
`usize` compilent sans changement.
Littéraux `SourceLimits {` (`grep -rn "SourceLimits {" crates`) : ajouter `compact_after_secs: 0`
à chacun qui n'utilise pas `..MAX`/`..LIMITS` (dont les `const MAX`/`LIMITS`).

Aide (`docs/aide/fr.json`, `docs/aide/en.json`, carte « Limites », texte « Historique : 32 Mio ;
sources décompressées : 256 Mio. Réglables dans [sources] de config.toml. ») : ajouter « Un élément
supprimé est effacé pour de bon après 30 jours d'activité du deck (compact_after_days). » / « A
deleted element is erased for good after 30 days of activity on the deck (compact_after_days). »
Si un test vérifie le contenu de l'aide, le mettre à jour.

- [ ] **Step 3 : vérifier** — `$CARGO nextest run --workspace`
Expected : PASS.

- [ ] **Step 4 : commit** — `feat(sources)! : compaction à l'ouverture, réglage compact_after_days (t-770)`

### Task 5 : Mesure de la compaction

**Files :**
- Modify : `crates/oku-slides-core/src/replica/compact_tests.rs`, `docs/adr/0007-compaction-des-noeuds-morts.md`

- [ ] **Step 1 : mesure**

```rust
/// Mesure t-770 : 40 slides × 20 éléments, ~10 000 nœuds marqués, avant et après compaction.
#[test]
#[ignore = "mesure"]
fn measure_compaction_cost_40x20_with_10000_marked() {
    let text = |id: String, t: &str| serde_json::json!({ "id": id, "kind": "text", "params": { "body": [{ "runs": [{ "t": t }] }] } });
    let slot = |v: Vec<serde_json::Value>| -> crate::ParamValue {
        crate::ParamValue::Slot(serde_json::from_value(serde_json::Value::Array(v)).unwrap_or_else(|e| panic!("{e}")))
    };
    let mut deck = roadmap();
    let template = deck.slides.iter().find(|s| s.id == "s-constat").cloned().unwrap_or_else(|| panic!("s-constat"));
    deck.slides = (0..40)
        .map(|i| {
            let mut s = template.clone();
            s.id = format!("s{i}");
            s.params.set("blocks", slot((0..20).map(|j| text(format!("e{i}-{j}"), "x")).collect()));
            s
        })
        .collect();
    let mut r = DeckReplica::new(&deck, registry()).unwrap_or_else(|e| panic!("{e}"));
    for round in 0..500 {
        let mut more = deck.clone();
        let s = &mut more.slides[round % 40];
        let mut blocks: Vec<serde_json::Value> = (0..20).map(|j| text(format!("e{}-{j}", round % 40), "x")).collect();
        blocks.extend((0..20).map(|j| text(format!("t{round}-{j}"), "y")));
        s.params.set("blocks", slot(blocks));
        r.apply(&more, "ajout", None).unwrap_or_else(|e| panic!("{e}"));
        r.apply(&deck, "retrait", None).unwrap_or_else(|e| panic!("{e}"));
    }
    let measure = |r: &DeckReplica, label: &str| {
        let t = std::time::Instant::now();
        let read = r.deck().0;
        let read_time = t.elapsed();
        let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
        let t = std::time::Instant::now();
        let (mut back, _) = DeckReplica::from_snapshot(&bytes, 1 << 26, registry()).unwrap_or_else(|e| panic!("{e}"));
        let open_time = t.elapsed();
        let mut renamed = read.clone();
        renamed.slides[0].name = Some("renommée".to_owned());
        let t = std::time::Instant::now();
        back.apply(&renamed, "titre", None).unwrap_or_else(|e| panic!("{e}"));
        let apply_time = t.elapsed();
        let mut other = DeckReplica::from_snapshot(&bytes, 1 << 26, registry()).unwrap_or_else(|e| panic!("{e}")).0;
        let t = std::time::Instant::now();
        other.merge(&back).unwrap_or_else(|e| panic!("{e}"));
        println!(
            "{label} : deck() {read_time:?}, from_snapshot {open_time:?}, apply {apply_time:?}, merge {:?}, snapshot {} o",
            t.elapsed(),
            bytes.len()
        );
    };
    measure(&r, "avant");
    activity_at(&r, now() + 31 * DAY);
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    let compacted_doc = loro::LoroDoc::new();
    compacted_doc.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let count = compact_doc(&compacted_doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"));
    println!("nœuds compactés : {count}");
    assert_that!(count).is_greater_than(9_000);
    let limits = ReplicaLimits { max_bytes: 1 << 26, compact_after_secs: DELAY };
    let (compacted, _) =
        DeckReplica::import_snapshot(&bytes, limits, registry(), now() + 31 * DAY).unwrap_or_else(|e| panic!("{e}"));
    measure(&compacted, "après");
}
```

Run : `$CARGO nextest run -p oku-slides-core --release --run-ignored only --no-capture measure_compaction_cost`.

- [ ] **Step 2 : ADR-0007, Conséquences** : les deux lignes de mesure et le nombre de nœuds compactés.

- [ ] **Step 3 : commit** — `docs(adr) : mesures de la compaction (t-770)`

---

## Lot 3 — t-809

### Task 6 : Sonde de Loro committée (go / no-go)

**Files :**
- Create : `crates/oku-slides-core/tests/loro_depth_probe.rs`

**Interfaces :** aucune. Les tests non `#[ignore]` fixent le comportement de Loro dont dépend Task 7 ;
un échec après une montée de Loro oblige à revoir t-809.

- [ ] **Step 1 : écrire la sonde**

```rust
//! Sonde t-809 : comment Loro 1.16.2 traite une chaîne profonde, et pourquoi
//! l'ouverture rejoue les opérations au lieu de lire l'état reçu.
//!
//! Mesures du 2026-10-11 (release, pile de 256 Kio, 100 000 niveaux) :
//! un snapshot importé récurse au premier accès, dans `is_node_deleted`, à
//! l'export, au `fork` et au drop ; un document **rejoué** (updates importées
//! d'un document dont l'état n'est jamais lu) ne récurse ni au premier accès,
//! ni en remontant `parent()`, ni au drop — seul l'export d'un arbre profond
//! déborde ; une version passée profonde relue par `updates_till` passe ;
//! 50 000 cartes imbriquées rejouées passent l'export, le `fork` et le drop ;
//! un snapshot *shallow* n'exporte pas `all_updates`. Coût du rejeu : 100 000
//! ops 60 ms (import direct 4 ms), 1 M ops 0,8 s (5 ms).
//!
//! Les tests sans `#[ignore]` vérifient ces faits sur une pile réduite ; les
//! chaînes sont construites sur une grande pile (leur export récurse).

use std::process::Command;

use loro::{Container, ExportMode, LoroDoc, LoroMap, LoroTree, TreeParentId, ValueOrContainer};
use speculoos::prelude::*;

const N: usize = 100_000;

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement sur une pile de {size} o"))
}

fn deck_tree(doc: &LoroDoc) -> LoroTree {
    match doc.get_map("deck").get("slides") {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        other => panic!("arbre attendu : {other:?}"),
    }
}

/// Snapshot d'une chaîne de `n` niveaux sans méta ; tête supprimée si `delete_head`.
fn chain(n: usize, delete_head: bool) -> Vec<u8> {
    on_stack(1 << 30, move || {
        let doc = LoroDoc::new();
        let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
        let mut parent = TreeParentId::Root;
        let mut head = None;
        for _ in 0..n {
            let node = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            head.get_or_insert(node);
            parent = TreeParentId::Node(node);
        }
        if let (true, Some(h)) = (delete_head, head) {
            tree.delete(h).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    })
}

/// Le document rejoué des opérations de `bytes` ; l'état reçu n'est jamais lu.
fn replayed(bytes: &[u8]) -> LoroDoc {
    let received = LoroDoc::new();
    received.import(bytes).unwrap_or_else(|e| panic!("{e}"));
    let updates = received.export(ExportMode::all_updates()).unwrap_or_else(|e| panic!("{e}"));
    drop(received);
    let doc = LoroDoc::new();
    doc.import(&updates).unwrap_or_else(|e| panic!("{e}"));
    doc
}

/// Profondeur d'une chaîne : remontée depuis le nœud créé en dernier (linéaire).
fn depth(tree: &LoroTree) -> usize {
    let Some(mut at) = tree.nodes().into_iter().max_by_key(|n| n.counter) else { return 0 };
    let mut d = 1;
    while let Some(TreeParentId::Node(p)) = tree.parent(at) {
        at = p;
        d += 1;
    }
    d
}

#[test]
fn replayed_deep_chain_is_measured_and_dropped_on_a_small_stack() {
    for delete_head in [false, true] {
        let bytes = chain(N, delete_head);
        let d = on_stack(256 * 1024, move || {
            let doc = replayed(&bytes);
            let d = depth(&deck_tree(&doc));
            drop(doc);
            d
        });
        assert_that!(d).named(&format!("tête supprimée : {delete_head}")).is_equal_to(N);
    }
}

#[test]
fn replayed_past_version_of_a_flattened_chain_is_read_on_a_small_stack() {
    // Les octets, et la version juste après la chaîne (avant toute remontée).
    let (bytes, chained) = on_stack(1 << 30, || {
        let doc = LoroDoc::new();
        let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
        let mut parent = TreeParentId::Root;
        let mut nodes = Vec::new();
        for _ in 0..N {
            let node = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            nodes.push(node);
            parent = TreeParentId::Node(node);
        }
        doc.commit();
        let chained = doc.oplog_frontiers();
        for n in nodes.iter().rev() {
            tree.mov(*n, TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        (bytes, chained)
    });
    let d = on_stack(256 * 1024, move || {
        let doc = replayed(&bytes);
        // Les ID survivent au rejeu : la version se retrouve par sa frontière.
        let vv = doc.frontiers_to_vv(&chained).unwrap_or_else(|| panic!("version inconnue"));
        let version = LoroDoc::new();
        version
            .import(&doc.export(ExportMode::updates_till(&vv)).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
        let d = depth(&deck_tree(&version));
        drop(version);
        drop(doc);
        d
    });
    assert_that!(d).is_equal_to(N);
}

#[test]
fn replayed_nested_maps_export_fork_and_drop_on_a_small_stack() {
    let bytes = on_stack(1 << 30, || {
        let doc = LoroDoc::new();
        let mut m: LoroMap = doc.get_map("deck").insert_container("x", LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..50_000 {
            m = m.insert_container("x", LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    });
    on_stack(256 * 1024, move || {
        let doc = replayed(&bytes);
        drop(doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}")));
        drop(doc.fork());
        drop(doc);
    });
}

#[test]
fn shallow_snapshot_is_detected_and_cannot_export_all_updates() {
    let doc = LoroDoc::new();
    doc.get_map("deck").insert("k", 1).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::shallow_snapshot(&doc.oplog_frontiers())).unwrap_or_else(|e| panic!("{e}"));
    let received = LoroDoc::new();
    received.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(received.is_shallow()).is_true();
    assert_that!(received.export(ExportMode::all_updates()).is_err()).is_true();
}

/// Enfant : importe `PROBE_FILE` et y accède sur une pile de 256 Kio (sans rejeu).
#[test]
#[ignore = "lancé par la mesure, dans un processus enfant"]
fn probe_child_reads_a_received_snapshot() {
    let file = std::env::var("PROBE_FILE").unwrap_or_else(|e| panic!("{e}"));
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{e}"));
    on_stack(256 * 1024, move || {
        let doc = LoroDoc::new();
        doc.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
        let _ = deck_tree(&doc).nodes();
        std::mem::forget(doc);
    });
}

/// Mesure : lire l'état d'un snapshot reçu déborde (d'où le rejeu).
#[test]
#[ignore = "mesure"]
fn measure_received_snapshot_first_access_overflows() {
    let path = std::env::temp_dir().join(format!("oku-depth-probe-{}.loro", std::process::id()));
    std::fs::write(&path, chain(N, false)).unwrap_or_else(|e| panic!("{e}"));
    let exe = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let ok = Command::new(exe)
        .args(["probe_child_reads_a_received_snapshot", "--exact", "--ignored", "--test-threads=1"])
        .env("PROBE_FILE", &path)
        .output()
        .is_ok_and(|o| o.status.success());
    std::fs::remove_file(&path).ok();
    println!("premier accès d'un snapshot reçu (100 000 niveaux, 256 Kio) : {}", if ok { "OK" } else { "déborde" });
}
```

- [ ] **Step 2 : lancer** — `$CARGO nextest run -p oku-slides-core --release --test loro_depth_probe`
puis `$CARGO nextest run -p oku-slides-core --test loro_depth_probe` (debug).
Expected : PASS dans les deux profils. **Si un test non `#[ignore]` échoue, s'arrêter et le
signaler** (l'option D repose sur ces faits). Puis la mesure :
`$CARGO nextest run -p oku-slides-core --release --test loro_depth_probe --run-ignored only --no-capture measure_`
Expected : « déborde ».

- [ ] **Step 3 : commit** — `test(replica) : sonde de récursion de Loro, rejeu des opérations (t-809)`

### Task 7 : Ouverture par rejeu des opérations, un seul arbre, profondeur bornée

**Files :**
- Create : `crates/oku-slides-core/src/replica/entry.rs`, `crates/oku-slides-core/src/replica/entry_tests.rs`,
  `crates/oku-slides-core/tests/regressions/issue_0809_deep_tree_chain.rs`
- Modify : `crates/oku-slides-core/src/replica/mod.rs` (`guarded` déplacé, `check_trees`, `import_snapshot`, imports),
  `crates/oku-slides-core/src/replica/tree/deck_tree_tests.rs`, `crates/oku-slides-core/tests/regressions/main.rs`,
  `docs/adr/0006-deck-en-un-seul-arbre-suppression-douce.md`

**Interfaces :**
- Produces (`replica::entry`) :
  - `pub(in crate::replica) const MAX_TREE_DEPTH: usize = 1024;`
  - `pub(super) fn guarded<T>(doc: LoroDoc, f: impl FnOnce(&LoroDoc) -> Result<T, ReplicaInner>) -> Result<(LoroDoc, T), ReplicaInner>` (déplacé de `mod.rs`, inchangé)
  - `pub(super) fn replayable_updates(bytes: &[u8]) -> Result<Vec<u8>, ReplicaInner>`
  - `pub(in crate::replica) fn check_entry(doc: &LoroDoc, tree: Option<&LoroTree>) -> Result<HashSet<TreeID>, ReplicaInner>`
    : aucun arbre hors de `tree` (tout arbre est étranger si `tree` vaut `None`), puis profondeur
    de tous les nœuds ; rend les nœuds supprimés par Loro (calcul linéaire, sans `is_node_deleted`).
  - `pub(in crate::replica) fn deleted_nodes(tree: &LoroTree) -> Result<HashSet<TreeID>, ReplicaInner>`
    : la même passe, seule (profondeur bornée comprise).
- Modifie : `check_trees` et `compact_doc` lisent l'ensemble des supprimés au lieu de boucler
  sur `is_node_deleted` (O(nœuds × profondeur) : 29 s pour un balai d'un million de feuilles).

- [ ] **Step 1 : tests unitaires qui échouent** (`entry_tests.rs`)

```rust
//! Bornes d'entrée : rejeu, arbre unique, profondeur, supprimés (t-809).

use loro::{ExportMode, LoroDoc, LoroTree, TreeParentId};
use speculoos::prelude::*;

use super::*;
use crate::error::ReplicaInner;

/// Un document dont `deck.slides` est une chaîne de `n` niveaux.
fn chain_doc(n: usize) -> (LoroDoc, LoroTree) {
    let doc = LoroDoc::new();
    let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
    let mut parent = TreeParentId::Root;
    for _ in 0..n {
        parent = TreeParentId::Node(tree.create(parent).unwrap_or_else(|e| panic!("{e}")));
    }
    doc.commit();
    (doc, tree)
}

fn refused<T>(r: Result<T, ReplicaInner>, what: &str) -> bool {
    matches!(r, Err(ReplicaInner::MalformedDeckTree { what: w, .. }) if w == what)
}

#[test]
fn check_entry_at_the_depth_bound_is_accepted() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH);
    assert_that!(check_entry(&doc, Some(&tree))).is_ok();
}

#[test]
fn check_entry_one_level_past_the_bound_is_refused() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    assert_that!(refused(check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_counts_nodes_deleted_by_loro() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH + 1);
    tree.delete(tree.roots()[0]).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    // Sous la racine des supprimés, la chaîne garde MAX + 1 niveaux.
    assert_that!(refused(check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_deleted_chain_at_the_bound_is_accepted_and_reported_deleted() {
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH);
    tree.delete(tree.roots()[0]).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let deleted = check_entry(&doc, Some(&tree)).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(deleted.len()).is_equal_to(MAX_TREE_DEPTH);
}

#[test]
fn deleted_nodes_matches_is_node_deleted_on_a_mixed_tree() {
    let (doc, tree) = chain_doc(10);
    let root = tree.roots()[0];
    let kept = tree.create(TreeParentId::Node(root)).unwrap_or_else(|e| panic!("{e}"));
    let gone = tree.create(TreeParentId::Node(root)).unwrap_or_else(|e| panic!("{e}"));
    tree.create(TreeParentId::Node(gone)).unwrap_or_else(|e| panic!("{e}"));
    tree.delete(gone).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let deleted = deleted_nodes(&tree).unwrap_or_else(|e| panic!("{e}"));
    for n in tree.nodes() {
        let loro_says = tree.is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(deleted.contains(&n)).named(&format!("{n:?}")).is_equal_to(loro_says);
    }
    assert_that!(deleted.contains(&kept)).is_false();
    assert_that!(deleted.contains(&gone)).is_true();
}

#[test]
fn check_entry_branch_past_the_bound_is_refused_whatever_the_visit_order() {
    // Un tronc de MAX - 1, deux feuilles à la borne, puis un enfant sous l'une :
    // l'une des deux remontées passe forcément par le mémo de l'autre.
    let (doc, tree) = chain_doc(MAX_TREE_DEPTH - 1);
    let tip = tree.nodes().into_iter().max_by_key(|n| n.counter).unwrap_or_else(|| panic!("nœud"));
    let a = tree.create(TreeParentId::Node(tip)).unwrap_or_else(|e| panic!("{e}"));
    tree.create(TreeParentId::Node(tip)).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(check_entry(&doc, Some(&tree))).is_ok();
    tree.create(TreeParentId::Node(a)).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(check_entry(&doc, Some(&tree)), TOO_DEEP)).is_true();
}

#[test]
fn check_entry_with_a_second_tree_is_refused() {
    let (doc, tree) = chain_doc(1);
    doc.get_tree("autre").create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(check_entry(&doc, Some(&tree)), FOREIGN_TREE)).is_true();
}

#[test]
fn check_entry_without_a_deck_tree_refuses_any_tree() {
    let doc = LoroDoc::new();
    doc.get_tree("autre").create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    assert_that!(refused(check_entry(&doc, None), FOREIGN_TREE)).is_true();
}

#[test]
fn replayable_updates_of_a_shallow_snapshot_is_refused() {
    let doc = LoroDoc::new();
    doc.get_map("deck").insert("k", 1).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::shallow_snapshot(&doc.oplog_frontiers())).unwrap_or_else(|e| panic!("{e}"));
    let r = replayable_updates(&bytes);
    assert_that!(matches!(r, Err(ReplicaInner::NotADeck { what: SHALLOW, .. }))).is_true();
}

#[test]
fn replayable_updates_rebuild_the_same_state() {
    let (doc, _) = chain_doc(3);
    doc.get_map("deck").insert("titre", "x").unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
    let rebuilt = LoroDoc::new();
    rebuilt.import(&replayable_updates(&bytes).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    // Document de test, peu profond et construit ici : `get_deep_value` y est sûr.
    assert_that!(rebuilt.get_deep_value()).is_equal_to(doc.get_deep_value());
}
```

Ajouter dans `entry.rs` : `#[cfg(test)] #[path = "entry_tests.rs"] mod entry_tests;`.

- [ ] **Step 2 : vérifier l'échec** — `$CARGO nextest run -p oku-slides-core entry_tests`
Expected : erreur de compilation (`entry` absent).

- [ ] **Step 3 : `entry.rs`**

```rust
//! L'entrée des octets non fiables : rejeu, arbre unique, profondeur bornée (t-809).
//!
//! Loro 1.16.2 récurse sur la profondeur d'un arbre : au premier accès de
//! l'état d'un snapshot, puis, même supprimé, dans `is_node_deleted`, l'export
//! et `fork`. Un débordement de pile n'est pas une panique. L'état d'un snapshot
//! n'est pas vérifié par Loro contre son historique : il peut être forgé.
//!
//! L'état reçu n'est donc jamais lu : ses opérations sont rejouées dans un
//! document neuf, qui se mesure sans récursion (sonde `tests/loro_depth_probe.rs`).
//! [`check_entry`] refuse ensuite tout arbre autre que `deck.slides` (tout arbre
//! s'il manque), puis tout nœud, supprimé ou non, au-delà de [`MAX_TREE_DEPTH`],
//! et rend les nœuds supprimés : plus de `is_node_deleted` en boucle, en
//! O(nœuds × profondeur). Un document accepté ne déborde plus : export, `fork`
//! et drop d'un arbre de 1024 niveaux tiennent dans ~1 Mio de pile, sous les
//! 2 Mio des fils de l'app.
//!
//! Résidus : les valeurs plates imbriquées (t-223) ; ne jamais appeler
//! `get_deep_value` sur un document reçu ; au-delà de 65 535 conteneurs
//! imbriqués, Loro panique (rattrapé par [`guarded`]) ; le rejeu d'un très grand
//! historique est lent (un million de nœuds : ~10 s).

use std::collections::{HashMap, HashSet};
use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{ContainerTrait, ContainerType, ExportMode, LoroDoc, LoroTree, TreeID, TreeParentId};
use snafu::{ResultExt, ensure};

use crate::error::{ExportSnafu, ImportPanickedSnafu, ImportSnafu, MalformedDeckTreeSnafu, NotADeckSnafu, ReplicaInner};

/// Profondeur maximale de l'arbre du deck ; une racine vaut 1.
///
/// Une suppression garde son sous-arbre caché, qu'un `mov` emporte : la
/// profondeur réelle dépasse celle des seuls nœuds vivants (`MAX_SLOT_DEPTH`).
pub(in crate::replica) const MAX_TREE_DEPTH: usize = 1024;

pub(super) const TOO_DEEP: &str = "deck tree too deep";
pub(super) const FOREIGN_TREE: &str = "a tree outside deck.slides";
pub(super) const SHALLOW: &str = "shallow snapshot";

/// Exécute `f` sur `doc` sous `catch_unwind` ; une panique devient `ImportPanicked`.
///
/// Après une panique, les verrous de `doc` sont empoisonnés et son drop
/// paniquerait à nouveau : `doc` est abandonné sans drop (fuite assumée).
pub(super) fn guarded<T>(doc: LoroDoc, f: impl FnOnce(&LoroDoc) -> Result<T, ReplicaInner>) -> Result<(LoroDoc, T), ReplicaInner> {
    match catch_unwind(AssertUnwindSafe(|| f(&doc))) {
        Ok(r) => r.map(|t| (doc, t)),
        Err(_) => {
            std::mem::forget(doc);
            ImportPanickedSnafu.fail()
        }
    }
}

/// Les opérations d'un snapshot reçu, sans jamais lire son état.
///
/// Refuse un snapshot *shallow* (ses opérations ne se rejouent pas). À appeler
/// sous [`guarded`] : une panique laisse fuir le document reçu, jamais libéré
/// par le déroulement.
pub(super) fn replayable_updates(bytes: &[u8]) -> Result<Vec<u8>, ReplicaInner> {
    let received = ManuallyDrop::new(LoroDoc::new());
    let out = (|| {
        received.import(bytes).context(ImportSnafu)?;
        ensure!(!received.is_shallow(), NotADeckSnafu { what: SHALLOW });
        received.export(ExportMode::all_updates()).context(ExportSnafu)
    })();
    // L'état reçu n'a pas été décodé : son drop ne récurse pas.
    drop(ManuallyDrop::into_inner(received));
    out
}

/// Contrôle d'entrée d'un document rejoué ; rend ses nœuds supprimés par Loro.
///
/// Refuse tout arbre hors de `tree` (tout arbre si `tree` vaut `None`), puis
/// tout nœud au-delà de [`MAX_TREE_DEPTH`]. À appeler en premier, jamais sur
/// un état reçu.
pub(in crate::replica) fn check_entry(doc: &LoroDoc, tree: Option<&LoroTree>) -> Result<HashSet<TreeID>, ReplicaInner> {
    check_single_tree(doc, tree)?;
    tree.map_or_else(|| Ok(HashSet::new()), deleted_nodes)
}

/// Refuse tout arbre touché par l'historique autre que `tree`.
fn check_single_tree(doc: &LoroDoc, tree: Option<&LoroTree>) -> Result<(), ReplicaInner> {
    let deck = tree.map(ContainerTrait::id);
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let mut changes: Vec<(loro::ID, usize)> = Vec::new();
    doc.travel_change_ancestors(&heads, &mut |m| {
        changes.push((m.id, m.len));
        std::ops::ControlFlow::Continue(())
    })
    .map_err(|_| NotADeckSnafu { what: "unreadable history" }.build())?;
    for (id, len) in changes {
        for c in doc.get_changed_containers_in(id, len) {
            ensure!(
                c.container_type() != ContainerType::Tree || Some(&c) == deck.as_ref(),
                MalformedDeckTreeSnafu { what: FOREIGN_TREE }
            );
        }
    }
    Ok(())
}

/// Les nœuds de `tree` supprimés par Loro ; refuse un nœud au-delà de [`MAX_TREE_DEPTH`].
///
/// Remonte `parent()` (une lecture de table) jusqu'à un ancêtre mesuré, la
/// racine ou celle des supprimés ; linéaire en nœuds. Jamais
/// `is_node_deleted`, qui récurse et coûte la profondeur à chaque appel.
pub(in crate::replica) fn deleted_nodes(tree: &LoroTree) -> Result<HashSet<TreeID>, ReplicaInner> {
    let nodes = tree.nodes();
    // Par nœud : profondeur, et s'il est sous la racine des supprimés.
    let mut seen: HashMap<TreeID, (usize, bool)> = HashMap::with_capacity(nodes.len());
    let mut chain: Vec<TreeID> = Vec::new();
    for node in nodes {
        chain.clear();
        let mut at = node;
        let (base, deleted) = loop {
            if let Some(known) = seen.get(&at) {
                break *known;
            }
            chain.push(at);
            ensure!(chain.len() <= MAX_TREE_DEPTH, MalformedDeckTreeSnafu { what: TOO_DEEP });
            match tree.parent(at) {
                Some(TreeParentId::Node(p)) => at = p,
                Some(TreeParentId::Deleted) => break (0, true),
                _ => break (0, false),
            }
        };
        ensure!(base + chain.len() <= MAX_TREE_DEPTH, MalformedDeckTreeSnafu { what: TOO_DEEP });
        for (i, n) in chain.iter().rev().enumerate() {
            seen.insert(*n, (base + i + 1, deleted));
        }
    }
    Ok(seen.into_iter().filter(|(_, (_, deleted))| *deleted).map(|(n, _)| n).collect())
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
```

(Vérifier en 1.16.2 : `TreeParentId::Deleted` est bien la variante rendue par `parent()` pour un
enfant direct de la racine des supprimés ; `tree.id()` et `get_changed_containers_in` rendent des
`ContainerID` comparables ; `TreeID` est `Ord`. `deleted_nodes_matches_is_node_deleted…` le
vérifie à l'exécution.)

`mod.rs` :
- supprimer `fn guarded` (l. 161-192) et son doc, et les imports devenus inutiles
  (`AssertUnwindSafe`, `catch_unwind`, `ImportPanickedSnafu` s'ils ne servent plus) ; `mod entry;` ;
  `use entry::{check_entry, guarded, replayable_updates};`.
- `check_trees` devient (le contrôle d'entrée passe **avant** tout, même sans `deck.slides`) :

```rust
fn check_trees(doc: &LoroDoc) -> Result<(), ReplicaInner> {
    let tree = match doc.get_map(ROOT).get(SLIDES) {
        None => None,
        Some(ValueOrContainer::Container(Container::Tree(tree))) => Some(tree),
        Some(_) => return MalformedDeckTreeSnafu { what: "slides are not a tree" }.fail(),
    };
    // En premier, même sans arbre du deck : arbre unique, profondeur bornée (t-809).
    let deleted = check_entry(doc, tree.as_ref())?;
    let Some(tree) = tree else { return Ok(()) };
    for node in tree.nodes() {
        if deleted.contains(&node) {
            continue;
        }
        // … corps inchangé (méta, marques, `@slide`, `@slot`)
    }
    Ok(())
}
```

  Doc : ajouter « un arbre autre que `deck.slides` (tout arbre s'il manque), ou un nœud, même
  supprimé, au-delà de `MAX_TREE_DEPTH` (contrôlés en premier, sur un document rejoué) ».
- `compact.rs`, `compact_doc` : remplacer la boucle sur `is_node_deleted` par
  `let deleted = super::entry::deleted_nodes(&tree)?;` puis `if !deleted.contains(&n) && is_dead(…)?`,
  et le `tree.delete` des nœuds de `top` ne change pas. Retirer le commentaire « Profondeur bornée
  à l'entrée : `is_node_deleted` est sûr ici ».
- `import_snapshot`, dans la fermeture de `guarded` :

```rust
        let (doc, read) = guarded(doc, |doc| {
            // L'état reçu n'est jamais lu : le document est rejoué de ses opérations (t-809).
            doc.import(&replayable_updates(bytes)?).context(ImportSnafu)?;
            ensure!(!uid_of(doc).is_empty(), NotADeckSnafu { what: "no deck_uid" });
            check_format(doc)?;
            check_trees(doc)?;
            let read = read_deck(doc)?;
            compact::compact_doc(doc, now, limits.compact_after_secs)?;
            Ok(read)
        })?;
```

- `merge_inner` : l'essai passe déjà par `check_trees` (donc par `check_entry`) ; ajouter au
  commentaire : « L'autre replica a été ouvert par rejeu ; l'essai refuse un arbre étranger ou trop
  profond. L'import final dans `self` n'en récurse pas (sonde t-809). »
- Doc de `from_snapshot` : « … l'état du snapshot n'est jamais lu : ses opérations sont rejouées
  (t-809). »

- [ ] **Step 4 : régression** (`tests/regressions/issue_0809_deep_tree_chain.rs`)

```rust
//! t-809 : une chaîne forgée de milliers de niveaux dans l'arbre du deck
//! faisait déborder la pile dans Loro et avorter le processus. L'état d'un
//! snapshot reçu n'est plus lu : ses opérations sont rejouées, puis bornées.

use loro::{Container, ExportMode, LoroDoc, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

use crate::common;

/// Profondeur maximale de l'arbre du deck (`replica::entry::MAX_TREE_DEPTH`).
const MAX_TREE_DEPTH: usize = 1024;
const SMALL: usize = 256 * 1024;
const APP_THREAD: usize = 2 << 20;
const BIG: usize = 1 << 30;

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement sur une pile de {size} o"))
}

fn slide_node(tree: &LoroTree, id: &str) -> TreeID {
    tree.roots()
        .into_iter()
        .find(|n| tree.get_meta(*n).is_ok_and(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id)))
        .unwrap_or_else(|| panic!("slide {id} absente"))
}

/// Le snapshot de `roadmap` après `edit` sur son document brut ; construit sur une grande pile.
fn forged(edit: impl FnOnce(&LoroDoc, &LoroTree) + Send + 'static, shallow: bool) -> Vec<u8> {
    on_stack(BIG, move || {
        let deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("{e}"));
        let r = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("{e}"));
        let doc = LoroDoc::new();
        doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
        let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
            panic!("arbre du deck attendu")
        };
        edit(&doc, &tree);
        doc.commit();
        let mode = if shallow { ExportMode::shallow_snapshot(&doc.oplog_frontiers()) } else { ExportMode::Snapshot };
        let bytes = doc.export(mode).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    })
}

/// Une chaîne de `n` nœuds sans méta sous `parent` ; rend sa tête.
fn chain(tree: &LoroTree, parent: TreeParentId, n: usize) -> Option<TreeID> {
    let (mut at, mut head) = (parent, None);
    for _ in 0..n {
        let node = tree.create(at).unwrap_or_else(|e| panic!("{e}"));
        head.get_or_insert(node);
        at = TreeParentId::Node(node);
    }
    head
}

/// Ouvre `bytes` sur une pile de `stack` octets ; en cas de succès, enregistre puis libère sur cette pile.
fn open_on(stack: usize, bytes: Vec<u8>) -> Result<(), ReplicaErrorKind> {
    on_stack(stack, move || {
        let (r, _) = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).map_err(|e| e.kind())?;
        r.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        drop(r);
        Ok(())
    })
}

#[test]
fn from_snapshot_deep_live_chain_is_refused_on_a_small_stack() {
    let bytes = forged(|_, tree| { chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), 20_000); }, false);
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_deep_chain_under_a_deleted_head_is_refused_on_a_small_stack() {
    let bytes = forged(
        |_, tree| {
            if let Some(head) = chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), 20_000) {
                tree.delete(head).unwrap_or_else(|e| panic!("{e}"));
            }
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_with_a_deep_tree_outside_the_deck_is_refused_on_a_small_stack() {
    let bytes = forged(|doc, _| { chain(&doc.get_tree("autre"), TreeParentId::Root, 20_000); }, false);
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_shallow_is_refused() {
    let bytes = forged(|_, _| {}, true);
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_chain_at_the_bound_opens_saves_and_drops_on_an_app_thread() {
    // La slide est à la profondeur 1 : MAX - 1 niveaux finissent à la borne.
    let bytes = forged(|_, tree| { chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH - 1); }, false);
    assert_that!(open_on(APP_THREAD, bytes)).is_ok();
}

#[test]
fn from_snapshot_one_level_past_the_bound_is_refused() {
    let bytes = forged(|_, tree| { chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH); }, false);
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_without_deck_slides_and_a_deep_foreign_tree_is_refused() {
    let bytes = forged(
        |doc, _| {
            doc.get_map("deck").delete("slides").unwrap_or_else(|e| panic!("{e}"));
            chain(&doc.get_tree("autre"), TreeParentId::Root, 20_000);
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

/// Témoin : le même chemin de forge, sans rien ajouter, s'ouvre sur une petite pile.
#[test]
fn from_snapshot_of_an_untouched_forge_opens_on_a_small_stack() {
    assert_that!(open_on(SMALL, forged(|_, _| {}, false))).is_ok();
}

/// À la borne, fusion, restauration et historique tiennent sur un fil de l'app (2 Mio).
#[test]
fn replica_at_the_bound_merges_restores_and_reads_history_on_an_app_thread() {
    let bytes = forged(|_, tree| { chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH - 1); }, false);
    on_stack(APP_THREAD, move || {
        let open = || DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).unwrap_or_else(|e| panic!("{e}")).0;
        let (mut mine, mut theirs) = (open(), open());
        let mut renamed = theirs.deck().0;
        renamed.slides[0].name = Some("renommée".to_owned());
        theirs.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
        mine.merge(&theirs).unwrap_or_else(|e| panic!("merge : {e}"));
        let history = mine.history();
        let first = history.deck_at(0).unwrap_or_else(|e| panic!("version : {e}")).0;
        mine.restore(&first, "Restaurer").unwrap_or_else(|e| panic!("restore : {e}"));
        mine.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        drop((mine, theirs, history));
    });
}
```

Ajouter `mod issue_0809_deep_tree_chain;` à `tests/regressions/main.rs`.

- [ ] **Step 5 : fusion d'un pair forgé** (`deck_tree_tests.rs`, à côté de `forge_deep_chain`)

```rust
/// Exécute `f` sur un fil à pile de `size` octets.
fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement de pile"))
}

/// Un pair forgé en mémoire (jamais ouvert) : la fusion est refusée sur une petite pile, `self` intact (t-809).
#[test]
fn merge_of_a_forged_peer_too_deep_or_with_a_foreign_tree_is_refused() {
    for foreign in [false, true] {
        let mine = new(&roadmap());
        let snap = ok(mine.snapshot());
        let other = on_stack(1 << 30, move || {
            let other = ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
            if foreign {
                let tree = other.doc.get_tree("autre");
                let mut at = TreeParentId::Root;
                for _ in 0..20_000 {
                    at = TreeParentId::Node(ok(tree.create(at)));
                }
                other.doc.commit();
            } else {
                forge_deep_chain(&other, 20_000);
            }
            other
        });
        let before = mine.deck().0;
        let (mine, other, kind) = on_stack(256 * 1024, move || {
            let mut mine = mine;
            let kind = mine.merge(&other).err().map(|e| e.kind());
            (mine, other, kind)
        });
        assert_that!(kind).named(&format!("arbre étranger : {foreign}")).is_equal_to(Some(ReplicaErrorKind::Corrupted));
        assert_that!(mine.deck().0).is_equal_to(before);
        // Le pair forgé se libère sur une grande pile (son état n'est pas rejoué).
        on_stack(1 << 30, move || drop(other));
    }
}
```

`deep_element_chain_in_the_deck_tree_reads_and_applies_in_a_small_stack` (l. 764-793) reste à
300 niveaux (sous la borne) ; mettre à jour son doc : « …le premier accès se fait sur un document
rejoué (t-809) ». Chercher les autres tests qui forgent un arbre de plus de 1024 niveaux
(`grep -rn "forge_deep_chain\|_000" crates/oku-slides-core/src crates/oku-slides-core/tests`) et
attendre `Corrupted` s'ils l'ouvrent ou le fusionnent.

- [ ] **Step 6 : vérifier**

Commenter provisoirement `let deleted = check_entry(doc, tree.as_ref())?;` (et le remplacer par
`let deleted = HashSet::new();`) :
`$CARGO nextest run -p oku-slides-core --test regressions issue_0809`
Expected : au moins `…deleted_head…`, `…outside_the_deck…`, `…without_deck_slides…` échouent ou le
processus avorte.
Remettre la ligne ; puis `$CARGO nextest run -p oku-slides-core -p oku-slides-session`
Expected : PASS.

- [ ] **Step 7 : ADR-0006**

**Décision → Entrée stricte**, ajouter : « L'état d'un snapshot reçu n'est jamais lu : ses
opérations sont rejouées dans un document neuf (un snapshot *shallow* est refusé). Sur ce
document, avant tout autre contrôle : aucun arbre autre que `deck.slides` ; tous les nœuds,
supprimés compris, à 1024 niveaux au plus (t-809, sonde `tests/loro_depth_probe.rs`). »
**Limites** : « L'ouverture rejoue tout l'historique (chaîne de 100 000 ops : ~60 ms ; 1 M : ~0,8 s ;
arbre large d'un million de nœuds : ~10 s). Les
valeurs plates imbriquées (t-223) restent hors borne. » Ajouter t-809 à **Tâches**.

- [ ] **Step 8 : commit** — `fix(replica)! : l'ouverture rejoue les opérations et borne l'arbre du deck (t-809)`

### Task 8 : Version passée par rejeu, et ARCHITECTURE

**Files :**
- Modify : `crates/oku-slides-core/src/replica/history.rs` (`deck_at`), `crates/oku-slides-core/src/replica/history_tests.rs`,
  `crates/oku-slides-core/src/replica/ARCHITECTURE.md`

**Interfaces :**
- Consumes : `check_trees` (avec `check_entry`), `configure`, `read_deck`.
- Produces : `HistoryPreview::deck_at` inchangé de signature ; plus de `checkout` ; une version trop
  profonde rend `Corrupted`, les autres restent lisibles.

- [ ] **Step 1 : test qui échoue** (`history_tests.rs`)

```rust
fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement de pile"))
}

/// Un replica dont l'historique porte une chaîne de 20 000 nœuds vides sous
/// `s-constat` (versions « forge »), remontés ensuite sous la slide (« aplatie »).
fn forged_then_flattened() -> crate::DeckReplica {
    on_stack(1 << 30, || {
        let r = crate::DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap_or_else(|e| panic!("{e}"));
        let tree = crate::replica::tree::deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
        let points = crate::replica::tree::node_of(&r.doc, "s-constat", "b-points").1;
        let Some(loro::TreeParentId::Node(slide)) = tree.parent(points) else { panic!("slide de b-points") };
        let mut parent = loro::TreeParentId::Node(slide);
        let mut chain = Vec::new();
        for _ in 0..20_000 {
            let n = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            chain.push(n);
            parent = loro::TreeParentId::Node(n);
        }
        r.doc.set_next_commit_message("forge");
        r.doc.commit();
        for n in chain.iter().rev() {
            tree.mov(*n, loro::TreeParentId::Node(slide)).unwrap_or_else(|e| panic!("{e}"));
        }
        r.doc.set_next_commit_message("aplatie");
        r.doc.commit();
        r
    })
}

#[test]
fn deck_at_refuses_a_version_too_deep_and_reads_the_others_on_a_small_stack() {
    let r = forged_then_flattened();
    let current = r.deck().0;
    let h = r.history();
    // Loro peut découper un gros commit : la dernière version « forge » porte toute la chaîne.
    let forge = h.versions().iter().rposition(|v| v.label.as_deref() == Some("forge")).unwrap_or_else(|| panic!("forge"));
    let last = h.len() - 1;
    let (deep, latest, first) = on_stack(256 * 1024, move || {
        let deep = h.deck_at(forge).err().map(|e| e.kind());
        let latest = h.deck_at(last).map(|(d, _)| d).ok();
        let first = h.deck_at(0).map(|(d, _)| d).ok();
        (deep, latest, first)
    });
    assert_that!(deep).is_equal_to(Some(crate::error::ReplicaErrorKind::Corrupted));
    assert_that!(latest).is_equal_to(Some(current));
    assert_that!(first).is_equal_to(Some(roadmap()));
    on_stack(1 << 30, move || drop(r));
}
```

```rust
/// Témoin : un fichier dont l'historique a porté une chaîne profonde, mais dont
/// l'état est plat, s'ouvre par rejeu, s'enregistre et liste ses versions sur un fil de l'app.
#[test]
fn file_with_a_deep_past_and_a_flat_present_opens_saves_and_lists_history_on_an_app_thread() {
    let r = forged_then_flattened();
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    on_stack(1 << 30, move || drop(r));
    let deep = on_stack(2 << 20, move || {
        let (back, _) = crate::DeckReplica::from_snapshot(&bytes, 1 << 26, crate::test_registry::shared())
            .unwrap_or_else(|e| panic!("ouverture : {e}"));
        back.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        let h = back.history();
        let forge = h.versions().iter().rposition(|v| v.label.as_deref() == Some("forge")).unwrap_or_else(|| panic!("forge"));
        let deep = h.deck_at(forge).err().map(|e| e.kind());
        drop((h, back));
        deep
    });
    assert_that!(deep).is_equal_to(Some(crate::error::ReplicaErrorKind::Corrupted));
}
```

(`roadmap()` est celle de `history_tests.rs` ; si `deck_at(0)` n'est pas le deck de création,
comparer à la version de label `None`.)

- [ ] **Step 2 : vérifier l'échec** — `$CARGO nextest run -p oku-slides-core deck_at_refuses`
Expected : avortement (débordement au `checkout`) ou FAIL.

- [ ] **Step 3 : implémenter** (`history.rs`)

```rust
    /// Le deck juste après la version `index` (bornée à la dernière).
    ///
    /// La version est rejouée dans un document neuf à partir de ses
    /// opérations (`updates_till`), contrôlée comme à l'ouverture, lue puis
    /// libérée : jamais de `checkout` (t-809). Une panique empoisonne la copie.
    pub fn deck_at(&self, index: usize) -> Result<DeckRead, ReplicaError> {
        let (Some(doc), Some(cut)) = (self.doc.as_ref(), self.cut(index)) else {
            return Ok((Deck::empty(), Vec::new()));
        };
        if self.poisoned.get() {
            return Err(ImportPanickedSnafu.build().into());
        }
        let version = ManuallyDrop::new(LoroDoc::new());
        configure(&version);
        let read = catch_unwind(AssertUnwindSafe(|| -> Result<DeckRead, ReplicaInner> {
            let vv = doc.frontiers_to_vv(cut).context(NotADeckSnafu { what: "unknown version" })?;
            let updates = doc.export(ExportMode::updates_till(&vv)).context(ExportSnafu)?;
            version.import(&updates).context(ImportSnafu)?;
            check_trees(&version)?;
            read_deck(&version)
        }));
        match read {
            Ok(read) => {
                // Une version acceptée, ou refusée avant d'être lue, se libère sans récursion.
                drop(ManuallyDrop::into_inner(version));
                Ok(read?)
            }
            Err(_) => {
                self.poisoned.set(true);
                Err(ImportPanickedSnafu.build().into())
            }
        }
    }
```

(Imports : `std::mem::ManuallyDrop`, `loro::ExportMode`, `snafu::OptionExt`,
`super::{check_trees, configure, read_deck}`, `crate::error::{ExportSnafu, ImportSnafu,
NotADeckSnafu, ReplicaInner}`. `check_trees` doit être `pub(super)` dans `mod.rs`. Une version
refusée par `check_entry` contient un arbre profond **rejoué** : son drop ne récurse pas (sonde).)
Doc du module : « Une version se lit par rejeu de ses opérations, jamais par `checkout` (t-809). »

- [ ] **Step 4 : vérifier** — `$CARGO nextest run -p oku-slides-core history`
Expected : PASS (nouveau test, `poison_for_tests`, `send_test`).

- [ ] **Step 5 : ARCHITECTURE** (`crates/oku-slides-core/src/replica/ARCHITECTURE.md`, ≤ 50 lignes, condenser au besoin)

Modules `entry` (ouverture par rejeu, arbre unique, profondeur ≤ 1024, t-809) et `compact`
(compaction à l'ouverture, ADR-0007) ; « Suppression douce » : la marque vaut l'heure de sa pose,
une création annulable naît marquée (t-776), la compaction fait le seul `tree.delete` ; une version
passée se lit par rejeu ; `meta.format = 4`.

- [ ] **Step 6 : vérifier**

Run : `$CARGO nextest run --workspace` puis `$CARGO clippy --workspace --all-targets -- -D warnings`
Expected : PASS, aucun avertissement.

- [ ] **Step 7 : commit** — `fix(replica) : une version passée se lit par rejeu des opérations (t-809)`
