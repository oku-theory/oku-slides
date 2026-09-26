# Reprise de l'état de l'app — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** quitter puis relancer l'app ramène exactement au même état (deck même non enregistré, historique, slide courante, mode modèles, grille, fenêtre).

**Architecture:** la session écrit une copie de travail (`document.json` par document + assets/polices par empreinte + `state.json`) après chaque commande de `Session::handle`, sous `<data>/resume/`. L'app Tauri appelle `Session::resume` au lancement. L'UI envoie la slide courante et la grille via une nouvelle requête `ui`. La fenêtre passe par `tauri-plugin-window-state`.

**Tech Stack:** Rust (snafu, serde, speculoos, OpenDAL via `Storage`), Tauri 2, React 19 + Bun.

**Spec:** `docs/superpowers/specs/2026-09-26-resume-state-design.md`

**Tâche Oku :** t-81 (projet `slider`), sous-tâches t-82 à t-88 (une par tâche du plan, dans l'ordre). Chaque commit cite sa sous-tâche, ex. `(t-82)` ; `oku cli task done --project slider --id t-8x` à la fin de chaque tâche. Pas de ligne `Co-Authored-By`.

## Global Constraints

- Pas de `.unwrap()` / `.expect()` en production ; erreurs Snafu avec `backtrace`.
- Pas de `std::fs` en production : `Storage` (OpenDAL). `std::fs` autorisé dans les tests.
- Toute écriture de reprise passe par `Storage::write_atomic`.
- Un échec d'écriture de reprise est journalisé (`tracing::warn!`), jamais renvoyé à l'UI.
- Tests : side-by-side `*_tests.rs` inclus par `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`, noms `<unité>_<cas>_<résultat>`, `speculoos`.
- Texte utilisateur via Fluent ; toute clé ajoutée l'est dans les 20 locales (`every_locale_has_exactly_the_reference_keys`).
- Commentaires et docs en français, comme le code existant. Fichiers < 1000 lignes.
- Commandes : `cargo nextest run -p <crate>`, `just test`, `just check`, `bun test` (dans `ui/`).

---

### Task 1 : Historique sérialisable

**Files:**
- Modify: `crates/oku-slides-core/src/history.rs`
- Test: `crates/oku-slides-core/tests/ops.rs` (à côté des tests `history_*` existants)

**Interfaces:**
- Produces: `History: Serialize + Deserialize` ; `History::set_limit(&mut self, limit: usize)`.

- [ ] **Step 1 : tests qui échouent** — ajouter après `history_commit_with_failing_op_rolls_back_whole_transaction` :

```rust
#[test]
fn history_serde_round_trip_keeps_undo_and_redo() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut h = History::new(500);
    h.commit(&mut deck, "Tout", sample_ops()).unwrap();
    h.commit(&mut deck, "Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    h.undo(&mut deck).unwrap();
    let json = serde_json::to_string(&h).unwrap();
    let mut back: History = serde_json::from_str(&json).unwrap();
    assert_that!(back.undo_label()).is_equal_to(Some("Tout"));
    assert_that!(back.redo_label()).is_equal_to(Some("Thème"));
    back.undo(&mut deck).unwrap();
    assert_that!(deck).is_equal_to(&before);
}

#[test]
fn history_set_limit_drops_oldest_transactions() {
    let mut deck = fixture();
    let mut h = History::new(500);
    h.commit(&mut deck, "Un", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    h.commit(&mut deck, "Deux", vec![Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    h.set_limit(1);
    assert_that!(h.undo(&mut deck).unwrap()).is_equal_to(Some("Deux"));
    assert_that!(h.undo(&mut deck).unwrap()).is_none();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-core history_` → échec de compilation (`History: Serialize` absent, `set_limit` absent).

- [ ] **Step 3 : implémentation** dans `history.rs` :

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Transaction { /* inchangé */ }

/// Sérialisable : la reprise de l'app le garde d'un lancement à l'autre.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct History { /* inchangé */ }

// dans impl History :
    /// Garde au plus `limit` transactions de chaque côté ; les plus anciennes partent.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        while self.redo.len() > self.limit {
            self.redo.pop_front();
        }
    }
```

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-core` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core/src/history.rs crates/oku-slides-core/tests/ops.rs
git commit -m "feat(core) : historique sérialisable, limite ajustable (t-82)"
```

---

### Task 2 : État de l'interface dans la session

**Files:**
- Modify: `crates/oku-slides-session/src/views.rs` (type `UiState`, champ `Snapshot::ui`)
- Modify: `crates/oku-slides-session/src/lib.rs` (champ `Session::ui`, `Request::Ui`, remise à zéro de la slide)
- Test: `crates/oku-slides-session/tests/session.rs`

**Interfaces:**
- Produces: `pub struct UiState { pub slide: Option<String>, pub grid: bool }` (Serialize, Deserialize, Default → `grid: true`), réexporté depuis `lib.rs` ; `Request::Ui(UiState)` (JSON `{"cmd":"ui","slide":…,"grid":…}`) ; `Snapshot.ui: &UiState` ; `Session.ui: UiState`.

- [ ] **Step 1 : tests qui échouent** (dans `tests/session.rs`) :

```rust
fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap()
}

fn snapshot_json(s: &mut Session) -> serde_json::Value {
    serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap()
}

#[test]
fn ui_request_is_kept_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": "s-1", "grid": false}));
}

#[test]
fn snapshot_ui_defaults_to_grid_shown_and_no_slide() {
    let mut s = session();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": null, "grid": true}));
}

#[test]
fn new_deck_forgets_the_remembered_slide_but_keeps_grid() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    s.handle(req(json!({"cmd": "new"}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": null, "grid": false}));
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session ui` → échec (variante `ui` inconnue).

- [ ] **Step 3 : implémentation**

`views.rs` — importer `Deserialize` et ajouter :

```rust
/// Ce que l'interface fait garder à l'hôte : slide courante et grille.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub slide: Option<String>,
    /// Grille de la slide toujours visible.
    #[serde(default = "shown")]
    pub grid: bool,
}

fn shown() -> bool {
    true
}

impl Default for UiState {
    fn default() -> Self {
        Self { slide: None, grid: true }
    }
}
```

et dans `Snapshot`, après `editing` :

```rust
    /// Slide courante et grille, gardées pour la reprise.
    pub ui: &'a UiState,
```

`TemplateMode` : ajouter `Deserialize` à ses dérivés (utilisé en Task 3).

`lib.rs` :
- `pub use views::{…, UiState};`
- `Request` : variante

```rust
    /// Slide courante et grille de l'interface (gardées pour la reprise).
    Ui(UiState),
```

- `Session` : champ `ui: UiState,` initialisé à `UiState::default()` dans `new`.
- `handle` : `Request::Ui(ui) => self.ui = ui,` dans le bloc des commandes.
- `snapshot()` : `ui: &self.ui,`.
- Remplacer `load` par deux fonctions, et oublier la slide retenue quand le document change :

```rust
    /// Charge un document (déjà lu) dans la session.
    pub fn load(&mut self, sources: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        self.load_document(Document::new(sources, path, self.history_limit))
    }

    /// Charge un document ; ses graphiques sont mis sous forme canonique,
    /// pour que la grille puisse éditer leurs données.
    fn load_document(&mut self, mut doc: Document) -> Result<()> {
        charts::canonicalize_deck(&mut doc.sources.deck, &self.chart_labels);
        if doc.sources.fonts != self.doc.sources.fonts {
            // (bloc inchangé, avec `doc.sources.fonts`)
        }
        self.doc = doc;
        self.ui.slide = None;
        self.rerender()
    }
```

- `edit_templates` et `close_templates` : `self.ui.slide = None;` avant `self.rerender()`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session/src/views.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/tests/session.rs
git commit -m "feat(session) : requête ui, slide courante et grille dans le snapshot (t-83)"
```

---

### Task 3 : Stockage de reprise

**Files:**
- Create: `crates/oku-slides-session/src/resume.rs`
- Create: `crates/oku-slides-session/src/resume_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod resume;`, champ `Document::saved_hash`)
- Modify: `crates/oku-slides-session/src/files.rs` (`Opened::hash`)

**Interfaces:**
- Consumes: `History: Serialize/Deserialize`, `History::set_limit` (Task 1) ; `UiState`, `TemplateMode: Deserialize` (Task 2).
- Produces (tous `pub(crate)`, dans `resume.rs`) :
  - `const VERSION: u32 = 1; const DECK_DIR: &str = "deck"; const TEMPLATES_DIR: &str = "templates";`
  - `enum ResumeError { Access, Json, Version, Session }` (Snafu, backtrace)
  - `enum SavedMode { Deck, Templates(TemplateMode) }`
  - `struct State { version: u32, mode: SavedMode, ui: UiState }`
  - `struct Resume` avec `new(storage: Storage, dir: PathBuf)`, `write_state(&State)`, `read_state() -> Result<Option<State>>`, `write_doc(sub: &str, doc: &Document)`, `read_doc(sub: &str, history_limit: usize) -> Result<Document>`
  - `Document.saved_hash: Option<String>` ; `Opened.hash: String`

- [ ] **Step 1 : `Opened::hash` et `Document::saved_hash`** (pas encore de comportement ; préparation)

`files.rs` : champ `/// Empreinte du fichier lu (\`sha256:…\`).\n pub hash: String,` rempli par `oku_slides_core::asset_id(&bytes)` dans les deux branches de `read_document`.

`lib.rs` : `Document` gagne

```rust
    /// Empreinte du fichier à l'ouverture ou au dernier enregistrement : la
    /// reprise voit ainsi s'il a changé depuis.
    saved_hash: Option<String>,
```

(`None` dans `Document::new`). Dans `open` : `let Opened { sources, pdf, modified_elsewhere, hash } = …;` puis après `self.load(...)` : `self.doc.saved_hash = pdf.then_some(hash);`. Dans `save`, avant l'écriture : `let hash = oku_slides_core::asset_id(&out);`, et après : `self.doc.saved_hash = Some(hash);`.

- [ ] **Step 2 : tests qui échouent** — `resume_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use oku_slides_core::{Deck, Op, SourceBundle};

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!("oku-slides-resume-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)))
}

fn resume() -> Resume {
    Resume::new(Storage::local().unwrap(), temp_dir())
}

fn deck() -> Deck {
    Deck::from_json(r#"{"theme":"papier","slides":[]}"#).unwrap()
}

fn document() -> Document {
    let mut sources = SourceBundle::new(deck());
    sources.add_asset(b"image".to_vec());
    sources.fonts.insert("Inter.ttf".into(), b"police".to_vec());
    let mut doc = Document::new(sources, Some(PathBuf::from("/tmp/essai.oku-slides.pdf")), 10);
    doc.history.commit(&mut doc.sources.deck, "Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    doc.dirty = true;
    doc.saved_hash = Some("sha256:00".into());
    doc
}

#[test]
fn write_doc_then_read_doc_round_trips_sources_history_and_meta() {
    let r = resume();
    let doc = document();
    r.write_doc(DECK_DIR, &doc).unwrap();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources).is_equal_to(&doc.sources);
    assert_that!(back.history.undo_label()).is_equal_to(Some("Thème"));
    assert_that!(back.path).is_equal_to(&doc.path);
    assert_that!(back.dirty).is_true();
    assert_that!(back.saved_hash).is_equal_to(&doc.saved_hash);
    assert_that!(back.notice).is_none();
}

#[test]
fn write_doc_removes_orphan_assets() {
    let r = resume();
    let mut doc = document();
    r.write_doc(DECK_DIR, &doc).unwrap();
    doc.sources.assets.clear();
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().sources.assets.len()).is_equal_to(0);
}

#[test]
fn read_doc_with_corrupt_history_keeps_deck_with_empty_history() {
    let r = resume();
    r.write_doc(DECK_DIR, &document()).unwrap();
    let path = r.dir.join(DECK_DIR).join("document.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["history"] = serde_json::json!("illisible");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources.deck.theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(back.history.can_undo()).is_false();
}

#[test]
fn read_state_missing_returns_none() {
    assert_that!(resume().read_state().unwrap()).is_none();
}

#[test]
fn write_state_then_read_state_round_trips_mode_and_ui() {
    let r = resume();
    let edit = TemplateMode { set: "essentiel".into(), name: "Essentiel".into(), builtin: true };
    let ui = UiState { slide: Some("s-1".into()), grid: false };
    r.write_state(&State { version: VERSION, mode: SavedMode::Templates(edit), ui: ui.clone() }).unwrap();
    let back = r.read_state().unwrap().unwrap();
    assert_that!(matches!(back.mode, SavedMode::Templates(ref m) if m.set == "essentiel")).is_true();
    assert_that!(back.ui).is_equal_to(ui);
}

#[test]
fn read_state_unknown_version_returns_error() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), br#"{"version":99,"mode":"deck"}"#).unwrap();
    assert_that!(r.read_state()).is_err();
}
```

- [ ] **Step 3 :** `cargo nextest run -p oku-slides-session resume` → échec de compilation (module absent).

- [ ] **Step 4 : implémentation** — `resume.rs` :

```rust
//! Reprise : l'état de la session, écrit à chaque modification sous
//! `<data>/resume/`, relu au lancement suivant.
//!
//! `state.json` dit le mode et l'état de l'interface ; chaque document
//! (`deck/`, `templates/`) tient dans un `document.json` (deck, historique,
//! fichier) écrit d'un coup, plus ses assets et polices, écrits une fois par
//! nom.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

use oku_slides_core::{Deck, History, SourceBundle};
use oku_slides_platform::storage::{Storage, StorageError};

use crate::views::{TemplateMode, UiState};
use crate::{Document, SessionError};

pub(crate) const VERSION: u32 = 1;
pub(crate) const DECK_DIR: &str = "deck";
pub(crate) const TEMPLATES_DIR: &str = "templates";

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ResumeError {
    #[snafu(display("cannot access {}", path.display()))]
    Access { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("unreadable {}", path.display()))]
    Json { path: PathBuf, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unsupported resume state version {version}"))]
    Version { version: u32, backtrace: Backtrace },
    #[snafu(display("cannot restore the session"))]
    Session { source: SessionError, backtrace: Backtrace },
}

type Result<T, E = ResumeError> = std::result::Result<T, E>;

/// Ce que la session éditait.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedMode {
    Deck,
    Templates(TemplateMode),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct State {
    pub version: u32,
    pub mode: SavedMode,
    #[serde(default)]
    pub ui: UiState,
}

#[derive(Serialize)]
struct DocOut<'a> {
    path: Option<&'a Path>,
    dirty: bool,
    saved_hash: Option<&'a str>,
    deck: &'a Deck,
    history: &'a History,
}

#[derive(Deserialize)]
struct DocIn {
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    saved_hash: Option<String>,
    deck: Deck,
    /// Lu à part : un historique illisible ne coûte pas le deck.
    #[serde(default)]
    history: serde_json::Value,
}

/// Le dossier de reprise et ses fichiers.
#[derive(Debug)]
pub(crate) struct Resume {
    storage: Storage,
    pub(crate) dir: PathBuf,
}

impl Resume {
    pub fn new(storage: Storage, dir: PathBuf) -> Self {
        Self { storage, dir }
    }

    fn put(&self, path: &Path, bytes: Vec<u8>) -> Result<()> {
        self.storage.write_atomic(path, bytes).context(AccessSnafu { path })
    }

    fn get(&self, path: &Path) -> Result<Vec<u8>> {
        self.storage.read(path).context(AccessSnafu { path })
    }

    pub fn write_state(&self, state: &State) -> Result<()> {
        let path = self.dir.join("state.json");
        let bytes = serde_json::to_vec(state).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)
    }

    /// L'état du dernier lancement ; `None` au premier.
    pub fn read_state(&self) -> Result<Option<State>> {
        let path = self.dir.join("state.json");
        if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
            return Ok(None);
        }
        let state: State = serde_json::from_slice(&self.get(&path)?).context(JsonSnafu { path: &path })?;
        ensure!(state.version == VERSION, VersionSnafu { version: state.version });
        Ok(Some(state))
    }

    /// Écrit un document : assets et polices d'abord (le deck les cite), puis
    /// `document.json` d'un coup.
    pub fn write_doc(&self, sub: &str, doc: &Document) -> Result<()> {
        let dir = self.dir.join(sub);
        let assets = doc.sources.assets.iter().map(|(id, b)| (id.strip_prefix("sha256:").unwrap_or(id), b));
        self.sync_files(&dir.join("assets"), assets)?;
        self.sync_files(&dir.join("fonts"), doc.sources.fonts.iter().map(|(n, b)| (n.as_str(), b)))?;
        let path = dir.join("document.json");
        let out = DocOut {
            path: doc.path.as_deref(),
            dirty: doc.dirty,
            saved_hash: doc.saved_hash.as_deref(),
            deck: &doc.sources.deck,
            history: &doc.history,
        };
        let bytes = serde_json::to_vec(&out).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)
    }

    /// Écrit les fichiers absents du dossier et supprime ceux qui n'y sont plus.
    fn sync_files<'a>(&self, dir: &Path, files: impl Iterator<Item = (&'a str, &'a Vec<u8>)>) -> Result<()> {
        let mut keep = BTreeSet::new();
        for (name, bytes) in files {
            // Noms déjà validés à la lecture des sources ; par prudence, un nom
            // qui sortirait du dossier est ignoré.
            if Path::new(name).file_name().is_none_or(|n| n != name) {
                continue;
            }
            let path = dir.join(name);
            if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
                self.put(&path, bytes.clone())?;
            }
            keep.insert(name);
        }
        for path in self.storage.list_files(dir).context(AccessSnafu { path: dir })? {
            let stale = path.file_name().is_some_and(|n| !keep.contains(&*n.to_string_lossy()));
            if stale {
                self.storage.delete(&path).context(AccessSnafu { path: &path })?;
            }
        }
        Ok(())
    }

    fn read_files(&self, dir: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut files = BTreeMap::new();
        for path in self.storage.list_files(dir).context(AccessSnafu { path: dir })? {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !name.is_empty() && !name.starts_with('.') {
                files.insert(name, self.get(&path)?);
            }
        }
        Ok(files)
    }

    /// Relit un document ; son historique garde au plus `history_limit` transactions.
    pub fn read_doc(&self, sub: &str, history_limit: usize) -> Result<Document> {
        let dir = self.dir.join(sub);
        let path = dir.join("document.json");
        let raw: DocIn = serde_json::from_slice(&self.get(&path)?).context(JsonSnafu { path: &path })?;
        let mut history = serde_json::from_value::<History>(raw.history).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "historique de reprise illisible : historique vide");
            History::new(history_limit)
        });
        history.set_limit(history_limit);
        let assets = self.read_files(&dir.join("assets"))?.into_iter().map(|(n, b)| (format!("sha256:{n}"), b)).collect();
        let fonts = self.read_files(&dir.join("fonts"))?;
        Ok(Document {
            sources: SourceBundle { deck: raw.deck, assets, fonts },
            history,
            path: raw.path,
            dirty: raw.dirty,
            notice: None,
            saved_hash: raw.saved_hash,
        })
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
```

`lib.rs` : `mod resume;` dans la liste des modules. Si `Document` doit être nommé depuis `resume.rs` : il est privé à la racine du crate, donc visible des modules enfants — `use crate::Document;` fonctionne tel quel.

- [ ] **Step 5 :** `cargo nextest run -p oku-slides-session` → tout passe ; `cargo clippy -p oku-slides-session --all-targets -- -D warnings` → propre (le code mort `Session`/`TEMPLATES_DIR` est utilisé en Task 4 : si clippy signale `dead_code`, ajouter provisoirement `#[expect(dead_code, reason = "branché en Task 4")]` sur l'élément, retiré en Task 4).
- [ ] **Step 6 : commit**

```bash
git add crates/oku-slides-session/src/resume.rs crates/oku-slides-session/src/resume_tests.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/src/files.rs
git commit -m "feat(session) : stockage de reprise, documents et état (t-84)"
```

---

### Task 4 : Reprise de la session

**Files:**
- Modify: `crates/oku-slides-session/src/resume.rs` (bloc `impl Session`)
- Modify: `crates/oku-slides-session/src/lib.rs` (champ `resume`, écriture dans `handle`)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 locales)
- Test: `crates/oku-slides-session/tests/session.rs`

**Interfaces:**
- Consumes: `Resume`, `State`, `SavedMode`, `DECK_DIR`, `TEMPLATES_DIR`, `ResumeError` (Task 3) ; `load_document`, `UiState` (Task 2).
- Produces: `pub fn Session::resume(&mut self, dir: PathBuf, file: Option<&Path>)` (appelé par l'app Tauri, Task 5).

- [ ] **Step 1 : helper de test** — dans `tests/session.rs`, extraire le dossier de données :

```rust
fn data_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oku-slides-session-{}-{}", std::process::id(), unique()))
}

/// Une session sur le dossier de données `data`.
fn session_in(data: &std::path::Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
}

/// Une session isolée : son propre dossier de données, vide.
fn session() -> Session {
    session_in(&data_dir())
}

/// Une session qui reprend l'état laissé dans `data`.
fn resumed(data: &std::path::Path, file: Option<&std::path::Path>) -> Session {
    let mut s = session_in(data);
    s.resume(data.join("resume"), file);
    s
}

fn set_theme(theme: &str) -> Request {
    req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]}))
}
```

- [ ] **Step 2 : tests qui échouent**

```rust
#[test]
fn resume_after_edits_restores_deck_history_and_ui() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    let before = snapshot_json(&mut a);
    drop(a);
    let mut b = resumed(&data, None);
    assert_that!(snapshot_json(&mut b)).is_equal_to(&before);
    b.undo().unwrap();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("papier"));
}

#[test]
fn resume_while_editing_templates_restores_mode_and_stashed_deck() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}))).unwrap();
    drop(a);
    let mut b = resumed(&data, None);
    assert_that!(snapshot_json(&mut b)["editing"]["set"]).is_equal_to(json!(oku_slides_session::DEFAULT_SET));
    b.handle(req(json!({"cmd": "close_templates"}))).unwrap();
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
}

#[test]
fn resume_saved_deck_modified_elsewhere_reopens_from_disk() {
    let data = data_dir();
    let file = data.join("essai.oku-slides.pdf");
    let mut a = resumed(&data, None);
    a.handle(req(json!({"cmd": "save", "path": file}))).unwrap();
    drop(a);
    // Un autre lancement (sans reprise) change le fichier.
    let mut other = session();
    other.open(&file).unwrap();
    other.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    other.save(Some(&file)).unwrap();
    let mut b = resumed(&data, None);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["can_undo"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_corrupt_state_starts_blank() {
    let data = data_dir();
    std::fs::create_dir_all(data.join("resume")).unwrap();
    std::fs::write(data.join("resume/state.json"), b"pas du json").unwrap();
    let mut b = resumed(&data, None);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["slides"].as_array().unwrap().len()).is_equal_to(2);
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_file_and_unsaved_work_keeps_work_with_notice() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    drop(a);
    let mut b = resumed(&data, Some(&fixture_path()));
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["notice"].as_str()).is_some();
}

#[test]
fn resume_with_file_and_no_unsaved_work_opens_file() {
    let data = data_dir();
    drop(resumed(&data, None));
    let mut b = resumed(&data, Some(&fixture_path()));
    assert_that!(snapshot_json(&mut b)["slides"].as_array().unwrap().len()).is_equal_to(8);
}

#[test]
fn session_without_resume_writes_nothing() {
    let data = data_dir();
    let mut s = session_in(&data);
    s.handle(set_theme("pro")).unwrap();
    assert_that!(data.join("resume").exists()).is_false();
}
```

- [ ] **Step 3 :** `cargo nextest run -p oku-slides-session resume_` → échec (`Session::resume` absent).

- [ ] **Step 4 : clé Fluent** — dans les 20 `crates/oku-slides-i18n/src/locales/<l>/session.ftl`, juste après `notice-modified-elsewhere` (garder le registre tu/vous du fichier) :

| locale | `notice-resume-kept-unsaved = …` |
|---|---|
| en | Your unsaved changes from last time were kept: the file you asked to open was not opened. |
| fr | Vos modifications non enregistrées de la dernière fois ont été gardées : le fichier demandé n'a pas été ouvert. |
| de | Deine nicht gespeicherten Änderungen vom letzten Mal wurden behalten: Die angeforderte Datei wurde nicht geöffnet. *(Sie-Form si le fichier vouvoie)* |
| es | Se conservaron tus cambios sin guardar de la última vez: no se abrió el archivo solicitado. |
| it | Le modifiche non salvate dell'ultima volta sono state mantenute: il file richiesto non è stato aperto. |
| pt | Suas alterações não salvas da última vez foram mantidas: o arquivo solicitado não foi aberto. |
| nl | Je niet-opgeslagen wijzigingen van de vorige keer zijn bewaard: het gevraagde bestand is niet geopend. |
| pl | Zachowano niezapisane zmiany z poprzedniego razu: żądany plik nie został otwarty. |
| ru | Ваши несохранённые изменения с прошлого раза восстановлены: запрошенный файл не открыт. |
| uk | Ваші незбережені зміни з минулого разу відновлено: запитаний файл не відкрито. |
| tr | Geçen seferki kaydedilmemiş değişiklikleriniz korundu: istenen dosya açılmadı. |
| ar | تم الاحتفاظ بتعديلاتك غير المحفوظة من المرة السابقة: لم يُفتح الملف المطلوب. |
| hi | पिछली बार के बिना सहेजे गए बदलाव रखे गए हैं: माँगी गई फ़ाइल नहीं खोली गई। |
| bn | গতবারের অসংরক্ষিত পরিবর্তনগুলো রাখা হয়েছে: অনুরোধ করা ফাইলটি খোলা হয়নি। |
| th | เก็บการเปลี่ยนแปลงที่ยังไม่ได้บันทึกจากครั้งก่อนไว้แล้ว: ไม่ได้เปิดไฟล์ที่ขอ |
| vi | Các thay đổi chưa lưu từ lần trước đã được giữ lại: tệp được yêu cầu chưa được mở. |
| ja | 前回の未保存の変更を保持しました。指定されたファイルは開いていません。 |
| ko | 지난번에 저장하지 않은 변경 사항을 유지했습니다. 요청한 파일은 열지 않았습니다. |
| zh-Hans | 已保留上次未保存的更改：未打开所请求的文件。 |
| zh-Hant | 已保留上次未儲存的變更：未開啟所要求的檔案。 |

- [ ] **Step 5 : implémentation**

`lib.rs` :
- `Session` gagne `/// Reprise activée (app de bureau) : l'état est écrit à chaque commande.\n resume: Option<resume::Resume>,` (`None` dans `new`).
- `handle` : avant le `match`,

```rust
        // Tout sauf la lecture du snapshot change l'état ; `ui` ne touche pas aux documents.
        let (changes, documents) = (!matches!(req, Request::Snapshot), !matches!(req, Request::Snapshot | Request::Ui(_)));
```

  et après le `match`, avant `json(&self.snapshot())` : `if changes { self.persist(documents); }`.

`resume.rs` — ajouter :

```rust
use oku_slides_i18n::Localize as _; // si nécessaire pour `tr`
use crate::files::read_document;
use crate::{Mode, Session};

impl Session {
    /// Active la reprise dans `dir` : relit l'état du dernier lancement, puis
    /// ouvre `file` (« ouvrir avec… »), sauf s'il y a du travail non
    /// enregistré : il passe avant, et un avis le dit.
    pub fn resume(&mut self, dir: PathBuf, file: Option<&Path>) {
        let resume = Resume::new(self.storage.clone(), dir);
        if let Err(e) = self.restore(&resume) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise impossible : deck vierge");
            if let Err(e) = self.new_deck() {
                tracing::warn!(error = %e.one_line(), "deck vierge");
            }
        }
        if let Some(file) = file {
            self.open_at_launch(file);
        }
        self.resume = Some(resume);
        self.persist(true);
    }

    fn restore(&mut self, resume: &Resume) -> Result<()> {
        let Some(state) = resume.read_state()? else { return Ok(()) };
        let deck = self.fresh_from_disk(resume.read_doc(DECK_DIR, self.history_limit)?);
        let templates = match state.mode {
            SavedMode::Deck => None,
            SavedMode::Templates(edit) => match resume.read_doc(TEMPLATES_DIR, self.history_limit) {
                Ok(doc) => Some((doc, edit)),
                Err(e) => {
                    tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "modèles de reprise illisibles");
                    None
                }
            },
        };
        self.load_document(deck).context(SessionSnafu)?;
        if let Some((doc, edit)) = templates {
            let stash = std::mem::replace(&mut self.doc, doc);
            self.mode = Mode::Templates { stash: Box::new(stash), edit };
            self.rerender().context(SessionSnafu)?;
        }
        self.ui = state.ui;
        Ok(())
    }

    /// Un document non modifié dont le fichier a changé depuis est relu sur
    /// le disque, sans historique ; absent ou illisible, la copie de travail reste.
    fn fresh_from_disk(&self, saved: Document) -> Document {
        let Some(path) = saved.path.as_ref().filter(|_| !saved.dirty) else { return saved };
        match read_document(&self.storage, path) {
            Ok(opened) if saved.saved_hash.as_deref() != Some(opened.hash.as_str()) => {
                let mut doc = Document::new(opened.sources, Some(path.clone()), self.history_limit);
                doc.saved_hash = Some(opened.hash);
                doc.notice = opened.modified_elsewhere.then(|| self.localizer.tr("notice-modified-elsewhere"));
                doc
            }
            _ => saved,
        }
    }

    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours.
    fn unsaved(&self) -> bool {
        self.doc.dirty || matches!(&self.mode, Mode::Templates { stash, .. } if stash.dirty)
    }

    fn open_at_launch(&mut self, file: &Path) {
        if self.unsaved() {
            self.doc.notice = Some(self.localizer.tr("notice-resume-kept-unsaved"));
            return;
        }
        let opened = self.close_templates().and_then(|()| self.open(file));
        if let Err(e) = opened {
            tracing::warn!(path = %file.display(), error = %e.one_line(), "ouverture au démarrage");
        }
    }

    /// Écrit l'état pour la reprise ; `documents` : aussi les documents.
    pub(crate) fn persist(&self, documents: bool) {
        let Some(resume) = &self.resume else { return };
        if let Err(e) = self.write_resume(resume, documents) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "état de reprise non enregistré");
        }
    }

    fn write_resume(&self, resume: &Resume, documents: bool) -> Result<()> {
        let (deck, mode) = match &self.mode {
            Mode::Deck => (&self.doc, SavedMode::Deck),
            Mode::Templates { stash, edit } => (&**stash, SavedMode::Templates(edit.clone())),
        };
        if documents {
            resume.write_doc(DECK_DIR, deck)?;
            if matches!(self.mode, Mode::Templates { .. }) {
                resume.write_doc(TEMPLATES_DIR, &self.doc)?;
            }
        }
        resume.write_state(&State { version: VERSION, mode, ui: self.ui.clone() })
    }
}
```

Notes pour l'implémenteur :
- `close_templates` en mode deck ne fait qu'un rendu : sans effet, d'où l'enchaînement sans condition.
- `Mode`, `Document`, `load_document`, `rerender`, `new_deck`, champs privés : visibles depuis le module enfant `resume`.
- `SessionError::one_line()` existe (utilisé dans `main.rs`).
- Retirer les `#[expect(dead_code…)]` éventuels de la Task 3.

- [ ] **Step 6 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` → tout passe (dont `every_locale_has_exactly_the_reference_keys`).
- [ ] **Step 7 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n/src/locales
git commit -m "feat(session) : reprise de l'état à chaque commande et au lancement (t-85)"
```

---

### Task 5 : App Tauri — reprise au lancement et fenêtre

**Files:**
- Modify: `Cargo.toml` (workspace : `tauri-plugin-window-state = "2"`)
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-window-state.workspace = true`)
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `Session::resume(dir, file)` (Task 4).

- [ ] **Step 1 :** vérifier l'API du plugin (context7 / docs.rs `tauri-plugin-window-state` 2.x) : `Builder::default().build()`, trait `AppHandleExt::save_window_state(StateFlags)`.
- [ ] **Step 2 : implémentation** — `main.rs` :

```rust
use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
```

remplacer le bloc « Un fichier passé en argument… » par :

```rust
            // L'état du dernier lancement ; un fichier passé en argument (ouvrir
            // avec…) est ouvert, sauf travail non enregistré.
            let file = std::env::args_os().nth(1).map(PathBuf::from).filter(|p| p.exists());
            session.resume(platform.settings.paths.data().join("resume"), file.as_deref());
```

et dans le builder :

```rust
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_window_event(|window, event| {
            // Taille et position gardées à chaque changement, pas seulement à la fermeture.
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_))
                && let Err(e) = window.app_handle().save_window_state(StateFlags::all())
            {
                tracing::warn!(error = %e, "état de la fenêtre non enregistré");
            }
        })
```

Mettre à jour le doc de module : « Démarrage : journaux → plateforme → session (reprise de l'état) et dictée ; la fenêtre reprend sa taille et sa position. »

- [ ] **Step 3 :** `cargo build -p oku-slides-app` et `cargo clippy -p oku-slides-app -- -D warnings` → propres.
- [ ] **Step 4 : commit**

```bash
git add Cargo.toml Cargo.lock src-tauri/Cargo.toml src-tauri/src/main.rs
git commit -m "feat(app) : reprise au lancement, taille et position de la fenêtre (t-86)"
```

---

### Task 6 : UI — slide courante et grille gardées par l'hôte

**Files:**
- Modify: `ui/src/api.ts`
- Modify: `ui/src/deck.ts`, Test: `ui/src/deck.test.ts`
- Modify: `ui/src/App.tsx`

**Interfaces:**
- Consumes: requête `{cmd:"ui", slide, grid}` et `Snapshot.ui` (Task 2).
- Produces: `api.ui(slide, grid)`, `pickSlide(slides, current, remembered)`.

- [ ] **Step 1 : tests qui échouent** — `deck.test.ts` (importer `pickSlide`) :

```ts
describe("pickSlide", () => {
  const slides = [{ id: "a" }, { id: "b" }, { id: "c" }];
  test("pickSlide courante présente la garde", () => {
    expect(pickSlide(slides, "b", "c")).toBe("b");
  });
  test("pickSlide sans courante prend la slide retenue", () => {
    expect(pickSlide(slides, null, "c")).toBe("c");
  });
  test("pickSlide retenue disparue prend la première", () => {
    expect(pickSlide(slides, null, "z")).toBe("a");
  });
  test("pickSlide deck vide renvoie null", () => {
    expect(pickSlide([], null, "a")).toBeNull();
  });
});
```

- [ ] **Step 2 :** `cd ui && bun test deck` → échec (`pickSlide` absent).
- [ ] **Step 3 : implémentation**

`deck.ts` (à côté de `slideToFetch`) :

```ts
/** Slide à montrer : la courante si elle existe encore, sinon celle retenue par l'hôte, sinon la première. */
export function pickSlide(slides: { id: string }[], current: string | null, remembered: string | null): string | null {
  const has = (id: string | null): id is string => id !== null && slides.some((s) => s.id === id);
  if (has(current)) return current;
  if (has(remembered)) return remembered;
  return slides[0]?.id ?? null;
}
```

`api.ts` :

```ts
/** Slide courante et grille, gardées par l'hôte d'un lancement à l'autre. */
export interface UiState {
  slide: string | null;
  grid: boolean;
}
```

`Snapshot` gagne `ui: UiState;` ; `Request` gagne `| { cmd: "ui"; slide: string | null; grid: boolean }` ; `api` gagne `ui: (slide: string | null, grid: boolean) => call<Snapshot>({ cmd: "ui", slide, grid }),`.

`App.tsx` :
- importer `pickSlide` depuis `./deck`.
- remplacer l'effet « Slide courante : la première si l'actuelle a disparu » par :

```tsx
  // Au premier snapshot, l'interface reprend la slide et la grille gardées par
  // l'hôte ; ensuite, la slide courante, ou la première si elle a disparu.
  const restored = useRef(false);
  useEffect(() => {
    if (!snap) return;
    if (!restored.current) {
      restored.current = true;
      setGridAlways(snap.ui.grid);
    }
    const next = pickSlide(snap.slides, current, snap.ui.slide);
    if (next !== current) setCurrent(next);
  }, [snap, current]);
```

- remplacer l'initialisation `localStorage` de la grille et son effet par :

```tsx
  const [gridAlways, setGridAlways] = useState(true);
  // Slide courante et grille : l'hôte les garde pour le prochain lancement.
  useEffect(() => {
    if (!restored.current) return;
    api.ui(current, gridAlways).catch(() => {
      /* reprise facultative : l'édition continue */
    });
  }, [current, gridAlways]);
```

  (déclarer `gridAlways` **avant** l'effet de reprise, qui appelle `setGridAlways` ; déplacer le bloc si nécessaire.)

- [ ] **Step 4 :** `cd ui && bun test && bunx tsc --noEmit` → tout passe.
- [ ] **Step 5 : commit**

```bash
git add ui/src/api.ts ui/src/deck.ts ui/src/deck.test.ts ui/src/App.tsx
git commit -m "feat(ui) : slide courante et grille gardées par l'hôte (t-87)"
```

---

### Task 7 : Vérification finale

- [ ] `just check` (fmt, clippy `-D warnings`, tsc) → propre.
- [ ] `just test` (nextest + bun) → tout passe.
- [ ] `just e2e` → les tests Playwright passent (la requête `ui` fonctionne aussi via `oku-slides serve`).
- [ ] Essai réel de l'app : lancer, changer de thème, aller à la slide 3, masquer la grille, déplacer la fenêtre, quitter ; relancer → même deck (non enregistré), même slide, grille masquée, Ctrl+Z annule le thème, fenêtre au même endroit. Vérifier aussi `~/Library/Application Support/me.dartiguenave.oku-slides/resume/`.
- [ ] Consolidation (G10) : pas de doublon de helper (`temp_dir`, hash) ; `localStorage` n'est plus utilisé pour la grille (`grep -n localStorage ui/src`).
- [ ] `oku cli task done --project slider --id t-88` puis `--id t-81` une fois tout vert.
