# Sources Loro, plan 3 : mode historique — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** revoir n'importe quelle version passée d'un deck, en lecture seule, avec un curseur (un cran par commit, repères de session et de fusion), puis la restaurer par un nouveau commit annulable.

**Architecture :** le cœur liste les versions (tri topologique des *changes* Loro, Lamport puis pair) et lit l'état après chaque préfixe sur une copie du document (`HistoryPreview`) ; il restaure par `revert_to`, en gardant les noms d'auteurs. La session ajoute `Mode::History` sur le patron du mode modèles : le deck courant attend de côté, la version s'affiche. L'UI montre un bandeau et un curseur ; pendant le glissement, seul le dernier cran demandé est rendu.

**Tech Stack :** Rust 2024, `loro` 1.16, Snafu, Fluent (20 langues), React + Bun.

**Spec :** `docs/superpowers/specs/2026-09-27-sources-loro-design.md`, sections « Mode historique », « Erreurs », « Tests » (tâche t-188). Plans 1 et 2 fusionnés (`59f0d5e`, `8346fd8`).

## Écarts à la spec (décidés à l'écriture, reportés dans la spec en tâche 6)

1. **Coupe d'un préfixe, pas d'un change** : un cran montre l'état après **tous** les changes jusqu'à lui dans l'ordre (vecteur de versions du préfixe → `vv_to_frontiers`), pas seulement l'histoire causale du dernier ; sinon une branche concurrente plus ancienne dans l'ordre disparaîtrait d'un cran à l'autre.
2. **Pile : comparaison des decks, pas `doc.diff`** : les marques « supprimée / modifiée depuis » et le nombre de slides « ajoutées depuis » viennent d'une comparaison des slides par `id` entre la version et le deck courant (tous deux déjà lus). Même résultat, sans remonter des conteneurs Loro aux slides.
3. **Restaurer garde les auteurs** : `revert_to` ramènerait aussi la map `peers` à l'ancienne version et effacerait les noms des sessions plus récentes ; ils sont réinscrits dans le même commit.
4. **Deux erreurs de session** : `HistoryNotOpen` (spec) et `InHistory` (modifier, enregistrer ou fusionner pendant que l'historique est ouvert).
5. **Versions demandées à part** : la liste complète des versions part une fois (`Request::HistoryVersions`) ; le snapshot ne porte que le cran, sa version et les changements de slides.
6. **Mesure avant le curseur** (tâche 3) : si un cran coûte plus de 100 ms (rendu compris, p50), le rendu se fait au relâchement du curseur au lieu de pendant le glissement ; pas de vignette basse résolution (YAGNI tant que la mesure ne l'exige pas).

## Global Constraints

- Pas de `.unwrap()` / `.expect()` hors tests ; `Mutex` : `lock().unwrap_or_else(PoisonError::into_inner)`.
- Erreurs Snafu opaques + `kind()` + `Localize`, `backtrace: Backtrace` dans chaque variante, `display` en minuscules.
- Tout message utilisateur passe par Fluent ; une clé ajoutée l'est dans **les 20 locales** (sinon `every_locale_has_exactly_the_reference_keys` échoue) ; une clé `ui-*` doit être utilisée littéralement dans l'UI (sinon `ui/src/i18n-keys.test.ts` échoue). Traductions en annexe A.
- Tests côte à côte `foo_tests.rs` / `foo.test.ts`, nommés `<unité>_<scénario>_<résultat>`, `speculoos` / `bun:test`.
- Fichiers : cible 500 lignes, **limite 1000 (CI)** — `crates/oku-slides-session/src/lib.rs` (≈ 912) est allégé en tâche 1 avant d'accueillir le mode historique ; toute logique nouvelle va dans un module à part.
- Verrous : `crates/oku-slides-session/ARCHITECTURE.md` (inchangés par ce plan : tout se passe sous le verrou d'une session).
- Pas d'accès `std::fs` en production.
- Requêtes miroir : toute variante ajoutée à `Request` l'est aussi dans l'union TypeScript de `ui/src/api.ts`.
- Commits Conventional, en français, `(t-<sous-tâche>)`, **pas de `Co-Authored-By`**.
- Vérification complète : `just check` puis `just test`.

## Sous-tâches Oku

Créées sous t-188 ; chaque commit cite **sa** sous-tâche, marquée faite dès sa tâche terminée.

| Tâche | Sous-tâche |
|---|---|
| 1. Mode modèles hors de lib.rs | t-243 |
| 2. Versions, lecture d'une version, restauration | t-244 |
| 3. Mesure du coût d'un cran | t-245 |
| 4. Mode historique dans la session | t-246 |
| 5. Bandeau et curseur | t-247 |
| 6. Documentation et vérification | t-248 |

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-session/src/templates_mode.rs` | édition des ensembles de modèles, sortie de `lib.rs` |
| `crates/oku-slides-core/src/replica/history.rs` (+ `_tests.rs`) | `Version`, `HistoryPreview`, `SlideChanges`, `slide_changes`, restauration |
| `crates/oku-slides-core/src/replica/authors.rs` | lecture et réinscription de toute la map `peers` |
| `crates/oku-slides-core/src/sources.rs` | `SourceBundle::history`, `SourceBundle::restore` |
| `crates/oku-slides-cli/src/bench.rs` | mesure d'un cran (lecture + rendu) |
| `crates/oku-slides-session/src/history.rs` (+ `_tests.rs`) | `Mode::History`, ouvrir, avancer, restaurer, fermer |
| `crates/oku-slides-session/src/lib.rs`, `views.rs`, `error.rs` | requêtes, refus, snapshot, erreurs |
| `crates/oku-slides-session/tests/history.rs` | parcours bout en bout |
| `ui/src/history.ts` (+ `history.test.ts`) | repères, touches, envoi du dernier cran |
| `ui/src/HistoryBar.tsx` | bandeau et curseur |
| `ui/src/App.tsx`, `ui/src/SlidePile.tsx`, `ui/src/api.ts`, `ui/src/styles.css` | bouton, lecture seule, marques de la pile |

---

### Task 1 : Mode modèles hors de lib.rs

Pur déplacement : aucun comportement ne change, les tests existants restent verts sans modification.

**Files :**
- Create : `crates/oku-slides-session/src/templates_mode.rs`
- Modify : `crates/oku-slides-session/src/lib.rs`

**Interfaces :**
- Produces : `lib.rs` sous 800 lignes ; `impl Session` réparti entre `lib.rs` et `templates_mode.rs` (mêmes signatures publiques).

- [ ] **Step 1 : état de départ** — `wc -l crates/oku-slides-session/src/lib.rs` et `cargo nextest run -p oku-slides-session` : noter le nombre de lignes et de tests.
- [ ] **Step 2 : déplacer** — dans `templates_mode.rs` (doc de module : « Édition d'un ensemble de modèles : le deck attend de côté »), un bloc `impl Session { … }` qui reçoit, tels quels, `edit_templates`, `edited_set`, `save_templates`, `close_templates`, et les aides pures des aperçus de modèles (`styled_like`, `same_style`, `Previews`, `templates_view`, et ce qui ne sert qu'à elles). Les champs de `Session` et `Document` utilisés passent en `pub(crate)` si besoin. `mod templates_mode;` dans `lib.rs`. `Mode` reste dans `lib.rs`.
- [ ] **Step 3 : vérifier** — `cargo nextest run -p oku-slides-session` : même nombre de tests, tous verts ; `cargo clippy -p oku-slides-session --all-targets -- -D warnings` ; `wc -l` : `lib.rs` < 800, `templates_mode.rs` < 500.
- [ ] **Step 4 : commit**

```bash
git add crates/oku-slides-session
git commit -m "refactor(session) : l'édition des modèles dans son propre module (t-243)"
```

---

### Task 2 : Versions, lecture d'une version, restauration

**Files :**
- Create : `crates/oku-slides-core/src/replica/history.rs`, `replica/history_tests.rs`
- Modify : `crates/oku-slides-core/src/replica/mod.rs`, `replica/authors.rs`, `crates/oku-slides-core/src/sources.rs`, `crates/oku-slides-core/src/lib.rs`

**Interfaces :**
- Produces (réexportés à la racine du crate) :
  - `pub struct Version { pub author: Option<String>, pub timestamp: i64, pub label: Option<String>, pub session_start: bool, pub merge: bool }` (`Debug, Clone, PartialEq, Eq, serde::Serialize`).
  - `pub struct HistoryPreview` : `versions(&self) -> &[Version]`, `len(&self) -> usize`, `is_empty(&self) -> bool`, `deck_at(&self, index: usize) -> Result<DeckRead, ReplicaError>` (index borné au dernier).
  - `pub struct SlideChanges { pub added_since: Vec<String>, pub removed_since: Vec<String>, pub changed_since: Vec<String> }` (`Debug, Clone, PartialEq, Eq, Default, serde::Serialize`) et `pub fn slide_changes(version: &Deck, current: &Deck) -> SlideChanges`.
  - `DeckReplica::history(&self) -> HistoryPreview` ; `DeckReplica::restore(&mut self, preview: &HistoryPreview, index: usize, label: &str) -> Result<(), ReplicaError>`.
  - `SourceBundle::history(&self) -> HistoryPreview` ; `SourceBundle::restore(&mut self, preview: &HistoryPreview, index: usize, label: &str) -> Result<(), SourcesError>` (vue relue).

- [ ] **Step 1 : tests rouges** — `replica/history_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::{Deck, SourceBundle, slide_changes};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].title = Some(t.into());
    d
}

fn edited(labels: &[&str]) -> (Deck, SourceBundle) {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("Julie".into()));
    for l in labels {
        b.apply(titled(&deck, l), l).unwrap();
    }
    (deck, b)
}

#[test]
fn history_lists_creation_then_one_version_per_commit() {
    let (_, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.len()).is_equal_to(3);
    let labels: Vec<_> = h.versions().iter().map(|v| v.label.clone().unwrap_or_default()).collect();
    assert_that!(labels[1..].to_vec()).is_equal_to(vec!["Un".to_owned(), "Deux".to_owned()]);
    assert_that!(h.versions()[1].author.as_deref()).is_equal_to(Some("Julie"));
    assert_that!(h.versions()[1].session_start).is_true();
    assert_that!(h.versions()[2].session_start).is_false();
}

#[test]
fn deck_at_reads_the_state_after_each_version() {
    let (deck, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.deck_at(0).unwrap().0).is_equal_to(&deck);
    assert_that!(h.deck_at(1).unwrap().0.slides[2].title.as_deref()).is_equal_to(Some("Un"));
    assert_that!(h.deck_at(99).unwrap().0.slides[2].title.as_deref()).is_equal_to(Some("Deux"));
}

#[test]
fn history_marks_the_merge_and_keeps_concurrent_branches_in_each_prefix() {
    let deck = roadmap();
    let base = SourceBundle::new(deck.clone()).unwrap();
    let zip = base.to_zip().unwrap();
    let limits = crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28 };
    let (mut a, _) = SourceBundle::from_zip(&zip, limits).unwrap();
    let (mut b, _) = SourceBundle::from_zip(&zip, limits).unwrap();
    a.apply(titled(&deck, "de A"), "A").unwrap();
    let mut d = deck.clone();
    d.theme = Some("pro".into());
    b.apply(d, "B").unwrap();
    a.merge(b).unwrap();
    let mut after = a.deck().clone();
    after.slides[3].title = Some("après".into());
    a.apply(after, "Après").unwrap();
    let h = a.history();
    let last = h.len() - 1;
    assert_that!(h.versions()[last].merge).is_true();
    // Le cran juste avant la fusion contient les deux branches (préfixe, pas ancêtres).
    let before = h.deck_at(last - 1).unwrap().0;
    assert_that!(before.slides[2].title.as_deref()).is_equal_to(Some("de A"));
    assert_that!(before.theme.as_deref()).is_equal_to(Some("pro"));
}

#[test]
fn restore_brings_back_a_version_as_a_new_undoable_commit() {
    let (_, mut b) = edited(&["Un", "Deux"]);
    let h = b.history();
    b.restore(&h, 1, "Restaurer").unwrap();
    assert_that!(b.deck().slides[2].title.as_deref()).is_equal_to(Some("Un"));
    assert_that!(b.history().len()).is_equal_to(4);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
    b.undo().unwrap();
    assert_that!(b.deck().slides[2].title.as_deref()).is_equal_to(Some("Deux"));
}

#[test]
fn restore_keeps_authors_of_later_sessions() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let old = b.history();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Un").unwrap();
    let julie = b.replica().peer_id();
    b.restore(&old, 0, "Restaurer").unwrap();
    assert_that!(b.replica().author_of(julie)).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn slide_changes_compares_slides_by_id() {
    let current = roadmap();
    let mut version = current.clone();
    version.slides[2].title = Some("autre".into());
    let removed = version.slides.remove(4).id;
    let mut extra = version.slides[0].clone();
    extra.id = "s-disparue".into();
    version.slides.push(extra);
    let c = slide_changes(&version, &current);
    assert_that!(c.changed_since).is_equal_to(vec![current.slides[2].id.clone()]);
    assert_that!(c.removed_since).is_equal_to(vec!["s-disparue".to_owned()]);
    assert_that!(c.added_since).is_equal_to(vec![removed]);
}
```

(`SourceLimits` : reprendre le constructeur réel ; `SourceBundle::from_zip` rend `(bundle, issues)`.)

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/history/)'` → échec de compilation.

- [ ] **Step 3 : auteurs** — dans `authors.rs` :

```rust
/// Toute la map `peers` : clé (pair en décimal) → nom.
pub(super) fn all(doc: &LoroDoc) -> Vec<(String, String)> {
    let peers = doc.get_map(PEERS);
    peers
        .keys()
        .filter_map(|k| match peers.get(&k) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => Some((k.to_string(), s.to_string())),
            _ => None,
        })
        .collect()
}

/// Réinscrit les noms absents (après un retour en arrière du document).
pub(super) fn reinstate(doc: &LoroDoc, names: &[(String, String)]) -> Result<(), ReplicaInner> {
    let peers = doc.get_map(PEERS);
    for (key, name) in names {
        if peers.get(key).is_none() {
            peers.insert(key, name.as_str()).context(LoroSnafu)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4 : `history.rs`**

```rust
//! Historique du deck : ses versions dans l'ordre, la lecture d'une
//! version, et la comparaison d'une version avec le deck courant.
//!
//! L'ordre est un tri topologique des *changes* (Lamport, puis pair) : tout
//! préfixe est une coupe causale. Un cran montre l'état après **tous** les
//! changes du préfixe, branches concurrentes comprises.

use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};

use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{Frontiers, ID, LoroDoc, VersionVector};
use snafu::ResultExt;

use super::{DeckRead, authors, configure, read_deck};
use crate::error::{ImportPanickedSnafu, LoroSnafu, ReplicaError};
use crate::model::Deck;

/// Une version : l'état juste après un commit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Version {
    pub author: Option<String>,
    /// Secondes Unix (horloge de la machine de l'auteur : affichée, jamais triée).
    pub timestamp: i64,
    /// Libellé du commit (« Dictée : … ») ; `None` pour la création.
    pub label: Option<String>,
    /// Premier commit de ce pair dans l'ordre : début d'une session d'édition.
    pub session_start: bool,
    /// Commit qui réunit des branches concurrentes (après une fusion).
    pub merge: bool,
}

/// Les versions d'un deck, et une copie du document pour les lire.
pub struct HistoryPreview {
    /// Copie détachée ; jamais libérée si une lecture a paniqué (verrous empoisonnés).
    doc: Option<LoroDoc>,
    poisoned: Cell<bool>,
    versions: Vec<Version>,
    cuts: Vec<Frontiers>,
}

impl std::fmt::Debug for HistoryPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryPreview").field("versions", &self.versions.len()).finish()
    }
}

impl Drop for HistoryPreview {
    fn drop(&mut self) {
        // Libérer un document empoisonné par une panique ferait tomber l'app :
        // on le laisse fuir, comme `guarded` le fait à l'import.
        if self.poisoned.get()
            && let Some(doc) = self.doc.take()
        {
            std::mem::forget(doc);
        }
    }
}
```

`HistoryPreview` n'est pas `Sync` (`Cell`) mais doit rester `Send` (il vit dans la session derrière un `Mutex`) : ajouter un test `fn assert_send<T: Send>() {}` sur `HistoryPreview`.

```rust
pub(super) fn build(doc: &LoroDoc) -> HistoryPreview {
    let heads: Vec<ID> = doc.oplog_frontiers().iter().collect();
    let mut metas = Vec::new();
    // Un historique cohérent ne fait pas échouer le parcours ; en cas d'erreur,
    // la liste est simplement plus courte.
    let _ = doc.travel_change_ancestors(&heads, &mut |m| {
        metas.push(m);
        ControlFlow::Continue(())
    });
    metas.sort_by_key(|m| (m.lamport, m.id.peer));
    let (mut vv, mut seen) = (VersionVector::default(), HashSet::new());
    let (mut versions, mut cuts) = (Vec::with_capacity(metas.len()), Vec::with_capacity(metas.len()));
    for m in metas {
        let len = i32::try_from(m.len).unwrap_or(i32::MAX);
        vv.extend_to_include_last_id(ID::new(m.id.peer, m.id.counter + len - 1));
        cuts.push(doc.vv_to_frontiers(&vv));
        versions.push(Version {
            author: authors::author_of(doc, m.id.peer),
            timestamp: m.timestamp,
            label: m.message.map(|s| s.to_string()).filter(|s| !s.is_empty()),
            session_start: seen.insert(m.id.peer),
            merge: m.deps.len() > 1,
        });
    }
    let copy = doc.fork();
    configure(&copy);
    HistoryPreview { doc: Some(copy), poisoned: Cell::new(false), versions, cuts }
}

impl HistoryPreview {
    pub fn versions(&self) -> &[Version] {
        &self.versions
    }

    pub fn len(&self) -> usize {
        self.versions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    pub(super) fn cut(&self, index: usize) -> Option<&Frontiers> {
        self.cuts.get(index.min(self.cuts.len().saturating_sub(1)))
    }

    /// Le deck juste après la version `index` (bornée à la dernière).
    ///
    /// Une ancienne version touche des conteneurs jamais lus : la lecture
    /// est gardée comme l'import d'un snapshot.
    pub fn deck_at(&self, index: usize) -> Result<DeckRead, ReplicaError> {
        let (Some(doc), Some(cut)) = (self.doc.as_ref(), self.cut(index)) else {
            return Ok((Deck::empty(), Vec::new()));
        };
        doc.checkout(cut).context(LoroSnafu)?;
        match catch_unwind(AssertUnwindSafe(|| read_deck(doc))) {
            Ok(read) => Ok(read),
            Err(_) => {
                self.poisoned.set(true);
                Err(ImportPanickedSnafu.build().into())
            }
        }
    }
}

/// Les slides de `version` comparées à celles de `current`, par `id`.
pub fn slide_changes(version: &Deck, current: &Deck) -> SlideChanges {
    let now: BTreeMap<&str, _> = current.slides.iter().map(|s| (s.id.as_str(), s)).collect();
    let then: BTreeMap<&str, _> = version.slides.iter().map(|s| (s.id.as_str(), s)).collect();
    SlideChanges {
        added_since: current.slides.iter().filter(|s| !then.contains_key(s.id.as_str())).map(|s| s.id.clone()).collect(),
        removed_since: version.slides.iter().filter(|s| !now.contains_key(s.id.as_str())).map(|s| s.id.clone()).collect(),
        changed_since: version
            .slides
            .iter()
            .filter(|s| now.get(s.id.as_str()).is_some_and(|c| *c != *s))
            .map(|s| s.id.clone())
            .collect(),
    }
}

/// Ce qui distingue une version du deck courant, slide par slide.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct SlideChanges {
    /// Slides du deck courant absentes de la version.
    pub added_since: Vec<String>,
    /// Slides de la version absentes du deck courant.
    pub removed_since: Vec<String>,
    /// Slides présentes des deux côtés, mais différentes.
    pub changed_since: Vec<String>,
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
```

Adapter aux noms réels : `configure`, `read_deck`, `DeckRead` dans `mod.rs` (visibilité `pub(super)`), l'erreur de panique existante (`ImportPanicked`, mappée `Corrupted`), `ID::new` (ou `ID { peer, counter }`), `Deck::empty()` (si absent, construire le deck vide comme dans `read.rs`). `serde` est déjà une dépendance du crate.

- [ ] **Step 5 : `DeckReplica` et `SourceBundle`** — `mod.rs` : `mod history;`, puis

```rust
/// Les versions du deck, lisibles sur une copie.
pub fn history(&self) -> HistoryPreview {
    history::build(&self.doc)
}

/// Ramène le deck à la version `index` de `preview`, par un nouveau commit
/// annulable. Les noms d'auteurs plus récents sont gardés.
pub fn restore(&mut self, preview: &HistoryPreview, index: usize, label: &str) -> Result<(), ReplicaError> {
    let Some(cut) = preview.cut(index) else { return Ok(()) };
    let names = authors::all(&self.doc);
    self.claim_author()?;
    self.doc.revert_to(cut).context(LoroSnafu)?;
    authors::reinstate(&self.doc, &names)?;
    self.claim_author()?;
    self.commit(label);
    Ok(())
}
```

(`claim_author` avant **et** après `revert_to` : si ce pair n'était pas encore inscrit, `revert_to` pourrait retirer l'inscription du premier appel.) `sources.rs` : `history(&self) -> HistoryPreview` délègue ; `restore` délègue puis `self.deck = self.replica.deck().0` (même mappage d'erreur que `apply`). `lib.rs` : réexporter `Version`, `HistoryPreview`, `SlideChanges`, `slide_changes` et les ajouter au doc de tête.

- [ ] **Step 6 : vérifier** — `cargo nextest run -p oku-slides-core` et `cargo clippy -p oku-slides-core --all-targets -- -D warnings` → propres.

- [ ] **Step 7 : commit**

```bash
git add crates/oku-slides-core
git commit -m "feat(core) : versions du deck, lecture d'une version et restauration (t-244)"
```

---

### Task 3 : Mesure du coût d'un cran

**Files :**
- Modify : `crates/oku-slides-cli/src/bench.rs`, `crates/oku-slides-cli/src/main.rs` (si l'argument manque), `crates/oku-slides-i18n/src/locales/*/cli.ftl` (une clé, annexe A)

**Interfaces :**
- Consumes : `SourceBundle::history`, `HistoryPreview::deck_at`, le `Renderer` du banc existant.
- Produces : une ligne de banc `cli-bench-history` (moyenne et p50 d'un cran, lecture + rendu) ; la décision de l'écart 6, écrite dans le rapport et reprise en tâche 5.

- [ ] **Step 1 : banc** — dans `bench.rs`, après les mesures existantes : construire un `SourceBundle` depuis le deck du banc, appliquer 50 éditions successives (titre de la troisième slide, libellés « 1 » à « 50 »), prendre `history()`, puis pour chaque index de 1 à `len - 1` : `deck_at(i)`, rendu avec le `Renderer` gardé vivant ; mesurer chaque cran (`Instant`), afficher moyenne et p50 avec la clé Fluent `cli-bench-history` (arguments `mean`, `p50`, en ms), sur le modèle des lignes voisines.
- [ ] **Step 2 : lancer** — `cargo run --release -q -p oku-slides-cli -- bench fixtures/roadmap.json 20` ; noter la sortie.
- [ ] **Step 3 : décider** — p50 ≤ 100 ms : rendu pendant le glissement ; sinon : rendu au relâchement. Écrire la mesure et la décision dans le rapport de tâche.
- [ ] **Step 4 : vérifier et committer** — `cargo clippy -p oku-slides-cli --all-targets -- -D warnings`, `cargo nextest run -p oku-slides-i18n`.

```bash
git add crates/oku-slides-cli crates/oku-slides-i18n
git commit -m "perf(cli) : mesure du coût d'un cran d'historique (t-245)"
```

---

### Task 4 : Mode historique dans la session

**Files :**
- Create : `crates/oku-slides-session/src/history.rs`, `history_tests.rs`, `crates/oku-slides-session/tests/history.rs`
- Modify : `crates/oku-slides-session/src/lib.rs` (`Mode::History`, requêtes, refus, dispatch), `views.rs` (`Snapshot.history`), `error.rs` (`HistoryNotOpen`, `InHistory`), `resume.rs` (reprise d'un onglet ouvert sur l'historique), `crates/oku-slides-i18n/src/locales/*/session.ftl` (3 clés, annexe A)

**Interfaces :**
- Consumes : `SourceBundle::{history, restore, new, deck}`, `slide_changes`, `Version`, `SlideChanges`.
- Produces :
  - `Mode::History { stash: Box<Document>, history: HistoryMode }` ; `pub(crate) struct HistoryMode { preview: HistoryPreview, index: usize, version: Version, changes: SlideChanges }`.
  - `Request::HistoryOpen`, `Request::HistorySeek { index: usize }`, `Request::HistoryRestore`, `Request::HistoryClose`, `Request::HistoryVersions` (JSON : tableau de `Version`).
  - `Session::{history_open, history_seek(index), history_restore, history_close, history_versions}` dans `history.rs`.
  - `Snapshot.history: Option<HistoryView<'_>>` avec `HistoryView { index: usize, count: usize, version: &Version, changes: &SlideChanges }`.
  - `SessionErrorKind::{HistoryNotOpen, InHistory}`.

Règles :
- Ouvrir : seulement en `Mode::Deck` (sinon `EditingTemplates` en mode modèles, `InHistory` si déjà ouvert). Le document courant va dans `stash` ; un document d'aperçu est construit sur `deck_at(dernier)`, avec les assets et polices **déplacés** (`std::mem::take`) depuis `stash` (aucune copie) ; rendu.
- Avancer : borne l'index, lit `deck_at`, reconstruit l'aperçu (assets et polices déplacés de l'ancien aperçu vers le nouveau), calcule `slide_changes(version, stash.deck())`, rend.
- Restaurer : ramène assets et polices dans `stash`, `stash.sources.restore(&preview, index, label)` (libellé `history-restore-label`, date locale « AAAA-MM-JJ HH:MM »), `dirty = true`, sort du mode, rend.
- Fermer : ramène assets et polices, `self.doc = *stash`, rend.
- Pendant le mode : `Apply`, `Undo`, `Redo`, `Save`, `Merge`, `ChartKind`, `EditTemplates`, `SaveTemplates`, `CloseTemplates`, `DeleteTemplateSet` → `InHistory`. `Seek`, `Restore`, `Close`, `Versions` hors du mode → `HistoryNotOpen`.
- `tab_info`, `unsaved`, `user_deck`, `document_name` voient `stash` (comme en mode modèles). La reprise écrit `stash` comme deck et un mode `Deck` : un onglet rouvert ne revient pas dans l'historique.
- Aucun chemin d'erreur ne perd les assets ni les polices : chaque déplacement est rendu avant de propager une erreur.

- [ ] **Step 1 : tests rouges** — `tests/history.rs` (helpers de `tests/common/mod.rs` : `session`, `req`, `snapshot_json`, `fixture_path`, `set_theme`) :

```rust
mod common;

use common::*;
use serde_json::json;
use speculoos::prelude::*;

fn edited_session() -> oku_slides_session::Session {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.handle(set_theme("pro")).unwrap();
    s.handle(set_theme("papier")).unwrap();
    s
}

#[test]
fn history_open_shows_the_last_version_read_only() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["history"]["count"].as_u64().unwrap()).is_greater_than(2);
    assert_that!(snap["history"]["index"]).is_equal_to(json!(snap["history"]["count"].as_u64().unwrap() - 1));
    let err = s.handle(set_theme("pro")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::InHistory);
}

#[test]
fn history_seek_shows_an_earlier_version() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = snapshot_json(&mut s)["history"]["count"].as_u64().unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": count - 2 }))).unwrap();
    assert_that!(snapshot_json(&mut s)["theme"]).is_equal_to(json!("pro"));
}

#[test]
fn history_close_returns_to_the_current_deck_untouched() {
    let mut s = edited_session();
    let before = snapshot_json(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    let after = snapshot_json(&mut s);
    assert_that!(after["theme"]).is_equal_to(&before["theme"]);
    assert_that!(after["history"].is_null()).is_true();
    assert_that!(after["dirty"]).is_equal_to(&before["dirty"]);
}

#[test]
fn history_restore_is_a_new_undoable_commit() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = snapshot_json(&mut s)["history"]["count"].as_u64().unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": count - 2 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_restore" }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["history"].is_null()).is_true();
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
    assert_that!(snap["can_undo"]).is_equal_to(json!(true));
    s.handle(req(json!({ "cmd": "undo" }))).unwrap();
    assert_that!(snapshot_json(&mut s)["theme"]).is_equal_to(json!("papier"));
}

#[test]
fn history_requests_outside_the_mode_return_history_not_open() {
    let mut s = edited_session();
    let err = s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::HistoryNotOpen);
}

#[test]
fn history_versions_lists_labels_in_order() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req(json!({ "cmd": "history_versions" }))).unwrap()).unwrap();
    let n = v.as_array().unwrap().len();
    assert_that!(v[n - 1]["label"].as_str().unwrap_or_default()).is_not_empty();
}

#[test]
fn history_keeps_assets_across_open_seek_close() {
    let mut s = edited_session();
    let assets = s.sources_for_tests().assets.len();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    assert_that!(s.sources_for_tests().assets.len()).is_equal_to(assets);
}
```

Dans `history_tests.rs` : le libellé de restauration (format de date), et le refus de `history_open` en mode modèles. Adapter la forme exacte du snapshot (`theme`, `can_undo`, `dirty`) et les commandes serde (`history_open`…) à ce que produit `Request`.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-session --test history` → échec de compilation.

- [ ] **Step 3 : implémentation** — `history.rs` : `HistoryMode`, les cinq méthodes selon les règles ci-dessus, et une aide `fn take_blobs(from: &mut Document) -> (assets, fonts)` / `fn put_blobs(to: &mut Document, blobs)` utilisée par chaque transition. Date du libellé : `time::OffsetDateTime::now_utc().to_offset(time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC))` formaté `[year]-[month]-[day] [hour]:[minute]` (la version restaurée : utiliser son `timestamp`). `lib.rs` : variante de `Mode`, dispatch des requêtes, refus en tête de `handle` (même endroit que le refus existant du mode modèles), `user_deck`/`editing`/`unsaved`/`tab_info`/`document_name`/`panel` complétés pour `Mode::History`. `views.rs` : `HistoryView` et `Snapshot.history`. `error.rs` : les deux variantes, clés `session-history-not-open` et `session-in-history`. `resume.rs` : en `Mode::History`, écrire `stash` comme document du deck et le mode `Deck`.

- [ ] **Step 4 : Fluent** — `session-history-not-open`, `session-in-history`, `history-restore-label` (`$date`) dans `session.ftl`, 20 locales (annexe A).

- [ ] **Step 5 : vérifier** — `cargo nextest run --workspace --exclude oku-slides-app`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build -p oku-slides-app` ; `wc -l crates/oku-slides-session/src/lib.rs` < 1000.

- [ ] **Step 6 : commit**

```bash
git add crates
git commit -m "feat(session) : mode historique, lire, parcourir et restaurer une version (t-246)"
```

---

### Task 5 : Bandeau et curseur

**Files :**
- Create : `ui/src/history.ts`, `ui/src/history.test.ts`, `ui/src/HistoryBar.tsx`
- Modify : `ui/src/api.ts`, `ui/src/App.tsx`, `ui/src/SlidePile.tsx`, `ui/src/styles.css`, `crates/oku-slides-i18n/src/locales/*/ui.ftl` (9 clés, annexe A)

**Interfaces :**
- Consumes : `Request::{HistoryOpen, HistorySeek, HistoryRestore, HistoryClose, HistoryVersions}`, `Snapshot.history`.
- Produces :
  - TS : `Snapshot.history: HistoryView | null`, `HistoryView { index: number; count: number; version: Version; changes: SlideChanges }`, `Version`, `SlideChanges` ; `tabApi(tab).{historyOpen, historySeek(index), historyRestore, historyClose, historyVersions}`.
  - `history.ts` : `markers(versions: Version[]): { index: number; kind: "session" | "merge" }[]`, `step(index: number, versions: Version[], key: "ArrowLeft" | "ArrowRight", shift: boolean): number`, `class Seeker` (dernier cran gagnant, sur `Latest` de `latest.ts`), constante `RENDER_WHILE_DRAGGING` (décision de la tâche 3).

- [ ] **Step 1 : tests rouges** — `ui/src/history.test.ts` :

```ts
import { describe, expect, test } from "bun:test";

import { markers, Seeker, step } from "./history";

const v = (peer: string, merge = false, start = false) =>
  ({ author: peer, timestamp: 0, label: peer, session_start: start, merge }) as never;

const versions = [v("A", false, true), v("A"), v("B", false, true), v("B"), v("A", true, false), v("C", false, true)];

describe("markers", () => {
  test("markers flags each session start and each merge", () => {
    expect(markers(versions)).toEqual([
      { index: 0, kind: "session" },
      { index: 2, kind: "session" },
      { index: 4, kind: "merge" },
      { index: 5, kind: "session" },
    ]);
  });
});

describe("step", () => {
  test("step with an arrow moves one version and stays in bounds", () => {
    expect(step(0, versions, "ArrowLeft", false)).toBe(0);
    expect(step(1, versions, "ArrowRight", false)).toBe(2);
    expect(step(5, versions, "ArrowRight", false)).toBe(5);
  });

  test("step with shift jumps to the previous or next session start", () => {
    expect(step(3, versions, "ArrowLeft", true)).toBe(2);
    expect(step(2, versions, "ArrowLeft", true)).toBe(0);
    expect(step(1, versions, "ArrowRight", true)).toBe(2);
    expect(step(5, versions, "ArrowRight", true)).toBe(5);
  });
});

describe("Seeker", () => {
  test("Seeker keeps only the answer to the last requested version", async () => {
    const shown: number[] = [];
    const resolvers: ((n: number) => void)[] = [];
    const seeker = new Seeker(
      (i: number) => new Promise<number>((r) => resolvers.push(() => r(i))),
      (s: number) => shown.push(s),
    );
    const first = seeker.go(1);
    const second = seeker.go(2);
    resolvers[1](2);
    await second;
    resolvers[0](1);
    await first;
    expect(shown).toEqual([2]);
  });
});
```

(`Seeker` : `constructor(seek: (index: number) => Promise<T>, show: (answer: T) => void)`, `go(index): Promise<void>` ; les réponses plus anciennes que la dernière acceptée sont ignorées — utiliser `Latest`.)

- [ ] **Step 2 : vérifier l'échec** — `cd ui && bun test history` → échec (module absent).

- [ ] **Step 3 : `history.ts`**

```ts
// Curseur de l'historique : repères, déplacements au clavier, et envoi du
// dernier cran demandé (les réponses dépassées sont ignorées).

import type { Version } from "./api";
import { Latest } from "./latest";

/** Rendu pendant le glissement (sinon au relâchement) : mesure de la tâche 3. */
export const RENDER_WHILE_DRAGGING = true;

export function markers(versions: Version[]): { index: number; kind: "session" | "merge" }[] {
  return versions.flatMap((v, index) =>
    v.merge ? [{ index, kind: "merge" as const }] : v.session_start ? [{ index, kind: "session" as const }] : [],
  );
}

export function step(index: number, versions: Version[], key: "ArrowLeft" | "ArrowRight", shift: boolean): number {
  const last = versions.length - 1;
  if (!shift) return Math.max(0, Math.min(last, index + (key === "ArrowRight" ? 1 : -1)));
  if (key === "ArrowRight") {
    for (let i = index + 1; i <= last; i++) if (versions[i].session_start) return i;
    return last;
  }
  for (let i = index - 1; i >= 0; i--) if (versions[i].session_start) return i;
  return 0;
}

export class Seeker<T> {
  private latest = new Latest();

  constructor(
    private seek: (index: number) => Promise<T>,
    private show: (answer: T) => void,
  ) {}

  async go(index: number): Promise<void> {
    const n = this.latest.begin();
    const answer = await this.seek(index);
    if (this.latest.accept(n)) this.show(answer);
  }
}
```

(Vérifier la sémantique exacte de `Latest.begin` / `accept` dans `latest.ts` ; `step` avec shift vers la gauche depuis un début de session va au début précédent — le test le fixe. Mettre `RENDER_WHILE_DRAGGING` à la valeur décidée en tâche 3, lue dans son rapport.)

- [ ] **Step 4 : API** — `api.ts` : types `Version`, `SlideChanges`, `HistoryView` ; `Snapshot.history: HistoryView | null` ; union `Request` : `{ cmd: "history_open" } | { cmd: "history_seek"; index: number } | { cmd: "history_restore" } | { cmd: "history_close" } | { cmd: "history_versions" }` ; méthodes de `tabApi` correspondantes.

- [ ] **Step 5 : `HistoryBar.tsx`** — composant affiché quand `snap.history` : bandeau « Version du {date} · {auteur ou ui-history-anonymous} · {libellé} » (date : `Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" })` sur `timestamp * 1000`, locale de `i18n.ts`), nombre de slides ajoutées depuis (`ui-history-added-since`, masqué à 0), boutons `ui-history-restore` et `ui-history-back` ; un `<input type="range" aria-label={t("ui-history-slider")}>` de 0 à `count - 1`, et les repères de `markers` posés au-dessus (session : trait fin ; fusion : losange). `onInput` : si `RENDER_WHILE_DRAGGING`, `seeker.go(value)` ; sinon, mettre à jour seulement l'étiquette ; `onChange` (relâchement) : `seeker.go(value)`. Les versions viennent de `historyVersions()` une fois, à l'ouverture. Couleurs et rayons : variables de `styles.css` (`--ink`, `--accent`, `--rule`, `--radius-*`).

- [ ] **Step 6 : App et pile** — `App.tsx` : bouton `ui-history` dans la barre (masqué en mode modèles), qui appelle `historyOpen` ; quand `snap.history` : `HistoryBar` à la place de la légende, scène en lecture seule (pas de sélection ni d'édition : réutiliser la garde existante du mode modèles ou une garde `snap.history`), touches ←/→ et ⇧←/⇧→ vers `step` puis `seeker.go` (dans le gestionnaire clavier existant, **avant** les branches d'édition qui utilisent aussi les flèches). `SlidePile.tsx` : prop optionnelle `marks?: Record<string, "removed" | "changed">` (depuis `changes`), une pastille par vignette (`ui-history-removed-since`, `ui-history-changed-since` en `title`).

- [ ] **Step 7 : Fluent** — les 9 clés `ui-history*` dans `ui.ftl`, 20 locales (annexe A).

- [ ] **Step 8 : vérifier** — `cd ui && bun test && bun run typecheck` ; `cargo nextest run -p oku-slides-i18n`.

- [ ] **Step 9 : commit**

```bash
git add ui/src crates/oku-slides-i18n
git commit -m "feat(ui) : historique du deck, bandeau et curseur de versions (t-247)"
```

---

### Task 6 : Documentation et vérification

**Files :**
- Modify : spec (« Mode historique », « Erreurs », « Tests », « Limites connues » : écarts 1 à 6 et mesure de la tâche 3), `docs/adr/0002-sources-du-deck-en-crdt-loro.md` (mode historique livré), `crates/oku-slides-session/ARCHITECTURE.md` et `crates/oku-slides-core/ARCHITECTURE.md` (≤ 50 lignes chacun)

- [ ] **Step 1 : spec** — les six écarts ; la mesure (moyenne, p50) et la décision ; limites : un pair qui annule sa première édition apparaît anonyme dans l'historique ; l'heure affichée est celle de la machine de l'auteur.
- [ ] **Step 2 : ARCHITECTURE et ADR** — `history.rs` (session) et `replica/history.rs` (cœur) ; `templates_mode.rs`.
- [ ] **Step 3 : consolidation (G10)** — `grep -rn "fn slide_changes\|fn markers\|fn step\b\|class Seeker" crates ui/src` : une définition chacune ; `wc -l` des fichiers touchés < 1000.
- [ ] **Step 4 : vérification complète** — `just check` puis `just test`.
- [ ] **Step 5 : commit**

```bash
git add docs crates
git commit -m "docs(core) : mode historique dans la spec, l'ADR et l'architecture (t-248)"
```

---

## Annexe A : clés Fluent

`session.ftl` : `session-history-not-open`, `session-in-history`, `history-restore-label` (`$date`). `ui.ftl` : `ui-history`, `ui-history-version` (`$date`), `ui-history-restore`, `ui-history-back`, `ui-history-anonymous`, `ui-history-slider`, `ui-history-removed-since`, `ui-history-changed-since`, `ui-history-added-since` (`$count`). `cli.ftl` (tâche 3) : `cli-bench-history` (`$mean`, `$p50`) — anglais « history step: { $mean } ms on average, p50 { $p50 } ms », français « cran d'historique : { $mean } ms en moyenne, p50 { $p50 } ms », les 18 autres langues sur le modèle des lignes `cli-bench-*` voisines de chaque `cli.ftl`.

Ordre dans chaque ligne : session-history-not-open · session-in-history · history-restore-label · ui-history · ui-history-version · ui-history-restore · ui-history-back · ui-history-anonymous · ui-history-slider · ui-history-removed-since · ui-history-changed-since · ui-history-added-since.

**en** · The history is not open · Close the history before changing the presentation · Restore the version of { $date } · History · Version of { $date } · Restore · Back · Anonymous · Versions of the presentation · deleted since · changed since · slides added since: { $count }

**fr** · L'historique n'est pas ouvert · Fermez l'historique avant de modifier la présentation · Restaurer la version du { $date } · Historique · Version du { $date } · Restaurer · Revenir · Anonyme · Versions de la présentation · supprimée depuis · modifiée depuis · slides ajoutées depuis : { $count }

**de** · Der Verlauf ist nicht geöffnet · Schließen Sie den Verlauf, bevor Sie die Präsentation ändern · Version vom { $date } wiederherstellen · Verlauf · Version vom { $date } · Wiederherstellen · Zurück · Anonym · Versionen der Präsentation · seitdem gelöscht · seitdem geändert · seitdem hinzugefügte Folien: { $count }

**es** · El historial no está abierto · Cierre el historial antes de modificar la presentación · Restaurar la versión del { $date } · Historial · Versión del { $date } · Restaurar · Volver · Anónimo · Versiones de la presentación · eliminada desde entonces · modificada desde entonces · diapositivas añadidas desde entonces: { $count }

**it** · La cronologia non è aperta · Chiudi la cronologia prima di modificare la presentazione · Ripristina la versione del { $date } · Cronologia · Versione del { $date } · Ripristina · Indietro · Anonimo · Versioni della presentazione · eliminata da allora · modificata da allora · diapositive aggiunte da allora: { $count }

**pt** · O histórico não está aberto · Feche o histórico antes de alterar a apresentação · Restaurar a versão de { $date } · Histórico · Versão de { $date } · Restaurar · Voltar · Anônimo · Versões da apresentação · excluído desde então · alterado desde então · slides adicionados desde então: { $count }

**nl** · De geschiedenis is niet geopend · Sluit de geschiedenis voordat u de presentatie wijzigt · Versie van { $date } herstellen · Geschiedenis · Versie van { $date } · Herstellen · Terug · Anoniem · Versies van de presentatie · sindsdien verwijderd · sindsdien gewijzigd · sindsdien toegevoegde dia's: { $count }

**pl** · Historia nie jest otwarta · Zamknij historię przed zmianą prezentacji · Przywróć wersję z { $date } · Historia · Wersja z { $date } · Przywróć · Wróć · Anonim · Wersje prezentacji · usunięty od tego czasu · zmieniony od tego czasu · slajdy dodane od tego czasu: { $count }

**ru** · История не открыта · Закройте историю, прежде чем изменять презентацию · Восстановить версию от { $date } · История · Версия от { $date } · Восстановить · Назад · Аноним · Версии презентации · удалён с тех пор · изменён с тех пор · слайдов добавлено с тех пор: { $count }

**uk** · Історію не відкрито · Закрийте історію, перш ніж змінювати презентацію · Відновити версію від { $date } · Історія · Версія від { $date } · Відновити · Назад · Анонім · Версії презентації · видалено відтоді · змінено відтоді · слайдів додано відтоді: { $count }

**tr** · Geçmiş açık değil · Sunumu değiştirmeden önce geçmişi kapatın · { $date } sürümünü geri yükle · Geçmiş · { $date } sürümü · Geri yükle · Geri dön · Anonim · Sunumun sürümleri · o zamandan beri silindi · o zamandan beri değişti · o zamandan beri eklenen slaytlar: { $count }

**vi** · Lịch sử chưa được mở · Hãy đóng lịch sử trước khi thay đổi bản trình bày · Khôi phục phiên bản lúc { $date } · Lịch sử · Phiên bản lúc { $date } · Khôi phục · Quay lại · Ẩn danh · Các phiên bản của bản trình bày · đã xóa từ đó · đã thay đổi từ đó · số trang chiếu thêm từ đó: { $count }

**ja** · 履歴が開いていません · プレゼンテーションを変更する前に履歴を閉じてください · { $date } のバージョンを復元 · 履歴 · { $date } のバージョン · 復元 · 戻る · 匿名 · プレゼンテーションのバージョン · その後削除 · その後変更 · その後追加されたスライド: { $count }

**ko** · 기록이 열려 있지 않습니다 · 프레젠테이션을 변경하기 전에 기록을 닫으세요 · { $date } 버전 복원 · 기록 · { $date } 버전 · 복원 · 돌아가기 · 익명 · 프레젠테이션 버전 · 이후 삭제됨 · 이후 변경됨 · 이후 추가된 슬라이드: { $count }

**zh-Hans** · 历史记录未打开 · 请先关闭历史记录再修改演示文稿 · 恢复 { $date } 的版本 · 历史记录 · { $date } 的版本 · 恢复 · 返回 · 匿名 · 演示文稿的版本 · 此后已删除 · 此后已修改 · 此后新增的幻灯片：{ $count }

**zh-Hant** · 歷程記錄未開啟 · 請先關閉歷程記錄再修改簡報 · 還原 { $date } 的版本 · 歷程記錄 · { $date } 的版本 · 還原 · 返回 · 匿名 · 簡報的版本 · 此後已刪除 · 此後已修改 · 此後新增的投影片：{ $count }

**ar** · السجل غير مفتوح · أغلق السجل قبل تعديل العرض التقديمي · استعادة نسخة { $date } · السجل · نسخة { $date } · استعادة · رجوع · مجهول · نسخ العرض التقديمي · حُذفت منذ ذلك الحين · عُدّلت منذ ذلك الحين · الشرائح المضافة منذ ذلك الحين: { $count }

**hi** · इतिहास खुला नहीं है · प्रस्तुति बदलने से पहले इतिहास बंद करें · { $date } का संस्करण पुनर्स्थापित करें · इतिहास · { $date } का संस्करण · पुनर्स्थापित करें · वापस · अनाम · प्रस्तुति के संस्करण · तब से हटाई गई · तब से बदली गई · तब से जोड़ी गई स्लाइडें: { $count }

**bn** · ইতিহাস খোলা নেই · উপস্থাপনা পরিবর্তনের আগে ইতিহাস বন্ধ করুন · { $date }-এর সংস্করণ পুনরুদ্ধার করুন · ইতিহাস · { $date }-এর সংস্করণ · পুনরুদ্ধার করুন · ফিরে যান · বেনামী · উপস্থাপনার সংস্করণ · তারপর থেকে মুছে ফেলা হয়েছে · তারপর থেকে পরিবর্তিত · তারপর থেকে যোগ করা স্লাইড: { $count }

**th** · ไม่ได้เปิดประวัติ · ปิดประวัติก่อนแก้ไขงานนำเสนอ · กู้คืนเวอร์ชันของ { $date } · ประวัติ · เวอร์ชันของ { $date } · กู้คืน · กลับ · ไม่ระบุชื่อ · เวอร์ชันของงานนำเสนอ · ถูกลบตั้งแต่นั้น · ถูกแก้ไขตั้งแต่นั้น · สไลด์ที่เพิ่มตั้งแต่นั้น: { $count }
