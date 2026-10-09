# Zone agent détachable : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** détacher le fil de l'agent, son dock, le brouillon et l'appareil de dictée dans une fenêtre `agent`. Cette fenêtre suit l'onglet actif de la fenêtre au premier plan (t-654).

**Architecture:** l'hôte reste la seule source de vérité.
- L'espace de travail porte `agent_detached`, persisté.
- Le hub diffuse la portée vue (`McpEvent::Viewing`).
- La pile tient le brouillon de chaque onglet (`set_draft`, `AgentEvent::Draft`). Elle le rejoue sur `follow` par un ordre du gardien (`Keep::Replay`), dans l'ordre de la file.
- Côté interface, `AgentPanel` (t-638) devient soit **pilote**, soit **suiveur** de la portée :
  - la fenêtre principale garde un panneau pilote sans fil (`thread=false`), avec fiches, tiroir, fenêtre et touches rapides ;
  - la fenêtre `agent` monte un panneau suiveur avec le fil.
- Un canal `agent-link` (évènement Tauri `agent-link`, ou `BroadcastChannel` hors Tauri) ne porte que ce qui n'existe que dans la fenêtre principale : où écrire, et le contexte de l'interface.

**Tech Stack:** Rust 2024 (Tauri 2.12, tokio, serde, speculoos, nextest), React 19 + TypeScript (bun test, happy-dom, zod), Playwright (e2e contre `oku-slides serve`), Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-10-09-agent-detachable-design.md`. Sa section « Précisions du plan » remplace ce qu'elle contredit plus haut.

**Revue :** ce plan intègre la revue d'équipe du 2026-10-09 (14 relecteurs) et le rebasage sur `cdc4a96f` (t-638 : `AgentPanel`, `useDictation`, `hub.rs` découpé).

## Global Constraints

**Fenêtres**
- Label de la fenêtre : `agent` (`agent_window::LABEL` côté Rust, `AGENT_LABEL` côté TS).
- Les fenêtres de l'espace de travail se reconnaissent par `oku_slides_session::is_window_label` (`w-<n>`). C'est la seule définition.
- Fenêtre agent : 420×800 par défaut, 320×400 au minimum (points logiques).
- La fermeture système de la fenêtre agent passe par la page (`close-requested`, comme les fenêtres de l'espace de travail). Celle-ci vide la frappe en attente, puis appelle `attach_agent`.

**Brouillon**
- La frappe est envoyée en debounce de 300 ms. La dictée, Envoyer, Insérer, Abandonner et l'ouverture partent tout de suite.
- Les envois sont chaînés (une promesse à la fois), et chacun porte l'`origin` de sa vue : une vue ignore l'écho de ses propres envois.
- Taille maximale : 256 Kio (octets UTF-8). Au-delà, `set_draft` est refusé avec une erreur.
- `follow` et `set_draft` ne démarrent jamais un onglet. Sans agent démarré, `follow` rejoue « rien » et `set_draft` est refusé (erreur localisée), pour que la vue qui écrit ne l'attende pas.
- `origin` (identifiant de vue) : 64 octets au plus ; au-delà, `set_draft` est refusé.

**Canal entre fenêtres**
- `insert` sans `insert-ack` sous 3000 ms : `insert-cancel` est envoyé et le texte revient au brouillon. La fenêtre principale ignore toute insertion annulée.
- Chaque message du canal est validé par un schéma zod à la réception. Un message invalide est écarté sans être journalisé (il peut contenir du texte dicté).

**Hors Tauri**
- Ni bouton ⧉ ni menu. La fenêtre agent ne s'ouvre qu'à la main, par `/?window=agent` (e2e).

**Menu**
- Le menu « Fenêtre » n'existe que sous macOS, seul système où l'app a un menu.
- Sa case reflète l'espace de travail (`set_agent_checked` à chaque `sync`). Un clic bascule l'état lu dans l'espace de travail, pas l'état de la case.

**Code**
- Pas de `.unwrap()` ni `.expect()` hors tests. Les erreurs sont journalisées en `tracing::warn!`, comme le code voisin.
- Tout texte visible passe par Fluent. Chaque nouvelle clé est ajoutée aux 20 langues (`crates/oku-slides-i18n/src/locales/*/`), traduite.
- Fichiers ≤ 1000 lignes (CI). `App.tsx` en compte 815 aujourd'hui.
- Aucun utilitaire recopié. Les minuteries réelles viennent de `debounce.ts` (`REAL_TIMERS`). Les doubles de test partagés vivent dans `ui/src/test/` (`timers.ts`, `link.ts`) et dans `crates/oku-slides-agent/tests/common/mod.rs` (`wait_n_after`, `relayed_len`).

**Commits et tests**
- Commits : Conventional Commits en français, avec `(t-654)` à la fin du titre, **jamais** de ligne `Co-Authored-By`.
- Commiter en deux commandes : `git add` dans l'une, puis `git commit -F <fichier>` dans une autre (hook de revue).
- Nommage des tests : Rust `unite_scenario_resultat` ; TS `"<unité> <scénario> <résultat>"`. Un comportement par test.
- Vérifications côté Rust : `just check` (fmt, clippy, nextest).
- Vérifications côté interface : `cd ui && bun run typecheck && bun run test`.
- e2e : `just e2e <port libre>`. La recette isole les données (`OKU_SLIDES_PATHS__DATA` et `__DECKS` dans un dossier temporaire).
- Paquet Tauri : `oku-slides-app`.

---

### Task 1: Espace de travail — état détaché persisté

**Files:**
- Modify: `crates/oku-slides-session/src/workspace.rs` : `is_window_label` devient public ; modifier `Layout`, `WorkspaceView`, `WorkspaceRequest`, `view()`, et ajouter le module de test.
- Modify: `crates/oku-slides-session/src/workspace_windows.rs` : ajouter `set_agent_detached` et `agent_detached`.
- Modify: `crates/oku-slides-session/src/workspace_tabs.rs:232-257` : `change`.
- Modify: `crates/oku-slides-session/src/workspace_resume.rs` : la reprise, juste après `let saved = file.layout;`.
- Modify: `crates/oku-slides-session/src/lib.rs:114` : réexporter `is_window_label`.
- Modify: `crates/oku-slides-session/src/test_support.rs` : y déplacer les utilitaires de `workspace_tests.rs` (`workspace_platform`, `workspace_on`, `apply_ws`) et ajouter `resumed_on`.
- Modify: `crates/oku-slides-session/src/workspace_tests.rs` : utiliser ces utilitaires partagés au lieu de ses copies locales (`platform`, `workspace_in`, `send`).
- Modify: `crates/oku-slides-session/ARCHITECTURE.md` : une ligne.
- Modify: `src-tauri/src/commands.rs:42-62` : `classify`.
- Modify: `src-tauri/src/windows.rs` : le `WorkspaceView` de repli de `sync`.
- Create: `crates/oku-slides-session/src/workspace_detach_tests.rs`.

**Interfaces:**
- Produces:
  - `WorkspaceView.agent_detached: bool` (JSON `agent_detached`).
  - `WorkspaceRequest::DetachAgent` / `AttachAgent` (`{"cmd":"detach_agent"}` / `{"cmd":"attach_agent"}`), qui rendent la vue.
  - `Workspace::agent_detached(&self) -> bool`.
  - `pub fn oku_slides_session::is_window_label(label: &str) -> bool`.

- [ ] **Step 1: Shared test helpers** (déplacés de `workspace_tests.rs` vers `test_support.rs`)

Aujourd'hui, `workspace_tests.rs:25-40` définit `platform`, `workspace_in` et `send`. Les déplacer, à l'identique, en fin de `test_support.rs`, dans le style du fichier (`unwrap_or_else(|e| panic!(…))`) :

```rust
/// Les réglages d'un espace de travail de test : données et decks sous `data`, interface en français.
pub(crate) fn workspace_platform(data: &std::path::Path) -> crate::Platform {
    let mut settings = crate::Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    crate::Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

/// Un espace de travail neuf sur `data`, sans reprise.
pub(crate) fn workspace_on(data: &std::path::Path) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::new(crate::Fonts::default(), &workspace_platform(data), None);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("espace de travail : {e}")))
}

/// L'espace de travail repris de `dir` (neuf s'il n'y a rien), sur `data`.
pub(crate) fn resumed_on(data: &std::path::Path, dir: std::path::PathBuf) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::resume(crate::Fonts::default(), &workspace_platform(data), dir);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("reprise : {e}")))
}

/// Applique une requête JSON de l'espace de travail.
pub(crate) fn apply_ws(
    ws: &std::sync::Mutex<crate::Workspace>,
    v: serde_json::Value,
) -> crate::Result<crate::WorkspaceOutcome> {
    crate::Workspace::apply(ws, serde_json::from_value(v).unwrap_or_else(|e| panic!("requête : {e}")))
}
```

Dans `workspace_tests.rs` :
- supprimer `platform`, `workspace_in` et `send` ;
- importer `use crate::test_support::{apply_ws, resumed_on, workspace_on, workspace_platform};` ;
- remplacer partout, par recherche et remplacement :
  - `platform(&data.0)` par `workspace_platform(&data.0)` ;
  - `workspace_in(&data)` par `workspace_on(&data.0)` ;
  - les appels `send(` par `apply_ws(`.

Les imports du fichier se réduisent à `use std::path::PathBuf;` et `use crate::{Request, SessionErrorKind};` : `Path` et `Settings` ne servaient qu'à `platform` (sinon `unused_imports`, et clippy échoue). Les autres utilitaires du fichier (`Data`, `view`, `ids`, `set_theme`, `saved_pdf`) restent. Lancer `cargo nextest run -p oku-slides-session workspace_tests` : PASS, comme avant.

`workspace_help_tests.rs` garde son `workspace(data, locale)`, qui choisit la langue : ce n'est pas une copie.
- [ ] **Step 2: Write the failing tests** (`workspace_detach_tests.rs`)

```rust
//! L'agent détaché dans sa fenêtre : un état de l'espace de travail, repris au lancement.

use serde_json::json;
use speculoos::prelude::*;

use crate::test_support::{apply_ws, fresh_dir, resumed_on, workspace_on};
use crate::{Workspace, is_window_label};

#[test]
fn new_workspace_keeps_the_agent_attached() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    assert_that!(Workspace::lock(&ws).view().agent_detached).is_false();
}

/// Applique une requête qui doit réussir.
fn apply(ws: &std::sync::Mutex<Workspace>, v: serde_json::Value) {
    apply_ws(ws, v).unwrap_or_else(|e| panic!("requête refusée : {e}"));
}

#[test]
fn detach_agent_sets_the_view_flag() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    apply(&ws, json!({"cmd": "detach_agent"}));
    assert_that!(Workspace::lock(&ws).view().agent_detached).is_true();
}

#[test]
fn attach_agent_clears_the_view_flag() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    apply(&ws, json!({"cmd": "detach_agent"}));
    apply(&ws, json!({"cmd": "attach_agent"}));
    assert_that!(Workspace::lock(&ws).agent_detached()).is_false();
}

#[test]
fn detached_agent_is_restored_by_resume() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let dir = data.path().join("resume");
    let ws = resumed_on(data.path(), dir.clone());
    apply(&ws, json!({"cmd": "detach_agent"}));
    Workspace::settle(&ws);
    drop(ws);
    let again = resumed_on(data.path(), dir);
    assert_that!(Workspace::lock(&again).agent_detached()).is_true();
}

#[test]
fn focus_of_the_agent_window_is_refused_and_keeps_the_focused_window() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    let mut w = Workspace::lock(&ws);
    let before = w.focused().to_owned();
    assert_that!(w.focus("agent")).is_err();
    assert_that!(w.focused()).is_equal_to(before.as_str());
}

#[test]
fn is_only_window_for_the_agent_window_is_false() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    assert_that!(Workspace::lock(&ws).is_only_window("agent")).is_false();
}

#[test]
fn is_window_label_numbered_w_label_is_true() {
    assert_that!(is_window_label("w-3")).is_true();
}

#[test]
fn is_window_label_agent_label_is_false() {
    assert_that!(is_window_label("agent")).is_false();
}
```

En bas de `workspace.rs`, à côté de `workspace_tests` :

```rust
#[cfg(test)]
#[path = "workspace_detach_tests.rs"]
mod workspace_detach_tests;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo nextest run -p oku-slides-session workspace_detach`
Expected: FAIL à la compilation (`no field agent_detached`, `is_window_label` privée).

- [ ] **Step 4: Implement**

`workspace.rs` : `fn is_window_label` devient `pub fn is_window_label`, avec la doc « Un label de fenêtre de l'espace de travail : `w-<n>`. ». `lib.rs:114` ajoute `is_window_label` à la liste `pub use workspace::{…}`.

`Layout` (après `kept`) :

```rust
    /// L'agent est dans sa propre fenêtre (`agent`), pas dans la colonne de droite.
    #[serde(default)]
    agent_detached: bool,
```

`Layout::new()` : `agent_detached: false,`. `WorkspaceView` (après `windows`) :

```rust
    /// L'agent est dans sa propre fenêtre : la colonne de droite ne le montre plus.
    pub agent_detached: bool,
```

`view()` : `agent_detached: self.layout.agent_detached,`. `WorkspaceRequest` (après `CloseWindow`) :

```rust
    /// L'agent passe dans sa propre fenêtre.
    DetachAgent,
    /// L'agent revient dans la colonne de droite.
    AttachAgent,
```

`workspace_windows.rs`, dans `impl Workspace` :

```rust
    /// L'agent est-il dans sa propre fenêtre ?
    pub fn agent_detached(&self) -> bool {
        self.layout.agent_detached
    }

    /// Détache (`true`) ou rattache l'agent ; écrit par `apply`, comme tout changement.
    pub(super) fn set_agent_detached(&mut self, detached: bool) {
        self.layout.agent_detached = detached;
    }
```

`workspace_tabs.rs::change`, avant `WorkspaceRequest::View` :

```rust
            WorkspaceRequest::DetachAgent => self.set_agent_detached(true),
            WorkspaceRequest::AttachAgent => self.set_agent_detached(false),
```

`workspace_resume.rs`, juste après `let saved = file.layout;` :

```rust
        self.layout.agent_detached = saved.agent_detached;
```

`src-tauri/src/commands.rs::classify`, dans le groupe `(true, false)` :

```rust
        | WorkspaceRequest::DetachAgent
        | WorkspaceRequest::AttachAgent
```

`src-tauri/src/windows.rs::sync` : le `WorkspaceView { generation: 0, … }` de repli prend `agent_detached: false`.

`crates/oku-slides-session/ARCHITECTURE.md`, ligne de `workspace` : ajouter « agent détaché (`agent_detached`, persisté) ».

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-session workspace && cargo check -p oku-slides-app`
Expected: PASS ; l'hôte compile.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-session/src/workspace.rs crates/oku-slides-session/src/workspace_windows.rs crates/oku-slides-session/src/workspace_tabs.rs crates/oku-slides-session/src/workspace_resume.rs crates/oku-slides-session/src/lib.rs crates/oku-slides-session/src/test_support.rs crates/oku-slides-session/src/workspace_tests.rs crates/oku-slides-session/src/workspace_detach_tests.rs crates/oku-slides-session/ARCHITECTURE.md src-tauri/src/commands.rs src-tauri/src/windows.rs
```
Message : `feat(session) : l'agent détaché, un état de l'espace de travail repris au lancement (t-654)`

---

### Task 2: Hub — la portée vue est diffusée

**Files:**
- Modify: `crates/oku-slides-mcp/src/hub.rs` : `McpEvent` (vers la ligne 31), `record_and_send` (ligne 191), `resync` (ligne 204).
- Modify: `crates/oku-slides-mcp/src/hub_scopes.rs:57` : `rename_scope`.
- Test: `crates/oku-slides-mcp/src/hub_scopes_tests.rs`.
- Modify: `crates/oku-slides-mcp/ARCHITECTURE.md` : une ligne.
- Modify: `ui/src/api.ts` : type `McpEvent`.

**Interfaces:**
- Produces:
  - `McpEvent::Viewing { scope: AgentScope }`, en JSON `{"kind":"viewing","scope":{…}}`.
  - Il est émis par `resync` **avant** la liste, et par `rename_scope` quand la portée vue est renommée.
  - `SurfaceHub::viewing(&self) -> AgentScope`.

- [ ] **Step 1: Write the failing tests** (fin de `hub_scopes_tests.rs` ; ajouter l'import `use oku_slides_session::pack::DEFAULT_PACK;`, absent aujourd'hui)

```rust
fn viewing_events(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<AgentScope> {
    let mut found = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::Viewing { scope } = e {
            found.push(scope);
        }
    }
    found
}

#[test]
fn resync_announces_the_viewed_scope_before_the_list() {
    let hub = hub();
    let mut rx = hub.subscribe();
    hub.resync(atelier());
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Viewing { scope }) if scope == atelier())).is_true();
    assert_that!(matches!(rx.try_recv(), Ok(McpEvent::Surfaces { .. }))).is_true();
}

#[test]
fn viewing_before_any_resync_is_slides() {
    assert_that!(hub().viewing()).is_equal_to(AgentScope::Slides);
}

#[test]
fn viewing_after_resync_is_the_scope_shown() {
    let hub = hub();
    hub.resync(atelier());
    assert_that!(hub.viewing()).is_equal_to(atelier());
}

#[test]
fn rename_of_the_viewed_scope_announces_the_new_one() {
    let hub = hub();
    let from = AgentScope::Widget { pack: DEFAULT_PACK, name: "kpi".into() };
    let to = AgentScope::Widget { pack: DEFAULT_PACK, name: "score".into() };
    hub.resync(from.clone());
    let mut rx = hub.subscribe();
    hub.rename_scope(&from, &to);
    assert_that!(viewing_events(&mut rx)).is_equal_to(vec![to]);
}

#[test]
fn rename_of_another_scope_announces_nothing() {
    let hub = hub();
    hub.resync(AgentScope::Slides);
    let mut rx = hub.subscribe();
    hub.rename_scope(&atelier(), &AgentScope::Atelier { pack: uuid::Uuid::nil() });
    assert_that!(viewing_events(&mut rx)).is_empty();
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p oku-slides-mcp hub_scopes`
Expected: FAIL à la compilation (`no variant Viewing`, `no method viewing`).

- [ ] **Step 3: Implement**

`hub.rs`, `McpEvent` (après `WidgetTouched`) :

```rust
    /// La portée dont le fil est montré.
    ///
    /// Une autre vue du même onglet (la fenêtre de l'agent) la suit.
    Viewing {
        /// La portée montrée.
        scope: AgentScope,
    },
```

`record_and_send`, ligne 191 : le bras devient

```rust
            McpEvent::Changed | McpEvent::Focus { .. } | McpEvent::WidgetTouched { .. } | McpEvent::Viewing { .. } => true,
```

`resync`, remplacé par :

```rust
    /// Montre le fil de `scope`, piloté par le gardien de l'onglet.
    ///
    /// Annonce la portée (`McpEvent::Viewing`), puis envoie la liste entière (`McpEvent::Surfaces`).

    pub fn resync(&self, scope: AgentScope) {
        let mut state = self.state();
        state.viewing = scope.clone();
        self.record_and_send(&mut state, McpEvent::Viewing { scope });
        self.record_and_send(&mut state, McpEvent::Surfaces { list: Vec::new() });
    }

    /// La portée dont le fil est montré.
    pub fn viewing(&self) -> AgentScope {
        self.state().viewing.clone()
    }
```

`hub_scopes.rs::rename_scope` est un module enfant de `hub` (`#[path] mod scopes`), donc il voit `record_and_send`. Le bloc `if state.viewing == *from { … }` devient :

```rust
        if state.viewing == *from {
            state.viewing = to.clone();
            self.record_and_send(&mut state, McpEvent::Viewing { scope: to.clone() });
        }
```

`ui/src/api.ts`, type `McpEvent` : ajouter `| { kind: "viewing"; scope: AgentScope }`.

`crates/oku-slides-mcp/ARCHITECTURE.md`, ligne du hub : « `resync` annonce la portée vue (`Viewing`) puis la liste ».

- [ ] **Step 4: Run to verify pass**

Run: `cargo nextest run -p oku-slides-mcp && cargo nextest run -p oku-slides-agent && (cd ui && bun run typecheck)`
Expected: PASS. Les tests de la pile filtrent les évènements `mcp` par `kind`, et le `surfacesReducer` du TS a un `default`.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-mcp/src/hub.rs crates/oku-slides-mcp/src/hub_scopes.rs crates/oku-slides-mcp/src/hub_scopes_tests.rs crates/oku-slides-mcp/ARCHITECTURE.md ui/src/api.ts
```
Message : `feat(mcp) : le hub annonce la portée dont il montre le fil (t-654)`

---

### Task 3: Pile — `follow` et brouillon par onglet

**Files:**
- Modify: `crates/oku-slides-agent/src/events.rs` : `AgentEvent::Draft`.
- Modify: `crates/oku-slides-agent/src/turns.rs:147-166, :260` : les deux `match` exhaustifs.
- Test: `crates/oku-slides-agent/src/turns_tests.rs`.
- Modify: `crates/oku-slides-agent/src/keeper.rs` : `Keep::Replay`, son application, et `Keep::failure` (match exhaustif).
- Create: `crates/oku-slides-agent/src/command_error.rs` : `CommandError`, localisée.
- Modify: `crates/oku-slides-agent/src/lib.rs` : modules et réexports (`CommandError`, `MAX_DRAFT_BYTES`, `MAX_ORIGIN_BYTES`).
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 langues) : deux clés.
- Modify: `crates/oku-slides-agent/src/tab_entry.rs` : `TabLink` (brouillon, `replay`), et le relais `relay_mcp` (branche `Lagged`).
- Modify: `crates/oku-slides-agent/src/stack.rs` : `AgentCommand`, `tab()`, `handle`, `replay_closed`, `MAX_DRAFT_BYTES`.
- Modify: `crates/oku-slides-agent/tests/common/mod.rs` : `wait_n_after` et `relayed_len`.
- Test: `crates/oku-slides-agent/tests/stack.rs`.
- Modify: `crates/oku-slides-agent/ARCHITECTURE.md` : une ligne.
- Modify: `ui/src/api.ts` : `AgentEvent`, `AgentCommand`, `agentApi`.
- Modify: `ui/src/agent/useAgentScope.ts` : export `agentStatusReducer`, qui ne lit que `status`.
- Test: `ui/src/agent/useAgentScope.render.test.tsx`.
- Modify: `ui/src/agent/AgentPanel.render.test.tsx:168` : l'évènement d'état y est envoyé sans `kind`, caché par `as never` ; le nouveau réducteur l'ignorerait.

**Interfaces:**
- Consumes: `SurfaceHub::viewing()` (tâche 2).
- Produces (Rust) :
  - `AgentCommand::Follow { tab }`.
  - `AgentCommand::SetDraft { tab, text: Option<String>, origin: Option<String> }`.
  - `AgentEvent::Draft { text: Option<String>, origin: Option<String> }`, sur le canal `agent`.
  - `Keep::Replay`.
  - `pub const MAX_DRAFT_BYTES: usize = 256 * 1024` et `pub const MAX_ORIGIN_BYTES: usize = 64`.
  - `CommandError` (`impl Localize`) : brouillon ou origine trop longs, onglet sans agent démarré. `handle` rend son texte localisé, comme pour une action de surface refusée.
- Produces (TS) :
  - `AgentEvent = { kind: "status"; … } | { kind: "draft"; text: string | null; origin: string | null }`.
  - `agentApi(tab).follow()` et `agentApi(tab).setDraft(text: string | null, origin: string)`.
  - `export function agentStatusReducer(cur: AgentStatus, e: AgentEvent): AgentStatus`.
- Sémantique :
  - `follow` rejoue l'état et le brouillon, puis envoie `Keep::Replay`. Le gardien appelle alors `hub.resync(hub.viewing())`, à son tour dans la file : un `resync` envoyé juste avant passe d'abord.
  - Si l'onglet n'a pas d'agent démarré, `follow` envoie `replay_closed` : `idle`, `Draft(None)`, `Viewing(Slides)`, et une liste vide.
  - `resync` rejoue aussi le brouillon.
  - La branche `Lagged` du relais renvoie `Viewing` avant la liste.

- [ ] **Step 1: Shared test helpers** (`tests/common/mod.rs`, après `wait_for` ; le module a déjà `#![allow(dead_code, …)]`)

Les évènements passent par un canal asynchrone avant d'arriver dans `Relayed` : on ne prend jamais de mesure (« ce qui est arrivé jusqu'ici ») juste après un `handle`. On attend toujours un évènement précis, puis on repart de **sa position**.

```rust
/// Le nombre de relais reçus jusqu'ici.
pub fn relayed_len(rx: &Relayed) -> usize {
    rx.lock().unwrap_or_else(|e| panic!("relais : {e}")).len()
}

/// Attend `n` relais vérifiant `pred` parmi ceux reçus après les `from` premiers ; rend leurs positions et valeurs.
pub async fn wait_n_after(
    rx: &Relayed,
    from: usize,
    n: usize,
    pred: impl Fn(&(&'static str, Value)) -> bool,
) -> Vec<(usize, (&'static str, Value))> {
    for _ in 0..500 {
        let found: Vec<_> = rx
            .lock()
            .unwrap_or_else(|e| panic!("relais : {e}"))
            .iter()
            .enumerate()
            .skip(from)
            .filter(|(_, r)| pred(r))
            .map(|(i, r)| (i, r.clone()))
            .collect();
        if found.len() >= n {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{n} relais attendus après les {from} premiers, non reçus");
}
```

- [ ] **Step 2: Write the failing Rust tests** (fin de `tests/stack.rs` ; importer `relayed_len`, `wait_n_after`, `TestStack`, `Relayed`)

```rust
fn atelier_json() -> serde_json::Value {
    json!({ "kind": "atelier", "pack": DEFAULT_PACK })
}

fn is(ch: &'static str, kind: &'static str, tab: &'static str) -> impl Fn(&(&'static str, serde_json::Value)) -> bool {
    move |(c, v)| *c == ch && v["event"]["kind"] == kind && v["tab"] == tab
}

/// Le premier relais vérifiant `pred` après `from` : sa position et sa valeur.
async fn next(rx: &Relayed, from: usize, pred: impl Fn(&(&'static str, serde_json::Value)) -> bool) -> (usize, serde_json::Value) {
    let (i, (_, v)) = wait_n_after(rx, from, 1, pred).await.remove(0);
    (i, v)
}

/// Un `resync` de `tab` dans `scope` (ou les slides) : rend la position juste après sa liste.
///
/// La relecture des fils au démarrage de l'onglet envoie aussi une liste, sans `viewing` avant :
/// on attend donc la portée annoncée, puis la liste qui la suit.
async fn shown(stack: &TestStack, rx: &Relayed, tab: &'static str, scope: Option<serde_json::Value>) -> usize {
    let from = relayed_len(rx);
    let mut req = json!({ "cmd": "resync", "tab": tab });
    if let Some(s) = scope {
        req["scope"] = s;
    }
    stack.handle(cmd(req)).unwrap();
    let (viewing, _) = next(rx, from, is("mcp", "viewing", tab)).await;
    let (list, _) = next(rx, viewing, is("mcp", "surfaces", tab)).await;
    list + 1
}

/// Écrit le brouillon de `tab` depuis la vue `v-1`, et rend la position juste après son écho.
async fn drafted(stack: &TestStack, rx: &Relayed, tab: &'static str, text: Option<&str>, from: usize) -> usize {
    stack.handle(cmd(json!({ "cmd": "set_draft", "tab": tab, "text": text, "origin": "v-1" }))).unwrap();
    let (echo, _) = next(rx, from, move |(c, v)| *c == "agent" && v["event"]["kind"] == "draft" && v["tab"] == tab && v["event"]["origin"] == "v-1" && v["event"]["text"] == json!(text)).await;
    echo + 1
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_is_broadcast_with_its_tab_and_origin() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "à dire", "origin": "v-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "à dire", "origin": "v-1" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_clears_the_replayed_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_longer_than_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
    let long = "a".repeat(oku_slides_agent::MAX_DRAFT_BYTES + 1);
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": long, "origin": "v-1" })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_with_an_origin_over_the_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
    let origin = "v".repeat(oku_slides_agent::MAX_ORIGIN_BYTES + 1);
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "x", "origin": origin })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_on_a_tab_without_agent_is_refused() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let refused = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "perdu", "origin": "v-1" })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_on_a_tab_without_agent_starts_nothing() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let _ = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "perdu", "origin": "v-1" })));
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_replays_the_draft_without_origin() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("gardé"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "gardé", "origin": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_sends_the_state_before_the_viewed_scope_and_its_list() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", Some(atelier_json())).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (status, _) = next(&rx, from, is("agent", "status", "t-1")).await;
    let (viewing, scope) = next(&rx, from, is("mcp", "viewing", "t-1")).await;
    let (list, _) = next(&rx, viewing, is("mcp", "surfaces", "t-1")).await;
    assert_that!(status).is_less_than(viewing);
    assert_that!(viewing).is_less_than(list);
    assert_that!(scope["event"]["scope"]["kind"].clone()).is_equal_to(json!("atelier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_sent_right_after_a_resync_keeps_the_resync_scope() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1", "scope": atelier_json() }))).unwrap();
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    // Celle du resync, puis celle du follow : toutes deux l'atelier, jamais les slides d'avant.
    let viewings = wait_n_after(&rx, from, 2, is("mcp", "viewing", "t-1")).await;
    let kinds: Vec<_> = viewings.iter().map(|(_, (_, v))| v["event"]["scope"]["kind"].clone()).collect();
    assert_that!(kinds).is_equal_to(vec![json!("atelier"), json!("atelier")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_on_a_tab_without_agent_announces_the_slides() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, viewing) = next(&rx, from, is("mcp", "viewing", "t-1")).await;
    assert_that!(viewing["event"]["scope"].clone()).is_equal_to(json!({ "kind": "slides" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn follow_on_a_tab_without_agent_sends_an_empty_list() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, list) = next(&rx, from, is("mcp", "surfaces", "t-1")).await;
    assert_that!(list["event"]["list"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_replays_the_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("encore là"), from).await;
    stack.handle(cmd(json!({ "cmd": "resync", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"].clone()).is_equal_to(json!({ "kind": "draft", "text": "encore là", "origin": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn draft_of_one_tab_is_not_replayed_for_another() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1", "t-2"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = shown(&stack, &rx, "t-2", None).await.max(from);
    let from = drafted(&stack, &rx, "t-1", Some("à t-1"), from).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-2" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-2")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(null));
}
```

Dans `turns_tests.rs` :

```rust
#[test]
fn apply_draft_event_emits_nothing() {
    let mut turns = Turns::default();
    let out = turns.apply(&AgentEvent::Draft { text: Some("x".into()), origin: None }, Instant::now(), &fr());
    assert_that!(out).is_empty();
}
```
- [ ] **Step 3: Run to verify failure**

Run: `cargo nextest run -p oku-slides-agent`
Expected: FAIL. `cmd()` panique sur `unknown variant set_draft`, et `Draft` n'existe pas.

- [ ] **Step 4: Implement (Rust)**

`events.rs`, `AgentEvent` (dernière variante) :

```rust
    /// Le brouillon de l'onglet (`None` : fermé), tel que l'hôte le tient pour toutes ses vues.
    Draft {
        text: Option<String>,
        /// La vue qui l'a écrit ; absente quand l'hôte le rejoue. Une vue ignore ses propres échos.
        origin: Option<String>,
    },
```

`turns.rs:159` : `AgentEvent::Status { .. } | AgentEvent::Draft { .. } => return Vec::new(),`. `turns.rs:260` : ajouter `| AgentEvent::Draft { .. }` au bras `=> return out`.

`keeper.rs`, `Keep` (après `Show`) :

```rust
    /// Remontrer le fil de la portée que montre le hub.
    ///
    /// La portée est lue à l'application de l'ordre : une autre vue de l'onglet s'abonne
    /// (`follow`) sans jamais revenir sur un `Show` plus récent.
    Replay,
```

`Keep::failure` (`keeper.rs:50-57`, `match` exhaustif) : `Self::Show(_) | Self::Replay | Self::Sync | Self::Flush(_) => "ordre du gardien non appliqué",`.

et dans `apply`, à côté de `Keep::Show(scope) => …` :

```rust
        Keep::Replay => {
            hub.resync(hub.viewing());
            Ok(())
        }
```

`tab_entry.rs`, `TabLink` : champ `draft: Arc<Mutex<Option<String>>>`, initialisé dans `TabEntry::start` (`Arc::new(Mutex::new(None))`), puis :

```rust
    /// Garde le brouillon et le diffuse à toutes les vues de l'onglet.
    ///
    /// Diffusé sous le même verrou : l'ordre des diffusions est celui des écritures.
    pub fn set_draft(&self, out: &mpsc::UnboundedSender<Out>, text: Option<String>, origin: Option<String>) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        draft.clone_from(&text);
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin });
    }

    /// Envoie l'état de l'agent et le brouillon ; les surfaces des tours sont rejouées par le hub.
    pub fn replay(&self, out: &mpsc::UnboundedSender<Out>) {
        let (state, scope) = self.status.lock().unwrap_or_else(PoisonError::into_inner).clone();
        emit(out, "agent", &self.tab, &AgentEvent::Status { state, scope });
        // Sous le verrou, comme `set_draft` : jamais un brouillon périmé diffusé après un plus récent.
        let draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text: draft.clone(), origin: None });
    }
```

`tab_entry.rs::relay_mcp`, branche `Err(RecvError::Lagged(_))`, juste avant `let list = hub.visible_surfaces();` :

```rust
                emit(&out, "mcp", &tab, &McpEvent::Viewing { scope: hub.viewing() });
```

`stack.rs` :

```rust
/// Taille maximale d'un brouillon, en octets UTF-8.
pub const MAX_DRAFT_BYTES: usize = 256 * 1024;
/// Taille maximale de l'origine d'un brouillon (identifiant de vue), en octets.
pub const MAX_ORIGIN_BYTES: usize = 64;
```

`command_error.rs` :

```rust
//! Commandes de l'interface refusées par la pile, dites dans la langue de l'utilisateur.
//!
//! Le message localisé ne cite aucune donnée de l'utilisateur ; l'erreur complète (tailles)
//! reste dans `Display`, pour les journaux.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Une commande de l'interface refusée.
#[derive(Debug, Snafu)]
pub struct CommandError(CommandInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum CommandInner {
    #[snafu(display("draft of {bytes} bytes over the {max} bytes limit"))]
    DraftTooLong { bytes: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("draft origin of {bytes} bytes over the {max} bytes limit"))]
    OriginTooLong { bytes: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("draft for a tab whose agent has not started"))]
    NoAgent { backtrace: Backtrace },
}

impl Localize for CommandError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            CommandInner::DraftTooLong { .. } => l.tr("agent-error-draft-too-long"),
            CommandInner::OriginTooLong { .. } => l.tr("agent-error-draft-origin"),
            CommandInner::NoAgent { .. } => l.tr("agent-error-draft-no-agent"),
        }
    }
}

#[cfg(test)]
#[path = "command_error_tests.rs"]
mod command_error_tests;
```

`command_error_tests.rs` :

```rust
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{CommandError, DraftTooLongSnafu};

#[test]
fn draft_too_long_is_said_in_the_user_language() {
    let e = CommandError::from(DraftTooLongSnafu { bytes: 10usize, max: 5usize }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to("Le brouillon est trop long.".to_owned());
}
```

Fluent, `agent.ftl` des 20 langues (fr / en ; les autres traduites) :

```
agent-error-draft-too-long = Le brouillon est trop long.
agent-error-draft-origin = Brouillon refusé : vue inconnue.
agent-error-draft-no-agent = L'agent de cet onglet démarre : brouillon non gardé.
```
```
agent-error-draft-too-long = The draft is too long.
agent-error-draft-origin = Draft refused: unknown view.
agent-error-draft-no-agent = This tab's agent is starting: draft not kept.
```

(réexporté par `lib.rs` à côté d'`AgentCommand`). Dans `AgentCommand` :

```rust
    /// Une autre vue de l'onglet (fenêtre de l'agent) s'abonne.
    ///
    /// Rejoue l'état, le brouillon, puis le fil de la portée montrée ; ne démarre jamais l'onglet.
    Follow {
        tab: String,
    },
    /// Le brouillon de l'onglet ; `None` : fermé. Refusé si l'onglet n'a pas d'agent démarré.
    SetDraft {
        tab: String,
        text: Option<String>,
        /// La vue qui l'écrit, rendue avec l'écho.
        origin: Option<String>,
    },
```

`lib.rs` : `mod command_error;` et `pub use command_error::CommandError;`, à côté des autres modules. `stack.rs` importe `use crate::command_error::{CommandError, DraftTooLongSnafu, NoAgentSnafu, OriginTooLongSnafu};` et `use oku_slides_i18n::Localize;` (s'il ne l'a pas déjà pour `HubError`). `MAX_DRAFT_BYTES` et `MAX_ORIGIN_BYTES` sont définis dans `stack.rs` même.

`tab()` : `| Self::Follow { tab } | Self::SetDraft { tab, .. }`. Dans `handle`, nouveaux bras ; la branche `None` de `Resync` devient `None => self.replay_closed(&tab),` :

```rust
            AgentCommand::Follow { tab } => match self.tab(&tab) {
                Some(t) => {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Replay);
                }
                None => self.replay_closed(&tab),
            },
            AgentCommand::SetDraft { tab, text, origin } => {
                let checked = match (text.as_ref().map(String::len), origin.as_ref().map(String::len)) {
                    (Some(bytes), _) if bytes > MAX_DRAFT_BYTES => {
                        Err(DraftTooLongSnafu { bytes, max: MAX_DRAFT_BYTES }.build())
                    }
                    (_, Some(bytes)) if bytes > MAX_ORIGIN_BYTES => {
                        Err(OriginTooLongSnafu { bytes, max: MAX_ORIGIN_BYTES }.build())
                    }
                    _ => Ok(()),
                };
                if let Err(inner) = checked {
                    let e = CommandError::from(inner);
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
                    return Err(e.localize(&self.platform.localizer));
                }
                match self.tab(&tab) {
                    Some(t) => t.set_draft(&self.out_tx, text, origin),
                    // Refusé, pas ignoré : la vue qui écrit attend un écho ou une erreur (tâche 6).
                    None => {
                        let e = CommandError::from(NoAgentSnafu.build());
                        tracing::debug!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
                        return Err(e.localize(&self.platform.localizer));
                    }
                }
            }
```

et, dans `impl Stack` :

```rust
    /// Ce que voit une vue d'un onglet sans agent démarré.
    ///
    /// Repos, aucun brouillon, les slides, aucune surface.
    fn replay_closed(&self, tab: &str) {
        emit(&self.out_tx, "agent", tab, &AgentEvent::idle());
        emit(&self.out_tx, "agent", tab, &AgentEvent::Draft { text: None, origin: None });
        emit(&self.out_tx, "mcp", tab, &McpEvent::Viewing { scope: AgentScope::Slides });
        emit(&self.out_tx, "mcp", tab, &McpEvent::Surfaces { list: Vec::new() });
    }
```

`crates/oku-slides-agent/ARCHITECTURE.md`, ligne de `stack` : « `follow` (une autre vue suit l'onglet, `Keep::Replay`) et `set_draft` (brouillon tenu par `TabLink`, `AgentEvent::Draft`) ».

- [ ] **Step 5: Run Rust tests**

Run: `cargo nextest run -p oku-slides-agent`
Expected: PASS.

- [ ] **Step 6: Write the failing TS test** (`useAgentScope.render.test.tsx`)

```tsx
test("useAgentScope un évènement draft ne change pas l'état de l'agent", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onAgentEvent({ kind: "status", state: "working", scope: { kind: "slides" } }));
  await act(async () => h.state().onAgentEvent({ kind: "draft", text: "x", origin: null }));
  expect(h.state().status.state).toBe("working");
});
```

Run: `cd ui && bun test .render. -t "évènement draft"` → FAIL (typecheck : `kind: "draft"` inconnu).

- [ ] **Step 7: Implement (TS)**

`api.ts` :

```ts
/** Évènements du canal `agent` : état de l'agent et brouillon de l'onglet. */

export type AgentEvent =
  | { kind: "status"; state: AgentState; scope?: AgentScope }
  | { kind: "draft"; text: string | null; origin: string | null };
```

`AgentCommand` (type local) : `| { cmd: "follow"; tab: string } | { cmd: "set_draft"; tab: string; text: string | null; origin: string }`. `agentApi(tab)` :

```ts
    /** Une autre vue de l'onglet s'abonne : état, brouillon, puis le fil de la portée montrée. */
    follow: () => agentCall({ cmd: "follow", tab }),
    /** Le brouillon de l'onglet, tenu par l'hôte ; `null` : fermé. `origin` : la vue qui l'écrit. */
    setDraft: (text: string | null, origin: string) => agentCall({ cmd: "set_draft", tab, text, origin }),
```

`useAgentScope.ts` :

```ts
/** L'état de l'agent d'après ses évènements : seul `status` le change. */
export function agentStatusReducer(cur: AgentStatus, e: AgentEvent): AgentStatus {
  return e.kind === "status" ? { state: e.state, scope: e.scope } : cur;
}
```

et `useReducer(agentStatusReducer, IDLE)`. `const IDLE` (`useAgentScope.ts:8`) devient `export const IDLE` : la tâche 5 l'importe.

`AgentPanel.render.test.tsx:168` : `onAgent({ kind: "status", state: "working", scope: { kind: "slides" } })`, sans `as never`.

- [ ] **Step 8: Run TS checks**

Run: `cd ui && bun run typecheck && bun run test`
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/oku-slides-agent crates/oku-slides-i18n/src/locales ui/src/api.ts ui/src/agent/useAgentScope.ts ui/src/agent/useAgentScope.render.test.tsx ui/src/agent/AgentPanel.render.test.tsx

```
Message : `feat(agent) : brouillon tenu par l'hôte et suivi d'un onglet par une autre vue (t-654)`

---

### Task 4: Hôte Tauri — la fenêtre `agent`

**Files:**
- Create: `src-tauri/src/agent_window.rs`, `src-tauri/src/agent_window_tests.rs`
- Modify: `src-tauri/src/main.rs` : module, en-tête de module, `on_menu_event`.
- Modify: `src-tauri/src/windows.rs` : `sync`, et `overlay_title_bar` (partagée avec `agent_window::open`).
- Modify: `src-tauri/src/commands.rs:168-187` : `focused`.
- Modify: `src-tauri/src/menu.rs` : case à cocher (macOS) et état géré.
- Modify: `src-tauri/capabilities/default.json`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 langues)

**Interfaces:**
- Consumes: `WorkspaceView.agent_detached`, `Workspace::agent_detached()`, `WorkspaceRequest::{DetachAgent, AttachAgent}`, `oku_slides_session::is_window_label` (tâche 1).
- Produces :
  - `agent_window::LABEL: &str = "agent"`.
  - `agent_window::step(detached: bool, open: bool) -> Step`, avec `Step::{Open, Close, Keep}`.
  - `agent_window::reconcile(app: &AppHandle, detached: bool)`.
  - `agent_window::toggle(app: &AppHandle)`.
  - `menu::AGENT_ID: &str = "agent-detached"`.
  - `menu::set_agent_checked(app, detached)`.
  - Clés Fluent `window-agent-title` et `menu-agent-detached`.
  - Un changement de fenêtre au premier plan diffuse la vue `workspace`.
- La fermeture système de `agent` n'a besoin d'aucun code : `close_requested` voit que `is_only_window("agent")` est faux (tâche 1) et transmet `close-requested` à la page.

- [ ] **Step 1: Write the failing tests** (`agent_window_tests.rs`)

```rust
use speculoos::prelude::*;

use super::{Step, step};

#[test]
fn step_detached_without_window_opens_it() {
    assert_that!(step(true, false)).is_equal_to(Step::Open);
}

#[test]
fn step_attached_with_window_closes_it() {
    assert_that!(step(false, true)).is_equal_to(Step::Close);
}

#[test]
fn step_detached_with_window_keeps_it() {
    assert_that!(step(true, true)).is_equal_to(Step::Keep);
}

#[test]
fn step_attached_without_window_keeps_it() {
    assert_that!(step(false, false)).is_equal_to(Step::Keep);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p oku-slides-app agent_window`
Expected: FAIL (module absent).

- [ ] **Step 3: Implement `agent_window.rs`**

```rust
//! La fenêtre de l'agent détaché : une seule pour l'app, label `agent`.
//!
//! Elle charge la même page que les fenêtres de l'espace de travail ; l'interface s'y
//! reconnaît à son label. Ouverte ou fermée par `windows::sync` d'après
//! `WorkspaceView::agent_detached` ; sa géométrie est gardée par `tauri-plugin-window-state`,
//! sous ce label. Ce n'est pas une fenêtre de l'espace de travail : son focus ne change pas la
//! fenêtre active (`commands::focused`), et sa fermeture système passe par sa page
//! (`close-requested`), qui vide la frappe du brouillon puis rattache l'agent.

use std::sync::atomic::Ordering;

use oku_slides_session::{Workspace, WorkspaceRequest};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::blocking;
use crate::state::AppState;
use crate::windows;

/// Label de la fenêtre de l'agent (aussi lu par l'interface).
pub const LABEL: &str = "agent";

/// Taille d'une fenêtre neuve, en points.
const SIZE: (f64, f64) = (420.0, 800.0);
/// Taille minimale, en points : le fil et la barre de dictée y tiennent.
const MIN_SIZE: (f64, f64) = (320.0, 400.0);

/// Ce que `reconcile` fait de la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Agent détaché, fenêtre absente : l'ouvrir.
    Open,
    /// Agent rattaché, fenêtre ouverte : la fermer.
    Close,
    /// Déjà d'accord.
    Keep,
}

/// Que faire, l'agent étant détaché ou non et la fenêtre ouverte ou non.
pub fn step(detached: bool, open: bool) -> Step {
    match (detached, open) {
        (true, false) => Step::Open,
        (false, true) => Step::Close,
        _ => Step::Keep,
    }
}

fn open(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let title = match app.try_state::<AppState>() {
        Some(state) => state.localizer.tr("window-agent-title"),
        None => {
            tracing::debug!("état de l'app absent : fenêtre de l'agent sans titre");
            String::new()
        }
    };
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    // Comme les fenêtres de l'espace de travail : la page pose sa barre de saisie (`.titlebar`).
    windows::overlay_title_bar(builder).build()
}

/// Met la fenêtre de l'agent d'accord avec `detached`.
pub fn reconcile(app: &AppHandle, detached: bool) {
    let window = app.get_webview_window(LABEL);
    match step(detached, window.is_some()) {
        Step::Open => {
            if let Err(e) = open(app) {
                tracing::warn!(error = %e, "fenêtre de l'agent non ouverte");
            }
        }
        Step::Close => {
            if let Some(w) = window
                && let Err(e) = w.destroy()
            {
                tracing::warn!(error = %e, "fenêtre de l'agent non fermée");
            }
        }
        Step::Keep => {}
    }
}

/// Détache l'agent s'il est rattaché, et inversement (menu), hors du fil principal.
pub fn toggle(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let synced = blocking(&app, |app, state| {
            if state.closing.load(Ordering::SeqCst) {
                return Ok(());
            }
            // Lu puis appliqué sous deux verrous : deux clics très rapprochés peuvent faire la même
            // bascule ; sans danger, `sync` remet la case d'accord avec l'espace de travail.
            let detached = Workspace::lock(&state.workspace).agent_detached();
            let req = if detached { WorkspaceRequest::AttachAgent } else { WorkspaceRequest::DetachAgent };
            Workspace::apply(&state.workspace, req).map_err(|e| state.refuse(e))?;
            windows::sync(app, None, false);
            Ok(())
        })
        .await;
        if let Err(e) = synced {
            tracing::warn!(error = %e, "agent non détaché ou rattaché");
        }
    });
}

#[cfg(test)]
#[path = "agent_window_tests.rs"]
mod agent_window_tests;
```

`commands::blocking` est déjà `pub`. `AppState::localizer` est un `Arc<Localizer>` qui a `tr` (`state.rs:22`).

`main.rs` : ajouter `mod agent_window;`. L'en-tête du module cite la fenêtre `agent` et sa fermeture (elle passe par sa page, comme les autres fenêtres non dernières).

- [ ] **Step 4: Wire `windows.rs`**

`windows.rs::open` : ses deux lignes `#[cfg(target_os = "macos")] let builder = builder.title_bar_style(…).hidden_title(true);` deviennent un appel à une fonction partagée :

```rust
/// La barre de titre des fenêtres de l'app : sous macOS, fondue dans la page (barre `.titlebar`).
pub(crate) fn overlay_title_bar<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    builder
}
```

`sync` : dans la boucle de destruction, ne jamais détruire la fenêtre de l'agent. Juste après cette boucle, **avant** le bloc qui met la fenêtre active au premier plan (la fenêtre de l'agent ouverte au lancement ne lui vole pas le focus), réconcilier et cocher le menu :

```rust
    for (label, w) in app.webview_windows() {
        if label != agent_window::LABEL
            && !view.windows.iter().any(|v| v.label == label)
            && let Err(e) = w.destroy()
        {
            tracing::warn!(label, error = %e, "fenêtre non fermée");
        }
    }
    agent_window::reconcile(app, view.agent_detached);
    menu::set_agent_checked(app, view.agent_detached);
```

`rects` ne change pas : `Workspace::window_at` ne cherche déjà que parmi les fenêtres de l'espace de travail (`recent`). Un onglet lâché au-dessus de la fenêtre de l'agent vise la fenêtre de l'espace de travail qui est dessous, si elle y est, sinon une nouvelle fenêtre : comportement accepté.

- [ ] **Step 5: Focus**

`commands::focused` : en tête, avant toute autre chose :

```rust
    // La fenêtre de l'agent n'est pas une fenêtre de l'espace de travail : la cible ne change pas.
    if !oku_slides_session::is_window_label(label) {
        return;
    }
```

Dans la tâche `spawn_blocking`, le bloc `if let Err(e) = ws.focus(&label) { … }` devient :

```rust
        let before = ws.focused().to_owned();
        if let Err(e) = ws.focus(&label) {
            // Une fenêtre en cours de création n'y est pas encore.
            tracing::debug!(label, error = %e.one_line(), "focus ignoré");
            return;
        }
        let changed = ws.focused() != before;
        // `emit_view` prend le verrou de `sync` puis celui de l'espace de travail : rendre celui-ci d'abord.
        drop(ws);
        if changed {
            // La fenêtre de l'agent suit l'onglet actif de la fenêtre au premier plan.
            windows::emit_view(&app);
        }
```

- [ ] **Step 6: Menu (macOS), capability, Fluent**

`menu.rs` :

```rust
/// Identifiant de la case « Agent dans une fenêtre séparée ».
pub const AGENT_ID: &str = "agent-detached";

/// La case de l'agent, gardée par l'app : `Menu::get` ne cherche pas dans les sous-menus.
#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "menu d'app : macOS seulement"))]
struct AgentCheck<R: Runtime>(CheckMenuItem<R>);
```

Dans `build`, créer `let agent = CheckMenuItem::with_id(app, AGENT_ID, l.tr("menu-agent-detached"), true, false, None::<&str>)?;`. L'ajouter à la fin du sous-menu « Fenêtre », après un séparateur à ajouter (`&PredefinedMenuItem::separator(app)?`, puis `&agent`), puis appeler `app.manage(AgentCheck(agent.clone()));` avant de rendre le menu. Ensuite :

```rust
/// Coche la case de l'agent d'après l'espace de travail (macOS ; ailleurs, aucun menu : rien).
pub fn set_agent_checked<R: Runtime>(app: &AppHandle<R>, detached: bool) {
    #[cfg(target_os = "macos")]
    {
        let Some(check) = app.try_state::<AgentCheck<R>>() else { return };
        if let Err(e) = check.0.set_checked(detached) {
            tracing::warn!(error = %e, "case de l'agent non mise à jour");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, detached);
}
```

Hors macOS, le corps de `set_agent_checked` est vide : `AgentCheck` n'est alors construit que par `build`, lui-même jamais appelé. La structure est donc morte de façon certaine, et son `expect(dead_code)` toujours rempli. `build` garde son attribut `cfg_attr(not(target_os = "macos"), expect(dead_code, …))` (`menu.rs:23`) : hors macOS, il n'est toujours pas appelé. Importer `CheckMenuItem` et `Manager`.


`main.rs::on_menu_event`, à côté de `QUIT_ID` :

```rust
                if event.id() == menu::AGENT_ID {
                    agent_window::toggle(app);
                }
```

`capabilities/default.json` :
- `"windows": ["w-*", "agent"]` ;
- `"core:window:allow-set-focus"` ajouté aux permissions (⌘K met la fenêtre de l'agent devant) ;
- description : « Fenêtres de l'app (w-<n>, et la fenêtre de l'agent, même confiance) : commandes, événements, dialogues. »

Fluent, `session.ftl` des 20 langues (fr et en ci-dessous ; les 18 autres traduites) :

```
window-agent-title = Agent — Oku Slides
menu-agent-detached = Agent dans une fenêtre séparée
```
```
window-agent-title = Agent — Oku Slides
menu-agent-detached = Agent in a Separate Window
```

- [ ] **Step 7: Run checks**

Run: `just check`
Expected: PASS (fmt, clippy, nextest).

- [ ] **Step 8: Commit**

```bash
git add src-tauri crates/oku-slides-i18n/src/locales
```
Message : `feat(app) : fenêtre de l'agent, ouverte et fermée d'après l'espace de travail (t-654)`

---

### Task 5: Interface — `AgentPanel` pilote ou suiveur de la portée

`AgentPanel` (t-638) fixe aujourd'hui la portée d'après la vue de `App`, et route les fiches d'après la slide affichée. Cette tâche lui donne deux modes. En mode pilote, rien ne change pour `App`. En mode suiveur, la portée vient de l'hôte (`viewing`) et le routage d'une fiche posée ailleurs (`staged`). Elle extrait aussi ce que la fenêtre de l'agent partagera avec `App` : `useSlideThumbs` et `deckVocabulary`.

**Files:**
- Create: `ui/src/agent/useFollowScope.ts`, `ui/src/agent/useFollowScope.render.test.tsx`
- Create: `ui/src/useSlideThumbs.ts`, `ui/src/useSlideThumbs.render.test.tsx`
- Modify: `ui/src/agent/AgentPanel.tsx` : props `scope`, `route`, `ui`, `keyPlaces` et `onDraft` ; abonnement.
- Modify: `ui/src/agent/AgentPanel.render.test.tsx` : montage avec les nouvelles props, et nouveaux tests.
- Modify: `ui/src/agent/surfaces.ts` : `routeStaged`. Test : `ui/src/agent/surfaces.test.ts`.
- Modify: `ui/src/draft.ts` : `deckVocabulary`. Test : `ui/src/draft.test.ts`.
- Modify: `ui/src/App.tsx:92-97, :757-766`
- Modify: `ui/src/useDictation.ts:44-47` : `vocabulary` passe par `deckVocabulary`.

**Interfaces:**
- Consumes: `agentStatusReducer`, `IDLE` (tâche 3) ; `agentApi(tab).follow` (tâche 3) ; `McpEvent` `viewing` (tâche 2).
- Produces :

```ts
// AgentPanel.tsx
/** D'où vient la portée : cette fenêtre la fixe (`drive`), ou elle suit celle de l'hôte (`follow`). */
export type ScopeSource = { kind: "drive"; view: ScopeView | null } | { kind: "follow" };
// Props remplacées : scopeView → scope: ScopeSource ; view/current → route ; current/selected/editField/widget → ui.
//   route: (list: Surface[]) => Routed;   // le routage des surfaces, stable tant que la scène ne change pas
//   ui: AgentUi;                          // ce que voit l'utilisateur, joint aux consignes
//   keyPlaces?: QuickTarget["kind"][];    // où les touches rapides agissent ; défaut : toutes
//   onDraft?: (e: { text: string | null; origin: string | null }) => void;  // brouillon reçu de l'hôte

// useFollowScope.ts
/** La portée suivie : celle que l'hôte annonce (`viewing`) après `follow`. */
export function useFollowScope(agent: Pick<ReturnType<typeof agentApi>, "follow">): AgentScopeState & { onMcp: (e: McpEvent) => void };

// surfaces.ts
/** Le routage d'une vue sans scène : seule la fiche `staged` reste hors du fil. */
export function routeStaged(list: Surface[], staged: string | null): Routed;

// draft.ts
/** Les mots du deck utiles à la dictée : son titre et ceux des slides. */
export function deckVocabulary(snap: Snapshot | null): string[];

// useSlideThumbs.ts
/** Les miniatures des reçus de l'agent, aux empreintes actuelles du deck. */
export function useSlideThumbs(api: TabApi, slides: Snapshot["slides"] | undefined): SlideThumbs;
```

`useFollowScope` : `scope` vaut `{ kind: "slides" }` et `ready` vaut faux jusqu'au premier `viewing` **et** au succès de `follow`. `onReady` appelle `follow()`, à chaque abonnement et à chaque reconnexion ; seule la réponse du **dernier** `follow` compte (un échec tardif d'un appel plus ancien n'écrase pas un succès plus récent).

- [ ] **Step 1: Write the failing pure tests**

`surfaces.test.ts`, dans le `describe` où est définie l'usine `sheet(id, slide, owner, field?)` (`surfaces.test.ts:59`) :

```ts
  test("routeStaged la fiche posée sur la scène n'est pas dans le fil", () => {
    const routed = routeStaged([sheet("s-a", "s1", "e1"), sheet("s-b", "s2", "e2")], "s-a");
    expect(routed.loose.map((s) => s.id)).toEqual(["s-b"]);
  });

  test("routeStaged la fiche posée est rendue comme fiche de la scène", () => {
    const routed = routeStaged([sheet("s-a", "s1", "e1")], "s-a");
    expect(routed.stage?.id).toBe("s-a");
  });

  test("routeStaged sans fiche posée : toutes les fiches vont au fil", () => {
    expect(routeStaged([sheet("s-a", "s1", "e1")], null).loose.map((s) => s.id)).toEqual(["s-a"]);
  });
```
`draft.test.ts` :

```ts
test("deckVocabulary prend le titre du deck et ceux des slides, sans vides", () => {
  const snap = { title: "Feuille", slides: [{ title: "Constat" }, { title: null }] } as unknown as Snapshot;
  expect(deckVocabulary(snap)).toEqual(["Feuille", "Constat"]);
});

test("deckVocabulary sans instantané : aucun mot", () => {
  expect(deckVocabulary(null)).toEqual([]);
});
```

Run: `cd ui && bun test src/agent/surfaces.test.ts src/draft.test.ts` → FAIL (fonctions absentes).

- [ ] **Step 2: Implement the pure helpers**

```ts
export function routeStaged(list: Surface[], staged: string | null): Routed {
  const routed = routeSurfaces(list, null);
  const stage = staged === null ? undefined : routed.loose.find((s) => s.id === staged);
  return stage ? { ...routed, stage, loose: routed.loose.filter((s) => s !== stage) } : routed;
}
```

```ts
export function deckVocabulary(snap: Snapshot | null): string[] {
  return snap ? [snap.title, ...snap.slides.map((s) => s.title)].filter((s): s is string => !!s) : [];
}
```

`useDictation.ts:44-47` : `const vocabulary = useMemo(() => deckVocabulary(snap), [snap]);`. `App.tsx` : `const slideThumbs = useSlideThumbs(api, snap?.slides);`. `useSlideThumbs.ts` reprend `App.tsx:92-97` tel quel :

```ts
// Les miniatures des reçus de l'agent : le cache des SVG de l'onglet (partagé avec la pile),
// aux empreintes actuelles du deck. Utilisé par `App` et par la fenêtre de l'agent.

import { useMemo } from "react";
import type { Snapshot, TabApi } from "./api";
import type { SlideThumbs } from "./a2ui/slideThumbs";
import { useSvgCache } from "./useSvgCache";

export function useSlideThumbs(api: TabApi, slides: Snapshot["slides"] | undefined): SlideThumbs {
  const svgs = useSvgCache(api);
  return useMemo<SlideThumbs>(
    () => ({ cached: svgs.cached, svg: svgs.svg, current: (id) => slides?.find((s) => s.id === id)?.hash }),
    [svgs, slides],
  );
}
```

`useSlideThumbs.render.test.tsx` (montage comme les autres tests de rendu ; `api` factice `{ svg: async () => "<svg/>" }` typé `as unknown as TabApi`) :

```tsx
test("useSlideThumbs donne l'empreinte actuelle d'une slide", async () => {
  const thumbs = await mountThumbs([{ id: "s1", hash: "h1" }]);
  expect(thumbs().current("s1")).toBe("h1");
});

test("useSlideThumbs slide disparue : pas d'empreinte", async () => {
  const thumbs = await mountThumbs([]);
  expect(thumbs().current("s1")).toBeUndefined();
});
```

(`mountThumbs(slides)` monte un composant qui appelle `useSlideThumbs(api, slides as never)` et rend `() => dernière valeur`.)

Run: `cd ui && bun test src/agent/surfaces.test.ts src/draft.test.ts` → PASS.

- [ ] **Step 3: Write the failing hook and panel tests**

`useFollowScope.render.test.tsx` :

```tsx
import { afterEach, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useFollowScope } = await import("./useFollowScope");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mountFollow(fail = false) {
  const follows: number[] = [];
  const agent = {
    follow: async () => {
      follows.push(1);
      if (fail) throw new Error("no tools");
    },
  };
  let state: ReturnType<typeof useFollowScope> | null = null;
  const Probe = () => {
    state = useFollowScope(agent);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  return { follows, state: () => state! };
}

test("useFollowScope à l'abonnement prêt demande follow", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  expect(h.follows).toHaveLength(1);
});

test("useFollowScope à la reconnexion redemande follow", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  await act(async () => h.state().onReady());
  expect(h.follows).toHaveLength(2);
});

test("useFollowScope un évènement viewing fixe la portée", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "atelier", pack: "p-1" } }));
  expect(h.state().scope).toEqual({ kind: "atelier", pack: "p-1" });
});

test("useFollowScope n'est pas prêt avant le premier viewing", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(false);
});

test("useFollowScope est prêt après follow réussi et un viewing", async () => {
  const h = await mountFollow();
  await act(async () => h.state().onReady());
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  expect(h.state().ready).toBe(true);
});

test("useFollowScope follow en échec : pas prêt", async () => {
  const h = await mountFollow(true);
  await act(async () => h.state().onReady());
  await act(async () => h.state().onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  expect(h.state().ready).toBe(false);
});

test("useFollowScope un échec tardif d'un follow plus ancien ne défait pas le plus récent", async () => {
  const replies: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const agent = { follow: () => new Promise<void>((resolve, reject) => void replies.push({ resolve, reject })) };
  let state: ReturnType<typeof useFollowScope> | null = null;
  const Probe = () => {
    state = useFollowScope(agent);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  await act(async () => state!.onReady());
  await act(async () => state!.onReady());
  await act(async () => state!.onMcp({ kind: "viewing", scope: { kind: "slides" } }));
  await act(async () => replies[1].resolve());
  await act(async () => replies[0].reject(new Error("tard")));
  expect(state!.ready).toBe(true);
});
```

`AgentPanel.render.test.tsx`, utilitaire `mount()` (aujourd'hui lignes 47-128) :
- il prend des options `{ promptFails = false, scope = DRIVE, keyPlaces }`, avec `const DRIVE: ScopeSource = { kind: "drive", view: { templates: false, history: false, pack: null, widget: null } }` ;
- il passe `scope`, `keyPlaces`, `route: (list) => routeSurfaces(list, { slide: "s1", has: () => true })`, `ui: UI` (`{ slide: "s1", element: null, field: null, editing: false, widget: null }`) et `onDraft: (e) => void drafts.push(e)`. Il ne passe plus `view`, `current`, `selected`, `editField`, `widget` ni `scopeView` ;
- son faux `agent` enregistre : `prompt: async (s, text) => { prompts.push(text); promptScopes.push(s); if (promptFails) throw new Error("no agent"); }` (le reste du corps actuel, inchangé), `resync: async (s) => void resyncs.push(s)`, `follow: async () => void follows.push(1)` et `surfaceAction: async (id, name) => void actions.push(`${id}:${name}`)` ;
- il rend en plus `slots`, `drafts`, `promptScopes`, `resyncs`, `follows` et `actions`.

Nouveaux tests :

```tsx
const escape = () => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));

test("AgentPanel suiveur à l'abonnement prêt demande follow", async () => {
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.follows).toHaveLength(1);
});

test("AgentPanel suiveur à l'abonnement prêt ne demande jamais resync", async () => {
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.resyncs).toEqual([]);
});

test("AgentPanel suiveur une consigne part dans la portée annoncée par l'hôte", async () => {
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  await m.mcp({ kind: "viewing", scope: { kind: "atelier", pack: "p-1" } });
  await act(async () => void (await m.handle.current!.prompt("bonjour")));
  expect(m.promptScopes).toEqual([{ kind: "atelier", pack: "p-1" }]);
});

test("AgentPanel toutes places : Échap répond à la fenêtre de confirmation", async () => {
  const m = await mount();
  await m.mcp({ kind: "surface", surface: modal });
  await escape();
  expect(m.actions.some((a) => a.startsWith("m-1:"))).toBe(true);
});

test("AgentPanel touches limitées au dock : Échap laisse la fenêtre de confirmation", async () => {
  const m = await mount({ keyPlaces: ["dock"] });
  await m.mcp({ kind: "surface", surface: modal });
  await escape();
  expect(m.actions).toEqual([]);
});

test("AgentPanel un brouillon reçu de l'hôte est transmis", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "draft", text: "x", origin: "v-2" }));
  expect(m.drafts).toEqual([{ text: "x", origin: "v-2" }]);
});
```

(`modal` est la surface déjà définie dans ce fichier, ligne 134. Le premier test d'Échap est le témoin positif : sans lui, le second passerait même si Échap n'agissait jamais.)

Run: `cd ui && bun test .render. -t "useFollowScope|AgentPanel|useSlideThumbs"` → FAIL.
- [ ] **Step 4: Implement `useFollowScope.ts`**

```ts
// La portée suivie par une autre vue de l'onglet (fenêtre de l'agent) : celle que montre l'hôte.

import { useCallback, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentScope, McpEvent, agentApi } from "../api";
import { t } from "../i18n";
import { busyElsewhere, scopePlace } from "./scope";
import { IDLE, agentStatusReducer, type AgentScopeState } from "./useAgentScope";

const SLIDES: AgentScope = { kind: "slides" };

export function useFollowScope(agent: Pick<ReturnType<typeof agentApi>, "follow">): AgentScopeState & { onMcp: (e: McpEvent) => void } {
  const [viewing, setViewing] = useState<AgentScope | null>(null);
  const [followed, setFollowed] = useState(false);
  const [status, onStatus] = useReducer(agentStatusReducer, IDLE);
  // Seule la réponse du dernier `follow` compte : une reconnexion en relance un.
  const asked = useRef(0);
  const onReady = useCallback(() => {
    const n = ++asked.current;
    agent.follow().then(
      () => n === asked.current && setFollowed(true),
      (err) => {
        if (n === asked.current) setFollowed(false);
        console.warn("follow", err);
      },
    );
  }, [agent]);
  const onMcp = useCallback((e: McpEvent) => {
    if (e.kind === "viewing") setViewing(e.scope);
  }, []);
  const onAgentEvent = useCallback((e: AgentEvent) => onStatus(e), []);
  const scope = viewing ?? SLIDES;
  const elsewhere = busyElsewhere(status, scope);
  const notice = elsewhere ? t("ui-agent-busy-elsewhere", { place: scopePlace(elsewhere, t) }) : null;
  return { scope, status, elsewhere, notice, ready: followed && viewing !== null, onAgentEvent, onReady, onMcp };
}
```

- [ ] **Step 5: Implement `AgentPanel` modes**

Dans `AgentPanel.tsx` :
- Ajouter `ScopeSource` et les props de l'interface ci-dessus. Retirer `scopeView`, `view`, `current`, `selected`, `editField` et `widget` de `AgentPanelProps`.
- Portée :

```ts
  const driven = useAgentScope(agent, props.scope.kind === "drive" ? props.scope.view : null, onSurfaceEvent);
  const followed = useFollowScope(agent);
  const agentScopeState = props.scope.kind === "follow" ? followed : driven;
```

  Un panneau ne change jamais de mode : il est monté par onglet et par fenêtre. Un `useAgentScope` dont la vue vaut `null` ne rappelle jamais le fil.
- Routage : `const routed = useMemo(() => props.route(surfaces), [surfaces, props.route]);`.
- Abonnement, dans `onMcp`, après `onSurfaceEvent(e)` : `latest.current!.onMcpFollow(e);`. Les rappels existants (`onChanged`, `onFocus`, `onWidgetTouched`) se lisent désormais dans `latest.current!.props`. Puis :

```ts
        onAgent: (e) => {
          latest.current!.scopeState.onAgentEvent(e);
          if (e.kind === "draft") latest.current!.props.onDraft?.({ text: e.text, origin: e.origin });
        },
        onReady: () => {
          latest.current!.scopeState.onReady();
          latest.current!.props.onChanged();
        },
```


  `latest` change de forme. Aujourd'hui `const latest = useRef(props); latest.current = props;` est en tête du composant. Il devient, **après** les hooks de portée :

```ts
  /** Ce que lit l'abonnement, posé une fois par onglet : toujours la dernière version. */
  interface Latest {
    props: AgentPanelProps;
    scopeState: AgentScopeState;
    onMcpFollow: (e: McpEvent) => void;
  }
  const latest = useRef<Latest | null>(null);
  latest.current = { props, scopeState: agentScopeState, onMcpFollow: props.scope.kind === "follow" ? followed.onMcp : NOOP };
```

  (`interface Latest` au niveau du module, `const NOOP = () => {};` aussi.) Les rappels de l'abonnement lisent `latest.current!.props.onChanged()`, `latest.current!.scopeState…` et `latest.current!.onMcpFollow(e)`. Ils ne s'exécutent qu'après le premier rendu, donc `latest.current` est posé.
- Touches : `const keyed = !keysOff && keysTarget && (props.keyPlaces ?? ALL_PLACES).includes(keysTarget.kind) ? keysTarget : null;`, avec `const ALL_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal", "dock"];`. L'`answerTarget` des consignes reste calculé sur `keysTarget` entier, comme aujourd'hui.
- Consignes : `agent.prompt(scope, text, props.ui, answer)`. `agentUi` n'est plus importé ici.

Dans `App.tsx` :

```tsx
  const shown = useMemo(
    () => (view && view.slide.id === current ? { slide: view.slide.id, has: (a: Anchor) => anchorBox(view, a) !== null } : null),
    [view, current],
  );
  const route = useCallback((list: Surface[]) => routeSurfaces(list, shown), [shown]);
  const ui = useMemo(() => agentUi(current, selected, editField, widgets.activeName), [current, selected, editField, widgets.activeName]);
  const templates = !!snap?.editing;
  const inHistory = !!snap?.history;
  const pack = snap?.editing?.set ?? null;
  const scopeSource = useMemo<ScopeSource>(
    () => ({ kind: "drive", view: { templates, history: inHistory, pack, widget: widgets.activeName } }),
    [templates, inHistory, pack, widgets.activeName],
  );
```

Puis `<AgentPanel tab={tab} agent={agent} scope={scopeSource} route={route} ui={ui} keysOff={!!editing || picking || widgetActive} thread={!snap.read_only} thumbs={slideThumbs} slots={agentSlots} handle={agentHandle} onHostAction={(s, a) => host.onAction(s, a) || target.onAction(s, a)} onStatus={setAgentStatus} onChanged={() => void run(api.snapshot())} onFocus={onAgentFocus} onWidgetTouched={(name, removed) => widgets.touched(name, removed)} say={say} />` : les props d'aujourd'hui (`App.tsx:760-766`), moins `view`, `current`, `selected`, `editField`, `widget` et `scopeView`. `AgentPanel.tsx:26` importe en plus le type : `import { answerTarget, quickTarget, useQuickKeys, type QuickTarget } from "./useQuickKeys";`. Importer `anchorBox`, `routeSurfaces`, `agentUi` et `Anchor`, depuis les mêmes modules qu'`AgentPanel` aujourd'hui.

- [ ] **Step 6: Run checks**

Run: `cd ui && bun run typecheck && bun run test && just e2e <port libre>`
Expected: PASS, dont `agent.cjs`, `agent-scopes.cjs`, `lists-dock.cjs` et `target-panel.cjs` : le mode pilote ne change rien.

- [ ] **Step 7: Commit**

```bash
git add ui/src/agent/AgentPanel.tsx ui/src/agent/AgentPanel.render.test.tsx ui/src/agent/useFollowScope.ts ui/src/agent/useFollowScope.render.test.tsx ui/src/agent/surfaces.ts ui/src/agent/surfaces.test.ts ui/src/draft.ts ui/src/draft.test.ts ui/src/useSlideThumbs.ts ui/src/useSlideThumbs.render.test.tsx ui/src/useDictation.ts ui/src/App.tsx
```
Message : `refactor(ui) : panneau de l'agent pilote ou suiveur de la portée (t-654)`

---

### Task 6: Interface — brouillon tenu par l'hôte (`useDraft`)

**Files:**
- Create: `ui/src/agent/useDraft.ts`, `ui/src/agent/useDraft.render.test.tsx`
- Create: `ui/src/agent/draftActions.ts`, `ui/src/agent/draftActions.test.ts`
- Create: `ui/src/test/timers.ts`, `ui/src/randomId.ts`, `ui/src/randomId.test.ts`
- Modify: `ui/src/debounce.ts` (export `REAL_TIMERS`), `ui/src/debounce.test.ts` (utilise `test/timers.ts`)
- Modify: `ui/src/useDictation.ts` (le brouillon vient de `useDraft` ; `insertText` rend un booléen)
- Modify: `ui/src/useDictation.render.test.tsx` (il appelle `onDictated` et `sendDraft`, déplacés dans `actions`, et monte sans `setDraft`)
- Modify: `ui/src/App.tsx` (`onDraft` du panneau ; `DictationDraft` et `Dictation` lisent le nouveau brouillon)

**Interfaces:**
- Consumes: `agentApi(tab).setDraft(text, origin)`, `AgentEvent` `draft` (tâche 3) ; prop `onDraft` d'`AgentPanel` (tâche 5).
- Produces:

```ts
// debounce.ts
export const REAL_TIMERS: Timers;

// test/timers.ts
/** Minuteries à la main : `run()` déclenche ce qui attend, `delays` garde les délais demandés. */
export function manualTimers(): Timers & { run: () => void; delays: number[] };

// useDraft.ts
export const DRAFT_TYPING_MS = 300;
export interface DraftState { text: string | null; dictations: number; typing: number }
export interface Draft {
  state: DraftState;
  /** Brouillon reçu de l'hôte ; l'écho d'un envoi de cette vue est ignoré. */
  onHost: (e: { text: string | null; origin: string | null }) => void;
  /** Texte dicté ajouté (envoyé tout de suite). */
  append: (text: string) => void;
  /** Ouvre le brouillon pour écrire (⌘K ; envoyé tout de suite). */
  type: () => void;
  /** Frappe (envoyée après DRAFT_TYPING_MS). */
  change: (text: string) => void;
  /** Ferme le brouillon (envoyé tout de suite). */
  close: () => void;
  /** Envoie tout de suite la frappe en attente ; résolue une fois tous les envois partis. */
  flush: () => Promise<void>;
}
export function useDraft(setDraft: (text: string | null, origin: string) => Promise<void>, timers?: Timers): Draft;

// draftActions.ts
export interface DraftActions {
  /** Texte dicté : au curseur pendant l'édition (`insert`), sinon au brouillon. */
  onDictated: (text: string, mode: DictationMode) => void;
  /** « Envoyer » : le brouillon ne se ferme qu'une fois la consigne partie. */
  sendDraft: (text: string) => Promise<void>;
  /** « Insérer » : le brouillon se ferme, le texte s'écrit ; refusé, il revient au brouillon. */
  insertDraft: (text: string) => Promise<void>;
}
/** Ce dont les sorties du brouillon ont besoin. */
export interface DraftActionsDeps {
  draft: Pick<Draft, "append" | "close">;
  editing: boolean;
  /** Écrit le texte dans le deck ; `false` : nulle part où l'écrire. */
  insert: (text: string) => boolean | Promise<boolean>;
  prompt: (text: string) => Promise<boolean>;
}
export function draftActions(deps: DraftActionsDeps): DraftActions;

// randomId.ts
/** Un identifiant aléatoire court, préfixé ; jamais un secret. */
export function randomId(prefix: string): string;
```

**Synchronisation.** L'hôte diffuse les écritures dans l'ordre où il les applique. Une vue compte ses envois dont l'écho n'est pas encore revenu (`inflight`) :
- chaque envoi incrémente `inflight` ;
- l'écho de la vue, reconnu par son `origin`, le décrémente ;
- un envoi en échec le décrémente aussi. L'hôte refuse explicitement `set_draft` sans agent démarré (tâche 3), donc chaque envoi rend soit un écho, soit une erreur.

Tant que `inflight > 0` (ou qu'une frappe attend), un brouillon venu d'ailleurs est **plus ancien** que la dernière écriture de cette vue : il est ignoré. Une fois le dernier écho revenu, ce qui arrive est plus récent, donc appliqué.

`useDictation` rend `{ dictated, insertTo, destination, vocabulary, draft, actions, insertText }` : `draft` est un `Draft`, `actions` un `DraftActions`, et `insertText(text): boolean` vaut vrai s'il a écrit. Les anciens `setDraftText`, `closeDraft`, `typeDraft`, `sendDraft`, `insertDraft` et `onDictated` sont supprimés. `useDictation` reçoit en plus `setDraft` (`agent.setDraft`).

- [ ] **Step 1: Shared timers and the failing tests**

`ui/src/test/timers.ts` :

```ts
// Minuteries à la main pour les tests : rien ne part avant `run()`.

import type { Timers } from "../debounce";

export function manualTimers(): Timers & { run: () => void; delays: number[] } {
  const waiting = new Map<number, () => void>();
  const delays: number[] = [];
  let n = 0;
  return {
    delays,
    set: (fn, ms) => {
      delays.push(ms);
      waiting.set(++n, fn);
      return n;
    },
    clear: (h) => void waiting.delete(h as number),
    run: () => {
      const now = [...waiting.values()];
      waiting.clear();
      now.forEach((fn) => fn());
    },
  };
}
```

`debounce.test.ts` : remplacer ses minuteries écrites dans le fichier (`manual()`) par `manualTimers()`. Les assertions suivent l'utilitaire partagé : `m.fire()` devient `m.timers.run()`, et `m.delay()` devient `m.timers.delays.at(-1) ?? null`. Le fichier garde ses scénarios.

`randomId.ts` (partagé par `useDraft` et `link.ts`) :

```ts
// Identifiants aléatoires courts : relier un envoi à sa réponse, reconnaître une vue. Jamais un secret.

export function randomId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 12)}`;
}
```

`randomId.test.ts` :

```ts
test("randomId commence par son préfixe", () => {
  expect(randomId("v").startsWith("v-")).toBe(true);
});

test("randomId deux appels donnent deux identifiants", () => {
  expect(randomId("v")).not.toBe(randomId("v"));
});
```

`useDraft.render.test.tsx` :

```tsx
import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { manualTimers } from "../test/timers";

const { StrictMode, act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useDraft, DRAFT_TYPING_MS } = await import("./useDraft");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount({ strict = false, slow = false, fail = false }: { strict?: boolean; slow?: boolean; fail?: boolean } = {}) {
  const timers = manualTimers();
  const sent: (string | null)[] = [];
  const origins = new Set<string>();
  let release: () => void = () => {};
  const setDraft = (text: string | null, origin: string) => {
    origins.add(origin);
    if (fail) return Promise.reject(new Error("no agent"));
    if (!slow) return Promise.resolve(void sent.push(text));
    return new Promise<void>((resolve) => {
      release = () => {
        sent.push(text);
        resolve();
      };
    });
  };
  let draft: ReturnType<typeof useDraft> | null = null;
  const Probe = () => {
    draft = useDraft(setDraft, timers);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  const tree = strict ? createElement(StrictMode, null, createElement(Probe)) : createElement(Probe);
  await act(async () => root.render(tree));
  cleanup = async () => act(async () => root.unmount());
  return { timers, sent, origins, draft: () => draft!, release: () => release(), unmount: () => act(async () => root.unmount()) };
}

describe("useDraft", () => {
  test("useDraft append sous StrictMode envoie une seule fois", async () => {
    const m = await mount({ strict: true });
    await act(async () => m.draft().append("bonjour"));
    expect(m.sent).toEqual(["bonjour"]);
  });

  test("useDraft append ajoute le texte et compte la dictée", async () => {
    const m = await mount();
    await act(async () => m.draft().append("bonjour"));
    expect(m.draft().state).toEqual({ text: "bonjour", dictations: 1, typing: 0 });
  });

  test("useDraft type ouvre un brouillon vide et l'envoie tout de suite", async () => {
    const m = await mount();
    await act(async () => m.draft().type());
    expect(m.draft().state.text).toBe("");
    expect(m.sent).toEqual([""]);
  });

  test("useDraft change attend le délai de frappe avant d'envoyer", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    expect(m.sent).toEqual([]);
    expect(m.timers.delays.at(-1)).toBe(DRAFT_TYPING_MS);
  });

  test("useDraft change en rafale n'envoie que le dernier texte", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    await act(async () => m.draft().change("ab"));
    await act(async () => m.timers.run());
    expect(m.sent).toEqual(["ab"]);
  });

  test("useDraft close envoie null et oublie la frappe en attente", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    await act(async () => m.draft().close());
    await act(async () => m.timers.run());
    expect(m.sent).toEqual([null]);
  });

  test("useDraft brouillon de l'hôte venu d'une autre vue remplace le texte", async () => {
    const m = await mount();
    await act(async () => m.draft().onHost({ text: "de l'autre vue", origin: "ailleurs" }));
    expect(m.draft().state.text).toBe("de l'autre vue");
  });

  test("useDraft brouillon rejoué par l'hôte (sans origine) remplace le texte", async () => {
    const m = await mount();
    await act(async () => m.draft().onHost({ text: "rejoué", origin: null }));
    expect(m.draft().state.text).toBe("rejoué");
  });

  test("useDraft l'écho d'un envoi de cette vue est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().append("a"));
    await act(async () => m.draft().close());
    const [own] = [...m.origins];
    await act(async () => m.draft().onHost({ text: "a", origin: own }));
    expect(m.draft().state.text).toBeNull();
  });

  test("useDraft brouillon de l'hôte pendant une frappe en attente est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().change("ici"));
    await act(async () => m.draft().onHost({ text: "ailleurs", origin: "autre" }));
    expect(m.draft().state.text).toBe("ici");
  });

  test("useDraft brouillon de l'hôte après l'envoi de la frappe est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().change("ici"));
    await act(async () => m.timers.run());
    await act(async () => m.draft().onHost({ text: "ailleurs", origin: "autre" }));
    expect(m.draft().state.text).toBe("ailleurs");
  });

  test("useDraft au démontage la frappe en attente part", async () => {
    const m = await mount();
    await act(async () => m.draft().change("dernier mot"));
    await m.unmount();
    cleanup = null;
    expect(m.sent).toEqual(["dernier mot"]);
  });

  test("useDraft les envois partent dans l'ordre, un à la fois", async () => {
    const m = await mount({ slow: true });
    await act(async () => m.draft().append("un"));
    await act(async () => m.draft().close());
    expect(m.sent).toEqual([]);
    await act(async () => m.release());
    expect(m.sent).toEqual(["un"]);
    await act(async () => m.release());
    expect(m.sent).toEqual(["un", null]);
  });

  test("useDraft brouillon d'ailleurs reçu avant l'écho de cette vue est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "plus ancien", origin: "autre" }));
    expect(m.draft().state.text).toBe("mien");
  });

  test("useDraft brouillon d'ailleurs reçu après l'écho de cette vue est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    const [own] = [...m.origins];
    await act(async () => m.draft().onHost({ text: "mien", origin: own }));
    await act(async () => m.draft().onHost({ text: "plus récent", origin: "autre" }));
    expect(m.draft().state.text).toBe("plus récent");
  });

  test("useDraft un envoi en échec ne bloque pas les brouillons suivants", async () => {
    const failing = await mount({ fail: true });
    await act(async () => failing.draft().append("refusé"));
    await act(async () => failing.draft().onHost({ text: "d'ailleurs", origin: "autre" }));
    expect(failing.draft().state.text).toBe("d'ailleurs");
  });

  test("useDraft écho perdu puis rejeu de l'hôte : le texte de cette vue est renvoyé", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "ancien", origin: null }));
    expect(m.sent).toEqual(["mien", "mien"]);
  });

  test("useDraft écho perdu puis rejeu : le brouillon suivant d'ailleurs est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "ancien", origin: null }));
    const [own] = [...m.origins];
    await act(async () => m.draft().onHost({ text: "mien", origin: own }));
    await act(async () => m.draft().onHost({ text: "plus récent", origin: "autre" }));
    expect(m.draft().state.text).toBe("plus récent");
  });

  test("useDraft flush se résout une fois la frappe envoyée", async () => {
    const m = await mount({ slow: true });
    await act(async () => m.draft().change("dernier mot"));
    let done = false;
    const flushed = m.draft().flush().then(() => void (done = true));
    // L'envoi part au tour suivant de la file : laisser `setDraft` poser `release`.
    await act(async () => {});
    expect(done).toBe(false);
    await act(async () => m.release());
    await act(async () => flushed);
    expect(done).toBe(true);
    expect(m.sent).toEqual(["dernier mot"]);
  });
});
```


`draftActions.test.ts` :

```ts
import { expect, test } from "bun:test";
import { draftActions } from "./draftActions";

function deps(over: Partial<Parameters<typeof draftActions>[0]> = {}) {
  const log: string[] = [];
  const base = {
    draft: { append: (t: string) => void log.push(`append:${t}`), close: () => void log.push("close") },
    editing: false,
    insert: (t: string) => (log.push(`insert:${t}`), true),
    prompt: async (t: string) => (log.push(`prompt:${t}`), true),
  };
  return { log, actions: draftActions({ ...base, ...over }) };
}

test("draftActions dictée pendant l'édition : au curseur", () => {
  const { log, actions } = deps({ editing: true });
  actions.onDictated("mot", "auto");
  expect(log).toEqual(["insert:mot"]);
});

test("draftActions dictée hors édition : au brouillon", () => {
  const { log, actions } = deps();
  actions.onDictated("mot", "auto");
  expect(log).toEqual(["append:mot"]);
});

test("draftActions envoyer : ferme le brouillon une fois la consigne partie", async () => {
  const { log, actions } = deps();
  await actions.sendDraft("fais ceci");
  expect(log).toEqual(["prompt:fais ceci", "close"]);
});

test("draftActions envoyer en échec : le brouillon reste", async () => {
  const { log, actions } = deps({ prompt: async () => false });
  await actions.sendDraft("fais ceci");
  expect(log).toEqual([]);
});

test("draftActions insérer : ferme puis écrit", async () => {
  const { log, actions } = deps();
  await actions.insertDraft("texte");
  expect(log).toEqual(["close", "insert:texte"]);
});

test("draftActions insérer refusé : le texte revient au brouillon", async () => {
  const { log, actions } = deps({ insert: () => false });
  await actions.insertDraft("texte");
  expect(log).toEqual(["close", "append:texte"]);
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd ui && bun test src/agent/draftActions.test.ts && bun test .render. -t useDraft`
Expected: FAIL (modules absents).

- [ ] **Step 3: Implement**

`debounce.ts` : renommer la constante `REAL` en `export const REAL_TIMERS`, et la garder comme valeur par défaut de `debounced`.

`useDraft.ts` :

```ts
// Le brouillon de l'onglet, tenu par l'hôte : il survit au détachement de l'agent. Chaque vue
// garde le sien en local, l'envoie dans l'ordre, et ignore l'écho de ses propres envois.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { REAL_TIMERS, debounced, type Timers } from "../debounce";
import { appendDraft } from "../draft";
import { randomId } from "../randomId";

/** Délai d'envoi de la frappe. */
export const DRAFT_TYPING_MS = 300;

const EMPTY: DraftState = { text: null, dictations: 0, typing: 0 };

/** Un identifiant de vue, pour reconnaître ses propres échos. */


export function useDraft(setDraft: (text: string | null, origin: string) => Promise<void>, timers: Timers = REAL_TIMERS): Draft {
  const [state, setState] = useState<DraftState>(EMPTY);
  const current = useRef(state);
  const write = useCallback((next: DraftState) => {
    current.current = next;
    setState(next);
  }, []);
  const [origin] = useState(() => randomId("v"));
  const push = useRef(setDraft);
  push.current = setDraft;
  // Un envoi à la fois : l'hôte reçoit les écritures dans l'ordre où elles ont été faites.
  const queue = useRef(Promise.resolve());
  // Envois dont l'écho n'est pas revenu : tant qu'il y en a, ce qui vient d'ailleurs est plus ancien.
  const inflight = useRef(0);
  const send = useCallback(
    (text: string | null) => {
      inflight.current++;
      queue.current = queue.current
        .then(() => push.current(text, origin))
        .catch((e) => {
          // Pas d'écho pour un envoi refusé : il ne compte plus.
          inflight.current = Math.max(0, inflight.current - 1);
          console.warn("brouillon", String(e));
        });
    },
    [origin],
  );
  const pending = useRef(false);
  const typing = useMemo(
    () =>
      debounced(
        (text: string) => {
          pending.current = false;
          send(text);
        },
        DRAFT_TYPING_MS,
        timers,
      ),
    [send, timers],
  );
  useEffect(() => () => typing.flush(), [typing]);
  const now = useCallback(
    (text: string | null) => {
      typing.cancel();
      pending.current = false;
      send(text);
    },
    [typing, send],
  );
  const onHost = useCallback(
    (e: { text: string | null; origin: string | null }) => {
      if (e.origin === origin) {
        inflight.current = Math.max(0, inflight.current - 1);
        return;
      }
      // Un rejeu de l'hôte (`follow`, `resync`) pendant qu'un écho manque : l'écho est perdu
      // (relais en retard, vue sans abonnement). Cette vue a écrit en dernier : elle se resynchronise
      // en renvoyant son texte, au lieu d'ignorer pour toujours ce qui vient d'ailleurs.
      if (e.origin === null && inflight.current > 0 && !pending.current) {
        inflight.current = 0;
        send(current.current.text);
        return;
      }
      if (pending.current || inflight.current > 0) return;
      if (current.current.text !== e.text) write({ ...current.current, text: e.text });
    },
    [origin, write, send],
  );
  const append = useCallback(
    (text: string) => {
      const d = current.current;
      const next = appendDraft(d.text, text);
      write({ ...d, text: next, dictations: d.dictations + 1 });
      now(next);
    },
    [write, now],
  );
  const type = useCallback(() => {
    const d = current.current;
    const next = d.text ?? "";
    write({ ...d, text: next, typing: d.typing + 1 });
    now(next);
  }, [write, now]);
  const change = useCallback(
    (text: string) => {
      pending.current = true;
      write({ ...current.current, text });
      typing(text);
    },
    [write, typing],
  );
  const close = useCallback(() => {
    write({ ...current.current, text: null });
    now(null);
  }, [write, now]);
  const flush = useCallback(() => {
    typing.flush();
    return queue.current;
  }, [typing]);
  return useMemo(() => ({ state, onHost, append, type, change, close, flush }), [state, onHost, append, type, change, close, flush]);
}
```

(Les types `DraftState` et `Draft` sont déclarés et exportés en tête de fichier, comme dans l'interface ci-dessus.)

`draftActions.ts` :

```ts
// Ce que font le texte dicté et les sorties du brouillon, dans la fenêtre principale comme dans
// celle de l'agent : seule l'écriture dans le deck (`insert`) diffère.

import type { DictationMode } from "./keys";
import { dictationRoute } from "../draft";

export function draftActions({ draft, editing, insert, prompt }: DraftActionsDeps): DraftActions {
  const write = async (text: string) => {
    if (!(await insert(text))) draft.append(text);
  };
  return {
    onDictated: (text, mode) => {
      if (dictationRoute(mode, editing) === "cursor") void write(text);
      else draft.append(text);
    },
    sendDraft: async (text) => {
      if (await prompt(text)) draft.close();
    },
    insertDraft: async (text) => {
      draft.close();
      await write(text);
    },
  };
}
```

(`DraftActionsDeps` et `DraftActions` sont déclarés et exportés comme dans l'interface.)

`useDictation.render.test.tsx` : le montage passe `setDraft: async () => {}`, et les deux tests appellent `out!.actions.onDictated(…)` / `d.state().actions.sendDraft(…)` puis lisent `d.state().draft.state.text`. Les scénarios et les assertions ne changent pas (t-211 : brouillon gardé si la consigne échoue).

Le test « dictée pendant l'édition » attend `insert:mot` sans `append` : `write` attend le booléen, qui vaut vrai.

`useDictation.ts` :
- retirer le `useState` du brouillon et ses fonctions ;
- ajouter le paramètre `setDraft: (text: string | null, origin: string) => Promise<void>` ;
- `const draft = useDraft(setDraft);` ;
- `insertText` rend `boolean` : `true` après `setDictated`, après `apply(...)` et après `addElement(...)` ; `false` sans `view` et après le toast `ui-toast-select-zone` ;
- `const actions = draftActions({ draft, editing: editing !== null, insert: insertText, prompt });`.

`App.tsx` :
- `useDictation({ …, setDraft: agent.setDraft })` ;
- `<AgentPanel … onDraft={dictation.draft.onHost} />` ;
- `DictationDraft` reçoit `text={dictation.draft.state.text}`, `dictations={…state.dictations}`, `typing={…state.typing}`, `onChange={dictation.draft.change}`, `onInsert={(t) => void dictation.actions.insertDraft(t)}`, `onSend={(t) => void dictation.actions.sendDraft(t)}` et `onDiscard={dictation.draft.close}` ;
- `Dictation` reçoit `onText={dictation.actions.onDictated}` et `onType={dictation.draft.type}`.

- [ ] **Step 4: Run checks**

Run: `cd ui && bun run typecheck && bun run test && just e2e <port libre>`
Expected: PASS (`agent-scopes.cjs` envoie par le brouillon avec ⌘K).

- [ ] **Step 5: Commit**

```bash
git add ui/src/randomId.ts ui/src/randomId.test.ts ui/src/useDictation.render.test.tsx ui/src/agent/useDraft.ts ui/src/agent/useDraft.render.test.tsx ui/src/agent/draftActions.ts ui/src/agent/draftActions.test.ts ui/src/test/timers.ts ui/src/debounce.ts ui/src/debounce.test.ts ui/src/useDictation.ts ui/src/App.tsx
```
Message : `feat(ui) : le brouillon de l'agent est tenu par l'hôte (t-654)`

---

### Task 7: Interface — le canal `agent-link`

**Files:**
- Create: `ui/src/agent/link.ts`, `ui/src/agent/link.test.ts`, `ui/src/test/link.ts`

**Interfaces:**
- Produces:

```ts
export const AGENT_LABEL = "agent";
export const INSERT_ACK_MS = 3000;
/** Ce que la fenêtre principale dit de l'onglet affiché. */
export interface LinkContext {
  /** Une édition de texte est en cours : la dictée va au curseur. */
  editing: boolean;
  /** Libellé « Insérer dans … » ; null : nulle part où écrire. */
  insertTo: string | null;
  /** Ce que voit l'utilisateur, joint aux consignes. */
  ui: AgentUi;
  /** La fiche posée sur la scène, sinon null. */
  staged: string | null;
  /** Touches rapides coupées (édition, pointage, widget actif). */
  keysOff: boolean;
}
/** Raccourci brut relayé : la fenêtre de l'agent l'interprète avec son propre état. */
export type LinkShortcut = "auto" | "draft" | "type";
export type LinkMessage =
  | { to: "agent"; kind: "context"; tab: string; context: LinkContext }
  | { to: "main"; kind: "hello"; tab: string }
  | { to: "main"; kind: "insert"; tab: string; id: string; text: string }
  | { to: "main"; kind: "insert-cancel"; tab: string; id: string }
  | { to: "agent"; kind: "insert-ack"; tab: string; id: string; ok: boolean }
  | { to: "agent"; kind: "shortcut"; tab: string; key: LinkShortcut };
/** Un transport : diffuse à toutes les pages, celle qui envoie comprise. */
export interface LinkTransport {
  send(m: LinkMessage): void;
  /** Écoute ; renvoie de quoi arrêter. */
  listen(cb: (m: LinkMessage) => void): () => void;
  /** Résolue une fois l'écoute de l'hôte en place (Tauri : `listen` est asynchrone). */
  ready: Promise<void>;
  /** Arrête l'écoute de l'hôte (ferme le `BroadcastChannel`). */
  close(): void;
}
/** Plus long texte accepté dans un message (`insert`), en caractères : la limite du brouillon. */
export const MAX_LINK_TEXT = 256 * 1024;
/** Le message, s'il a la forme d'un `LinkMessage` (schéma zod) ; sinon null. */
export function parseLink(raw: unknown): LinkMessage | null;
/** Évènement Tauri « agent-link », émis à toutes les fenêtres (l'émettrice comprise). */
export function tauriTransport(): LinkTransport;
/** `BroadcastChannel` « oku-slides-agent-link » (hors Tauri, e2e) ; livre aussi à la page elle-même. */
export function channelTransport(): LinkTransport;
/** Le transport de cette page, créé une fois. */
export function linkTransport(): LinkTransport;
/** Les messages pour `to` et `tab` seulement. */
export function onlyFor(to: "agent" | "main", tab: string, cb: (m: LinkMessage) => void): (m: LinkMessage) => void;
/** Envoie `insert` et attend l'accusé ; refusé ou sans réponse : false (sans réponse : `insert-cancel`). */
export function insertWithAck(t: LinkTransport, tab: string, text: string, ms?: number, timers?: Timers): Promise<boolean>;
```

- [ ] **Step 1: Shared memory transport** (`ui/src/test/link.ts`)

```ts
// Un transport `agent-link` en mémoire : ce qui est envoyé est livré à tous les écouteurs.

import type { LinkMessage, LinkTransport } from "../agent/link";

export function memoryTransport(): LinkTransport & { sent: LinkMessage[]; listeners: () => number } {
  const listeners = new Set<(m: LinkMessage) => void>();
  const sent: LinkMessage[] = [];
  return {
    sent,
    listeners: () => listeners.size,
    ready: Promise.resolve(),
    close: () => listeners.clear(),
    send: (m) => {
      sent.push(m);
      [...listeners].forEach((cb) => cb(m));
    },
    listen: (cb) => {
      listeners.add(cb);
      return () => void listeners.delete(cb);
    },
  };
}
```

- [ ] **Step 2: Write the failing tests** (`link.test.ts`)

```ts
import { describe, expect, test } from "bun:test";
import { memoryTransport } from "../test/link";
import { manualTimers } from "../test/timers";
import { MAX_LINK_TEXT, channelTransport, insertWithAck, onlyFor, parseLink, type LinkMessage } from "./link";

/** Une fenêtre principale qui accuse chaque `insert` de `tab` avec `ok`. */
function answering(t: ReturnType<typeof memoryTransport>, tab: string, ok: boolean) {
  t.listen(onlyFor("main", tab, (m) => {
    if (m.kind === "insert") t.send({ to: "agent", kind: "insert-ack", tab, id: m.id, ok });
  }));
}

describe("agent-link", () => {
  test("insertWithAck accusé ok : true", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    expect(await insertWithAck(t, "t-1", "texte", 3000, manualTimers())).toBe(true);
  });

  test("insertWithAck refusé : false", async () => {
    const t = memoryTransport();
    answering(t, "t-1", false);
    expect(await insertWithAck(t, "t-1", "texte", 3000, manualTimers())).toBe(false);
  });

  test("insertWithAck sans réponse : false à l'échéance", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck sans réponse : insert-cancel envoyé pour cet identifiant", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    await done;
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    expect(t.sent).toContainEqual({ to: "main", kind: "insert-cancel", tab: "t-1", id: insert.id });
  });

  test("insertWithAck ignore l'accusé d'un autre envoi", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    t.send({ to: "agent", kind: "insert-ack", tab: "t-1", id: "autre", ok: true });
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck ignore l'accusé d'un autre onglet au même identifiant", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    t.send({ to: "agent", kind: "insert-ack", tab: "t-2", id: insert.id, ok: true });
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck un accusé après l'échéance ne change rien", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    t.send({ to: "agent", kind: "insert-ack", tab: "t-1", id: insert.id, ok: true });
    expect(await done).toBe(false);
  });

  test("insertWithAck retire son écoute une fois fini", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    const before = t.listeners();
    await insertWithAck(t, "t-1", "texte", 3000, manualTimers());
    expect(t.listeners()).toBe(before);
  });

  test("insertWithAck accusé reçu : la minuterie est oubliée", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    const timers = manualTimers();
    await insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    expect(t.sent.filter((m) => m.kind === "insert-cancel")).toEqual([]);
  });

  test("onlyFor ne garde que le sens et l'onglet visés", () => {
    const got: LinkMessage[] = [];
    const cb = onlyFor("agent", "t-1", (m) => void got.push(m));
    const mine: LinkMessage = { to: "agent", kind: "shortcut", tab: "t-1", key: "auto" };
    cb({ to: "agent", kind: "shortcut", tab: "t-2", key: "auto" });
    cb({ to: "main", kind: "hello", tab: "t-1" });
    cb(mine);
    expect(got).toEqual([mine]);
  });

  test("parseLink écarte un texte plus long que la limite", () => {
    expect(parseLink({ to: "main", kind: "insert", tab: "t-1", id: "i-1", text: "a".repeat(MAX_LINK_TEXT + 1) })).toBeNull();
  });

  test("parseLink écarte un message mal formé", () => {
    expect(parseLink({ to: "main", kind: "insert", tab: "t-1", text: 3 })).toBeNull();
  });

  test("parseLink garde un message bien formé", () => {
    const m = { to: "main", kind: "hello", tab: "t-1" };
    expect(parseLink(m)).toEqual(m);
  });

  test("channelTransport livre à la page elle-même", () => {
    const t = channelTransport();
    const got: LinkMessage[] = [];
    t.listen((m) => void got.push(m));
    t.send({ to: "main", kind: "hello", tab: "t-1" });
    t.close();
    expect(got).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
  });

  test("channelTransport livre à une autre page du même navigateur", async () => {
    const a = channelTransport();
    const b = channelTransport();
    const got: LinkMessage[] = [];
    b.listen((m) => void got.push(m));
    a.send({ to: "main", kind: "hello", tab: "t-1" });
    for (let i = 0; i < 50 && got.length === 0; i++) await new Promise((r) => setTimeout(r, 5));
    a.close();
    b.close();
    expect(got).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
  });
});
```

(`link.test.ts` tourne sans happy-dom : c'est le `BroadcastChannel` de Bun, entre deux instances du même processus. Chaque test ferme ses transports, pour ne pas garder le lanceur en vie.)

- [ ] **Step 3: Run to verify failure**

Run: `cd ui && bun test src/agent/link.test.ts`
Expected: FAIL (module absent).

- [ ] **Step 4: Implement `link.ts`**

```ts
// Canal entre la fenêtre principale et la fenêtre de l'agent : ce qui n'existe que dans la
// fenêtre principale (où écrire, contexte de l'interface). Tout le reste passe par l'hôte.
// Tauri : `emit` livre à toutes les fenêtres, l'émettrice comprise ; `onlyFor` trie.

import { z } from "zod";
import type { AgentUi } from "../api";
import { inTauri, listenTauri } from "../api";
import { REAL_TIMERS, type Timers } from "../debounce";
import { randomId } from "../randomId";

export const AGENT_LABEL = "agent";

export const INSERT_ACK_MS = 3000;
const EVENT = "agent-link";

// Ici, tels quels : les types `LinkContext`, `LinkShortcut`, `LinkMessage`, `LinkTransport`
// et la constante `MAX_LINK_TEXT` du bloc « Interfaces » de cette tâche.

// Bornes : un identifiant (onglet, envoi, élément) reste court ; un libellé, une ligne.
const Id = z.string().max(64);
const Label = z.string().max(256);
const Ui = z.object({ slide: Id.nullable(), element: Id.nullable(), field: Label.nullable(), editing: z.boolean(), widget: Label.nullable() });
const Context = z.object({ editing: z.boolean(), insertTo: Label.nullable(), ui: Ui, staged: Id.nullable(), keysOff: z.boolean() });
const Message = z.discriminatedUnion("kind", [
  z.object({ to: z.literal("agent"), kind: z.literal("context"), tab: Id, context: Context }),
  z.object({ to: z.literal("main"), kind: z.literal("hello"), tab: Id }),
  z.object({ to: z.literal("main"), kind: z.literal("insert"), tab: Id, id: Id, text: z.string().max(MAX_LINK_TEXT) }),
  z.object({ to: z.literal("main"), kind: z.literal("insert-cancel"), tab: Id, id: Id }),
  z.object({ to: z.literal("agent"), kind: z.literal("insert-ack"), tab: Id, id: Id, ok: z.boolean() }),
  z.object({ to: z.literal("agent"), kind: z.literal("shortcut"), tab: Id, key: z.enum(["auto", "draft", "type"]) }),
]);

export function parseLink(raw: unknown): LinkMessage | null {
  const r = Message.safeParse(raw);
  return r.success ? (r.data as LinkMessage) : null;
}

/** Les écouteurs d'une page, servis par une seule écoute de l'hôte. */
function fanOut() {
  const listeners = new Set<(m: LinkMessage) => void>();
  return {
    deliver: (raw: unknown) => {
      const m = parseLink(raw);
      if (m) [...listeners].forEach((cb) => cb(m));
    },
    listen: (cb: (m: LinkMessage) => void) => {
      listeners.add(cb);
      return () => void listeners.delete(cb);
    },
  };
}

export function tauriTransport(): LinkTransport {
  const out = fanOut();
  let resolveReady: () => void = () => {};
  const ready = new Promise<void>((r) => (resolveReady = r));
  const stopListening = listenTauri(async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const stop = await listen(EVENT, (e) => out.deliver(e.payload));
    resolveReady();
    return stop;
  });
  return {
    ready,
    listen: out.listen,
    close: stopListening,
    send: (m) => void import("@tauri-apps/api/event").then(({ emit }) => emit(EVENT, m)).catch((e) => console.warn(EVENT, String(e))),
  };
}

export function channelTransport(): LinkTransport {
  const out = fanOut();
  const channel = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel("oku-slides-agent-link");
  if (channel) channel.onmessage = (e: MessageEvent) => out.deliver(e.data);
  return {
    ready: Promise.resolve(),
    listen: out.listen,
    close: () => channel?.close(),
    // Un BroadcastChannel ne se livre pas à lui-même : la page reçoit aussi, comme avec Tauri.
    send: (m) => {
      channel?.postMessage(m);
      out.deliver(m);
    },
  };
}

let shared: LinkTransport | null = null;
export function linkTransport(): LinkTransport {
  shared ??= inTauri ? tauriTransport() : channelTransport();
  return shared;
}

export function onlyFor(to: "agent" | "main", tab: string, cb: (m: LinkMessage) => void) {
  return (m: LinkMessage) => {
    if (m.to === to && m.tab === tab) cb(m);
  };
}


export function insertWithAck(t: LinkTransport, tab: string, text: string, ms = INSERT_ACK_MS, timers: Timers = REAL_TIMERS): Promise<boolean> {
  // L'écoute de l'hôte est en place depuis le `hello` de la fenêtre (attendu sur `ready`) :
  // un `insert` ne part qu'ensuite, sur un geste de l'utilisateur.
  const id = randomId("i");
  return new Promise((resolve) => {
    let done = false;
    let handle: unknown = null;
    let stop: () => void = () => {};
    const finish = (ok: boolean) => {
      if (done) return;
      done = true;
      if (handle !== null) timers.clear(handle);
      stop();
      resolve(ok);
    };
    stop = t.listen(onlyFor("agent", tab, (m) => {
      if (m.kind === "insert-ack" && m.id === id) finish(m.ok);
    }));
    handle = timers.set(() => {
      t.send({ to: "main", kind: "insert-cancel", tab, id });
      finish(false);
    }, ms);
    t.send({ to: "main", kind: "insert", tab, id, text });
  });
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cd ui && bun test src/agent/link.test.ts && bun run typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add ui/src/agent/link.ts ui/src/agent/link.test.ts ui/src/test/link.ts
```
Message : `feat(ui) : canal entre la fenêtre principale et la fenêtre de l'agent (t-654)`

---

### Task 8: Interface — l'édition survit à la perte de focus de la fenêtre

**Files:**
- Modify: `ui/src/TextEditor.tsx` (gestionnaire `blur` de `EditorView.domEventHandlers`)
- Test: `ui/src/TextEditor.render.test.tsx`

**Interfaces:**
- Produces:
  - Un `blur` de l'éditeur pendant que le document n'a plus le focus (`document.hasFocus() === false` : une autre fenêtre ou une autre app a le focus) ne termine pas l'édition.
  - Quand le focus revient, l'éditeur est toujours là, curseur compris.
  - `finish(view, true)` appelle toujours `onCommit` (`TextEditor.tsx`, `commit`).
- Hors périmètre : `a2ui/RichTextField.tsx` garde son `blur` actuel. La dictée n'écrit jamais dans ses champs : elle écrit dans `TextEditor` (`setDictated`).

- [ ] **Step 1: Write the failing tests** (dans le `describe("TextEditor")`)

```tsx
  /** Monte un éditeur de titre ; rend le compte des validations et de quoi émettre un `blur`. */
  async function mountEditor() {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const counts = { commits: 0 };
    const nop = () => {};
    const props = { initial: "a", plain: true, label: "Titre", style: {}, palette: [], onChange: nop, onCommit: () => void counts.commits++, onCancel: nop, onNext: nop };
    await act(async () => root.render(createElement(TextEditor, props)));
    const blur = () => act(async () => host.querySelector(".cm-content")!.dispatchEvent(new FocusEvent("blur")));
    const unmount = async () => {
      await act(async () => root.unmount());
      host.remove();
    };
    return { counts, blur, unmount };
  }

  test("TextEditor blur quand la fenêtre perd le focus : l'édition continue (t-654)", async () => {
    const e = await mountEditor();
    const hasFocus = document.hasFocus;
    try {
      document.hasFocus = () => false;
      await e.blur();
      expect(e.counts.commits).toBe(0);
    } finally {
      document.hasFocus = hasFocus;
      await e.unmount();
    }
  });

  test("TextEditor blur dans la fenêtre : l'édition se termine (t-654)", async () => {
    const e = await mountEditor();
    const hasFocus = document.hasFocus;
    try {
      document.hasFocus = () => true;
      await e.blur();
      expect(e.counts.commits).toBe(1);
    } finally {
      document.hasFocus = hasFocus;
      await e.unmount();
    }
  });
```

- [ ] **Step 2: Run to verify failure**

Run: `cd ui && bun test .render. -t "perd le focus"`
Expected: FAIL (`commits` vaut 1).

- [ ] **Step 3: Implement**

```tsx
          EditorView.domEventHandlers({
            blur: (_event, target) => {
              // La fenêtre perd le focus (fenêtre de l'agent, autre app) : l'édition et son
              // curseur restent, une dictée peut encore y écrire.
              if (!document.hasFocus()) return;
              finish(target, true);
            },
          }),
```

- [ ] **Step 4: Run checks**

Run: `cd ui && bun run test && just e2e <port libre>` (au minimum `editor.cjs` et `fields.cjs`)
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ui/src/TextEditor.tsx ui/src/TextEditor.render.test.tsx
```
Message : `fix(ui) : l'édition d'un texte continue quand la fenêtre perd le focus (t-654)`

---

### Task 9: Interface — la fenêtre de l'agent

**Files:**
- Create: `ui/src/agent/target.ts`, `ui/src/agent/target.test.ts`
- Create: `ui/src/AgentWindow.tsx`, `ui/src/AgentWindow.render.test.tsx`
- Create: `ui/src/useToast.tsx`, `ui/src/useToast.render.test.tsx`
- Modify: `ui/src/main.tsx`
- Modify: `ui/src/App.tsx` (le toast passe par `useToast`)
- Modify: `ui/src/api.ts` (`WorkspaceView.agent_detached`, `WorkspaceRequest`, `workspace.detachAgent/attachAgent`)
- Modify: `ui/src/agent/keys.ts` (`shortcutAction`) ; test `ui/src/agent/keys.test.ts`
- Modify: `ui/src/Dictation.tsx` (`applyShortcut`, prop `remote`) ; test `ui/src/Dictation.render.test.tsx`
- Modify: `ui/src/agent/AgentThread.tsx`, `ui/src/agent/AgentPanel.tsx` (props `onDetach`, `onAttach`)
- Modify: `ui/src/icons.tsx` (`Icon.detach`, `Icon.attach`)
- Modify: `ui/src/styles.css` (`.agent-window`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 langues)

**Interfaces:**
- Consumes: tâches 3 (`follow`, `setDraft`), 5 (`AgentPanel` suiveur, `routeStaged`, `useSlideThumbs`, `deckVocabulary`), 6 (`useDraft`, `draftActions`), 7 (`link`).
- Produces:

```ts
// agent/target.ts
/** L'onglet actif de la fenêtre au premier plan ; null si elle est inconnue. */
export function agentTarget(view: WorkspaceView): string | null;
/** Cette page est-elle la fenêtre de l'agent ? Synchrone : label Tauri, ou `?window=agent`. */
export function isAgentWindow(): boolean;

// agent/keys.ts
/** Ce que fait ⌘D (`auto`) ou ⌘⇧D (`draft`) : dicter, ou arrêter un tour qui n'écoute pas. */
export function shortcutAction(key: "auto" | "draft", s: { agentBusy: boolean; recording: boolean }): DictationMode | "agent-cancel";

// useToast.tsx
/** Un avis passager : texte, erreur ou non, effacé après un délai. */
export function useToast(): { say: (text: string, error?: boolean) => void; node: ReactNode };

// AgentWindow.tsx
export interface AgentWindowHost {
  agent: AgentApi;
  api: TabApi;
  subscribe: typeof subscribeHost;
  /** Met la fenêtre de l'agent devant (⌘K relayé). */
  focusSelf: () => void;
  /** La fermeture système demandée (`close-requested`) ; renvoie de quoi arrêter. */
  onCloseRequested: (cb: () => void) => () => void;
  attach: () => Promise<unknown>;
}
export function AgentWindowView(props: { view: WorkspaceView | null }): JSX.Element;
export function AgentWindowFor(props: { tab: string; transport: LinkTransport; host: AgentWindowHost }): JSX.Element;
export default function AgentWindow(): JSX.Element;
```

- `workspace.detachAgent()`, `workspace.attachAgent()` : `Promise<WorkspaceView>`.
- `Dictation` : prop `remote?: { seq: number; key: LinkShortcut }`. Un numéro plus grand que le dernier vu agit une fois ; celui présent au montage n'agit pas. Les numéros viennent d'un compteur du module `AgentWindow`, qui ne repart jamais à zéro, même à travers un changement d'onglet.
- `AgentThread` et `AgentPanel` : `onDetach?: () => void` (`.agent-detach`) et `onAttach?: () => void` (`.agent-attach`).
- Clés Fluent : `ui-agent-detach`, `ui-agent-attach`, `ui-agent-window-empty`, `ui-agent-kept-in-draft`.
- Sous macOS, la fenêtre de l'agent pose la même barre de saisie que les fenêtres de l'espace de travail (`div.titlebar[data-tauri-drag-region]`, `Workspace.tsx:248`), dans tous ses états : sans elle, la fenêtre ne se déplace pas (barre de titre `Overlay`).

- [ ] **Step 1: Write the failing pure tests**

`target.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { agentTarget } from "./target";

const view = (focused: string) => ({
  generation: 1,
  focused,
  agent_detached: true,
  windows: [
    { label: "w-1", active: "t-1", tabs: [] },
    { label: "w-2", active: "t-5", tabs: [] },
  ],
});

describe("agentTarget", () => {
  test("agentTarget suit l'onglet actif de la fenêtre au premier plan", () => {
    expect(agentTarget(view("w-2"))).toBe("t-5");
  });
  test("agentTarget fenêtre au premier plan inconnue : null", () => {
    expect(agentTarget(view("w-9"))).toBeNull();
  });
});
```

`keys.test.ts` :

```ts
test("shortcutAction ⌘⇧D pendant un tour, sans écoute : arrête l'agent", () => {
  expect(shortcutAction("draft", { agentBusy: true, recording: false })).toBe("agent-cancel");
});

test("shortcutAction ⌘⇧D pendant une écoute : arrête l'écoute, jamais l'agent", () => {
  expect(shortcutAction("draft", { agentBusy: true, recording: true })).toBe("draft");
});

test("shortcutAction ⌘D : dicte", () => {
  expect(shortcutAction("auto", { agentBusy: true, recording: false })).toBe("auto");
});
```

Run: `cd ui && bun test src/agent/target.test.ts src/agent/keys.test.ts` → FAIL.

- [ ] **Step 2: Implement the pure parts**

`target.ts` :

```ts
// La fenêtre de l'agent : laquelle elle est, et quel onglet elle suit.

import { inTauri, type WorkspaceView } from "../api";
import { AGENT_LABEL } from "./link";

export function agentTarget(view: WorkspaceView): string | null {
  return view.windows.find((w) => w.label === view.focused)?.active ?? null;
}

export function isAgentWindow(): boolean {
  if (!inTauri) return new URLSearchParams(window.location.search).get("window") === AGENT_LABEL;
  const internals = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label === AGENT_LABEL;
}
```

`keys.ts` : la règle de ⌘⇧D, aujourd'hui à la fin de `dictationShortcut`, devient une fonction, que `dictationShortcut` appelle :

```ts
export function shortcutAction(key: "auto" | "draft", { agentBusy, recording }: { agentBusy: boolean; recording: boolean }): DictationMode | "agent-cancel" {
  return key === "draft" && agentBusy && !recording ? "agent-cancel" : key;
}
```

et, dans `dictationShortcut`, `if (!e.shiftKey) return "auto"; return agentBusy && !recording ? "agent-cancel" : "draft";` devient `return shortcutAction(e.shiftKey ? "draft" : "auto", { agentBusy, recording });`.

`api.ts` : `WorkspaceView` gagne `agent_detached: boolean` ; `WorkspaceRequest` gagne `| { cmd: "detach_agent" } | { cmd: "attach_agent" }` ; `workspace` gagne :

```ts
  /** L'agent passe dans sa propre fenêtre. */
  detachAgent: () => ws<WorkspaceView>({ cmd: "detach_agent" }),
  /** L'agent revient dans la colonne de droite. */
  attachAgent: () => ws<WorkspaceView>({ cmd: "attach_agent" }),
```

`main.tsx` : `const agentWindow = isAgentWindow();` avant `createRoot` ; `render` monte `agentWindow ? <AgentWindow key={generation++} /> : <Workspace key={generation++} />`. Aucun `await`.

Run: `cd ui && bun test src/agent/target.test.ts src/agent/keys.test.ts && bun run typecheck` → PASS.

- [ ] **Step 3: `useToast`, the failing test, then the hook**

`useToast.render.test.tsx` (montage comme les autres tests de rendu, minuteries de `test/timers.ts` injectées) :

```tsx
test("useToast say affiche le texte", async () => {
  const m = await mountToast();
  await act(async () => m.toast().say("enregistré"));
  expect(m.host.querySelector(".toast")?.textContent).toBe("enregistré");
});

test("useToast une erreur s'annonce comme alerte", async () => {
  const m = await mountToast();
  await act(async () => m.toast().say("refusé", true));
  expect(m.host.querySelector(".toast.error")?.getAttribute("role")).toBe("alert");
});

test("useToast l'avis s'efface après son délai", async () => {
  const m = await mountToast();
  await act(async () => m.toast().say("enregistré"));
  await act(async () => m.timers.run());
  expect(m.host.querySelector(".toast")).toBeNull();
});
```

(`mountToast()` monte un composant qui rend `useToast(timers).node` et expose le hook.)

`useToast.tsx` reprend `App.tsx:89-101` et le rendu du toast d'`App` :

```tsx
// Un avis passager en bas de l'écran : partagé par `App` et la fenêtre de l'agent.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { REAL_TIMERS, type Timers } from "./debounce";

/** Durée d'un avis, et d'une erreur (plus longue à lire). */
const SHOW_MS = 2500;
const ERROR_MS = 6000;

export function useToast(timers: Timers = REAL_TIMERS): { say: (text: string, error?: boolean) => void; node: ReactNode } {
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const handle = useRef<unknown>(null);
  useEffect(() => () => void (handle.current !== null && timers.clear(handle.current)), [timers]);
  const say = useCallback(
    (text: string, error = false) => {
      setToast({ text, error });
      // Un nouvel avis remplace le précédent : son minuteur ne doit pas l'effacer en avance.
      if (handle.current !== null) timers.clear(handle.current);
      handle.current = timers.set(() => setToast((cur) => (cur?.text === text ? null : cur)), error ? ERROR_MS : SHOW_MS);
    },
    [timers],
  );
  const node = toast ? <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div> : null;
  return { say, node };
}
```

`App.tsx` : `const { say, node: toastNode } = useToast();` remplace l'état `toast` et `say`. `{toastNode}` remplace le rendu du toast, au même endroit. `say` garde la même signature pour tous ses appelants.

- [ ] **Step 4: `Dictation` remote — failing tests, then implementation**

`Dictation.render.test.tsx` : l'utilitaire existant `mount(agentBusy = false, vocabulary = [])` gagne un troisième paramètre, `remote?: { seq: number; key: LinkShortcut }`, passé à `Dictation`. Il rend en plus `rerenderRemote(remote)`, qui rend à nouveau le même composant avec ce `remote`, et `remount(remote)`, qui démonte puis remonte un composant neuf avec ce `remote`. Ses rendus `typed()` et `cancels()` ne changent pas.

```tsx
test("Dictation remote type ouvre le brouillon une fois", async () => {
  const m = await mount();
  await m.rerenderRemote({ seq: 1, key: "type" });
  expect(m.typed()).toBe(1);
});

test("Dictation remote même numéro au rendu suivant : rien de plus", async () => {
  const m = await mount();
  await m.rerenderRemote({ seq: 1, key: "type" });
  await m.rerenderRemote({ seq: 1, key: "type" });
  expect(m.typed()).toBe(1);
});

test("Dictation remote présent au montage : rien", async () => {
  const m = await mount(false, [], { seq: 3, key: "type" });
  expect(m.typed()).toBe(0);
});

test("Dictation remote draft pendant un tour, sans écoute : arrête l'agent", async () => {
  const m = await mount(true);
  await m.rerenderRemote({ seq: 1, key: "draft" });
  expect(m.cancels()).toBe(1);
});
```

Dans `Dictation.tsx`, après `keys` (`Dictation.tsx:137`) :

```tsx
  /** Ce que fait un raccourci de dictée, au clavier ou relayé par la fenêtre principale. */
  const applyShortcut = (key: LinkShortcut) => {
    const { status: now, agentBusy: busy, toggle: switchMode } = keys.current;
    if (key === "type") return handlers.current.onType();
    const action = shortcutAction(key, { agentBusy: busy, recording: now?.state === "recording" });
    if (action === "agent-cancel") handlers.current.onAgentCancel();
    else switchMode(action);
  };

  // Un raccourci relayé (agent détaché) : chaque numéro neuf agit une fois ; celui présent au montage, jamais.
  const seen = useRef(remote?.seq ?? 0);
  useEffect(() => {
    if (!remote || remote.seq <= seen.current) return;
    seen.current = remote.seq;
    // `applyShortcut` ne lit que des refs (`keys`, `handlers`) : sa version courante suffit.
    applyShortcut(remote.key);
  }, [remote?.seq]);
```

`onKey` passe par la même décision : `typingShortcut(e)` → `applyShortcut("type")` ; sinon, si `dictationShortcut(e, …)` n'est pas nul, `applyShortcut(e.shiftKey ? "draft" : "auto")`. Échap, `preventDefault` et `stopPropagation` ne changent pas.

Run: `cd ui && bun test .render. -t "Dictation"` → PASS.

- [ ] **Step 5: Write the failing `AgentWindow` tests**

`AgentWindow.render.test.tsx` :

```tsx
import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import { memoryTransport } from "./test/link";
import type { Snapshot, Surface } from "./api";
import type { HostHandlers } from "./agent/events";
import type { LinkContext } from "./agent/link";
import type { AgentWindowHost } from "./AgentWindow";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentWindowFor, AgentWindowView } = await import("./AgentWindow");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const SNAP = { title: "Deck", slides: [], read_only: false } as unknown as Snapshot;
const READ_ONLY = { title: "Aide", slides: [], read_only: true } as unknown as Snapshot;
const UI = { slide: "s1", element: null, field: null, editing: false, widget: null };
const ctx = (over: Partial<LinkContext> = {}): LinkContext => ({ editing: false, insertTo: "titre", ui: UI, staged: null, keysOff: false, ...over });
const sheetOn = (id: string): Surface => ({ id, place: "sheet", live: true, question: false, anchor: { slide: "s1", owner: "e1" }, messages: [] });

function fakeHost(snapshot: Snapshot = SNAP) {
  const calls: string[] = [];
  let handlers: HostHandlers | null = null;
  let closing: () => void = () => {};
  const host = {
    agent: {
      follow: async () => void calls.push("follow"),
      resync: async () => void calls.push("resync"),
      setDraft: async () => {},
      prompt: async () => {},
      cancel: async () => {},
      newConversation: async () => {},
      surfaceAction: async () => {},
    },
    api: { snapshot: async () => snapshot, svg: async () => "<svg/>" },
    subscribe: (_tab: string, h: HostHandlers) => ((handlers = h), () => {}),
    focusSelf: () => void calls.push("focus"),
    onCloseRequested: (cb: () => void) => ((closing = cb), () => {}),
    attach: async () => void calls.push("attach"),
  } satisfies Partial<Record<keyof AgentWindowHost, unknown>>;
  return { host: host as unknown as AgentWindowHost, calls, handlers: () => handlers!, close: () => closing() };
}

async function mountWindow(snapshot?: Snapshot) {
  const transport = memoryTransport();
  const h = fakeHost(snapshot);
  const el = document.createElement("div");
  const root = createRoot(el);
  await act(async () => root.render(createElement(AgentWindowFor, { tab: "t-1", transport, host: h.host })));
  cleanup = async () => act(async () => root.unmount());
  const ready = () => act(async () => h.handlers().onReady?.());
  const send = (m: Parameters<typeof transport.send>[0]) => act(async () => transport.send(m));
  return { ...h, transport, el, ready, send };
}

describe("AgentWindow", () => {
  test("AgentWindow abonnée : demande follow", async () => {
    const m = await mountWindow();
    await m.ready();
    expect(m.calls.filter((c) => c === "follow")).toHaveLength(1);
  });

  test("AgentWindow abonnée : ne demande jamais resync", async () => {
    const m = await mountWindow();
    await m.ready();
    expect(m.calls).not.toContain("resync");
  });

  test("AgentWindow dit hello une seule fois, écoute du canal en place", async () => {
    const m = await mountWindow();
    expect(m.transport.sent.filter((x) => x.kind === "hello")).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
    expect(m.transport.listeners()).toBeGreaterThan(0);
  });

  // La cible d'insertion ne s'affiche que brouillon ouvert (`DictationDraft.tsx:65`) : ⌘K relayé l'ouvre.
  const openDraft = (m: Awaited<ReturnType<typeof mountWindow>>) => m.send({ to: "agent", kind: "shortcut", tab: "t-1", key: "type" });

  test("AgentWindow le contexte de son onglet donne la cible d'insertion", async () => {
    const m = await mountWindow();
    await openDraft(m);
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx({ insertTo: "le titre" }) });
    expect(m.el.querySelector(".draft-insert-to")?.textContent).toContain("le titre");
  });

  test("AgentWindow le contexte d'un autre onglet est ignoré", async () => {
    const m = await mountWindow();
    await openDraft(m);
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx({ insertTo: "le titre" }) });
    await m.send({ to: "agent", kind: "context", tab: "t-2", context: ctx({ insertTo: "ailleurs" }) });
    expect(m.el.querySelector(".draft-insert-to")?.textContent).toContain("le titre");
  });

  test("AgentWindow une fiche non posée sur la scène est rendue dans le fil", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx() });
    await act(async () => m.handlers().onMcp({ kind: "surfaces", list: [sheetOn("sheet-1")] }));
    expect(m.el.querySelector('[data-surface-id="sheet-1"]')).not.toBeNull();
  });

  test("AgentWindow la fiche posée sur la scène n'est pas rendue dans le fil", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx({ staged: "sheet-1" }) });
    await act(async () => m.handlers().onMcp({ kind: "surfaces", list: [sheetOn("sheet-1")] }));
    expect(m.el.querySelector('[data-surface-id="sheet-1"]')).toBeNull();
  });

  test("AgentWindow ⌘K relayé met la fenêtre devant", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "shortcut", tab: "t-1", key: "type" });
    expect(m.calls).toContain("focus");
  });

  test("AgentWindow fermeture système : rattache l'agent", async () => {
    const m = await mountWindow();
    await act(async () => m.close());
    expect(m.calls).toContain("attach");
  });

  test("AgentWindow onglet en lecture seule : état vide", async () => {
    const m = await mountWindow(READ_ONLY);
    await m.ready();
    expect(m.el.querySelector(".agent-window-empty")).not.toBeNull();
  });

  test("AgentWindow onglet en lecture seule : la dictée reste", async () => {
    const m = await mountWindow(READ_ONLY);
    await m.ready();
    expect(m.el.querySelector(".device")).not.toBeNull();
  });

  test("AgentWindowView sans vue reçue : chargement", async () => {
    const el = document.createElement("div");
    const root = createRoot(el);
    await act(async () => root.render(createElement(AgentWindowView, { view: null })));
    cleanup = async () => act(async () => root.unmount());
    expect(el.querySelector(".agent-window-loading")).not.toBeNull();
  });

  test("AgentWindowView sans onglet suivi : état vide", async () => {
    const el = document.createElement("div");
    const root = createRoot(el);
    const view = { generation: 1, focused: "w-9", agent_detached: true, windows: [] };
    await act(async () => root.render(createElement(AgentWindowView, { view })));
    cleanup = async () => act(async () => root.unmount());
    expect(el.querySelector(".agent-window-empty")).not.toBeNull();
  });
});
```

Les sélecteurs viennent du code actuel :
- `data-surface-id` est posé par `AgentSurfaceBox.tsx:84` et `AgentThread.tsx:116/123` ;
- `.draft-insert-to` vient de `DictationDraft.tsx:128` ;
- `.device` est la classe racine de `Dictation` (`Dictation.tsx:179`).

Run: `cd ui && bun test .render. -t "AgentWindow|useToast"` → FAIL.

- [ ] **Step 6: Implement `AgentWindow.tsx`**

Exporter trois composants. Un compteur de module numérote les raccourcis relayés :

```tsx
/** Numéro du prochain raccourci relayé : jamais remis à zéro, même quand l'onglet suivi change. */
let remoteSeq = 0;
const FOLLOW: ScopeSource = { kind: "follow" };
const DOCK_ONLY: QuickTarget["kind"][] = ["dock"];
const NO_UI: AgentUi = { slide: null, element: null, field: null, editing: false, widget: null };
const NOOP = () => {};
/** Délai avant de relire l'instantané quand le contexte change (vocabulaire, miniatures). */
const REREAD_MS = 500;

/** La barre de saisie de la fenêtre (macOS, barre de titre fondue) : sans elle, elle ne se déplace pas. */
const TitleBar = () => (inTauri && isMac ? <div className="titlebar" data-tauri-drag-region /> : null);
```

**`AgentWindowView({ view })`**
- `view === null` : `<div className="agent-window agent-window-loading"><TitleBar /><p>{t("ui-loading")}</p></div>`.
- `agentTarget(view) === null` : `<div className="agent-window agent-window-empty"><TitleBar /><p>{t("ui-agent-window-empty")}</p></div>`.
- Sinon : `<TauriAgentWindow key={tab} tab={tab} />`.

`TauriAgentWindow({ tab })` construit l'hôte **une fois par onglet**, pour qu'un évènement `workspace` ne réabonne pas le panneau :

```tsx
function TauriAgentWindow({ tab }: { tab: string }) {
  const [host] = useState(() => tauriHost(tab));
  return <AgentWindowFor tab={tab} transport={linkTransport()} host={host} />;
}
```

`tauriHost(tab)` fournit :
- `agent: agentApi(tab)`, `api: tabApi(tab)`, `subscribe: subscribeHost`, `attach: workspace.attachAgent` ;
- `focusSelf` : `import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().setFocus())`, sans effet hors Tauri ;
- `onCloseRequested` : écoute `close-requested` sur la fenêtre courante, comme `Workspace.tsx:163` (`getCurrentWebviewWindow().listen`), via `listenTauri`, qui rend de quoi arrêter. Sans effet hors Tauri.

**`AgentWindowFor({ tab, transport, host })`** : la vue d'un onglet.

État :
- `[context, setContext] = useState<LinkContext | null>(null)` ;
- `[snap, setSnap] = useState<Snapshot | null>(null)` ;
- `[remote, setRemote] = useState<{ seq: number; key: LinkShortcut } | undefined>()` ;
- `[status, setStatus] = useState(IDLE_AGENT)` ;
- `handle = useRef<AgentHandle>(null)` ;
- `[slots] = useState(() => ({ sheet: createSlot(), drawer: createSlot(), modal: createSlot() }))`, jamais affichés : fiches, tiroir et fenêtre restent dans la fenêtre principale ;
- `const { say, node: toastNode } = useToast()`.

Brouillon et sorties :
- `draft = useDraft(host.agent.setDraft)` ;
- `insertOrKeep = async (text) => { const ok = await insertWithAck(transport, tab, text); if (!ok) say(t("ui-agent-kept-in-draft"), true); return ok; }` ;
- `actions = draftActions({ draft, editing: context?.editing ?? false, insert: insertOrKeep, prompt: (text) => handle.current?.prompt(text) ?? Promise.resolve(false) })`. Un refus remet le texte au brouillon (`draftActions`).
- `attach = useCallback(() => void draft.flush().then(() => host.attach()), [draft.flush, host])` : la dernière frappe est chez l'hôte **avant** que la fenêtre se ferme.

Effets :
- **Canal**, effet `[tab, transport]` :

```ts
    let live = true;
    const stop = transport.listen(onlyFor("agent", tab, (m) => {
      if (m.kind === "context") setContext(m.context);
      if (m.kind === "shortcut") {
        if (m.key === "type") host.focusSelf();
        setRemote({ seq: ++remoteSeq, key: m.key });
      }
    }));
    void transport.ready.then(() => live && transport.send({ to: "main", kind: "hello", tab }));
    return () => {
      live = false;
      stop();
    };
```

- **Fermeture système**, effet `[host, attach]` : `return host.onCloseRequested(attach);`.
- **Instantané** : `reread = useCallback(() => void host.api.snapshot().then(setSnap).catch((e) => say(String(e), true)), [host, say])`, appelé par `onChanged` du panneau (à l'abonnement prêt et après un outil de l'agent). Une modification faite dans la fenêtre principale n'émet pas `changed` : relire aussi, au plus une fois par `REREAD_MS`, quand le contexte change. Utiliser `const rereadSoon = useMemo(() => debounced(reread, REREAD_MS), [reread])`, appelé dans un effet sur `JSON.stringify(context)`, et oublié au démontage : `useEffect(() => () => rereadSoon.cancel(), [rereadSoon])` (un changement d'onglet ne relit pas l'ancien).

Imports d'`AgentWindow.tsx` :

```tsx
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { agentApi, inTauri, isMac, listenTauri, tabApi, workspace, type AgentUi, type Snapshot, type TabApi, type WorkspaceView } from "./api";
import { newest } from "./latest";
import { t } from "./i18n";
import { debounced } from "./debounce";
import { deckVocabulary } from "./draft";
import { useSlideThumbs } from "./useSlideThumbs";
import { useToast } from "./useToast";
import Dictation from "./Dictation";
import { DictationDraft } from "./DictationDraft";
import { AgentPanel, IDLE_AGENT, type AgentApi, type AgentHandle, type ScopeSource } from "./agent/AgentPanel";
import { draftActions } from "./agent/draftActions";
import { subscribeHost } from "./agent/events";
import { insertWithAck, linkTransport, onlyFor, type LinkContext, type LinkShortcut, type LinkTransport } from "./agent/link";
import { createSlot } from "./agent/slot";
import { routeStaged } from "./agent/surfaces";
import { agentTarget } from "./agent/target";
import { useDraft } from "./agent/useDraft";
import type { DictationMode } from "./agent/keys";
import type { QuickTarget } from "./agent/useQuickKeys";
```

Valeurs dérivées :
- `thumbs = useSlideThumbs(host.api, snap?.slides)` ;
- `vocabulary = useMemo(() => deckVocabulary(snap), [snap])` ;
- `route = useCallback((list) => routeStaged(list, context?.staged ?? null), [context?.staged])`.

Rendu :

```tsx
    const dictation = (onText: (text: string, mode: DictationMode) => void) => (
      <Dictation
        destination={t(context?.editing ? "ui-dictation-to-cursor" : "ui-dictation-to-draft")}
        vocabulary={vocabulary} onText={onText} onError={(m) => say(m, true)}
        agentBusy={status.busy} onAgentCancel={() => handle.current?.cancel()}
        onType={draft.type} remote={remote}
      />
    );
    if (snap?.read_only)
      return (
        <div className="agent-window agent-window-empty">
          <TitleBar />
          <p>{t("ui-agent-window-empty")}</p>
          {/* Lecture seule : pas de brouillon (rien ne l'afficherait) ; le texte part à la fenêtre principale, qui le refuse. */}
          {dictation((text) => void insertWithAck(transport, tab, text))}
          {toastNode}
        </div>
      );
    return (
      <div className="agent-window">
        <TitleBar />
        <AgentPanel
          tab={tab} agent={host.agent} scope={FOLLOW} route={route} ui={context?.ui ?? NO_UI}
          keysOff={context?.keysOff ?? false} keyPlaces={DOCK_ONLY} thread thumbs={thumbs} slots={slots} handle={handle}
          onHostAction={() => false} onStatus={setStatus} onChanged={reread} onFocus={NOOP} onWidgetTouched={NOOP}
          onDraft={draft.onHost} say={say} subscribe={host.subscribe} onAttach={attach}
        />
        <DictationDraft
          text={draft.state.text} dictations={draft.state.dictations} typing={draft.state.typing}
          busy={status.busy} notice={status.notice} ready={status.ready}
          insertTo={context?.insertTo ?? null}
          onChange={draft.change}
          onInsert={(text) => void actions.insertDraft(text)}
          onSend={(text) => void actions.sendDraft(text)}
          onDiscard={draft.close}
        />
        {dictation(actions.onDictated)}
        {toastNode}
      </div>
    );
```

En lecture seule, tout texte dicté part à la fenêtre principale (`insert`), qui le refuse comme aujourd'hui (la session est en lecture seule) : jamais au brouillon, qui ne serait pas affiché, et dont l'écho ne reviendrait pas (aucun abonnement ici). L'envoi d'une consigne reste désactivé tant que la portée n'est pas annoncée : `status.ready` est faux, et `DictationDraft` grise « Envoyer » (t-211).

**`default export AgentWindow()`** lit la vue : `workspace.view()`, puis l'évènement Tauri `workspace`, comme `Workspace.tsx:72-95` (avec `newest()`). Il rend `<AgentWindowView view={view} />`.

**`AgentThread.tsx`**, dans l'en-tête, après « nouvelle conversation » :

```tsx
          {onDetach && <button type="button" className="token ghost agent-detach" aria-label={t("ui-agent-detach")} title={t("ui-agent-detach")} onClick={onDetach}>{Icon.detach}</button>}
          {onAttach && <button type="button" className="token ghost agent-attach" aria-label={t("ui-agent-attach")} title={t("ui-agent-attach")} onClick={onAttach}>{Icon.attach}</button>}
```

`AgentPanel` transmet ses props optionnelles `onDetach` et `onAttach` à `AgentThread`.

**`icons.tsx`** (même `stroke` que `newChat`) :

```tsx
  detach: stroke(<><path d="M14 4h6v6" /><path d="M20 4l-8 8" /><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></>),
  attach: stroke(<><path d="M10 20H4v-6" /><path d="M4 20l8-8" /><path d="M6 10V5a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1h-5" /></>),
```

**`styles.css`**, à côté de `.notes`, avec les jetons de `.notes` et de `body` :

```css
.agent-window { height: 100vh; box-sizing: border-box; display: flex; flex-direction: column; gap: 22px; padding: 0 var(--space-6) var(--space-6); overflow-y: auto; }
.agent-window > .titlebar { margin: 0 calc(-1 * var(--space-6)); }
.agent-window .agent-thread { flex: 1; min-height: 0; }
.agent-window-empty, .agent-window-loading { justify-content: center; align-items: center; }
```

Fluent, `ui.ftl` des 20 langues (fr et en ci-dessous ; les autres traduites) :

```
ui-agent-detach = Ouvrir l'agent dans une fenêtre séparée
ui-agent-attach = Ramener l'agent dans la fenêtre principale
ui-agent-window-empty = Pas d'agent pour cet onglet
ui-agent-kept-in-draft = Rien où écrire : texte gardé dans le brouillon
```
```
ui-agent-detach = Open the agent in a separate window
ui-agent-attach = Bring the agent back to the main window
ui-agent-window-empty = No agent for this tab
ui-agent-kept-in-draft = Nowhere to write: text kept in the draft
```

- [ ] **Step 7: Run checks**

Run: `cd ui && bun run typecheck && bun run test && just e2e <port libre>`
Expected: PASS, y compris `i18n-keys.test.ts` et les boutons à pictogramme seul (`expectIconOnly`). Le toast d'`App` est inchangé à l'écran.

- [ ] **Step 8: Commit**

```bash
git add ui/src/agent/target.ts ui/src/agent/target.test.ts ui/src/agent/keys.ts ui/src/agent/keys.test.ts ui/src/AgentWindow.tsx ui/src/AgentWindow.render.test.tsx ui/src/useToast.tsx ui/src/useToast.render.test.tsx ui/src/main.tsx ui/src/App.tsx ui/src/api.ts ui/src/Dictation.tsx ui/src/Dictation.render.test.tsx ui/src/agent/AgentThread.tsx ui/src/agent/AgentPanel.tsx ui/src/icons.tsx ui/src/styles.css crates/oku-slides-i18n/src/locales
```
Message : `feat(ui) : fenêtre de l'agent — fil, brouillon et dictée de l'onglet au premier plan (t-654)`

---

### Task 10: Interface — `App` avec l'agent détaché

**Files:**
- Create: `ui/src/agent/useMainLink.ts`, `ui/src/agent/useMainLink.render.test.tsx`
- Modify: `ui/src/App.tsx` (prop `agentDetached`, colonne, contexte, bouton ⧉, touches rapides sans le dock)
- Modify: `ui/src/agent/AgentPanel.tsx` (`AgentStatusView.sheetId`, `IDLE_AGENT.sheetId: null`)
- Modify: `ui/src/Workspace.tsx` (passe `agentDetached={view.agent_detached}` à `App`)
- Modify: `ui/src/styles.css` (`.table.agent-detached`)

**Interfaces:**
- Consumes: `LinkTransport`, `LinkContext`, `onlyFor` (tâche 7) ; `workspace.detachAgent` (tâche 9) ; `useDictation(...).insertText: (text) => boolean` (tâche 6).
- Produces:

```ts
export interface MainLinkInput {
  tab: string;
  /** L'agent est dans sa fenêtre : publier, répondre, relayer. Sinon rien. */
  detached: boolean;
  context: LinkContext;
  /** Écrit le texte au curseur, dans la zone sélectionnée ou en nouveau bloc ; `false` : nulle part. */
  insert: (text: string) => boolean;
  transport: LinkTransport;
}
/** Agent détaché : publie le contexte, répond à hello et insert, relaie ⌘D / ⌘⇧D / ⌘K bruts. */
export function useMainLink(input: MainLinkInput): void;
```

Limite connue, documentée dans la spec : Échap dans la fenêtre principale n'arrête pas une écoute de la fenêtre de l'agent ; ⌘D l'arrête.

- [ ] **Step 1: Write the failing tests** (`useMainLink.render.test.tsx`)

```tsx
import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { memoryTransport } from "../test/link";
import type { LinkContext } from "./link";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useMainLink } = await import("./useMainLink");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const context: LinkContext = { editing: false, insertTo: "nouveau bloc", ui: { slide: "s1", element: null, field: null, editing: false, widget: null }, staged: null, keysOff: false };

async function mountLink({ detached = true, insert = (_t: string) => true } = {}) {
  const transport = memoryTransport();
  const written: string[] = [];
  const Probe = ({ d, c }: { d: boolean; c: LinkContext }) => {
    useMainLink({ tab: "t-1", detached: d, context: c, insert: (t) => (written.push(t), insert(t)), transport });
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe, { d: detached, c: context })));
  cleanup = async () => act(async () => root.unmount());
  const rerender = (d: boolean, c: LinkContext) => act(async () => root.render(createElement(Probe, { d, c })));
  return { transport, written, rerender };
}

const press = (init: KeyboardEventInit) => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", init)));

describe("useMainLink", () => {
  test("useMainLink détaché : publie le contexte de l'onglet", async () => {
    const m = await mountLink();
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "context", tab: "t-1", context });
  });

  test("useMainLink détaché : un contexte changé est republié", async () => {
    const m = await mountLink();
    const next = { ...context, insertTo: "titre" };
    await m.rerender(true, next);
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "context", tab: "t-1", context: next });
  });

  test("useMainLink détaché : hello redonne le contexte", async () => {
    const m = await mountLink();
    m.transport.sent.length = 0;
    await act(async () => m.transport.send({ to: "main", kind: "hello", tab: "t-1" }));
    expect(m.transport.sent.filter((x) => x.kind === "context")).toHaveLength(1);
  });

  test("useMainLink détaché : insert écrit puis accuse ok", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-7", text: "dicté" }));
    expect(m.written).toEqual(["dicté"]);
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "insert-ack", tab: "t-1", id: "i-7", ok: true });
  });

  test("useMainLink détaché : insert sans zone où écrire accuse refus", async () => {
    const m = await mountLink({ insert: () => false });
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-8", text: "x" }));
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "insert-ack", tab: "t-1", id: "i-8", ok: false });
  });

  test("useMainLink détaché : insert annulé avant d'arriver n'est pas écrit", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert-cancel", tab: "t-1", id: "i-9" }));
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-9", text: "tard" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink détaché : insert d'un autre onglet ignoré", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-2", id: "i-1", text: "x" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink détaché : ⌘K est relayé brut", async () => {
    const m = await mountLink();
    await press({ key: "k", code: "KeyK", metaKey: true });
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "shortcut", tab: "t-1", key: "type" });
  });

  test("useMainLink détaché : ⌘⇧D est relayé brut, jamais en arrêt de l'agent", async () => {
    const m = await mountLink();
    await press({ key: "D", code: "KeyD", metaKey: true, shiftKey: true });
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "shortcut", tab: "t-1", key: "draft" });
  });

  test("useMainLink rattaché : hello ne publie rien", async () => {
    const m = await mountLink({ detached: false });
    await act(async () => m.transport.send({ to: "main", kind: "hello", tab: "t-1" }));
    expect(m.transport.sent.filter((x) => x.to === "agent")).toEqual([]);
  });

  test("useMainLink rattaché : insert n'écrit rien", async () => {
    const m = await mountLink({ detached: false });
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-1", text: "x" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink rattaché : ⌘K n'est pas relayé", async () => {
    const m = await mountLink({ detached: false });
    await press({ key: "k", code: "KeyK", metaKey: true });
    expect(m.transport.sent.filter((x) => x.kind === "shortcut")).toEqual([]);
  });

  test("useMainLink détaché puis rattaché : l'écoute est retirée", async () => {
    const m = await mountLink();
    await m.rerender(false, context);
    expect(m.transport.listeners()).toBe(0);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd ui && bun test .render. -t useMainLink`
Expected: FAIL (module absent).

- [ ] **Step 3: Implement `useMainLink.ts`**

```ts
// Côté fenêtre principale, l'agent détaché : contexte publié, textes à écrire, raccourcis relayés.
// Les raccourcis partent bruts : la fenêtre de l'agent les interprète avec son état (écoute, tour).

import { useEffect, useRef } from "react";
import { dictationShortcut, typingShortcut } from "./keys";
import { onlyFor, type LinkContext, type LinkShortcut, type LinkTransport } from "./link";

export function useMainLink({ tab, detached, context, insert, transport }: MainLinkInput): void {
  const latest = useRef({ context, insert });
  latest.current = { context, insert };
  // Clé du contexte, calculée seulement détaché : une égalité profonde en guise de dépendance.
  const contextKey = detached ? JSON.stringify(context) : "";
  useEffect(() => {
    if (detached) transport.send({ to: "agent", kind: "context", tab, context: latest.current.context });
  }, [detached, tab, contextKey, transport]);
  useEffect(() => {
    if (!detached) return;
    // Insertions annulées par la fenêtre de l'agent avant d'arriver ici ; bornées : une annulation
    // ne sert qu'à l'insertion qui la suit de près.
    const cancelled: string[] = [];
    return transport.listen(onlyFor("main", tab, (m) => {
      if (m.kind === "hello") transport.send({ to: "agent", kind: "context", tab, context: latest.current.context });
      if (m.kind === "insert-cancel") {
        cancelled.push(m.id);
        if (cancelled.length > 64) cancelled.shift();
      }
      if (m.kind === "insert") {
        const at = cancelled.indexOf(m.id);
        if (at >= 0) {
          cancelled.splice(at, 1);
          return;
        }
        const ok = latest.current.insert(m.text);
        transport.send({ to: "agent", kind: "insert-ack", tab, id: m.id, ok });
      }
    }));
  }, [detached, tab, transport]);
  useEffect(() => {
    if (!detached) return;
    const onKey = (e: KeyboardEvent) => {
      const key: LinkShortcut | null = typingShortcut(e)
        ? "type"
        : dictationShortcut(e, { agentBusy: false, recording: false }) === null
          ? null
          : e.shiftKey ? "draft" : "auto";
      if (!key) return;
      e.preventDefault();
      e.stopPropagation();
      transport.send({ to: "agent", kind: "shortcut", tab, key });
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [detached, tab, transport]);
}
```

(`dictationShortcut` ne sert ici qu'à reconnaître ⌘D / ⌘⇧D. Avec `agentBusy: false`, il ne rend jamais `agent-cancel`.)

- [ ] **Step 4: Wire `App.tsx`**

- Prop `agentDetached?: boolean`, passée par `Workspace.tsx` (`agentDetached={view.agent_detached}` sur `<App … />`).
- `const transport = useMemo(() => linkTransport(), []);`.
- Contexte : `const linkContext = useMemo<LinkContext>(() => ({ editing: editing !== null, insertTo: dictation.insertTo, ui, staged: agentStatus.sheet ? stagedId : null, keysOff: !!editing || picking || widgetActive }), [...])`.
- `stagedId` est l'identifiant de la fiche posée : `agentStatus.sheetId`. `AgentStatusView` gagne `sheetId: string | null`, rempli par `AgentPanel` à côté de `sheet` (`sheet?.id ?? null`) et ajouté aux dépendances de son `useLayoutEffect` ; `IDLE_AGENT` (`AgentPanel.tsx:52`) gagne `sheetId: null`.
- `useMainLink({ tab, detached: !!agentDetached, context: linkContext, insert: dictation.insertText, transport })`.
- Détaché :
  - `AgentPanel` reste monté avec `thread={false}` : fiches, tiroir, fenêtre, touches rapides et portée restent ici. Ses touches rapides ne visent plus le dock, invisible ici : `keyPlaces={agentDetached ? MAIN_PLACES : undefined}`, avec `const MAIN_PLACES: QuickTarget["kind"][] = ["sheet", "drawer", "modal"];` au niveau du module (`import type { QuickTarget } from "./agent/useQuickKeys";`) ;


  - `DictationDraft` et `Dictation` ne sont pas montés.
- Rattaché : `AgentPanel` reçoit `onDetach={inTauri ? () => void workspace.detachAgent() : undefined}`.
- La classe de la grille : `table` plus ` agent-detached` quand `agentDetached && !themeActive`, ajoutée là où `App` compose déjà la classe de `.table`.

`styles.css`, à côté de `.table:has(> .help-bar)` :

```css
.table.agent-detached { grid-template-columns: var(--pile-w) minmax(0, 1fr) var(--space-6); }
.table.agent-detached > .notes, .table.agent-detached > .col-divider[data-side="notes"] { display: none; }
```

- [ ] **Step 5: Run checks**

Run: `cd ui && bun run typecheck && bun run test && wc -l src/App.tsx && just e2e <port libre>`
Expected: PASS ; `App.tsx` < 1000 lignes.

- [ ] **Step 6: Commit**

```bash
git add ui/src/agent/useMainLink.ts ui/src/agent/useMainLink.render.test.tsx ui/src/agent/AgentPanel.tsx ui/src/App.tsx ui/src/Workspace.tsx ui/src/styles.css
```
Message : `feat(ui) : la fenêtre principale sans agent quand il est détaché, son contexte relayé (t-654)`

---

### Task 11: e2e — fenêtre de l'agent

**Files:**
- Create: `ui/e2e/agent-window.cjs`
- Modify: `ui/package.json` (script `e2e`, après `agent-scopes.cjs`)

**Interfaces:**
- Consumes : toutes les tâches précédentes.
- Le serveur de dev ne diffuse pas l'évènement `workspace`. La fenêtre principale relit donc la vue en se rechargeant après `detach_agent` ou `attach_agent`. C'est la seule raison des rechargements. Le bug du « brouillon ignoré après une frappe » est couvert par `useDraft`.
- Un brouillon restauré s'affiche replié (`.draft-note`) ; `.draft-text` n'existe qu'en saisie.

- [ ] **Step 1: Write the e2e**

```js
// Agent détaché : la fenêtre principale perd sa colonne, la fenêtre de l'agent (ici une page
// `?window=agent` ; le canal passe par BroadcastChannel) a le fil, le brouillon et la dictée.
// Contre `oku-slides serve` lancé avec le faux agent (just e2e s'en charge).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const INSERTED = 'inséré par la fenêtre agent';

/** Le deck de t-1 contient-il `text` ? */
const deckHas = (p, text) =>
  p.evaluate(async (needle) => {
    const r = await fetch('/api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ tab: 't-1', req: { cmd: 'snapshot' } }) });
    return JSON.stringify(await r.json()).includes(needle);
  }, text);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const ctx = await b.newContext({ viewport: { width: 1440, height: 900 } });
  const main = await ctx.newPage();
  const errors = [];
  main.on('pageerror', (e) => errors.push(e.message));
  try {
    await main.goto(URL);
    await post(main, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    await post(main, '/api/workspace', { cmd: 'detach_agent' });
    await main.reload();
    await main.waitForSelector('.stage img');

    // 1. La fenêtre principale n'a plus le fil.
    assert.equal(await main.locator('.agent-thread').count(), 0);

    // 2. La fenêtre de l'agent l'a, avec sa dictée ; on part d'une conversation vide.
    const agent = await ctx.newPage();
    agent.on('pageerror', (e) => errors.push(e.message));
    await agent.goto(`${URL}?window=agent`);
    await agent.waitForSelector('.agent-window .agent-thread');
    await agent.locator('.agent-new').click();
    await agent.waitForSelector('.agent-thread.vacant');

    // 3. Une consigne écrite dans le brouillon de la fenêtre de l'agent ouvre un tour.
    await agent.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').fill('depuis la fenêtre agent');
    await agent.locator('.draft-send').click();
    await agent.locator('.agent-turn', { hasText: 'depuis la fenêtre agent' }).waitFor();
    await agent.waitForSelector('.draft-text', { state: 'detached' });

    // 4. ⌘K dans la fenêtre principale ouvre le brouillon de la fenêtre de l'agent.
    await main.bringToFront();
    await main.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').waitFor();

    // 5. « Insérer » depuis la fenêtre de l'agent écrit dans la zone sélectionnée de la fenêtre principale.
    const order = (await post(main, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).slides.map((s) => s.id);
    await main.click(`.thumb >> nth=${order.indexOf('s-constat')}`);
    await main.waitForSelector('.overlay .target');
    await main.locator('.overlay .target').first().click({ position: { x: 4, y: 4 } });
    await main.waitForSelector('.overlay .target.selected');
    await agent.locator('.draft-text').fill(INSERTED);
    await agent.locator('.draft-insert').click();
    await main.waitForFunction(async (needle) => {
      const r = await fetch('/api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ tab: 't-1', req: { cmd: 'snapshot' } }) });
      return JSON.stringify(await r.json()).includes(needle);
    }, INSERTED);
    // Annulé, pour les scripts suivants : le deck ne contient plus le texte.
    await main.bringToFront();
    await main.keyboard.press('ControlOrMeta+z');
    await main.waitForFunction(async (needle) => {
      const r = await fetch('/api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ tab: 't-1', req: { cmd: 'snapshot' } }) });
      return !JSON.stringify(await r.json()).includes(needle);
    }, INSERTED);
    assert.equal(await deckHas(main, INSERTED), false);

    // 6. Le brouillon survit au rattachement : « Rattacher » envoie la frappe en attente, puis rattache.
    await agent.keyboard.press('ControlOrMeta+k');
    await agent.locator('.draft-text').fill('brouillon gardé');
    await agent.locator('.agent-attach').click();
    await main.reload();
    await main.waitForSelector('.notes .agent-thread');
    await main.locator('.draft-note', { hasText: 'brouillon gardé' }).waitFor();

    // Rangement : brouillon fermé, conversation vidée.
    await main.locator('.draft-discard').click();
    await main.locator('.agent-new').click();
    assert.deepEqual(errors, []);
  } finally {
    await post(main, '/api/agent', { cmd: 'set_draft', tab: 't-1', text: null, origin: 'e2e' }).catch(() => {});
    await post(main, '/api/workspace', { cmd: 'attach_agent' }).catch(() => {});
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

`deckHas` sert à la dernière vérification de l'étape 5 ; les deux `waitForFunction` gardent leur requête en ligne, parce qu'une fonction passée à `waitForFunction` s'exécute dans la page et ne voit pas les fonctions de Node.

- [ ] **Step 2: Run it**

Run: `just e2e <port libre>`
Expected: PASS, y compris `agent-window.cjs`. Les scripts suivants partent d'un t-1 rattaché, sans brouillon.

- [ ] **Step 3: Commit**

```bash
git add ui/e2e/agent-window.cjs ui/package.json
```
Message : `test(e2e) : fenêtre de l'agent détachée, insertion relayée et brouillon gardé au rattachement (t-654)`

---

### Task 12: Vérification dans l'app et clôture

**Files:**
- Modify: `docs/aide/fr.json`, `docs/aide/en.json` (l'aide utilisateur : détacher l'agent)
- Check: `docs/superpowers/specs/2026-10-09-agent-detachable-design.md` (section « Précisions du plan », comparée au code livré ; attendu : rien à changer, sinon la corriger dans le même commit)

- [ ] **Step 1: Aide utilisateur**

Dans `docs/aide/{fr,en}.json`, dans la section de l'agent, ajouter une entrée « Détacher l'agent ». Elle couvre trois points : le bouton ⧉, le second écran et le menu Fenêtre sous macOS ; « Rattacher » ou la fermeture de la fenêtre ; le fait que l'agent suit l'onglet actif. Suivre la structure des entrées voisines.

- [ ] **Step 2: Checks complets**

Run: `just check && (cd ui && bun run typecheck && bun run test) && just e2e <port libre>`
Expected: tout PASS.

- [ ] **Step 3: Essai dans l'app Tauri** (données isolées : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` vers un dossier temporaire)

Noter le résultat de chaque point :
1. ⧉ ouvre la fenêtre `agent` (420×800) et fait disparaître la colonne de droite ; la case du menu Fenêtre est cochée (macOS).
2. La fenêtre de l'agent suit quand on change d'onglet, puis quand on change de fenêtre d'espace de travail. Cliquer dans la fenêtre de l'agent ne change pas l'onglet suivi.
3. Le micro fonctionne dans la fenêtre de l'agent (autorisation, écoute, transcription ; voir t-671).
4. Pendant l'édition d'un texte, cliquer le micro de la fenêtre de l'agent puis dicter : le texte arrive au curseur.
5. Sans rien de sélectionné sur une slide de titre, « Insérer » affiche le toast « texte gardé dans le brouillon ».
6. Pendant un tour, ⌘⇧D dans la fenêtre principale **pendant une écoute** l'arrête et n'annule pas l'agent.
7. Déplacer la fenêtre de l'agent sur un second écran, quitter avec ⌘Q, relancer : elle revient détachée, au même endroit (t-672).
8. Fermer la fenêtre de l'agent (bouton système) juste après une frappe : l'agent revient dans la colonne, avec la frappe dans le brouillon.
9. Le menu Fenêtre › « Agent dans une fenêtre séparée » détache, puis rattache.
10. Sous macOS, la fenêtre de l'agent se déplace par sa barre du haut (`.titlebar`) et se pose sur un second écran.
11. Pendant un tour, avec l'agent détaché, une question du dock ne reçoit pas les touches 1–4 depuis la fenêtre principale ; elle les reçoit depuis la fenêtre de l'agent.

- [ ] **Step 4: Clôture**

Marquer t-654 et ses sous-tâches faites avec `oku cli task done`. Suivre les tâches différées : t-669, t-670, t-671, t-672 et t-673.

Le commit qui rassemble le travail sur `main` (squash) est un `feat!` : les types de l'IPC (`AgentEvent`, `AgentCommand`, `McpEvent`) et les props d'`AgentPanel` changent.
