# Instance unique, onglets et fenêtres — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** plusieurs présentations ouvertes à la fois, dans des onglets répartis sur plusieurs fenêtres (donc plusieurs écrans), dans une seule instance de l'app.

**Architecture:** une `Session` par onglet, inchangée pour l'essentiel. Un nouveau `Workspace` (`crates/oku-slides-session/src/workspace.rs`) tient les fenêtres, leurs onglets ordonnés et les sessions (`Arc<Mutex<Session>>`). Trois fonctions associées, `Workspace::apply` (structure), `Workspace::request` (requête d'un onglet) et `Workspace::open_files` (fichiers reçus), portent seules l'ordre des verrous ; le serveur de dev et l'hôte Tauri les appellent. La reprise v1 sert telle quelle par onglet (`resume/tabs/<id>/`) ; `resume/workspace.json` décrit fenêtres et onglets. L'hôte crée et ferme les vraies fenêtres pour qu'elles suivent la vue et diffuse l'événement `workspace`. L'UI monte une barre d'onglets et l'`App` de l'onglet actif.

**Tech Stack:** Rust (snafu, serde, speculoos, OpenDAL via `Storage`), Tauri 2.11 (plugins dialog 2.7, window-state 2.4, single-instance 2.4), React 19 + Bun, Playwright.

**Spec:** `docs/superpowers/specs/2026-09-26-onglets-fenetres-design.md`

**Tâche Oku :** t-142 (projet `slider`), sous-tâches t-147 à t-154, une par tâche du plan, dans l'ordre. Chaque commit cite sa sous-tâche, ex. `(t-147)`. À la fin de chaque tâche : `oku cli task done --project slider --id t-14x`. Pas de ligne `Co-Authored-By`. Suites : t-156 à t-160 (backlog).

**Commits intermédiaires :** la branche est fusionnée par squash. Le serveur de dev (`just e2e`) reste vert à chaque commit ; l'app de bureau ne fonctionne pas entre t-150 (l'UI parle aux onglets) et t-152 (l'hôte les connaît). Ne pas lancer l'app de bureau entre ces deux commits.

## Global Constraints

- Pas de `.unwrap()` / `.expect()` en production ; erreurs Snafu avec `backtrace`.
- Pas de `std::fs` en production : `Storage` (OpenDAL). `std::fs` autorisé dans les tests. `Path::is_file` reste admis dans l'hôte pour filtrer les arguments de lancement (le code existant y utilisait `exists`).
- Toute écriture de reprise passe par `Storage::write_atomic`. Un échec d'écriture de reprise est journalisé (`tracing::warn!`), jamais renvoyé à l'UI.
- Verrous : l'espace de travail puis une session, jamais l'inverse. Seules `Workspace::apply`, `Workspace::request` et `Workspace::open_files` combinent les deux ; aucun autre code ne verrouille une session. Un verrou empoisonné est repris (`PoisonError::into_inner`).
- Aucun travail lent (lecture de fichier, rendu Typst) sous le verrou de l'espace de travail, sauf la vérification d'état d'une session à la fermeture ; rien de bloquant sur le fil principal de Tauri (callbacks d'événements : `spawn_blocking`, ou `try_lock`).
- Données relues (`workspace.json`) : non fiables. Identifiants `t-<chiffres>` et labels `w-<chiffres>` seulement, avant tout `join`, suppression ou création de fenêtre.
- Tests : side-by-side `*_tests.rs` inclus par `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`, noms `<unité>_<cas>_<résultat>`, `speculoos`, dossiers temporaires supprimés en fin de test. UI : `*.test.ts` à côté, `bun test`.
- Texte utilisateur via Fluent ; toute clé ajoutée l'est dans les 20 locales (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`) dans la même tâche que le code qui l'utilise ; `every_locale_has_exactly_the_reference_keys` et `ui/src/i18n-keys.test.ts` passent à chaque commit.
- Docs : tout élément public a une doc d'une phrase de moins de 15 mots ; tout module une doc de module. Commentaires et docs en français. Fichiers < 1000 lignes.
- Commandes : `cargo nextest run -p <crate>`, `just test`, `just check`, `just e2e`, `bun test` et `bun run typecheck` (dans `ui/`).
- Messages de commit : `type(scope) : description (t-NN)`.

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-platform/src/storage.rs` (+ `_tests`) | `resolve`, `list_dirs`, `delete_dir`, `write_atomic` (fichier temporaire unique) |
| `crates/oku-slides-session/src/resume.rs` | reprise d'un dossier ; helpers JSON versionnés ; `quarantine` réutilisable |
| `crates/oku-slides-session/src/lib.rs` | `Session::resume(dir) -> bool`, `tab_info`, `set_notice`, `stop_resume`, `Document::blank` ; doc du crate |
| `crates/oku-slides-session/src/views.rs` | `TabInfo` |
| `crates/oku-slides-session/src/error.rs` | `UnknownTab`, `UnknownWindow`, `Unsaved` |
| `crates/oku-slides-session/src/workspace.rs` (+ `_tests`) | `Workspace`, `WorkspaceRequest`, `WorkspaceView`, `Rect` |
| `crates/oku-slides-session/tests/workspace.rs` | reprise de l'espace de travail |
| `crates/oku-slides-cli/src/serve.rs` (+ `tests/cli.rs`) | serveur de dev sur l'espace de travail, origine vérifiée |
| `ui/src/api.ts`, `tabs.ts` (+ `tabs.test.ts`), `close.ts` (+ `close.test.ts`) | API par onglet, logique de la barre, fermeture |
| `ui/src/Workspace.tsx`, `TabBar.tsx` | coquille à onglets |
| `ui/src/App.tsx`, `TemplatePicker.tsx`, `main.tsx`, `i18n.ts`, `styles.css` | App par onglet |
| `ui/e2e/tabs.cjs`, `ui/e2e/smoke.cjs` | e2e |
| `src-tauri/src/{main,commands,windows,menu}.rs`, `tauri.conf.json`, `capabilities/default.json`, `Cargo.toml` | hôte ; menu macOS sans « Fermer la fenêtre » |

---

### Task 1 : Session — reprise par dossier d'onglet, infos d'onglet (t-147)

**Files:**
- Modify: `crates/oku-slides-platform/src/storage.rs`, `storage_tests.rs`
- Modify: `crates/oku-slides-session/src/resume.rs`, `lib.rs`, `views.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (clé retirée)
- Modify: `src-tauri/src/main.rs` (transitoire)
- Test: `crates/oku-slides-session/tests/session.rs`

**Interfaces:**
- Produces:
  - `pub fn oku_slides_platform::storage::resolve(path: &Path) -> PathBuf` : chemin absolu, `.` et `..` résolus, sans toucher au disque.
  - `pub fn Session::resume(&mut self, dir: PathBuf) -> bool` (`#[must_use]`) : relit `dir` ou l'initialise ; `false` : état illisible, dossier mis de côté, deck vierge, rien n'est écrit.
  - `pub fn Session::tab_info(&self) -> TabInfo`, `pub fn Session::set_notice(&mut self, text: String)`, `pub(crate) fn Session::stop_resume(&mut self)`.
  - `pub struct TabInfo { title, dirty, editing_templates, path: Option<PathBuf>, pristine }` (`Debug, Clone, PartialEq, Serialize`), réexporté.
- Supprimé : `Session::resume(dir, file)`, `open_at_launch`, clé `notice-resume-kept-unsaved`.

- [ ] **Step 1 : test de `resolve` qui échoue**, dans `storage_tests.rs` :

```rust
#[test]
fn resolve_makes_the_path_absolute_and_drops_dot_segments() {
    let cwd = std::env::current_dir().unwrap();
    assert_that!(resolve(Path::new("a/./b/../c.pdf"))).is_equal_to(cwd.join("a/c.pdf"));
    assert_that!(resolve(Path::new("/x/y/../z"))).is_equal_to(PathBuf::from("/x/z"));
}
```

  (ajouter `use std::path::Path;` à l'import du fichier de test.)

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-platform resolve` → échec (`resolve` absent).

- [ ] **Step 3 :** dans `storage.rs`, après `fn normalize` :

```rust
/// Chemin absolu, `.` et `..` résolus, sans toucher au disque.
///
/// Les liens ne sont pas suivis : deux chemins vers un même fichier par un
/// lien restent différents.
pub fn resolve(path: &Path) -> PathBuf {
    normalize(&std::path::absolute(path).unwrap_or_else(|_| path.to_owned()))
}
```

  `cargo nextest run -p oku-slides-platform` → vert.

- [ ] **Step 4 : tests de session qui échouent.** Dans `crates/oku-slides-session/tests/session.rs` :

  a. Remplacer le helper `resumed` :

```rust
/// Une session qui reprend l'état laissé dans `data`.
fn resumed(data: &std::path::Path) -> Session {
    let mut s = session_in(data);
    let _restored = s.resume(data.join("resume"));
    s
}
```

  b. Remplacer tous les appels `resumed(&data, None)` par `resumed(&data)`.

  c. Supprimer : `resume_with_file_and_unsaved_work_keeps_work_with_notice`, `resume_with_file_and_no_unsaved_work_opens_file`, `resume_with_file_while_editing_templates_and_dirty_deck_keeps_notice_on_deck`, `resume_with_file_matching_the_dirty_document_path_does_not_set_misleading_notice`, `notice_is_shown_once_then_cleared_from_the_next_snapshot`. L'ouverture au lancement passe à l'espace de travail (tâche 3).

  d. Dans `resume_with_corrupt_state_starts_blank`, remplacer `let mut b = resumed(&data, None);` par :

```rust
    let mut b = session_in(&data);
    assert_that!(b.resume(data.join("resume"))).is_false();
```

  e. Dans `resume_with_unreadable_state_moves_old_data_aside_instead_of_overwriting`, remplacer `s.resume(resume_dir.clone(), None);` par `assert_that!(s.resume(resume_dir.clone())).is_false();` et ajouter à la fin :

```rust
    // Rien n'est écrit à la place : l'onglet sera retiré par l'espace de travail.
    assert_that!(resume_dir.exists()).is_false();
```

  f. Dans `resume_with_unreadable_state_and_failed_quarantine_disables_resume_but_keeps_old_data`, remplacer `s.resume(resume_dir.clone(), None);` par `assert_that!(s.resume(resume_dir.clone())).is_false();`.

  g. Ajouter :

```rust
#[test]
fn resume_in_empty_dir_returns_true_and_writes_state() {
    let data = data_dir();
    let mut s = session_in(&data);
    assert_that!(s.resume(data.join("resume"))).is_true();
    assert_that!(data.join("resume/state.json").exists()).is_true();
    std::fs::remove_dir_all(&data).ok();
}

#[test]
fn tab_info_new_session_is_pristine_clean_and_named_like_the_snapshot() {
    let s = session();
    let info = s.tab_info();
    assert_that!(info.pristine).is_true();
    assert_that!(info.dirty).is_false();
    assert_that!(info.editing_templates).is_false();
    assert_that!(info.path).is_none();
    assert_that!(info.title).is_equal_to(s.snapshot().name);
}

#[test]
fn tab_info_after_edit_is_dirty_and_no_longer_pristine() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    let info = s.tab_info();
    assert_that!(info.dirty).is_true();
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_after_undoing_every_edit_is_not_pristine() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    s.undo().unwrap();
    assert_that!(s.tab_info().pristine).is_false();
}

/// Un deck JSON ouvert n'a pas de fichier associé : il n'est pas vierge
/// pour autant, et ouvrir un autre fichier ne doit pas le remplacer.
#[test]
fn tab_info_after_opening_a_json_deck_is_not_pristine() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let info = s.tab_info();
    assert_that!(info.path).is_none();
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_after_save_has_file_name_as_title_and_resolved_path() {
    let data = data_dir();
    let file = data.join("essai.oku-slides.pdf");
    let mut s = session_in(&data);
    s.handle(req(json!({"cmd": "save", "path": data.join("./x/../essai.oku-slides.pdf")}))).unwrap();
    let info = s.tab_info();
    assert_that!(info.title.as_str()).is_equal_to("essai");
    assert_that!(info.path).is_equal_to(Some(file));
    assert_that!(info.pristine).is_false();
    std::fs::remove_dir_all(&data).ok();
}

#[test]
fn tab_info_editing_templates_keeps_deck_title_and_reports_mode() {
    let mut s = session();
    let title = s.tab_info().title;
    s.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}))).unwrap();
    let info = s.tab_info();
    assert_that!(info.editing_templates).is_true();
    assert_that!(info.title).is_equal_to(title);
    assert_that!(info.pristine).is_false();
}

#[test]
fn set_notice_is_shown_once_in_the_snapshot() {
    let mut s = session();
    s.set_notice("Attention".into());
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!("Attention"));
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!(null));
}

/// Test de caractérisation (vert dès l'écriture, et doit le rester) : deux
/// onglets, un seul dossier de modèles ; le cache d'aperçus d'une session
/// suit ce qu'une autre enregistre, car il compare l'ensemble relu.
#[test]
fn templates_view_after_other_session_saves_the_set_shows_the_new_templates() {
    let data = data_dir();
    let mut a = session_in(&data);
    a.edit_templates(oku_slides_session::DEFAULT_SET).unwrap();
    a.save_templates(true).unwrap();
    let id = snapshot_json(&mut a)["editing"]["set"].as_str().unwrap().to_owned();
    let mut b = session_in(&data);
    let before = b.templates_view(Some(id.clone())).unwrap().templates.len();
    let first = a.deck().slides[0].id.clone();
    a.apply("Retirer", vec![Op::RemoveSlide { slide: first }]).unwrap();
    a.save_templates(false).unwrap();
    let after = b.templates_view(Some(id)).unwrap().templates.len();
    assert_that!(after).is_equal_to(before - 1);
    std::fs::remove_dir_all(&data).ok();
}
```

- [ ] **Step 5 :** `cargo nextest run -p oku-slides-session` → échec de compilation (`resume` à deux arguments, `tab_info`, `set_notice` absents).

- [ ] **Step 6 : `TabInfo`** dans `views.rs`. Remplacer `use std::path::Path;` par `use std::path::{Path, PathBuf};` et ajouter après `UiState` :

```rust
/// Un onglet vu de sa barre : titre, travail non enregistré, mode.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabInfo {
    /// Nom du fichier du deck, sinon son titre.
    pub title: String,
    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours.
    pub dirty: bool,
    /// L'onglet édite un ensemble de modèles.
    pub editing_templates: bool,
    /// Fichier du deck, absolu et normalisé : un fichier ouvert une fois.
    pub path: Option<PathBuf>,
    /// Deck neuf jamais touché : ouvrir un fichier le remplace.
    pub pristine: bool,
}
```

  Dans `lib.rs`, ajouter `TabInfo` à `pub use views::{…}`.

- [ ] **Step 7 : `lib.rs`.**

  a. `Document` gagne un champ, après `saved_hash` :

```rust
    /// Deck neuf de la session, jamais remplacé : l'onglet est vierge tant
    /// que rien n'y est fait.
    blank: bool,
```

  `Document::new` met `blank: false`. Dans `Session::new`, le document initial devient :

```rust
            doc: Document { blank: true, ..Document::new(SourceBundle::new(blank_deck(l)), None, settings.history.limit) },
```

  Dans `resume.rs`, le drapeau survit à la reprise (un onglet vierge reste vierge au relancement) : `DocOut` gagne `blank: bool` (rempli par `doc.blank`), `DocIn` gagne `#[serde(default)] blank: bool`, et le `Document { … }` construit par `read_doc` reçoit `blank: raw.blank`. Ajouter à `resume_tests.rs` :

```rust
#[test]
fn write_doc_then_read_doc_keeps_the_blank_flag() {
    let r = resume();
    let mut doc = document();
    doc.blank = true;
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().blank).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}
```

  b. Ajouter dans `impl Session`, après `editing()` :

```rust
    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours.
    fn unsaved(&self) -> bool {
        self.doc.dirty || matches!(&self.mode, Mode::Templates { stash, .. } if stash.dirty)
    }

    /// Nom d'un document : son fichier, sinon le titre de son deck.
    fn document_name(&self, doc: &Document) -> String {
        let deck = &doc.sources.deck;
        doc.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().trim_end_matches(EXTENSION).to_owned())
            .or_else(|| deck.params.title.as_deref().map(|t| interp(t, &deck.vars)))
            .unwrap_or_else(|| self.localizer.tr("deck-untitled"))
    }

    /// Ce que la barre d'onglets montre de cette session.
    ///
    /// Pendant l'édition des modèles, le titre et le fichier restent ceux du
    /// deck mis de côté.
    pub fn tab_info(&self) -> TabInfo {
        let deck = match &self.mode {
            Mode::Deck => &self.doc,
            Mode::Templates { stash, .. } => stash,
        };
        TabInfo {
            title: self.document_name(deck),
            dirty: self.unsaved(),
            editing_templates: self.editing().is_some(),
            path: deck.path.as_deref().map(oku_slides_platform::storage::resolve),
            pristine: matches!(self.mode, Mode::Deck)
                && deck.blank
                && deck.path.is_none()
                && !deck.dirty
                && !deck.history.can_undo()
                && !deck.history.can_redo(),
        }
    }

    /// Un avis à montrer une fois, avec le prochain snapshot.
    pub fn set_notice(&mut self, text: String) {
        self.doc.notice = Some(text);
    }

    /// Plus rien n'est écrit pour la reprise (l'onglet est fermé).
    #[expect(dead_code, reason = "appelé par l'espace de travail : tâche 2")]
    pub(crate) fn stop_resume(&mut self) {
        self.resume = None;
    }
```

  c. Dans `snapshot()`, remplacer le calcul de `name` (le bloc `let name = doc.path … .unwrap_or_else(|| self.localizer.tr("deck-untitled"));`) par `let name = self.document_name(doc);`.

- [ ] **Step 8 : `resume.rs`.**

  a. Remplacer la doc du module (lignes 1-10) par :

```rust
//! Reprise d'une session (un onglet) : son état, écrit à chaque modification
//! dans son dossier, relu au lancement suivant.
//!
//! `state.json` dit le mode et l'état de l'interface ; chaque document
//! (`deck/`, `templates/`) tient dans un `document.json` (deck, historique,
//! fichier) écrit d'un coup, plus ses assets et polices, écrits une fois par
//! nom.
//!
//! Un seul écrivain par dossier : la session de l'onglet. L'espace de travail
//! (`workspace.rs`) écrit seul `workspace.json`, au-dessus.
```

  Et la doc de `struct Resume` : `/// Un dossier de reprise (celui d'un onglet, ou la racine) et ses fichiers.`

  b. Remplacer `Session::resume` (doc comprise) par :

```rust
    /// Active la reprise dans `dir`, le dossier d'un onglet.
    ///
    /// Relit l'état qui s'y trouve, ou y écrit l'état courant s'il n'y en a
    /// pas. Renvoie `false` si l'état est illisible : il ne doit jamais
    /// coûter le travail non enregistré qu'il contenait peut-être, le dossier
    /// est donc déplacé de côté (jamais écrasé) ; la session repart d'un deck
    /// vierge et n'écrit rien. L'espace de travail retire alors l'onglet.
    #[must_use]
    pub fn resume(&mut self, dir: PathBuf) -> bool {
        let resume = Resume::new(self.storage.clone(), dir);
        if let Err(e) = self.restore(&resume) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise impossible");
            // `restore` peut avoir échoué après avoir déjà basculé en mode
            // modèles (son propre rendu a échoué) : on repart d'un mode
            // cohérent avant `new_deck`, qui ne touche qu'au deck.
            self.mode = Mode::Deck;
            if let Err(e) = self.new_deck() {
                tracing::warn!(error = %e.one_line(), "deck vierge");
            }
            match resume.quarantine() {
                Ok(moved) => tracing::warn!(path = %moved.display(), "reprise en échec mise de côté"),
                Err(e) => tracing::warn!(
                    error = %oku_slides_platform::errors::one_line(&e),
                    "reprise en échec non déplacée"
                ),
            }
            return false;
        }
        self.resume = Some(resume);
        self.persist(true);
        true
    }
```

  c. Supprimer `fn unsaved` et `fn open_at_launch` de `resume.rs`.

  d. `quarantine` devient `pub(crate) fn quarantine(&self) -> Result<PathBuf>` ; la première phrase de sa doc devient « Déplace ce dossier (celui d'un onglet, ou la racine) vers un dossier voisin `<nom>.failed-<horodatage>[-<n>]`… ».

- [ ] **Step 9 : clé Fluent retirée** des 20 locales :

```bash
sed -i '' '/^notice-resume-kept-unsaved = /d' crates/oku-slides-i18n/src/locales/*/session.ftl
grep -rn "notice-resume-kept-unsaved" crates src-tauri ui/src
```

  Le `grep` ne trouve rien.

- [ ] **Step 10 : `src-tauri/src/main.rs`** (transitoire, remplacé en tâche 6 ; un fichier reçu pendant l'édition de modèles non modifiés est ouvert à la place des modèles, accepté pour ces quelques commits) : remplacer `session.resume(platform.settings.paths.data().join("resume"), file.as_deref());` par :

```rust
            if !session.resume(platform.settings.paths.data().join("resume")) {
                tracing::warn!("reprise illisible : deck vierge");
            }
            // Le travail non enregistré passe avant le fichier demandé.
            if let Some(file) = file {
                if session.tab_info().dirty {
                    tracing::warn!(path = %file.display(), "travail non enregistré gardé : fichier non ouvert");
                } else if let Err(e) = session.open(&file) {
                    tracing::warn!(path = %file.display(), error = %e.one_line(), "ouverture au démarrage");
                }
            }
```

- [ ] **Step 11 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n -p oku-slides-platform && cargo clippy -p oku-slides-app -- -D warnings` → vert.

- [ ] **Step 12 : commit**

```bash
git add crates/oku-slides-platform crates/oku-slides-session crates/oku-slides-i18n src-tauri/src/main.rs
git commit -m "refactor(session) : reprise par dossier d'onglet, infos d'onglet (t-147)"
oku cli task done --project slider --id t-147
```

---

### Task 2 : Espace de travail — fenêtres et onglets en mémoire (t-148)

**Files:**
- Create: `crates/oku-slides-session/src/workspace.rs`, `workspace_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (module, réexports, doc du crate), `error.rs`
- Modify: `crates/oku-slides-platform/src/storage.rs`, `storage_tests.rs` (`delete_dir`)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (3 clés)

**Interfaces:**
- Consumes: `Session::{new, open, resume, tab_info, stop_resume, set_notice, handle}`, `TabInfo`, `storage::resolve` (tâche 1).
- Produces (réexportés à la racine) :
  - `pub struct Workspace` ;
    `Workspace::new(fonts: Fonts, platform: &Platform, file: Option<&Path>) -> Result<Workspace>` (une fenêtre, un onglet `t-1` : vierge, ou `file` ouvert dedans ; rien n'est écrit) ;
    `Workspace::apply(ws: &Mutex<Workspace>, req: WorkspaceRequest) -> Result<String>` (JSON : la vue, ou les messages) ;
    `Workspace::request(ws: &Mutex<Workspace>, tab: &str, req: Request) -> (Result<String>, bool)` (réponse, barre à redessiner) ;
    `Workspace::lock(ws: &Mutex<Workspace>) -> MutexGuard<'_, Workspace>` (repris s'il est empoisonné) ;
    `view(&self) -> WorkspaceView` ; `focused(&self) -> &str` ; `focus(&mut self, label: &str) -> Result<()>` (en mémoire seulement) ;
    `window_at(&self, x: f64, y: f64, rects: &[(String, Rect)]) -> Option<String>`.
  - `pub enum WorkspaceRequest` (serde `tag = "cmd"`, `snake_case`) : `View`, `Messages`, `NewTab { window }`, `OpenTab { window, path }`, `Activate { tab }`, `Close { tab, discard: bool }`, `Reorder { tab, index }`, `MoveTab { tab, window: Option<String>, index: Option<usize> }`, `NewWindow`, `CloseWindow { window, discard: Vec<String> }`.
  - `pub struct WorkspaceView { focused, windows: Vec<WindowView> }`, `WindowView { label, active, tabs: Vec<TabView> }`, `TabView { id, #[serde(flatten)] info: TabInfo }`.
  - `pub struct Rect { x, y, w, h }` (unités de l'hôte).
  - `SessionErrorKind::{UnknownTab, UnknownWindow, Unsaved}`.
  - `pub fn Storage::delete_dir(&self, dir: &Path) -> Result<(), StorageError>` ; `Storage::write_atomic` avec un fichier temporaire propre à chaque écriture (deux onglets peuvent écrire en parallèle).
  - Pour la tâche 3 (privés au module) : `Layout`, `WindowLayout`, `Tab`, `is_tab_id`, `is_window_label`, `number`, `MAX_NUMBER`, `LAYOUT_VERSION`, `TABS_DIR`, `Trash`, `Workspace::{empty, add_window, insert_tab, pending, save_layout, touch}`, `relock`.

- [ ] **Step 1a : écriture atomique concurrente.** Test qui échoue, dans `storage_tests.rs` :

```rust
/// Deux écritures simultanées du même fichier (deux onglets qui enregistrent
/// le même ensemble de modèles) : aucune ne perd son fichier temporaire.
#[test]
fn write_atomic_concurrent_writes_both_succeed() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("atomic-race");
    let path = dir.join("set.json");
    let writers: Vec<_> = (0..8u8)
        .map(|n| {
            let (s, path) = (s.clone(), path.clone());
            std::thread::spawn(move || (0..20).try_for_each(|_| s.write_atomic(&path, vec![n; 4096])))
        })
        .collect();
    for w in writers {
        assert_that!(w.join().unwrap()).is_ok();
    }
    assert_that!(s.read(&path).unwrap().len()).is_equal_to(4096);
    std::fs::remove_dir_all(&dir).ok();
}
```

  `cargo nextest run -p oku-slides-platform write_atomic` → échec (un renommage trouve le fichier temporaire déjà renommé par un autre fil). Puis, dans `storage.rs`, le nom temporaire devient unique :

```rust
    /// Écrit dans un fichier temporaire à côté, puis renomme : un lecteur ne
    /// voit jamais de fichier à moitié écrit.
    ///
    /// Le nom temporaire est propre à chaque écriture : deux écritures
    /// simultanées du même fichier ne se marchent pas dessus.
    pub fn write_atomic(&self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<(), StorageError> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let tmp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        self.write(&tmp, bytes)?;
        self.rename(&tmp, path)
    }
```

  Le test `write_atomic_replaces_content_and_leaves_no_temp_file` existant doit rester vert (il vérifie qu'aucun `.tmp` ne reste). → vert.

- [ ] **Step 1b : test de `delete_dir` qui échoue**, dans `storage_tests.rs` :

```rust
#[test]
fn delete_dir_removes_the_tree_and_ignores_missing_dirs() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("rmdir");
    s.write(&dir.join("t-1/deck/document.json"), vec![1]).unwrap();
    s.delete_dir(&dir.join("t-1")).unwrap();
    assert_that!(dir.join("t-1").exists()).is_false();
    assert_that!(dir.exists()).is_true();
    assert_that!(s.delete_dir(&dir.join("absent"))).is_ok();
    std::fs::remove_dir_all(&dir).ok();
}
```

  `cargo nextest run -p oku-slides-platform delete_dir` → échec (absent). Puis, après `delete` dans `storage.rs` :

```rust
    /// Supprime un dossier et tout son contenu ; un dossier absent n'est pas une erreur.
    pub fn delete_dir(&self, dir: &Path) -> Result<(), StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let opts = opendal::options::DeleteOptions { recursive: true, ..Default::default() };
        Ok(op.delete_options(&key, opts).context(AccessSnafu { action: "delete", path: dir })?)
    }
```

  → vert.

- [ ] **Step 2 : squelette qui rend les tests visibles.** Créer `crates/oku-slides-session/src/workspace.rs` avec seulement :

```rust
//! Espace de travail : les fenêtres, leurs onglets, et une session par onglet.

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
```

  et `mod workspace;` dans la liste des modules de `lib.rs`.

- [ ] **Step 3 : tests qui échouent.** Créer `crates/oku-slides-session/src/workspace_tests.rs` :

```rust
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Request, SessionErrorKind, Settings};

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Data {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "oku-slides-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap()
}

fn workspace_in(data: &Data) -> Mutex<Workspace> {
    Mutex::new(Workspace::new(Fonts::default(), &platform(&data.0), None).unwrap())
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) -> Result<String> {
    Workspace::apply(ws, serde_json::from_value(v).unwrap())
}

fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

fn ids(w: &WindowView) -> Vec<&str> {
    w.tabs.iter().map(|t| t.id.as_str()).collect()
}

fn set_theme() -> Request {
    serde_json::from_value(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": "pro"}]}))
        .unwrap()
}

/// Un vrai `.oku-slides.pdf` : un deck JSON n'a pas de fichier associé.
fn saved_pdf(data: &Data, name: &str) -> PathBuf {
    let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    let mut s = Session::new(Fonts::default(), &platform(&data.0)).unwrap();
    s.open(&fixture).unwrap();
    let path = data.0.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap();
    path
}

#[test]
fn new_workspace_has_one_window_with_one_pristine_tab() {
    let data = Data::new();
    let v = view(&workspace_in(&data));
    assert_that!(v.windows).has_length(1);
    let w = &v.windows[0];
    assert_that!(w.label.as_str()).is_equal_to("w-1");
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.active.as_str()).is_equal_to("t-1");
    assert_that!(w.tabs[0].info.pristine).is_true();
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
}

#[test]
fn new_workspace_with_file_opens_it_in_the_first_tab() {
    let data = Data::new();
    let file = saved_pdf(&data, "essai");
    let ws = Mutex::new(Workspace::new(Fonts::default(), &platform(&data.0), Some(&file)).unwrap());
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file));
    assert_that!(w.tabs[0].info.pristine).is_false();
}

/// Un deck vierge enregistré a un fichier : il n'est plus remplacé.
#[test]
fn open_tab_after_saving_the_blank_deck_adds_a_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let saved = data.0.join("vierge.oku-slides.pdf");
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": saved})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
}

/// Un verrou empoisonné (panique d'un autre fil) est repris : l'app continue.
#[test]
fn workspace_lock_poisoned_by_a_panic_still_serves_requests() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_in(&data));
    let poisoner = std::sync::Arc::clone(&ws);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("panique volontaire");
    })
    .join();
    assert_that!(ws.is_poisoned()).is_true();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    assert_that!(view(&ws).windows[0].tabs).has_length(2);
}

#[test]
fn new_tab_appends_and_activates_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_with_pristine_active_tab_replaces_it_with_a_new_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file));
}

#[test]
fn open_tab_with_edited_active_tab_adds_a_tab_holding_the_file() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.dirty).is_true();
    assert_that!(w.tabs[0].info.path).is_none();
    assert_that!(w.tabs[1].info.path).is_equal_to(Some(file));
}

#[test]
fn open_tab_already_open_in_other_window_activates_it_and_focuses_its_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    assert_that!(Workspace::lock(&ws).focused()).is_equal_to("w-2");
    send(&ws, json!({"cmd": "open_tab", "window": "w-2", "path": file})).unwrap();
    let v = view(&ws);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
    assert_that!(v.windows[1].tabs).has_length(1);
    assert_that!(v.windows[1].tabs[0].info.pristine).is_true();
}

#[test]
fn open_tab_same_file_spelled_differently_activates_the_existing_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let other = data.0.join("x/../essai.oku-slides.pdf");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": other})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(w.tabs).has_length(2);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_unreadable_file_returns_error_and_keeps_the_tabs() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let missing = data.0.join("absent.oku-slides.pdf");
    assert_that!(send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": missing}))).is_err();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.pristine).is_true();
}

#[test]
fn close_active_tab_activates_its_right_neighbour() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "activate", "tab": "t-2"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(w.active.as_str()).is_equal_to("t-3");
}

#[test]
fn close_rightmost_active_tab_activates_its_left_neighbour() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-1");
}

#[test]
fn close_edited_tab_without_discard_returns_unsaved_and_keeps_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.dirty).is_true();
}

/// Seul l'ensemble de modèles en cours est modifié : c'est du travail aussi.
#[test]
fn close_tab_with_only_edited_templates_returns_unsaved() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let edit: Request =
        serde_json::from_value(json!({"cmd": "edit_templates", "set": crate::DEFAULT_SET})).unwrap();
    Workspace::request(&ws, "t-1", edit).0.unwrap();
    let first = Workspace::lock(&ws).session("t-1").unwrap().lock().unwrap().deck().slides[0].id.clone();
    let remove: Request = serde_json::from_value(
        json!({"cmd": "apply", "label": "Retirer", "ops": [{"op": "remove_slide", "slide": first}]}),
    )
    .unwrap();
    Workspace::request(&ws, "t-1", remove).0.unwrap();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
}

#[test]
fn close_edited_tab_with_discard_closes_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    Workspace::request(&ws, "t-2", set_theme()).0.unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_only_tab_of_a_window_removes_the_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
}

#[test]
fn close_window_with_an_edited_tab_not_discarded_returns_unsaved_and_closes_nothing() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    // Un onglet propre d'abord, le modifié ensuite : rien ne doit être fermé.
    send(&ws, json!({"cmd": "new_tab", "window": "w-2"})).unwrap();
    Workspace::request(&ws, "t-3", set_theme()).0.unwrap();
    let err = send(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    assert_that!(ids(&view(&ws).windows[1])).is_equal_to(vec!["t-2", "t-3"]);
    send(&ws, json!({"cmd": "close_window", "window": "w-2", "discard": ["t-3"]})).unwrap();
    assert_that!(view(&ws).windows).has_length(1);
    let gone = Workspace::request(&ws, "t-2", Request::Snapshot).0.unwrap_err();
    assert_that!(gone.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn reorder_moves_the_tab_to_its_new_index_clamped() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 0})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-3", "t-1", "t-2"]);
    send(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 99})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2", "t-3"]);
}

#[test]
fn move_tab_to_other_window_at_index_activates_it_there() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-2", "index": 0})).unwrap();
    let v = view(&ws);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(v.windows[1].active.as_str()).is_equal_to("t-1");
    assert_that!(v.focused.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_its_own_window_without_index_changes_nothing() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1"})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1", "index": 1})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-2", "t-1"]);
}

#[test]
fn move_tab_to_new_window_creates_a_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(2);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1"]);
}

#[test]
fn move_only_tab_to_new_window_removes_its_old_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].label.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_unknown_window_returns_error_and_keeps_the_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let err = send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn unknown_tab_returns_unknown_tab_error() {
    let data = Data::new();
    let err = send(&workspace_in(&data), json!({"cmd": "activate", "tab": "t-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let err = send(&workspace_in(&data), json!({"cmd": "new_tab", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

#[test]
fn focus_unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let err = Workspace::lock(&ws).focus("w-9").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

fn rect(x: f64, y: f64) -> Rect {
    Rect { x, y, w: 100.0, h: 100.0 }
}

#[test]
fn window_at_inside_the_only_window_returns_it_edges_included() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0))];
    let w = Workspace::lock(&ws);
    assert_that!(w.window_at(0.0, 0.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    assert_that!(w.window_at(100.0, 50.0, &rects)).is_none();
}

#[test]
fn window_at_overlap_prefers_the_most_recently_focused_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0)), ("w-2".to_owned(), rect(50.0, 0.0))];
    let mut w = Workspace::lock(&ws);
    w.focus("w-1").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    w.focus("w-2").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-2".to_owned()));
    // Seule w-1 contient ce point, même si w-2 est la plus récente.
    assert_that!(w.window_at(25.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
}

#[test]
fn window_at_ignores_rects_of_unknown_windows() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let rects = vec![("autre".to_owned(), rect(0.0, 0.0))];
    assert_that!(Workspace::lock(&ws).window_at(50.0, 50.0, &rects)).is_none();
}

#[test]
fn request_reports_a_change_of_tab_info_only_once() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let (reply, changed) = Workspace::request(&ws, "t-1", Request::Snapshot);
    assert_that!(reply).is_ok();
    assert_that!(changed).is_false();
    assert_that!(Workspace::request(&ws, "t-1", set_theme()).1).is_true();
    assert_that!(Workspace::request(&ws, "t-1", Request::Snapshot).1).is_false();
    assert_that!(Workspace::request(&ws, "t-9", Request::Snapshot).1).is_false();
}

/// L'ordre des verrous : une requête d'onglet et la fermeture du même
/// onglet, en parallèle, ne s'interbloquent pas (un interblocage échoue au
/// bout de 10 s au lieu de bloquer la suite de tests).
#[test]
fn request_and_close_of_the_same_tab_in_parallel_do_not_deadlock() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_in(&data));
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let (done, finished) = std::sync::mpsc::channel();
    {
        let (ws, done) = (std::sync::Arc::clone(&ws), done.clone());
        std::thread::spawn(move || {
            for _ in 0..20 {
                let _ = Workspace::request(&ws, "t-2", Request::Snapshot);
            }
            done.send(()).unwrap();
        });
    }
    {
        let ws = std::sync::Arc::clone(&ws);
        std::thread::spawn(move || {
            send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
            done.send(()).unwrap();
        });
    }
    for _ in 0..2 {
        finished.recv_timeout(std::time::Duration::from_secs(10)).expect("interblocage");
    }
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_last_tab_of_last_window_leaves_no_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).is_empty();
    assert_that!(v.focused.as_str()).is_equal_to("");
}

#[test]
fn messages_give_the_ui_its_language_and_fluent_resources() {
    let data = Data::new();
    let json = send(&workspace_in(&data), json!({"cmd": "messages"})).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["locale"].as_str()).is_equal_to(Some("fr"));
    let resources: Vec<String> = serde_json::from_value(v["resources"].clone()).unwrap();
    let expected: Vec<String> =
        crate::Localizer::new(Some("fr")).ui_messages().into_iter().map(String::from).collect();
    assert_that!(resources).is_equal_to(expected);
}

#[test]
fn is_tab_id_accepts_only_t_and_digits() {
    assert_that!(is_tab_id("t-12")).is_true();
    assert_that!(is_tab_id("t-999999999999999999")).is_true();
    for bad in ["t-", "t-1a", "../t-1", "/tmp", "t-1234567890123456789", "t-12345678901234567890", "w-1"] {
        asserting(bad).that(&is_tab_id(bad)).is_false();
    }
    assert_that!(is_window_label("w-3")).is_true();
    assert_that!(is_window_label("main")).is_false();
}
```

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session workspace` → échec de compilation (`Workspace`, `WorkspaceRequest`… absents).

- [ ] **Step 5 : erreurs.** Dans `error.rs` :
  - `SessionErrorKind`, après `Response` :

```rust
    /// Onglet inconnu (fermé entre-temps).
    UnknownTab,
    /// Fenêtre inconnue (fermée entre-temps).
    UnknownWindow,
    /// Fermeture refusée : travail non enregistré.
    Unsaved,
```

  - `SessionInner` :

```rust
    #[snafu(display("tab not found: {id}"))]
    UnknownTab { id: String, backtrace: Backtrace },
    #[snafu(display("window not found: {label}"))]
    UnknownWindow { label: String, backtrace: Backtrace },
    #[snafu(display("{title} has unsaved changes"))]
    Unsaved { title: String, backtrace: Backtrace },
```

  - `kind()` : `UnknownTab { .. } => SessionErrorKind::UnknownTab, UnknownWindow { .. } => SessionErrorKind::UnknownWindow, Unsaved { .. } => SessionErrorKind::Unsaved,`
  - `localize()` : `UnknownTab { .. } => l.tr("session-unknown-tab"), UnknownWindow { .. } => l.tr("session-unknown-window"), Unsaved { title, .. } => l.tr_with("session-unsaved", &[("title", title.as_str().into())]),`

  Clés Fluent, après `session-busy` dans chaque `session.ftl` :

| locale | `session-unknown-tab` | `session-unknown-window` | `session-unsaved` |
|---|---|---|---|
| en | This tab is no longer open. | This window is no longer open. | “{ $title }” has unsaved changes. |
| fr | Cet onglet n'est plus ouvert. | Cette fenêtre n'est plus ouverte. | « { $title } » a des modifications non enregistrées. |
| de | Dieser Tab ist nicht mehr geöffnet. | Dieses Fenster ist nicht mehr geöffnet. | „{ $title }“ hat nicht gespeicherte Änderungen. |
| es | Esta pestaña ya no está abierta. | Esta ventana ya no está abierta. | «{ $title }» tiene cambios sin guardar. |
| it | Questa scheda non è più aperta. | Questa finestra non è più aperta. | «{ $title }» ha modifiche non salvate. |
| pt | Esta aba não está mais aberta. | Esta janela não está mais aberta. | “{ $title }” tem alterações não salvas. |
| nl | Dit tabblad is niet meer geopend. | Dit venster is niet meer geopend. | ‘{ $title }’ heeft niet-opgeslagen wijzigingen. |
| pl | Ta karta nie jest już otwarta. | To okno nie jest już otwarte. | „{ $title }” ma niezapisane zmiany. |
| ru | Эта вкладка больше не открыта. | Это окно больше не открыто. | В «{ $title }» есть несохранённые изменения. |
| uk | Ця вкладка більше не відкрита. | Це вікно більше не відкрите. | У «{ $title }» є незбережені зміни. |
| tr | Bu sekme artık açık değil. | Bu pencere artık açık değil. | “{ $title }” kaydedilmemiş değişiklikler içeriyor. |
| ar | لم تعد علامة التبويب هذه مفتوحة. | لم تعد هذه النافذة مفتوحة. | في «{ $title }» تغييرات غير محفوظة. |
| hi | यह टैब अब खुला नहीं है। | यह विंडो अब खुली नहीं है। | “{ $title }” में बिना सहेजे बदलाव हैं। |
| bn | এই ট্যাবটি আর খোলা নেই। | এই উইন্ডোটি আর খোলা নেই। | “{ $title }”-এ অসংরক্ষিত পরিবর্তন আছে। |
| ja | このタブはもう開いていません。 | このウィンドウはもう開いていません。 | 「{ $title }」に未保存の変更があります。 |
| ko | 이 탭은 더 이상 열려 있지 않습니다. | 이 창은 더 이상 열려 있지 않습니다. | “{ $title }”에 저장하지 않은 변경 사항이 있습니다. |
| zh-Hans | 此标签页已不再打开。 | 此窗口已不再打开。 | “{ $title }”有未保存的更改。 |
| zh-Hant | 此分頁已不再開啟。 | 此視窗已不再開啟。 | 「{ $title }」有未儲存的變更。 |
| th | แท็บนี้ไม่ได้เปิดอยู่แล้ว | หน้าต่างนี้ไม่ได้เปิดอยู่แล้ว | “{ $title }” มีการเปลี่ยนแปลงที่ยังไม่บันทึก |
| vi | Thẻ này không còn mở nữa. | Cửa sổ này không còn mở nữa. | “{ $title }” có thay đổi chưa lưu. |

- [ ] **Step 6 : `workspace.rs`** (remplace le squelette ; la ligne `mod workspace_tests` reste en fin de fichier).

```rust
//! Espace de travail : les fenêtres, leurs onglets, et une session par onglet.
//!
//! Chaque fenêtre a ses onglets, dans l'ordre de sa barre, et un onglet
//! actif ; un onglet n'est que dans une fenêtre. Les sessions sont partagées
//! (`Arc<Mutex<_>>`) : une requête d'onglet ne verrouille que la sienne.
//!
//! Ordre des verrous : l'espace de travail, puis une session, jamais
//! l'inverse. Seules [`Workspace::apply`], [`Workspace::request`] et
//! `Workspace::open_files` combinent les deux ; une session neuve se
//! construit hors du verrou de l'espace de travail.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_platform::storage::{Storage, resolve};

use crate::error::{ResponseSnafu, Result, UnknownTabSnafu, UnknownWindowSnafu, UnsavedSnafu};
use crate::views::TabInfo;
use crate::{Fonts, Platform, Request, Session};

/// Version de `workspace.json`.
const LAYOUT_VERSION: u32 = 2;
/// Sous-dossier des onglets, dans le dossier de reprise.
const TABS_DIR: &str = "tabs";

/// Plus grand numéro d'onglet ou de fenêtre : 18 chiffres.
#[expect(dead_code, reason = "reprise : tâche 3")]
const MAX_NUMBER: u64 = 999_999_999_999_999_999;

/// `prefix` suivi de 1 à 18 chiffres (un `u64` sans débordement).
fn numbered(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|n| (1..=18).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Identifiant d'onglet sûr dans un chemin : `t-` et des chiffres.
fn is_tab_id(id: &str) -> bool {
    numbered(id, "t-")
}

/// Label de fenêtre accepté par Tauri et par la capacité `w-*`.
#[cfg_attr(not(test), expect(dead_code, reason = "reprise : tâche 3"))]
fn is_window_label(label: &str) -> bool {
    numbered(label, "w-")
}

/// Le numéro d'un identifiant déjà validé.
#[expect(dead_code, reason = "reprise : tâche 3")]
fn number(id: &str) -> u64 {
    id.get(2..).and_then(|n| n.parse().ok()).unwrap_or(0)
}

/// Un verrou, repris s'il est empoisonné : une panique ne bloque pas l'app.
fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Une requête de la barre d'onglets ou de l'hôte.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WorkspaceRequest {
    /// La vue, sans rien changer.
    View,
    /// Langue de l'utilisateur et messages Fluent de l'interface.
    Messages,
    /// Un onglet vierge, à la fin de `window`.
    NewTab {
        /// Label de la fenêtre.
        window: String,
    },
    /// Ouvre un fichier : son onglet s'il est déjà ouvert, sinon un nouvel onglet.
    OpenTab {
        /// Fenêtre d'arrivée d'un nouvel onglet.
        window: String,
        /// Le fichier (normalisé à la réception).
        path: PathBuf,
    },
    /// L'onglet devient actif dans sa fenêtre.
    Activate {
        /// Identifiant de l'onglet.
        tab: String,
    },
    /// Ferme un onglet.
    Close {
        /// Identifiant de l'onglet.
        tab: String,
        /// L'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: bool,
    },
    /// Place l'onglet à `index` dans sa barre.
    Reorder {
        /// Identifiant de l'onglet.
        tab: String,
        /// Place finale (bornée à la barre).
        index: usize,
    },
    /// Vers une autre fenêtre, ou une nouvelle.
    MoveTab {
        /// Identifiant de l'onglet.
        tab: String,
        /// Fenêtre d'arrivée ; `None` : une nouvelle fenêtre.
        #[serde(default)]
        window: Option<String>,
        /// Place dans la barre d'arrivée ; `None` : à la fin.
        #[serde(default)]
        index: Option<usize>,
    },
    /// Une fenêtre avec un onglet vierge.
    NewWindow,
    /// Ferme une fenêtre et ses onglets.
    CloseWindow {
        /// Label de la fenêtre.
        window: String,
        /// Onglets dont l'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: Vec<String>,
    },
}

/// Un onglet de la vue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabView {
    /// Identifiant `t-<n>`.
    pub id: String,
    /// Titre, travail non enregistré, fichier.
    #[serde(flatten)]
    pub info: TabInfo,
}

/// Une fenêtre de la vue : ses onglets et l'onglet actif.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowView {
    /// Label `w-<n>`, aussi celui de la fenêtre Tauri.
    pub label: String,
    /// Identifiant de l'onglet actif.
    pub active: String,
    /// Onglets, dans l'ordre de la barre.
    pub tabs: Vec<TabView>,
}

/// Ce que l'interface voit : les fenêtres et leurs onglets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceView {
    /// Dernière fenêtre active.
    pub focused: String,
    /// Fenêtres, dans l'ordre de création.
    pub windows: Vec<WindowView>,
}

/// Un rectangle à l'écran, dans les unités de l'hôte (celles du point comparé).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

impl Rect {
    /// Le point est dedans (bords gauche et haut compris).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Fenêtres et onglets, tels qu'écrits dans `workspace.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Layout {
    version: u32,
    /// Prochains numéros : jamais réutilisés.
    next_tab: u64,
    next_window: u64,
    focused: String,
    windows: Vec<WindowLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowLayout {
    label: String,
    tabs: Vec<String>,
    active: String,
}

impl Layout {
    fn new() -> Self {
        Self { version: LAYOUT_VERSION, next_tab: 1, next_window: 1, focused: String::new(), windows: Vec::new() }
    }
}

struct Tab {
    session: Arc<Mutex<Session>>,
    /// Dernières infos connues : la vue se construit sans verrouiller les sessions.
    info: TabInfo,
}

impl Tab {
    fn new(session: Session) -> Self {
        Self { info: session.tab_info(), session: Arc::new(Mutex::new(session)) }
    }
}

/// De quoi supprimer les dossiers d'onglets fermés, verrou rendu.
struct Trash {
    storage: Storage,
    dir: Option<PathBuf>,
}

impl Trash {
    /// Chaque session n'écrit plus rien, puis son dossier est supprimé.
    ///
    /// Toujours après l'écriture de `workspace.json` : jamais d'onglet cité
    /// sans dossier.
    fn empty(&self, gone: Vec<(String, Tab)>) {
        for (id, tab) in gone {
            relock(&tab.session).stop_resume();
            let Some(dir) = &self.dir else { continue };
            // Double contrôle : jamais de suppression hors de `tabs/`.
            if !is_tab_id(&id) {
                tracing::warn!(tab = %id, "identifiant d'onglet refusé : rien n'est supprimé");
                continue;
            }
            let path = dir.join(TABS_DIR).join(&id);
            if let Err(e) = self.storage.delete_dir(&path) {
                tracing::warn!(path = %path.display(), error = %oku_slides_platform::errors::one_line(&e), "dossier d'onglet");
            }
        }
    }
}

/// Un onglet à créer hors du verrou : tout ce qu'il faut pour sa session.
struct Pending {
    id: String,
    /// Fenêtre d'arrivée ; `None` : une nouvelle fenêtre.
    window: Option<String>,
    path: Option<PathBuf>,
    /// Onglet vierge à fermer une fois le nouveau prêt.
    replace: Option<String>,
    fonts: Fonts,
    platform: Platform,
    dir: Option<PathBuf>,
}

impl Pending {
    /// La session de l'onglet : créée, fichier ouvert, reprise activée.
    fn build(&self) -> Result<Session> {
        let mut session = Session::new(self.fonts.clone(), &self.platform)?;
        if let Some(path) = &self.path {
            session.open(path)?;
        }
        if let Some(dir) = &self.dir
            && !session.resume(dir.join(TABS_DIR).join(&self.id))
        {
            tracing::warn!(tab = %self.id, "reprise de l'onglet inactive");
        }
        Ok(session)
    }
}

/// Les fenêtres, leurs onglets et une session par onglet.
pub struct Workspace {
    fonts: Fonts,
    platform: Platform,
    layout: Layout,
    /// Fenêtres, de la moins à la plus récemment active.
    recent: Vec<String>,
    tabs: HashMap<String, Tab>,
    /// Dossier de reprise ; `None` : rien n'est écrit (serveur de dev, tests).
    dir: Option<PathBuf>,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace").field("layout", &self.layout).field("dir", &self.dir).finish_non_exhaustive()
    }
}

impl Workspace {
    /// Une fenêtre et un onglet `t-1`, avec `file` ouvert ; rien n'est écrit.
    ///
    /// Sans fichier, l'onglet est vierge (tests, serveur lancé sans fichier).
    pub fn new(fonts: Fonts, platform: &Platform, file: Option<&Path>) -> Result<Self> {
        let mut ws = Self::empty(fonts, platform, None);
        let pending = ws.pending(None, file.map(resolve), None);
        let session = pending.build()?;
        ws.insert_tab(None, pending.id, session, None);
        Ok(ws)
    }

    fn empty(fonts: Fonts, platform: &Platform, dir: Option<PathBuf>) -> Self {
        Self { fonts, platform: platform.clone(), layout: Layout::new(), recent: Vec::new(), tabs: HashMap::new(), dir }
    }

    /// Le verrou de l'espace de travail, repris s'il est empoisonné.
    ///
    /// Ne jamais le garder pendant un appel à `apply`, `request` ou
    /// `open_files` : le verrou n'est pas réentrant.
    pub fn lock(ws: &Mutex<Self>) -> MutexGuard<'_, Self> {
        relock(ws)
    }

    /// Dernière fenêtre active.
    pub fn focused(&self) -> &str {
        &self.layout.focused
    }

    /// `label` passe au premier plan ; rien n'est écrit (le prochain changement le fera).
    pub fn focus(&mut self, label: &str) -> Result<()> {
        self.window_index(label)?;
        self.touch(label);
        Ok(())
    }

    /// Les fenêtres et leurs onglets.
    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
            focused: self.layout.focused.clone(),
            windows: self
                .layout
                .windows
                .iter()
                .map(|w| WindowView {
                    label: w.label.clone(),
                    active: w.active.clone(),
                    tabs: w
                        .tabs
                        .iter()
                        .filter_map(|id| self.tabs.get(id).map(|t| TabView { id: id.clone(), info: t.info.clone() }))
                        .collect(),
                })
                .collect(),
        }
    }

    /// La fenêtre sous le point `(x, y)` : la plus récemment active qui le contient.
    pub fn window_at(&self, x: f64, y: f64, rects: &[(String, Rect)]) -> Option<String> {
        self.recent
            .iter()
            .rev()
            .find(|label| rects.iter().any(|(l, r)| l == *label && r.contains(x, y)))
            .cloned()
    }

    /// Une requête pour l'onglet `tab` ; `true` : la barre est à redessiner.
    ///
    /// Seule la session de l'onglet est verrouillée pendant le rendu ;
    /// l'espace de travail l'est avant et après, jamais en même temps.
    pub fn request(ws: &Mutex<Self>, tab: &str, req: Request) -> (Result<String>, bool) {
        let session = match Self::lock(ws).session(tab) {
            Ok(session) => session,
            Err(e) => return (Err(e), false),
        };
        let (reply, info) = {
            let mut s = relock(&session);
            let reply = s.handle(req);
            (reply, s.tab_info())
        };
        let changed = Self::lock(ws).note(tab, info);
        (reply, changed)
    }

    /// Traite une requête de structure ; renvoie la vue (ou les messages), en JSON.
    ///
    /// Un onglet neuf est construit hors du verrou : numéro réservé, session
    /// créée et ouverte, puis insérée. Les dossiers des onglets fermés sont
    /// supprimés après l'écriture de `workspace.json`, hors du verrou aussi.
    pub fn apply(ws: &Mutex<Self>, req: WorkspaceRequest) -> Result<String> {
        let pending = {
            let mut w = Self::lock(ws);
            match req {
                WorkspaceRequest::View => return w.view_json(),
                WorkspaceRequest::Messages => {
                    let l = &w.platform.localizer;
                    let messages = serde_json::json!({ "locale": l.locale(), "resources": l.ui_messages() });
                    return Ok(serde_json::to_string(&messages).context(ResponseSnafu)?);
                }
                WorkspaceRequest::NewTab { window } => {
                    w.window_index(&window)?;
                    w.pending(Some(window), None, None)
                }
                WorkspaceRequest::OpenTab { window, path } => {
                    let path = resolve(&path);
                    if let Some(id) = w.tab_with_path(&path) {
                        w.activate(&id)?;
                        w.save_layout();
                        return w.view_json();
                    }
                    let active = w.layout.windows[w.window_index(&window)?].active.clone();
                    w.pending(Some(window), Some(path), Some(active))
                }
                WorkspaceRequest::NewWindow => w.pending(None, None, None),
                other => {
                    let gone = w.change(other)?;
                    w.save_layout();
                    let (json, trash) = (w.view_json(), w.trash());
                    drop(w);
                    trash.empty(gone);
                    return json;
                }
            }
        };
        let session = pending.build()?;
        let Pending { id, window, path, replace, .. } = pending;
        let mut w = Self::lock(ws);
        // Le même fichier a pu être ouvert pendant la construction (Finder et
        // seconde instance, double clic) : le premier onglet reste, seul.
        if let Some(existing) = path.as_deref().and_then(|p| w.tab_with_path(p)) {
            let activated = w.activate(&existing);
            w.save_layout();
            let (json, trash) = (w.view_json(), w.trash());
            drop(w);
            // La session construite pour rien part à la corbeille, même en cas d'erreur.
            trash.empty(vec![(id, Tab::new(session))]);
            activated?;
            return json;
        }
        let window = match window {
            Some(label) if w.window_index(&label).is_ok() => Some(label),
            // Fenêtre disparue pendant la construction : la fenêtre active.
            Some(_) => Some(w.layout.focused.clone()).filter(|l| w.window_index(l).is_ok()),
            None => None,
        };
        // Toujours vierge ? L'état réel de la session fait foi. Sans attendre
        // sous le verrou : une session occupée (un rendu en cours) est en
        // train d'être modifiée, donc pas vierge.
        let replaced = replace.filter(|r| {
            w.session(r).is_ok_and(|s| match s.try_lock() {
                Ok(s) => s.tab_info().pristine,
                Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner().tab_info().pristine,
                Err(std::sync::TryLockError::WouldBlock) => false,
            })
        });
        w.insert_tab(window.as_deref(), id, session, replaced.as_deref());
        let mut gone = Vec::new();
        if let Some(r) = replaced {
            if let Err(e) = w.detach(&r) {
                tracing::warn!(tab = %r, error = %e.one_line(), "onglet vierge non retiré");
            }
            gone.extend(w.tabs.remove(&r).map(|t| (r, t)));
        }
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        json
    }

    /// De quoi supprimer des onglets fermés une fois le verrou rendu.
    fn trash(&self) -> Trash {
        Trash { storage: self.platform.storage.clone(), dir: self.dir.clone() }
    }

    fn view_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.view()).context(ResponseSnafu)?)
    }

    /// Réserve un numéro d'onglet et prépare sa construction hors du verrou.
    fn pending(&mut self, window: Option<String>, path: Option<PathBuf>, replace: Option<String>) -> Pending {
        Pending {
            id: self.next_tab_id(),
            window,
            path,
            replace,
            fonts: self.fonts.clone(),
            platform: self.platform.clone(),
            dir: self.dir.clone(),
        }
    }

    fn next_tab_id(&mut self) -> String {
        let id = format!("t-{}", self.layout.next_tab);
        self.layout.next_tab = self.layout.next_tab.saturating_add(1);
        id
    }

    /// Les requêtes qui ne créent rien ; renvoie les onglets à supprimer
    /// après l'écriture de `workspace.json`.
    fn change(&mut self, req: WorkspaceRequest) -> Result<Vec<(String, Tab)>> {
        match req {
            WorkspaceRequest::Activate { tab } => self.activate(&tab)?,
            WorkspaceRequest::Close { tab, discard } => {
                self.ensure_closable(&tab, discard)?;
                self.detach(&tab)?;
                return Ok(self.tabs.remove(&tab).map(|t| (tab, t)).into_iter().collect());
            }
            WorkspaceRequest::Reorder { tab, index } => self.reorder(&tab, index)?,
            WorkspaceRequest::MoveTab { tab, window, index } => self.move_tab(&tab, window.as_deref(), index)?,
            WorkspaceRequest::CloseWindow { window, discard } => {
                let w = self.window_index(&window)?;
                for tab in &self.layout.windows[w].tabs {
                    self.ensure_closable(tab, discard.contains(tab))?;
                }
                let win = self.layout.windows.remove(w);
                self.forget_window(&window);
                return Ok(win.tabs.into_iter().filter_map(|id| self.tabs.remove(&id).map(|t| (id, t))).collect());
            }
            WorkspaceRequest::View
            | WorkspaceRequest::Messages
            | WorkspaceRequest::NewTab { .. }
            | WorkspaceRequest::OpenTab { .. }
            | WorkspaceRequest::NewWindow => {}
        }
        Ok(Vec::new())
    }

    /// Refuse de fermer un onglet modifié que l'utilisateur n'a pas choisi
    /// d'abandonner : l'état réel de la session fait foi, pas celui de l'UI.
    fn ensure_closable(&self, tab: &str, discard: bool) -> Result<()> {
        let session = self.session(tab)?;
        let info = relock(&session).tab_info();
        ensure!(discard || !info.dirty, UnsavedSnafu { title: info.title });
        Ok(())
    }

    /// La session d'un onglet.
    fn session(&self, tab: &str) -> Result<Arc<Mutex<Session>>> {
        Ok(self.tabs.get(tab).map(|t| Arc::clone(&t.session)).context(UnknownTabSnafu { id: tab })?)
    }

    /// Les infos d'un onglet après une requête ; `true` si elles ont changé.
    fn note(&mut self, tab: &str, info: TabInfo) -> bool {
        match self.tabs.get_mut(tab) {
            Some(t) if t.info != info => {
                t.info = info;
                true
            }
            _ => false,
        }
    }

    fn tab_with_path(&self, path: &Path) -> Option<String> {
        self.tabs.iter().find(|(_, t)| t.info.path.as_deref() == Some(path)).map(|(id, _)| id.clone())
    }

    fn window_index(&self, label: &str) -> Result<usize> {
        Ok(self.layout.windows.iter().position(|w| w.label == label).context(UnknownWindowSnafu { label })?)
    }

    fn window_of(&self, tab: &str) -> Result<usize> {
        Ok(self
            .layout
            .windows
            .iter()
            .position(|w| w.tabs.iter().any(|t| t == tab))
            .context(UnknownTabSnafu { id: tab })?)
    }

    /// `label` devient la fenêtre la plus récemment active.
    fn touch(&mut self, label: &str) {
        self.recent.retain(|l| l != label);
        self.recent.push(label.to_owned());
        self.layout.focused = label.to_owned();
    }

    fn forget_window(&mut self, label: &str) {
        self.recent.retain(|l| l != label);
        if self.layout.focused == label {
            self.layout.focused = self.recent.last().cloned().unwrap_or_default();
        }
    }

    fn add_window(&mut self) -> String {
        let label = format!("w-{}", self.layout.next_window);
        self.layout.next_window = self.layout.next_window.saturating_add(1);
        self.layout.windows.push(WindowLayout { label: label.clone(), tabs: Vec::new(), active: String::new() });
        self.touch(&label);
        label
    }

    /// Insère un onglet et l'active : dans `window` (à la place de `beside`
    /// s'il y est, sinon à la fin), ou dans une nouvelle fenêtre.
    fn insert_tab(&mut self, window: Option<&str>, id: String, session: Session, beside: Option<&str>) {
        // La fenêtre d'abord : un onglet n'existe jamais sans fenêtre.
        let label = match window.filter(|l| self.window_index(l).is_ok()) {
            Some(label) => label.to_owned(),
            None => self.add_window(),
        };
        self.tabs.insert(id.clone(), Tab::new(session));
        if let Some(win) = self.layout.windows.iter_mut().find(|w| w.label == label) {
            let at = beside.and_then(|b| win.tabs.iter().position(|t| t == b)).unwrap_or(win.tabs.len());
            win.tabs.insert(at, id.clone());
            win.active = id;
        }
        self.touch(&label);
    }

    fn activate(&mut self, tab: &str) -> Result<()> {
        let w = self.window_of(tab)?;
        self.layout.windows[w].active = tab.to_owned();
        let label = self.layout.windows[w].label.clone();
        self.touch(&label);
        Ok(())
    }

    /// Retire l'onglet de sa fenêtre (son voisin de droite, sinon de gauche,
    /// devient actif) ; une fenêtre vide est retirée.
    fn detach(&mut self, tab: &str) -> Result<()> {
        let w = self.window_of(tab)?;
        let win = &mut self.layout.windows[w];
        if let Some(i) = win.tabs.iter().position(|t| t == tab) {
            win.tabs.remove(i);
            if win.active == tab {
                win.active = win.tabs.get(i).or_else(|| win.tabs.last()).cloned().unwrap_or_default();
            }
        }
        if win.tabs.is_empty() {
            let label = win.label.clone();
            self.layout.windows.remove(w);
            self.forget_window(&label);
        }
        Ok(())
    }

    fn reorder(&mut self, tab: &str, index: usize) -> Result<()> {
        let w = self.window_of(tab)?;
        let tabs = &mut self.layout.windows[w].tabs;
        let from = tabs.iter().position(|t| t == tab).context(UnknownTabSnafu { id: tab })?;
        let id = tabs.remove(from);
        tabs.insert(index.min(tabs.len()), id);
        Ok(())
    }

    /// Déplace un onglet vers `window`, ou une nouvelle fenêtre ; il y devient actif.
    fn move_tab(&mut self, tab: &str, window: Option<&str>, index: Option<usize>) -> Result<()> {
        let from = self.window_of(tab)?;
        if window == Some(self.layout.windows[from].label.as_str()) {
            // Même fenêtre : un index réordonne ; sans index, rien ne change.
            if let Some(index) = index {
                self.reorder(tab, index)?;
            }
            return self.activate(tab);
        }
        let dest = match window {
            Some(label) => {
                self.window_index(label)?;
                label.to_owned()
            }
            None => self.add_window(),
        };
        self.detach(tab)?;
        let d = self.window_index(&dest)?;
        let win = &mut self.layout.windows[d];
        let at = index.unwrap_or(win.tabs.len()).min(win.tabs.len());
        win.tabs.insert(at, tab.to_owned());
        win.active = tab.to_owned();
        self.touch(&dest);
        Ok(())
    }

    /// Écrit `workspace.json` (tâche 3) ; sans dossier de reprise, rien.
    fn save_layout(&self) {
        let Some(_) = &self.dir else { return };
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
```

  Note : `save_layout` n'a rien à écrire tant qu'aucun `Workspace` n'a de dossier (seul `Workspace::resume`, tâche 3, en donne un) ; la tâche 3 le complète. Retirer le `#[expect(dead_code)]` de `Session::stop_resume` (tâche 1) : `Trash::empty` l'appelle.

  Dans `lib.rs` : `pub use workspace::{Rect, TabView, WindowView, Workspace, WorkspaceRequest, WorkspaceView};`. Remplacer la doc du crate par :

```rust
//! Session d'édition et espace de travail : le deck de chaque onglet, son
//! historique, son rendu et son fichier ; les fenêtres et leurs onglets.
//!
//! - [`Session`] et [`Request`] : un deck (une session par onglet).
//! - [`Workspace`], [`WorkspaceRequest`], [`WorkspaceView`], [`TabInfo`] :
//!   les fenêtres, les onglets, la reprise de l'ensemble.
//!
//! L'app Tauri et le serveur de développement exposent la même API. Ordre
//! des verrous : l'espace de travail puis une session, jamais l'inverse ;
//! `Workspace::apply`, `Workspace::request` et `Workspace::open_files` s'en
//! chargent.
```

- [ ] **Step 7 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n -p oku-slides-platform` → vert ; `cargo clippy -p oku-slides-session --all-targets -- -D warnings` → propre (`MAX_NUMBER`, `number` et `is_window_label` portent déjà leur `expect`, retirés en tâche 3).

- [ ] **Step 8 : commit**

```bash
git add crates
git commit -m "feat(session) : espace de travail, fenêtres et onglets en mémoire (t-148)"
oku cli task done --project slider --id t-148
```

---

### Task 3 : Reprise de l'espace de travail et fichiers au lancement (t-149)

**Files:**
- Modify: `crates/oku-slides-platform/src/storage.rs`, `storage_tests.rs` (`list_dirs`)
- Modify: `crates/oku-slides-session/src/resume.rs`, `workspace.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`notice-open-failed`)
- Create: `crates/oku-slides-session/tests/workspace.rs`

**Interfaces:**
- Consumes: tout ce que la tâche 2 produit ; `Session::resume`, `Resume::quarantine`.
- Produces:
  - `pub fn Workspace::resume(fonts: Fonts, platform: &Platform, dir: PathBuf) -> Result<Workspace>`
  - `pub fn Workspace::open_files(ws: &Mutex<Workspace>, files: &[PathBuf])`
  - `pub fn Storage::list_dirs(&self, dir: &Path) -> Result<Vec<PathBuf>, StorageError>`
  - `pub(crate) fn Resume::write_json(&self, name: &str, value: &impl Serialize) -> Result<()>`, `pub(crate) fn Resume::read_json<T: DeserializeOwned>(&self, name: &str, version: u32) -> Result<Option<T>>`
  - `Layout` gagne `#[serde(default)] kept: Vec<String>` : les onglets cités mais non restaurés dont le dossier existe encore ; ils restent cités de lancement en lancement, leur dossier n'est jamais supprimé.
- Ancien format (v1, un `resume/state.json` sans `workspace.json`) : pas géré (décision de l'utilisateur, pas de shim). On efface `<data>/resume/` à la main, une fois, avant le premier lancement de cette version ; sinon il reste là, ignoré.

- [ ] **Step 1 : test de `list_dirs` qui échoue**, dans `storage_tests.rs` :

```rust
#[test]
fn list_dirs_returns_only_subdirectories() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("dirs");
    s.write(&dir.join("a/x.bin"), vec![1]).unwrap();
    s.write(&dir.join("b/y.bin"), vec![1]).unwrap();
    s.write(&dir.join("z.bin"), vec![1]).unwrap();
    let mut names: Vec<String> =
        s.list_dirs(&dir).unwrap().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_that!(names).is_equal_to(vec!["a".to_owned(), "b".to_owned()]);
    assert_that!(s.list_dirs(&dir.join("absent")).unwrap()).is_empty();
    std::fs::remove_dir_all(&dir).ok();
}
```

  `cargo nextest run -p oku-slides-platform list_dirs` → échec. Puis, après `list_files` :

```rust
    /// Sous-dossiers d'un dossier ; un dossier absent est vide.
    pub fn list_dirs(&self, dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
        let (op, key) = self.dir_at(dir)?;
        let entries = match op.list(&key) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).context(AccessSnafu { action: "list", path: dir })?,
        };
        Ok(entries
            .into_iter()
            // La liste d'un dossier peut contenir le dossier lui-même.
            .filter(|e| e.metadata().is_dir() && e.path() != key)
            .map(|e| dir.join(e.name().trim_end_matches('/')))
            .collect())
    }
```

  → vert.

- [ ] **Step 2 : tests d'intégration qui échouent.** Créer `crates/oku-slides-session/tests/workspace.rs` :

```rust
//! Reprise de l'espace de travail : fenêtres, onglets, et chaque session.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_session::{Fonts, Platform, Request, Settings, Workspace, WorkspaceRequest, WorkspaceView};
use serde_json::json;
use speculoos::prelude::*;

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Data {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "oku-slides-ws-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn resume(&self) -> PathBuf {
        self.0.join("resume")
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

// Helpers hors des `#[test]` : `unwrap_or_else(panic!)` plutôt que
// `unwrap()`, que clippy refuse ici (`unwrap_used`), comme dans `session.rs`.

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"))
}

/// L'espace de travail qui reprend l'état laissé dans `data`.
fn resumed(data: &Data) -> Mutex<Workspace> {
    let ws = Workspace::resume(Fonts::default(), &platform(&data.0), data.resume())
        .unwrap_or_else(|e| panic!("reprise impossible : {}", e.one_line()));
    Mutex::new(ws)
}

fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) {
    let req: WorkspaceRequest = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    Workspace::apply(ws, req).unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
}

fn tab(ws: &Mutex<Workspace>, id: &str, v: serde_json::Value) -> serde_json::Value {
    let req: Request = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    let reply = Workspace::request(ws, id, req).0.unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse illisible : {e}"))
}

/// Un vrai `.oku-slides.pdf` : un deck JSON n'a pas de fichier associé.
fn saved_pdf(data: &Data, name: &str) -> PathBuf {
    let mut s = oku_slides_session::Session::new(Fonts::default(), &platform(&data.0))
        .unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()));
    s.open(&fixture()).unwrap_or_else(|e| panic!("fixture illisible : {}", e.one_line()));
    let path = data.0.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("enregistrement impossible : {}", e.one_line()));
    path
}

fn set_theme(theme: &str) -> serde_json::Value {
    json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]})
}

fn fixture() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

fn write_layout(data: &Data, layout: serde_json::Value) {
    let bytes = serde_json::to_vec(&layout).unwrap_or_else(|e| panic!("layout : {e}"));
    std::fs::write(data.resume().join("workspace.json"), bytes).unwrap_or_else(|e| panic!("workspace.json : {e}"));
}

#[test]
fn first_launch_writes_layout_and_tab_folder() {
    let data = Data::new();
    let ws = resumed(&data);
    assert_that!(view(&ws).windows).has_length(1);
    assert_that!(data.resume().join("workspace.json").exists()).is_true();
    assert_that!(data.resume().join("tabs/t-1/state.json").exists()).is_true();
}

/// Deux fenêtres, trois onglets dont un en édition de modèles, onglet actif,
/// fenêtre active non dernière, historique : tout revient.
#[test]
fn resume_restores_windows_tabs_focus_mode_and_history() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(&a, "t-2", set_theme("pro"));
    send(&a, json!({"cmd": "new_window"}));
    tab(&a, "t-3", json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}));
    send(&a, json!({"cmd": "activate", "tab": "t-1"}));
    let before = view(&a);
    assert_that!(before.focused.as_str()).is_equal_to("w-1");
    drop(a);
    let b = resumed(&data);
    assert_that!(view(&b)).is_equal_to(&before);
    assert_that!(view(&b).windows[1].tabs[0].info.editing_templates).is_true();
    let snap = tab(&b, "t-2", json!({"cmd": "undo"}));
    assert_that!(snap["theme"]).is_equal_to(json!("papier"));
}

#[test]
fn resume_with_corrupt_layout_moves_resume_aside_and_starts_blank() {
    let data = Data::new();
    drop(resumed(&data));
    std::fs::write(data.resume().join("workspace.json"), b"pas du json").unwrap();
    let ws = resumed(&data);
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].info.pristine).is_true();
    let aside: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(aside).has_length(1);
}

#[test]
fn resume_with_one_corrupt_tab_keeps_the_others_and_its_data() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#).unwrap();
    let ws = resumed(&data);
    let ids: Vec<String> = view(&ws).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-3".to_owned()]);
    let tabs = names(&data.resume().join("tabs"));
    assert_that!(tabs.iter().filter(|n| n.starts_with("t-2.failed-")).count()).is_equal_to(1);
}

#[test]
fn resume_with_listed_tab_without_folder_drops_it() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::remove_dir_all(data.resume().join("tabs/t-2")).unwrap();
    assert_that!(view(&resumed(&data)).windows[0].tabs).has_length(1);
}

#[test]
fn resume_with_every_tab_lost_opens_one_blank_tab_with_a_fresh_id() {
    let data = Data::new();
    drop(resumed(&data));
    std::fs::remove_dir_all(data.resume().join("tabs/t-1")).unwrap();
    let v = view(&resumed(&data));
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].id.as_str()).is_equal_to("t-2");
}

#[test]
fn resume_removes_orphans_but_keeps_set_aside_and_foreign_folders() {
    let data = Data::new();
    drop(resumed(&data));
    let tabs = data.resume().join("tabs");
    for dir in ["t-7/deck", "t-2.failed-123", "notes"] {
        std::fs::create_dir_all(tabs.join(dir)).unwrap();
    }
    drop(resumed(&data));
    assert_that!(names(&tabs)).is_equal_to(vec!["notes".to_owned(), "t-1".to_owned(), "t-2.failed-123".to_owned()]);
    // Le numéro de l'orphelin n'est jamais réutilisé, et c'est enregistré.
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"}));
    let ids: Vec<String> = view(&ws).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-8".to_owned()]);
}

/// Fermer le dernier onglet de la dernière fenêtre : rien n'est gardé, le
/// lancement suivant ouvre un onglet vierge neuf.
#[test]
fn resume_after_closing_the_last_tab_opens_one_blank_tab() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "close", "tab": "t-1"}));
    assert_that!(view(&ws).windows).is_empty();
    drop(ws);
    let v = view(&resumed(&data));
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].id.as_str()).is_equal_to("t-2");
    assert_that!(data.resume().join("tabs/t-1").exists()).is_false();
}

/// Un identifiant altéré ne sort jamais du dossier : ni lu, ni supprimé.
#[test]
fn resume_with_tampered_ids_ignores_them_and_touches_nothing_outside() {
    let data = Data::new();
    drop(resumed(&data));
    let victim = data.0.join("victime");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("state.json"), br#"{"version":1,"mode":"deck"}"#).unwrap();
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 2, "next_window": 2, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1", "../../victime", "t-1"], "active": "t-1"},
                           {"label": "../x", "tabs": ["t-1"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs).has_length(1);
    send(&ws, json!({"cmd": "close_window", "window": "w-1", "discard": ["t-1"]}));
    assert_that!(victim.join("state.json").exists()).is_true();
}

/// Des compteurs trop bas ne donnent jamais un identifiant déjà pris.
#[test]
fn resume_with_low_counters_never_reuses_an_id() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 1, "next_window": 1, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1", "t-2"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    let v = view(&ws);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(v.windows[1].tabs[0].id.as_str()).is_equal_to("t-3");
}

/// Des compteurs trop hauts (fichier altéré) sont ramenés au plus grand
/// identifiant relu : les identifiants restent valides.
#[test]
fn resume_with_huge_counters_resets_them() {
    let data = Data::new();
    drop(resumed(&data));
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 18446744073709551615u64, "next_window": 18446744073709551615u64,
               "focused": "w-1", "windows": [{"label": "w-1", "tabs": ["t-1"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    let v = view(&ws);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(v.windows[1].tabs[0].id.as_str()).is_equal_to("t-2");
}

/// Un onglet cité mais illisible (ici : sans `state.json`) garde son dossier,
/// ce lancement-ci et les suivants : il peut contenir du travail.
#[test]
fn resume_with_cited_unrestorable_tab_keeps_its_folder_across_launches() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::remove_file(data.resume().join("tabs/t-2/state.json")).unwrap();
    for _ in 0..2 {
        let ws = resumed(&data);
        assert_that!(view(&ws).windows[0].tabs).has_length(1);
        drop(ws);
        assert_that!(data.resume().join("tabs/t-2/deck/document.json").exists()).is_true();
    }
}

/// Les onglets d'une fenêtre refusée (label invalide ou en double) restent
/// cités : leurs dossiers ne sont pas supprimés.
#[test]
fn resume_with_rejected_window_keeps_the_folders_of_its_tabs() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_window"}));
    drop(a);
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 3, "next_window": 3, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1"], "active": "t-1"},
                           {"label": "w-1", "tabs": ["t-2"], "active": "t-2"}]}),
    );
    drop(resumed(&data));
    drop(resumed(&data));
    assert_that!(data.resume().join("tabs/t-2/state.json").exists()).is_true();
}

#[test]
fn close_tab_and_close_window_remove_their_folders() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&ws, json!({"cmd": "new_window"}));
    send(&ws, json!({"cmd": "close", "tab": "t-2"}));
    send(&ws, json!({"cmd": "close_window", "window": "w-2"}));
    assert_that!(names(&data.resume().join("tabs"))).is_equal_to(vec!["t-1".to_owned()]);
    drop(ws);
    assert_that!(view(&resumed(&data)).windows[0].tabs).has_length(1);
}

#[test]
fn open_files_same_file_twice_opens_one_tab_in_the_focused_window() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    tab(&ws, "t-2", set_theme("pro"));
    let file = saved_pdf(&data, "essai");
    Workspace::open_files(&ws, &[file.clone(), file]);
    let v = view(&ws);
    assert_that!(v.windows[0].tabs).has_length(1);
    assert_that!(v.windows[1].tabs).has_length(2);
    assert_that!(v.windows[1].active.as_str()).is_equal_to("t-3");
}

#[test]
fn open_files_unreadable_file_sets_a_notice_once_and_opens_the_next() {
    let data = Data::new();
    let ws = resumed(&data);
    tab(&ws, "t-1", set_theme("pro"));
    Workspace::open_files(&ws, &[data.0.join("absent.oku-slides.pdf"), fixture()]);
    let w = &view(&ws).windows[0];
    assert_that!(w.tabs).has_length(2);
    let first = tab(&ws, "t-1", json!({"cmd": "snapshot"}));
    assert_that!(first["notice"].as_str()).is_some();
    let second = tab(&ws, "t-1", json!({"cmd": "snapshot"}));
    assert_that!(second["notice"].as_str()).is_none();
}
```

- [ ] **Step 3 :** `cargo nextest run -p oku-slides-session --test workspace` → échec de compilation (`Workspace::resume`, `open_files` absents).

- [ ] **Step 4 : helpers JSON de `Resume`.** Dans `resume.rs`, ajouter `use serde::de::DeserializeOwned;`, puis remplacer `write_state` / `read_state` par :

```rust
    /// Écrit `name` (JSON) dans le dossier, d'un coup.
    pub fn write_json(&self, name: &str, value: &impl Serialize) -> Result<()> {
        let path = self.dir.join(name);
        let bytes = serde_json::to_vec(value).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)
    }

    /// Relit `name` s'il existe ; une autre version que `version` est une erreur.
    pub fn read_json<T: DeserializeOwned>(&self, name: &str, version: u32) -> Result<Option<T>> {
        let path = self.dir.join(name);
        if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
            return Ok(None);
        }
        let bytes = self.get(&path)?;
        // La version se lit à part : une version future peut avoir changé de
        // forme, illisible en `T` alors que sa version l'est bien.
        let versioned: Versioned = serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?;
        ensure!(versioned.version == version, VersionSnafu { version: versioned.version });
        Ok(Some(serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?))
    }

    pub fn write_state(&self, state: &State) -> Result<()> {
        self.write_json("state.json", state)
    }

    /// L'état du dernier lancement ; `None` au premier.
    pub fn read_state(&self) -> Result<Option<State>> {
        self.read_json("state.json", VERSION)
    }
```

  `cargo nextest run -p oku-slides-session resume` → les tests `resume_tests` restent verts.

- [ ] **Step 5 : reprise dans `workspace.rs`.** Ajouter aux imports `use std::collections::HashSet;`, `use crate::Localize;`, `use crate::resume::{Resume, ResumeError};`. Retirer les `#[expect(dead_code, …)]` de `MAX_NUMBER`, `number` et `is_window_label` (tâche 2). `Layout` gagne un champ, et `Layout::new` le met à `Vec::new()` :

```rust
    /// Onglets cités mais non restaurés, dont le dossier existe encore : ils
    /// restent cités, leur dossier n'est jamais supprimé.
    #[serde(default)]
    kept: Vec<String>,
```

  Remplacer `save_layout` et ajouter dans `impl Workspace` :

```rust
    /// Reprend l'espace de travail laissé dans `dir` (`<data>/resume`).
    ///
    /// Ses fenêtres, ses onglets et leurs sessions reviennent ; ensuite tout
    /// est écrit à chaque changement. `workspace.json` illisible : tout le
    /// dossier est mis de côté, jamais écrasé. Un onglet illisible est mis de
    /// côté seul et retiré ; les autres restent.
    pub fn resume(fonts: Fonts, platform: &Platform, dir: PathBuf) -> Result<Self> {
        let mut ws = Self::empty(fonts, platform, Some(dir.clone()));
        let cited = match ws.restore(&dir) {
            Ok(cited) => cited,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise de l'espace de travail impossible");
                ws.tabs.clear();
                ws.layout = Layout::new();
                ws.recent.clear();
                match Resume::new(platform.storage.clone(), dir).quarantine() {
                    Ok(moved) => tracing::warn!(path = %moved.display(), "reprise en échec mise de côté"),
                    Err(e) => {
                        tracing::warn!(
                            error = %oku_slides_platform::errors::one_line(&e),
                            "reprise en échec non déplacée : rien n'est écrit"
                        );
                        ws.dir = None;
                    }
                }
                HashSet::new()
            }
        };
        // Un orphelin qu'on n'a pas pu supprimer garde son numéro : jamais réutilisé.
        let listed = ws.remove_orphans(&cited);
        ws.layout.next_tab = ws.layout.next_tab.max(listed.saturating_add(1)).min(MAX_NUMBER);
        if ws.layout.windows.is_empty() {
            let pending = ws.pending(None, None, None);
            let session = pending.build()?;
            ws.insert_tab(None, pending.id, session, None);
        }
        ws.save_layout();
        Ok(ws)
    }

    /// Relit `workspace.json` et chaque onglet ; renvoie les identifiants
    /// cités (restaurés ou non), dont les dossiers ne sont jamais supprimés.
    ///
    /// Le fichier n'est pas digne de confiance : identifiants et labels
    /// validés, doublons refusés, compteurs ramenés entre le plus grand
    /// identifiant relu + 1 et `MAX_NUMBER`.
    fn restore(&mut self, dir: &Path) -> std::result::Result<HashSet<String>, ResumeError> {
        let resume = Resume::new(self.platform.storage.clone(), dir.to_owned());
        let Some(mut saved) = resume.read_json::<Layout>("workspace.json", LAYOUT_VERSION)? else {
            return Ok(HashSet::new());
        };
        let windows = std::mem::take(&mut saved.windows);
        let mut cited = HashSet::new();
        let mut restored = HashSet::new();
        let mut labels = HashSet::new();
        let mut max_window = 0;
        for w in windows {
            // Une fenêtre refusée cite quand même ses onglets : leurs dossiers restent.
            let accepted = is_window_label(&w.label) && labels.insert(w.label.clone());
            if accepted {
                max_window = max_window.max(number(&w.label));
            } else {
                tracing::warn!(label = %w.label, "fenêtre de reprise refusée");
            }
            let mut tabs = Vec::new();
            for id in w.tabs {
                if !is_tab_id(&id) || !cited.insert(id.clone()) {
                    tracing::warn!(tab = %id, "onglet de reprise refusé");
                    continue;
                }
                if accepted && self.restore_tab(dir, &id) {
                    restored.insert(id.clone());
                    tabs.push(id);
                }
            }
            if tabs.is_empty() {
                continue;
            }
            let active = if tabs.contains(&w.active) { w.active } else { tabs[0].clone() };
            self.recent.push(w.label.clone());
            self.layout.windows.push(WindowLayout { label: w.label, tabs, active });
        }
        // Les onglets gardés la dernière fois restent cités (après les
        // fenêtres : un identifiant des deux côtés reste restauré).
        cited.extend(saved.kept.iter().filter(|id| is_tab_id(id)).cloned());
        // Cités, non restaurés, dossier encore là : gardés d'un lancement à l'autre.
        let storage = &self.platform.storage;
        let mut kept: Vec<String> = cited
            .iter()
            .filter(|id| !restored.contains(*id))
            .filter(|id| storage.exists(&dir.join(TABS_DIR).join(id.as_str())).unwrap_or(true))
            .cloned()
            .collect();
        kept.sort();
        self.layout.kept = kept;
        let max_tab = cited.iter().map(|id| number(id)).max().unwrap_or(0);
        // Jamais plus de 18 chiffres, même depuis un identifiant maximal relu.
        let counter = |saved: u64, max: u64| {
            let next = if saved > MAX_NUMBER { max + 1 } else { saved.max(max + 1) };
            next.min(MAX_NUMBER)
        };
        self.layout.next_tab = counter(saved.next_tab, max_tab);
        self.layout.next_window = counter(saved.next_window, max_window);
        if self.recent.contains(&saved.focused) {
            self.touch(&saved.focused);
        } else {
            self.layout.focused = self.recent.last().cloned().unwrap_or_default();
        }
        Ok(cited)
    }

    /// Relit la session d'un onglet ; `false` : onglet retiré (sans dossier,
    /// ou illisible). Son dossier, s'il existe, reste en place.
    fn restore_tab(&mut self, dir: &Path, id: &str) -> bool {
        let tab_dir = dir.join(TABS_DIR).join(id);
        match self.platform.storage.exists(&tab_dir.join("state.json")) {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(tab = id, "onglet sans dossier de reprise : retiré");
                return false;
            }
            Err(e) => {
                tracing::warn!(tab = id, error = %oku_slides_platform::errors::one_line(&e), "onglet illisible : retiré, gardé sur le disque");
                return false;
            }
        }
        let mut session = match Session::new(self.fonts.clone(), &self.platform) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(tab = id, error = %e.one_line(), "session de l'onglet");
                return false;
            }
        };
        if !session.resume(tab_dir) {
            return false;
        }
        self.tabs.insert(id.to_owned(), Tab::new(session));
        true
    }

    /// Supprime les dossiers `t-<n>` que `workspace.json` ne citait pas.
    ///
    /// Ce sont les restes d'une fermeture interrompue. Les dossiers cités,
    /// mis de côté ou étrangers restent. Renvoie le plus grand numéro de
    /// dossier `t-<n>` vu (0 sans dossier).
    fn remove_orphans(&self, cited: &HashSet<String>) -> u64 {
        let Some(dir) = &self.dir else { return 0 };
        let storage = &self.platform.storage;
        let listed = match storage.list_dirs(&dir.join(TABS_DIR)) {
            Ok(listed) => listed,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "dossiers d'onglets");
                return 0;
            }
        };
        let mut highest = 0;
        for path in listed {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !is_tab_id(&name) {
                continue;
            }
            highest = highest.max(number(&name));
            if !cited.contains(&name)
                && !self.tabs.contains_key(&name)
                && let Err(e) = storage.delete_dir(&path)
            {
                tracing::warn!(path = %path.display(), error = %oku_slides_platform::errors::one_line(&e), "orphelin");
            }
        }
        highest
    }

    /// Ouvre des fichiers reçus dans la fenêtre active.
    ///
    /// Sources : argument, seconde instance, macOS. Un fichier illisible le
    /// dit dans l'onglet actif (avis montré une fois).
    pub fn open_files(ws: &Mutex<Self>, files: &[PathBuf]) {
        for path in files {
            let window = Self::lock(ws).focused().to_owned();
            let open = WorkspaceRequest::OpenTab { window: window.clone(), path: path.clone() };
            let Err(e) = Self::apply(ws, open) else { continue };
            tracing::warn!(path = %path.display(), error = %e.one_line(), "ouverture d'un fichier reçu");
            let (text, session) = {
                let w = Self::lock(ws);
                let l = &w.platform.localizer;
                let text = l.tr_with(
                    "notice-open-failed",
                    &[("path", path.display().to_string().into()), ("reason", e.localize(l).into())],
                );
                let active = w.layout.windows.iter().find(|x| x.label == window).map(|x| x.active.clone());
                (text, active.and_then(|id| w.session(&id).ok()))
            };
            if let Some(session) = session {
                relock(&session).set_notice(text);
            }
        }
    }

    /// Écrit `workspace.json` ; un échec est journalisé.
    fn save_layout(&self) {
        let Some(dir) = &self.dir else { return };
        let resume = Resume::new(self.platform.storage.clone(), dir.clone());
        if let Err(e) = resume.write_json("workspace.json", &self.layout) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "espace de travail non enregistré");
        }
    }
```

  `Resume` et `ResumeError` sont `pub(crate)` dans `resume.rs`.

- [ ] **Step 6 : clé `notice-open-failed`**, après `notice-modified-elsewhere` dans chaque `session.ftl` :

| locale | `notice-open-failed` |
|---|---|
| en | { $path } could not be opened: { $reason } |
| fr | { $path } n'a pas pu être ouvert : { $reason } |
| de | { $path } konnte nicht geöffnet werden: { $reason } |
| es | No se pudo abrir { $path }: { $reason } |
| it | Impossibile aprire { $path }: { $reason } |
| pt | Não foi possível abrir { $path }: { $reason } |
| nl | { $path } kon niet worden geopend: { $reason } |
| pl | Nie udało się otworzyć { $path }: { $reason } |
| ru | Не удалось открыть { $path }: { $reason } |
| uk | Не вдалося відкрити { $path }: { $reason } |
| tr | { $path } açılamadı: { $reason } |
| ar | تعذّر فتح { $path }: { $reason } |
| hi | { $path } नहीं खुल सका: { $reason } |
| bn | { $path } খোলা যায়নি: { $reason } |
| ja | { $path } を開けませんでした: { $reason } |
| ko | { $path }을(를) 열 수 없습니다: { $reason } |
| zh-Hans | 无法打开 { $path }：{ $reason } |
| zh-Hant | 無法開啟 { $path }：{ $reason } |
| th | ไม่สามารถเปิด { $path } ได้: { $reason } |
| vi | Không thể mở { $path }: { $reason } |

- [ ] **Step 7 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n -p oku-slides-platform` et `cargo clippy -p oku-slides-session --all-targets -- -D warnings` → vert.

- [ ] **Step 8 : commit**

```bash
git add crates
git commit -m "feat(session) : reprise des fenêtres et onglets, fichiers au lancement (t-149)"
oku cli task done --project slider --id t-149
```

---

### Task 4 : Serveur de développement et interface par onglet (t-150)

Serveur de dev et UI changent ensemble : chaque commit garde `just e2e` vert.

**Files:**
- Modify: `crates/oku-slides-cli/src/serve.rs`, `crates/oku-slides-cli/tests/cli.rs`, `crates/oku-slides-cli/Cargo.toml` (`serde.workspace = true` : `TabRequest` dérive `Deserialize`)
- Modify: `crates/oku-slides-session/src/lib.rs` (`Request` sans `New`/`Open`/`Messages`), `tests/errors.rs`, `tests/session.rs`
- Modify: `ui/src/api.ts`, `ui/src/i18n.ts`, `ui/src/App.tsx`, `ui/src/TemplatePicker.tsx`, `ui/src/main.tsx`, `ui/e2e/smoke.cjs`
- Create: `ui/src/Workspace.tsx`, `ui/src/tabs.ts`, `ui/src/tabs.test.ts`

**Interfaces:**
- Consumes: `Workspace::{new(fonts, platform, file), apply, request}`, `WorkspaceRequest` (tâches 2-3).
- Produces:
  - HTTP : `POST /api` `{ "tab": "t-1", "req": <Request> }` ; `POST /api/workspace` `<WorkspaceRequest>` ; 403 si `Host` ou `Origin` n'est pas un nom de bouclage (port quelconque).
  - `ui/src/api.ts` : `tabApi(tab)` → `{ snapshot, chartKind, save, slide, svg, panel, apply, undo, redo, templates, templateSets, editTemplates, saveTemplates, closeTemplates, deleteTemplateSet, ui }` ; `type TabApi` ; `workspace = { view, messages, newTab, openTab, activate, close, reorder, moveTab, newWindow, closeWindow }` ; types `TabInfo`, `WindowView`, `WorkspaceView` ; `errorText(e: unknown): string` ; `isMac: boolean`.
  - `ui/src/tabs.ts` : `slotAt`, `finalIndex`, `cycle`, `nth`, `tabShortcut`, `outside`.
  - `App({ tab, onOpen, onChange? })` ; `TemplatePicker` reçoit `api: TabApi`.

- [ ] **Step 1 : test du serveur qui échoue.** Dans `tests/cli.rs`, ajouter un helper à côté de `http` :

```rust
/// Un `POST` avec l'en-tête `Host` donné et des en-têtes en plus ; renvoie le statut.
fn http_host(port: u16, path: &str, body: &str, host: &str, headers: &str) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("serveur injoignable : {e}"));
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {host}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap_or_else(|e| panic!("requête : {e}"));
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap_or_else(|e| panic!("réponse : {e}"));
    response.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// Comme `http`, avec des en-têtes en plus (ex. `Origin`) ; renvoie le statut.
fn http_with(port: u16, path: &str, body: &str, headers: &str) -> u16 {
    http_host(port, path, body, "localhost", headers)
}

/// Comme `http`, avec un autre en-tête `Host` ; renvoie le statut.
fn http_raw(port: u16, path: &str, body: &str, host: &str) -> u16 {
    http_host(port, path, body, host, "")
}
```

  Dans `serve_answers_the_api_the_dictation_and_static_files`, remplacer les corps `/api` : `r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#` et `r#"{"tab":"t-1","req":{"cmd":"slide","id":"absente"}}"#` (assertions inchangées ; `"pas du json"` inchangé). Ajouter ensuite :

```rust
    // Un deuxième onglet : vierge, distinct du premier.
    let (status, view) = http(port, "POST", "/api/workspace", r#"{"cmd":"new_tab","window":"w-1"}"#);
    assert_that!(status).is_equal_to(200);
    assert_that!(view.as_str()).contains(r#""id":"t-2""#);
    let (status, blank) = http(port, "POST", "/api", r#"{"tab":"t-2","req":{"cmd":"snapshot"}}"#);
    assert_that!(status).is_equal_to(200);
    let blank: serde_json::Value = serde_json::from_str(&blank).unwrap();
    assert_that!(blank["slides"].as_array().map(Vec::len)).is_equal_to(Some(2));
    let (status, error) = http(port, "POST", "/api", r#"{"tab":"t-9","req":{"cmd":"snapshot"}}"#);
    assert_that!(status).is_equal_to(400);
    assert_that!(error).is_equal_to(r#"{"error":"Cet onglet n'est plus ouvert."}"#.to_owned());
    // Une page web d'une autre origine ne pilote pas le serveur.
    let evil = http_with(port, "/api/workspace", r#"{"cmd":"new_window"}"#, "Origin: http://evil.example\r\n");
    assert_that!(evil).is_equal_to(403);
    // DNS rebinding : un nom étranger qui pointe sur 127.0.0.1.
    let rebound = http_raw(port, "/api/workspace", r#"{"cmd":"view"}"#, "evil.example");
    assert_that!(rebound).is_equal_to(403);
    let same = http_with(port, "/api/workspace", r#"{"cmd":"view"}"#, &format!("Origin: http://localhost:{port}\r\n"));
    assert_that!(same).is_equal_to(200);
    // Le serveur Vite de `just web` (port 5173) relaie l'origine de sa page.
    let vite = http_with(port, "/api/workspace", r#"{"cmd":"view"}"#, "Origin: http://localhost:5173\r\n");
    assert_that!(vite).is_equal_to(200);
```

  Dans `serve_stops_cleanly_on_sigterm` : `r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#`.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-cli serve` → échec (400 sur `/api`, 404 sur `/api/workspace`).

- [ ] **Step 3 : `serve.rs`.**

  a. Remplacer la création de la session par :

```rust
    // Le fichier donné s'ouvre dans le premier onglet, `t-1`.
    let workspace = std::sync::Mutex::new(oku_slides_session::Workspace::new(fonts, platform, input.as_deref())?);
```

  (`SessionError` implémente `std::error::Error` : `?` vers `color_eyre` garde la chaîne des causes.)

  b. Dans `crates/oku-slides-cli/Cargo.toml`, `[dependencies]` : `serde.workspace = true`. Ajouter en tête de `serve.rs` :

```rust
/// Une requête d'onglet : le corps de `POST /api`.
#[derive(serde::Deserialize)]
struct TabRequest {
    tab: String,
    req: oku_slides_session::Request,
}

/// Hôte de bouclage, port facultatif : `localhost`, `127.0.0.1`, `[::1]`.
fn loopback(host: &str) -> bool {
    let bare = match host.rsplit_once(':') {
        Some((name, port)) if !name.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    matches!(bare, "localhost" | "127.0.0.1" | "[::1]")
}

/// La requête vient d'une page locale (ce serveur, ou Vite sur un autre port).
///
/// Une page web qui viserait `localhost` est refusée : son `Origin` n'est
/// pas locale (CSRF), ou son `Host` n'est pas un nom de bouclage (DNS
/// rebinding). Serveur de développement seulement.
fn same_origin(req: &tiny_http::Request) -> bool {
    let header = |name: &'static str| {
        req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str().to_owned())
    };
    header("Host").is_some_and(|h| loopback(&h))
        && header("Origin").is_none_or(|o| o.strip_prefix("http://").is_some_and(loopback))
}
```

  c. Dans la boucle : la condition devient `url == "/api" || url == "/api/workspace" || url == "/api/stt"`. Avant de lire le corps :

```rust
            if !same_origin(&req) {
                tracing::warn!(url, "requête d'une autre origine refusée");
                if let Err(e) = req.respond(Response::from_string("").with_status_code(403)) {
                    tracing::debug!(error = %e, "réponse non envoyée");
                }
                continue;
            }
```

  Le calcul de `(status, reply)` devient :

```rust
            let refused = |e: oku_slides_session::SessionError| {
                tracing::warn!(error = %e.one_line(), "requête refusée");
                (400, serde_json::json!({ "error": e.localize(l) }).to_string())
            };
            let (status, reply) = match url.as_str() {
                "/api" => match serde_json::from_str::<TabRequest>(&body) {
                    Ok(TabRequest { tab, req }) => {
                        oku_slides_session::Workspace::request(&workspace, &tab, req).0.map_or_else(refused, |json| (200, json))
                    }
                    Err(e) => (400, bad_request(e)),
                },
                "/api/workspace" => match serde_json::from_str::<oku_slides_session::WorkspaceRequest>(&body) {
                    Ok(r) => oku_slides_session::Workspace::apply(&workspace, r).map_or_else(refused, |json| (200, json)),
                    Err(e) => (400, bad_request(e)),
                },
                _ => match serde_json::from_str::<oku_slides_stt::SttRequest>(&body) {
                    Ok(r) => (200, serde_json::to_string(&stt.handle(r)).unwrap_or_default()),
                    Err(e) => (400, bad_request(e)),
                },
            };
```

  d. Doc de `serve`, première phrase : « Sert l'interface et l'API, pour le développement seulement. » Deuxième paragraphe : « Onglets : `POST /api` (`{ tab, req }`) ; espace de travail : `POST /api/workspace` ; dictée : `POST /api/stt`. Une requête d'une page non locale est refusée (403). »

- [ ] **Step 4 : `Request` allégé.** Dans `lib.rs`, supprimer les variantes `New`, `Open { path }` et `Messages` et leurs bras dans `handle` ; la garde du mode modèles devient `matches!(req, Request::Save { .. })`. Tests :
  - `tests/errors.rs` : supprimer `messages_give_the_ui_its_language_and_fluent_resources` (couvert par `workspace_tests`). Renommer `open_while_editing_templates_is_refused` en `save_while_editing_templates_is_refused` et y envoyer `json!({"cmd": "save", "path": null})` au lieu de `{"cmd": "new"}` (même refus `EditingTemplates`).
  - `tests/session.rs` (vers la ligne 390) : `s.handle(req(json!({"cmd": "new"}))).unwrap();` → `s.new_deck().unwrap();`.

- [ ] **Step 5 :** `cargo nextest run -p oku-slides-cli -p oku-slides-session` → vert.

- [ ] **Step 6 : tests UI qui échouent.** Créer `ui/src/tabs.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { cycle, finalIndex, nth, outside, slotAt, tabShortcut } from "./tabs";

const key = (code: string, key: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; altKey: boolean; repeat: boolean }> = {}) => ({
  code,
  key,
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  repeat: false,
  ...mods,
});

describe("barre d'onglets", () => {
  test("slotAt compte les milieux à gauche du pointeur", () => {
    expect(slotAt([50, 150, 250], 10)).toBe(0);
    expect(slotAt([50, 150, 250], 150)).toBe(1);
    expect(slotAt([50, 150, 250], 400)).toBe(3);
  });

  test("finalIndex tient compte de l'onglet retiré avant sa place", () => {
    expect(finalIndex(0, 3)).toBe(2);
    expect(finalIndex(2, 0)).toBe(0);
    expect(finalIndex(1, 1)).toBe(1);
  });

  test("cycle boucle dans les deux sens et reste sur place si rien ne va", () => {
    expect(cycle(["a", "b", "c"], "c", 1)).toBe("a");
    expect(cycle(["a", "b", "c"], "a", -1)).toBe("c");
    expect(cycle(["a"], "a", 1)).toBe("a");
    expect(cycle(["a", "b"], "z", 1)).toBeUndefined();
    expect(cycle([], "a", 1)).toBeUndefined();
  });

  test("nth donne l'onglet n, et 9 le dernier", () => {
    expect(nth(["a", "b", "c"], 2)).toBe("b");
    expect(nth(["a", "b", "c"], 9)).toBe("c");
    expect(nth(["a", "b", "c"], 5)).toBeUndefined();
  });

  test("tabShortcut reconnaît ⌘T, ⌘W, ⌘⇧N et ⌘1 sur Mac", () => {
    expect(tabShortcut(key("KeyT", "t", { metaKey: true }), true)).toEqual({ kind: "new-tab" });
    expect(tabShortcut(key("KeyW", "w", { metaKey: true }), true)).toEqual({ kind: "close-tab" });
    expect(tabShortcut(key("KeyN", "N", { metaKey: true, shiftKey: true }), true)).toEqual({ kind: "new-window" });
    expect(tabShortcut(key("Digit1", "1", { metaKey: true }), true)).toEqual({ kind: "nth", n: 1 });
  });

  test("tabShortcut en AZERTY : chiffres par touche physique, lettres par caractère", () => {
    // ⌘1 : la touche « & / 1 » donne `key` « & ».
    expect(tabShortcut(key("Digit1", "&", { metaKey: true }), true)).toEqual({ kind: "nth", n: 1 });
    // ⌘Z (annuler) se tape sur la touche physique `KeyW` : jamais fermer l'onglet.
    expect(tabShortcut(key("KeyW", "z", { metaKey: true }), true)).toBeNull();
    // ⌘W se tape sur la touche physique `KeyZ`.
    expect(tabShortcut(key("KeyZ", "w", { metaKey: true }), true)).toEqual({ kind: "close-tab" });
  });

  test("tabShortcut ignore les répétitions, Alt, ⌘⇧T et ⌘⇧W", () => {
    expect(tabShortcut(key("KeyW", "w", { metaKey: true, repeat: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyT", "t", { metaKey: true, altKey: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyT", "T", { metaKey: true, shiftKey: true }), true)).toBeNull();
    expect(tabShortcut(key("KeyW", "W", { metaKey: true, shiftKey: true }), true)).toBeNull();
  });

  test("tabShortcut prend Ctrl hors de macOS et ignore ⌘ seul", () => {
    expect(tabShortcut(key("KeyT", "t", { ctrlKey: true }), false)).toEqual({ kind: "new-tab" });
    expect(tabShortcut(key("KeyT", "t", { metaKey: true }), false)).toBeNull();
    expect(tabShortcut(key("Digit2", "2", { metaKey: true }), false)).toBeNull();
  });

  test("tabShortcut : Ctrl+Tab et Ctrl+Maj+Tab changent d'onglet partout", () => {
    expect(tabShortcut(key("Tab", "Tab", { ctrlKey: true }), true)).toEqual({ kind: "cycle", delta: 1 });
    expect(tabShortcut(key("Tab", "Tab", { ctrlKey: true, shiftKey: true }), true)).toEqual({ kind: "cycle", delta: -1 });
  });

  test("outside : hors de la fenêtre dès le bord", () => {
    expect(outside(10, 10, 100, 100)).toBe(false);
    expect(outside(-1, 10, 100, 100)).toBe(true);
    expect(outside(10, 100, 100, 100)).toBe(true);
  });
});
```

  `cd ui && bun test tabs` → échec (module absent).

- [ ] **Step 7 : `ui/src/tabs.ts`.**

```ts
// Barre d'onglets : logique pure (places de dépôt, raccourcis, navigation).

/** Place de dépôt (0 = avant le premier onglet), d'après le milieu de chaque onglet. */
export function slotAt(mids: number[], x: number): number {
  return mids.filter((m) => m < x).length;
}

/** Index final d'un onglet parti de `from` et déposé à la place `slot`. */
export function finalIndex(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** Onglet voisin, en boucle ; `undefined` si `active` n'est pas dans la liste. */
export function cycle(tabs: string[], active: string, delta: number): string | undefined {
  const n = tabs.length;
  const i = tabs.indexOf(active);
  if (n === 0 || i < 0) return undefined;
  return tabs[(((i + delta) % n) + n) % n];
}

/** Onglet n (1 à 8) ; 9 : le dernier. */
export function nth(tabs: string[], n: number): string | undefined {
  return n === 9 ? tabs[tabs.length - 1] : tabs[n - 1];
}

type TabAction =
  | { kind: "new-tab" }
  | { kind: "close-tab" }
  | { kind: "new-window" }
  | { kind: "cycle"; delta: 1 | -1 }
  | { kind: "nth"; n: number };

type Keys = Pick<KeyboardEvent, "code" | "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "repeat">;

/**
 * Raccourci des onglets : ⌘ sur macOS, Ctrl ailleurs ; Ctrl+Tab partout.
 *
 * Lettres par caractère (`key`), comme les raccourcis d'`App` et les menus :
 * ⌘Z reste « annuler » en AZERTY. Chiffres par touche physique (`code`) :
 * ⌘1 reste ⌘1 en AZERTY, où `key` vaut « & ».
 */
export function tabShortcut(e: Keys, mac: boolean): TabAction | null {
  if (e.repeat || e.altKey) return null;
  if (e.ctrlKey && e.key === "Tab") return { kind: "cycle", delta: e.shiftKey ? -1 : 1 };
  if (!(mac ? e.metaKey : e.ctrlKey)) return null;
  const letter = e.key.toLowerCase();
  if (letter === "t" && !e.shiftKey) return { kind: "new-tab" };
  if (letter === "w" && !e.shiftKey) return { kind: "close-tab" };
  if (letter === "n" && e.shiftKey) return { kind: "new-window" };
  const digit = /^Digit([1-9])$/.exec(e.code);
  return digit && !e.shiftKey ? { kind: "nth", n: Number(digit[1]) } : null;
}

/** Le point (coordonnées de la page) est hors de la fenêtre. */
export function outside(x: number, y: number, width: number, height: number): boolean {
  return x < 0 || y < 0 || x >= width || y >= height;
}
```

  `bun test tabs` → vert.

- [ ] **Step 8 : `api.ts`.**

  a. En-tête du fichier : « Accès à l'hôte : requêtes par onglet, espace de travail (fenêtres et onglets), dictée et dialogues de fichier. `invoke` dans Tauri, `POST` au serveur de développement ailleurs. Les types décrivent le JSON de l'hôte. » (remplace l'en-tête actuel.)

  b. Remplacer `host` :

```ts
/** Requête à l'hôte : commande Tauri, ou `POST` en développement. */
async function host<T>(command: string, args: Record<string, unknown>, endpoint: string, body: unknown): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  // Un refus (403) ou une page d'erreur du proxy peuvent arriver sans corps
  // JSON : le statut reste lisible.
  const text = await res.text();
  let json: { error?: string } = {};
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  if (!res.ok) throw new Error(json.error ?? `${res.status} ${res.statusText}`);
  return json as unknown as T;
}

/** Le texte d'une erreur, sans le préfixe « Error: ». */
export function errorText(e: unknown): string {
  return String(e instanceof Error ? e.message : e);
}

/** Sur macOS : ⌘ et barre de titre intégrée. */
export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
```

  c. `sttCall` : `(req: SttRequest) => host<SttStatus>("stt", { req }, "/api/stt", req)`.

  d. Type `Request` : supprimer `{ cmd: "new" }`, `{ cmd: "open"; path: string }`, `{ cmd: "messages" }`.

  e. Remplacer `const call = …` et `export const api = { … }` par :

```ts
/** Les requêtes d'un onglet : chacune ne touche que sa session. */
export function tabApi(tab: string) {
  const call = <T>(req: Request) => host<T>("request", { tab, req }, "/api", { tab, req });
  return {
    snapshot: () => call<Snapshot>({ cmd: "snapshot" }),
    chartKind: (label: string, slide: string, element: string, kind: string) =>
      call<Snapshot>({ cmd: "chart_kind", label, slide, element, kind }),
    save: (path: string | null) => call<Snapshot>({ cmd: "save", path }),
    slide: (id: string) => call<SlideView>({ cmd: "slide", id }),
    svg: (id: string) => call<string>({ cmd: "svg", id }),
    /** Surface A2UI du panneau de droite pour une slide (vide hors des modèles). */
    panel: (slide: string) => call<A2uiMessage[]>({ cmd: "panel", slide }),
    apply: (label: string, ops: Op[]) => call<Snapshot>({ cmd: "apply", label, ops }),
    undo: () => call<Snapshot>({ cmd: "undo" }),
    redo: () => call<Snapshot>({ cmd: "redo" }),
    templates: (set?: string | null) => call<TemplatesView>({ cmd: "templates", set: set ?? null }),
    templateSets: () => call<SetInfo[]>({ cmd: "template_sets" }),
    editTemplates: (set: string) => call<Snapshot>({ cmd: "edit_templates", set }),
    saveTemplates: (asNew = false) => call<Snapshot>({ cmd: "save_templates", as_new: asNew }),
    closeTemplates: () => call<Snapshot>({ cmd: "close_templates" }),
    deleteTemplateSet: (set: string) => call<SetInfo[]>({ cmd: "delete_template_set", set }),
    ui: (slide: string | null, grid: boolean) => call<Snapshot>({ cmd: "ui", slide, grid }),
  };
}

/** Les requêtes d'un onglet (voir `tabApi`). */
export type TabApi = ReturnType<typeof tabApi>;

/** Un onglet, tel que la barre le montre. */
export interface TabInfo {
  id: string;
  title: string;
  dirty: boolean;
  editing_templates: boolean;
  path: string | null;
  pristine: boolean;
}

/** Une fenêtre : ses onglets et l'onglet actif. */
export interface WindowView {
  label: string;
  active: string;
  tabs: TabInfo[];
}

/** Les fenêtres et leurs onglets ; `focused` : la dernière fenêtre active. */
export interface WorkspaceView {
  focused: string;
  windows: WindowView[];
}

type WorkspaceRequest =
  | { cmd: "view" }
  | { cmd: "messages" }
  | { cmd: "new_tab"; window: string }
  | { cmd: "open_tab"; window: string; path: string }
  | { cmd: "activate"; tab: string }
  | { cmd: "close"; tab: string; discard: boolean }
  | { cmd: "reorder"; tab: string; index: number }
  | { cmd: "move_tab"; tab: string; window: string | null; index: number | null }
  | { cmd: "new_window" }
  | { cmd: "close_window"; window: string; discard: string[] };

const ws = <T>(req: WorkspaceRequest) => host<T>("workspace", { req }, "/api/workspace", req);

/** L'espace de travail : fenêtres et onglets. */
export const workspace = {
  view: () => ws<WorkspaceView>({ cmd: "view" }),
  messages: () => ws<Messages>({ cmd: "messages" }),
  newTab: (window: string) => ws<WorkspaceView>({ cmd: "new_tab", window }),
  openTab: (window: string, path: string) => ws<WorkspaceView>({ cmd: "open_tab", window, path }),
  activate: (tab: string) => ws<WorkspaceView>({ cmd: "activate", tab }),
  /** `discard` : l'utilisateur a choisi de perdre le travail non enregistré. */
  close: (tab: string, discard = false) => ws<WorkspaceView>({ cmd: "close", tab, discard }),
  reorder: (tab: string, index: number) => ws<WorkspaceView>({ cmd: "reorder", tab, index }),
  moveTab: (tab: string, window: string | null, index: number | null = null) =>
    ws<WorkspaceView>({ cmd: "move_tab", tab, window, index }),
  newWindow: () => ws<WorkspaceView>({ cmd: "new_window" }),
  closeWindow: (window: string, discard: string[]) => ws<WorkspaceView>({ cmd: "close_window", window, discard }),
};
```

- [ ] **Step 9 : `i18n.ts`** : `import { workspace } from "./api";` et `await workspace.messages()`.

- [ ] **Step 10 : `TemplatePicker.tsx`.** Remplacer ses deux imports de `./api` par `import { errorText, type SetInfo, type TabApi, type TemplateView } from "./api";` ; `Props` gagne `/** Requêtes de l'onglet du picker. */ api: TabApi;` ; la signature déstructure `api` ; l'effet de chargement dépend de `[themeKey, set, api]` (le thème reste une dépendance) ; son `.catch` utilise `errorText(e)`. Dans `App.tsx`, `<TemplatePicker … api={api} />`.

- [ ] **Step 11 : `App.tsx` par onglet.**
  - Imports : `import { errorText, pickOpenPath, pickSavePath, tabApi } from "./api";` et `import type { TabApi } from "./api";` (`inTauri` n'y sert plus). Dans `run`, `say(String(e instanceof Error ? e.message : e), true)` devient `say(errorText(e), true)`.
  - `useSvgCache()` devient `useSvgCache(api: TabApi)` ; son `useCallback` dépend de `[api]`. App est remonté à chaque changement d'onglet : les URL sont libérées à la destruction, et un SVG arrivé après n'en crée plus. Dans le hook :

```tsx
  const alive = useRef(true);
  useEffect(() => {
    // StrictMode monte, démonte puis remonte en développement : on repart
    // d'un cache vide et vivant à chaque montage.
    alive.current = true;
    const [urls, waiting] = [cache.current, pending.current];
    return () => {
      alive.current = false;
      urls.forEach((url) => URL.revokeObjectURL(url));
      urls.clear();
      waiting.clear();
    };
  }, []);
```

  et dans le `.then` de `api.svg(id)`, en premier : `pending.current.delete(hash); if (!alive.current) return;` (la ligne `pending.current.delete(hash);` plus bas disparaît).

  - Signature :

```tsx
/** Un onglet : le deck de la session `tab`. */
export default function App({ tab, onOpen, onChange }: {
  tab: string;
  /** Ouvrir un fichier : l'espace de travail choisit l'onglet. */
  onOpen: (path: string) => void;
  /** Un snapshot est arrivé (la barre d'onglets se relit, en développement). */
  onChange?: () => void;
}) {
  // Monté par onglet (`key`) : l'API ne change pas pendant la vie du composant.
  const [api] = useState(() => tabApi(tab));
  // Dernier `onChange` reçu, sans faire changer `run` d'identité.
  const changed = useRef(onChange);
  changed.current = onChange;
  const svg = useSvgCache(api);
```

  - Dans `run`, après `setSnap(s);` : `changed.current?.();` (dépendances du `useCallback` inchangées : `[]`).
  - `open` devient :

```tsx
  const open = async () => {
    const path = await pickOpenPath();
    if (path) onOpen(path);
  };
```

  - Supprimer `const titlebarOverlay = …`, la classe `overlay-titlebar` du `<div className="table…">` (il devient `<div className="table">`) et la ligne `{titlebarOverlay && <div className="drag-strip" data-tauri-drag-region />}` : la barre d'onglets porte la barre de titre (tâche 5). Dans `styles.css`, supprimer aussi les règles devenues mortes `.table.overlay-titlebar { … }`, `.table.overlay-titlebar .label { … }` et `.drag-strip { … }`.

- [ ] **Step 12 : coquille minimale** (complétée en tâche 5). `ui/src/Workspace.tsx` :

```tsx
// Une fenêtre : la barre d'onglets et l'`App` de l'onglet actif.

import { useCallback, useEffect, useState } from "react";
import App from "./App";
import { errorText, inTauri, workspace, type WorkspaceView } from "./api";
import { t } from "./i18n";

/** Label de cette fenêtre : celui de Tauri, ou la seule fenêtre en développement. */
async function ownLabel(view: WorkspaceView): Promise<string> {
  if (!inTauri) return view.windows[0]?.label ?? "";
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().label;
}

/** Une fenêtre : l'`App` de son onglet actif. */
export default function Workspace() {
  const [view, setView] = useState<WorkspaceView | null>(null);
  const [label, setLabel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    workspace.view().then(setView).catch((e) => setError(errorText(e)));
  }, []);

  useEffect(() => {
    workspace
      .view()
      .then(async (v) => {
        setView(v);
        setLabel(await ownLabel(v));
      })
      .catch((e) => setError(errorText(e)));
  }, []);

  const win = view?.windows.find((w) => w.label === label);
  if (!view || !label || !win) {
    return <div className="table"><p className="empty" style={{ padding: 40 }}>{error ?? t("ui-loading")}</p></div>;
  }

  // Un fichier ouvert change toujours d'onglet actif (nouvel onglet, ou
  // onglet où il l'était déjà) : `key` suffit à remonter l'`App`.
  const onOpen = (path: string) => workspace.openTab(label, path).then(setView).catch((e) => setError(errorText(e)));

  // Sans hôte qui prévient (développement), la vue se relit après chaque commande.
  return <App key={win.active} tab={win.active} onOpen={onOpen} onChange={inTauri ? undefined : refresh} />;
}
```

  `ui/src/main.tsx` : remplacer `import App from "./App";` par `import Workspace from "./Workspace";` et `<App />` par `<Workspace />`.

- [ ] **Step 13 : `ui/e2e/smoke.cjs`** : les deux `fetch('/api', …)` envoient `JSON.stringify({tab:'t-1', req:{cmd:'slide', id:'s-libre'}})` (le fichier du serveur est dans `t-1`) ; son adresse vient de `process.env.E2E_URL ?? 'http://127.0.0.1:4870/'`, comme les autres scripts.

- [ ] **Step 14 :** `cd ui && bun test && bun run typecheck`, puis `just e2e` → vert (un onglet, même comportement qu'avant).

- [ ] **Step 15 : commit**

```bash
git add crates/oku-slides-cli crates/oku-slides-session ui
git commit -m "feat(ui) : requêtes par onglet, serveur de développement sur l'espace de travail (t-150)"
oku cli task done --project slider --id t-150
```

---

### Task 5 : Interface — barre d'onglets, fermeture et raccourcis (t-151)

**Files:**
- Create: `ui/src/TabBar.tsx`, `ui/src/close.ts`, `ui/src/close.test.ts`, `ui/e2e/tabs.cjs`
- Modify: `ui/src/Workspace.tsx`, `ui/src/styles.css`, `ui/package.json`
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (12 clés)

**Interfaces:**
- Consumes: `workspace`, `tabApi`, `TabApi`, `errorText`, `isMac`, `pickSavePath`, types (tâche 4) ; `slotAt`, `finalIndex`, `cycle`, `nth`, `tabShortcut`, `outside` (tâche 4) ; en Tauri (tâche 6) : événements `workspace` et `close-requested`, commandes `drop_tab(tab, x, y)` et `quit`.
- Produces: `close.ts` : `choiceOf(result: string, labels: CloseLabels): CloseChoice`, `saveForClose(api: TabApi): Promise<boolean>`, `mayClose(tab: TabInfo, api: TabApi): Promise<"discard" | "keep" | null>` (`askUnsaved` reste privé) ; `TabBar` ; `Workspace` complet.

- [ ] **Step 1 : tests qui échouent.** `ui/src/close.test.ts` :

```ts
import { describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi } from "./api";
import { choiceOf, saveForClose } from "./close";

const labels = { save: "Enregistrer", discard: "Ne pas enregistrer", cancel: "Annuler" };

describe("fermeture d'un onglet modifié", () => {
  test("choiceOf lit le libellé cliqué et les réponses système", () => {
    expect(choiceOf("Enregistrer", labels)).toBe("save");
    expect(choiceOf("Ne pas enregistrer", labels)).toBe("discard");
    expect(choiceOf("Yes", labels)).toBe("save");
    expect(choiceOf("No", labels)).toBe("discard");
    expect(choiceOf("Annuler", labels)).toBe("cancel");
    expect(choiceOf("Cancel", labels)).toBe("cancel");
  });

  const snap = (s: Partial<Snapshot>) => ({ dirty: false, path: null, name: "essai", editing: null, ...s }) as Snapshot;

  test("saveForClose : deck propre, rien n'est enregistré", async () => {
    const save = mock(async () => snap({}));
    const api = { snapshot: async () => snap({}), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).not.toHaveBeenCalled();
  });

  test("saveForClose : deck avec fichier, enregistré sur place", async () => {
    const save = mock(async () => snap({ path: "/x.oku-slides.pdf" }));
    const api = { snapshot: async () => snap({ dirty: true, path: "/x.oku-slides.pdf" }), save } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(save).toHaveBeenCalledWith(null);
  });

  test("saveForClose : modèles modifiés enregistrés, puis le deck retrouvé", async () => {
    const calls: string[] = [];
    const api = {
      snapshot: async () => snap({ dirty: true, editing: { set: "s", name: "S", builtin: false } }),
      saveTemplates: async () => (calls.push("saveTemplates"), snap({ editing: { set: "s", name: "S", builtin: false } })),
      closeTemplates: async () => (calls.push("closeTemplates"), snap({})),
      save: async () => (calls.push("save"), snap({})),
    } as unknown as TabApi;
    expect(await saveForClose(api)).toBe(true);
    expect(calls).toEqual(["saveTemplates", "closeTemplates"]);
  });
});
```

  Et `ui/e2e/tabs.cjs` :

```js
// Onglets : nouvel onglet, bascule, fermeture, question pour un onglet modifié,
// contre `oku-slides serve fixtures/roadmap.json`.
// Passe après toolbar.cjs et panel.cjs sur le même serveur : le premier onglet
// (le fichier) a déjà été modifié ; ce script ne ferme que ses propres onglets.
// Il laisse un onglet modifié ouvert : relancé à la main, il lui faut un
// serveur neuf. Lancer : just e2e. À la main : bun e2e/tabs.cjs, E2E_URL =
// adresse d'un serveur tout juste démarré.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  const tabs = p.getByRole('tab');
  const count = (n) => p.waitForFunction((n) => document.querySelectorAll('[role="tab"]').length === n, n);
  const selected = (i) =>
    p.waitForFunction((i) => document.querySelectorAll('[role="tab"]')[i]?.getAttribute('aria-selected') === 'true', i);
  const thumbs = (n) => p.waitForFunction((n) => document.querySelectorAll('.thumb').length === n, n);
  await count(1);
  const slides = await p.locator('.thumb').count();
  // Nouvel onglet A : un deck vierge (2 slides), actif.
  await p.getByRole('button', { name: 'Nouvel onglet' }).click();
  await count(2);
  await selected(1);
  await thumbs(2);
  // Retour au fichier : son deck, intact.
  await tabs.nth(0).click();
  await selected(0);
  await thumbs(slides);
  // Nouvel onglet B, modifié : sa fermeture pose la question ; refuser le garde.
  await p.getByRole('button', { name: 'Nouvel onglet' }).click();
  await count(3);
  await selected(2);
  await thumbs(2);
  await p.getByRole('button', { name: 'Supprimer la slide' }).click();
  await tabs.nth(2).locator('.tab-dirty').waitFor();
  const dialog = p.waitForEvent('dialog');
  await tabs.nth(2).locator('.tab-close').click();
  await (await dialog).dismiss();
  // Refus : aucune requête de fermeture ; la vue relue montre toujours B.
  await p.evaluate(() => fetch('/api/workspace', { method: 'POST', body: JSON.stringify({ cmd: 'view' }) }));
  await tabs.nth(2).locator('.tab-dirty').waitFor();
  assert.equal(await tabs.count(), 3);
  assert.equal(await tabs.nth(2).getAttribute('aria-selected'), 'true');
  // A, non modifié, se ferme sans question.
  await tabs.nth(1).locator('.tab-close').click();
  await count(2);
  assert.deepEqual(errors, []);
  await b.close();
  console.log('tabs: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

  `ui/package.json` : `"e2e": "bun e2e/toolbar.cjs && bun e2e/panel.cjs && bun e2e/tabs.cjs"` ; et, parce que `close.ts` utilise les boutons personnalisés de `message` : `"@tauri-apps/api": "^2.11.0"`, `"@tauri-apps/plugin-dialog": "^2.7.0"`, puis `cd ui && bun install`.

- [ ] **Step 2 :** `cd ui && bun test close` → échec (module absent) ; `just e2e` → `tabs.cjs` échoue (pas de rôle `tab`).

- [ ] **Step 3 : clés Fluent** dans chaque `ui.ftl`, en fin de fichier. Le script écrit tout (commentaire de section traduit compris) :

```bash
python3 - <<'EOF'
keys = ["ui-tabs","ui-tab-new","ui-tab-close","ui-tab-unsaved","ui-tab-templates","ui-tab-move-new-window",
        "ui-tab-move-to","ui-tab-close-others","ui-close-unsaved","ui-close-save","ui-close-discard","ui-close-cancel"]
rows = """
en | Tabs | Open presentations | New tab | Close tab | Unsaved changes | Templates | Move to new window | Move to window { $n } | Close other tabs | Save the changes to “{ $name }” before closing? | Save | Don't save | Cancel
fr | Onglets | Présentations ouvertes | Nouvel onglet | Fermer l'onglet | Modifications non enregistrées | Modèles | Déplacer vers une nouvelle fenêtre | Déplacer vers la fenêtre { $n } | Fermer les autres onglets | Enregistrer les modifications de « { $name } » avant de fermer ? | Enregistrer | Ne pas enregistrer | Annuler
de | Tabs | Geöffnete Präsentationen | Neuer Tab | Tab schließen | Nicht gespeicherte Änderungen | Vorlagen | In neues Fenster verschieben | In Fenster { $n } verschieben | Andere Tabs schließen | Änderungen an „{ $name }“ vor dem Schließen speichern? | Speichern | Nicht speichern | Abbrechen
es | Pestañas | Presentaciones abiertas | Nueva pestaña | Cerrar pestaña | Cambios sin guardar | Plantillas | Mover a una ventana nueva | Mover a la ventana { $n } | Cerrar las demás pestañas | ¿Guardar los cambios de «{ $name }» antes de cerrar? | Guardar | No guardar | Cancelar
it | Schede | Presentazioni aperte | Nuova scheda | Chiudi scheda | Modifiche non salvate | Modelli | Sposta in una nuova finestra | Sposta nella finestra { $n } | Chiudi le altre schede | Salvare le modifiche a «{ $name }» prima di chiudere? | Salva | Non salvare | Annulla
pt | Abas | Apresentações abertas | Nova aba | Fechar aba | Alterações não salvas | Modelos | Mover para uma nova janela | Mover para a janela { $n } | Fechar as outras abas | Salvar as alterações em “{ $name }” antes de fechar? | Salvar | Não salvar | Cancelar
nl | Tabbladen | Geopende presentaties | Nieuw tabblad | Tabblad sluiten | Niet-opgeslagen wijzigingen | Sjablonen | Naar nieuw venster verplaatsen | Naar venster { $n } verplaatsen | Andere tabbladen sluiten | Wijzigingen in ‘{ $name }’ opslaan voor het sluiten? | Opslaan | Niet opslaan | Annuleren
pl | Karty | Otwarte prezentacje | Nowa karta | Zamknij kartę | Niezapisane zmiany | Szablony | Przenieś do nowego okna | Przenieś do okna { $n } | Zamknij pozostałe karty | Zapisać zmiany w „{ $name }” przed zamknięciem? | Zapisz | Nie zapisuj | Anuluj
ru | Вкладки | Открытые презентации | Новая вкладка | Закрыть вкладку | Несохранённые изменения | Шаблоны | Переместить в новое окно | Переместить в окно { $n } | Закрыть другие вкладки | Сохранить изменения в «{ $name }» перед закрытием? | Сохранить | Не сохранять | Отмена
uk | Вкладки | Відкриті презентації | Нова вкладка | Закрити вкладку | Незбережені зміни | Шаблони | Перемістити в нове вікно | Перемістити у вікно { $n } | Закрити інші вкладки | Зберегти зміни в «{ $name }» перед закриттям? | Зберегти | Не зберігати | Скасувати
tr | Sekmeler | Açık sunumlar | Yeni sekme | Sekmeyi kapat | Kaydedilmemiş değişiklikler | Şablonlar | Yeni pencereye taşı | { $n }. pencereye taşı | Diğer sekmeleri kapat | Kapatmadan önce “{ $name }” değişiklikleri kaydedilsin mi? | Kaydet | Kaydetme | İptal
ar | علامات التبويب | العروض المفتوحة | علامة تبويب جديدة | إغلاق علامة التبويب | تغييرات غير محفوظة | القوالب | نقل إلى نافذة جديدة | نقل إلى النافذة { $n } | إغلاق علامات التبويب الأخرى | هل تريد حفظ التغييرات على «{ $name }» قبل الإغلاق؟ | حفظ | عدم الحفظ | إلغاء
hi | टैब | खुली प्रस्तुतियाँ | नया टैब | टैब बंद करें | बिना सहेजे बदलाव | टेम्पलेट | नई विंडो में ले जाएँ | विंडो { $n } में ले जाएँ | अन्य टैब बंद करें | बंद करने से पहले “{ $name }” के बदलाव सहेजें? | सहेजें | न सहेजें | रद्द करें
bn | ট্যাব | খোলা উপস্থাপনা | নতুন ট্যাব | ট্যাব বন্ধ করুন | অসংরক্ষিত পরিবর্তন | টেমপ্লেট | নতুন উইন্ডোতে সরান | উইন্ডো { $n }-এ সরান | অন্য ট্যাবগুলি বন্ধ করুন | বন্ধ করার আগে “{ $name }”-এর পরিবর্তন সংরক্ষণ করবেন? | সংরক্ষণ করুন | সংরক্ষণ করবেন না | বাতিল
ja | タブ | 開いているプレゼンテーション | 新しいタブ | タブを閉じる | 未保存の変更 | テンプレート | 新しいウィンドウに移動 | ウィンドウ { $n } に移動 | ほかのタブを閉じる | 閉じる前に「{ $name }」の変更を保存しますか？ | 保存 | 保存しない | キャンセル
ko | 탭 | 열린 프레젠테이션 | 새 탭 | 탭 닫기 | 저장하지 않은 변경 사항 | 템플릿 | 새 창으로 이동 | { $n }번 창으로 이동 | 다른 탭 닫기 | 닫기 전에 “{ $name }”의 변경 사항을 저장할까요? | 저장 | 저장 안 함 | 취소
zh-Hans | 标签页 | 已打开的演示文稿 | 新建标签页 | 关闭标签页 | 未保存的更改 | 模板 | 移到新窗口 | 移到窗口 { $n } | 关闭其他标签页 | 关闭前是否保存对“{ $name }”的更改？ | 保存 | 不保存 | 取消
zh-Hant | 分頁 | 已開啟的簡報 | 新增分頁 | 關閉分頁 | 未儲存的變更 | 範本 | 移至新視窗 | 移至視窗 { $n } | 關閉其他分頁 | 關閉前要儲存「{ $name }」的變更嗎？ | 儲存 | 不要儲存 | 取消
th | แท็บ | งานนำเสนอที่เปิดอยู่ | แท็บใหม่ | ปิดแท็บ | การเปลี่ยนแปลงที่ยังไม่บันทึก | เทมเพลต | ย้ายไปหน้าต่างใหม่ | ย้ายไปหน้าต่าง { $n } | ปิดแท็บอื่น | บันทึกการเปลี่ยนแปลงใน “{ $name }” ก่อนปิดหรือไม่ | บันทึก | ไม่บันทึก | ยกเลิก
vi | Thẻ | Bản trình bày đang mở | Thẻ mới | Đóng thẻ | Thay đổi chưa lưu | Mẫu | Chuyển sang cửa sổ mới | Chuyển sang cửa sổ { $n } | Đóng các thẻ khác | Lưu thay đổi của “{ $name }” trước khi đóng? | Lưu | Không lưu | Hủy
"""
count = 0
for line in rows.strip().splitlines():
    lang, section, *values = [p.strip() for p in line.split(" | ")]
    assert len(values) == len(keys), lang
    path = f"crates/oku-slides-i18n/src/locales/{lang}/ui.ftl"
    with open(path, "a", encoding="utf-8") as f:
        f.write(f"\n## {section}\n\n" + "".join(f"{k} = {v}\n" for k, v in zip(keys, values)))
    count += 1
assert count == 20, count
EOF
```

- [ ] **Step 4 : `ui/src/close.ts`.**

```ts
// Fermer un onglet modifié : la question, et l'enregistrement qui la suit.

import { inTauri, pickSavePath, type TabApi, type TabInfo } from "./api";
import { t } from "./i18n";

/** Ce que l'utilisateur veut faire du travail non enregistré. */
export type CloseChoice = "save" | "discard" | "cancel";

/** Libellés des trois boutons du dialogue. */
export interface CloseLabels {
  save: string;
  discard: string;
  cancel: string;
}

/** Le choix d'après la réponse du dialogue (libellé cliqué, ou réponse système). */
export function choiceOf(result: string, labels: CloseLabels): CloseChoice {
  if (result === labels.save || result === "Yes") return "save";
  if (result === labels.discard || result === "No") return "discard";
  return "cancel";
}

/** Demande quoi faire d'un document modifié qu'on ferme. */
async function askUnsaved(name: string): Promise<CloseChoice> {
  const text = t("ui-close-unsaved", { name });
  // Navigateur (développement) : OK enregistre, Annuler renonce.
  if (!inTauri) return window.confirm(text) ? "save" : "cancel";
  const labels = { save: t("ui-close-save"), discard: t("ui-close-discard"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const result = await message(text, { kind: "warning", buttons: { yes: labels.save, no: labels.discard, cancel: labels.cancel } });
  return choiceOf(result, labels);
}

/**
 * Enregistre un onglet avant de le fermer ; `false` : renoncé.
 *
 * L'ensemble de modèles en cours d'abord (puis le deck retrouvé), le deck
 * ensuite, avec un dialogue s'il n'a pas de fichier.
 */
export async function saveForClose(api: TabApi): Promise<boolean> {
  let s = await api.snapshot();
  if (s.editing) {
    if (s.dirty) s = await api.saveTemplates(false);
    s = await api.closeTemplates();
  }
  if (!s.dirty) return true;
  const path = s.path ? null : await pickSavePath(s.name);
  if (!s.path && !path) return false;
  await api.save(path);
  return true;
}

/**
 * Peut-on fermer cet onglet ? Pose la question s'il est modifié.
 *
 * `"discard"` : fermer en abandonnant le travail ; `"keep"` : fermer (rien à
 * perdre, ou enregistré) ; `null` : ne pas fermer.
 */
export async function mayClose(tab: TabInfo, api: TabApi): Promise<"discard" | "keep" | null> {
  if (!tab.dirty) return "keep";
  const choice = await askUnsaved(tab.title);
  if (choice === "discard") return "discard";
  if (choice === "save") return (await saveForClose(api)) ? "keep" : null;
  return null;
}
```

  `bun test close` → vert.

- [ ] **Step 5 : `ui/src/TabBar.tsx`.**

```tsx
// Barre d'onglets d'une fenêtre : cliquer, glisser, menu contextuel.

import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { WindowView, WorkspaceView } from "./api";
import { t } from "./i18n";
import { cycle, finalIndex, outside, slotAt } from "./tabs";

interface Props {
  view: WorkspaceView;
  win: WindowView;
  /** Barre de titre intégrée (macOS) : place pour les boutons de fenêtre. */
  overlay: boolean;
  /** Plusieurs fenêtres possibles (Tauri) : actions de fenêtre visibles. */
  windows: boolean;
  onActivate: (tab: string) => void;
  onClose: (tab: string) => void;
  onCloseOthers: (tab: string) => void;
  onNew: () => void;
  onReorder: (tab: string, index: number) => void;
  /** Vers `window` ; `null` : une nouvelle fenêtre. */
  onMove: (tab: string, window: string | null) => void;
  /** Lâché hors de la fenêtre, au point écran (`screenX`, `screenY`). */
  onDropOutside: (tab: string, x: number, y: number) => void;
}

/** Barre d'onglets d'une fenêtre. */
export default function TabBar({ view, win, overlay, windows, onActivate, onClose, onCloseOthers, onNew, onReorder, onMove, onDropOutside }: Props) {
  const els = useRef(new Map<string, HTMLElement>());
  const winRef = useRef(win);
  winRef.current = win;
  const [drag, setDrag] = useState<{ id: string; slot: number } | null>(null);
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(null);

  // Le menu se ferme au premier clic ailleurs, ou avec Échap.
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  /** Milieu de chaque onglet, à l'instant (les onglets ont pu changer). */
  const mids = () =>
    winRef.current.tabs.map((tab) => {
      const r = els.current.get(tab.id)?.getBoundingClientRect();
      return r ? r.left + r.width / 2 : 0;
    });

  const onPointerDown = (e: ReactPointerEvent<HTMLElement>, id: string) => {
    if (e.button !== 0) return;
    if (id !== winRef.current.active) onActivate(id);
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const [startX, startY] = [e.clientX, e.clientY];
    let moved = false;
    let slot = -1;
    const move = (ev: PointerEvent) => {
      // Dans les deux axes : tirer un onglet vers le bas le détache aussi.
      if (!moved && Math.hypot(ev.clientX - startX, ev.clientY - startY) < 4) return;
      moved = true;
      const next = slotAt(mids(), ev.clientX);
      if (next !== slot) {
        slot = next;
        setDrag({ id, slot });
      }
    };
    const end = (ev: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      setDrag(null);
      if (!moved || ev.type === "pointercancel") return;
      if (outside(ev.clientX, ev.clientY, window.innerWidth, window.innerHeight)) {
        if (windows) onDropOutside(id, ev.screenX, ev.screenY);
        return;
      }
      const from = winRef.current.tabs.findIndex((tab) => tab.id === id);
      const to = finalIndex(from, slotAt(mids(), ev.clientX));
      if (from >= 0 && to !== from) onReorder(id, to);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  };

  const pick = (action: () => void) => () => {
    action();
    setMenu(null);
  };
  // Dans le navigateur, le dernier onglet reste : sans lui, plus de fenêtre.
  const lastStays = !windows && win.tabs.length === 1;

  return (
    <div className={`tabbar${overlay ? " tabbar-overlay" : ""}${drag?.slot === win.tabs.length ? " drop-end" : ""}`} data-tauri-drag-region>
      <div className="tabs" role="tablist" aria-label={t("ui-tabs")}>
        {win.tabs.map((tab, i) => (
          <div
            key={tab.id}
            ref={(el) => {
              if (el) els.current.set(tab.id, el);
              else els.current.delete(tab.id);
            }}
            role="tab"
            tabIndex={tab.id === win.active ? 0 : -1}
            aria-selected={tab.id === win.active}
            className={`tab${tab.id === win.active ? " active" : ""}${drag?.id === tab.id ? " dragging" : ""}${drag?.slot === i ? " drop-before" : ""}`}
            title={tab.path ?? tab.title}
            onPointerDown={(e) => onPointerDown(e, tab.id)}
            onKeyDown={(e) => {
              // Motif WAI-ARIA des onglets : Entrée / Espace activent, les flèches déplacent.
              const ids = win.tabs.map((x) => x.id);
              const next =
                e.key === "ArrowRight" ? cycle(ids, tab.id, 1) : e.key === "ArrowLeft" ? cycle(ids, tab.id, -1) : undefined;
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onActivate(tab.id);
              } else if (next) {
                e.preventDefault();
                onActivate(next);
                els.current.get(next)?.focus();
              }
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              setMenu({
                id: tab.id,
                x: Math.min(e.clientX, window.innerWidth - 260),
                y: Math.min(e.clientY, window.innerHeight - 180),
              });
            }}
          >
            {tab.editing_templates && <span className="tab-badge">{t("ui-tab-templates")}</span>}
            <span className="tab-title">{tab.title}</span>
            {tab.dirty && <span className="tab-dirty" role="img" aria-label={t("ui-tab-unsaved")} />}
            <button
              className="tab-close"
              aria-label={t("ui-tab-close")}
              title={t("ui-tab-close")}
              disabled={lastStays}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={() => onClose(tab.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <button className="tab-new" aria-label={t("ui-tab-new")} title={t("ui-tab-new")} onClick={onNew}>
        +
      </button>
      {menu && (
        <div className="tab-menu object" role="menu" style={{ left: menu.x, top: menu.y }} onPointerDown={(e) => e.stopPropagation()}>
          {windows && <button role="menuitem" onClick={pick(() => onMove(menu.id, null))}>{t("ui-tab-move-new-window")}</button>}
          {windows &&
            view.windows.map((w, i) =>
              w.label === win.label ? null : (
                <button key={w.label} role="menuitem" onClick={pick(() => onMove(menu.id, w.label))}>
                  {t("ui-tab-move-to", { n: i + 1 })}
                </button>
              ),
            )}
          <button role="menuitem" disabled={win.tabs.length < 2} onClick={pick(() => onCloseOthers(menu.id))}>{t("ui-tab-close-others")}</button>
          <button role="menuitem" disabled={lastStays} onClick={pick(() => onClose(menu.id))}>{t("ui-tab-close")}</button>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 6 : `ui/src/Workspace.tsx` complet** (remplace la version de la tâche 4) :

```tsx
// Une fenêtre : la barre d'onglets et l'`App` de l'onglet actif.

import { useCallback, useEffect, useRef, useState } from "react";
import App from "./App";
import TabBar from "./TabBar";
import { errorText, inTauri, isMac, tabApi, workspace, type WorkspaceView } from "./api";
import { mayClose } from "./close";
import { t } from "./i18n";
import { cycle, nth, tabShortcut } from "./tabs";

/** Label de cette fenêtre : celui de Tauri, ou la seule fenêtre en développement. */
async function ownLabel(view: WorkspaceView): Promise<string> {
  if (!inTauri) return view.windows[0]?.label ?? "";
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().label;
}

/** Écoute un événement Tauri ; l'abonnement suit la vie de l'effet (StrictMode compris). */
function listenTauri(subscribe: () => Promise<() => void>): () => void {
  let stop: (() => void) | undefined;
  let dead = false;
  subscribe()
    .then((un) => (dead ? un() : (stop = un)))
    .catch((e) => console.error("événement de l'hôte", e));
  return () => {
    dead = true;
    stop?.();
  };
}

/** Une fenêtre : sa barre d'onglets et l'`App` de l'onglet actif. */
export default function Workspace() {
  const [view, setView] = useState<WorkspaceView | null>(null);
  const [label, setLabel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    workspace.view().then(setView).catch((e) => setError(errorText(e)));
  }, []);

  const run = useCallback(
    async (p: Promise<WorkspaceView>) => {
      try {
        setView(await p);
        setError(null);
      } catch (e) {
        setError(errorText(e));
        refresh();
      }
    },
    [refresh],
  );

  // L'écoute d'abord, la vue de départ ensuite : aucune diffusion perdue entre les deux.
  useEffect(() => {
    const first = () =>
      workspace
        .view()
        .then(async (v) => {
          setView(v);
          setLabel(await ownLabel(v));
        })
        .catch((e) => setError(errorText(e)));
    if (!inTauri) {
      first();
      return;
    }
    return listenTauri(async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen<WorkspaceView>("workspace", (e) => setView(e.payload));
      first();
      return stop;
    });
  }, []);

  const win = view?.windows.find((w) => w.label === label);
  const viewRef = useRef(view);
  viewRef.current = view;
  const winRef = useRef(win);
  winRef.current = win;

  /** La vue relue juste avant de demander : son « modifié » est à jour. */
  const freshTabs = async () => {
    const v = await workspace.view();
    setView(v);
    return v.windows.find((w) => w.label === label)?.tabs ?? [];
  };

  /** Ferme un onglet après la question ; `false` : l'utilisateur a renoncé. */
  const closeTab = useCallback(
    async (id: string): Promise<boolean> => {
      try {
        const tab = (await freshTabs()).find((x) => x.id === id);
        if (!tab) return true;
        const verdict = await mayClose(tab, tabApi(id));
        if (!verdict) return false;
        await run(workspace.close(id, verdict === "discard"));
        return true;
      } catch (e) {
        setError(errorText(e));
        return false;
      }
    },
    [run, label],
  );

  // « Annuler » sur un onglet arrête tout.
  const closeOthers = async (id: string) => {
    for (const tab of winRef.current?.tabs ?? []) {
      if (tab.id !== id && !(await closeTab(tab.id))) return;
    }
  };

  // Fermer la fenêtre (l'hôte l'a empêché et prévient) : la dernière quitte
  // en gardant tout ; une autre pose la question pour ses onglets modifiés.
  // Deux demandes rapprochées (double clic) ne posent pas deux fois les questions.
  const closing = useRef(false);
  useEffect(() => {
    if (!inTauri || !label) return;
    return listenTauri(async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const { invoke } = await import("@tauri-apps/api/core");
      return getCurrentWebviewWindow().listen("close-requested", async () => {
        if (closing.current) return;
        closing.current = true;
        try {
          if ((viewRef.current?.windows.length ?? 0) <= 1) return void (await invoke("quit"));
          const discard: string[] = [];
          for (const tab of await freshTabs()) {
            if (!tab.dirty) continue;
            await workspace.activate(tab.id);
            const verdict = await mayClose(tab, tabApi(tab.id));
            if (!verdict) return;
            if (verdict === "discard") discard.push(tab.id);
          }
          await workspace.closeWindow(label, discard);
        } catch (e) {
          setError(errorText(e));
        } finally {
          closing.current = false;
        }
      });
    });
  }, [label]);

  // Raccourcis des onglets.
  useEffect(() => {
    if (!label) return;
    const onKey = (e: KeyboardEvent) => {
      const action = tabShortcut(e, isMac);
      const w = winRef.current;
      if (!action || !w) return;
      if (action.kind === "new-window" && !inTauri) return;
      // Pendant une saisie (texte d'une slide, champ), changer d'onglet
      // perdrait ce qui n'est pas validé : seul Ctrl+Tab reste actif.
      const typing = (e.target as HTMLElement | null)?.closest?.("input, textarea, select, [contenteditable]");
      if (typing && action.kind !== "cycle") return;
      e.preventDefault();
      const ids = w.tabs.map((x) => x.id);
      if (action.kind === "new-tab") run(workspace.newTab(label));
      else if (action.kind === "close-tab") {
        if (inTauri || ids.length > 1) closeTab(w.active);
      } else if (action.kind === "new-window") run(workspace.newWindow());
      else {
        const id = action.kind === "cycle" ? cycle(ids, w.active, action.delta) : nth(ids, action.n);
        if (id && id !== w.active) run(workspace.activate(id));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [label, run, closeTab]);

  if (!view || !label || !win) {
    return <div className="table"><p className="empty" style={{ padding: 40 }}>{error ?? t("ui-loading")}</p></div>;
  }

  // Un fichier ouvert devient l'onglet actif (nouveau, ou déjà ouvert
  // ailleurs) : `key` remonte l'`App`. Déjà actif : rien à recharger.
  const onOpen = (path: string) => void run(workspace.openTab(label, path));
  const dropOutside = async (id: string, x: number, y: number) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("drop_tab", { tab: id, x, y });
    } catch (e) {
      setError(errorText(e));
    }
  };

  return (
    <div className="workspace">
      <TabBar
        view={view}
        win={win}
        overlay={inTauri && isMac}
        windows={inTauri}
        onActivate={(id) => run(workspace.activate(id))}
        onClose={closeTab}
        onCloseOthers={closeOthers}
        onNew={() => run(workspace.newTab(label))}
        onReorder={(id, index) => run(workspace.reorder(id, index))}
        onMove={(id, target) => run(workspace.moveTab(id, target))}
        onDropOutside={dropOutside}
      />
      {error && <p className="workspace-error" role="alert">{error}</p>}
      <div className="workspace-body">
        <App key={win.active} tab={win.active} onOpen={onOpen} onChange={inTauri ? undefined : refresh} />
      </div>
    </div>
  );
}
```

  Note : ⌘W est capté ici. Sous macOS, le menu d'app par défaut de Tauri a « Fermer la fenêtre » sur ⌘W, qui passerait avant la page : la tâche 6 construit un menu sans cet élément.

- [ ] **Step 7 : `styles.css`.** Ajouter (à la fin du fichier) :

```css
/* Onglets : la bande du haut de la fenêtre (la barre de titre intégrée sous macOS). */
.workspace { height: 100%; display: flex; flex-direction: column; }
.workspace-body { flex: 1; min-height: 0; }
.workspace-error { margin: 0; padding: 4px 12px; background: var(--danger); color: var(--on-accent); font-size: 12px; }
.tabbar {
  height: 36px; flex: none; display: flex; align-items: flex-end; gap: 2px;
  padding: 0 var(--space-2); background: var(--shade); position: relative;
}
/* `tabbar-overlay`, pas `overlay` : cette classe existe déjà (plein écran). */
.tabbar.tabbar-overlay { padding-left: 80px; }
.tabs { display: flex; align-items: flex-end; gap: 2px; min-width: 0; }
.tab {
  display: flex; align-items: center; gap: 6px; min-width: 96px; max-width: 220px; height: 30px;
  padding: 0 4px 0 12px; border-radius: var(--radius-md) var(--radius-md) 0 0;
  color: var(--ink-muted); cursor: default; position: relative;
}
.tab.active { background: var(--paper-200); color: var(--ink); }
.tab.dragging { opacity: 0.5; }
.tab.drop-before::before, .tabbar.drop-end::after {
  content: ""; position: absolute; top: 6px; bottom: 6px; width: 2px; background: var(--accent);
}
.tab.drop-before::before { left: -2px; }
.tabbar.drop-end::after { right: var(--space-2); }
.tab-title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tab-badge { font-size: 11px; color: var(--accent); }
.tab-dirty { width: 7px; height: 7px; border-radius: 50%; background: currentColor; flex: none; }
.tab-close, .tab-new {
  border: 0; background: none; color: inherit; width: 22px; height: 22px; flex: none;
  border-radius: var(--radius-sm); font-size: 16px; line-height: 1; cursor: pointer;
}
/* Invisible mais toujours dans l'arbre d'accessibilité (état « désactivé » lisible). */
.tab-close:disabled { opacity: 0; pointer-events: none; }
.tab-new { margin-bottom: 4px; }
.tab-close:hover, .tab-new:hover { background: var(--paper-200); }
.tab-menu { position: fixed; z-index: 20; display: flex; flex-direction: column; padding: 4px; min-width: 240px; }
.tab-menu button { border: 0; background: none; color: var(--ink); text-align: start; padding: 6px 10px; border-radius: var(--radius-sm); }
.tab-menu button:hover:not(:disabled) { background: var(--shade); }
.tab-menu button:disabled { color: var(--ink-muted); }
```

- [ ] **Step 8 :** `cd ui && bun test && bun run typecheck`, `cargo nextest run -p oku-slides-i18n`, puis `just e2e` → vert (dont `tabs: ok`).

- [ ] **Step 9 : commit**

```bash
git add ui crates/oku-slides-i18n
git commit -m "feat(ui) : barre d'onglets, fermeture et raccourcis (t-151)"
oku cli task done --project slider --id t-151
```

---

### Task 6 : Hôte Tauri — fenêtres, commandes et glisser entre fenêtres (t-152)

**Files:**
- Create: `src-tauri/src/commands.rs`, `src-tauri/src/windows.rs`, `src-tauri/src/menu.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`menu-edit`, `menu-window` ajoutées ; `session-busy` retirée)

**Interfaces:**
- Consumes: `Workspace::{resume, apply, request, lock, open_files, view, focus, window_at}`, `WorkspaceRequest`, `Rect`.
- Produces: commandes `request(tab, req)`, `workspace(req)`, `drop_tab(tab, x, y)`, `quit()`, `stt(req)` ; événements `workspace` (payload `WorkspaceView`, toutes les fenêtres) et `close-requested` (la fenêtre concernée, jamais la dernière) ; `commands::open_files(app, files)`, `commands::AppState` et `launch_files` pour la tâche 7.

`src-tauri` n'a pas de tests automatiques (exclu de `just test`) : la logique testable est dans `Workspace`. Cette tâche se vérifie par clippy et à la main.

- [ ] **Step 1 : `src-tauri/src/windows.rs`.**

```rust
//! Fenêtres de l'app : une par fenêtre de l'espace de travail, même label.
//!
//! Taille et position : `tauri-plugin-window-state`, par label. Toutes les
//! coordonnées ici sont logiques (points sous macOS).

use std::sync::PoisonError;

use oku_slides_session::{Rect, Workspace, WorkspaceView};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::AppState;

/// Taille d'une fenêtre neuve, et taille minimale.
const SIZE: (f64, f64) = (1440.0, 900.0);
const MIN_SIZE: (f64, f64) = (1024.0, 640.0);
/// Décalage du coin de la fenêtre au pointeur : l'onglet lâché tombe dans sa
/// barre, à droite des boutons de fenêtre.
const DROP_OFFSET: (f64, f64) = (160.0, 18.0);

/// Ouvre la fenêtre `label`, sous le pointeur `at` s'il est donné.
fn open(app: &AppHandle, label: &str, at: Option<LogicalPosition<f64>>) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Oku Slides")
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    if let Some(p) = at {
        builder = builder.position(p.x - DROP_OFFSET.0, p.y - DROP_OFFSET.1);
    }
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    builder.build()
}

/// Rectangle logique de chaque fenêtre visible.
pub fn rects(app: &AppHandle) -> Vec<(String, Rect)> {
    app.webview_windows()
        .into_iter()
        .filter(|(_, w)| !w.is_minimized().unwrap_or(false))
        .filter_map(|(label, w)| {
            let scale = w.scale_factor().ok()?;
            let p = w.outer_position().ok()?.to_logical::<f64>(scale);
            let s = w.outer_size().ok()?.to_logical::<f64>(scale);
            Some((label, Rect { x: p.x, y: p.y, w: s.width, h: s.height }))
        })
        .collect()
}

/// Diffuse une vue à toutes les fenêtres.
fn emit(app: &AppHandle, view: &WorkspaceView) {
    if let Err(e) = app.emit("workspace", view) {
        tracing::warn!(error = %e, "vue de l'espace de travail non diffusée");
    }
}

/// Diffuse la vue courante (un onglet a changé de titre ou d'état).
///
/// Sérialisé avec `sync` : une vue plus ancienne n'arrive jamais après.
pub fn emit_view(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let _serial = state.sync.lock().unwrap_or_else(PoisonError::into_inner);
    let view = Workspace::lock(&state.workspace).view();
    emit(app, &view);
}

/// Met les fenêtres d'accord avec la vue courante.
///
/// Ouvre celles qui manquent (sous `at`), ferme celles qui n'y sont plus,
/// prévient toutes les fenêtres ; `focus` : la fenêtre active passe au
/// premier plan. Sérialisé et toujours sur la vue la plus récente : deux
/// commandes simultanées ne s'appliquent jamais dans le désordre. Plus
/// aucune fenêtre (dernier onglet fermé) : l'app se termine.
pub fn sync(app: &AppHandle, at: Option<LogicalPosition<f64>>, focus: bool) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let _serial = state.sync.lock().unwrap_or_else(PoisonError::into_inner);
    let view = Workspace::lock(&state.workspace).view();
    for w in &view.windows {
        if app.get_webview_window(&w.label).is_none()
            && let Err(e) = open(app, &w.label, at)
        {
            tracing::warn!(label = %w.label, error = %e, "fenêtre non ouverte");
        }
    }
    for (label, w) in app.webview_windows() {
        if !view.windows.iter().any(|v| v.label == label)
            && let Err(e) = w.destroy()
        {
            tracing::warn!(label, error = %e, "fenêtre non fermée");
        }
    }
    if focus && let Some(w) = app.get_webview_window(&view.focused) {
        let shown = w.unminimize().and_then(|()| w.show()).and_then(|()| w.set_focus());
        if let Err(e) = shown {
            tracing::warn!(label = %view.focused, error = %e, "fenêtre non mise au premier plan");
        }
    }
    emit(app, &view);
    if view.windows.is_empty() {
        app.exit(0);
    }
}
```

- [ ] **Step 2 : `src-tauri/src/commands.rs`.**

```rust
//! Commandes de l'interface et réactions aux événements des fenêtres.
//!
//! Le travail (rendu, lecture de fichiers) tourne hors du fil principal
//! (`spawn_blocking`) ; les fenêtres suivent la vue ensuite (`windows::sync`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use oku_slides_session::{Localize, Localizer, Request, SessionError, Workspace, WorkspaceRequest};
use oku_slides_stt::{Status, Stt, SttRequest};
use tauri::ipc::Response;
use tauri::{AppHandle, LogicalPosition, Manager};

use crate::windows;

/// État de l'app : l'espace de travail et ce qu'il faut autour.
pub struct AppState {
    /// Fenêtres, onglets, et une session par onglet.
    pub workspace: Mutex<Workspace>,
    /// Langue des erreurs renvoyées à l'interface.
    pub localizer: Arc<Localizer>,
    /// Sérialise `windows::sync`.
    pub sync: Mutex<()>,
}

impl AppState {
    /// Une erreur dans la langue de l'utilisateur ; journalisée en anglais.
    fn refuse(&self, e: SessionError) -> String {
        tracing::warn!(error = %e.one_line(), "requête refusée");
        e.localize(&self.localizer)
    }

    /// L'espace de travail s'il est libre (fil principal : jamais d'attente).
    ///
    /// Un verrou empoisonné est repris, comme partout.
    pub fn try_workspace(&self) -> Option<MutexGuard<'_, Workspace>> {
        match self.workspace.try_lock() {
            Ok(ws) => Some(ws),
            Err(TryLockError::Poisoned(p)) => Some(p.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }
}

/// Travail bloquant hors du fil principal ; sa panique devient une erreur.
async fn blocking<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.try_state::<AppState>().ok_or_else(|| "app not ready".to_owned())?;
        f(&state)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// La fenêtre est la seule : la fermer, c'est quitter en gardant tout.
fn is_last_window(state: &AppState, label: &str) -> bool {
    let ws = Workspace::lock(&state.workspace);
    let view = ws.view();
    view.windows.len() <= 1 && view.windows.iter().all(|w| w.label == label)
}

/// Une requête pour l'onglet `tab`.
#[tauri::command]
pub async fn request(app: AppHandle, tab: String, req: Request) -> Result<Response, String> {
    let emitter = app.clone();
    let reply = blocking(&app, move |state| {
        let (reply, changed) = Workspace::request(&state.workspace, &tab, req);
        if changed {
            windows::emit_view(&emitter);
        }
        Ok(reply.map_err(|e| state.refuse(e)))
    })
    .await?;
    reply.map(Response::new)
}

/// Une requête de l'espace de travail ; les fenêtres suivent.
///
/// Fermer la dernière fenêtre quitte l'app en gardant tout : la règle vit
/// ici, pas seulement dans l'interface.
#[tauri::command]
pub async fn workspace(app: AppHandle, req: WorkspaceRequest) -> Result<Response, String> {
    let changes = !matches!(req, WorkspaceRequest::View | WorkspaceRequest::Messages);
    let focus = matches!(
        req,
        WorkspaceRequest::OpenTab { .. } | WorkspaceRequest::NewWindow | WorkspaceRequest::MoveTab { .. }
    );
    // `None` : c'était la dernière fenêtre, on quitte en gardant tout.
    let reply = blocking(&app, move |state| {
        if let WorkspaceRequest::CloseWindow { window, .. } = &req
            && is_last_window(state, window)
        {
            return Ok(None);
        }
        Workspace::apply(&state.workspace, req).map(Some).map_err(|e| state.refuse(e))
    })
    .await?;
    let Some(reply) = reply else {
        app.exit(0);
        return Ok(Response::new(String::new()));
    };
    if changes {
        windows::sync(&app, None, focus);
    }
    Ok(Response::new(reply))
}

/// Onglet lâché hors de sa fenêtre au point écran `(x, y)`.
///
/// Il rejoint la fenêtre sous ce point, sinon une nouvelle fenêtre à cet
/// endroit (l'écran visé).
#[tauri::command]
pub async fn drop_tab(app: AppHandle, tab: String, x: f64, y: f64) -> Result<(), String> {
    if !(x.is_finite() && y.is_finite()) {
        return Err("invalid drop point".to_owned());
    }
    let rects = windows::rects(&app);
    blocking(&app, move |state| {
        let window = Workspace::lock(&state.workspace).window_at(x, y, &rects);
        Workspace::apply(&state.workspace, WorkspaceRequest::MoveTab { tab, window, index: None })
            .map_err(|e| state.refuse(e))
    })
    .await?;
    windows::sync(&app, Some(LogicalPosition::new(x, y)), true);
    Ok(())
}

/// Quitter en gardant tout (repli de l'interface pour la dernière fenêtre).
#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Dictée : jamais bloquant (téléchargement, micro et Whisper tournent dans leurs fils).
#[tauri::command]
pub fn stt(engine: tauri::State<'_, Stt>, req: SttRequest) -> Status {
    engine.handle(req)
}

/// Une fenêtre passe au premier plan (fil principal : jamais d'attente).
///
/// Si l'espace de travail est occupé, l'ordre attendra le prochain focus.
pub fn focused(app: &AppHandle, label: &str) {
    let Some(state) = app.try_state::<AppState>() else { return };
    if let Some(mut ws) = state.try_workspace()
        && let Err(e) = ws.focus(label)
    {
        // Une fenêtre en cours de création n'y est pas encore.
        tracing::debug!(label, error = %e.one_line(), "focus ignoré");
    }
}

/// Fermeture native demandée (bouton, ⌘W du système, Alt+F4).
///
/// La dernière fenêtre quitte l'app en gardant tout, sans passer par la
/// page ; une autre prévient sa page, qui pose la question pour ses onglets
/// modifiés. Espace de travail occupé : la page décide (elle aussi sait
/// quitter depuis la dernière fenêtre).
pub fn close_requested(app: &AppHandle, label: &str) {
    use tauri::Emitter;
    let last = app.try_state::<AppState>().and_then(|state| {
        let ws = state.try_workspace()?;
        let view = ws.view();
        Some(view.windows.len() <= 1)
    });
    if last == Some(true) {
        app.exit(0);
        return;
    }
    if let Err(e) = app.emit_to(label, "close-requested", ()) {
        tracing::warn!(label, error = %e, "fermeture non transmise");
    }
}

/// Ouvre des fichiers reçus, hors du fil principal ; la fenêtre active passe devant.
pub fn open_files(app: &AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let opened = blocking(&app, move |state| {
            Workspace::open_files(&state.workspace, &files);
            Ok(())
        })
        .await;
        if let Err(e) = opened {
            tracing::warn!(error = %e, "fichiers reçus non ouverts");
        }
        windows::sync(&app, None, true);
    });
}
```

- [ ] **Step 3 : menu macOS.** Le menu d'app par défaut de Tauri met « Fermer la fenêtre » sur ⌘W, qui passerait avant la page : ⌘W fermerait la fenêtre au lieu de l'onglet. `src-tauri/src/menu.rs` :

```rust
//! Menu d'app sous macOS : le menu par défaut, sans « Fermer la fenêtre ».
//!
//! ⌘W appartient à la page (fermer l'onglet). Les autres systèmes n'ont pas
//! de menu d'app.

use oku_slides_session::Localizer;
use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Runtime};

/// Le menu : application, édition (couper, copier, coller), fenêtre.
pub fn build<R: Runtime>(app: &AppHandle<R>, l: &Localizer) -> tauri::Result<Menu<R>> {
    let about = PredefinedMenuItem::about(app, None, None)?;
    let app_menu = Submenu::with_items(
        app,
        "Oku Slides",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        l.tr("menu-edit"),
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        l.tr("menu-window"),
        true,
        &[&PredefinedMenuItem::minimize(app, None)?, &PredefinedMenuItem::fullscreen(app, None)?],
    )?;
    Menu::with_items(app, &[&app_menu, &edit, &window])
}
```

  Clés Fluent, dans chaque `session.ftl`, sous un commentaire `## Menu` ; et retrait de `session-busy` (plus aucun usage : l'ancienne commande `request` disparaît) :

| locale | `menu-edit` | `menu-window` |
|---|---|---|
| en | Edit | Window |
| fr | Édition | Fenêtre |
| de | Bearbeiten | Fenster |
| es | Edición | Ventana |
| it | Modifica | Finestra |
| pt | Editar | Janela |
| nl | Wijzig | Venster |
| pl | Edycja | Okno |
| ru | Правка | Окно |
| uk | Редагування | Вікно |
| tr | Düzen | Pencere |
| ar | تحرير | نافذة |
| hi | संपादन | विंडो |
| bn | সম্পাদনা | উইন্ডো |
| ja | 編集 | ウインドウ |
| ko | 편집 | 윈도우 |
| zh-Hans | 编辑 | 窗口 |
| zh-Hant | 編輯 | 視窗 |
| th | แก้ไข | หน้าต่าง |
| vi | Sửa | Cửa sổ |

```bash
sed -i '' '/^session-busy = /d' crates/oku-slides-i18n/src/locales/*/session.ftl
grep -rn '"session-busy"' crates src-tauri
```

  Le `grep` ne trouve rien. (Le plan historique `docs/superpowers/plans/2026-09-26-agent-vocal.md` cite cette clé : c'est un document daté, il reste tel quel.)

- [ ] **Step 4 : `src-tauri/src/main.rs`** entier :

```rust
//! Oku Slides : application de bureau.
//!
//! Toute la logique est dans `oku-slides-session` : l'espace de travail
//! (fenêtres, onglets) et une session par onglet. L'interface appelle
//! `request` (un onglet), `workspace` (onglets et fenêtres), `drop_tab` et
//! `quit`. Démarrage : journaux → plateforme → espace de travail (reprise)
//! et dictée → fenêtres ; chacune reprend sa taille et sa position. Les
//! fichiers reçus au lancement s'ouvrent ensuite, hors du fil principal.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod menu;
mod windows;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_session::{Fonts, Platform, Storage, Workspace};
use oku_slides_stt::Stt;
use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use commands::AppState;

/// Polices livrées avec l'app (ressource `fonts/`), plus `paths.fonts` des
/// réglages ; un dossier illisible est signalé et ignoré.
fn app_fonts(storage: &Storage, dirs: &[PathBuf]) -> Fonts {
    Fonts::new(
        dirs.iter()
            .flat_map(|dir| {
                storage.read_fonts(dir).unwrap_or_else(|e| {
                    tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "polices");
                    Default::default()
                })
            })
            .map(|(_, bytes)| bytes),
    )
}

/// Fichiers reçus en arguments : relatifs au dossier de lancement, et des fichiers.
fn launch_files(args: impl Iterator<Item = PathBuf>, cwd: &Path) -> Vec<PathBuf> {
    args.map(|p| if p.is_absolute() { p } else { cwd.join(p) }).filter(|p| p.is_file()).collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let platform = Platform::load()?;
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_window_event(|window, event| match event {
            // Taille et position gardées à chaque changement, pas seulement à la fermeture.
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if let Err(e) = window.app_handle().save_window_state(StateFlags::all()) {
                    tracing::warn!(error = %e, "état de la fenêtre non enregistré");
                }
            }
            WindowEvent::Focused(true) => commands::focused(window.app_handle(), window.label()),
            // Jamais de fermeture native : l'hôte (dernière fenêtre) ou la page décide.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                commands::close_requested(window.app_handle(), window.label());
            }
            _ => {}
        });
    #[cfg(target_os = "macos")]
    let builder = {
        let localizer = platform.localizer.clone();
        builder.menu(move |app| menu::build(app, &localizer))
    };
    let app = builder
        .setup(move |app| {
            let mut dirs = Vec::new();
            if let Ok(res) = app.path().resource_dir() {
                dirs.push(res.join("fonts"));
            }
            dirs.extend(platform.settings.paths.fonts.clone());
            // En développement, les polices du dépôt.
            dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
            let fonts = app_fonts(&platform.storage, &dirs);
            let workspace = Workspace::resume(fonts, &platform, platform.settings.paths.data().join("resume"))?;
            app.manage(AppState {
                workspace: Mutex::new(workspace),
                localizer: platform.localizer.clone(),
                sync: Mutex::new(()),
            });
            app.manage(Stt::new(&platform));
            windows::sync(app.handle(), None, true);
            // Ouvrir avec… (Windows, Linux) : chaque fichier dans un onglet, hors du fil principal.
            let cwd = std::env::current_dir().unwrap_or_default();
            commands::open_files(app.handle(), launch_files(std::env::args_os().skip(1).map(PathBuf::from), &cwd));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::request,
            commands::workspace,
            commands::drop_tab,
            commands::quit,
            commands::stt
        ])
        .build(tauri::generate_context!())?;
    app.run(|_, _| {});
    Ok(())
}
```

  (`menu` n'est utilisé que sous macOS : sur les autres systèmes, `#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "menu d'app : macOS seulement"))]` sur `pub fn build`.)

- [ ] **Step 5 : configuration.**
  - `tauri.conf.json` : `"windows": []` (les fenêtres sont créées par `windows::sync`).
  - `capabilities/default.json` :

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Fenêtres de l'app (toutes étiquetées w-<n>, même confiance) : commandes, événements, dialogues.",
  "windows": ["w-*"],
  "permissions": [
    "core:default",
    "core:window:allow-set-background-color",
    "core:window:allow-start-dragging",
    "dialog:allow-open",
    "dialog:allow-save",
    "dialog:allow-message"
  ]
}
```

  (Pas de `allow-destroy` : l'hôte seul ferme les fenêtres. La CSP de `tauri.conf.json` interdit tout script distant : les webviews restent de confiance. Toute fenêtre ajoutée plus tard doit rester `w-<n>` et de même confiance, ou avoir sa propre capacité.)

- [ ] **Step 6 :** `cargo nextest run -p oku-slides-i18n`, `cargo clippy -p oku-slides-app --all-targets -- -D warnings` et `cargo build -p oku-slides-app` → propres.

- [ ] **Step 7 : vérification manuelle** (`cargo tauri dev`) :
  1. Avant le premier lancement de cette version, effacer `<data>/resume/` à la main (l'ancien format n'est pas repris). Premier lancement : une fenêtre, un onglet vierge. La taille et la position enregistrées sous l'ancien label `main` sont perdues une fois : attendu.
  2. ⌘T, ⌘1/⌘2 (clavier AZERTY), ⌃Tab : les onglets suivent. ⌘Z annule, sans fermer d'onglet. ⌘W ferme l'onglet, jamais la fenêtre ; ⌘W sur le dernier onglet de la dernière fenêtre termine l'app.
  3. ⌘⇧N : deuxième fenêtre. Menu contextuel → « Déplacer vers la fenêtre 2 » : l'onglet y passe, actif.
  4. Deux écrans : glisser un onglet hors de sa fenêtre (y compris vers le bas), loin de toute fenêtre, sur l'autre écran → nouvelle fenêtre dont la barre est sous le pointeur. Sur l'autre fenêtre → il la rejoint.
  5. Modifier un deck, ⌘W : dialogue Enregistrer / Ne pas enregistrer / Annuler, chacun fait ce qu'il dit. Fermer une fenêtre (pas la dernière) avec un onglet modifié : même question.
  6. Fermer la dernière fenêtre (bouton rouge) : l'app quitte sans question ; relancer : mêmes fenêtres, onglets, onglet actif, historique. Idem avec ⌘Q.
  7. Fermer le dernier onglet de la dernière fenêtre (⌘W) : l'app se termine ; relancer : une fenêtre, un onglet vierge.
  8. Menus : Édition (couper, copier, coller dans un champ de texte) et Fenêtre dans la langue du système.

- [ ] **Step 8 : commit**

```bash
git add src-tauri crates/oku-slides-i18n
git commit -m "feat(app) : fenêtres et onglets, glisser un onglet entre fenêtres (t-152)"
oku cli task done --project slider --id t-152
```

---

### Task 7 : Instance unique et ouverture sous macOS (t-153)

**Files:**
- Modify: `Cargo.toml` (workspace), `Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `commands::{open_files, AppState::try_workspace}`, `launch_files` (tâche 6).

- [ ] **Step 1 : dépendances.** `[workspace.dependencies]` de `Cargo.toml` : `tauri = "2.11"`, `tauri-plugin-dialog = "2.7"` (boutons personnalisés de `message`, comme le paquet JS relevé en tâche 5), `tauri-plugin-window-state = "2.4"` (restauration des fenêtres créées après le démarrage), et après lui : `tauri-plugin-single-instance = "2.4"`. `src-tauri/Cargo.toml` : `tauri-plugin-single-instance.workspace = true`. Le plugin sert sous tous les systèmes (y compris macOS, pour un binaire relancé depuis un terminal).

- [ ] **Step 2 :** `cargo deny check` → propre (licences, avis de sécurité, sources ; le plugin ajoute `zbus` sous Linux).

- [ ] **Step 3 : file d'attente macOS.** Dans `main.rs` :

```rust
/// Fichiers envoyés par macOS avant la fin du démarrage (`RunEvent::Opened`
/// peut précéder `setup`) ; vidé à la fin de `setup`.
#[derive(Default)]
struct Pending(Mutex<Vec<PathBuf>>);
```

  `tauri::Builder::default()` devient `tauri::Builder::default().manage(Pending::default())`. Dans `setup`, les fichiers en attente rejoignent ceux des arguments :

```rust
            let cwd = std::env::current_dir().unwrap_or_default();
            let mut files = launch_files(std::env::args_os().skip(1).map(PathBuf::from), &cwd);
            files.append(&mut app.state::<Pending>().0.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
            commands::open_files(app.handle(), files);
```

  (à la place des deux lignes `let cwd …` / `commands::open_files(…)` de la tâche 6.)

- [ ] **Step 4 : plugin** (toujours le premier : la seconde instance s'arrête dans son `setup`), juste après `.manage(Pending::default())` :

```rust
        // Toujours en premier : une seconde instance transmet ses arguments
        // à celle-ci, puis s'arrête avant d'avoir rien ouvert.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let files = launch_files(argv.into_iter().skip(1).map(PathBuf::from), Path::new(&cwd));
            if !files.is_empty() {
                return commands::open_files(app, files);
            }
            // Relancée sans fichier : la fenêtre active revient au premier plan.
            let Some(state) = app.try_state::<AppState>() else { return };
            let Some(focused) = state.try_workspace().map(|ws| ws.focused().to_owned()) else { return };
            if let Some(w) = app.get_webview_window(&focused)
                && let Err(e) = w.unminimize().and_then(|()| w.set_focus())
            {
                tracing::warn!(label = focused, error = %e, "fenêtre non mise au premier plan");
            }
        }))
```

- [ ] **Step 5 : macOS « ouvrir avec… ».** Remplacer `app.run(|_, _| {});` par :

```rust
    app.run(|app, event| {
        // macOS : Finder ne relance pas l'app, il lui envoie les fichiers
        // (au lancement aussi : l'argument n'y est pas).
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = &event {
            let files: Vec<PathBuf> = urls.iter().filter_map(|u| u.to_file_path().ok()).filter(|p| p.is_file()).collect();
            if app.try_state::<AppState>().is_some() {
                commands::open_files(app, files);
            } else {
                app.state::<Pending>().0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(files);
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
```

  Doc du module `main.rs`, ajouter : « Une seule instance : une seconde transmet ses fichiers à la première ; sous macOS, Finder les envoie (`RunEvent::Opened`). »

- [ ] **Step 6 :** `cargo clippy -p oku-slides-app --all-targets -- -D warnings` → propre.

- [ ] **Step 7 : vérification manuelle.**
  1. App ouverte, deck modifié ; lancer une seconde fois le binaire avec un `.oku-slides.pdf` (`./target/debug/oku-slides-app ~/x.oku-slides.pdf`) → pas de seconde fenêtre ; un onglet de plus dans la fenêtre active, au premier plan ; le deck modifié est intact.
  2. Relancer sans argument → la fenêtre active revient au premier plan.
  3. macOS (bundle `cargo tauri build --debug`), app lancée : `open -a "Oku Slides" ~/x.oku-slides.pdf` → onglet ouvert. Même commande app fermée → l'app démarre, reprend son état et ouvre le fichier dans un onglet.
  4. Le même fichier une deuxième fois, y compris par un chemin relatif (`cd ~ && ./…/oku-slides-app x.oku-slides.pdf`) → son onglet est activé, pas de doublon.

- [ ] **Step 8 : commit**

```bash
git add Cargo.toml Cargo.lock src-tauri
git commit -m "feat(app) : une seule instance, fichiers reçus dans un onglet (t-153)"
oku cli task done --project slider --id t-153
```

---

### Task 8 : Vérification finale (t-154)

- [ ] **Step 1 :** `just check` → propre (fmt, clippy `-D warnings`, types TypeScript).
- [ ] **Step 2 :** `just test` → vert.
- [ ] **Step 3 :** `just e2e` → vert (`toolbar`, `panel`, `tabs`). Puis `just serve` dans un terminal et `cd ui && E2E_URL=http://127.0.0.1:4870/ bun e2e/smoke.cjs` → sans erreur.
- [ ] **Step 4 : `just web`** (Vite sur 5173, qui relaie `/api` au serveur) : l'interface se charge, les onglets s'ouvrent et se ferment, aucune réponse 403 dans la console du navigateur.
- [ ] **Step 5 : consolidation (G10).**

```bash
grep -rnE 'notice-resume-kept-unsaved|session-busy|open_at_launch|titlebarOverlay|drag-strip|api\.newDeck|api\.open\(|api\.messages|import \{[^}]*\bapi\b[^}]*\} from "\./api"|cmd: *.(new|open|messages).\b' crates src-tauri ui/src ui/e2e
```

  → rien. Aucun code hors de `workspace.rs` ne verrouille une session : `grep -rn "\.lock()" src-tauri crates/oku-slides-cli` ne montre que `AppState::sync`, `Pending` et `Workspace::lock`.
- [ ] **Step 6 : spec à jour.** Relire `docs/superpowers/specs/2026-09-26-onglets-fenetres-design.md` contre le code livré ; corriger toute différence (G9).
- [ ] **Step 7 :** reprendre la vérification manuelle des tâches 6 et 7 sur un build `cargo tauri build --debug`, deux écrans, clavier AZERTY ; vérifier en particulier que ⌘W ne ferme jamais la fenêtre et que ⌘Z n'en ferme pas l'onglet.
- [ ] **Step 8 : commit et clôture**

```bash
git add -A docs
git commit -m "docs(spec) : onglets et fenêtres, spec conforme au code (t-154)"
oku cli task done --project slider --id t-154
oku cli task done --project slider --id t-142
```
