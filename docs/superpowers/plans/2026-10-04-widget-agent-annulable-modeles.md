# Écriture d'un widget annulable en mode modèles — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** En mode modèles, une écriture de widget (agent ou utilisateur) est une étape d'annulation du document des modèles ; un tour de l'agent = un seul ⌘Z.

**Architecture:** `adopt_session_version` fait passer le document des modèles à la nouvelle version par `apply(SetTemplates…)` (étape, clé de séance) au lieu d'`apply_silent`, sauf pour l'enregistrement des modèles. Après annuler/rétablir en mode modèles, `realign_templates` remet `TemplateMode::{base, session_version}` sur `deck.templates` du document, remet la version en bibliothèque et fait suivre le deck mis de côté.

**Tech Stack:** Rust, Loro (`UndoManager` via `DeckReplica`), nextest + speculoos.

**Spec:** `docs/superpowers/specs/2026-10-04-widget-agent-annulable-modeles-design.md`

## Global Constraints

- Tâche parente : t-504 ; chaque commit se termine par `(t-504)`.
- Format de reprise (`SavedMode::Templates(TemplateMode)`) inchangé.
- `save_templates` reste silencieux (pas d'étape).
- Annuler ne change pas la version courante de la bibliothèque (`set_current` jamais appelé par `realign_templates`).
- Interface : aucun changement (`useTemplateWidgets` relit sur `editing.base`).
- Commits : `git add` séparé, vérifier l'index (`git diff --cached --name-only`) juste avant `git commit` ; pas de ligne `Co-Authored-By`.
- Fichiers ≤ 500 lignes visés (`widgets_edit_tests.rs` : 376 lignes avant).

---

### Task 1: Écriture de widget en mode modèles = étape du document des modèles

**Files:**
- Modify: `crates/oku-slides-session/src/templates_mode.rs:200-265` (`adopt_session_version`, et son appel `:188` dans `save_templates`)
- Modify: `crates/oku-slides-session/src/widgets_edit.rs:380-386` (`commit_files`, branche `Mode::Templates`)
- Test: `crates/oku-slides-session/src/widgets_edit_tests.rs` (fin du fichier)

**Interfaces:**
- Produces: `Session::adopt_session_version(&mut self, deck: Deck, files: PackFiles, edit: &TemplateMode, as_new: bool, step: Option<&str>) -> Result<PackVersion>` — `step = Some(label)` : étape annulable sous `label` avec la clé `adopt_merge_key(id, parent)` ; `None` : silencieux (enregistrement).

- [ ] **Step 1: Tests RED** — le premier existe déjà (non commité) en fin de `widgets_edit_tests.rs` ; ajouter les deux suivants :

```rust
#[test]
fn set_widget_by_agent_in_templates_turn_can_be_undone() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.begin_turn("widget kpi");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().can_undo).is_true();
}

#[test]
fn two_widget_writes_in_templates_turn_are_one_step_labelled_by_the_turn() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.begin_turn("deux écritures");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.set_widget_by_agent("kpi", KPI_V2.into()).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : deux écritures"));
}

#[test]
fn two_widget_writes_in_templates_mode_outside_a_turn_merge_into_one_step() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let steps = s.doc.sources.undo_steps();
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps);
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo nextest run -p oku-slides-session widgets_edit_tests`
Expected: les trois nouveaux échouent (`can_undo` faux, `undo_label` `None`) ; le troisième peut passer par accident (0 = 0) — il garde la fusion après le changement.

- [ ] **Step 3: Implémentation** — dans `adopt_session_version`, ajouter le paramètre `step: Option<&str>`, et remplacer le passage silencieux du document des modèles :

```rust
        let r = PackRef { id, version: version.clone() };
        let merge = adopt_merge_key(id, parent.as_ref());
        // Le document des modèles passe à la nouvelle version : une étape pour
        // une écriture de widget (clé de séance, ou groupe du tour), rien pour
        // un enregistrement. Ses versions restent dans ses sources : une
        // étape peut y ramener.
        self.doc.sources.add_pack(files.clone());
        deck.templates = Some(r.clone());
        match step {
            Some(label) => self.doc.sources.apply(deck, label, Some(&merge)),
            None => self.doc.sources.apply_silent(deck),
        }
        .context(SourcesSnafu)?;
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", manifest.name.as_str().into())]);
```

(supprimer l'ancienne ligne `let merge = …` plus bas, désormais au-dessus) et, dans la branche « fusionnée », ne plus retirer la version du document des modèles :

```rust
                if stash.sources.undo_steps() == steps {
                    // Fusionnée dans l'étape qui l'avait introduite : plus
                    // atteignable depuis le deck mis de côté.
                    stash.sources.packs.remove(old);
                } else if let Some(files) = stash.sources.packs.get(old) {
```

Mettre à jour la doc de la fonction : « `deck` (le document des modèles) y passe en une étape `step` (silencieux sans), et le deck mis de côté en une étape d'annulation. »

Appels : `save_templates` (`templates_mode.rs:188`) passe `None` ; `commit_files` (`widgets_edit.rs:385`) passe le libellé des écritures de widgets :

```rust
                let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
                let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
                let deck = self.doc.sources.deck().clone();
                self.adopt_session_version(deck, files, &edit, false, Some(&label))?;
```

- [ ] **Step 4: Vérifier**

Run: `cargo nextest run -p oku-slides-session`
Expected: tout passe, dont `set_widget_twice_then_save_makes_one_version_holding_the_file` (la bibliothèque garde une version par séance).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session/src/templates_mode.rs crates/oku-slides-session/src/widgets_edit.rs crates/oku-slides-session/src/widgets_edit_tests.rs
git diff --cached --name-only
git commit -m "fix(session) : écriture d'un widget en mode modèles = étape d'annulation, un tour = une étape (t-504)"
```

### Task 2: Annuler/rétablir en mode modèles réaligne l'édition, la bibliothèque et le deck mis de côté

**Files:**
- Modify: `crates/oku-slides-session/src/templates_mode.rs` (nouvelle fonction `realign_templates`, après `adopt_session_version`)
- Modify: `crates/oku-slides-session/src/lib.rs:762-785` (`Session::undo`, `Session::redo`)
- Test: `crates/oku-slides-session/src/widgets_edit_tests.rs` (fin du fichier)

**Interfaces:**
- Consumes: `adopt_merge_key(id, parent)` (`templates_mode.rs:38`), `Library::add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool>` (sans effet si présente).
- Produces: `pub(crate) fn realign_templates(&mut self) -> Result<()>` sur `Session`.

- [ ] **Step 1: Tests RED**

```rust
/// Une écriture hors tour (V1), puis un tour de l'agent (V2) ; rend la base V1.
fn written_then_agent_turn(s: &mut Session) -> oku_slides_core::PackVersion {
    s.set_widget("kpi", KPI.into()).unwrap();
    let v1 = s.snapshot().editing.unwrap().base.clone();
    s.begin_turn("v2");
    s.set_widget_by_agent("kpi", KPI_V2.into()).unwrap();
    s.end_turn();
    v1
}

#[test]
fn undo_of_a_templates_turn_restores_the_widget_source() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    written_then_agent_turn(&mut s);
    s.undo().unwrap();
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
}

#[test]
fn undo_of_a_templates_turn_brings_the_replaced_version_back_to_the_library() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let v1 = written_then_agent_turn(&mut s);
    s.undo().unwrap();
    assert_that!(crate::test_support::library_of(&s).version(DEFAULT_PACK, &v1).is_some()).is_true();
}

#[test]
fn redo_of_a_templates_turn_restores_the_turn_source() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    written_then_agent_turn(&mut s);
    s.undo().unwrap();
    s.redo().unwrap();
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI_V2.to_owned());
}

#[test]
fn undo_to_the_parent_then_close_leaves_the_deck_on_the_parent_version() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let parent = s.snapshot().editing.unwrap().parent.clone();
    s.begin_turn("kpi");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.end_turn();
    s.undo().unwrap();
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(parent);
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo nextest run -p oku-slides-session widgets_edit_tests`
Expected: les quatre échouent (source V2 après annuler — `widget_target` lit encore `edit.base` ; version absente de la bibliothèque ; deck resté sur V1).

- [ ] **Step 3: Implémentation** — dans `templates_mode.rs`, après `adopt_session_version` :

```rust
    /// Réaligne l'édition des modèles sur la version du document, après annuler ou rétablir.
    ///
    /// La bibliothèque retrouve cette version (une écriture de la séance l'a
    /// peut-être remplacée), sans qu'elle devienne courante ; le deck mis de
    /// côté y passe, dans l'étape de la séance.
    pub(crate) fn realign_templates(&mut self) -> Result<()> {
        let Some(r) = self.doc.sources.deck().templates.clone() else { return Ok(()) };
        let Mode::Templates { stash, edit, .. } = &mut self.mode else { return Ok(()) };
        if r.id != edit.set || r.version == edit.base {
            return Ok(());
        }
        let files = self
            .doc
            .sources
            .packs
            .get(&r.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        if let Err(e) = self.library.add_version(r.id, &files) {
            tracing::error!(error = %e.one_line(), "version rétablie absente de la bibliothèque");
        }
        edit.base = r.version.clone();
        edit.session_version = (edit.parent.as_ref() != Some(&r.version)).then(|| r.version.clone());
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", edit.name.as_str().into())]);
        let merge = adopt_merge_key(edit.set, edit.parent.as_ref());
        stash.sources.add_pack(files);
        let mut user = stash.sources.deck().clone();
        user.templates = Some(r);
        stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
        stash.dirty = true;
        self.template_cache = None;
        Ok(())
    }
```

(importer `PackVersionMissingSnafu` depuis `crate::error` si absent.) Dans `lib.rs`, `undo` et `redo` l'appellent avant `sync_widgets` :

```rust
        if self.doc.sources.undo().context(SourcesSnafu)? {
            self.doc.dirty = true;
            self.realign_templates()?;
            self.sync_widgets()?;
            self.rerender()?;
        }
```

(idem dans `redo` avec `self.doc.sources.redo()`.)

- [ ] **Step 4: Vérifier**

Run: `cargo nextest run -p oku-slides-session`
Expected: tout passe.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session/src/templates_mode.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/src/widgets_edit_tests.rs
git diff --cached --name-only
git commit -m "fix(session) : annuler en mode modèles réaligne la version éditée, la bibliothèque et le deck (t-504)"
```

### Task 3: Test MCP de bout en bout et documentation

**Files:**
- Test: `crates/oku-slides-mcp/tests/widgets.rs` (fin du fichier)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md:19` (puce widgets)
- Modify: `crates/oku-slides-session/src/widgets_edit.rs:1-10` (doc du module, ligne sur `adopt_session_version`)

**Interfaces:**
- Consumes: `Session::begin_turn` / `end_turn`, outils MCP `edit_templates`, `set_widget` ; harnais `start`, `client`, `call` (`tests/common`).

- [ ] **Step 1: Test** (vert attendu après Tasks 1-2 ; il garde le chemin MCP)

```rust
#[tokio::test(flavor = "multi_thread")]
async fn set_widget_twice_in_a_templates_turn_is_one_undo_step() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10" })).await;
    h.session.lock().unwrap().begin_turn("deux widgets");
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let mut s = h.session.lock().unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : deux widgets"));
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-mcp set_widget_twice_in_a_templates_turn` — Expected: PASS.

- [ ] **Step 3: Docs** — `ARCHITECTURE.md`, puce widgets : ajouter « en mode modèles, une écriture de widget est aussi une étape du document des modèles (`adopt_session_version`, clé de séance) ; annuler/rétablir réaligne l'édition (`realign_templates`) ». Doc du module `widgets_edit.rs` : remplacer la mention de `adopt_session_version` par « `adopt_session_version` en mode modèles (une étape d'annulation du document des modèles) ».

- [ ] **Step 4: Vérification complète** — `just check` (fmt + clippy + tests). Expected: vert.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-mcp/tests/widgets.rs crates/oku-slides-session/ARCHITECTURE.md crates/oku-slides-session/src/widgets_edit.rs
git diff --cached --name-only
git commit -m "test(mcp) : deux set_widget d'un tour en mode modèles = une étape ; docs (t-504)"
```
