# Sources Loro, plan 2 : fusion côté session — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** fusionner deux PDF d'un même deck depuis l'app : commande *Fusionner avec un PDF…*, fusion automatique à l'enregistrement quand le fichier a changé sur disque, proposition de fusion à l'ouverture d'un deck déjà ouvert ; avis « N modifications de Julie ».

**Architecture :** le cœur apprend à nommer les auteurs (map `peers`), à faire d'un commit un *change* Loro portant son libellé, et à fusionner un `SourceBundle` entier (replica + union des assets et polices) en rendant un `MergeReport`. La session expose `Request::Merge`, vérifie le disque avant d'écrire (`Save { overwrite }`, indicateur `disk_replaced` du snapshot), et l'espace de travail répond à une requête en lecture seule `SameDeck` grâce au `deck_uid` gardé dans `TabInfo`. L'UI pose les deux questions sur le modèle de `ui/src/close.ts`.

**Tech Stack :** Rust 2024, `loro` 1.16, Snafu, Fluent (20 langues), React + Bun, `@tauri-apps/plugin-dialog`.

**Spec :** `docs/superpowers/specs/2026-09-27-sources-loro-design.md`, sections « Schéma » (`peers`), « Fusion », « Erreurs » (tâche t-188). Plan 1 exécuté : `docs/superpowers/plans/2026-09-27-sources-loro-fondation.md` (squash `59f0d5e`).

## Écarts à la spec (décidés à l'écriture de ce plan, à reporter dans la spec en tâche 7)

1. **`DiskReplaced` n'est pas une erreur** : les erreurs n'arrivent à l'UI que sous forme de texte traduit (`state.refuse` → `localize`), l'UI ne pourrait pas les distinguer. `Save` rend le snapshot avec `disk_replaced: true` (une fois, comme `notice`) ; l'UI propose *Écraser* (`Save { overwrite: true }`) ou *Enregistrer sous…*.
2. **Déclencheur « à l'ouverture » par une requête en lecture seule** : sous Tauri, une requête d'espace de travail qui change la structure rend la vue synchronisée, pas sa réponse (`src-tauri/src/commands.rs:77-95`). L'UI demande donc d'abord `SameDeck { path }` (sans effet), pose la question, puis fusionne (`Request::Merge` sur l'onglet trouvé) ou ouvre à part (`OpenTab`).
3. **Fichiers remis par l'OS** (Finder, glisser-déposer, ligne de commande : `open_files`) : l'hôte ne peut pas poser de question ; le fichier s'ouvre à part, avec l'avis `notice-same-deck-open` qui désigne l'onglet déjà ouvert.
4. **Un commit = un *change* Loro, avec son libellé comme message** (`set_change_merge_interval(-1)`, `set_next_commit_message`) : sans cela Loro fusionne les commits locaux d'une fenêtre de 1000 s en un seul *change*, et « N modifications » (comme le curseur du plan 3) ne voudrait rien dire.
5. **`MergeUnrelated`** reprend le message existant `replica-unrelated` (pas de nouvelle clé).

## Global Constraints

- Pas de `.unwrap()` / `.expect()` hors tests ; `Mutex` : `lock().unwrap_or_else(PoisonError::into_inner)`.
- Erreurs Snafu opaques + `kind()` + `Localize`, `backtrace: Backtrace` dans chaque variante, messages `display` en minuscules.
- Tout message utilisateur passe par Fluent ; une clé ajoutée l'est dans **les 20 locales** (`crates/oku-slides-i18n/src/locales/*/`), sinon `every_locale_has_exactly_the_reference_keys` échoue. Traductions en annexe A.
- Tests côte à côte `foo_tests.rs`, nommés `<unité>_<scénario>_<résultat>`, assertions `speculoos` ; UI : `foo.test.ts` à côté, `bun test`.
- Fichiers : cible 500 lignes, limite 1000 (`crates/oku-slides-session/src/lib.rs` est déjà gros : toute logique nouvelle va dans un module à part).
- Verrous de l'espace de travail (`crates/oku-slides-session/ARCHITECTURE.md`) : l'espace de travail, puis une session, jamais l'inverse ; jamais de lecture de fichier sous le verrou de l'espace de travail.
- Pas d'accès `std::fs` en production : `Storage` (`oku_slides_platform::storage`).
- Requêtes miroir : toute variante ajoutée à `Request` / `WorkspaceRequest` l'est aussi dans les unions TypeScript de `ui/src/api.ts`.
- Commits Conventional, en français, `(t-<sous-tâche>)`, **pas de `Co-Authored-By`**.
- Vérification complète : `just check` puis `just test`.

## Sous-tâches Oku

Créées sous t-188 ; chaque commit cite **sa** sous-tâche, marquée faite dès sa tâche terminée (`oku cli task done --project slider --id t-N --result …`).

| Tâche | Sous-tâche |
|---|---|
| 1. Un commit, un change Loro ; auteurs dans peers | t-235 |
| 2. Fusion d'un SourceBundle et rapport | t-236 |
| 3. Nom d'auteur dans les réglages | t-237 |
| 4. Commande Fusionner avec un PDF | t-238 |
| 5. Enregistrer : fusion ou fichier remplacé | t-239 |
| 6. Ouvrir un deck déjà ouvert | t-240 |
| 7. Documentation et vérification | t-241 |

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/replica/mod.rs` | message de commit, pas de fusion de *changes*, auteur, `merge` → `MergeReport` |
| `crates/oku-slides-core/src/replica/authors.rs` (+ `_tests.rs`) | map `peers`, rapport de fusion (changes importés, auteurs) |
| `crates/oku-slides-core/src/sources.rs` | `SourceBundle::merge`, `set_author` |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `[author] name`, repli sur l'utilisateur système |
| `crates/oku-slides-session/src/merge.rs` (+ `_tests.rs`) | fusion d'un fichier dans la session, avis, vérification du disque avant écriture |
| `crates/oku-slides-session/src/lib.rs` | `Request::Merge`, `Save { overwrite }`, `disk_replaced`, auteur des documents |
| `crates/oku-slides-session/src/error.rs` | `MergeUnrelated` |
| `crates/oku-slides-session/src/views.rs` | `Snapshot.disk_replaced`, `TabInfo.deck_uid` |
| `crates/oku-slides-session/src/workspace.rs`, `workspace_resume.rs` | `WorkspaceRequest::SameDeck`, avis pour `open_files` |
| `src-tauri/src/commands.rs` | `classify` : `SameDeck` sans effet |
| `ui/src/api.ts` | unions et fonctions `merge`, `save(path, overwrite)`, `sameDeck` |
| `ui/src/merge.ts` (+ `merge.test.ts`) | questions « même deck » et « fichier remplacé », enregistrement gardé |
| `ui/src/App.tsx`, `ui/src/Workspace.tsx`, `ui/src/close.ts` | bouton *Fusionner…*, ouverture, enregistrement |

---

### Task 1 : Un commit, un change Loro ; auteurs dans peers

**Files :**
- Create : `crates/oku-slides-core/src/replica/authors.rs`, `replica/authors_tests.rs`
- Modify : `crates/oku-slides-core/src/replica/mod.rs`, `crates/oku-slides-core/src/sources.rs`

**Interfaces :**
- Produces :
  - `DeckReplica::set_author(&mut self, name: Option<String>)` : nom de l'auteur de ce pair (blanc = aucun).
  - `DeckReplica::author_of(&self, peer: u64) -> Option<String>`.
  - `pub(crate) fn claim_author(&self) -> Result<(), ReplicaError>` : écrit `peers[<peer_id>] = nom` si absent (sans commit).
  - `DeckReplica::commit(&mut self, label)` : le libellé devient aussi le message du *change* Loro.
  - `DeckReplica::len_changes(&self) -> usize`.
  - `SourceBundle::set_author(&mut self, name: Option<String>)`.
  - `SourceBundle::apply` appelle `claim_author` avant `reconcile` (la première édition de ce pair l'inscrit, dans le même commit).
  - Racine Loro : map `peers` (`{ "<peer id décimal>": "<nom>" }`).

- [ ] **Step 1 : tests rouges** — `replica/authors_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::{Deck, DeckReplica, SourceBundle};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].title = Some(t.into());
    d
}

#[test]
fn each_commit_is_its_own_change() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let before = r.len_changes();
    for t in ["Un", "Deux", "Trois"] {
        r.reconcile(&titled(&deck, t)).unwrap();
        r.commit(t);
    }
    assert_that!(r.len_changes()).is_equal_to(before + 3);
}

#[test]
fn first_edit_records_author_for_this_peer() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Renommer").unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn blank_author_records_nothing() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("   ".into()));
    b.apply(titled(&deck, "Un"), "Renommer").unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_none();
}

#[test]
fn silent_apply_records_no_author() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("Julie".into()));
    b.apply_silent(titled(&deck, "Canonique")).unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_none();
}
```

Ajouter `DeckReplica::peer_id(&self) -> u64` (utile aussi à la tâche 2). `#[cfg(test)] #[path = "authors_tests.rs"] mod authors_tests;` dans `authors.rs`.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/authors/)'` → échec de compilation.

- [ ] **Step 3 : implémentation** — `replica/authors.rs` :

```rust
//! Auteurs du deck : le nom de chaque pair, dans la map racine `peers`.
//!
//! Un pair (une session d'édition) s'inscrit à sa première édition, dans le
//! même commit : un pair qui n'a rien gardé n'a pas de nom.

use loro::{LoroDoc, LoroValue, ValueOrContainer};
use snafu::ResultExt;

use crate::error::{LoroSnafu, ReplicaInner};

pub(super) const PEERS: &str = "peers";

pub(super) fn author_of(doc: &LoroDoc, peer: u64) -> Option<String> {
    match doc.get_map(PEERS).get(&peer.to_string()) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Inscrit `name` pour ce pair s'il ne l'est pas encore (sans committer).
pub(super) fn claim(doc: &LoroDoc, name: &str) -> Result<(), ReplicaInner> {
    let key = doc.peer_id().to_string();
    let peers = doc.get_map(PEERS);
    if peers.get(&key).is_none() {
        peers.insert(&key, name).context(LoroSnafu)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "authors_tests.rs"]
mod authors_tests;
```

Dans `mod.rs` :
- `mod authors;` ; champ `author: Option<String>` dans `DeckReplica` (initialisé à `None` dans `create` et `import_snapshot`).
- `configure(doc)` ajoute `doc.set_change_merge_interval(-1);` avec un commentaire : sans cela Loro fusionne les commits d'une fenêtre de 1000 s en un *change*.
- Méthodes :

```rust
/// Nom de l'auteur de ce pair ; un nom blanc vaut aucun.
pub fn set_author(&mut self, name: Option<String>) {
    self.author = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
}

pub fn author_of(&self, peer: u64) -> Option<String> {
    authors::author_of(&self.doc, peer)
}

pub fn peer_id(&self) -> u64 {
    self.doc.peer_id()
}

pub fn len_changes(&self) -> usize {
    self.doc.len_changes()
}

/// Inscrit l'auteur de ce pair, s'il en a un, avant sa première édition.
pub(crate) fn claim_author(&self) -> Result<(), ReplicaError> {
    match &self.author {
        Some(name) => Ok(authors::claim(&self.doc, name)?),
        None => Ok(()),
    }
}
```

- `commit(&mut self, label)` : `self.doc.set_next_commit_message(label);` avant `self.doc.commit()` (garder le reste : `label_next`, `clear_next`).
- `sources.rs` : `SourceBundle::set_author(&mut self, name)` délègue ; `apply` appelle `self.replica.claim_author().context(ReplicaSnafu)?;` **avant** `reconcile` (même mappage d'erreur que la ligne voisine). `apply_silent` n'appelle pas `claim_author`.

- [ ] **Step 4 : vérifier** — `cargo nextest run -p oku-slides-core` → PASS (y compris les tests d'annulation : l'inscription de l'auteur fait partie de la première étape et s'annule avec elle, ce qui est voulu).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core
git commit -m "feat(core) : un commit, un change Loro libellé ; auteurs du deck dans peers (t-235)"
```

---

### Task 2 : Fusion d'un SourceBundle et rapport

**Files :**
- Modify : `crates/oku-slides-core/src/replica/authors.rs` (rapport), `replica/mod.rs` (`merge`), `crates/oku-slides-core/src/sources.rs`, `crates/oku-slides-core/src/lib.rs`
- Test : `replica/authors_tests.rs`, `crates/oku-slides-core/src/sources_tests.rs`

**Interfaces :**
- Consumes : `DeckReplica::{set_author, author_of, peer_id}`, `SourceBundle::apply`.
- Produces :
  - `pub struct MergeReport { pub changes: usize, pub authors: Vec<String>, pub anonymous: usize }` (`Debug, Clone, PartialEq, Eq, Default`), exporté à la racine du crate.
  - `DeckReplica::merge(&mut self, other: &DeckReplica) -> Result<MergeReport, ReplicaError>` (était `()`).
  - `SourceBundle::merge(&mut self, other: SourceBundle) -> Result<(MergeReport, Vec<Issue>), SourcesError>` : replica fusionné, union des assets et des polices (on garde les nôtres à nom égal), vue relue.

- [ ] **Step 1 : tests rouges** — dans `authors_tests.rs` :

```rust
fn peer(deck: &Deck, snap: &[u8], author: &str) -> SourceBundle {
    let (replica, read) = DeckReplica::from_snapshot(snap, 1 << 24).unwrap();
    let (mut b, _) = SourceBundle::from_replica(replica, read, Default::default(), Default::default());
    b.set_author(Some(author.into()));
    let _ = deck;
    b
}

#[test]
fn merge_reports_imported_changes_and_authors() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let mut b = peer(&deck, &snap, "Julie");
    b.apply(titled(&deck, "Un"), "Un").unwrap();
    let mut d = titled(&deck, "Un");
    d.theme = Some("pro".into());
    b.apply(d, "Thème").unwrap();
    let (report, issues) = a.merge(b).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(report.changes).is_equal_to(2);
    assert_that!(report.authors).is_equal_to(vec!["Julie".to_owned()]);
    assert_that!(report.anonymous).is_equal_to(0);
}

#[test]
fn merge_twice_imports_nothing_the_second_time() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let mut b = peer(&deck, &snap, "Julie");
    b.apply(titled(&deck, "Un"), "Un").unwrap();
    let again = SourceBundle::from_zip(&b.to_zip().unwrap(), crate::SourceLimits::default()).unwrap().0;
    a.merge(b).unwrap();
    let (report, _) = a.merge(again).unwrap();
    assert_that!(report).is_equal_to(crate::MergeReport::default());
}

#[test]
fn merge_counts_nameless_peers_as_anonymous() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let (replica, read) = DeckReplica::from_snapshot(&snap, 1 << 24).unwrap();
    let (mut b, _) = SourceBundle::from_replica(replica, read, Default::default(), Default::default());
    b.apply(titled(&deck, "Un"), "Un").unwrap();
    let (report, _) = a.merge(b).unwrap();
    assert_that!(report.authors).is_empty();
    assert_that!(report.anonymous).is_equal_to(1);
}
```

Dans `sources_tests.rs` :

```rust
#[test]
fn merge_unions_assets_and_keeps_our_font_on_name_clash() {
    let deck = roadmap();
    let mut a = SourceBundle::new(deck.clone()).unwrap();
    let snap = a.to_zip().unwrap();
    let mut b = SourceBundle::from_zip(&snap, SourceLimits::default()).unwrap().0;
    a.fonts.insert("f.ttf".into(), b"nôtre".to_vec());
    b.fonts.insert("f.ttf".into(), b"sienne".to_vec());
    let id = b.add_asset(b"png".to_vec());
    a.merge(b).unwrap();
    assert_that!(a.assets.contains_key(&id)).is_true();
    assert_that!(a.fonts.get("f.ttf").map(Vec::as_slice)).is_equal_to(Some(&b"n\xc3\xb4tre"[..]));
}

#[test]
fn merge_unrelated_bundles_returns_replica_unrelated() {
    let mut a = SourceBundle::new(roadmap()).unwrap();
    let b = SourceBundle::new(roadmap()).unwrap();
    let err = a.merge(b).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(crate::ReplicaErrorKind::Unrelated));
}
```

Adapter les signatures réelles : `SourceBundle::from_replica`, `from_zip` et `SourceLimits` sont ceux de `sources.rs` à HEAD (lire le fichier ; si `SourceLimits` n'implémente pas `Default`, construire `SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28 }`). Les tests existants de `DeckReplica::merge` (`mod_tests.rs`, `undo_tests.rs`, `tests/convergence.rs`) ignorent le nouveau retour (`.unwrap();` reste valable).

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-core -E 'test(/merge/)'` → échec de compilation.

- [ ] **Step 3 : rapport** — dans `authors.rs` :

```rust
use std::collections::BTreeSet;
use std::ops::ControlFlow;

use loro::VersionVector;

/// Ce qu'une fusion a importé : ses modifications, et qui les a faites.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeReport {
    /// Modifications importées (une par commit, voir `configure`).
    pub changes: usize,
    /// Auteurs nommés, triés, sans doublon.
    pub authors: Vec<String>,
    /// Pairs sans nom parmi ces modifications.
    pub anonymous: usize,
}

/// Les modifications de `doc` absentes de `before`.
pub(super) fn report_since(doc: &LoroDoc, before: &VersionVector) -> MergeReport {
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let (mut changes, mut peers) = (0, BTreeSet::new());
    let _ = doc.travel_change_ancestors(&heads, &mut |meta| {
        if !before.includes_id(meta.id) {
            changes += 1;
            peers.insert(meta.id.peer);
        }
        ControlFlow::Continue(())
    });
    let (mut authors, mut anonymous) = (BTreeSet::new(), 0);
    for peer in peers {
        match author_of(doc, peer) {
            Some(name) => {
                authors.insert(name);
            }
            None => anonymous += 1,
        }
    }
    MergeReport { changes, authors: authors.into_iter().collect(), anonymous }
}
```

Vérifier dans `~/.cargo/registry/src/*/loro-1.16.2/src/lib.rs` les noms exacts (`Frontiers::iter`, `VersionVector::includes_id`, signature de `travel_change_ancestors`) et adapter ; l'erreur de parcours est ignorée faute de mieux (un historique cohérent ne la produit pas) — le dire en commentaire. Le parcours visite tout l'historique : acceptable (une fusion est rare).

- [ ] **Step 4 : `DeckReplica::merge`** — dans `merge_inner`, garder `let before = self.doc.oplog_vv();` avant l'import réel, puis `Ok(authors::report_since(&self.doc, &before))`. Signature publique : `-> Result<MergeReport, ReplicaError>`. Exporter `MergeReport` (`pub use replica::MergeReport;` et doc de tête de `lib.rs`).

- [ ] **Step 5 : `SourceBundle::merge`**

```rust
/// Fusionne `other` : son historique, puis ses assets et polices (à nom égal,
/// on garde les nôtres). La vue est relue ; ses parties illisibles sont rendues.
pub fn merge(&mut self, other: SourceBundle) -> Result<(MergeReport, Vec<Issue>), SourcesError> {
    let report = self.replica.merge(&other.replica).context(ReplicaSnafu)?;
    for (id, bytes) in other.assets {
        self.assets.entry(id).or_insert(bytes);
    }
    for (name, bytes) in other.fonts {
        self.fonts.entry(name).or_insert(bytes);
    }
    let (deck, issues) = self.replica.deck();
    self.deck = deck;
    Ok((report, issues))
}
```

Reprendre le mappage d'erreur exact de `apply` (le `ReplicaSnafu` de `sources.rs` enveloppe peut-être la source dans un `Box`).

- [ ] **Step 6 : vérifier** — `cargo nextest run -p oku-slides-core` et `cargo clippy -p oku-slides-core --all-targets -- -D warnings` → propres.

- [ ] **Step 7 : commit**

```bash
git add crates/oku-slides-core
git commit -m "feat(core) : fusion d'un SourceBundle, rapport des modifications et de leurs auteurs (t-236)"
```

---

### Task 3 : Nom d'auteur dans les réglages

**Files :**
- Modify : `crates/oku-slides-platform/src/config.rs`, `config.default.toml`, `config_tests.rs`
- Modify : `crates/oku-slides-session/src/lib.rs` (`Document::new`, champ de session), `crates/oku-slides-session/src/resume.rs` (appel de `Document::new`)
- Test : `crates/oku-slides-session/tests/session.rs`

**Interfaces :**
- Produces :
  - `oku_slides_platform::config::AuthorSettings { pub name: Option<String> }`, `Settings.author`.
  - `AuthorSettings::resolved(&self) -> Option<String>` : le nom réglé, sinon `$USER` puis `$USERNAME`, sinon `None` ; blanc = absent.
  - Session : `struct DocDefaults { undo_limit: usize, author: Option<String> }` remplace `history_limit` ; `Document::new(sources, path, &DocDefaults)` appelle `sources.set_undo_limit` et `sources.set_author`.

- [ ] **Step 1 : tests rouges** — `config_tests.rs` :

```rust
#[test]
fn author_name_setting_wins_over_system_user() {
    let a = AuthorSettings { name: Some("Julie".into()) };
    assert_that!(a.resolved()).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn blank_author_name_falls_back_to_system_user() {
    let a = AuthorSettings { name: Some("  ".into()) };
    let expected = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok().filter(|u| !u.trim().is_empty());
    assert_that!(a.resolved()).is_equal_to(expected);
}

#[test]
fn default_settings_have_no_author_name() {
    assert_that!(Settings::defaults().author.name).is_none();
}
```

(`Settings::defaults()` : utiliser le constructeur réel des tests voisins.) Dans `tests/session.rs` :

```rust
#[test]
fn edit_records_the_configured_author() {
    let data = tempdir();
    let mut settings = Settings::defaults();
    settings.author.name = Some("Julie".into());
    // construire la session comme `session_in`, avec ces réglages
    let mut s = session_with_settings(&data, settings);
    s.handle(set_theme("pro")).unwrap();
    let r = s.sources_for_tests().replica();
    assert_that!(r.author_of(r.peer_id())).is_equal_to(Some("Julie".to_owned()));
}
```

Écrire `session_with_settings` à partir de `session_in` (même fichier) ; `sources_for_tests` : accesseur `#[doc(hidden)] pub fn` ou helper existant — reprendre celui qu'utilisent déjà les tests de reprise du plan 1 s'il existe (`grep -n "replica()" crates/oku-slides-session/tests/*.rs`).

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-platform -p oku-slides-session -E 'test(/author/)'` → échec de compilation.

- [ ] **Step 3 : implémentation** — `config.default.toml` :

```toml
[author]
# Nom inscrit dans l'historique des decks que vous modifiez ; absent =
# l'utilisateur du système.
# name = "Julie"
```

`config.rs` :

```rust
/// Auteur des modifications, inscrit dans l'historique du deck.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct AuthorSettings {
    #[serde(default)]
    pub name: Option<String>,
}

impl AuthorSettings {
    /// Le nom réglé, sinon l'utilisateur du système ; blanc = aucun.
    pub fn resolved(&self) -> Option<String> {
        let blank = |s: &String| s.trim().is_empty();
        self.name
            .clone()
            .filter(|n| !blank(n))
            .or_else(|| std::env::var("USER").ok().filter(|n| !blank(n)))
            .or_else(|| std::env::var("USERNAME").ok().filter(|n| !blank(n)))
            .map(|n| n.trim().to_owned())
    }
}
```

`Settings` gagne `#[serde(default)] pub author: AuthorSettings`. Session : `DocDefaults { undo_limit: settings.history.limit, author: settings.author.resolved() }` gardé dans `Session` à la place de `history_limit` ; chaque `Document::new(.., self.history_limit)` devient `Document::new(.., &self.doc_defaults)` (y compris `resume.rs::read_doc` : lui passer `&DocDefaults` au lieu de `undo_limit`, et adapter ses appelants). `Document::new` appelle `sources.set_author(defaults.author.clone())`. Le mode modèles utilise les mêmes réglages.

- [ ] **Step 4 : vérifier** — `cargo nextest run -p oku-slides-platform -p oku-slides-session` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-platform crates/oku-slides-session
git commit -m "feat(session) : nom d'auteur réglable, inscrit dans l'historique du deck (t-237)"
```

---

### Task 4 : Commande Fusionner avec un PDF

**Files :**
- Create : `crates/oku-slides-session/src/merge.rs`, `merge_tests.rs`
- Modify : `crates/oku-slides-session/src/lib.rs` (`Request::Merge`, dispatch, extraction de `refresh_fonts`), `error.rs` (`MergeUnrelated`)
- Modify : `crates/oku-slides-i18n/src/locales/*/session.ftl` (`notice-merged`, `notice-merged-anonymous`), `*/ui.ftl` (`ui-merge`) — annexe A
- Modify : `ui/src/api.ts`, `ui/src/App.tsx`
- Test : `crates/oku-slides-session/tests/session.rs`, `ui/src/api.test.ts` si les requêtes y sont testées

**Interfaces :**
- Consumes : `SourceBundle::merge -> (MergeReport, Vec<Issue>)`, `read_document(&Storage, &Path, SourceLimits) -> Result<Opened>`.
- Produces :
  - `Request::Merge { path: PathBuf }` (serde `"merge"`).
  - `Session::merge_file(&mut self, path: &Path) -> Result<MergeReport>` (dans `merge.rs`, `impl Session`).
  - `SessionErrorKind::MergeUnrelated` (message : clé `replica-unrelated`).
  - `pub(crate) fn merge_notice(l: &Localizer, file: &str, report: &MergeReport) -> String`.
  - TS : `Request` gagne `{ cmd: "merge"; path: string }`, `tabApi(tab).merge(path)`.

- [ ] **Step 1 : tests rouges (session)** — `tests/session.rs`, avec les helpers du fichier (`session()`, `fixture_path()`, `set_theme`) :

```rust
#[test]
fn merge_file_brings_the_other_copy_changes() {
    let dir = tempdir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut b = session();
    b.open(&path).unwrap();
    b.handle(set_theme("pro")).unwrap();
    let other = dir.join("julie.oku-slides.pdf");
    b.save(Some(&other)).unwrap();
    let report = a.merge_file(&other).unwrap();
    assert_that!(report.changes).is_greater_than(0);
    assert_that!(a.deck().theme.as_deref()).is_equal_to(Some("pro"));
    let snap: serde_json::Value = serde_json::from_str(&a.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["dirty"]).is_equal_to(serde_json::json!(true));
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("julie.oku-slides.pdf");
}

#[test]
fn merge_file_of_another_deck_returns_merge_unrelated() {
    let dir = tempdir();
    let other = dir.join("autre.oku-slides.pdf");
    let mut b = session();
    b.open(&fixture_path()).unwrap();
    b.save(Some(&other)).unwrap();
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    let err = a.merge_file(&other).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::MergeUnrelated);
}

#[test]
fn merge_file_of_a_json_deck_returns_not_oku_slides() {
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    let err = a.merge_file(&fixture_path()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotOkuSlides);
}
```

Adapter aux helpers réels (`tempdir` ou `data_dir`, lecture du snapshot : reprendre le style des tests voisins). Dans `merge_tests.rs`, tester `merge_notice` : avec auteurs (`"Julie, Marc"` dans le texte), sans auteurs (clé anonyme), en français (`Localizer` des tests i18n).

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-session -E 'test(/merge/)'` → échec de compilation.

- [ ] **Step 3 : erreur** — `SessionInner::MergeUnrelated { source: SourcesError, backtrace }` ; `kind()` → `SessionErrorKind::MergeUnrelated` ; `Localize` → `l.tr("replica-unrelated")`. `merge_file` mappe `SourcesErrorKind::Replica(ReplicaErrorKind::Unrelated)` vers cette variante, les autres erreurs de sources vers la variante existante (`SourcesSnafu`).

- [ ] **Step 4 : `merge.rs`**

```rust
//! Fusion d'un autre PDF du même deck dans la session.

use std::path::Path;

use oku_slides_core::MergeReport;
use oku_slides_i18n::Localizer;
use snafu::ensure;

use crate::files::read_document;
use crate::{Result, Session};

/// L'avis montré après une fusion.
pub(crate) fn merge_notice(l: &Localizer, file: &str, report: &MergeReport) -> String {
    let count = i64::try_from(report.changes).unwrap_or(i64::MAX);
    if report.authors.is_empty() {
        l.tr_with("notice-merged-anonymous", &[("file", file.into()), ("count", count.into())])
    } else {
        let authors = report.authors.join(", ");
        l.tr_with("notice-merged", &[("file", file.into()), ("count", count.into()), ("authors", authors.as_str().into())])
    }
}

impl Session {
    /// Fusionne le `.oku-slides.pdf` `path` (même deck) dans le deck ouvert.
    ///
    /// La fusion ne s'annule pas (les modifications importées sont celles
    /// d'autres pairs) : l'historique garde la version d'avant.
    pub fn merge_file(&mut self, path: &Path) -> Result<MergeReport> {
        ensure!(self.editing_deck(), crate::error::EditingTemplatesSnafu);
        let opened = read_document(&self.storage, path, self.source_limits)?;
        ensure!(opened.pdf, crate::error::NotOkuSlidesSnafu);
        let (report, issues) = self.doc.sources.merge(opened.sources).map_err(crate::error::merge_error)?;
        for issue in &issues {
            tracing::warn!(issue = %issue, "partie illisible après fusion");
        }
        self.refresh_fonts()?;
        self.doc.dirty = true;
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.doc.notice = Some(merge_notice(&self.localizer, &file, &report));
        self.rerender()?;
        Ok(report)
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod merge_tests;
```

Adapter aux noms réels : le test « mode deck » (`matches!(self.mode, Mode::Deck)`), les sélecteurs Snafu existants (`EditingTemplatesSnafu`, `NotOkuSlidesSnafu`), `Localizer::tr_with` et la conversion d'arguments (`crates/oku-slides-i18n/src/lib.rs`), la visibilité des champs de `Session` depuis un module frère (`pub(crate)` si besoin). `merge_error(SourcesError) -> SessionError` vit dans `error.rs`. `refresh_fonts` : extraire de `load_document` le bloc « nouvelles polices : nouveau monde Typst » en `fn refresh_fonts(&mut self) -> Result<()>` qui ne fait rien si les polices du rendu sont déjà celles des sources, et l'appeler aux deux endroits.

- [ ] **Step 5 : requête** — `Request::Merge { path: PathBuf }` ; dans `handle`, `Request::Merge { path } => { self.merge_file(&path)?; }`, classée parmi les requêtes qui changent le document (reprise écrite). Refusée en mode modèles comme `Save`.

- [ ] **Step 6 : Fluent** — `notice-merged`, `notice-merged-anonymous` dans `session.ftl`, `ui-merge` dans `ui.ftl`, 20 locales (annexe A).

- [ ] **Step 7 : UI** — `api.ts` : `| { cmd: "merge"; path: string }` et `merge: (path: string) => call<Snapshot>({ cmd: "merge", path })` dans `tabApi`. `App.tsx`, à côté de `open` / `save` :

```tsx
const merge = async () => {
  const path = await pickOpenPath();
  if (path) await run(api.merge(path));
};
```

et un bouton *Fusionner…* (`t("ui-merge")`) à côté du bouton d'ouverture, avec le même style ; masqué en mode modèles (`snap.editing`). L'avis de fusion arrive par `snap.notice`, déjà affiché (`App.tsx:151`).

- [ ] **Step 8 : vérifier** — `cargo nextest run -p oku-slides-session -p oku-slides-i18n`, `cd ui && bun test && bun run typecheck` → PASS.

- [ ] **Step 9 : commit**

```bash
git add crates ui/src
git commit -m "feat(session) : fusionner avec un PDF du même deck, avis des modifications importées (t-238)"
```

---

### Task 5 : Enregistrer : fusion ou fichier remplacé

**Files :**
- Modify : `crates/oku-slides-session/src/merge.rs` (vérification du disque), `lib.rs` (`Request::Save { path, overwrite }`, `save_checked`, `Document.disk_replaced`), `views.rs` (`Snapshot.disk_replaced`)
- Create : `ui/src/merge.ts`, `ui/src/merge.test.ts`
- Modify : `ui/src/api.ts`, `ui/src/App.tsx`, `ui/src/close.ts`, `ui/src/close.test.ts`
- Modify : `crates/oku-slides-i18n/src/locales/*/ui.ftl` (`ui-disk-replaced-question`, `ui-disk-replaced-overwrite`, `ui-disk-replaced-save-as`) — annexe A
- Test : `crates/oku-slides-session/tests/session.rs`

**Interfaces :**
- Consumes : `Session::merge_file` (logique partagée), `read_document`.
- Produces :
  - `Request::Save { path: Option<PathBuf>, #[serde(default)] overwrite: bool }`.
  - `pub enum Saved { Written, Merged(MergeReport), DiskReplaced }` (session).
  - `Session::save_checked(&mut self, path: Option<&Path>, overwrite: bool) -> Result<Saved>` ; `Session::save(path)` reste (CLI, tests) = `save_checked(path, true)` sans vérification.
  - `Snapshot.disk_replaced: bool` (vrai une fois, après un `Save` refusé).
  - TS : `tabApi(tab).save(path, overwrite = false)`, `Snapshot.disk_replaced: boolean`, `ui/src/merge.ts` : `saveGuarded(api, snap, as, ask?, pick?) -> Promise<Snapshot | null>`.

- [ ] **Step 1 : tests rouges (session)**

```rust
#[test]
fn save_over_a_file_changed_by_another_copy_merges_first() {
    let dir = tempdir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut b = session();
    b.open(&path).unwrap();
    b.handle(set_theme("pro")).unwrap();
    b.save(Some(&path)).unwrap();
    // A modifie un titre puis enregistre au même endroit.
    a.handle(retitle_third_slide("Contexte (A)")).unwrap();
    let saved = a.save_checked(None, false).unwrap();
    assert_that!(matches!(saved, Saved::Merged(_))).is_true();
    let mut c = session();
    c.open(&path).unwrap();
    assert_that!(c.deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(c.deck().slides[2].title.as_deref()).is_equal_to(Some("Contexte (A)"));
}

#[test]
fn save_over_a_file_replaced_by_another_deck_is_refused_until_overwrite() {
    let dir = tempdir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut other = session();
    other.open(&fixture_path()).unwrap();
    other.save(Some(&path)).unwrap(); // autre deck_uid au même chemin
    a.handle(set_theme("pro")).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::DiskReplaced)).is_true();
    let snap: serde_json::Value = serde_json::from_str(&a.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["disk_replaced"]).is_equal_to(serde_json::json!(true));
    assert_that!(matches!(a.save_checked(None, true).unwrap(), Saved::Written)).is_true();
}

#[test]
fn save_over_an_unchanged_file_just_writes() {
    let dir = tempdir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    a.handle(set_theme("pro")).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::Written)).is_true();
}

#[test]
fn save_after_the_file_was_deleted_just_writes() {
    let dir = tempdir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::Written)).is_true();
}
```

`retitle_third_slide(t) -> Request` : `Request::Apply` d'un `SetSlideField` sur la troisième slide de la fixture (écrire le helper à côté de `set_theme`).

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-session -E 'test(/save_/)'` → échec de compilation.

- [ ] **Step 3 : vérification du disque** — dans `merge.rs` :

```rust
/// Issue d'un enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    Written,
    /// Le fichier avait changé (même deck) : fusionné, puis écrit.
    Merged(MergeReport),
    /// Le fichier a été remplacé par un autre deck : rien n'est écrit.
    DiskReplaced,
}

/// Ce que le disque impose avant d'écrire `path`.
pub(crate) enum OnDisk {
    Unchanged,
    SameDeck(Box<oku_slides_core::SourceBundle>),
    Replaced,
}

impl Session {
    /// Le fichier `path` a-t-il changé depuis l'ouverture ou le dernier enregistrement ?
    pub(crate) fn on_disk(&self, path: &Path) -> OnDisk {
        let Some(saved) = self.doc.saved_hash.as_deref() else { return OnDisk::Unchanged };
        if self.doc.path.as_deref() != Some(path) {
            return OnDisk::Unchanged;
        }
        match read_document(&self.storage, path, self.source_limits) {
            // Supprimé ou illisible comme fichier : on l'écrit à nouveau.
            Err(e) if e.kind() == crate::SessionErrorKind::File => OnDisk::Unchanged,
            Ok(o) if o.hash == saved => OnDisk::Unchanged,
            Ok(o) if o.pdf && o.sources.replica().deck_uid() == self.doc.sources.replica().deck_uid() => {
                OnDisk::SameDeck(Box::new(o.sources))
            }
            _ => OnDisk::Replaced,
        }
    }
}
```

Vérifier le `kind` rendu par `read_document` pour un fichier absent (`File` d'après `error.rs`) et l'ajuster.

- [ ] **Step 4 : `save_checked`** — dans `lib.rs`, extraire l'écriture actuelle de `save` en `fn write_to(&mut self, path: PathBuf) -> Result<()>` (chemin déjà normalisé), puis :

```rust
/// Enregistre, sans écraser un fichier changé sur disque : même deck,
/// fusion puis écriture ; autre deck, refus (sauf `overwrite`).
pub fn save_checked(&mut self, path: Option<&Path>, overwrite: bool) -> Result<Saved> {
    let path = self.save_path(path)?;
    if !overwrite {
        match self.on_disk(&path) {
            OnDisk::Unchanged => {}
            OnDisk::SameDeck(theirs) => {
                let (report, _) = self.doc.sources.merge(*theirs).map_err(crate::error::merge_error)?;
                self.refresh_fonts()?;
                self.rerender()?;
                let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                self.doc.notice = Some(merge::merge_notice(&self.localizer, &file, &report));
                self.write_to(path)?;
                return Ok(Saved::Merged(report));
            }
            OnDisk::Replaced => {
                self.doc.disk_replaced = true;
                return Ok(Saved::DiskReplaced);
            }
        }
    }
    self.write_to(path)?;
    Ok(Saved::Written)
}

/// Enregistre sans vérifier le disque (CLI, choix « Écraser »).
pub fn save(&mut self, path: Option<&Path>) -> Result<()> {
    self.save_checked(path, true).map(|_| ())
}
```

`save_path` : la normalisation actuelle du chemin (`EXTENSION`), extraite. `Document` gagne `disk_replaced: bool` ; `snapshot()` le rend, et `handle` le remet à `false` après l'envoi, au même endroit que `notice`. `Request::Save { path, overwrite }` → `self.save_checked(path.as_deref(), overwrite)?`. Garder la règle existante : `Save` refusé en mode modèles.

- [ ] **Step 5 : tests rouges (UI)** — `ui/src/merge.test.ts` :

```ts
import { describe, expect, test } from "bun:test";

import { saveGuarded } from "./merge";

const snap = (over: Partial<Record<string, unknown>> = {}) =>
  ({ path: "/d/deck.oku-slides.pdf", name: "deck", dirty: true, disk_replaced: false, ...over }) as never;

describe("saveGuarded", () => {
  test("saveGuarded with an unchanged file saves once", async () => {
    const calls: unknown[] = [];
    const api = { save: async (p: string | null, o = false) => (calls.push([p, o]), snap({ dirty: false })) } as never;
    const out = await saveGuarded(api, snap(), false);
    expect(calls).toEqual([[null, false]]);
    expect(out).not.toBeNull();
  });

  test("saveGuarded on a replaced file and overwrite chosen saves with overwrite", async () => {
    const calls: unknown[] = [];
    let first = true;
    const api = {
      save: async (p: string | null, o = false) => {
        calls.push([p, o]);
        const r = first ? snap({ disk_replaced: true }) : snap({ dirty: false });
        first = false;
        return r;
      },
    } as never;
    await saveGuarded(api, snap(), false, async () => "overwrite");
    expect(calls).toEqual([[null, false], [null, true]]);
  });

  test("saveGuarded on a replaced file and save as chosen saves to the picked path", async () => {
    const calls: unknown[] = [];
    let first = true;
    const api = {
      save: async (p: string | null, o = false) => {
        calls.push([p, o]);
        const r = first ? snap({ disk_replaced: true }) : snap({ dirty: false });
        first = false;
        return r;
      },
    } as never;
    await saveGuarded(api, snap(), false, async () => "save-as", async () => "/d/autre.oku-slides.pdf");
    expect(calls).toEqual([[null, false], ["/d/autre.oku-slides.pdf", false]]);
  });

  test("saveGuarded on a replaced file and cancel returns null", async () => {
    const api = { save: async () => snap({ disk_replaced: true }) } as never;
    expect(await saveGuarded(api, snap(), false, async () => "cancel")).toBeNull();
  });
});
```

- [ ] **Step 6 : UI** — `ui/src/merge.ts` :

```ts
// Enregistrer sans écraser un fichier remplacé, et ouvrir un deck déjà
// ouvert : les deux questions, sur le modèle de `close.ts`.

import { inTauri, pickSavePath, type Snapshot, type TabApi } from "./api";
import { t } from "./i18n";

/** Que faire d'un fichier remplacé sur disque par un autre deck. */
export type ReplacedChoice = "overwrite" | "save-as" | "cancel";
export type AskReplaced = (file: string) => Promise<ReplacedChoice>;
export type PickSavePath = (name: string) => Promise<string | null>;

const fileName = (path: string | null, name: string) => (path ? path.split(/[\\/]/).pop() ?? name : name);

async function askReplaced(file: string): Promise<ReplacedChoice> {
  const text = t("ui-disk-replaced-question", { file });
  if (!inTauri) return window.confirm(text) ? "overwrite" : "cancel";
  const labels = { yes: t("ui-disk-replaced-overwrite"), no: t("ui-disk-replaced-save-as"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const r = await message(text, { kind: "warning", buttons: labels });
  if (r === labels.yes || r === "Yes") return "overwrite";
  if (r === labels.no || r === "No") return "save-as";
  return "cancel";
}

/**
 * Enregistre `snap` (sous un nouveau nom si `as`) ; un fichier remplacé par
 * un autre deck fait poser la question. `null` : rien n'a été enregistré.
 */
export async function saveGuarded(
  api: TabApi,
  snap: Snapshot,
  as: boolean,
  ask: AskReplaced = askReplaced,
  pick: PickSavePath = pickSavePath,
): Promise<Snapshot | null> {
  const path = as || !snap.path ? await pick(snap.name) : null;
  if ((as || !snap.path) && !path) return null;
  const s = await api.save(path);
  if (!s.disk_replaced) return s;
  const choice = await ask(fileName(snap.path, snap.name));
  if (choice === "overwrite") return api.save(null, true);
  if (choice === "save-as") {
    const other = await pick(snap.name);
    return other ? api.save(other) : null;
  }
  return null;
}
```

`api.ts` : `| { cmd: "save"; path: string | null; overwrite: boolean }`, `save: (path: string | null, overwrite = false) => call<Snapshot>({ cmd: "save", path, overwrite })`, `Snapshot.disk_replaced: boolean`. `App.tsx` `save` devient :

```tsx
const save = async (as = false) => {
  if (!snap) return;
  if (await run(saveGuarded(api, snap, as))) say(t("ui-toast-saved"));
};
```

(vérifier que `run` renvoie une valeur vraie seulement si la promesse aboutit à une valeur non nulle ; sinon tester `!== null`). `close.ts` `saveForClose` : remplacer `await api.save(path); return true;` par `return (await saveGuarded(api, s, false, undefined, pick)) !== null;` (garder l'injection de `pick`) et adapter `close.test.ts`.

- [ ] **Step 7 : Fluent** — `ui-disk-replaced-question`, `ui-disk-replaced-overwrite`, `ui-disk-replaced-save-as` dans `ui.ftl` des 20 locales (annexe A).

- [ ] **Step 8 : vérifier** — `cargo nextest run -p oku-slides-session -p oku-slides-i18n`, `cd ui && bun test && bun run typecheck` → PASS.

- [ ] **Step 9 : commit**

```bash
git add crates ui/src
git commit -m "feat(session) : enregistrer fusionne un fichier changé du même deck, et n'écrase pas un autre deck sans le demander (t-239)"
```

---

### Task 6 : Ouvrir un deck déjà ouvert

**Files :**
- Modify : `crates/oku-slides-session/src/views.rs` (`TabInfo.deck_uid`), `lib.rs` (`tab_info`), `workspace.rs` (`WorkspaceRequest::SameDeck`), `workspace_resume.rs` (`open_files`), `files.rs` (lecture du `deck_uid` d'un fichier)
- Modify : `src-tauri/src/commands.rs` (`classify`)
- Modify : `crates/oku-slides-i18n/src/locales/*/session.ftl` (`notice-same-deck-open`), `*/ui.ftl` (`ui-same-deck-question`, `ui-same-deck-merge`, `ui-same-deck-apart`) — annexe A
- Modify : `ui/src/api.ts`, `ui/src/merge.ts`, `ui/src/merge.test.ts`, `ui/src/Workspace.tsx`
- Test : `crates/oku-slides-session/tests/workspace.rs`

**Interfaces :**
- Consumes : `Session::merge_file`, `tabApi(tab).merge(path)`.
- Produces :
  - `TabInfo.deck_uid: Option<String>` (`#[serde(default)]` : un `workspace.json` plus ancien se relit).
  - `WorkspaceRequest::SameDeck { path: PathBuf }` → JSON `{ "tab": "t-3" | null, "title": "…" | null }` ; aucun effet.
  - `pub fn deck_uid_of(storage: &Storage, path: &Path, limits: SourceLimits) -> Result<Option<String>>` (`files.rs`) : `None` pour un deck JSON.
  - TS : `workspace.sameDeck(path) -> Promise<{ tab: string | null; title: string | null }>` ; `merge.ts` : `openGuarded(window, path, ask?)`.

- [ ] **Step 1 : tests rouges (espace de travail)** — `tests/workspace.rs`, avec ses helpers (`resumed`, `send`, `saved_pdf`, `view`) :

```rust
#[test]
fn same_deck_finds_the_tab_holding_that_deck() {
    let data = tempdir();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    // un onglet ouvre l'original ; une copie du fichier ailleurs
    open_in_first_window(&ws, &original);
    let copy = data.join("copie.oku-slides.pdf");
    std::fs::copy(&original, &copy).unwrap();
    let reply = send(&ws, WorkspaceRequest::SameDeck { path: copy.clone() });
    let v: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(v["tab"].is_string()).is_true();
}

#[test]
fn same_deck_for_the_very_same_path_answers_none() {
    // OpenTab active déjà l'onglet de ce chemin : pas de question.
    let data = tempdir();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let v: serde_json::Value =
        serde_json::from_str(&send(&ws, WorkspaceRequest::SameDeck { path: original })).unwrap();
    assert_that!(v["tab"].is_null()).is_true();
}

#[test]
fn same_deck_for_another_deck_answers_none() {
    let data = tempdir();
    let ws = resumed(&data);
    let a = saved_pdf(&data, "a");
    let b = saved_pdf(&data, "b"); // un autre deck_uid
    open_in_first_window(&ws, &a);
    let v: serde_json::Value = serde_json::from_str(&send(&ws, WorkspaceRequest::SameDeck { path: b })).unwrap();
    assert_that!(v["tab"].is_null()).is_true();
}

#[test]
fn tab_info_keeps_deck_uid_across_resume() {
    let data = tempdir();
    let original = saved_pdf(&data, "deck");
    {
        let ws = resumed(&data);
        open_in_first_window(&ws, &original);
    }
    let ws = resumed(&data);
    let view: serde_json::Value = serde_json::from_str(&send(&ws, WorkspaceRequest::View)).unwrap();
    let uids: Vec<_> = view["windows"][0]["tabs"].as_array().unwrap().iter().filter_map(|t| t["deck_uid"].as_str()).collect();
    assert_that!(uids).has_length(1);
}

#[test]
fn open_files_of_a_deck_already_open_opens_apart_with_a_notice() {
    let data = tempdir();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let copy = data.join("copie.oku-slides.pdf");
    std::fs::copy(&original, &copy).unwrap();
    Workspace::open_files(&ws, vec![copy]).unwrap();
    // le nouvel onglet porte l'avis qui désigne l'onglet déjà ouvert
    assert_that!(notice_of_active_tab(&ws)).is_some();
}
```

Écrire `open_in_first_window` (un `OpenTab` sur la première fenêtre de la vue) et `notice_of_active_tab` (un `Request::Snapshot` sur l'onglet actif via `Workspace::request`, puis `notice`) à partir des helpers existants. Vérifier que `saved_pdf` produit bien un deck neuf à chaque appel (nouveau `deck_uid`) ; sinon construire le second deck autrement. La forme exacte de la vue (`windows[].tabs[]`) est à reprendre de `views.rs`.

- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run -p oku-slides-session --test workspace` → échec de compilation.

- [ ] **Step 3 : `TabInfo.deck_uid`** — `views.rs` : `#[serde(default)] pub deck_uid: Option<String>` ; `Session::tab_info` : `deck_uid: Some(<document du deck de l'utilisateur>.sources.replica().deck_uid())` (en mode modèles, celui du deck mis de côté, comme `title` et `path`). Les onglets vierges (`pristine`) gardent leur `deck_uid` mais ne sont jamais proposés (étape 4).

- [ ] **Step 4 : `SameDeck`** — `files.rs` :

```rust
/// Le `deck_uid` du document `path` ; `None` pour un deck JSON.
pub fn deck_uid_of(storage: &Storage, path: &Path, limits: SourceLimits) -> Result<Option<String>> {
    let opened = read_document(storage, path, limits)?;
    Ok(opened.pdf.then(|| opened.sources.replica().deck_uid()))
}
```

`workspace.rs`, dans `Workspace::apply`, **avant** de prendre le verrou pour de bon (règle : pas de lecture de fichier sous le verrou de l'espace de travail) : prendre le verrou juste le temps de cloner `storage` et les limites, le relâcher, lire `deck_uid_of`, puis reprendre le verrou pour chercher, parmi les onglets **non vierges** dont `info.path` diffère de `path`, celui dont `info.deck_uid` est égal ; répondre `WorkspaceOutcome::Applied(json!({ "tab": id, "title": info.title }))`, ou `{ "tab": null, "title": null }`. Une erreur de lecture est rendue comme erreur (l'UI l'affiche, et n'ouvre pas). `src-tauri/src/commands.rs` `classify` : `WorkspaceRequest::SameDeck { .. } => (false, false)` (la réponse est rendue telle quelle). Le serveur `serve` n'a rien à changer.

- [ ] **Step 5 : `open_files`** — dans `workspace_resume.rs::open_files`, une fois la session du fichier construite (hors verrou), si un autre onglet non vierge de chemin différent porte le même `deck_uid`, poser sur la nouvelle session l'avis `notice-same-deck-open` (argument `title` : le titre de cet onglet) avec `Session::set_notice`. Garder l'ordre des verrous.

- [ ] **Step 6 : tests rouges (UI)** — dans `merge.test.ts` :

```ts
import { openGuarded } from "./merge";

describe("openGuarded", () => {
  const calls: string[] = [];
  const deps = (tab: string | null, choice: "merge" | "apart" | "cancel") => ({
    sameDeck: async () => ({ tab, title: "Roadmap" }),
    openTab: async () => void calls.push("open"),
    merge: async (t: string) => void calls.push(`merge:${t}`),
    activate: async (t: string) => void calls.push(`activate:${t}`),
    ask: async () => choice,
  });

  test("openGuarded without a tab holding the deck just opens", async () => {
    calls.length = 0;
    await openGuarded("w-1", "/d/x.oku-slides.pdf", deps(null, "merge"));
    expect(calls).toEqual(["open"]);
  });

  test("openGuarded and merge chosen merges into that tab then activates it", async () => {
    calls.length = 0;
    await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "merge"));
    expect(calls).toEqual(["merge:t-3", "activate:t-3"]);
  });

  test("openGuarded and apart chosen opens a new tab", async () => {
    calls.length = 0;
    await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "apart"));
    expect(calls).toEqual(["open"]);
  });

  test("openGuarded and cancel does nothing", async () => {
    calls.length = 0;
    await openGuarded("w-1", "/d/x.oku-slides.pdf", deps("t-3", "cancel"));
    expect(calls).toEqual([]);
  });
});
```

- [ ] **Step 7 : UI** — `api.ts` : `| { cmd: "same_deck"; path: string }` dans `WorkspaceRequest`, `sameDeck: (path: string) => ws<{ tab: string | null; title: string | null }>({ cmd: "same_deck", path })`. `merge.ts` :

```ts
export type SameDeckChoice = "merge" | "apart" | "cancel";

export interface OpenDeps {
  sameDeck: (path: string) => Promise<{ tab: string | null; title: string | null }>;
  openTab: (window: string, path: string) => Promise<unknown>;
  merge: (tab: string, path: string) => Promise<unknown>;
  activate: (tab: string) => Promise<unknown>;
  ask: (file: string, title: string) => Promise<SameDeckChoice>;
}

async function askSameDeck(file: string, title: string): Promise<SameDeckChoice> {
  const text = t("ui-same-deck-question", { file, title });
  if (!inTauri) return window.confirm(text) ? "merge" : "apart";
  const labels = { yes: t("ui-same-deck-merge"), no: t("ui-same-deck-apart"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const r = await message(text, { kind: "info", buttons: labels });
  if (r === labels.yes || r === "Yes") return "merge";
  if (r === labels.no || r === "No") return "apart";
  return "cancel";
}

/** Ouvre `path` ; s'il est une version d'un deck déjà ouvert, demande d'abord. */
export async function openGuarded(window: string, path: string, deps: OpenDeps): Promise<void> {
  const same = await deps.sameDeck(path);
  if (!same.tab) {
    await deps.openTab(window, path);
    return;
  }
  const choice = await deps.ask(fileName(path, path), same.title ?? "");
  if (choice === "merge") {
    await deps.merge(same.tab, path);
    await deps.activate(same.tab);
  } else if (choice === "apart") {
    await deps.openTab(window, path);
  }
}

/** Les dépendances réelles de `openGuarded`. */
export const openDeps: OpenDeps = {
  sameDeck: (path) => workspace.sameDeck(path),
  openTab: (window, path) => workspace.openTab(window, path),
  merge: (tab, path) => tabApi(tab).merge(path),
  activate: (tab) => workspace.activate(tab),
  ask: askSameDeck,
};
```

(importer `workspace`, `tabApi` depuis `./api` ; `openDeps` n'est pas utilisé par les tests.) `Workspace.tsx` : `const onOpen = (path: string) => void run(openGuarded(label, path, openDeps));` — vérifier que `run` accepte une promesse qui ne rend pas de vue ; sinon rafraîchir la vue après (`workspace.view()`), comme le fait déjà le code voisin.

- [ ] **Step 8 : Fluent** — `notice-same-deck-open` (`session.ftl`), `ui-same-deck-question`, `ui-same-deck-merge`, `ui-same-deck-apart` (`ui.ftl`), 20 locales (annexe A).

- [ ] **Step 9 : vérifier** — `cargo nextest run --workspace --exclude oku-slides-app`, `cargo build -p oku-slides-app`, `cd ui && bun test && bun run typecheck` → PASS.

- [ ] **Step 10 : commit**

```bash
git add crates src-tauri ui/src
git commit -m "feat(session) : ouvrir un deck déjà ouvert propose de le fusionner dans son onglet (t-240)"
```

---

### Task 7 : Documentation et vérification

**Files :**
- Modify : `docs/superpowers/specs/2026-09-27-sources-loro-design.md` (« Fusion », « Erreurs », « Schéma » : écarts 1 à 5 en tête de ce plan)
- Modify : `docs/adr/0002-sources-du-deck-en-crdt-loro.md` (conséquences : `peers` porte les noms d'auteurs, déjà annoncé ; un commit = un *change*)
- Modify : `crates/oku-slides-session/ARCHITECTURE.md` (≤ 50 lignes : `merge.rs`, `SameDeck`), `crates/oku-slides-core/ARCHITECTURE.md` (`peers`, `MergeReport`)

- [ ] **Step 1 : spec** — « Fusion » : déclencheurs tels que construits (écarts 1 à 3), avis (« modifications importées »), `SameDeck` ; « Erreurs » : session = `MergeUnrelated`, `HistoryNotOpen` (plan 3) ; `DiskReplaced` devient l'indicateur `disk_replaced` du snapshot ; « Schéma » : `peers` inscrit à la première édition d'un pair, nom tiré de `[author] name` ou de l'utilisateur système ; « Flux d'écriture » : un commit = un *change* Loro, libellé en message.
- [ ] **Step 2 : ARCHITECTURE et ADR** — comme listé ci-dessus, sans dépasser 50 lignes par ARCHITECTURE.
- [ ] **Step 3 : consolidation (G10)** — `grep -rn "fn merge_notice\|fn deck_uid_of\|fn saveGuarded\|fn openGuarded" crates ui/src` : une définition chacune ; aucune écriture de fichier sous le verrou de l'espace de travail.
- [ ] **Step 4 : vérification complète** — `just check` puis `just test` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add docs crates
git commit -m "docs(core) : fusion côté session, auteurs et changes dans la spec et l'architecture (t-241)"
```

---

## Annexe A : clés Fluent

`session.ftl` : `notice-merged` (`$file`, `$count`, `$authors`), `notice-merged-anonymous` (`$file`, `$count`), `notice-same-deck-open` (`$title`). `ui.ftl` : `ui-merge`, `ui-disk-replaced-question` (`$file`), `ui-disk-replaced-overwrite`, `ui-disk-replaced-save-as`, `ui-same-deck-question` (`$file`, `$title`), `ui-same-deck-merge`, `ui-same-deck-apart`. Le bouton d'annulation reprend `ui-close-cancel`. `notice-merged-anonymous` = `notice-merged` sans ` ({ $authors })`.

Ordre dans chaque ligne ci-dessous : notice-merged · notice-same-deck-open · ui-merge · ui-disk-replaced-question · ui-disk-replaced-overwrite · ui-disk-replaced-save-as · ui-same-deck-question · ui-same-deck-merge · ui-same-deck-apart.

**en** · Merged with { $file } — changes imported: { $count } ({ $authors }). · This presentation is already open in the tab “{ $title }”. To combine the two, use “Merge with a PDF…” there. · Merge with a PDF… · { $file } was replaced on disk by another presentation. · Overwrite · Save as… · { $file } is a version of the presentation open in the tab “{ $title }”. · Merge into that tab · Open separately

**fr** · Fusionné avec { $file } — modifications importées : { $count } ({ $authors }). · Cette présentation est déjà ouverte dans l'onglet « { $title } ». Pour réunir les deux, utilisez « Fusionner avec un PDF… » dans cet onglet. · Fusionner avec un PDF… · { $file } a été remplacé sur le disque par une autre présentation. · Écraser · Enregistrer sous… · { $file } est une version de la présentation ouverte dans l'onglet « { $title } ». · Fusionner dans cet onglet · Ouvrir à part

**de** · Mit { $file } zusammengeführt – übernommene Änderungen: { $count } ({ $authors }). · Diese Präsentation ist bereits im Tab „{ $title }“ geöffnet. Um beide zu vereinen, verwenden Sie dort „Mit PDF zusammenführen…“. · Mit PDF zusammenführen… · { $file } wurde auf dem Datenträger durch eine andere Präsentation ersetzt. · Überschreiben · Speichern unter… · { $file } ist eine Version der Präsentation, die im Tab „{ $title }“ geöffnet ist. · In diesen Tab zusammenführen · Separat öffnen

**es** · Fusionado con { $file }: cambios importados: { $count } ({ $authors }). · Esta presentación ya está abierta en la pestaña «{ $title }». Para unirlas, use «Fusionar con un PDF…» en esa pestaña. · Fusionar con un PDF… · { $file } fue reemplazado en el disco por otra presentación. · Sobrescribir · Guardar como… · { $file } es una versión de la presentación abierta en la pestaña «{ $title }». · Fusionar en esa pestaña · Abrir por separado

**it** · Unito con { $file } — modifiche importate: { $count } ({ $authors }). · Questa presentazione è già aperta nella scheda «{ $title }». Per unirle, usa «Unisci con un PDF…» in quella scheda. · Unisci con un PDF… · { $file } è stato sostituito sul disco da un'altra presentazione. · Sovrascrivi · Salva con nome… · { $file } è una versione della presentazione aperta nella scheda «{ $title }». · Unisci in quella scheda · Apri a parte

**pt** · Mesclado com { $file } — alterações importadas: { $count } ({ $authors }). · Esta apresentação já está aberta na guia “{ $title }”. Para juntá-las, use “Mesclar com um PDF…” nessa guia. · Mesclar com um PDF… · { $file } foi substituído no disco por outra apresentação. · Substituir · Salvar como… · { $file } é uma versão da apresentação aberta na guia “{ $title }”. · Mesclar nessa guia · Abrir separadamente

**nl** · Samengevoegd met { $file } — geïmporteerde wijzigingen: { $count } ({ $authors }). · Deze presentatie is al geopend in het tabblad ‘{ $title }’. Gebruik daar ‘Samenvoegen met een pdf…’ om ze te combineren. · Samenvoegen met een pdf… · { $file } is op de schijf vervangen door een andere presentatie. · Overschrijven · Opslaan als… · { $file } is een versie van de presentatie die open is in het tabblad ‘{ $title }’. · Samenvoegen in dat tabblad · Apart openen

**pl** · Scalono z { $file } — zaimportowane zmiany: { $count } ({ $authors }). · Ta prezentacja jest już otwarta na karcie „{ $title }”. Aby je połączyć, użyj tam „Scal z plikiem PDF…”. · Scal z plikiem PDF… · { $file } został zastąpiony na dysku inną prezentacją. · Nadpisz · Zapisz jako… · { $file } to wersja prezentacji otwartej na karcie „{ $title }”. · Scal w tej karcie · Otwórz osobno

**ru** · Объединено с { $file } — импортировано изменений: { $count } ({ $authors }). · Эта презентация уже открыта во вкладке «{ $title }». Чтобы объединить их, используйте там «Объединить с PDF…». · Объединить с PDF… · { $file } на диске заменён другой презентацией. · Перезаписать · Сохранить как… · { $file } — версия презентации, открытой во вкладке «{ $title }». · Объединить в этой вкладке · Открыть отдельно

**uk** · Об’єднано з { $file } — імпортовано змін: { $count } ({ $authors }). · Ця презентація вже відкрита у вкладці «{ $title }». Щоб об’єднати їх, скористайтеся там «Об’єднати з PDF…». · Об’єднати з PDF… · { $file } на диску замінено іншою презентацією. · Перезаписати · Зберегти як… · { $file } — версія презентації, відкритої у вкладці «{ $title }». · Об’єднати в цій вкладці · Відкрити окремо

**tr** · { $file } ile birleştirildi — içe aktarılan değişiklik: { $count } ({ $authors }). · Bu sunum zaten “{ $title }” sekmesinde açık. İkisini birleştirmek için o sekmede “PDF ile birleştir…” komutunu kullanın. · PDF ile birleştir… · { $file } diskte başka bir sunumla değiştirildi. · Üzerine yaz · Farklı kaydet… · { $file }, “{ $title }” sekmesinde açık olan sunumun bir sürümü. · O sekmede birleştir · Ayrı aç

**vi** · Đã hợp nhất với { $file } — số thay đổi đã nhập: { $count } ({ $authors }). · Bản trình bày này đã mở trong thẻ “{ $title }”. Để gộp hai bản, hãy dùng “Hợp nhất với PDF…” trong thẻ đó. · Hợp nhất với PDF… · { $file } đã bị thay trên ổ đĩa bằng một bản trình bày khác. · Ghi đè · Lưu thành… · { $file } là một phiên bản của bản trình bày đang mở trong thẻ “{ $title }”. · Hợp nhất vào thẻ đó · Mở riêng

**ja** · { $file } と統合しました — 取り込んだ変更: { $count }（{ $authors }）。 · このプレゼンテーションはタブ「{ $title }」ですでに開いています。統合するには、そのタブで「PDF と統合…」を使ってください。 · PDF と統合… · { $file } はディスク上で別のプレゼンテーションに置き換えられました。 · 上書き · 名前を付けて保存… · { $file } は、タブ「{ $title }」で開いているプレゼンテーションの別バージョンです。 · そのタブに統合 · 別に開く

**ko** · { $file }과(와) 병합했습니다 — 가져온 변경: { $count }({ $authors }). · 이 프레젠테이션은 이미 “{ $title }” 탭에 열려 있습니다. 합치려면 그 탭에서 “PDF와 병합…”을 사용하세요. · PDF와 병합… · { $file }이(가) 디스크에서 다른 프레젠테이션으로 바뀌었습니다. · 덮어쓰기 · 다른 이름으로 저장… · { $file }은(는) “{ $title }” 탭에 열려 있는 프레젠테이션의 다른 버전입니다. · 그 탭에 병합 · 따로 열기

**zh-Hans** · 已与 { $file } 合并 — 导入的更改：{ $count }（{ $authors }）。 · 此演示文稿已在标签页“{ $title }”中打开。如需合并，请在该标签页中使用“与 PDF 合并…”。 · 与 PDF 合并… · 磁盘上的 { $file } 已被另一个演示文稿替换。 · 覆盖 · 另存为… · { $file } 是标签页“{ $title }”中已打开演示文稿的一个版本。 · 合并到该标签页 · 单独打开

**zh-Hant** · 已與 { $file } 合併 — 匯入的變更：{ $count }（{ $authors }）。 · 此簡報已在分頁「{ $title }」中開啟。若要合併，請在該分頁使用「與 PDF 合併…」。 · 與 PDF 合併… · 磁碟上的 { $file } 已被另一份簡報取代。 · 覆寫 · 另存新檔… · { $file } 是分頁「{ $title }」中已開啟簡報的一個版本。 · 合併到該分頁 · 單獨開啟

**ar** · تم الدمج مع { $file } — التغييرات المستوردة: { $count } ({ $authors }). · هذا العرض التقديمي مفتوح بالفعل في علامة التبويب «{ $title }». لدمجهما، استخدم «دمج مع ملف PDF…» هناك. · دمج مع ملف PDF… · تم استبدال { $file } على القرص بعرض تقديمي آخر. · استبدال · حفظ باسم… · { $file } نسخة من العرض التقديمي المفتوح في علامة التبويب «{ $title }». · الدمج في علامة التبويب تلك · فتح بشكل منفصل

**hi** · { $file } के साथ मिलाया गया — आयात किए गए बदलाव: { $count } ({ $authors })। · यह प्रस्तुति पहले से टैब “{ $title }” में खुली है। दोनों को मिलाने के लिए वहाँ “PDF के साथ मिलाएँ…” का उपयोग करें। · PDF के साथ मिलाएँ… · डिस्क पर { $file } को किसी दूसरी प्रस्तुति से बदल दिया गया है। · अधिलेखित करें · इस रूप में सहेजें… · { $file } टैब “{ $title }” में खुली प्रस्तुति का एक संस्करण है। · उस टैब में मिलाएँ · अलग से खोलें

**bn** · { $file }-এর সঙ্গে একত্র করা হয়েছে — আমদানি করা পরিবর্তন: { $count } ({ $authors })। · এই উপস্থাপনাটি ইতিমধ্যে “{ $title }” ট্যাবে খোলা আছে। দুটি একত্র করতে সেখানে “PDF-এর সঙ্গে একত্র করুন…” ব্যবহার করুন। · PDF-এর সঙ্গে একত্র করুন… · ডিস্কে { $file } অন্য একটি উপস্থাপনা দিয়ে প্রতিস্থাপিত হয়েছে। · ওভাররাইট করুন · এভাবে সংরক্ষণ করুন… · { $file } হলো “{ $title }” ট্যাবে খোলা উপস্থাপনার একটি সংস্করণ। · ওই ট্যাবে একত্র করুন · আলাদাভাবে খুলুন

**th** · รวมกับ { $file } แล้ว — การเปลี่ยนแปลงที่นำเข้า: { $count } ({ $authors }) · งานนำเสนอนี้เปิดอยู่แล้วในแท็บ “{ $title }” หากต้องการรวม ให้ใช้ “รวมกับ PDF…” ในแท็บนั้น · รวมกับ PDF… · { $file } บนดิสก์ถูกแทนที่ด้วยงานนำเสนออื่น · เขียนทับ · บันทึกเป็น… · { $file } เป็นอีกเวอร์ชันหนึ่งของงานนำเสนอที่เปิดอยู่ในแท็บ “{ $title }” · รวมเข้ากับแท็บนั้น · เปิดแยก
