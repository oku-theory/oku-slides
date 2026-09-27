# Agent vocal dans l'espace de travail — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Brancher l'agent vocal sur l'espace de travail de `main` : un sous-processus ACP pour l'app, un agent (session ACP, outils MCP, surfaces, fil) par onglet, tours simultanés entre onglets ; reformulation lue par le parseur markdown de l'interface.

**Architecture:** Avant la fusion, l'agent apprend les onglets sur la branche (un seul onglet `t-1` côté hôtes) : `TabSession` remplace `SharedSession`, une connexion ACP partagée (`Link`) route les notifications par `SessionId` vers des acteurs par onglet, la `Stack` tient la table des onglets et relaie des enveloppes `{tab, event}`. Puis `main` est fusionnée : les hôtes branchent la pile sur `Workspace` (`Workspace::with_session`), l'interface de chaque onglet filtre ses enveloppes. Enfin reformulation en paragraphes, reprise des modifications de l'agent, bout à bout à deux onglets.

**Tech Stack:** Rust (tokio, agent-client-protocol 2.2, rust-mcp-sdk 1.1, snafu, speculoos, nextest), Tauri 2, React + TypeScript (Bun test, happy-dom, Playwright).

**Spec:** `docs/superpowers/specs/2026-09-27-agent-onglets-design.md` (et `2026-09-26-agent-vocal-design.md`).

**Tâche Oku :** t-224 (parent t-89) ; une sous-tâche par Task ci-dessous.

## Global Constraints

- Pas de `.unwrap()` / `.expect()` hors tests ; `#[expect(..., reason)]` plutôt que `#[allow]` ; aucun `#[allow(dead_code)]` hors `tests/common`.
- Ordre des verrous : espace de travail, puis session ; jamais les deux tenus ensemble ; jamais un verrou `std` tenu à travers un `.await`.
- Tout texte vu par l'utilisateur passe par Fluent, dans les 20 langues (`crates/oku-slides-i18n/src/locales/<l>/*.ftl`).
- Tests Rust : `<unité>_<scénario>_<résultat>`, speculoos, fichiers `_tests.rs` à côté ; régressions dans `tests/regressions/`.
- Tests UI : `bun run test` (deux passes : unitaires, puis `*.render.test.tsx` avec happy-dom).
- Commit : `git add` dans une commande à part, vérifier `git status`, puis `git commit` ; message `type(portée) : … (t-224)` ; jamais de ligne `Co-Authored-By`.
- Fichiers : cible 500 lignes, limite 1000 ; `ARCHITECTURE.md` de chaque crate touché à jour (< 50 lignes).
- Vérifications de chaque tâche Rust : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p <crates touchés>`.

---

### Task 1 : `Sessions` et `TabSession` (session crate)

**Files:**
- Create: `crates/oku-slides-session/src/agent/tabs.rs`, `crates/oku-slides-session/src/agent/tabs_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs` (module + ré-exports), `crates/oku-slides-session/src/lib.rs` (doc de la racine), `crates/oku-slides-session/ARCHITECTURE.md`

**Interfaces:**
- Produces:
  - `pub trait Sessions: Send + Sync + std::fmt::Debug { fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()>; }`
  - `impl Sessions for std::sync::Mutex<Session>` : une seule session, quel que soit l'onglet (tests, hôtes à un document).
  - `#[derive(Clone, Debug)] pub struct TabSession { tab: String, sessions: Arc<dyn Sessions> }` avec `new(tab: impl Into<String>, sessions: Arc<dyn Sessions>) -> Self`, `tab(&self) -> &str`, `with<R>(&self, f: impl FnOnce(&mut Session) -> R) -> crate::Result<R>` (bloquant : à appeler hors du fil async).
  - ré-exportés : `oku_slides_session::agent::{Sessions, TabSession}`.

- [ ] **Step 1 : tests qui échouent** (`tabs_tests.rs`)

```rust
use std::sync::{Arc, Mutex};

use speculoos::prelude::*;

use super::{Sessions, TabSession};
use crate::test_support::session;

#[derive(Debug)]
struct Closed;

impl Sessions for Closed {
    fn with(&self, tab: &str, _f: &mut dyn FnMut(&mut crate::Session)) -> crate::Result<()> {
        Err(crate::error::UnknownTabSnafu { id: tab }.build().into())
    }
}

#[test]
fn tab_session_with_runs_on_the_session_and_returns_its_value() {
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session())));
    let slides = tab.with(|s| s.deck().slides.len());
    assert_that!(slides).is_ok().is_greater_than(0);
    assert_that!(tab.tab()).is_equal_to("t-1");
}

#[test]
fn tab_session_with_unknown_tab_returns_the_error_without_calling_f() {
    let tab = TabSession::new("t-9", Arc::new(Closed));
    let mut called = false;
    let r = tab.with(|_| called = true);
    assert_that!(r).is_err();
    assert_that!(called).is_false();
}
```

`crate::test_support::session()` : créer `#[cfg(test)] pub(crate) mod test_support` dans `lib.rs` en y déplaçant la fabrique de session des tests de l'agent (`agent/context_tests.rs`, session ouverte sur `fixtures/roadmap.json`), utilisée ensuite des deux côtés (pas de copie). `UnknownTab` n'existe qu'après la fusion (Task 7) : sur la branche, `Closed` rend `crate::error::AgentFormSnafu { reason: format!("{tab} closed") }` ; la Task 7 le remplace par `UnknownTabSnafu`.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session tabs` → échec de compilation (`tabs` absent).

- [ ] **Step 3 : implémentation** (`tabs.rs`)

```rust
//! L'agent et les onglets : la session d'un onglet, fournie par l'hôte.

use std::sync::{Arc, Mutex, PoisonError};

use crate::Session;

/// Les sessions des onglets, vues par l'agent.
///
/// Implement this trait to give the agent's tools and turns a tab's session
/// (l'espace de travail de l'hôte, ou une session seule dans les tests).
pub trait Sessions: Send + Sync + std::fmt::Debug {
    /// Appelle `f` sur la session de `tab` ; bloquant. Onglet inconnu : erreur, `f` n'est pas appelée.
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()>;
}

/// Une seule session, quel que soit l'onglet (tests, hôtes à un document).
impl Sessions for Mutex<Session> {
    fn with(&self, _tab: &str, f: &mut dyn FnMut(&mut Session)) -> crate::Result<()> {
        f(&mut self.lock().unwrap_or_else(PoisonError::into_inner));
        Ok(())
    }
}

/// Un onglet et l'accès à sa session.
#[derive(Clone, Debug)]
pub struct TabSession {
    tab: String,
    sessions: Arc<dyn Sessions>,
}

impl TabSession {
    pub fn new(tab: impl Into<String>, sessions: Arc<dyn Sessions>) -> Self {
        Self { tab: tab.into(), sessions }
    }

    pub fn tab(&self) -> &str {
        &self.tab
    }

    /// `f` sur la session de l'onglet ; bloquant (verrou) : hors du fil async.
    pub fn with<R>(&self, f: impl FnOnce(&mut Session) -> R) -> crate::Result<R> {
        let mut f = Some(f);
        let mut out = None;
        self.sessions.with(&self.tab, &mut |s| {
            if let Some(f) = f.take() {
                out = Some(f(s));
            }
        })?;
        // `Sessions::with` appelle `f` au plus une fois ; `Ok` sans appel est une faute de l'implémentation.
        out.ok_or_else(|| crate::error::AgentFormSnafu { reason: format!("{}: session not called", self.tab) }.build().into())
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
```

`agent/mod.rs` : `mod tabs;` et `pub use tabs::{Sessions, TabSession};`. Ajouter `Sessions`/`TabSession` à la liste de la doc de `agent` et à `ARCHITECTURE.md` (une ligne : « `agent::tabs` : `Sessions` (fourni par l'hôte), `TabSession` »).

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session tabs` → PASS ; clippy.
- [ ] **Step 5 : commit** `feat(session) : Sessions et TabSession, la session d'un onglet pour l'agent (t-224)`.

---

### Task 2 : outils MCP sur `TabSession`

**Files:**
- Modify: `crates/oku-slides-mcp/src/lib.rs` (retirer `SharedSession`), `hub.rs`, `tools.rs`, `read_tools.rs`, `edit_tools.rs`, `guarded_tools.rs`, `surface_tools.rs`, `crates/oku-slides-mcp/tests/common/mod.rs`, `crates/oku-slides-mcp/ARCHITECTURE.md`
- Test: `crates/oku-slides-mcp/tests/surfaces.rs` (nouveau test), tous les tests existants du crate

**Interfaces:**
- Consumes: `TabSession`, `Sessions` (Task 1).
- Produces:
  - `SurfaceHub::new(session: TabSession, localizer: Arc<Localizer>) -> Arc<Self>` ; `SurfaceHub::tab(&self) -> &str`.
  - `Tools::new(session: TabSession, hub: Arc<SurfaceHub>, settings: Arc<dyn SettingsPort>, platform: &Platform) -> Self`.
  - `oku_slides_mcp::SharedSession` n'existe plus.
  - Harnais de test : `Harness.session: Arc<Mutex<Session>>` (pour les assertions) et `Harness.tab: TabSession`.

- [ ] **Step 1 : test qui échoue** (`tests/surfaces.rs`) : un outil sur un onglet fermé rend une erreur à l'agent, sans paniquer.

```rust
#[tokio::test(flavor = "multi_thread")]
async fn tool_on_closed_tab_returns_an_error_to_the_agent() {
    let h = start_closed().await; // harnais dont `Sessions` refuse tout onglet (voir Step 3)
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "deck_outline", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp` → échec de compilation (`start_closed`, `TabSession`).

- [ ] **Step 3 : implémentation.**
  - `lib.rs` : supprimer `SharedSession` et `use std::sync::{Arc, Mutex}` s'il ne sert plus.
  - `SurfaceHub` : champ `pub(crate) session: TabSession` ; `new(session: TabSession, localizer)` ; `pub fn tab(&self) -> &str { self.session.tab() }`.
  - `apply_param`, `apply_variant` : remplacer le bloc `self.session.lock()…` par

    ```rust
    self.session
        .with(|s| -> Result<(), String> {
            let op = s.param_op(field, &value).map_err(|e| e.one_line())?;
            let label = self.localizer.tr_with("agent-undo-param", &[("label", field.label.as_str().into())]);
            s.apply(&label, vec![op]).map_err(|e| e.one_line())?;
            s.note(format!("parameter: {:?} = {value} ({})", field.label, spec.slide));
            Ok(())
        })
        .map_err(|e| e.one_line())??;
    ```

    (même forme pour `apply_variant`).
  - `prune` : le corps actuel (lecture de `s.deck()`, calcul de `gone`, retrait de `state.shows`) passe dans `let Ok(gone) = self.session.with(|s| { … }) else { return };` ; seul l'accès à la session change.
  - `Tools` : champ `session: TabSession` ; `with_session` devient

    ```rust
    pub(crate) async fn with_session<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let session = self.session.clone();
        tokio::task::spawn_blocking(move || session.with(f).map_err(|e| e.one_line())?)
            .await
            .map_err(|e| format!("session task failed: {e}"))?
    }
    ```

    Les appelants (`read_tools.rs`, `edit_tools.rs`, `guarded_tools.rs`, `surface_tools.rs`) ne changent pas.
  - `tests/common/mod.rs` : `start()` construit `let session = Arc::new(Mutex::new(session));` puis `let tab = TabSession::new("t-1", session.clone());` et passe `tab.clone()` à `SurfaceHub::new` et `Tools::new` ; `Harness` gagne `tab: TabSession`, garde `session: Arc<Mutex<Session>>`. Ajouter `start_closed()` : même chose avec un `Sessions` de test qui renvoie toujours une erreur (struct `ClosedTabs`, `#[derive(Debug)]`).
  - `ARCHITECTURE.md` : « la session vient de `TabSession` (un onglet, fourni par l'hôte) ».

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp` → tout passe (dont la régression t-222).
- [ ] **Step 5 : commit** `refactor(mcp) : outils et surfaces sur la session d'un onglet (TabSession) (t-224)`.

(La crate `oku-slides-agent` ne compile plus jusqu'à la Task 4 : cette tâche ne lance que les tests de `oku-slides-mcp` ; ne pas pousser entre les Tasks 2 et 6.)

---

### Task 3 : faux agent à plusieurs sessions

**Files:**
- Modify: `crates/oku-slides-agent/src/bin/fake_agent.rs`

**Interfaces:**
- Produces (scénario JSON, en plus de l'existant) : chaque session a son serveur MCP (celui de son `session/new`) et son propre compteur de tours ; un tour peut attendre `"delay_ms": <u64>` avant de répondre ; le journal (`--log`) note `sessionId` avec chaque message reçu (déjà dans les paramètres ACP : vérifier que `log` écrit bien les `params` entiers).

- [ ] **Step 1 :** dans `State`, remplacer `mcp: Mutex<Option<(String, String)>>` par `mcp: Mutex<HashMap<String, (String, String)>>` (clé : `SessionId` en texte) et `turns: AtomicUsize` par `turns: Mutex<HashMap<String, usize>>` ; `wait_cancel` : une `Notify` par session (`Mutex<HashMap<String, Arc<Notify>>>`), réveillée par `session/cancel` de cette session seulement.
- [ ] **Step 2 :** `Turn` gagne `#[serde(default)] delay_ms: u64` ; `play` fait `tokio::time::sleep(Duration::from_millis(turn.delay_ms)).await` après le plan et avant les appels MCP.
- [ ] **Step 3 :** `session/new` enregistre le serveur HTTP de **cette** session ; `session/prompt` joue le tour `n` (modulo) de **cette** session et appelle **son** serveur.
- [ ] **Step 4 :** `Script` gagne `#[serde(default)] crash_on_session: Option<usize>` : le premier prompt de la n-ième session créée (1 = la première) fait quitter le processus, comme `crash`.
- [ ] **Step 5 :** mettre à jour la doc de tête (`Scénario : …`) : « par session : serveur MCP, compteur de tours, annulation », `delay_ms` et `crash_on_session`.
- [ ] **Step 6 :** `cargo build -p oku-slides-agent --bin oku-slides-fake-agent` ; les tests qui l'exercent viennent aux Tasks 4 et 5.
- [ ] **Step 7 : commit** `test(agent) : faux agent à plusieurs sessions, délai de tour (t-224)`.

---

### Task 4 : connexion partagée et agent par onglet

**Files:**
- Create: `crates/oku-slides-agent/src/link.rs` (connexion partagée, routage), `crates/oku-slides-agent/src/tab_agent.rs` (acteur d'un onglet)
- Delete: `crates/oku-slides-agent/src/actor.rs`, `crates/oku-slides-agent/src/connection.rs` (leur contenu passe dans `link.rs` et `tab_agent.rs`)
- Modify: `crates/oku-slides-agent/src/turn.rs`, `lib.rs`, `config.rs`, `tests/common/mod.rs`, `tests/turns.rs`, `tests/launch.rs`, `ARCHITECTURE.md`
- Test: `crates/oku-slides-agent/tests/tabs.rs` (nouveau)

**Interfaces:**
- Consumes: `TabSession` (Task 1), `SurfaceHub::new(TabSession, …)`, `Tools::new(TabSession, …)` (Task 2), faux agent (Task 3).
- Produces:
  - `pub(crate) struct Link` (`Clone`) : `Link::new(launch: Launch, localizer: Arc<Localizer>) -> Self` ; `async fn connect(&self) -> Result<Live, AgentError>` ; `fn route(&self, id: SessionId, route: Route)` ; `fn forget(&self, id: &SessionId)`.
  - `pub(crate) struct Live { pub cx: ConnectionTo<Agent>, pub generation: u64, pub closed: CancellationToken }` (`Clone`).
  - `pub(crate) struct Route { pub events: broadcast::Sender<AgentEvent>, pub hub: Arc<SurfaceHub> }`.
  - `config.rs` : `pub struct Launch { pub command: String, pub args: Vec<String>, pub fallback_cwd: PathBuf }` (remplace `AgentConfig` ; `mcp_url`/`authorization` passent au serveur de chaque onglet).
  - `pub struct AgentHandle` (inchangé côté API : `prompt(text, ui)`, `cancel()`, `new_conversation()`, `subscribe()`), créé par `AgentHandle::spawn(tab: TabSession, hub: Arc<SurfaceHub>, tools: ToolsAccess, link: Link, localizer: Arc<Localizer>) -> Self` où `pub(crate) struct ToolsAccess { pub url: String, pub authorization: String }`.

Conception (à respecter) :
- `Link::connect` : sous `tokio::sync::Mutex<Option<Live>>`, rend le `Live` courant s'il n'est pas `closed` ; sinon lance le sous-processus (code de l'actuel `connection::run` : `Command`, `kill_on_drop`, erreurs `NotFound`/`Spawn`), puis `tokio::spawn` d'une tâche qui fait `Client.builder()…connect_with(transport, async |cx| { handshake; ready.send(Ok(live)); select!{ cx.incoming_closed(), child.wait(), shutdown.cancelled() }; Ok(()) })` et, en sortie, `closed.cancel()` et vide `routes`. `connect` attend `ready` (oneshot) ; échec de la poignée de main → `NoHttp`. `generation` augmente à chaque lancement.
- Gestionnaire de notifications : `routes.lock().get(&n.session_id)` → `from_update(n.update)` → `route.events.send(e)` ; session inconnue : `tracing::debug!` et rien d'autre.
- Gestionnaire de permissions : même routage vers `route.hub.permission(...)` (code actuel) ; session inconnue → `Cancelled`.
- `TabAgent` (tâche par onglet, `tab_agent.rs`) : boucle de commandes de l'actuel `actor::run` + `connection::conversation`. Garde `conversation: Option<(u64, SessionId)>` ; au prompt : `link.connect()` (erreur → `AgentEvent::Error` localisé, `Status Idle`) ; si pas de conversation ou `generation` différente → `session/new` avec `cwd = tab.with(|s| s.document_dir())` (sinon `fallback_cwd`) et le serveur MCP de l'onglet (`ToolsAccess`), puis `link.route(id, Route { events, hub })`. Puis `turn::run`. Une erreur de requête pendant le tour avec `live.closed` annulé → `AgentEvent::Error { message: tr("agent-stopped") }`, conversation oubliée.
- `turn.rs` : `ctx.session` devient `ctx.tab: TabSession` ; `begin_turn`/`agent_context` et `end_turn` passent par `tokio::task::spawn_blocking(move || tab.with(|s| …))`.
- Fin de l'acteur (canal de commandes fermé : `AgentHandle` lâché) : `session/cancel` si un tour tourne, `link.forget(id)`, `hub.cancel_asks()`.

- [ ] **Step 1 : tests qui échouent** (`tests/tabs.rs`) — avec un harnais `start_tabs(script, &["t-1", "t-2"])` dans `tests/common/mod.rs` qui crée deux sessions (même fixture), un `Link` partagé et, par onglet, `SurfaceHub` + `ToolServer` + `AgentHandle`.

```rust
#[tokio::test(flavor = "multi_thread")]
async fn two_tabs_share_one_process_and_get_two_sessions() {
    let h = start_tabs(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1", "t-2"]).await;
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(&h.tab("t-1")).await;
    wait_turn_ended(&h.tab("t-2")).await;
    let log = read_log(&h.log);
    assert_that!(log.iter().filter(|m| m["method"] == "initialize").count()).is_equal_to(1);
    assert_that!(log.iter().filter(|m| m["method"] == "session/new").count()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn turns_of_two_tabs_run_at_the_same_time() {
    let h = start_tabs(json!({ "turns": [{ "delay_ms": 800, "chunks": ["fini"] }] }), &["t-1", "t-2"]).await;
    let started = std::time::Instant::now();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    h.tab("t-2").agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(&h.tab("t-1")).await;
    wait_turn_ended(&h.tab("t-2")).await;
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(1500));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_tab_receives_only_its_own_chunks() {
    let h = start_tabs(json!({ "turns": [{ "chunks": ["réponse"] }] }), &["t-1", "t-2"]).await;
    let mut other = h.tab("t-2").agent.subscribe();
    h.tab("t-1").agent.prompt("un".into(), AgentUi::default());
    wait_turn_ended(&h.tab("t-1")).await;
    assert_that!(other.try_recv().is_err()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_calls_act_on_their_own_tab() {
    // Le tour appelle `apply` sur la slide 1 : seule la session de t-2 change.
    let h = start_tabs(json!({ "turns": [{ "mcp": [{ "tool": "apply", "args": { "label": "x", "ops": [
        { "op": "set_slide_field", "slide": "s-libre", "field": "title", "value": "Changé" } ] } }] }] }), &["t-1", "t-2"]).await;
    h.tab("t-2").agent.prompt("change".into(), AgentUi::default());
    wait_turn_ended(&h.tab("t-2")).await;
    let title = |t: &str| h.session(t).lock().map(|s| s.deck().slide("s-libre").and_then(|s| s.title.clone())).unwrap();
    assert_that!(title("t-2")).is_equal_to(Some("Changé".to_owned()));
    assert_that!(title("t-1")).is_not_equal_to(Some("Changé".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_crash_ends_the_turns_of_every_tab_with_an_error() {
    // t-1 attend l'annulation ; le premier prompt de la 2e session (t-2) fait quitter l'agent.
    let h = start_tabs(json!({ "crash_on_session": 2, "turns": [{ "wait_cancel": true }] }), &["t-1", "t-2"]).await;
    let mut first = h.tab("t-1").agent.subscribe();
    let mut second = h.tab("t-2").agent.subscribe();
    h.tab("t-1").agent.prompt("attends".into(), AgentUi::default());
    wait_status(&mut first, AgentState::Working).await;
    h.tab("t-2").agent.prompt("plante".into(), AgentUi::default());
    assert_that!(wait_error(&mut first).await).contains("arrêté");
    assert_that!(wait_error(&mut second).await).contains("arrêté");
}
```

Helpers de `tests/common/mod.rs` (à ajouter) : `wait_turn_ended(tab)`, `wait_status(rx, AgentState)`, `wait_error(rx) -> String` (message de `AgentEvent::Error`), `read_log(path) -> Vec<Value>` ; chacun échoue après 10 s. « arrêté » vient de `agent-stopped` en français (locale `fr` du harnais).

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-agent --test tabs` → échec de compilation.
- [ ] **Step 3 : implémentation** selon la conception ci-dessus ; adapter `tests/turns.rs`, `tests/launch.rs` et le harnais (`start_with` = `start_tabs(script, &["t-1"])` puis `h.tab("t-1")`) ; `lib.rs` : `mod link; mod tab_agent;`, ré-exports `AgentHandle`, `Launch` (plus `AgentConfig`).
- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent` → tout passe (anciens tests de tour, d'annulation, de relance après arrêt inclus).
- [ ] **Step 5 :** `ARCHITECTURE.md` du crate : modules `link` (connexion partagée, routage par `SessionId`), `tab_agent` (acteur d'un onglet), `turn` ; sous 50 lignes.
- [ ] **Step 6 : commit** `feat(agent) : une connexion ACP partagée, un agent par onglet, tours simultanés (t-224)`.

---

### Task 5 : la pile par onglets, enveloppes et fil gardé

**Files:**
- Modify: `crates/oku-slides-agent/src/stack.rs`, `crates/oku-slides-agent/src/events.rs` (variant `Thread`), `crates/oku-slides-agent/src/events_tests.rs`, `crates/oku-slides-agent/tests/stack.rs`, `ARCHITECTURE.md`
- Create: `crates/oku-slides-agent/src/thread_log.rs`, `crates/oku-slides-agent/src/thread_log_tests.rs`

**Interfaces:**
- Consumes: Task 4 (`Link`, `AgentHandle::spawn`, `ToolsAccess`, `Launch`).
- Produces:
  - `AgentCommand` : chaque variante porte `tab: String` (JSON `{"cmd": "prompt", "tab": "t-1", "text", "ui"}` etc.).
  - `AgentEvent::Thread { entries: Vec<AgentEvent> }` (`kind: "thread"`).
  - `Stack::start(sessions: Arc<dyn Sessions>, platform: &Platform, stt: Option<Stt>, rt: tokio::runtime::Handle) -> Self` (plus d'`async`, plus de `Result` : les serveurs d'outils démarrent par onglet).
  - `Stack::handle(&self, cmd: AgentCommand) -> Result<(), String>`, `document_changed(&self, tab: &str)`, `after_change(&self, tab: &str)`, `tab_closed(&self, tab: &str)`, `forward(&self, sink: impl Fn(&'static str, String) + Send + Sync + 'static)` (une fois ; JSON émis : `{"tab": "t-1", "event": {…}}`).
  - `thread_log::ThreadLog` : `push(&mut self, e: &AgentEvent)` (garde les 200 derniers ; `Conversation` vide le fil puis se garde), `entries(&self) -> Vec<AgentEvent>`.

Conception :
- `Stack { link, sessions, platform: PlatformParts { localizer, storage, settings: Arc<AppSettings> }, rt, tabs: Mutex<HashMap<String, TabEntry>>, out_tx: mpsc::UnboundedSender<(&'static str, String)>, out_rx: Mutex<Option<mpsc::UnboundedReceiver<…>>> }`.
- `TabEntry { hub: Arc<SurfaceHub>, agent: AgentHandle, thread: Arc<Mutex<ThreadLog>>, cancel: CancellationToken }`, créé à la première commande `prompt`/`new_conversation`/`surface_action` de l'onglet : `TabSession::new(tab, sessions)`, `SurfaceHub::new`, `Tools::new`, puis, **dans une tâche `rt.spawn`**, `ToolServer::start(tools, cancel)` suivi de `AgentHandle::spawn(...)` ; en attendant le serveur, l'`AgentHandle` met les commandes en file (canal créé avant, passé à `spawn`). Deux tâches de relais par onglet : `hub.subscribe()` → `out_tx.send(("mcp", enveloppe))` (retard : `Changed` puis `Surfaces` de ce hub, comme aujourd'hui) ; `agent.subscribe()` → `thread.push(&e)` puis `out_tx.send(("agent", enveloppe))`.
- `resync{tab}` : si l'onglet a une entrée, `("agent", {tab, event: Thread{entries}})` puis `hub.resync()` ; sinon `Thread{entries: []}` et `Surfaces{list: []}` directement.
- `prompt{tab}` : `hub.answer_pending(text)` sinon `agent.prompt`. `cancel{tab}` : `hub.cancel_asks()` + `agent.cancel()`. `surface_action{tab}` : `hub.act`, onglet inconnu → `Err("unknown surface")`.
- `tab_closed(tab)` : retirer l'entrée ; son `Drop` : `hub.cancel_asks()`, `cancel.cancel()` (serveur), l'`AgentHandle` lâché arrête l'acteur (Task 4).
- `document_changed(tab)` / `after_change(tab)` : comme aujourd'hui, sur l'entrée de l'onglet s'il y en a une.

- [ ] **Step 1 : tests qui échouent** — `thread_log_tests.rs` :

```rust
#[test]
fn thread_log_keeps_the_last_200_events() {
    let mut log = ThreadLog::default();
    for i in 0..250 {
        log.push(&AgentEvent::Chunk { text: i.to_string() });
    }
    let entries = log.entries();
    assert_that!(entries).has_length(200);
    assert_that!(entries[0].clone()).is_equal_to(AgentEvent::Chunk { text: "50".into() });
}

#[test]
fn thread_log_conversation_clears_the_thread() {
    let mut log = ThreadLog::default();
    log.push(&AgentEvent::Chunk { text: "a".into() });
    log.push(&AgentEvent::Conversation);
    assert_that!(log.entries()).is_equal_to(vec![AgentEvent::Conversation]);
}
```

(`AgentEvent` doit dériver `Clone, PartialEq` ; l'ajouter s'il ne le fait pas.) `tests/stack.rs`, avec le faux agent :

```rust
#[tokio::test(flavor = "multi_thread")]
async fn forward_wraps_every_event_with_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1", "t-2"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-2", "text": "salut" })).unwrap()).unwrap();
    let chunk = wait_for(&rx, |(ch, v)| *ch == "agent" && v["event"]["kind"] == "chunk").await;
    assert_that!(chunk.1["tab"].clone()).is_equal_to(json!("t-2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_replays_the_tab_thread_then_its_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{ "chunks": ["bonjour"] }] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" })).unwrap()).unwrap();
    wait_for(&rx, |(_, v)| v["event"]["kind"] == "turn_ended").await;
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    let thread = wait_for(&rx, |(_, v)| v["event"]["kind"] == "thread").await;
    let kinds: Vec<_> = thread.1["event"]["entries"].as_array().unwrap().iter().map(|e| e["kind"].clone()).collect();
    assert_that!(kinds).contains(json!("chunk"));
    wait_for(&rx, |(_, v)| v["event"]["kind"] == "surfaces" && v["tab"] == "t-1").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_of_an_unused_tab_sends_an_empty_thread_and_no_surfaces() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(serde_json::from_value(json!({ "cmd": "resync", "tab": "t-1" })).unwrap()).unwrap();
    let thread = wait_for(&rx, |(_, v)| v["event"]["kind"] == "thread").await;
    assert_that!(thread.1["event"]["entries"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn tab_closed_during_a_turn_releases_its_questions_and_other_tabs_go_on() {
    // Premier tour de chaque session : une question ; t-1 la laisse ouverte, t-2 y répond.
    let script = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?",
        "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }], "chunks": ["fini"] }] });
    let (stack, rx) = stack_with_sink(script, &["t-1", "t-2"]).await;
    let prompt = |tab: &str| serde_json::from_value(json!({ "cmd": "prompt", "tab": tab, "text": "?" })).unwrap();
    stack.handle(prompt("t-1")).unwrap();
    stack.handle(prompt("t-2")).unwrap();
    let ask2 = wait_for(&rx, |(_, v)| v["tab"] == "t-2" && v["event"]["kind"] == "surface").await;
    wait_for(&rx, |(_, v)| v["tab"] == "t-1" && v["event"]["kind"] == "surface").await;
    stack.tab_closed("t-1");
    let id = ask2.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    stack.handle(serde_json::from_value(json!({ "cmd": "surface_action", "tab": "t-2", "surface": id,
        "name": "choose", "context": { "choice": "b" } })).unwrap()).unwrap();
    wait_for(&rx, |(_, v)| v["tab"] == "t-2" && v["event"]["kind"] == "turn_ended").await;
    let seen = rx.lock().unwrap().len();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let late: Vec<_> = rx.lock().unwrap()[seen..].iter().filter(|(_, v)| v["tab"] == "t-1").cloned().collect();
    assert_that!(late.iter().any(|(_, v)| v["event"]["kind"] == "chunk")).is_false();
}
```

Helpers (`tests/common/mod.rs`) : `stack_with_sink(script, tabs) -> (Stack, Arc<Mutex<Vec<(&'static str, Value)>>>)` (un `Sessions` de test qui tient une `Session` par onglet dans une `HashMap` ; `forward` pousse chaque `(canal, JSON lu)`) et `wait_for(rx, pred) -> (&'static str, Value)` (sondage toutes les 20 ms, échec après 10 s).

- [ ] **Step 2 :** tests → échec de compilation.
- [ ] **Step 3 : implémentation** selon la conception.
- [ ] **Step 4 :** `cargo nextest run -p oku-slides-agent` → PASS.
- [ ] **Step 5 :** `ARCHITECTURE.md` : `stack` (table des onglets, enveloppes, fil gardé), `thread_log`.
- [ ] **Step 6 : commit** `feat(agent) : pile par onglets, enveloppes {tab, event}, fil rejoué par resync (t-224)`.

---

### Task 6 : hôtes et interface de la branche sur un onglet

**Files:**
- Modify: `src-tauri/src/main.rs`, `crates/oku-slides-cli/src/serve.rs`, `crates/oku-slides-agent/examples/smoke.rs`, `ui/src/api.ts`, `ui/src/agent/events.ts`, `ui/src/agent/events.test.ts`, `ui/src/agent/thread.ts`, `ui/src/agent/thread.test.ts`, `ui/src/App.tsx`

**Interfaces:**
- Consumes: Task 5 (`Stack::start(sessions, platform, stt, rt)`, `AgentCommand` avec `tab`, enveloppes).
- Produces (UI) :
  - `agentApi(tab: string)` dans `api.ts` : `{ prompt(text, ui), cancel(), newConversation(), surfaceAction(surface, name, context), resync() }` (chaque appel ajoute `tab`).
  - `subscribeHost(tab: string, handlers: HostHandlers)` : lit `{tab, event}`, ne livre que les évènements de `tab`.
  - `threadReducer` gère `{ kind: "thread", entries }` : repart de `EMPTY_THREAD` et replie `entries`.
  - `App` : constante `const TAB = "t-1"` (remplacée par la prop `tab` à la Task 7).

- [ ] **Step 1 : tests qui échouent** —
  - `events.test.ts` : `subscribeTauri` avec un faux `listen` qui émet `{"tab":"t-2","event":{"kind":"changed"}}` puis `{"tab":"t-1","event":{"kind":"changed"}}` : `onMcp` n'est appelé qu'une fois pour l'abonnement de `t-1`.
  - `events.test.ts` : charge sans `tab` ou sans `event` → `onError`, rien livré.
  - `thread.test.ts` : `threadReducer(state_avec_un_chunk, { kind: "thread", entries: [{kind:"user_text",text:"a"},{kind:"chunk",text:"b"}] })` donne le même état que les deux évènements replayés sur `EMPTY_THREAD`.
- [ ] **Step 2 :** `cd ui && bun run test` → échecs.
- [ ] **Step 3 : implémentation UI** : `parse` lit l'enveloppe (`tab: string`, `event.kind: string`) ; `subscribeHost(tab, h)`, `subscribeTauri(load, tab, h)`, `subscribeSse(url, tab, h)` filtrent ; `api.ts` : `AgentCommand` avec `tab`, `agentApi(tab)` remplace `agent` ; `App` : `const agent = useMemo(() => agentApi(TAB), [])`, `subscribeHost(TAB, …)`.
- [ ] **Step 4 : hôtes** : `Stack::start(Arc::new(Mutex<Session>)… as Arc<dyn Sessions>, &platform, Some(stt), rt)` — Tauri : `rt = tauri::async_runtime::handle().inner().clone()` ; serve : `runtime.handle().clone()` ; `request` : `document_changed("t-1")` / `after_change("t-1")` (constante `SINGLE_TAB` dans chaque hôte, supprimée à la Task 7) ; `smoke.rs` : nouvelle API.
- [ ] **Step 5 :** `cargo nextest run --workspace`, `cargo build -p oku-slides-app`, `cd ui && bun run test && bunx tsc -b`, `just e2e` → tout passe.
- [ ] **Step 6 : commit** `feat(agent) : hôtes et interface parlent par onglet (un seul onglet avant la fusion) (t-224)`.

---

### Task 7 : fusion de `main`

**Files:** conflits connus : `Cargo.lock`, `crates/oku-slides-cli/src/serve.rs`, `crates/oku-slides-cli/tests/cli.rs`, `crates/oku-slides-session/src/{error.rs,lib.rs,text.rs}`, `docs/superpowers/plans/2026-09-26-agent-vocal.md`, `src-tauri/src/main.rs`, `ui/bun.lock`, `ui/e2e/smoke.cjs`, `ui/package.json`, `ui/src/{App.tsx,Stage.tsx,styles.css}` (liste relevée au premier essai ; la revérifier).
- Modify en plus : `crates/oku-slides-session/src/workspace.rs` (`with_session`, `impl Sessions`), `crates/oku-slides-session/src/workspace_tests.rs`, `src-tauri/src/{state.rs,commands.rs}`, `ui/src/Workspace.tsx` si besoin.

**Interfaces:**
- Produces:
  - `Workspace::with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool)` : comme `request` (slot cloné sous le verrou de l'espace de travail, `ready()`, `f`, puis `refresh`) ; `bool` : la barre d'onglets est à redessiner.
  - `impl Sessions for Mutex<Workspace>` (ignore le `bool`) ; côté Tauri, `struct AppSessions(tauri::AppHandle)` implémente `Sessions` via `with_session` et appelle `windows::emit_view` quand le `bool` est vrai.

- [ ] **Step 1 :** `git merge --no-edit main` ; résoudre :
  - plan de l'agent : garder la version de la branche (`git checkout --ours`) ; elle contient déjà 0a01468.
  - `error.rs`, `lib.rs` (session) : garder les deux côtés (variantes `AgentForm` + `UnknownTab`/`UnknownWindow`/`Unsaved` ; champs `agent_notes` + `sent_blobs`, `resume: Option<resume_writer::Writer>` de `main`).
  - `text.rs` : prendre `main` (markdown, `escape_list_marker`) **et** garder `text_to_paragraphs` provisoirement (supprimé à la Task 9) ; `agent/mod.rs` continue de l'utiliser d'ici là.
  - `serve.rs` : partir de `main` (espace de travail, `TabRequest`, `/api/workspace`, `same_origin`, `MAX_BODY`), ajouter de la branche : `/api/events` (SSE, `sse.rs`, fils), `/api/agent`, la garde `guard::check`, le runtime tokio et la `Stack` avec `Arc::new(workspace_sessions)` ; `/api` avec `TabRequest` → après la requête `stack.after_change(&tab)` ou `document_changed(&tab)` (`New`/`Open`) ; `/api/workspace` → pour chaque onglet absent de la nouvelle vue, `stack.tab_closed(id)`. L'espace de travail vit dans un `Arc<Mutex<Workspace>>` partagé par la boucle HTTP et `Sessions`.
  - `cli.rs` (tests) : garder les helpers de `main` et ceux de la branche ; `http_with` ajoute `Content-Type: application/json` (garde anti-CSRF).
  - `src-tauri/src/main.rs` : partir de `main` ; `AppState` (`state.rs`) gagne `stack: Option<Stack>` ; démarrage : `Stack::start(Arc::new(AppSessions(app.handle().clone())), &platform, Some(stt.clone()), rt)` après `Workspace::resume`, `forward` → `app.emit(&format!("oku-slides:{name}"), json)` ; commande `agent(state, req: AgentCommand)` ajoutée aux handlers ; `commands::request` appelle `after_change`/`document_changed(&tab)` ; `commands::workspace` compare les onglets avant/après et appelle `tab_closed`. Doc de tête : garder celle de `main`, ajouter une phrase : « L'agent (`oku-slides-agent::Stack`) : un par onglet, sessions via `AppSessions`. »
  - `ui/package.json`, `ui/bun.lock` : union des dépendances (happy-dom de la branche, @lezer/markdown de `main`), puis `cd ui && bun install` régénère le verrou ; scripts : `e2e` de la branche (avec `agent.cjs`) + ceux de `main`.
  - `ui/e2e/smoke.cjs` : prendre `main`, rajouter les ajouts de la branche (en-tête JSON des `fetch`).
  - `App.tsx` : partir de `main` (prop `tab`, `tabApi(tab)`), y remettre les ajouts de la branche (réducteurs de fil et de surfaces, `subscribeHost(tab, …)`, `agentApi(tab)`, surfaces, dictée à deux modes, `onError`) ; `TAB` disparaît au profit de `tab`.
  - `Stage.tsx`, `styles.css` : union (props de `main` comme `palette`, ajouts de la branche comme `.stage-frame`, la fiche ancrée).
  - `Cargo.lock` : `git checkout --theirs Cargo.lock` puis `cargo build` le complète.
- [ ] **Step 2 : test qui échoue** (`workspace_tests.rs`) :

```rust
#[test]
fn with_session_runs_f_and_reports_that_the_tab_changed() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let (read, changed) = Workspace::with_session(&ws, "t-1", |s| s.deck().slides.len());
    assert_that!(read).is_ok();
    assert_that!(changed).is_false();
    let (applied, changed) = Workspace::with_session(&ws, "t-1", |s| s.handle(set_theme()).is_ok());
    assert_that!(applied).is_ok().is_true();
    assert_that!(changed).is_true(); // l'onglet n'est plus vierge : la barre change
}

#[test]
fn with_session_on_a_closed_tab_returns_unknown_tab_without_calling_f() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-1", "discard": true})).unwrap();
    let mut called = false;
    let (r, changed) = Workspace::with_session(&ws, "t-1", |_| called = true);
    assert_that!(r.map_err(|e| e.kind())).is_err().is_equal_to(SessionErrorKind::UnknownTab);
    assert_that!(called).is_false();
    assert_that!(changed).is_false();
}

#[test]
fn with_session_releases_the_workspace_lock_while_f_runs() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let (free, _) = Workspace::with_session(&ws, "t-1", |_| ws.try_lock().is_ok());
    assert_that!(free).is_ok().is_true();
}
```

(Fabriques `Data`, `workspace_in`, `send`, `set_theme` : celles de `workspace_tests.rs` sur `main`. L'accès au `SessionErrorKind` d'une erreur : celui qu'utilisent déjà ces tests.) Dans `tabs_tests.rs` (Task 1), remplacer `AgentFormSnafu` par `UnknownTabSnafu { id: tab }`.
- [ ] **Step 3 :** implémenter `with_session` et `impl Sessions for Mutex<Workspace>` :

```rust
/// `f` sur la session de l'onglet (réveillée au besoin) ; `true` : la barre est à redessiner.
///
/// Même discipline que [`Workspace::request`] : l'espace de travail est
/// verrouillé avant et après, jamais pendant `f`.
pub fn with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool) {
    let session = match Self::lock(ws).session(tab) {
        Ok(session) => session,
        Err(e) => return (Err(e), false),
    };
    let (out, info) = {
        let mut slot = relock(&session);
        match slot.ready() {
            Ok(s) => {
                let out = f(s);
                (Ok(out), Some(s.tab_info()))
            }
            Err(e) => (Err(e), None),
        }
    };
    let changed = info.is_some_and(|info| Self::refresh(ws, tab, info));
    (out, changed)
}
```

```rust
impl crate::agent::Sessions for Mutex<Workspace> {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> Result<()> {
        Workspace::with_session(self, tab, |s| f(s)).0
    }
}
```

- [ ] **Step 4 :** vérifications complètes : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` (le `large_enum_variant` de `oku-slides-core` disparaît avec `main`), `cargo nextest run --workspace`, `cargo build -p oku-slides-app`, `cd ui && bun run test && bunx tsc -b`, `just e2e` (barre d'outils, panneau, agent). Vérifier à la main (preview `oku-slides-agent-demo`, faux agent) : deux onglets, une consigne dans chacun, les fils restent séparés, changer d'onglet puis revenir rejoue le fil.
- [ ] **Step 5 : commit de fusion** (message par défaut de git, complété) : `Merge branch 'main' into 89-feature/agent-vocal : agent par onglet sur l'espace de travail (t-224)`.

---

### Task 8 : reprise des modifications de l'agent

> Réécrite après la fusion (Task 7) : `main` a remplacé `History` par Loro (t-188) et retiré `Request::New/Open`. Constat de la revue de la Task 7 : `persist` n'est appelé qu'à la fin de `Session::handle` (`lib.rs`, `if changes { self.persist(documents) }`) ; les chemins de l'agent qui appellent la session directement ne sont jamais écrits pour la reprise : `s.apply` dans `crates/oku-slides-mcp/src/hub.rs` (`apply_param`, `apply_variant`), l'outil `apply` et la reformulation acceptée (`surface_tools.rs`), `s.open` et `Session::new_deck` dans `guarded_tools.rs`. Le `Snapshot` qui suit côté interface ne persiste pas non plus.

**Files:**
- Modify: `crates/oku-slides-session/src/lib.rs` (`apply`, `open`, `new_deck`, `handle`)
- Test: `crates/oku-slides-session/src/resume_tests.rs` ; régression `crates/oku-slides-session/tests/regressions/issue_0224_agent_edits_resume.rs` (+ `tests/regressions/main.rs` si absent)

**Interfaces:**
- Produces: `Session::apply(label, ops)`, `Session::open(path)` et `Session::new_deck()` envoient l'état de reprise (`self.persist(true)`) après succès ; `handle` ne le refait pas pour `Request::Apply` (il reste pour les autres commandes).

- [ ] **Step 1 : tests qui échouent** — `resume_tests.rs` (outillage de reprise du fichier) : une session avec reprise active, `s.apply("x", vec![op])` **sans passer par `handle`**, `s.flush_resume()`, puis une session reprise du même dossier porte l'op ; même chose pour `s.new_deck()` (la reprise rend un deck vierge). Régression (API publique) : `Workspace` avec reprise, `Workspace::with_session(&ws, "t-1", |s| s.apply(...))`, `Workspace::settle`, puis `Workspace::resume` du même dossier → l'onglet porte l'op.
- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session resume issue_0224` → FAIL.
- [ ] **Step 3 :** `persist(true)` à la fin (succès) de `apply`, `open`, `new_deck` ; dans `handle`, `changes` exclut `Request::Apply` (sinon double envoi) — garder `documents` cohérent pour les autres commandes.
- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-mcp -p oku-slides-agent` → PASS.
- [ ] **Step 5 : commit** `fix(session) : les modifications de l'agent sont écrites pour la reprise (t-224)`.

---

### Task 9 : reformulation lue par le parseur de l'interface

**Files:**
- Modify: `crates/oku-slides-session/src/agent/mod.rs` (`rewrite_ops`), `crates/oku-slides-session/src/text.rs` (supprimer `text_to_paragraphs`), `crates/oku-slides-session/src/lib.rs` (ré-export), `crates/oku-slides-mcp/src/surface_tools.rs` (`ask_rewrite`), `ui/src/App.tsx` (`sendAction`), `ui/src/agent/rewrite.ts` (nouveau), `ui/src/agent/rewrite.test.ts` (nouveau)
- Test: `crates/oku-slides-session/src/agent/mod_tests.rs` ou le fichier de tests existant de `rewrite_ops` ; `crates/oku-slides-mcp/tests/surfaces.rs`

**Interfaces:**
- Produces:
  - `Session::rewrite_ops(&self, spec: &AskRewrite, text: &str, paragraphs: Option<Vec<Paragraph>>) -> Result<Vec<Op>>` : un élément exige `paragraphs` (absent → `AgentForm { reason: "paragraphs missing" }`) ; un champ utilise `text`.
  - action `rewrite` : contexte `{ kept, text, paragraphs? }`.
  - `withRewriteParagraphs(surface: Surface, a: QuickAction): QuickAction` (`ui/src/agent/rewrite.ts`) : si `a.name === "rewrite"`, `a.context.kept !== "original"` et que l'ancre de la surface n'est pas un champ (`@…`), ajoute `paragraphs: textToParagraphs(String(a.context.text))`.

- [ ] **Step 1 : tests qui échouent**
  - Rust : `rewrite_ops` d'un élément avec `paragraphs` contenant un run `bold` → l'op `SetText` porte ce run ; sans `paragraphs` → erreur.
  - MCP (`tests/surfaces.rs`) : `ask_rewrite` répondu par `{kept: 1, text: "**Gras** fin", paragraphs: [...]}` → le texte de l'élément a un run gras.
  - UI (`rewrite.test.ts`) : `withRewriteParagraphs` sur `{name:"rewrite", context:{kept:0, text:"**a** b"}}` ajoute des paragraphes dont le premier run est gras ; `kept: "original"` inchangé ; ancre `@title` inchangée ; autre action inchangée.
- [ ] **Step 2 :** tests → FAIL.
- [ ] **Step 3 :** implémentation ; `App.sendAction` applique `withRewriteParagraphs` avant `agent.surfaceAction` ; supprimer `text_to_paragraphs` et son export ; `surface_tools.rs` lit `context.paragraphs` (`serde_json::from_value::<Vec<Paragraph>>`, erreur → `isError`).
- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-mcp`, `cd ui && bun run test` → PASS ; `rg text_to_paragraphs` ne trouve plus rien.
- [ ] **Step 5 : commit** `feat(agent) : la reformulation passe par le parseur markdown de l'interface (t-224)`.

---

### Task 10 : bout à bout à deux onglets, documentation, vérification

**Files:**
- Modify: `ui/e2e/agent.cjs`, `fixtures/agent/e2e.json`, `README.md`, `crates/oku-slides-{agent,mcp,session}/ARCHITECTURE.md`, `docs/superpowers/plans/2026-09-27-agent-onglets.md` (cases cochées)

- [x] **Step 1 :** `agent.cjs` : après le scénario actuel (onglet 1), ouvrir un second onglet (`POST /api/workspace {cmd:"new_tab", window}` puis attendre l'onglet actif), vérifier que le fil de l'agent y est vide, lancer un prompt dans l'onglet 2 pendant que l'onglet 1 attend une réponse (scénario : premier tour de la session 1 = `ask_choice` laissé ouvert ; ajouter ce tour au scénario), vérifier que l'onglet 2 reçoit sa réponse et que la fiche de l'onglet 1 est toujours là en y revenant.
- [x] **Step 2 :** `just e2e` → PASS.
- [x] **Step 3 :** README, section « Agent (⌘⇧D) » : « Conversation : une par onglet, jamais enregistrée ; les onglets travaillent en même temps. » ; `ARCHITECTURE.md` des trois crates à jour.
- [x] **Step 4 : vérification complète** : `just check`, `just test`, `just e2e`, `cargo build -p oku-slides-app` ; passe de consolidation (G10) : `rg SharedSession` vide, un seul `Sessions`, pas de `#[allow(dead_code)]` ajouté hors `tests/common`.
- [ ] **Step 5 : commit** `test(e2e) : l'agent dans deux onglets ; documentation (t-224)` ; `oku cli task done --project slider --id t-224`.
