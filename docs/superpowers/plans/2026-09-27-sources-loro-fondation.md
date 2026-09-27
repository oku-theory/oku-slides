# Sources Loro, plan 1 : fondation — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** le blob des sources du PDF devient un document Loro qui porte tout l'historique du deck ; la session écrit dedans, annule avec l'`UndoManager` de Loro et reprend depuis un snapshot.

**Architecture :** un module `oku-slides-core::replica` enveloppe un `LoroDoc`. Les ops s'appliquent toujours sur un `Deck` ; le replica est ensuite **réconcilié** avec le nouveau `Deck` par diff structurel, puis committé. Le `Deck` n'est plus qu'une vue que `SourceBundle` garde à jour. Le zip des sources et l'enveloppe PDF passent en version 2.

**Tech Stack :** Rust 2024, `loro` 1.16, `snafu`, `serde_json`, `speculoos`, `proptest` (dev), Fluent (20 langues), Bun pour l'UI.

**Spec :** `docs/superpowers/specs/2026-09-27-sources-loro-design.md` (tâche t-188). Lire la spec avant chaque tâche.

**Plans suivants (non couverts ici) :** plan 2, fusion côté session (`deck_uid` dans `TabInfo`, auteurs `peers`, trois déclencheurs, `DiskReplaced`) ; plan 3, mode historique (versions, curseur, restauration, mesure du coût de rendu). Ils seront écrits sur le code livré par ce plan.

## Global Constraints

- Loro : `loro = "1.16"` dans `[workspace.dependencies]`, `loro.workspace = true` dans les crates.
- Pas de `.unwrap()` ni `.expect()` hors tests (clippy `unwrap_used`/`expect_used` = deny) ; `Mutex` : `lock().unwrap_or_else(PoisonError::into_inner)`.
- Erreurs Snafu opaques + `kind()` + `Localize`, `backtrace: Backtrace` dans chaque variante, messages `display` en minuscules.
- Tout message utilisateur passe par Fluent ; une clé ajoutée l'est dans **les 20 locales** (`crates/oku-slides-i18n/src/locales/*/`), sinon `every_locale_has_exactly_the_reference_keys` échoue. Traductions en annexe.
- Tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), nommés `<unité>_<scénario>_<résultat>`, assertions `speculoos`.
- Fichiers : cible 500 lignes, limite 1000.
- Commits Conventional, en français, avec `(t-<sous-tâche>)` ; **pas de ligne `Co-Authored-By`**. Le hook `common:review` peut bloquer un commit : corriger ce qu'il signale, ne jamais le contourner.
- Vérification complète : `just check` puis `just test`.
- Séparateur de paragraphes de la prose riche : `U+2029`.
- Taille maximale d'un snapshot lu : réglage `[sources] max_replica_mib = 32`.

## Sous-tâches Oku

Avant la tâche 1, mettre à jour le corps de t-188 (chemin de ce plan + titres des tâches) et créer une sous-tâche par tâche, titre identique :

```bash
oku cli task update t-188 --body "Plan 1 : docs/superpowers/plans/2026-09-27-sources-loro-fondation.md (tâches 1 à 10). Plans 2 (fusion) et 3 (mode historique) à venir."
for name in \
  "Vide = absent dans le modèle" \
  "DeckReplica : schéma, réconciliation, lecture" \
  "Prose riche en texte à marques" \
  "Lecture totale, blob non fiable, fusion" \
  "Annuler et rétablir par UndoManager" \
  "Convergence par propriété" \
  "Zip des sources et PDF en version 2" \
  "Session et reprise sur le replica" \
  "Supprimer History et les inverses d'ops" \
  "Documentation, ADR et vérification complète"; do
  oku cli task create --project slider --parent t-188 --name "$name"
done
```

Vérifier la syntaxe exacte avec `oku cli task update --help` si besoin. Noter les identifiants `t-N` rendus : chaque commit cite **sa** sous-tâche. Marquer chaque sous-tâche faite dès sa tâche terminée (`oku cli task done t-N`).

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/replica/mod.rs` | `DeckReplica` : création, snapshot, lecture, réconciliation, commit, fusion |
| `crates/oku-slides-core/src/replica/schema.rs` | catégorie de chaque champ selon son contexte (`Ctx`, `Kind`) |
| `crates/oku-slides-core/src/replica/write.rs` | réconciliation `serde_json::Value` → conteneurs Loro |
| `crates/oku-slides-core/src/replica/read.rs` | conteneurs Loro → JSON → `Deck`, lecture totale |
| `crates/oku-slides-core/src/replica/rich.rs` | `Vec<Paragraph>` ↔ `LoroText` à marques |
| `crates/oku-slides-core/src/replica/undo.rs` | `UndoManager` + libellés |
| `crates/oku-slides-core/src/replica/*_tests.rs` | tests unitaires côte à côte |
| `crates/oku-slides-core/tests/convergence.rs` | propriété : N pairs convergent |
| `crates/oku-slides-core/src/error.rs` | + `ReplicaError` |
| `crates/oku-slides-core/src/sources.rs` | zip v2, `SourceBundle` tient le replica |
| `crates/oku-slides-core/src/ops.rs` | vide = absent ; plus d'inverses (tâche 9) |
| `crates/oku-slides-core/src/history.rs` | supprimé (tâche 9) |
| `crates/oku-slides-pdf/src/lib.rs` | `/Version 2`, v1 ignorée |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `[sources] max_replica_mib` |
| `crates/oku-slides-session/src/lib.rs` | apply rendre-puis-valider, undo/redo, vue |
| `crates/oku-slides-session/src/resume.rs` | `deck.loro` au lieu de `deck` + `history` |
| `crates/oku-slides-session/src/files.rs` | lecture v2, avis des parties illisibles |
| `ui/src/deck.ts`, `ui/src/a2ui/panelActions.ts`, `ui/src/App.tsx` | normalisation vide → `null` supprimée |

---

### Task 1 : Vide = absent dans le modèle

**Files :**
- Modify : `crates/oku-slides-core/src/ops.rs:293-299` (`opt_string`)
- Test : `crates/oku-slides-core/tests/ops.rs`
- Modify : `ui/src/deck.ts:76`, `ui/src/a2ui/panelActions.ts:25`, `ui/src/App.tsx:494`
- Test : `ui/src/deck.test.ts`

**Interfaces :**
- Produces : invariant « toute prose `Option<String>` posée par une op est `None` si vide ou faite d'espaces ». Le replica (tâche 2) s'appuie dessus.

- [ ] **Step 1 : tests rouges (Rust)** — ajouter à `crates/oku-slides-core/tests/ops.rs` :

```rust
#[rstest::rstest]
#[case("")]
#[case("   ")]
fn set_slide_field_blank_title_stores_none(#[case] blank: &str) {
    let mut deck = fixture();
    let id = deck.slides[2].id.clone();
    apply_all(
        &mut deck,
        vec![Op::SetSlideField { slide: id.clone(), field: SlideField::Title, value: serde_json::json!(blank) }],
    )
    .unwrap();
    assert_that!(deck.slide(&id).unwrap().title).is_none();
}

#[test]
fn set_param_blank_author_stores_none() {
    let mut deck = fixture();
    apply_all(&mut deck, vec![Op::SetParam { key: "author".into(), value: serde_json::json!("") }]).unwrap();
    assert_that!(deck.params.author).is_none();
}
```

Si `rstest` n'est pas encore une dépendance de dev de `oku-slides-core`, l'ajouter (`rstest = "0.26"` dans `[workspace.dependencies]`, `rstest.workspace = true` en `[dev-dependencies]`). Adapter les noms de champs d'`Op::SetSlideField` / `Op::SetParam` à ceux de `ops.rs:115` et `ops.rs:150`.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(blank)'` → FAIL (`Some("")`).

- [ ] **Step 3 : implémentation**

```rust
fn opt_string(field: &str, v: Value) -> Result<Option<String>, OpError> {
    match v {
        Value::Null => Ok(None),
        // Vide = absent : une prose vide ne s'affiche pas, et le replica
        // relit un texte vide en `None`.
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => Ok(Some(s)),
        _ => Err(InvalidValueSnafu { field, reason: "expected a string or null" }.build().into()),
    }
}
```

- [ ] **Step 4 : vérifier** — `cargo nextest run -p oku-slides-core` → PASS.

- [ ] **Step 5 : test UI rouge** — dans `ui/src/deck.test.ts`, un test qui vérifie que l'op produite pour un titre `"  "` porte la valeur telle quelle (`"  "`) : la normalisation est désormais côté cœur. Lancer `cd ui && bun test deck` → FAIL.

- [ ] **Step 6 : supprimer la normalisation UI** — `deck.ts:76` : `const v = value;` ; `panelActions.ts:25` : `const text = () => String(ctx.value ?? "");` ; `App.tsx:494` : `value: v`. Garder le reste intact.

- [ ] **Step 7 : vérifier** — `cd ui && bun test && bun run typecheck` → PASS.

- [ ] **Step 8 : commit**

```bash
git add crates/oku-slides-core ui/src Cargo.toml
git commit -m "feat(core) : une prose vide vaut absente, règle portée par le cœur (t-<N1>)"
```

---

### Task 2 : DeckReplica — schéma, réconciliation, lecture

Dans cette tâche, le `text` des éléments (prose riche) est provisoirement un **atome JSON** ; la tâche 3 le remplace. La lecture n'est pas encore totale (tâche 4).

**Files :**
- Modify : `Cargo.toml` (`loro = "1.16"`), `crates/oku-slides-core/Cargo.toml`
- Create : `crates/oku-slides-core/src/replica/{mod.rs,schema.rs,write.rs,read.rs}`, `replica/mod_tests.rs`
- Modify : `crates/oku-slides-core/src/lib.rs`, `crates/oku-slides-core/src/error.rs`

**Interfaces :**
- Produces :
  - `pub struct DeckReplica` (`Debug`, `Send`).
  - `DeckReplica::new(deck: &Deck) -> Result<DeckReplica, ReplicaError>`
  - `DeckReplica::from_snapshot(bytes: &[u8], max_bytes: usize) -> Result<DeckReplica, ReplicaError>`
  - `DeckReplica::snapshot(&self) -> Result<Vec<u8>, ReplicaError>`
  - `DeckReplica::deck(&self) -> Result<Deck, ReplicaError>` (devient `-> (Deck, Vec<Issue>)` en tâche 4)
  - `DeckReplica::reconcile(&self, deck: &Deck) -> Result<(), ReplicaError>`
  - `DeckReplica::commit(&mut self, label: &str)` (libellé exploité en tâche 5)
  - `DeckReplica::deck_uid(&self) -> String`
  - `DeckReplica::len_ops(&self) -> usize` (tests)
  - `pub struct ReplicaError` + `ReplicaErrorKind { Corrupted, TooLarge, Write, Unrelated }` (`Unrelated` servi en tâche 4).
  - Racine Loro : map `deck` ; map `meta` avec `deck_uid`.

- [ ] **Step 1 : dépendance** — `Cargo.toml` : `loro = "1.16"` sous `[workspace.dependencies]` ; `crates/oku-slides-core/Cargo.toml` : `loro.workspace = true`. `cargo build -p oku-slides-core` → OK.

- [ ] **Step 2 : erreur** — dans `error.rs`, sur le modèle de `SourcesError` :

```rust
/// Le replica Loro du deck ne se lit pas, ou ne s'écrit pas.
#[derive(Debug, Snafu)]
pub struct ReplicaError(ReplicaInner);

/// Nature d'une [`ReplicaError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplicaErrorKind {
    /// Snapshot illisible, ou qui n'est pas un deck Oku Slides.
    Corrupted,
    /// Snapshot au-delà de la taille permise.
    TooLarge,
    /// Écriture refusée par Loro.
    Write,
    /// Deux replicas sans origine commune.
    Unrelated,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ReplicaInner {
    #[snafu(display("unreadable replica snapshot"))]
    Import { source: loro::LoroError, backtrace: Backtrace },
    #[snafu(display("the replica snapshot is not an Oku Slides deck: {what}"))]
    NotADeck { what: &'static str, backtrace: Backtrace },
    #[snafu(display("replica import panicked"))]
    ImportPanicked { backtrace: Backtrace },
    #[snafu(display("replica snapshot too large: {size} bytes (max {max})"))]
    TooLarge { size: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("replica write failed"))]
    Loro { source: loro::LoroError, backtrace: Backtrace },
    #[snafu(display("replica text update timed out"))]
    TextUpdate { source: loro::UpdateTimeoutError, backtrace: Backtrace },
    #[snafu(display("replica export failed"))]
    Export { source: loro::LoroEncodeError, backtrace: Backtrace },
    #[snafu(display("deck (de)serialization failed"))]
    DeckJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("the two replicas have no common origin"))]
    Unrelated { backtrace: Backtrace },
}

impl ReplicaError {
    pub fn kind(&self) -> ReplicaErrorKind {
        match self.0 {
            ReplicaInner::Import { .. } | ReplicaInner::NotADeck { .. } | ReplicaInner::ImportPanicked { .. } => {
                ReplicaErrorKind::Corrupted
            }
            ReplicaInner::TooLarge { .. } => ReplicaErrorKind::TooLarge,
            ReplicaInner::Unrelated { .. } => ReplicaErrorKind::Unrelated,
            ReplicaInner::Loro { .. }
            | ReplicaInner::TextUpdate { .. }
            | ReplicaInner::Export { .. }
            | ReplicaInner::DeckJson { .. } => ReplicaErrorKind::Write,
        }
    }
}

impl Localize for ReplicaError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(match self.kind() {
            ReplicaErrorKind::Corrupted => "replica-corrupted",
            ReplicaErrorKind::TooLarge => "replica-too-large",
            ReplicaErrorKind::Write => "replica-write",
            ReplicaErrorKind::Unrelated => "replica-unrelated",
        })
    }
}
```

Ajouter les 4 clés `replica-*` dans `core.ftl` des 20 locales (annexe A). Exporter `ReplicaError`, `ReplicaErrorKind` depuis `lib.rs`.

- [ ] **Step 3 : schéma** — `replica/schema.rs` :

```rust
//! Catégorie de chaque champ du deck dans le replica, selon son contexte.

/// Où l'on se trouve dans le deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ctx {
    Deck,
    Params,
    Slide,
    Element,
    /// `vars`, `tokens`, `style`, `frame`, `cell`, `grid`, `layout` de groupe,
    /// extras de `params` : cartes à feuilles LWW.
    Opaque,
}

/// Ce que devient un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `LoroText`, fusion au caractère.
    Prose,
    /// `LoroText` à marques (paragraphes).
    Rich,
    /// `LoroMovableList` de `LoroMap` à `id`.
    Collection(Ctx),
    /// Une valeur JSON entière, LWW.
    Atom,
    /// Objet → `LoroMap` ; autre valeur → LWW.
    Nested(Ctx),
}

impl Ctx {
    pub(super) fn kind(self, key: &str) -> Kind {
        use Ctx::*;
        match (self, key) {
            (Deck, "slides") => Kind::Collection(Slide),
            (Deck, "params") => Kind::Nested(Params),
            (Params, "title" | "subtitle" | "author" | "date" | "organization") => Kind::Prose,
            (Slide, "name" | "description" | "title" | "subtitle" | "kicker" | "text" | "notes") => Kind::Prose,
            (Slide, "blocks" | "elements") => Kind::Collection(Element),
            (Element, "title" | "kicker") => Kind::Prose,
            (Element, "text") => Kind::Rich,
            (Element, "children") => Kind::Collection(Element),
            // Invariant global (`consistent_chart`) : un graphique est un atome.
            (Element, "option") => Kind::Atom,
            _ => Kind::Nested(Opaque),
        }
    }

    /// Textes créés dès la création de l'objet (spec : « toujours présents »).
    pub(super) fn texts(self) -> &'static [&'static str] {
        match self {
            Ctx::Params => &["title", "subtitle", "author", "date", "organization"],
            Ctx::Slide => &["name", "description", "title", "subtitle", "kicker", "text", "notes"],
            Ctx::Element => &["title", "kicker", "text"],
            Ctx::Deck | Ctx::Opaque => &[],
        }
    }

    /// Collections créées dès la création de l'objet.
    pub(super) fn lists(self) -> &'static [&'static str] {
        match self {
            Ctx::Deck => &["slides"],
            Ctx::Slide => &["blocks", "elements"],
            Ctx::Element => &["children"],
            Ctx::Params | Ctx::Opaque => &[],
        }
    }
}
```

- [ ] **Step 4 : tests rouges** — `replica/mod_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::{Deck, Op, SlideField, apply_all};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn new_then_deck_returns_same_deck() {
    let deck = roadmap();
    let r = DeckReplica::new(&deck).unwrap();
    assert_that!(r.deck().unwrap()).is_equal_to(&deck);
}

#[test]
fn reconcile_same_deck_adds_no_op() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let before = r.len_ops();
    r.reconcile(&deck).unwrap();
    r.commit("rien");
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn snapshot_then_from_snapshot_returns_same_deck_and_uid() {
    let r = DeckReplica::new(&roadmap()).unwrap();
    let back = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20).unwrap();
    assert_that!(back.deck().unwrap()).is_equal_to(r.deck().unwrap());
    assert_that!(back.deck_uid()).is_equal_to(r.deck_uid());
}

#[test]
fn reconcile_after_op_changes_deck() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let id = deck.slides[2].id.clone();
    apply_all(
        &mut deck,
        vec![Op::SetSlideField { slide: id, field: SlideField::Title, value: serde_json::json!("Nouveau") }],
    )
    .unwrap();
    r.reconcile(&deck).unwrap();
    r.commit("Titre");
    assert_that!(r.deck().unwrap()).is_equal_to(&deck);
}

#[test]
fn concurrent_title_edits_merge_both() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap(), DeckReplica::from_snapshot(&snap, 1 << 20).unwrap());
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[2].title = Some("Contexte (révisé)".into());
    db.slides[2].title = Some("Brouillon : Contexte".into());
    a.reconcile(&da).unwrap();
    a.commit("A");
    b.reconcile(&db).unwrap();
    b.commit("B");
    a.import_for_tests(&b);
    assert_that!(a.deck().unwrap().slides[2].title.as_deref()).is_equal_to(Some("Brouillon : Contexte (révisé)"));
}

#[test]
fn concurrent_first_element_on_empty_slide_keeps_both() {
    let mut deck = roadmap();
    let empty = deck.slides.iter().position(|s| s.elements.is_empty() && s.blocks.is_empty()).unwrap();
    let donor = deck.slides.iter().flat_map(|s| s.elements.iter()).next().unwrap().clone();
    deck.slides[empty].elements.clear();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap(), DeckReplica::from_snapshot(&snap, 1 << 20).unwrap());
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[empty].elements.push(crate::Element { id: "e-a".into(), ..donor.clone() });
    db.slides[empty].elements.push(crate::Element { id: "e-b".into(), ..donor });
    a.reconcile(&da).unwrap();
    a.commit("A");
    b.reconcile(&db).unwrap();
    b.commit("B");
    a.import_for_tests(&b);
    assert_that!(a.deck().unwrap().slides[empty].elements).has_length(2);
}

#[test]
fn concurrent_first_title_on_untitled_slide_keeps_both() {
    let deck = roadmap();
    let untitled = deck.slides.iter().position(|s| s.title.is_none()).unwrap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap(), DeckReplica::from_snapshot(&snap, 1 << 20).unwrap());
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[untitled].title = Some("A".into());
    db.slides[untitled].title = Some("B".into());
    a.reconcile(&da).unwrap();
    a.commit("A");
    b.reconcile(&db).unwrap();
    b.commit("B");
    a.import_for_tests(&b);
    let title = a.deck().unwrap().slides[untitled].title.clone().unwrap();
    assert_that!(title.contains('A') && title.contains('B')).is_true();
}

#[test]
fn blank_title_reads_as_none() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    deck.slides[2].title = None;
    r.reconcile(&deck).unwrap();
    r.commit("Sans titre");
    assert_that!(r.deck().unwrap().slides[2].title).is_none();
}
```

`import_for_tests` : `#[cfg(test)] pub(crate) fn import_for_tests(&mut self, other: &DeckReplica)` qui importe `other.snapshot()` (remplacé par `merge` en tâche 4). Si `roadmap.json` n'a pas de slide vide, en créer une dans le test (`Slide::new("s-vide", SlideKind::Content)`) au lieu de `position(...)`.

- [ ] **Step 5 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/replica/)'` → erreurs de compilation (`DeckReplica` n'existe pas).

- [ ] **Step 6 : écriture** — `replica/write.rs` :

```rust
//! Réconciliation : amène les conteneurs Loro à l'état d'un `Deck` JSON, par diff.

use loro::{Container, LoroMap, LoroMovableList, LoroText, LoroValue, UpdateOptions, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::schema::{Ctx, Kind};
use crate::error::{LoroSnafu, ReplicaInner, TextUpdateSnafu};

type Res = Result<(), ReplicaInner>;

pub(super) fn json_to_loro(v: &Value) -> LoroValue {
    match v {
        Value::Null => LoroValue::Null,
        Value::Bool(b) => LoroValue::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => LoroValue::I64(i),
            None => LoroValue::Double(n.as_f64().unwrap_or_default()),
        },
        Value::String(s) => LoroValue::from(s.as_str()),
        Value::Array(a) => LoroValue::from(a.iter().map(json_to_loro).collect::<Vec<_>>()),
        Value::Object(o) => LoroValue::from(
            o.iter().map(|(k, v)| (k.clone(), json_to_loro(v))).collect::<std::collections::HashMap<_, _>>(),
        ),
    }
}

pub(super) fn text_at(map: &LoroMap, key: &str) -> Result<LoroText, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Text(t))) => Ok(t),
        _ => map.insert_container(key, LoroText::new()).context(LoroSnafu),
    }
}

fn list_at(map: &LoroMap, key: &str) -> Result<LoroMovableList, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Ok(l),
        _ => map.insert_container(key, LoroMovableList::new()).context(LoroSnafu),
    }
}

fn map_at(map: &LoroMap, key: &str) -> Result<LoroMap, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) => Ok(m),
        _ => map.insert_container(key, LoroMap::new()).context(LoroSnafu),
    }
}

fn set_value(map: &LoroMap, key: &str, v: LoroValue) -> Res {
    if matches!(map.get(key), Some(ValueOrContainer::Value(old)) if old == v) {
        return Ok(());
    }
    map.insert(key, v).context(LoroSnafu)
}

pub(super) fn sync_text(t: &LoroText, s: &str) -> Res {
    if t.to_string() != s {
        t.update(s, UpdateOptions::default()).context(TextUpdateSnafu)?;
    }
    Ok(())
}

/// Amène `map` à l'état de `obj`.
pub(super) fn sync_map(map: &LoroMap, obj: &Map<String, Value>, ctx: Ctx) -> Res {
    // Structure toujours présente : jamais de création concurrente d'une même clé.
    for key in ctx.texts() {
        text_at(map, key)?;
    }
    for key in ctx.lists() {
        list_at(map, key)?;
    }
    let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
    for key in stale {
        match ctx.kind(&key) {
            // Absent = vide : on vide, la clé reste.
            Kind::Prose | Kind::Rich => sync_text(&text_at(map, &key)?, "")?,
            Kind::Collection(_) => {
                let l = list_at(map, &key)?;
                if !l.is_empty() {
                    l.delete(0, l.len()).context(LoroSnafu)?;
                }
            }
            Kind::Atom | Kind::Nested(_) => map.delete(&key).context(LoroSnafu)?,
        }
    }
    for (key, value) in obj {
        match ctx.kind(key) {
            Kind::Prose => sync_text(&text_at(map, key)?, value.as_str().unwrap_or_default())?,
            // Provisoire (tâche 3 : texte à marques) : les paragraphes en atome.
            Kind::Rich => set_value(map, key, json_to_loro(value))?,
            Kind::Collection(child) => {
                sync_list(&list_at(map, key)?, value.as_array().map(Vec::as_slice).unwrap_or_default(), child)?
            }
            Kind::Atom => set_value(map, key, json_to_loro(value))?,
            Kind::Nested(child) => match value {
                Value::Object(o) => sync_map(&map_at(map, key)?, o, child)?,
                v => set_value(map, key, json_to_loro(v))?,
            },
        }
    }
    Ok(())
}

fn item_id(list: &LoroMovableList, i: usize) -> String {
    match list.get(i) {
        Some(ValueOrContainer::Container(Container::Map(m))) => match m.get("id") {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// Réconcilie une collection à `id` : suppressions, puis insertions et déplacements.
fn sync_list(list: &LoroMovableList, items: &[Value], ctx: Ctx) -> Res {
    let id_of = |v: &Value| v.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    let wanted: Vec<String> = items.iter().map(id_of).collect();
    let mut ids: Vec<String> = (0..list.len()).map(|i| item_id(list, i)).collect();
    for i in (0..ids.len()).rev() {
        if !wanted.contains(&ids[i]) {
            list.delete(i, 1).context(LoroSnafu)?;
            ids.remove(i);
        }
    }
    for (target, item) in items.iter().enumerate() {
        let id = &wanted[target];
        match ids.iter().position(|x| x == id) {
            Some(pos) if pos == target => {}
            Some(pos) => {
                list.mov(pos, target).context(LoroSnafu)?;
                let moved = ids.remove(pos);
                ids.insert(target, moved);
            }
            None => {
                list.insert_container(target, LoroMap::new()).context(LoroSnafu)?;
                ids.insert(target, id.clone());
            }
        }
        if let (Some(ValueOrContainer::Container(Container::Map(m))), Some(obj)) = (list.get(target), item.as_object())
        {
            sync_map(&m, obj, ctx)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 7 : lecture** — `replica/read.rs` :

```rust
//! Lecture : conteneurs Loro → JSON du deck.

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use serde_json::{Map, Value};

use super::schema::{Ctx, Kind};

pub(super) fn loro_to_json(v: &LoroValue) -> Value {
    match v {
        LoroValue::Null | LoroValue::Binary(_) | LoroValue::Container(_) => Value::Null,
        LoroValue::Bool(b) => Value::Bool(*b),
        LoroValue::Double(d) => serde_json::json!(d),
        LoroValue::I64(i) => serde_json::json!(i),
        LoroValue::String(s) => Value::String(s.to_string()),
        LoroValue::List(l) => Value::Array(l.iter().map(loro_to_json).collect()),
        LoroValue::Map(m) => Value::Object(m.iter().map(|(k, v)| (k.to_string(), loro_to_json(v))).collect()),
    }
}

pub(super) fn read_map(map: &LoroMap, ctx: Ctx) -> Map<String, Value> {
    let mut out = Map::new();
    for key in map.keys() {
        let key = key.to_string();
        let Some(entry) = map.get(&key) else { continue };
        let value = match (ctx.kind(&key), entry) {
            (_, ValueOrContainer::Container(Container::Text(t))) => {
                let s = t.to_string();
                // Vide = absent.
                if s.trim().is_empty() {
                    continue;
                }
                Value::String(s)
            }
            (Kind::Collection(child), ValueOrContainer::Container(Container::MovableList(l))) => {
                let items = read_list(&l, child);
                // `slides` est obligatoire ; les autres collections vides sont omises.
                if items.is_empty() && ctx != Ctx::Deck {
                    continue;
                }
                Value::Array(items)
            }
            (_, ValueOrContainer::Container(Container::Map(m))) => {
                let child = match ctx.kind(&key) {
                    Kind::Nested(c) => c,
                    _ => Ctx::Opaque,
                };
                Value::Object(read_map(&m, child))
            }
            (_, ValueOrContainer::Value(v)) => loro_to_json(&v),
            // Conteneur d'un type inattendu : ignoré.
            _ => continue,
        };
        out.insert(key, value);
    }
    out
}

fn read_list(list: &LoroMovableList, ctx: Ctx) -> Vec<Value> {
    (0..list.len())
        .filter_map(|i| match list.get(i) {
            Some(ValueOrContainer::Container(Container::Map(m))) => Some(Value::Object(read_map(&m, ctx))),
            _ => None,
        })
        .collect()
}
```

- [ ] **Step 8 : `DeckReplica`** — `replica/mod.rs` :

```rust
//! Le deck dans un document Loro (CRDT) : historique complet et fusion.
//!
//! - [`DeckReplica`] : création, snapshot, lecture ([`DeckReplica::deck`]),
//!   réconciliation avec un `Deck` modifié, commit.
//! - Le `Deck` n'est qu'une vue ; le replica fait foi.

mod read;
mod schema;
mod write;

use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{ExportMode, LoroDoc, LoroValue, ValueOrContainer};
use snafu::{ResultExt, ensure};

use crate::error::{
    DeckJsonSnafu, ExportSnafu, ImportPanickedSnafu, ImportSnafu, LoroSnafu, NotADeckSnafu, ReplicaError,
    ReplicaInner, TooLargeSnafu,
};
use crate::model::Deck;
use schema::Ctx;

const ROOT: &str = "deck";
const META: &str = "meta";
const DECK_UID: &str = "deck_uid";

/// Le deck, son historique complet, et son identité (`deck_uid`).
pub struct DeckReplica {
    doc: LoroDoc,
}

impl std::fmt::Debug for DeckReplica {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeckReplica").field("deck_uid", &self.deck_uid()).field("changes", &self.doc.len_changes()).finish()
    }
}

/// Réglages communs à tout `LoroDoc` du deck (non persistés par Loro).
fn configure(doc: &LoroDoc) {
    doc.set_record_timestamp(true);
}

impl DeckReplica {
    /// Nouveau replica, avec un nouveau `deck_uid`.
    pub fn new(deck: &Deck) -> Result<Self, ReplicaError> {
        Ok(Self::create(deck)?)
    }

    fn create(deck: &Deck) -> Result<Self, ReplicaInner> {
        let doc = LoroDoc::new();
        configure(&doc);
        // Le pair créateur est tiré au hasard sur 64 bits : un identifiant suffisant.
        let uid = format!("{:016x}", doc.peer_id());
        doc.get_map(META).insert(DECK_UID, uid).context(LoroSnafu)?;
        let mut r = Self { doc };
        r.reconcile_inner(deck)?;
        r.commit("");
        Ok(r)
    }

    /// Relit un snapshot. Les octets viennent d'un fichier : non fiables.
    pub fn from_snapshot(bytes: &[u8], max_bytes: usize) -> Result<Self, ReplicaError> {
        Ok(Self::import_snapshot(bytes, max_bytes)?)
    }

    fn import_snapshot(bytes: &[u8], max: usize) -> Result<Self, ReplicaInner> {
        ensure!(bytes.len() <= max, TooLargeSnafu { size: bytes.len(), max });
        let imported = catch_unwind(AssertUnwindSafe(|| {
            let doc = LoroDoc::new();
            configure(&doc);
            doc.import(bytes).map(|_| doc)
        }));
        let doc = match imported {
            Ok(r) => r.context(ImportSnafu)?,
            Err(_) => return ImportPanickedSnafu.fail(),
        };
        let r = Self { doc };
        ensure!(!r.deck_uid().is_empty(), NotADeckSnafu { what: "no deck_uid" });
        Ok(r)
    }

    pub fn snapshot(&self) -> Result<Vec<u8>, ReplicaError> {
        Ok(self.doc.export(ExportMode::Snapshot).context(ExportSnafu)?)
    }

    pub fn deck_uid(&self) -> String {
        match self.doc.get_map(META).get(DECK_UID) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        }
    }

    /// Le deck relu depuis le replica.
    pub fn deck(&self) -> Result<Deck, ReplicaError> {
        let json = read::read_map(&self.doc.get_map(ROOT), Ctx::Deck);
        Ok(serde_json::from_value(serde_json::Value::Object(json)).context(DeckJsonSnafu)?)
    }

    /// Amène le replica à l'état de `deck` (sans committer).
    pub fn reconcile(&self, deck: &Deck) -> Result<(), ReplicaError> {
        Ok(self.reconcile_inner(deck)?)
    }

    fn reconcile_inner(&self, deck: &Deck) -> Result<(), ReplicaInner> {
        let json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
        let obj = json.as_object().cloned().unwrap_or_default();
        write::sync_map(&self.doc.get_map(ROOT), &obj, Ctx::Deck)
    }

    /// Valide les changements en cours ; un commit sans changement n'ajoute rien.
    pub fn commit(&mut self, label: &str) {
        let _ = label; // tâche 5 : libellé d'annulation
        self.doc.commit();
    }

    /// Nombre d'ops de l'historique (tests, mesures).
    pub fn len_ops(&self) -> usize {
        self.doc.len_ops()
    }

    #[cfg(test)]
    pub(crate) fn import_for_tests(&mut self, other: &DeckReplica) {
        if let Ok(bytes) = other.snapshot() {
            let _ = self.doc.import(&bytes);
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
```

`lib.rs` : `pub mod replica;` et `pub use replica::DeckReplica;` ; ajouter `DeckReplica` et `ReplicaError` à la liste du doc de tête (G4).

- [ ] **Step 9 : vérifier** — `cargo nextest run -p oku-slides-core -E 'test(/replica/)'` → PASS. Si `concurrent_first_*` échoue, relire `sync_map` : la structure doit être créée **avant** toute écriture, à la création de l'objet.

- [ ] **Step 10 : clippy** — `cargo clippy -p oku-slides-core --all-targets -- -D warnings` → propre.

- [ ] **Step 11 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-core crates/oku-slides-i18n
git commit -m "feat(core) : DeckReplica, le deck dans un document Loro réconcilié par diff (t-<N2>)"
```

---

### Task 3 : Prose riche en texte à marques

**Files :**
- Create : `crates/oku-slides-core/src/replica/rich.rs`, `replica/rich_tests.rs`
- Modify : `replica/write.rs` (branche `Kind::Rich`), `replica/read.rs` (lecture `Kind::Rich`), `replica/mod.rs` (`configure`)

**Interfaces :**
- Produces : `rich::sync(text: &LoroText, value: &Value) -> Result<(), ReplicaInner>`, `rich::read(text: &LoroText) -> Vec<Value>`, `rich::styles() -> StyleConfigMap`, constantes `SEP: char = '\u{2029}'`, `RUN_MARKS`, `PARA_MARKS`.

- [ ] **Step 1 : tests rouges** — `replica/rich_tests.rs` :

```rust
use loro::LoroDoc;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;

fn doc() -> LoroDoc {
    let d = LoroDoc::new();
    d.config_text_style(styles());
    d
}

fn paras(v: Value) -> Value {
    v
}

#[test]
fn every_run_and_paragraph_field_has_a_mark() {
    let run = serde_json::to_value(crate::Run {
        t: "x".into(),
        bold: true,
        italic: true,
        color: Some("#000".into()),
        size: Some(crate::TokenValue::Number(1.0)),
        link: Some("https://a".into()),
        highlight: Some(1),
    })
    .unwrap();
    let keys: Vec<&str> = run.as_object().unwrap().keys().map(String::as_str).filter(|k| *k != "t").collect();
    assert_that!(keys.iter().all(|k| RUN_MARKS.contains(k))).is_true();
    let p = serde_json::to_value(crate::Paragraph { runs: vec![], list: Some(crate::ListKind::Bullet) }).unwrap();
    let keys: Vec<&str> = p.as_object().unwrap().keys().map(String::as_str).filter(|k| *k != "runs").collect();
    assert_that!(keys.iter().all(|k| PARA_MARKS.contains(k))).is_true();
}

#[test]
fn sync_then_read_returns_same_paragraphs() {
    let d = doc();
    let t = d.get_text("t");
    let v = paras(json!([
        { "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé\nen deux ans." }] },
        { "runs": [{ "t": "lien", "link": "https://oku.test" }], "list": "bullet" }
    ]));
    sync(&t, &v).unwrap();
    d.commit();
    assert_that!(Value::Array(read(&t))).is_equal_to(&v);
}

#[test]
fn read_merges_adjacent_runs_with_same_format() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([{ "runs": [{ "t": "a" }, { "t": "b" }] }])).unwrap();
    assert_that!(Value::Array(read(&t))).is_equal_to(json!([{ "runs": [{ "t": "ab" }] }]));
}

#[test]
fn empty_paragraph_list_reads_as_empty() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([])).unwrap();
    assert_that!(read(&t)).is_empty();
}

#[test]
fn concurrent_bold_and_typing_keep_both() {
    let base = doc();
    base.set_peer_id(0).unwrap();
    sync(&base.get_text("t"), &json!([{ "runs": [{ "t": "Le marché a doublé." }] }])).unwrap();
    base.commit();
    let (a, b) = (base.fork(), base.fork());
    a.config_text_style(styles());
    b.config_text_style(styles());
    sync(
        &a.get_text("t"),
        &json!([{ "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé." }] }]),
    )
    .unwrap();
    a.commit();
    sync(&b.get_text("t"), &json!([{ "runs": [{ "t": "Le marché a doublé. [B]" }] }])).unwrap();
    b.commit();
    a.import(&b.export(loro::ExportMode::all_updates()).unwrap()).unwrap();
    assert_that!(Value::Array(read(&a.get_text("t")))).is_equal_to(json!([
        { "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé. [B]" }] }
    ]));
}

#[test]
fn typing_after_link_does_not_extend_link() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([{ "runs": [{ "t": "lien", "link": "https://oku.test" }] }])).unwrap();
    d.commit();
    t.insert(4, " suite").unwrap();
    assert_that!(Value::Array(read(&t))).is_equal_to(json!([
        { "runs": [{ "t": "lien", "link": "https://oku.test" }, { "t": " suite" }] }
    ]));
}
```

Vérifier le nom réel du variant numérique de `TokenValue` dans `model.rs:207` et l'ajuster.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/rich/)'` → échec de compilation.

- [ ] **Step 3 : implémentation** — `replica/rich.rs` :

```rust
//! Prose riche : `Vec<Paragraph>` ↔ un `LoroText` à marques (Peritext).
//!
//! Chaque paragraphe se termine par [`SEP`] (U+2029), le dernier compris. Les
//! champs d'un `Run` (hors `t`) sont des marques sur ses caractères ; ceux d'un
//! `Paragraph` (hors `runs`) sont des marques sur son séparateur.

use std::collections::BTreeMap;

use loro::{ExpandType, LoroText, LoroValue, StyleConfig, StyleConfigMap, TextDelta};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::read::loro_to_json;
use super::write::{json_to_loro, sync_text};
use crate::error::{LoroSnafu, ReplicaInner};

pub(super) const SEP: char = '\u{2029}';
/// Champs d'un `Run`, hors `t`.
pub(super) const RUN_MARKS: &[&str] = &["bold", "italic", "color", "size", "link", "highlight"];
/// Champs d'un `Paragraph`, hors `runs`.
pub(super) const PARA_MARKS: &[&str] = &["list"];

type Attrs = BTreeMap<String, LoroValue>;

/// Extension des marques : la frappe au bout d'un gras reste grasse, pas au bout d'un lien.
pub(super) fn styles() -> StyleConfigMap {
    let mut m = StyleConfigMap::new();
    for key in ["bold", "italic", "color", "size", "highlight"] {
        m.insert(key.into(), StyleConfig { expand: ExpandType::After });
    }
    for key in ["link", "list"] {
        m.insert(key.into(), StyleConfig { expand: ExpandType::None });
    }
    m
}

fn attrs_of(obj: &Map<String, Value>, keys: &[&str]) -> Attrs {
    keys.iter().filter_map(|k| obj.get(*k).map(|v| ((*k).to_owned(), json_to_loro(v)))).collect()
}

/// Texte cible et marques voulues, caractère par caractère.
fn flatten(value: &Value) -> (String, Vec<Attrs>) {
    let (mut text, mut marks) = (String::new(), Vec::new());
    for p in value.as_array().map(Vec::as_slice).unwrap_or_default() {
        let Some(p) = p.as_object() else { continue };
        for run in p.get("runs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let Some(run) = run.as_object() else { continue };
            let t = run.get("t").and_then(Value::as_str).unwrap_or_default();
            let a = attrs_of(run, RUN_MARKS);
            for c in t.chars() {
                text.push(c);
                marks.push(a.clone());
            }
        }
        text.push(SEP);
        marks.push(attrs_of(p, PARA_MARKS));
    }
    (text, marks)
}

/// Marques actuelles, caractère par caractère.
fn current(text: &LoroText) -> Vec<Attrs> {
    let mut out = Vec::new();
    for d in text.to_delta() {
        if let TextDelta::Insert { insert, attributes } = d {
            let a: Attrs = attributes
                .map(|m| m.into_iter().filter(|(v_k, v)| !matches!(v, LoroValue::Null) && !v_k.is_empty()).collect())
                .unwrap_or_default();
            out.extend(std::iter::repeat_n(a, insert.chars().count()));
        }
    }
    out
}

/// Amène `text` à l'état des paragraphes `value` : texte par diff, puis marques.
pub(super) fn sync(text: &LoroText, value: &Value) -> Result<(), ReplicaInner> {
    let (target, want) = flatten(value);
    sync_text(text, &target)?;
    let have = current(text);
    for key in RUN_MARKS.iter().chain(PARA_MARKS) {
        let (w, h): (Vec<_>, Vec<_>) = (want.iter().map(|a| a.get(*key)).collect(), have.iter().map(|a| a.get(*key)).collect());
        let mut i = 0;
        while i < w.len() {
            let mut j = i + 1;
            while j < w.len() && w[j] == w[i] {
                j += 1;
            }
            if (i..j).any(|k| h.get(k).copied().flatten() != w[k]) {
                match w[i] {
                    Some(v) => text.mark(i..j, key, v.clone()).context(LoroSnafu)?,
                    None => text.unmark(i..j, key).context(LoroSnafu)?,
                }
            }
            i = j;
        }
    }
    Ok(())
}

fn push_run(runs: &mut Vec<Value>, t: String, a: &Attrs) {
    if t.is_empty() {
        return;
    }
    let mut obj: Map<String, Value> =
        a.iter().filter(|(k, _)| RUN_MARKS.contains(&k.as_str())).map(|(k, v)| (k.clone(), loro_to_json(v))).collect();
    // Même mise en forme que le run précédent : on le prolonge (forme canonique).
    if let Some(Value::Object(last)) = runs.last_mut() {
        let same = last.iter().filter(|(k, _)| k.as_str() != "t").eq(obj.iter());
        if same {
            if let Some(Value::String(prev)) = last.get_mut("t") {
                prev.push_str(&t);
                return;
            }
        }
    }
    obj.insert("t".into(), Value::String(t));
    runs.push(Value::Object(obj));
}

fn push_paragraph(out: &mut Vec<Value>, runs: Vec<Value>, sep: &Attrs) {
    let mut p = Map::new();
    p.insert("runs".into(), Value::Array(runs));
    for (k, v) in sep.iter().filter(|(k, _)| PARA_MARKS.contains(&k.as_str())) {
        p.insert(k.clone(), loro_to_json(v));
    }
    out.push(Value::Object(p));
}

/// Relit les paragraphes. Un texte sans séparateur final forme un dernier paragraphe.
pub(super) fn read(text: &LoroText) -> Vec<Value> {
    let (mut out, mut runs) = (Vec::new(), Vec::new());
    for d in text.to_delta() {
        let TextDelta::Insert { insert, attributes } = d else { continue };
        let a: Attrs = attributes.map(|m| m.into_iter().collect()).unwrap_or_default();
        let mut chunk = String::new();
        for c in insert.chars() {
            if c == SEP {
                push_run(&mut runs, std::mem::take(&mut chunk), &a);
                push_paragraph(&mut out, std::mem::take(&mut runs), &a);
            } else {
                chunk.push(c);
            }
        }
        push_run(&mut runs, chunk, &a);
    }
    if !runs.is_empty() {
        push_paragraph(&mut out, runs, &Attrs::new());
    }
    out
}

#[cfg(test)]
#[path = "rich_tests.rs"]
mod rich_tests;
```

Note : dans `current`, le filtre `v_k.is_empty()` est inutile ; le simplifier en `filter(|(_, v)| !matches!(v, LoroValue::Null))`. Si le compilateur refuse la comparaison d'itérateurs de `push_run`, comparer deux `Map` construites.

- [ ] **Step 4 : brancher** — `write.rs`, branche `Kind::Rich` : `super::rich::sync(&text_at(map, key)?, value)?`. `read.rs`, avant le cas générique `Text` :

```rust
(Kind::Rich, ValueOrContainer::Container(Container::Text(t))) => {
    let paragraphs = super::rich::read(&t);
    if paragraphs.is_empty() {
        continue;
    }
    Value::Array(paragraphs)
}
```

`mod.rs` : `mod rich;` et, dans `configure`, `doc.config_text_style(rich::styles());`.

- [ ] **Step 5 : vérifier** — `cargo nextest run -p oku-slides-core` → PASS, y compris `new_then_deck_returns_same_deck` de la tâche 2. Si ce dernier échoue parce que `roadmap.json` a des runs adjacents de même format, remplacer l'égalité stricte par une comparaison sur `reconcile` → 0 op, et le noter dans le test.

- [ ] **Step 6 : commit**

```bash
git add crates/oku-slides-core
git commit -m "feat(core) : prose riche du replica en texte à marques (t-<N3>)"
```

---

### Task 4 : Lecture totale, blob non fiable, fusion

**Files :**
- Modify : `replica/read.rs` (fonction `materialize`), `replica/mod.rs` (`deck()`, `merge`)
- Create : `replica/read_tests.rs`
- Create : `crates/oku-slides-core/tests/untrusted.rs`
- Modify : `crates/oku-slides-i18n/src/locales/*/core.ftl` (3 clés `issue-unreadable-*`, annexe A)
- Modify : spec, tableau « Erreurs » (`Write` ajouté à `ReplicaError`)

**Interfaces :**
- Consumes : `read::read_map`, `ReplicaInner`.
- Produces :
  - `DeckReplica::deck(&self) -> (Deck, Vec<Issue>)` (remplace la version de la tâche 2 ; mettre à jour les tests existants en `.0`).
  - `DeckReplica::merge(&mut self, other: &DeckReplica) -> Result<(), ReplicaError>` (vérifie `deck_uid`, sinon `Unrelated`).
  - `import_for_tests` supprimé (remplacé par `merge`).

- [ ] **Step 1 : tests rouges** — `replica/read_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn issues_ids(issues: &[crate::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn materialize_unknown_element_kind_drops_element_only() {
    let root = json!({ "slides": [{ "id": "s1", "kind": "content", "elements": [
        { "id": "e-ok", "kind": "shape" },
        { "id": "e-ko", "kind": "hologramme" }
    ]}]});
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.slides[0].elements).has_length(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
}

#[test]
fn materialize_bad_slide_drops_slide_only() {
    let root = json!({ "slides": [
        { "id": "s1", "kind": "content" },
        { "id": "s2", "kind": 42 }
    ]});
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.slides).has_length(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-slide"]);
}

#[test]
fn materialize_bad_top_level_field_resets_it() {
    let root = json!({ "theme": 7, "slides": [] });
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.theme).is_none();
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
}

#[test]
fn materialize_incoherent_chart_drops_element() {
    let root = json!({ "slides": [{ "id": "s1", "kind": "content", "elements": [
        { "id": "c", "kind": "chart", "option": { "series": "pas une liste" } }
    ]}]});
    let (_, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
}
```

Dans `mod_tests.rs` :

```rust
#[test]
fn merge_unrelated_returns_unrelated() {
    let mut a = DeckReplica::new(&roadmap()).unwrap();
    let b = DeckReplica::new(&roadmap()).unwrap();
    assert_that!(a.merge(&b).unwrap_err().kind()).is_equal_to(crate::ReplicaErrorKind::Unrelated);
}
```

`crates/oku-slides-core/tests/untrusted.rs` :

```rust
use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

fn snapshot() -> Vec<u8> {
    let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
    DeckReplica::new(&deck).unwrap().snapshot().unwrap()
}

#[test]
fn from_snapshot_truncated_returns_corrupted() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s[..s.len() / 2], 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_random_bytes_returns_corrupted() {
    let bytes: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}

#[test]
fn from_snapshot_over_limit_returns_too_large() {
    let s = snapshot();
    let err = DeckReplica::from_snapshot(&s, s.len() - 1).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::TooLarge);
}

#[test]
fn from_snapshot_of_foreign_loro_doc_returns_corrupted() {
    let doc = loro::LoroDoc::new();
    doc.get_map("autre").insert("k", 1).unwrap();
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    let err = DeckReplica::from_snapshot(&bytes, 1 << 20).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReplicaErrorKind::Corrupted);
}
```

(`loro` en dépendance de dev de `oku-slides-core` pour ce dernier test : elle l'est déjà en dépendance normale.)

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core` → échecs de compilation (`materialize`, `merge`).

- [ ] **Step 3 : lecture totale** — dans `read.rs` :

```rust
use oku_slides_i18n::Message;

use crate::model::{Deck, Element, Slide};
use crate::validate::Issue;

fn issue(slide: Option<String>, element: Option<String>, message: Message) -> Issue {
    Issue { slide, element, message }
}

/// Garde les éléments lisibles (récursivement dans les groupes), signale les autres.
fn clean_elements(list: Vec<Value>, slide: &str, issues: &mut Vec<Issue>) -> Vec<Value> {
    list.into_iter()
        .filter_map(|mut e| {
            if let Some(Value::Array(children)) = e.get_mut("children") {
                *children = clean_elements(std::mem::take(children), slide, issues);
            }
            match serde_json::from_value::<Element>(e.clone()) {
                Ok(_) => Some(e),
                Err(_) => {
                    let id = e.get("id").and_then(Value::as_str).map(str::to_owned);
                    issues.push(issue(Some(slide.to_owned()), id, Message::new("issue-unreadable-element")));
                    None
                }
            }
        })
        .collect()
}

fn read_slide(mut s: Value, issues: &mut Vec<Issue>) -> Option<Slide> {
    let id = s.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    for key in ["blocks", "elements"] {
        if let Some(Value::Array(list)) = s.get_mut(key) {
            *list = clean_elements(std::mem::take(list), &id, issues);
        }
    }
    match serde_json::from_value::<Slide>(s) {
        Ok(slide) => Some(slide),
        Err(_) => {
            issues.push(issue(Some(id), None, Message::new("issue-unreadable-slide")));
            None
        }
    }
}

/// Lecture totale : tout état du replica donne un `Deck`. Ce qui ne se lit
/// pas est écarté (élément, slide) ou remis par défaut (champ du deck), et signalé.
pub(super) fn materialize(mut root: Map<String, Value>) -> (Deck, Vec<Issue>) {
    let mut issues = Vec::new();
    let slides: Vec<Slide> = match root.remove("slides") {
        Some(Value::Array(a)) => a.into_iter().filter_map(|s| read_slide(s, &mut issues)).collect(),
        _ => Vec::new(),
    };
    let keys: Vec<String> = root.keys().cloned().collect();
    for key in keys {
        let mut probe = Map::new();
        probe.insert("slides".into(), Value::Array(Vec::new()));
        if let Some(v) = root.get(&key) {
            probe.insert(key.clone(), v.clone());
        }
        if serde_json::from_value::<Deck>(Value::Object(probe)).is_err() {
            root.remove(&key);
            issues.push(issue(None, None, Message::new("issue-unreadable-field").arg("field", key)));
        }
    }
    root.insert("slides".into(), Value::Array(Vec::new()));
    let mut deck = serde_json::from_value::<Deck>(Value::Object(root)).unwrap_or_else(|_| Deck {
        version: crate::model::DECK_VERSION,
        theme: None,
        params: Default::default(),
        vars: Default::default(),
        tokens: Value::Null,
        templates: None,
        slides: Vec::new(),
    });
    deck.slides = slides;
    (deck, issues)
}
```

Adapter les chemins (`crate::validate::Issue`, `Message::arg` accepte `impl Into<Arg>` : vérifier qu'un `String` convient, sinon `.as_str()`). Si `Deck` n'implémente pas `Default`, garder la construction explicite ci-dessus.

- [ ] **Step 4 : `deck()` et `merge()`** — dans `mod.rs` :

```rust
/// Le deck relu depuis le replica, et ce qui n'a pas pu être lu.
pub fn deck(&self) -> (Deck, Vec<crate::Issue>) {
    read::materialize(read::read_map(&self.doc.get_map(ROOT), Ctx::Deck))
}

/// Fusionne `other` : seuls les changements inconnus sont importés.
pub fn merge(&mut self, other: &DeckReplica) -> Result<(), ReplicaError> {
    Ok(self.merge_inner(other)?)
}

fn merge_inner(&mut self, other: &DeckReplica) -> Result<(), ReplicaInner> {
    ensure!(self.deck_uid() == other.deck_uid(), crate::error::UnrelatedSnafu);
    let updates = other.doc.export(ExportMode::updates(&self.doc.oplog_vv())).context(ExportSnafu)?;
    self.doc.import(&updates).context(ImportSnafu)?;
    Ok(())
}
```

Supprimer `import_for_tests`, remplacer ses appels par `a.merge(&b).unwrap()`, et les `r.deck().unwrap()` par `r.deck().0`. Supprimer la variante `DeckJson` si elle n'est plus utilisée en lecture (elle reste pour `reconcile`). Vérifier le nom exact de `ExportMode::updates` (`ExportMode::updates(&vv)` en 1.16).

- [ ] **Step 5 : clés Fluent** — ajouter `issue-unreadable-element`, `issue-unreadable-slide`, `issue-unreadable-field` (argument `$field`) dans `core.ftl` des 20 locales (annexe A).

- [ ] **Step 6 : vérifier** — `cargo nextest run -p oku-slides-core -p oku-slides-i18n` → PASS.

- [ ] **Step 7 : spec** — tableau « Erreurs » : `ReplicaError` = `Corrupted`, `TooLarge`, `Write`, `Unrelated`.

- [ ] **Step 8 : commit**

```bash
git add crates/oku-slides-core crates/oku-slides-i18n docs/superpowers/specs/2026-09-27-sources-loro-design.md
git commit -m "feat(core) : lecture totale du replica, snapshot non fiable, fusion par deck_uid (t-<N4>)"
```

---

### Task 5 : Annuler et rétablir par UndoManager

**Files :**
- Create : `replica/undo.rs`, `replica/undo_tests.rs`
- Modify : `replica/mod.rs`

**Interfaces :**
- Produces (sur `DeckReplica`) :
  - `commit(&mut self, label: &str)` : une étape d'annulation portant `label`.
  - `commit_silent(&mut self)` : changement non annulable (canonicalisation au chargement, enregistrement de modèles).
  - `undo(&mut self) -> Result<bool, ReplicaError>`, `redo(&mut self) -> Result<bool, ReplicaError>`
  - `can_undo(&self) -> bool`, `can_redo(&self) -> bool`
  - `undo_label(&self) -> Option<String>`, `redo_label(&self) -> Option<String>`
  - `set_undo_limit(&mut self, steps: usize)`

- [ ] **Step 1 : tests rouges** — `replica/undo_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::{Deck, DeckReplica};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].title = Some(t.into());
    d
}

#[test]
fn undo_restores_previous_deck_and_moves_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.reconcile(&titled(&deck, "Un")).unwrap();
    r.commit("Renommer");
    assert_that!(r.undo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.redo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].title.as_deref()).is_equal_to(Some("Un"));
}

#[test]
fn new_replica_has_nothing_to_undo() {
    let r = DeckReplica::new(&roadmap()).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn commit_silent_is_not_undoable() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.reconcile(&titled(&deck, "Canonique")).unwrap();
    r.commit_silent();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn empty_commit_does_not_leak_its_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.commit("Vide");
    r.reconcile(&titled(&deck, "Deux")).unwrap();
    r.commit("Deux");
    assert_that!(r.undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn undo_after_merge_keeps_remote_change() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let mut a = DeckReplica::from_snapshot(&snap, 1 << 20).unwrap();
    let mut b = DeckReplica::from_snapshot(&snap, 1 << 20).unwrap();
    a.reconcile(&titled(&deck, "de A")).unwrap();
    a.commit("A");
    let mut db = deck.clone();
    db.slides[3].title = Some("de B".into());
    b.reconcile(&db).unwrap();
    b.commit("B");
    a.merge(&b).unwrap();
    a.undo().unwrap();
    let got = a.deck().0;
    assert_that!(got.slides[2].title).is_equal_to(&deck.slides[2].title);
    assert_that!(got.slides[3].title.as_deref()).is_equal_to(Some("de B"));
}

#[test]
fn undo_limit_drops_oldest_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.set_undo_limit(2);
    for t in ["1", "2", "3"] {
        r.reconcile(&titled(&deck, t)).unwrap();
        r.commit(t);
    }
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_false();
}
```

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/undo/)'` → échec de compilation.

- [ ] **Step 3 : implémentation** — `replica/undo.rs` :

```rust
//! Annuler / rétablir : l'`UndoManager` de Loro, qui ne défait que les
//! changements de ce pair, avec le libellé de chaque transaction.

use std::sync::{Arc, Mutex, PoisonError};

use loro::{LoroDoc, LoroValue, UndoItemMeta, UndoManager, UndoOrRedo};

/// Origine des commits non annulables.
pub(super) const SILENT: &str = "oku-silent";

pub(super) struct Undo {
    manager: UndoManager,
    /// Libellé du prochain commit local.
    next: Arc<Mutex<Option<String>>>,
}

fn take<T>(m: &Mutex<Option<T>>) -> Option<T> {
    m.lock().unwrap_or_else(PoisonError::into_inner).take()
}

impl Undo {
    pub(super) fn new(doc: &LoroDoc) -> Self {
        let mut manager = UndoManager::new(doc);
        manager.add_exclude_origin_prefix(SILENT);
        let next: Arc<Mutex<Option<String>>> = Arc::default();
        // `on_pop` précède le `on_push` de l'autre pile : le libellé passe de l'une à l'autre.
        let popped: Arc<Mutex<Option<LoroValue>>> = Arc::default();
        let (n, p) = (Arc::clone(&next), Arc::clone(&popped));
        manager.set_on_push(Some(Box::new(move |kind, _span, _event| {
            let value = match kind {
                UndoOrRedo::Undo => take(&n).map(LoroValue::from).or_else(|| take(&p)),
                UndoOrRedo::Redo => take(&p),
            };
            UndoItemMeta { value: value.unwrap_or(LoroValue::Null), cursors: Vec::new() }
        })));
        let p = Arc::clone(&popped);
        manager.set_on_pop(Some(Box::new(move |_kind, _span, meta| {
            *p.lock().unwrap_or_else(PoisonError::into_inner) = Some(meta.value);
        })));
        Self { manager, next }
    }

    pub(super) fn label_next(&self, label: &str) {
        *self.next.lock().unwrap_or_else(PoisonError::into_inner) = Some(label.to_owned());
    }

    /// Après un commit : un commit vide n'a rien empilé, son libellé ne doit pas fuir.
    pub(super) fn clear_next(&self) {
        take(&self.next);
    }

    pub(super) fn manager(&mut self) -> &mut UndoManager {
        &mut self.manager
    }

    pub(super) fn label(v: Option<LoroValue>) -> Option<String> {
        match v {
            Some(LoroValue::String(s)) => Some(s.to_string()),
            _ => None,
        }
    }

    pub(super) fn top_undo(&self) -> Option<String> {
        Self::label(self.manager.top_undo_value())
    }

    pub(super) fn top_redo(&self) -> Option<String> {
        Self::label(self.manager.top_redo_value())
    }

    pub(super) fn can_undo(&self) -> bool {
        self.manager.can_undo()
    }

    pub(super) fn can_redo(&self) -> bool {
        self.manager.can_redo()
    }
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod undo_tests;
```

Dans `mod.rs` : champ `undo: undo::Undo` ; construit par `Undo::new(&doc)` dans `create` (**après** le commit initial, pour que la création ne soit pas annulable) et dans `import_snapshot`. Méthodes :

```rust
pub fn commit(&mut self, label: &str) {
    self.undo.label_next(label);
    self.doc.commit();
    self.undo.clear_next();
}

pub fn commit_silent(&mut self) {
    self.doc.set_next_commit_origin(undo::SILENT);
    self.doc.commit();
}

pub fn undo(&mut self) -> Result<bool, ReplicaError> {
    Ok(self.undo.manager().undo().context(LoroSnafu)?)
}

pub fn redo(&mut self) -> Result<bool, ReplicaError> {
    Ok(self.undo.manager().redo().context(LoroSnafu)?)
}

pub fn can_undo(&self) -> bool { self.undo.can_undo() }
pub fn can_redo(&self) -> bool { self.undo.can_redo() }
pub fn undo_label(&self) -> Option<String> { self.undo.top_undo() }
pub fn redo_label(&self) -> Option<String> { self.undo.top_redo() }
pub fn set_undo_limit(&mut self, steps: usize) { self.undo.manager().set_max_undo_steps(steps.max(1)); }
```

Dans `create`, remplacer `r.commit("")` par un `doc.commit()` direct avant de construire `Undo`. Mettre à jour `Debug` si besoin (`Undo` n'est pas `Debug` : ne pas l'afficher).

- [ ] **Step 4 : vérifier** — `cargo nextest run -p oku-slides-core` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core
git commit -m "feat(core) : annuler et rétablir par l'UndoManager du replica, avec libellés (t-<N5>)"
```

---

### Task 6 : Convergence par propriété

**Files :**
- Modify : `Cargo.toml` (`proptest = "1"` en `[workspace.dependencies]`), `crates/oku-slides-core/Cargo.toml` (dev)
- Create : `crates/oku-slides-core/tests/convergence.rs`

**Interfaces :**
- Consumes : `DeckReplica::{new, from_snapshot, snapshot, reconcile, commit, merge, deck}`.

- [ ] **Step 1 : audit** — lancer l'agent `common:team-dep-auditor` sur l'ajout de `proptest` (dev seulement). Corriger ce qu'il signale.

- [ ] **Step 2 : test** — `tests/convergence.rs` :

```rust
//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.

use oku_slides_core::{Deck, DeckReplica, ElementBody, Slide, SlideKind, validate};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Edit {
    Title(usize, String),
    Notes(usize, String),
    Move(usize, usize),
    Delete(usize),
    Add(usize),
    Theme(Option<String>),
    Bold(usize),
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<usize>(), "[a-zé ]{0,10}").prop_map(|(i, s)| Edit::Title(i, s)),
        (any::<usize>(), "[a-z ]{0,10}").prop_map(|(i, s)| Edit::Notes(i, s)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Move(a, b)),
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Add),
        prop::option::of(prop_oneof![Just("papier".to_owned()), Just("pro".to_owned())]).prop_map(Edit::Theme),
        any::<usize>().prop_map(Edit::Bold),
    ]
}

fn apply(deck: &mut Deck, e: &Edit, peer: usize, n: &mut usize) {
    let len = deck.slides.len();
    match e {
        Edit::Add(i) => {
            *n += 1;
            let s = Slide::new(format!("p{peer}-{n}"), SlideKind::Content);
            deck.slides.insert(if len == 0 { 0 } else { i % (len + 1) }, s);
        }
        _ if len == 0 => {}
        Edit::Title(i, s) => deck.slides[i % len].title = (!s.trim().is_empty()).then(|| s.clone()),
        Edit::Notes(i, s) => deck.slides[i % len].notes.push_str(s),
        Edit::Move(a, b) => {
            let s = deck.slides.remove(a % len);
            deck.slides.insert(b % len, s);
        }
        Edit::Delete(i) => {
            deck.slides.remove(i % len);
        }
        Edit::Theme(t) => deck.theme = t.clone(),
        Edit::Bold(i) => {
            let slide = &mut deck.slides[i % len];
            for el in slide.elements.iter_mut().chain(slide.blocks.iter_mut()) {
                if let ElementBody::Text { text } = &mut el.body
                    && let Some(run) = text.first_mut().and_then(|p| p.runs.first_mut())
                {
                    run.bold = !run.bold;
                    return;
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn peers_editing_in_parallel_converge(
        edits in prop::collection::vec(prop::collection::vec(edit(), 0..10), 2..4),
        seed in any::<u64>(),
    ) {
        let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
        let snap = DeckReplica::new(&deck).unwrap().snapshot().unwrap();
        let mut peers: Vec<DeckReplica> =
            edits.iter().map(|_| DeckReplica::from_snapshot(&snap, 1 << 24).unwrap()).collect();
        for (p, (replica, list)) in peers.iter_mut().zip(&edits).enumerate() {
            let (mut d, mut n) = (replica.deck().0, 0);
            for e in list {
                apply(&mut d, e, p, &mut n);
                replica.reconcile(&d).unwrap();
                replica.commit("edit");
            }
        }
        // Échanges dans un ordre pseudo-aléatoire, jusqu'à ce que chacun ait tout.
        let k = peers.len();
        let mut order: Vec<(usize, usize)> = (0..k).flat_map(|a| (0..k).map(move |b| (a, b))).filter(|(a, b)| a != b).collect();
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            order.swap(i, (x % (i as u64 + 1)) as usize);
        }
        for _ in 0..2 {
            for &(a, b) in &order {
                let src = DeckReplica::from_snapshot(&peers[b].snapshot().unwrap(), 1 << 24).unwrap();
                peers[a].merge(&src).unwrap();
            }
        }
        let (first, issues) = peers[0].deck();
        prop_assert!(issues.is_empty(), "lecture partielle : {issues:?}");
        for p in &peers[1..] {
            prop_assert_eq!(&p.deck().0, &first);
        }
        prop_assert!(validate(&first, None).iter().all(|i| i.message.id() != "issue-duplicate-slide-id"));
    }
}
```

- [ ] **Step 3 : lancer** — `cargo nextest run -p oku-slides-core --test convergence` → PASS. Un échec : proptest affiche le cas minimal. **Ne pas** affaiblir la propriété : écrire d'abord un test unitaire dans `mod_tests.rs` qui reproduit le cas minimal (G13), puis corriger la réconciliation.

- [ ] **Step 4 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-core
git commit -m "test(core) : convergence du replica par propriété, N pairs en parallèle (t-<N6>)"
```

---

### Task 7 : Zip des sources et PDF en version 2

**Files :**
- Modify : `crates/oku-slides-core/src/sources.rs`, `sources_tests.rs`
- Modify : `crates/oku-slides-pdf/src/lib.rs`, `crates/oku-slides-pdf/tests/blob.rs`
- Modify : `crates/oku-slides-platform/src/config.rs`, `config.default.toml`, `config_tests.rs`
- Modify : appelants de `SourceBundle::new` / `from_zip` : `crates/oku-slides-cli/src/{main.rs,bench.rs}`, `crates/oku-slides-render/tests/render.rs`, `crates/oku-slides-core/tests/{ops.rs,regressions/issue_0001_asset_id_escaping_or_injecting.rs}`, `crates/oku-slides-session/src/{files.rs,lib.rs,resume.rs}` (compilation seulement ; la logique de session change en tâche 8)

**Interfaces :**
- Consumes : `DeckReplica`.
- Produces :
  - `SOURCES_VERSION: u32 = 2`
  - `pub struct SourceBundle { replica: DeckReplica, deck: Deck, pub assets, pub fonts }` (`Debug`, `PartialEq` manuel sur `deck`, `assets`, `fonts` ; **plus de `Clone`**).
  - `SourceBundle::new(deck: Deck) -> Result<SourceBundle, SourcesError>`
  - `SourceBundle::from_replica(replica: DeckReplica, assets, fonts) -> (SourceBundle, Vec<Issue>)`
  - `SourceBundle::deck(&self) -> &Deck`, `SourceBundle::replica(&self) -> &DeckReplica`
  - `SourceBundle::apply(&mut self, deck: Deck, label: &str) -> Result<(), SourcesError>` (réconcilie, committe, remplace la vue)
  - `SourceBundle::apply_silent(&mut self, deck: Deck) -> Result<(), SourcesError>`
  - `SourceBundle::undo(&mut self) -> Result<bool, SourcesError>`, `redo(&mut self) -> Result<bool, SourcesError>` (relisent la vue)
  - `SourceBundle::to_zip(&self) -> Result<Vec<u8>, SourcesError>`
  - `SourceBundle::from_zip(bytes: &[u8], max_replica_bytes: usize) -> Result<(SourceBundle, Vec<Issue>), SourcesError>`
  - `SourcesError` : nouvelle variante interne `Replica { source: ReplicaError }`, `kind()` → nouveau `SourcesErrorKind::Replica(ReplicaErrorKind)` ; `Localize` délègue à l'erreur de replica.
  - `oku_slides_pdf::VERSION = 2` ; blob v1 → `Ok(None)`.
  - `oku_slides_platform::config::SourcesSettings { max_replica_mib: usize }` + `fn max_replica_bytes(&self) -> usize`.

- [ ] **Step 1 : tests rouges (sources)** — dans `sources_tests.rs`, remplacer `MANIFEST`/`DECK` par un manifeste v2 et un `deck.loro` produit par `DeckReplica::new`, et ajouter :

```rust
const MAX: usize = 1 << 24;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn to_zip_then_from_zip_returns_same_bundle() {
    let mut b = SourceBundle::new(roadmap()).unwrap();
    b.add_asset(b"png".to_vec());
    let (back, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&b);
    assert_that!(back.replica().deck_uid()).is_equal_to(b.replica().deck_uid());
}

#[test]
fn to_zip_has_no_deck_json() {
    let zip = SourceBundle::new(roadmap()).unwrap().to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n == "deck.json")).is_false();
    assert_that!(za.file_names().any(|n| n == "deck.loro")).is_true();
}

#[test]
fn from_zip_version_1_returns_not_sources() {
    let manifest = br#"{"format":"oku-slides-sources","version":1}"#;
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", manifest), ("deck.json", DECK_JSON)]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_without_deck_loro_returns_missing_deck() {
    let err = SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)]), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn apply_then_undo_restores_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let mut next = deck.clone();
    next.theme = Some("pro".into());
    b.apply(next, "Thème").unwrap();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(b.undo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&deck);
}
```

(`MANIFEST` devient `{"format":"oku-slides-sources","version":2}` ; `DECK_JSON` garde l'ancien `{"slides":[]}`.) Adapter les autres tests existants de `sources_tests.rs` à la nouvelle signature.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/sources/)'` → échec de compilation.

- [ ] **Step 3 : implémentation (sources)** — dans `sources.rs` :
  - `SOURCES_VERSION = 2`.
  - Écriture : `manifest.json`, puis `deck.loro` (`Stored` : le snapshot Loro est déjà compact ; mesurer, et passer en `Deflated` si le zip v2 de `roadmap.json` gagne plus de 20 %), puis `assets/`, `fonts/` comme avant.
  - Lecture : `deck.loro` → lu seulement si `f.size() <= max_replica_bytes as u64`, sinon `Replica(TooLarge)` sans lire l'entrée (anti-bombe) ; `manifest.version != SOURCES_VERSION` : `< 2` → `NotSources`, `> 2` → `UnsupportedVersion` ; `deck.loro` absent → `MissingDeck` ; `deck.json` ignoré.
  - `SourceBundle` :

```rust
/// Le deck (vue du replica), ses assets et ses polices.
#[derive(Debug)]
pub struct SourceBundle {
    replica: DeckReplica,
    /// Vue matérialisée du replica : jamais modifiée hors de `apply`, `undo`, `redo`.
    deck: Deck,
    /// Assets par identifiant (`sha256:…`), en ajout seul.
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Polices par nom de fichier, en ajout seul.
    pub fonts: BTreeMap<String, Vec<u8>>,
}

impl PartialEq for SourceBundle {
    fn eq(&self, other: &Self) -> bool {
        self.deck == other.deck && self.assets == other.assets && self.fonts == other.fonts
    }
}

impl SourceBundle {
    pub fn new(deck: Deck) -> Result<Self, SourcesError> {
        let replica = DeckReplica::new(&deck).context(ReplicaSnafu)?;
        Ok(Self { replica, deck, assets: BTreeMap::new(), fonts: BTreeMap::new() })
    }

    pub fn from_replica(
        replica: DeckReplica,
        assets: BTreeMap<String, Vec<u8>>,
        fonts: BTreeMap<String, Vec<u8>>,
    ) -> (Self, Vec<Issue>) {
        let (deck, issues) = replica.deck();
        (Self { replica, deck, assets, fonts }, issues)
    }

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    pub fn replica(&self) -> &DeckReplica {
        &self.replica
    }

    /// Remplace le deck par `deck`, en une étape d'annulation `label`.
    pub fn apply(&mut self, deck: Deck, label: &str) -> Result<(), SourcesError> {
        self.replica.reconcile(&deck).context(ReplicaSnafu)?;
        self.replica.commit(label);
        self.deck = deck;
        Ok(())
    }

    /// Remplace le deck par `deck`, sans étape d'annulation.
    pub fn apply_silent(&mut self, deck: Deck) -> Result<(), SourcesError> {
        self.replica.reconcile(&deck).context(ReplicaSnafu)?;
        self.replica.commit_silent();
        self.deck = deck;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, SourcesError> {
        let done = self.replica.undo().context(ReplicaSnafu)?;
        if done {
            self.deck = self.replica.deck().0;
        }
        Ok(done)
    }

    pub fn redo(&mut self) -> Result<bool, SourcesError> {
        let done = self.replica.redo().context(ReplicaSnafu)?;
        if done {
            self.deck = self.replica.deck().0;
        }
        Ok(done)
    }

    pub fn set_undo_limit(&mut self, steps: usize) {
        self.replica.set_undo_limit(steps);
    }
}
```

  Les `Issue` de la relecture après `undo`/`redo` sont écartés : elles ne peuvent venir que d'un état déjà relu (et signalé) au chargement. Le dire en commentaire.

- [ ] **Step 4 : erreur de sources** — `SourcesInner::Replica { source: ReplicaError, backtrace }`, `SourcesErrorKind::Replica(ReplicaErrorKind)` (retirer `Copy`/`Eq` s'ils bloquent ; `ReplicaErrorKind` est `Copy + Eq`, donc ils restent), `Localize` : `SourcesErrorKind::Replica(_) => return source.localize(l)` (garder la source accessible via un `match &self.0`).

- [ ] **Step 5 : vérifier (core)** — `cargo nextest run -p oku-slides-core` → PASS.

- [ ] **Step 6 : tests rouges (PDF)** — `crates/oku-slides-pdf/tests/blob.rs` : l'assertion `version == 1` devient `2`, et :

```rust
fn with_version(pdf: &[u8], version: i64) -> Vec<u8> {
    let mut doc = Document::load_mem(pdf).unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let piece = doc.get_object(root).unwrap().as_dict().unwrap().get(b"PieceInfo").unwrap().as_dict().unwrap().clone();
    let mut piece = piece;
    let mut oku = piece.get(b"OkuSlides").unwrap().as_dict().unwrap().clone();
    let mut private = oku.get(b"Private").unwrap().as_dict().unwrap().clone();
    private.set("Version", Object::Integer(version));
    oku.set("Private", Object::Dictionary(private));
    piece.set("OkuSlides", Object::Dictionary(oku));
    doc.get_object_mut(root).unwrap().as_dict_mut().unwrap().set("PieceInfo", Object::Dictionary(piece));
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

#[test]
fn extract_sources_version_1_returns_none() {
    let out = with_version(&oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap(), 1);
    assert_that!(oku_slides_pdf::extract_sources(&out).unwrap()).is_none();
}

#[test]
fn extract_sources_version_3_returns_unsupported_version() {
    let out = with_version(&oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap(), 3);
    assert_that!(oku_slides_pdf::extract_sources(&out).unwrap_err().kind())
        .is_equal_to(oku_slides_pdf::PdfErrorKind::UnsupportedVersion);
}
```

`cargo nextest run -p oku-slides-pdf` → FAIL.

- [ ] **Step 7 : implémentation (PDF)** — `VERSION = 2` ; dans `extract_sources` :

```rust
let version = private.get(b"Version").and_then(Object::as_i64).unwrap_or(0);
// Plus ancien : format ignoré (pas de conversion, spec t-188).
if version < VERSION {
    return Ok(None);
}
ensure!(version == VERSION, UnsupportedVersionSnafu { version });
```

Mettre à jour le doc de tête du module (`/Version 2`, contenu du zip v2). `cargo nextest run -p oku-slides-pdf` → PASS.

- [ ] **Step 8 : réglage** — `config.default.toml` :

```toml
[sources]
# Taille maximale de l'historique d'un deck lu dans un PDF ou dans la reprise (Mio).
max_replica_mib = 32
```

`config.rs` : `pub sources: SourcesSettings` ; `SourcesSettings { pub max_replica_mib: usize }` avec `pub fn max_replica_bytes(&self) -> usize { self.max_replica_mib.saturating_mul(1024 * 1024) }`. Test dans `config_tests.rs` : la valeur par défaut vaut `32 * 1024 * 1024` octets.

- [ ] **Step 9 : appelants** — `SourceBundle::new(deck)` → `SourceBundle::new(deck)?` (ou `.unwrap()` dans les tests) ; `from_zip(bytes)` → `from_zip(bytes, max)?` ; `bundle.deck` → `bundle.deck()` en lecture. Dans la session, remplacer **pour l'instant** les écritures directes `sources.deck = …` / `&mut sources.deck` par un `let mut deck = sources.deck().clone(); …; sources.apply_silent(deck)?;` (la tâche 8 refait ces chemins). `render.rs:163` : l'égalité de bundles reste valable (`PartialEq` manuel). `files.rs` : `Opened` gagne `pub issues: Vec<Issue>` (vide pour un deck JSON). `resume.rs` : compiler seulement (`SourceBundle { deck: raw.deck, … }` → `SourceBundle::new(raw.deck)?` puis réaffecter `assets`/`fonts`), la tâche 8 le réécrit.

- [ ] **Step 10 : vérifier tout** — `cargo nextest run --workspace --exclude oku-slides-app` → PASS ; `cargo clippy --workspace --all-targets -- -D warnings` → propre.

- [ ] **Step 11 : commit**

```bash
git add crates Cargo.lock
git commit -m "feat(core)!: sources v2, le replica Loro remplace deck.json dans le zip et le PDF (t-<N7>)"
```

---

### Task 8 : Session et reprise sur le replica

**Files :**
- Modify : `crates/oku-slides-session/src/lib.rs` (`Document`, `apply`, `undo`, `redo`, `rerender`, `snapshot`, `tab_info`, `load_document`, `save_templates`, `open`)
- Modify : `crates/oku-slides-session/src/views.rs` (`undo_label`/`redo_label` : `Option<String>`)
- Modify : `crates/oku-slides-session/src/resume.rs`, `resume_tests.rs`, `resume_writer_tests.rs`
- Modify : `crates/oku-slides-session/src/files.rs`
- Modify : `crates/oku-slides-i18n/src/locales/*/session.ftl` (`notice-unreadable-parts`, annexe A)
- Test : `crates/oku-slides-session/src/lib_tests.rs` (ou le fichier de tests de session existant, à repérer avec `ls crates/oku-slides-session/src/*tests*`)

**Interfaces :**
- Consumes : `SourceBundle::{apply, apply_silent, undo, redo, deck, replica, set_undo_limit, from_replica}`, `DeckReplica::{snapshot, from_snapshot, can_undo, can_redo, undo_label, redo_label}`, `SourcesSettings::max_replica_bytes`.
- Produces : `Document` sans `history` ; `Session::apply` rendre-puis-valider ; reprise `tabs/t-<n>/deck/deck.loro` + `document.json { path, dirty, saved_hash, blank }`.

- [ ] **Step 1 : tests rouges (session)** — dans le fichier de tests de `Session` :

```rust
#[test]
fn apply_with_render_failure_leaves_replica_untouched() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    let ops_before = s.replica_ops_for_tests();
    // Un thème inconnu fait échouer le rendu Typst.
    let err = s.apply("Thème", vec![Op::SetTheme { theme: Some("n-existe-pas".into()) }]);
    assert_that!(err).is_err();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.replica_ops_for_tests()).is_equal_to(ops_before);
}

#[test]
fn apply_then_undo_then_redo_keeps_labels() {
    let mut s = session_with_fixture();
    s.apply("Renommer", vec![title_op(&s, "Nouveau")]).unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Renommer".to_owned()));
    s.undo().unwrap();
    assert_that!(s.snapshot().redo_label).is_equal_to(Some("Renommer".to_owned()));
    s.redo().unwrap();
    assert_that!(s.snapshot().can_undo).is_true();
}
```

Réutiliser les helpers de construction de session déjà présents dans les tests (chercher `fn session` dans `crates/oku-slides-session/src/*tests*.rs`) ; `title_op` fabrique un `Op::SetSlideField` sur la troisième slide ; `replica_ops_for_tests` : `#[cfg(test)] pub(crate) fn replica_ops_for_tests(&self) -> usize { self.doc.sources.replica().len_ops() }`. Si le thème inconnu ne fait pas échouer le rendu, provoquer l'échec autrement (repérer comment les tests existants de `apply` le font : `grep -n "render" crates/oku-slides-session/src/*tests*.rs`).

- [ ] **Step 2 : tests rouges (reprise)** — dans `resume_writer_tests.rs`, le helper `document` devient :

```rust
fn document(theme: &str) -> Document {
    let deck = Deck::from_json(r#"{"theme":"papier","slides":[]}"#).unwrap();
    let mut doc = Document::new(SourceBundle::new(deck.clone()).unwrap(), None, 10);
    let mut next = deck;
    next.theme = Some(theme.into());
    doc.sources.apply(next, "Thème").unwrap();
    doc
}
```

`job` : `DocSnapshot::of(&document(t), &mut Vec::new()).unwrap()`. Les lectures `read_doc(DECK_DIR, 10)` deviennent `read_doc(DECK_DIR, 10, 1 << 24)` et `doc.sources.deck.theme` devient `doc.sources.deck().theme`. Ajouter :

```rust
/// La reprise garde le deck et son identité, pas la pile d'annulation.
#[test]
fn read_doc_after_write_keeps_deck_and_uid_but_not_undo() {
    let dir = temp_dir();
    let r = resume(&dir);
    let doc = document("pro");
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, 10, 1 << 24).unwrap();
    assert_that!(back.sources.deck()).is_equal_to(doc.sources.deck());
    assert_that!(back.sources.replica().deck_uid()).is_equal_to(doc.sources.replica().deck_uid());
    assert_that!(back.sources.replica().can_undo()).is_false();
    std::fs::remove_dir_all(&dir).ok();
}

/// Un document de reprise à l'ancien format (sans `deck.loro`) ne se relit pas.
#[test]
fn read_doc_without_deck_loro_returns_error() {
    let dir = temp_dir();
    let sub = dir.join(DECK_DIR);
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("document.json"),
        r#"{"path":null,"dirty":false,"saved_hash":null,"blank":false,"deck":{"slides":[]},"history":{}}"#,
    )
    .unwrap();
    assert_that!(resume(&dir).read_doc(DECK_DIR, 10, 1 << 24)).is_err();
    std::fs::remove_dir_all(&dir).ok();
}
```

(`std::fs` est permis en test ; le code de production passe par `Storage`.) Faire les mêmes remplacements dans `resume_tests.rs` (`doc.history.commit(...)` → `doc.sources.apply(...)`, `sources.deck` → `sources.deck()`). Ces tests échouent à la compilation tant que `DocSnapshot` porte un `deck`.

- [ ] **Step 3 : vérifier l'échec** — `cargo nextest run -p oku-slides-session` → FAIL / erreurs de compilation.

- [ ] **Step 4 : `Document`** — retirer `history` ; `Document::new(mut sources, path, undo_limit)` appelle `sources.set_undo_limit(undo_limit)`. Les champs `history_limit` de `Session` restent (réglage `history.limit`, désormais profondeur d'annulation).

- [ ] **Step 5 : `apply`, `rerender`** — séparer le rendu de son installation :

```rust
fn install(&mut self, compiled: Compiled) {
    // Le cache ne garde que les pages encore présentes.
    let live: HashSet<u128> = (0..compiled.page_count()).filter_map(|i| compiled.page_hash(i)).collect();
    self.svg_cache.retain(|h, _| live.contains(h));
    self.appearance = compiled.layout().tokens.as_ref().map(appearance).unwrap_or_default();
    self.compiled = Some(compiled);
}

fn rerender(&mut self) -> Result<()> {
    let started = std::time::Instant::now();
    let compiled =
        self.renderer.render(self.doc.sources.deck(), &self.doc.sources.assets).context(RenderSnafu)?;
    tracing::debug!(pages = compiled.page_count(), ms = started.elapsed().as_millis(), "rendu");
    self.install(compiled);
    Ok(())
}

/// Applique une transaction : rendue d'abord, validée ensuite. L'historique
/// du replica est définitif ; un rendu raté n'y laisse aucune trace.
pub fn apply(&mut self, label: &str, mut ops: Vec<Op>) -> Result<()> {
    charts::canonicalize_charts(&mut ops, &self.chart_labels);
    let mut next = self.doc.sources.deck().clone();
    oku_slides_core::apply_all(&mut next, ops).context(OpSnafu)?;
    let compiled = self.renderer.render(&next, &self.doc.sources.assets).context(RenderSnafu)?;
    self.doc.sources.apply(next, label).context(SourcesSnafu)?;
    self.install(compiled);
    self.doc.dirty = true;
    Ok(())
}
```

Utiliser le vrai nom du type rendu (`Compiled` ou autre : voir la signature de `renderer.render`). Si `apply_all` renvoie encore des inverses (avant la tâche 9), ignorer la valeur : `oku_slides_core::apply_all(&mut next, ops).context(OpSnafu)?;`.

- [ ] **Step 6 : `undo`/`redo`**

```rust
pub fn undo(&mut self) -> Result<()> {
    if self.doc.sources.undo().context(SourcesSnafu)? {
        self.doc.dirty = true;
        self.rerender()?;
    }
    Ok(())
}
```

(idem `redo`).

- [ ] **Step 7 : vue** — `snapshot()` : `can_undo: doc.sources.replica().can_undo()`, `undo_label: doc.sources.replica().undo_label()` (type `Option<String>` dans `views.rs`), idem redo. `tab_info().pristine` : `!deck.sources.replica().can_undo() && !deck.sources.replica().can_redo()`.

- [ ] **Step 8 : écritures directes restantes** — `load_document` :

```rust
let mut deck = doc.sources.deck().clone();
charts::canonicalize_deck(&mut deck, &self.chart_labels);
if deck != *doc.sources.deck() {
    // Forme canonique des graphiques : pas une action de l'utilisateur.
    doc.sources.apply_silent(deck).context(SourcesSnafu)?;
}
```

`save_templates` : construire le deck modifié (titre de copie, `templates = Some(id)`) puis un seul `self.doc.sources.apply_silent(deck)`. Chercher les autres mutations : `grep -n "sources\.deck\b\|&mut .*deck" crates/oku-slides-session/src/*.rs` ; il ne doit plus en rester.

- [ ] **Step 9 : ouverture** — `files.rs::read_document` : `let (sources, issues) = SourceBundle::from_zip(&found.zip, max).context(SourcesSnafu)?;` (`max` passé en paramètre depuis la session : `settings.sources.max_replica_bytes()`) ; `Opened { issues, .. }`. `Session::open` : si `!issues.is_empty()`, avis `notice-unreadable-parts` (argument `count`), et `tracing::warn!` avec les `Issue` localisés. Ajouter la clé dans `session.ftl` des 20 locales (annexe A). Si un avis `modified_elsewhere` existe déjà, joindre les deux textes par un saut de ligne.

- [ ] **Step 10 : reprise** — `resume.rs` :
  - `DocSnapshot { replica: Vec<u8>, path, dirty, saved_hash, blank, blobs }` ; `DocSnapshot::of(doc, cache) -> Result<Self>` prend `doc.sources.replica().snapshot()` ; l'appelant journalise l'erreur (`warn`) et n'écrit rien pour ce document.
  - `write_snapshot` : assets, polices, puis `deck.loro` (`put`, atomique), puis `document.json` (`DocOut { path, dirty, saved_hash, blank }`).
  - `read_doc(sub, undo_limit, max_replica_bytes)` : lit `deck.loro` → `DeckReplica::from_snapshot` → `SourceBundle::from_replica(replica, assets, fonts)` ; les `Issue` sont journalisées (`warn`) ; `Document::new(sources, path, undo_limit)` puis `dirty`, `saved_hash`, `blank` depuis `document.json`. Retirer `History`, `DocIn.deck`, `DocIn.history`.
  - Mettre à jour les appelants de `read_doc` (`workspace_resume.rs`, `lib.rs`) avec le réglage.

- [ ] **Step 11 : vérifier** — `cargo nextest run -p oku-slides-session -p oku-slides-i18n` → PASS ; puis `cargo nextest run --workspace --exclude oku-slides-app` → PASS ; `cargo build -p oku-slides-app` (Tauri) → compile.

- [ ] **Step 12 : vérification réelle** — lancer l'app ou le serveur (`just serve`) : ouvrir `fixtures/roadmap.json`, modifier un titre, annuler, rétablir, enregistrer en `.oku-slides.pdf`, rouvrir le PDF : le deck est identique, et `unzip -l` sur le blob extrait (via `oku-slides` CLI si une commande existe, sinon un test) montre `deck.loro`. Relancer l'app : l'onglet revient, sans pile d'annulation.

- [ ] **Step 13 : commit**

```bash
git add crates Cargo.lock
git commit -m "feat(session)!: rendre avant de valider, annuler par le replica, reprise en deck.loro (t-<N8>)"
```

---

### Task 9 : Supprimer History et les inverses d'ops

**Files :**
- Delete : `crates/oku-slides-core/src/history.rs`
- Modify : `crates/oku-slides-core/src/lib.rs`, `ops.rs`, `error.rs`, `tests/ops.rs`
- Modify : `crates/oku-slides-i18n/src/locales/*/core.ftl` (clé `op-inconsistent` retirée des 20 locales si elle n'est plus utilisée)

**Interfaces :**
- Produces : `pub fn apply_all(deck: &mut Deck, ops: Vec<Op>) -> Result<(), TransactionError>` ; `Op::apply(self, deck: &mut Deck) -> Result<(), OpError>`.

- [ ] **Step 1 : test d'atomicité (garde-fou)** — vérifier que `apply_all_cell_outside_new_grid_returns_error_and_keeps_deck` existe et passe (`cargo nextest run -p oku-slides-core -E 'test(keeps_deck)'`). Il doit rester vert après la tâche.

- [ ] **Step 2 : supprimer les tests d'inverses et d'historique** — dans `tests/ops.rs`, retirer `apply_all_set_grid_and_set_cell_then_inverses_restore_deck` (le remplacer par la même séquence qui vérifie le deck obtenu), `apply_each_op_then_inverse_restores_deck`, `history_commit_then_undo_redo_restores_each_state`, `history_commit_with_failing_op_rolls_back_whole_transaction` (garder la partie atomicité en l'écrivant sur `apply_all`), `history_serde_round_trip_keeps_undo_and_redo`, `history_set_limit_drops_oldest_transactions`.

- [ ] **Step 3 : `apply_all`**

```rust
/// Applique une suite d'ops de façon atomique : en cas d'erreur, le deck
/// n'a pas changé.
pub fn apply_all(deck: &mut Deck, ops: Vec<Op>) -> Result<(), TransactionError> {
    let mut next = deck.clone();
    for (index, op) in ops.into_iter().enumerate() {
        op.apply(&mut next).map_err(|source| TransactionSnafu { index }.into_error(source))?;
    }
    *deck = next;
    Ok(())
}
```

- [ ] **Step 4 : `Op::apply`** — signature `-> Result<(), OpError>` ; dans chaque branche, supprimer la construction de l'op inverse et la sauvegarde de l'ancienne valeur qui ne servait qu'à elle ; terminer par `Ok(())`. Mettre à jour le doc de tête de `ops.rs` (« Chaque op appliquée renvoie son inverse » disparaît).

- [ ] **Step 5 : code mort** — supprimer `history.rs`, `pub mod history`, `pub use history::History`, la variante `OpInner::Inconsistent`, `OpErrorKind::Inconsistent`, `InconsistentSnafu` et la clé `op-inconsistent` des 20 `core.ftl` si plus rien ne l'utilise (`grep -rn "Inconsistent\|op-inconsistent" crates`). Mettre à jour la description du crate dans `crates/oku-slides-core/Cargo.toml` (« historique » → « replica »).

- [ ] **Step 6 : vérifier** — `cargo nextest run --workspace --exclude oku-slides-app` → PASS ; `cargo clippy --workspace --all-targets -- -D warnings` → propre.

- [ ] **Step 7 : commit**

```bash
git add crates
git commit -m "refactor(core)!: supprimer History et les inverses d'ops, remplacés par le replica (t-<N9>)"
```

---

### Task 10 : Documentation, ADR et vérification complète

**Files :**
- Modify : `crates/oku-slides-core/ARCHITECTURE.md` (créer s'il n'existe pas, < 50 lignes), `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-pdf/ARCHITECTURE.md` (s'il existe)
- Modify : `docs/superpowers/specs/2026-09-26-resume-state-design.md`
- Create : `docs/adr/0002-sources-du-deck-en-crdt-loro.md` ; Modify : `docs/adr/README.md`

- [ ] **Step 1 : ARCHITECTURE** — core : `replica` (rôle, `DeckReplica`, schéma en 4 catégories, lecture totale), `SourceBundle` qui garde la vue, plus de `History`. Session : `apply` rendre-puis-valider, annulation par le replica, disposition `tabs/t-<n>/deck/` = `deck.loro`, `document.json`, `assets/`, `fonts/`. PDF : `/Version 2`, v1 ignorée.

- [ ] **Step 2 : spec de reprise** — le point « historique annuler / rétablir » est retiré de l'objectif, avec un renvoi vers t-188 et l'ADR-0002.

- [ ] **Step 3 : ADR-0002** — lancer l'agent `common:adr-writer` avec : contexte (blob zip sans passé, objectifs historique et fusion, temps réel plus tard), décision (Loro, réconciliation par diff, 4 catégories, vide = absent, `UndoManager`), alternatives écartées (Git bundle, journal d'ops, crop), conséquences dont les **reculs assumés** : pile d'annulation perdue au relancement (recul sur t-145), PDF v1 plus lisibles, PDF qui ne maigrit jamais ; limites connues de la spec. Statut : accepté. Ajouter la ligne dans `docs/adr/README.md`.

- [ ] **Step 4 : consolidation (G10)** — chercher les doublons : `grep -rn "fn loro_to_json\|fn json_to_loro\|fn sync_text" crates` (une seule définition chacun) ; aucun `History` ni `history.` hors réglage (`grep -rn "History\b" crates src-tauri --include='*.rs'`) ; aucune écriture directe de `sources.deck`.

- [ ] **Step 5 : vérification complète** — `just check` puis `just test` → tout passe. Lancer la revue `common:review` sur la branche et traiter ses retours.

- [ ] **Step 6 : commit**

```bash
git add crates docs
git commit -m "docs(core) : architecture du replica, ADR-0002, reprise sans pile d'annulation (t-<N10>)"
```

- [ ] **Step 7 : clôture** — `oku cli task done` sur chaque sous-tâche restante ; t-188 reste ouverte (plans 2 et 3).

---

## Annexe A : clés Fluent

Ajoutées dans `core.ftl` : `replica-corrupted`, `replica-too-large`, `replica-write`, `replica-unrelated` (tâche 2), `issue-unreadable-element`, `issue-unreadable-slide`, `issue-unreadable-field` (tâche 4). Dans `session.ftl` : `notice-unreadable-parts` (tâche 8). Garder l'ordre des fichiers existants ; les messages `issue-*` sont en minuscules comme leurs voisins.

| Clé | en | fr |
|---|---|---|
| replica-corrupted | The presentation history in this file is damaged | L'historique de la présentation dans ce fichier est abîmé |
| replica-too-large | The presentation history in this file is too large to open | L'historique de la présentation dans ce fichier est trop volumineux pour être ouvert |
| replica-write | The presentation history could not be updated | L'historique de la présentation n'a pas pu être mis à jour |
| replica-unrelated | These two presentations do not share a common origin | Ces deux présentations n'ont pas d'origine commune |
| issue-unreadable-element | unreadable element, set aside | élément illisible, mis de côté |
| issue-unreadable-slide | unreadable slide, set aside | slide illisible, mise de côté |
| issue-unreadable-field | unreadable deck field { $field }, reset | champ { $field } du deck illisible, réinitialisé |
| notice-unreadable-parts | Some parts of this presentation could not be read and were set aside ({ $count }). | Des parties de cette présentation n'ont pas pu être lues et ont été mises de côté ({ $count }). |

Autres langues, dans l'ordre des clés ci-dessus :

**de** · Der Verlauf der Präsentation in dieser Datei ist beschädigt · Der Verlauf der Präsentation in dieser Datei ist zu groß zum Öffnen · Der Verlauf der Präsentation konnte nicht aktualisiert werden · Diese beiden Präsentationen haben keinen gemeinsamen Ursprung · unlesbares Element, beiseitegelegt · unlesbare Folie, beiseitegelegt · unlesbares Deck-Feld { $field }, zurückgesetzt · Teile dieser Präsentation konnten nicht gelesen werden und wurden beiseitegelegt ({ $count }).

**es** · El historial de la presentación en este archivo está dañado · El historial de la presentación en este archivo es demasiado grande para abrirlo · No se pudo actualizar el historial de la presentación · Estas dos presentaciones no tienen un origen común · elemento ilegible, apartado · diapositiva ilegible, apartada · campo { $field } del deck ilegible, restablecido · Algunas partes de esta presentación no se pudieron leer y se apartaron ({ $count }).

**it** · La cronologia della presentazione in questo file è danneggiata · La cronologia della presentazione in questo file è troppo grande per essere aperta · Impossibile aggiornare la cronologia della presentazione · Queste due presentazioni non hanno un'origine comune · elemento illeggibile, messo da parte · diapositiva illeggibile, messa da parte · campo { $field } del deck illeggibile, reimpostato · Alcune parti di questa presentazione non sono state lette e sono state messe da parte ({ $count }).

**pt** · O histórico da apresentação neste arquivo está danificado · O histórico da apresentação neste arquivo é grande demais para ser aberto · Não foi possível atualizar o histórico da apresentação · Estas duas apresentações não têm uma origem comum · elemento ilegível, deixado de lado · slide ilegível, deixado de lado · campo { $field } do deck ilegível, redefinido · Algumas partes desta apresentação não puderam ser lidas e foram deixadas de lado ({ $count }).

**nl** · De geschiedenis van de presentatie in dit bestand is beschadigd · De geschiedenis van de presentatie in dit bestand is te groot om te openen · De geschiedenis van de presentatie kon niet worden bijgewerkt · Deze twee presentaties hebben geen gemeenschappelijke oorsprong · onleesbaar element, apart gezet · onleesbare dia, apart gezet · onleesbaar deckveld { $field }, hersteld · Delen van deze presentatie konden niet worden gelezen en zijn apart gezet ({ $count }).

**pl** · Historia prezentacji w tym pliku jest uszkodzona · Historia prezentacji w tym pliku jest zbyt duża, aby ją otworzyć · Nie udało się zaktualizować historii prezentacji · Te dwie prezentacje nie mają wspólnego pochodzenia · nieczytelny element, odłożony · nieczytelny slajd, odłożony · nieczytelne pole { $field } prezentacji, zresetowane · Części tej prezentacji nie udało się odczytać i zostały odłożone ({ $count }).

**ru** · История презентации в этом файле повреждена · История презентации в этом файле слишком велика для открытия · Не удалось обновить историю презентации · У этих двух презентаций нет общего происхождения · нечитаемый элемент, отложен · нечитаемый слайд, отложен · нечитаемое поле { $field } презентации, сброшено · Некоторые части этой презентации не удалось прочитать, они отложены ({ $count }).

**uk** · Історія презентації в цьому файлі пошкоджена · Історія презентації в цьому файлі завелика, щоб її відкрити · Не вдалося оновити історію презентації · Ці дві презентації не мають спільного походження · нечитабельний елемент, відкладено · нечитабельний слайд, відкладено · нечитабельне поле { $field } презентації, скинуто · Деякі частини цієї презентації не вдалося прочитати, їх відкладено ({ $count }).

**tr** · Bu dosyadaki sunum geçmişi bozuk · Bu dosyadaki sunum geçmişi açılamayacak kadar büyük · Sunum geçmişi güncellenemedi · Bu iki sunumun ortak bir kökeni yok · okunamayan öğe, bir kenara ayrıldı · okunamayan slayt, bir kenara ayrıldı · okunamayan sunum alanı { $field }, sıfırlandı · Bu sunumun bazı bölümleri okunamadı ve bir kenara ayrıldı ({ $count }).

**vi** · Lịch sử bản trình bày trong tệp này bị hỏng · Lịch sử bản trình bày trong tệp này quá lớn để mở · Không thể cập nhật lịch sử bản trình bày · Hai bản trình bày này không có chung nguồn gốc · phần tử không đọc được, đã để riêng · trang chiếu không đọc được, đã để riêng · trường { $field } của bản trình bày không đọc được, đã đặt lại · Một số phần của bản trình bày này không đọc được và đã được để riêng ({ $count }).

**ja** · このファイル内のプレゼンテーション履歴が破損しています · このファイル内のプレゼンテーション履歴は大きすぎて開けません · プレゼンテーション履歴を更新できませんでした · この2つのプレゼンテーションには共通の起源がありません · 読み取れない要素を除外しました · 読み取れないスライドを除外しました · 読み取れないデッキ項目 { $field } をリセットしました · このプレゼンテーションの一部を読み取れなかったため除外しました（{ $count }）。

**ko** · 이 파일의 프레젠테이션 기록이 손상되었습니다 · 이 파일의 프레젠테이션 기록이 너무 커서 열 수 없습니다 · 프레젠테이션 기록을 업데이트할 수 없습니다 · 이 두 프레젠테이션은 공통된 출처가 없습니다 · 읽을 수 없는 요소를 따로 두었습니다 · 읽을 수 없는 슬라이드를 따로 두었습니다 · 읽을 수 없는 덱 항목 { $field }을(를) 초기화했습니다 · 이 프레젠테이션의 일부를 읽을 수 없어 따로 두었습니다({ $count }).

**zh-Hans** · 此文件中的演示文稿历史已损坏 · 此文件中的演示文稿历史过大，无法打开 · 无法更新演示文稿历史 · 这两个演示文稿没有共同的来源 · 无法读取的元素，已移出 · 无法读取的幻灯片，已移出 · 无法读取的演示文稿字段 { $field }，已重置 · 此演示文稿的部分内容无法读取，已移出（{ $count }）。

**zh-Hant** · 此檔案中的簡報歷程已損毀 · 此檔案中的簡報歷程過大，無法開啟 · 無法更新簡報歷程 · 這兩份簡報沒有共同的來源 · 無法讀取的元素，已移出 · 無法讀取的投影片，已移出 · 無法讀取的簡報欄位 { $field }，已重設 · 此簡報的部分內容無法讀取，已移出（{ $count }）。

**ar** · سجل العرض التقديمي في هذا الملف تالف · سجل العرض التقديمي في هذا الملف أكبر من أن يُفتح · تعذّر تحديث سجل العرض التقديمي · لا يشترك هذان العرضان التقديميان في أصل واحد · عنصر غير مقروء، نُحّي جانبًا · شريحة غير مقروءة، نُحّيت جانبًا · حقل العرض { $field } غير مقروء، أُعيد تعيينه · تعذّرت قراءة بعض أجزاء هذا العرض التقديمي فنُحّيت جانبًا ({ $count }).

**hi** · इस फ़ाइल में प्रस्तुति का इतिहास क्षतिग्रस्त है · इस फ़ाइल में प्रस्तुति का इतिहास खोलने के लिए बहुत बड़ा है · प्रस्तुति का इतिहास अपडेट नहीं हो सका · इन दोनों प्रस्तुतियों का कोई साझा मूल नहीं है · अपठनीय तत्व, अलग रखा गया · अपठनीय स्लाइड, अलग रखी गई · प्रस्तुति का अपठनीय फ़ील्ड { $field }, रीसेट किया गया · इस प्रस्तुति के कुछ हिस्से पढ़े नहीं जा सके और अलग रखे गए ({ $count })।

**bn** · এই ফাইলে উপস্থাপনার ইতিহাস ক্ষতিগ্রস্ত · এই ফাইলে উপস্থাপনার ইতিহাস খোলার জন্য খুব বড় · উপস্থাপনার ইতিহাস হালনাগাদ করা যায়নি · এই দুটি উপস্থাপনার কোনো সাধারণ উৎস নেই · অপাঠযোগ্য উপাদান, আলাদা রাখা হয়েছে · অপাঠযোগ্য স্লাইড, আলাদা রাখা হয়েছে · উপস্থাপনার অপাঠযোগ্য ক্ষেত্র { $field }, পুনরায় সেট করা হয়েছে · এই উপস্থাপনার কিছু অংশ পড়া যায়নি এবং আলাদা রাখা হয়েছে ({ $count })।

**th** · ประวัติงานนำเสนอในไฟล์นี้เสียหาย · ประวัติงานนำเสนอในไฟล์นี้มีขนาดใหญ่เกินกว่าจะเปิดได้ · ไม่สามารถอัปเดตประวัติงานนำเสนอได้ · งานนำเสนอทั้งสองนี้ไม่มีต้นกำเนิดร่วมกัน · องค์ประกอบที่อ่านไม่ได้ ถูกแยกไว้ · สไลด์ที่อ่านไม่ได้ ถูกแยกไว้ · ช่อง { $field } ของงานนำเสนอที่อ่านไม่ได้ ถูกรีเซ็ต · บางส่วนของงานนำเสนอนี้อ่านไม่ได้และถูกแยกไว้ ({ $count })
