# Packs and themes as tabs — lot 1: library drafts and adoption — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A library version becomes frozen once a deck pins it; until then a save replaces it in place, after that a save opens a child version — decided atomically.

**Architecture:** A new child module of `library.rs`, `library_draft.rs`, adds an `adopted` marker next to `received`, a process-wide draft lock (`DraftLock`, a proof token passed by reference), `Library::draft_write` (decide), `Library::save_draft` (write) and `Library::adopt` (freeze). Today's pack mode saves through it, rebasing its files onto a frozen draft; a deck in deck mode freezes every library version it pins (new `deck_adopt.rs`), and closing the pack mode freezes the version the deck now pins.

**Tech Stack:** Rust 2024, snafu, speculoos, rstest, cargo-nextest, OpenDAL `Storage`.

**Spec:** `docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md` (section "Versions and adoption", lot 1). Oku: t-1152 › t-1153 (lot 1) › t-1158, t-1159, t-1160.

**Scope of lot 1, versus the spec:** the library primitives serve both packs and themes (tested
for both), and decks freeze their theme pins. Saving a *theme* draft through `save_draft` comes
with `Mode::Theme` in lot 3: today's theme editor adds a new version per save and never replaces
one, so it cannot rewrite an adopted version meanwhile. Keeping the draft in `state.json` is lot 3 too.

## Global Constraints

- Commits: `type(scope) : description (t-N)`, in French, a space before the colon; never a `Co-Authored-By` line. `git add` in its own call, then `git diff --cached --stat` right before `git commit -F <message file in the scratchpad>` (other sessions stage on `main`).
- No `unwrap`/`expect` outside tests; no silent fallback; lock with `lock().unwrap_or_else(PoisonError::into_inner)`.
- No `std::fs` in production code: go through `Storage` (`write_atomic`, `len`).
- Every pub or pub(crate) item and new module documented, first sentence under 15 words. Code comments in French.
- Unit tests in a sibling `x_tests.rs` included by `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`, speculoos assertions, names `unit_scenario_result`, one behavior per test, rstest for tables, temp dirs from `test_support::fresh_dir` (tempfile).
- Source files: aim 500 lines, never above 1000 (`library.rs` is 639: new code goes in `library_draft.rs`).
- `crates/oku-slides-session/ARCHITECTURE.md` stays at most 50 lines (it is at 50: edit a line, add none).
- Every run of the app outside `just e2e` must set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir (no app run is needed in this lot).
- After each task: `oku cli task done --project slider --id <sub-task>`.

## File Structure

- Create `crates/oku-slides-session/src/library_draft.rs` — draft lock, `DraftWrite`, `is_frozen`, `draft_write`, `save_draft`, `adopt` (child module of `library`, sees its private items).
- Create `crates/oku-slides-session/src/library_draft_tests.rs` — library-level tests, both pack kinds and themes.
- Modify `crates/oku-slides-session/src/library.rs` — declare the child module, re-export, generalize the marker read (`is_marked`).
- Modify `crates/oku-slides-session/src/library_packs.rs` — `PackLibrary` wrappers; drop `replace_version` (no caller left).
- Modify `crates/oku-slides-session/src/library_tests.rs` — the two `replace_version` tests go through `save_draft`.
- Modify `crates/oku-slides-session/src/pack_library.rs` — `write_session_version` takes the lock and a `DraftWrite`; add `rebased`.
- Modify `crates/oku-slides-session/src/pack_mode_save.rs` — `adopt_session_version` decides under the lock.
- Test `crates/oku-slides-session/src/pack_mode_save_tests.rs`.
- Create `crates/oku-slides-session/src/deck_adopt.rs` + `deck_adopt_tests.rs` — `Session::adopt_pins`.
- Modify `crates/oku-slides-session/src/deck_packs.rs` (`refresh_pack_updates` calls `adopt_pins`), `pack_mode.rs` (`close_pack` calls it), `lib.rs` (module list).
- Modify `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-session/AGENTS.md`.

---

### Task 1: Draft lock, `save_draft` and `adopt` in the library (t-1158)

**Files:**
- Create: `crates/oku-slides-session/src/library_draft.rs`
- Create: `crates/oku-slides-session/src/library_draft_tests.rs`
- Modify: `crates/oku-slides-session/src/library.rs` (marker read near line 268, module declarations near line 630)
- Modify: `crates/oku-slides-session/src/library_packs.rs` (after `set_current`, near line 170)

**Interfaces:**
- Produces (all `pub(crate)`, re-exported from `crate::library`):
  - `struct DraftLock` and `fn draft_lock() -> DraftLock`
  - `enum DraftWrite { New, Replace(PackVersion), Child(PackVersion) }` with `fn replaced(&self) -> Option<&PackVersion>`
  - `Library<K>::is_frozen(&self, id: Uuid, v: &PackVersion) -> bool`
  - `Library<K>::draft_write(&self, lock: &DraftLock, id: Uuid, draft: Option<&PackVersion>) -> DraftWrite`
  - `Library<K>::save_draft(&self, lock: &DraftLock, id: Uuid, write: &DraftWrite, files: &PackFiles) -> Result<PackVersion>`
  - `Library<K>::adopt(&self, lock: &DraftLock, id: Uuid, files: &PackFiles) -> Result<()>`
  - The same four methods on `PackLibrary`, routed by kind.

- [ ] **Step 1: Write the failing tests**

Create `crates/oku-slides-session/src/library_draft_tests.rs`:

```rust
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Barrier;

use oku_slides_core::{PackFiles, PackKind, PackVersion};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::Storage;
use crate::library_kind::Widgets;
use crate::pack::BASE_PACK;
use crate::test_support::{builtin, fresh_dir, manifest, ns};
use crate::theme::{ThemeLibrary, ThemeManifest};

/// Une bibliothèque de packs de widgets sous `dir`.
fn widgets(dir: &Path) -> Library<Widgets> {
    Library::new(Some(dir.to_owned()), Storage::local().unwrap())
}

/// Les fichiers d'une version du pack de widgets `id`, sans widgets.
fn files(id: Uuid, name: &str, parent: Option<&PackVersion>) -> PackFiles {
    manifest(id, name, parent, "2026-10-24T10:00:00Z").to_files(PackKind::Widgets, BTreeMap::new()).unwrap()
}

/// Enregistre `name` comme brouillon `draft` de `id`, parente `base` sauf brouillon figé ; sa version.
fn save(lib: &Library<Widgets>, id: Uuid, draft: Option<&PackVersion>, base: Option<&PackVersion>, name: &str) -> PackVersion {
    let lock = draft_lock();
    let write = lib.draft_write(&lock, id, draft);
    let parent = match &write {
        DraftWrite::Child(v) => Some(v),
        DraftWrite::New | DraftWrite::Replace(_) => base,
    };
    lib.save_draft(&lock, id, &write, &files(id, name, parent)).unwrap()
}

#[test]
fn draft_write_without_draft_is_new() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    assert_that!(lib.draft_write(&draft_lock(), Uuid::new_v4(), None)).is_equal_to(DraftWrite::New);
}

#[test]
fn save_draft_unadopted_replaces_in_place() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    let second = save(&lib, id, Some(&first), None, "Deux");
    assert_that!(lib.version(id, &first)).is_none();
    assert_that!(lib.current(id)).is_equal_to(Some(second));
}

#[test]
fn save_draft_adopted_opens_child_version() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    lib.adopt(&draft_lock(), id, &files(id, "Un", None)).unwrap();
    let second = save(&lib, id, Some(&first), None, "Deux");
    assert_that!(lib.version(id, &first).is_some()).is_true();
    assert_that!(lib.current_version(id).unwrap().manifest.parent).is_equal_to(Some(first));
    assert_that!(lib.current(id)).is_equal_to(Some(second));
}

#[test]
fn adopt_marks_the_version_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = save(&lib, id, None, None, "Un");
    lib.adopt(&draft_lock(), id, &files(id, "Un", None)).unwrap();
    assert_that!(lib.is_frozen(id, &v)).is_true();
}

#[test]
fn unadopted_local_version_is_not_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = save(&lib, id, None, None, "Un");
    assert_that!(lib.is_frozen(id, &v)).is_false();
}

#[test]
fn builtin_version_is_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    assert_that!(lib.is_frozen(BASE_PACK, &builtin(BASE_PACK).version)).is_true();
}

#[test]
fn received_version_is_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = files(id, "Reçu", None);
    lib.receive_version(id, &v).unwrap();
    assert_that!(lib.is_frozen(id, &v.version())).is_true();
}

#[test]
fn adopt_is_idempotent() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = files(id, "Un", None);
    lib.adopt(&draft_lock(), id, &v).unwrap();
    assert_that!(lib.adopt(&draft_lock(), id, &v)).is_ok();
}

#[test]
fn adopt_restores_a_version_missing_from_the_library() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    let gone = files(id, "Ancien", None);
    lib.adopt(&draft_lock(), id, &gone).unwrap();
    assert_that!(lib.version(id, &gone.version()).is_some()).is_true();
    assert_that!(lib.current(id)).is_equal_to(Some(first));
}

/// Deux onglets (deux instances) : l'un enregistre sans arrêt, l'autre adopte ; rien d'adopté ne disparaît.
#[test]
fn adopt_then_save_concurrent_never_rewrites_adopted() {
    let dir = fresh_dir("oku-slides-drafts-");
    let id = Uuid::new_v4();
    let (editor, deck) = (widgets(dir.path()), widgets(dir.path()));
    let start = Barrier::new(2);
    let adopted: Vec<PackVersion> = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            start.wait();
            let mut draft = None;
            for n in 0..40 {
                draft = Some(save(&editor, id, draft.as_ref(), None, &format!("v{n}")));
            }
        });
        start.wait();
        let mut adopted = Vec::new();
        while !writer.is_finished() {
            let lock = draft_lock();
            if let Some(current) = deck.current_version(id) {
                deck.adopt(&lock, id, &current.files).unwrap();
                adopted.push(current.version);
            }
        }
        writer.join().unwrap();
        adopted
    });
    for v in &adopted {
        assert_that!(deck.version(id, v).is_some()).is_true();
    }
}

/// Un thème suit la même règle : adopté, son enregistrement suivant ouvre une fille.
#[test]
fn theme_save_draft_adopted_opens_child_version() {
    let dir = fresh_dir("oku-slides-drafts-themes-");
    let lib = ThemeLibrary::new(Some(dir.path().to_owned()), Storage::local().unwrap());
    let id = Uuid::new_v4();
    let theme = |name: &str, parent: Option<&PackVersion>| {
        ThemeManifest {
            id,
            name: name.into(),
            namespace: ns("marque"),
            extends: Vec::new(),
            description: String::new(),
            parent: parent.cloned(),
            created: "2026-10-24T10:00:00Z".into(),
            tokens: serde_json::json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap(),
        }
        .to_files()
        .unwrap()
    };
    let lock = draft_lock();
    let first = lib.save_draft(&lock, id, &DraftWrite::New, &theme("Un", None)).unwrap();
    lib.adopt(&lock, id, &theme("Un", None)).unwrap();
    let write = lib.draft_write(&lock, id, Some(&first));
    assert_that!(write).is_equal_to(DraftWrite::Child(first.clone()));
    lib.save_draft(&lock, id, &write, &theme("Deux", Some(&first))).unwrap();
    assert_that!(lib.version(id, &first).is_some()).is_true();
}
```

At the end of `crates/oku-slides-session/src/library_draft.rs` (created in Step 3), the test module is declared; for now, create the file with only the module doc and that declaration so the tests compile against missing items:

```rust
//! Brouillons de la bibliothèque : remplacés sur place jusqu'à leur adoption par un deck.

#[cfg(test)]
#[path = "library_draft_tests.rs"]
mod library_draft_tests;
```

and declare the child module in `crates/oku-slides-session/src/library.rs`, right before the first `#[cfg(test)]` block at the end of the file (near line 629):

```rust
#[path = "library_draft.rs"]
mod draft;
pub(crate) use draft::{DraftLock, DraftWrite, draft_lock};
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo nextest run -p oku-slides-session library_draft`
Expected: compile errors, `cannot find function draft_lock`, `no method named draft_write`, `adopt`, `save_draft`, `is_frozen`.

- [ ] **Step 3: Generalize the marker read in `library.rs`**

Replace `is_marked_received` (near line 268) with a marker-generic read, and update its two callers (`read_version` and `is_received`) to pass `RECEIVED_FILE`:

```rust
    /// Le dossier de version `folder` porte-t-il la marque `mark` (`received`, `adopted`) ?
    fn is_marked(&self, folder: &Path, mark: &str) -> bool {
        let path = folder.join(mark);
        self.storage
            .len(&path)
            .inspect_err(|e| tracing::warn!(path = %path.display(), error = %one_line(e), "marque de version illisible"))
            .is_ok_and(|len| len.is_some())
    }
```

```rust
        let received = self.is_marked(folder, RECEIVED_FILE);
```

```rust
    pub fn is_received(&self, id: Uuid, v: &PackVersion) -> bool {
        self.pack_dir(id).is_some_and(|d| self.is_marked(&d.join(VERSIONS_DIR).join(v.hex()), RECEIVED_FILE))
    }
```

- [ ] **Step 4: Write `library_draft.rs`**

Replace the file content with:

```rust
//! Brouillons de la bibliothèque : remplacés sur place jusqu'à leur adoption par un deck.
//!
//! Une version est *figée* quand elle est intégrée, reçue avec un deck
//! (`received`) ou adoptée (`adopted`, écrit dès qu'un deck l'épingle). Un
//! brouillon figé n'est jamais remplacé : l'enregistrement suivant crée sa
//! fille. Décider ([`Library::draft_write`]) puis écrire ([`Library::save_draft`])
//! se fait sous [`DraftLock`], un verrou du processus que prend aussi
//! [`Library::adopt`] : une adoption ne se glisse jamais entre les deux.

use std::sync::{Mutex, MutexGuard, PoisonError};

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use super::{Library, RECEIVED_FILE, VERSIONS_DIR};
use crate::Result;
use crate::error::{NoLibraryDirSnafu, WriteSnafu};
use crate::library_kind::{LibraryKind, VersionManifest};

/// Marque d'une version adoptée par un deck, dans son dossier.
const ADOPTED_FILE: &str = "adopted";

/// Le verrou des brouillons, commun à toutes les bibliothèques du processus.
static DRAFTS: Mutex<()> = Mutex::new(());

/// Preuve que le verrou des brouillons est pris ; à relâcher au plus tôt.
#[derive(Debug)]
pub(crate) struct DraftLock {
    /// Tenue pour sa durée de vie : la relâcher relâche le verrou.
    _guard: MutexGuard<'static, ()>,
}

/// Prend le verrou des brouillons ; jamais deux fois sur un même fil, il n'est pas réentrant.
pub(crate) fn draft_lock() -> DraftLock {
    DraftLock { _guard: DRAFTS.lock().unwrap_or_else(PoisonError::into_inner) }
}

/// Comment s'écrit l'enregistrement d'un brouillon, décidé sous le verrou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DraftWrite {
    /// Pas encore de brouillon : une nouvelle version.
    New,
    /// Le brouillon, pas figé : remplacé sur place.
    Replace(PackVersion),
    /// Le brouillon, figé : une nouvelle version, dont il est la parente.
    Child(PackVersion),
}

impl DraftWrite {
    /// La version que l'écriture remplace ; `None` quand elle en crée une.
    pub(crate) fn replaced(&self) -> Option<&PackVersion> {
        match self {
            Self::Replace(v) => Some(v),
            Self::New | Self::Child(_) => None,
        }
    }
}

impl<K: LibraryKind> Library<K> {
    /// La version `v` de `id` est-elle figée : intégrée, reçue ou adoptée ?
    pub(crate) fn is_frozen(&self, id: Uuid, v: &PackVersion) -> bool {
        if Self::builtin(id).is_some_and(|b| b.version == *v) {
            return true;
        }
        let Some(folder) = self.pack_dir(id).map(|d| d.join(VERSIONS_DIR).join(v.hex())) else { return false };
        self.is_marked(&folder, RECEIVED_FILE) || self.is_marked(&folder, ADOPTED_FILE)
    }

    /// Comment enregistrer le brouillon `draft` de `id` ; le verrou reste pris jusqu'à l'écriture.
    pub(crate) fn draft_write(&self, _lock: &DraftLock, id: Uuid, draft: Option<&PackVersion>) -> DraftWrite {
        match draft {
            None => DraftWrite::New,
            Some(v) if self.is_frozen(id, v) => DraftWrite::Child(v.clone()),
            Some(v) => DraftWrite::Replace(v.clone()),
        }
    }

    /// Écrit `files`, l'enregistrement d'un brouillon, et la rend courante ; rend sa version.
    ///
    /// `write` vient de [`Self::draft_write`] sous le même verrou. Pour
    /// `Child`, l'appelant a fait du brouillon figé la parente de `files`.
    pub(crate) fn save_draft(
        &self,
        _lock: &DraftLock,
        id: Uuid,
        write: &DraftWrite,
        files: &PackFiles,
    ) -> Result<PackVersion> {
        if let DraftWrite::Child(parent) = write {
            let found = K::Manifest::from_files(files).ok().and_then(|m| m.parent().cloned());
            debug_assert_eq!(found.as_ref(), Some(parent), "la fille d'un brouillon figé descend de lui");
        }
        match write {
            DraftWrite::Replace(old) => self.replace_version(id, old, files),
            DraftWrite::New | DraftWrite::Child(_) => {
                let version = files.version();
                self.add_version(id, files)?;
                self.set_current(id, &version)?;
                Ok(version)
            }
        }
    }

    /// Fige la version `files` de `id`, qu'un deck épingle ; absente, elle est d'abord rajoutée.
    ///
    /// Idempotent. La version courante ne change pas, sauf pour un pack
    /// inconnu (voir [`Self::add_version`]).
    pub(crate) fn adopt(&self, _lock: &DraftLock, id: Uuid, files: &PackFiles) -> Result<()> {
        let version = files.version();
        if self.is_frozen(id, &version) {
            return Ok(());
        }
        self.add_version(id, files)?;
        let dir = self.pack_dir(id).context(NoLibraryDirSnafu { kind: K::DIR })?;
        let path = dir.join(VERSIONS_DIR).join(version.hex()).join(ADOPTED_FILE);
        self.storage.write_atomic(&path, b"").context(WriteSnafu { path: &path })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "library_draft_tests.rs"]
mod library_draft_tests;
```

- [ ] **Step 5: Add the `PackLibrary` wrappers**

In `crates/oku-slides-session/src/library_packs.rs`, import `use crate::library::{DraftLock, DraftWrite};` and add after `set_current`:

```rust
    /// La version `v` du pack `id` est-elle figée (voir [`Library::is_frozen`]) ?
    pub(crate) fn is_frozen(&self, id: Uuid, v: &PackVersion) -> bool {
        by_kind!(self, self.kind_of(id), l => l.is_frozen(id, v))
    }

    /// Comment enregistrer le brouillon `draft` du pack `id` (voir [`Library::draft_write`]).
    pub(crate) fn draft_write(&self, lock: &DraftLock, id: Uuid, draft: Option<&PackVersion>) -> DraftWrite {
        by_kind!(self, self.kind_of(id), l => l.draft_write(lock, id, draft))
    }

    /// Écrit le brouillon `files`, dans la sorte de ses fichiers (voir [`Library::save_draft`]).
    pub(crate) fn save_draft(
        &self,
        lock: &DraftLock,
        id: Uuid,
        write: &DraftWrite,
        files: &PackFiles,
    ) -> Result<PackVersion> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.save_draft(lock, id, write, files))
    }

    /// Fige la version `files` du pack `id` (voir [`Library::adopt`]).
    pub(crate) fn adopt(&self, lock: &DraftLock, id: Uuid, files: &PackFiles) -> Result<()> {
        self.check_kinds(id, files)?;
        by_kind!(self, files.kind(), l => l.adopt(lock, id, files))
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo nextest run -p oku-slides-session library_draft`
Expected: 11 tests PASS.
Run: `cargo nextest run -p oku-slides-session library`
Expected: PASS (the received marker still reads).

- [ ] **Step 7: Lint**

Run: `cargo clippy -p oku-slides-session --all-targets -- -D warnings`
Expected: no warning. `PackLibrary::is_frozen`/`draft_write`/`save_draft`/`adopt` are used in Tasks 2 and 3; if clippy flags them as dead in this commit, add the Task 2 callers before committing rather than any `allow`.

- [ ] **Step 8: Commit**

```bash
git add crates/oku-slides-session/src/library_draft.rs crates/oku-slides-session/src/library_draft_tests.rs crates/oku-slides-session/src/library.rs crates/oku-slides-session/src/library_packs.rs
```

Message file: `feat(packs) : brouillons de la bibliothèque, figés à l'adoption par un deck (t-1158)`. Then `oku cli task done --project slider --id t-1158`.

---

### Task 2: Pack mode saves through `save_draft`, rebased onto a frozen draft (t-1159)

**Files:**
- Modify: `crates/oku-slides-session/src/pack_library.rs:46-70`
- Modify: `crates/oku-slides-session/src/pack_mode_save.rs:186-260` (`adopt_session_version`)
- Modify: `crates/oku-slides-session/src/library_packs.rs` (remove `replace_version`)
- Modify: `crates/oku-slides-session/src/library_tests.rs:147-180`
- Test: `crates/oku-slides-session/src/pack_mode_save_tests.rs`

**Interfaces:**
- Consumes: `draft_lock`, `DraftLock`, `DraftWrite`, `PackLibrary::{draft_write, save_draft, adopt, is_frozen}` from Task 1.
- Produces: `pack_library::write_session_version(library: &PackLibrary, lock: &DraftLock, id: Uuid, write: &DraftWrite, files: &PackFiles) -> Result<PackVersion>`; `pack_library::rebased(files: &PackFiles, parent: &PackVersion) -> Result<PackFiles>`.

- [ ] **Step 1: Write the failing test**

Append to `crates/oku-slides-session/src/pack_mode_save_tests.rs` (add `use crate::library::draft_lock;` to the imports):

```rust
/// Un autre deck a adopté la version de la séance : l'enregistrement suivant ouvre sa fille.
#[test]
fn save_pack_after_the_session_version_was_adopted_opens_a_child() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let first = library_of(&s).current_version(ESSENTIEL_PACK).unwrap();
    library_of(&s).adopt(&draft_lock(), ESSENTIEL_PACK, &first.files).unwrap();
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    let second = library_of(&s).current_version(ESSENTIEL_PACK).unwrap();
    assert_that!(library_of(&s).version(ESSENTIEL_PACK, &first.version).is_some()).is_true();
    assert_that!(second.manifest.parent.clone()).is_equal_to(Some(first.version.clone()));
    assert_that!(edit_of(&s).session_version).is_equal_to(Some(second.version));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p oku-slides-session save_pack_after_the_session_version_was_adopted_opens_a_child`
Expected: FAIL — the first version is gone (`replace_version` deleted it), `is_some()` is false.

- [ ] **Step 3: Rewrite `write_session_version` and add `rebased` in `pack_library.rs`**

Replace `write_session_version` (lines 46-70) with:

```rust
/// Écrit `files`, version de séance du pack `id`, comme le décide `write` ; rend sa version.
///
/// `write` vient de [`PackLibrary::draft_write`] sous `lock`. Un pack du deck seul n'y écrit rien.
pub(crate) fn write_session_version(
    library: &PackLibrary,
    lock: &DraftLock,
    id: Uuid,
    write: &DraftWrite,
    files: &PackFiles,
) -> Result<PackVersion> {
    if is_deck_only(files) {
        return Ok(files.version());
    }
    library.save_draft(lock, id, write, files)
}

/// `files`, version du pack `id`, avec `parent` pour parente : la fille d'un brouillon figé.
pub(crate) fn rebased(id: Uuid, files: &PackFiles, parent: &PackVersion) -> Result<PackFiles> {
    let mut manifest = PackManifest::from_files(files).context(PackManifestSnafu { id })?;
    manifest.parent = Some(parent.clone());
    Ok(manifest.to_files(files.kind(), files.widgets().clone()).context(PackJsonSnafu { id })?)
}
```

Imports to add in `pack_library.rs`: `use snafu::ResultExt;` (with `OptionExt, ensure`), `use crate::error::PackJsonSnafu;`, `use crate::library::{DraftLock, DraftWrite};`.

- [ ] **Step 4: Decide under the lock in `adopt_session_version`**

In `crates/oku-slides-session/src/pack_mode_save.rs`, the function takes `files: PackFiles` (make it `mut files`). Replace its first lines up to `let deck_only = …` with:

```rust
        // Décider puis écrire sous le verrou des brouillons : une adoption ne s'y glisse pas.
        let lock = draft_lock();
        let session = edit.session_version.as_ref().filter(|_| !as_new);
        let write = self.library.draft_write(&lock, edit.pack, session);
        // Le brouillon de la séance est figé (un deck l'a adopté) : la version suivante est sa fille.
        if let DraftWrite::Child(frozen) = &write {
            files = rebased(edit.pack, &files, frozen)?;
        }
        let manifest = PackManifest::from_files(&files).context(PackManifestSnafu { id: edit.pack })?;
        let (id, parent) = (manifest.id, manifest.parent.clone());
        let replaced = write.replaced();
        let version = files.version();
        let deck_only = !manifest.scope.is_library();
```

Replace the write (`write_session_version(&self.library, id, replaced, &files)?;`) with the following, which also releases the lock before anything that may refresh the pins:

```rust
        write_session_version(&self.library, &lock, id, &write, &files)?;
        drop(lock);
```

`replaced` stays an `Option<&PackVersion>` and the rest of the function (stash cleanup `if let Some(old) = replaced…`) is unchanged. Update imports: `use crate::library::{DraftWrite, draft_lock};` and `use crate::pack_library::{rebased, restore_session_version, write_session_version};`.

- [ ] **Step 5: Remove `PackLibrary::replace_version` and move its tests to `save_draft`**

Delete `replace_version` from `library_packs.rs` (its only caller was `write_session_version`). In `library_tests.rs`, add `use crate::library::{DraftWrite, draft_lock};` and, in the two tests near lines 147-180, replace

```rust
    let replaced = lib.replace_version(BASE_PACK, &first.version, &next).unwrap();
```

with

```rust
    let replaced = lib.save_draft(&draft_lock(), BASE_PACK, &DraftWrite::Replace(first.version.clone()), &next).unwrap();
```

and

```rust
    lib.replace_version(BASE_PACK, &builtin, &next).unwrap();
```

with

```rust
    lib.save_draft(&draft_lock(), BASE_PACK, &DraftWrite::Replace(builtin.clone()), &next).unwrap();
```

Rename the tests to `save_draft_replace_keeps_the_parent_removes_the_old_one_and_updates_current` and `save_draft_replace_never_deletes_the_builtin_version`.

- [ ] **Step 6: Run the tests**

Run: `cargo nextest run -p oku-slides-session pack_mode library widgets_edit layout_examples`
Expected: PASS, including the new test and `saving_twice_in_one_session_keeps_one_version_and_the_deck_follows_in_one_undo_step`.

- [ ] **Step 7: Lint and commit**

Run: `cargo clippy -p oku-slides-session --all-targets -- -D warnings` — expected clean.

```bash
git add crates/oku-slides-session/src/pack_library.rs crates/oku-slides-session/src/pack_mode_save.rs crates/oku-slides-session/src/pack_mode_save_tests.rs crates/oku-slides-session/src/library_packs.rs crates/oku-slides-session/src/library_tests.rs
```

Message file: `fix(packs) : le mode pack ne remplace plus une version adoptée, il ouvre sa fille (t-1159)`. Then `oku cli task done --project slider --id t-1159`.

---

### Task 3: Freeze the versions a deck pins; docs and rules (t-1160)

**Files:**
- Create: `crates/oku-slides-session/src/deck_adopt.rs`
- Create: `crates/oku-slides-session/src/deck_adopt_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs:46` (module list)
- Modify: `crates/oku-slides-session/src/deck_packs.rs:213` (`refresh_pack_updates`)
- Modify: `crates/oku-slides-session/src/pack_mode.rs:346-364` (`close_pack`)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-session/AGENTS.md`

**Interfaces:**
- Consumes: `draft_lock`, `PackLibrary::{adopt, is_frozen}`, `Library::<Themes>::{adopt, is_frozen}` from Task 1.
- Produces: `Session::adopt_pins(&self)` (pub(crate)).

- [ ] **Step 1: Write the failing tests**

Create `crates/oku-slides-session/src/deck_adopt_tests.rs`:

```rust
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{add_pack, add_version, fresh_dir, library_of, manifest, rename_title, session_on};

#[test]
fn adopt_pins_pinning_a_library_pack_freezes_its_version() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    let m = manifest(Uuid::new_v4(), "P", None, "2026-10-24T10:00:00Z");
    let pin = add_version(&s.library, &m, &[]);
    s.apply("Pack", vec![add_pack(&m, &pin)], None).unwrap();
    assert_that!(s.library.is_frozen(m.id, &pin.version)).is_true();
}

#[test]
fn adopt_pins_in_pack_mode_leaves_the_session_version_a_draft() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    let v = s.save_pack(false).unwrap();
    assert_that!(library_of(&s).is_frozen(ESSENTIEL_PACK, &v)).is_false();
}

#[test]
fn close_pack_freezes_the_version_the_deck_now_pins() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    let v = s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    assert_that!(library_of(&s).is_frozen(ESSENTIEL_PACK, &v)).is_true();
}

#[test]
fn adopt_pins_freezes_the_deck_theme_version() {
    let data = fresh_dir("oku-slides-adopt-");
    let s = session_on(data.path());
    let pin = s.deck_theme_pin().cloned().unwrap();
    assert_that!(s.themes.is_frozen(pin.id, &pin.version)).is_true();
}
```

`adopt_pins_freezes_the_deck_theme_version` passes even before the change (the default theme is a built-in): keep it as the guard that theme pins are read, and add one with a user theme — reuse `test_support::add_user_theme` (already used by `deck_packs_tests.rs`; read its signature there) to add a user theme, `s.set_theme(&its_namespace)`, then assert `s.themes.is_frozen(id, &version)`. Name it `adopt_pins_setting_a_user_theme_freezes_its_version`.

Declare the module: create `crates/oku-slides-session/src/deck_adopt.rs` with only

```rust
//! Les versions qu'épingle un deck sont figées dans la bibliothèque.

#[cfg(test)]
#[path = "deck_adopt_tests.rs"]
mod deck_adopt_tests;
```

and add `mod deck_adopt;` after `mod data_panel;` in `lib.rs`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-session deck_adopt`
Expected: `adopt_pins_pinning_a_library_pack_freezes_its_version`, `close_pack_freezes_the_version_the_deck_now_pins` and the user-theme test FAIL (`is_frozen` false); the pack-mode one and the built-in theme one PASS.

- [ ] **Step 3: Write `Session::adopt_pins`**

Replace `deck_adopt.rs` with:

```rust
//! Les versions qu'épingle un deck sont figées dans la bibliothèque.
//!
//! Un deck en mode deck adopte chaque version de la bibliothèque qu'il
//! épingle, packs et thèmes : la bibliothèque ne la remplacera plus. Le
//! deck de travail du mode pack, et le deck mis de côté qui épingle la
//! version de la séance, n'adoptent rien : fermer le mode pack adopte.

use crate::library::draft_lock;
use crate::pack_receive::is_deck_only;
use crate::{Mode, Session};

impl Session {
    /// Fige chaque version de la bibliothèque qu'épingle le deck, en mode deck seulement.
    ///
    /// Un échec est journalisé : le deck embarque la version, rien n'est
    /// perdu ; seul un enregistrement à venir pourrait la remplacer.
    pub(crate) fn adopt_pins(&self) {
        if !matches!(self.mode, Mode::Deck) {
            return;
        }
        let lock = draft_lock();
        let blobs = &self.doc.sources;
        let deck = blobs.deck();
        for (_, _, pin) in deck.all_pins() {
            let Some(files) = blobs.packs.get(&pin.version).filter(|f| !is_deck_only(f)) else { continue };
            if let Err(e) = self.library.adopt(&lock, pin.id, files) {
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "version épinglée non figée");
            }
        }
        for pin in deck.themes.values() {
            let Some(files) = blobs.themes.get(&pin.version) else { continue };
            if let Err(e) = self.themes.adopt(&lock, pin.id, files) {
                tracing::warn!(theme = %pin.id, error = %e.one_line(), "thème épinglé non figé");
            }
        }
    }
}

#[cfg(test)]
#[path = "deck_adopt_tests.rs"]
mod deck_adopt_tests;
```

Check the field names before writing: `SourceBundle` exposes `packs` and `themes` (`BTreeMap<PackVersion, PackFiles>`, see `deck_theme.rs:58` and `deck_pack.rs:140`), and `Deck::themes` maps namespaces to `ThemePin` (see `deck_theme.rs:44`).

- [ ] **Step 4: Call it where pins change and when the pack mode closes**

In `deck_packs.rs`, first line of `refresh_pack_updates` (line 213):

```rust
        // Les épinglages ont changé : le deck adopte ce qu'il épingle avant qu'on le compare.
        self.adopt_pins();
```

In `pack_mode.rs`, `close_pack`, inside the `Mode::Pack { stash, .. }` arm, after `self.doc = *stash;`:

```rust
                // Le deck retrouvé épingle la version de la séance : elle est figée.
                self.adopt_pins();
```

`adopt_pins` takes the draft lock: neither call site holds it (Task 2 releases it before `sync_widgets`, `refresh_pack_updates` runs after `adopt_session_version` returns).

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p oku-slides-session deck_adopt pack_mode deck_packs`
Expected: PASS. `a_new_editing_session_creates_a_second_version` still passes (a new session starts without a draft).
Run: `just test`
Expected: PASS.

- [ ] **Step 6: Docs and rules**

`crates/oku-slides-session/ARCHITECTURE.md` stays at 50 lines: in the line listing library modules (`… \`namespace_slug.rs\`, \`library.rs\`, \`library_index.rs\`).`), write `\`library.rs\` (\`library_draft.rs\`: drafts replaced in place until a deck adopts them, \`adopted\`), \`library_index.rs\`; \`deck_adopt.rs\` freezes a deck's pins).` instead of `\`library.rs\`, \`library_index.rs\`).`

`crates/oku-slides-session/AGENTS.md`, Invariants, after the "Never let a received deck change…" rule:

```markdown
- Save a pack or theme draft only through draft_write then save_draft under one draft_lock, never holding it twice on a thread; freeze with adopt every library version a deck in deck mode pins (and the session version on close_pack); never replace a built-in, received or adopted version. (spec:2026-10-24-onglets-pack-theme-design, t-1152)
```

Run: `just check`
Expected: PASS (fmt, clippy, tsc, agents-lint).

- [ ] **Step 7: Commit**

```bash
git add crates/oku-slides-session/src/deck_adopt.rs crates/oku-slides-session/src/deck_adopt_tests.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/src/deck_packs.rs crates/oku-slides-session/src/pack_mode.rs crates/oku-slides-session/ARCHITECTURE.md crates/oku-slides-session/AGENTS.md
```

Message file: `feat(packs) : un deck fige les versions qu'il épingle, packs et thèmes (t-1160)`. Then `oku cli task done --project slider --id t-1160` and `oku cli task done --project slider --id t-1153`.

---

## After lot 1

Lots 2 to 5 (t-1154 to t-1157) each get their own plan, written when the lot starts, on top of the merged previous lot: `docs/superpowers/plans/2026-10-24-onglets-pack-theme-lot<N>-<slug>.md`.
