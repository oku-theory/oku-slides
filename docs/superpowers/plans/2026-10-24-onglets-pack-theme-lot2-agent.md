# Packs and themes as tabs — lot 2: one agent conversation per tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `AgentScope`: a tab has exactly one agent conversation (one thread, one ACP session), restored on reopen; the workshop, still present until lot 3, shares the deck's thread.

**Architecture:** Bottom-up removal across the three crates and the UI. The session keeps one `Option<SavedConversation>` written to `agent/conversation.json`; the hub keeps the set of turns that belong to the thread instead of a turn → scope map and stops announcing a viewed scope; the tab keeper and actor lose every scope argument and the rename plumbing; the UI stops computing and sending a scope and drops the "the agent is answering elsewhere" block.

**Tech Stack:** Rust 2024 (snafu, speculoos, rstest, nextest, tokio), React/TypeScript (Bun tests, happy-dom render tests), Playwright e2e, Fluent.

**Spec:** `docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md`, section "Agent" and lot 2 in "Lots". Oku: t-1152 › t-1154 (lot 2) › t-1173 to t-1177. Starts from `main` at 26bbbcaf (lot 1 merged), branch `onglets-lot2-agent`.

**Scope of lot 2, versus the spec:** the spec's "Agent" section also filters MCP tools by document kind and adds `open_pack` / `open_theme`; that is lot 5. The context line "document kind, selected item, preview theme" needs `TabDocument` and is lot 3. Old per-scope files (`slides.json`, `atelier.*.json`, `widget.*.json`) in `agent/` are left on disk and ignored, not moved aside: the resume code has no single-file set-aside helper and the spec's "set aside" adds nothing over ignoring them (no shim reads them, nothing overwrites them). The conversation of the deck is therefore lost once on upgrade (breaking change, `feat!`).

## Global Constraints

- Commits: `type(scope) : description (t-N)`, in French, a space before the colon, `!` for breaking; never a `Co-Authored-By` line. `git add` in its own call, then `git diff --cached --stat`, then `git commit -F <message file in the scratchpad>`. Under SDD the controller commits after the task review (the commit-review hook needs a `Reviewed-by` trailer); implementers stage only.
- The workspace build is expected to break between Task 1 and Task 3 (the session's API changes first, its dependents follow). Each task's own crate tests must pass at its end; `cargo clippy --workspace --all-targets -- -D warnings` must be clean at the end of Task 3 and of every later task; `just check` and `just test` at the end of Task 5.
- No `unwrap`/`expect` outside tests; no silent fallback; mutexes with `lock().unwrap_or_else(PoisonError::into_inner)`; never hold a std Mutex guard across an await; synchronous session work in `spawn_blocking`.
- No `std::fs` in production code (Storage). Resume files through `Storage::write_atomic`, failures logged with `tracing::warn`, never failing the request.
- Every pub item and new module documented, first sentence under 15 words; code comments in French; no dead code; `#[expect]` instead of `allow`, never for dead code.
- Unit tests in sibling `x_tests.rs` via `#[cfg(test)] #[path]`; speculoos; names `unit_scenario_result`; one behavior per test; rstest for tables; temp dirs via tempfile (`test_support::fresh_dir`). UI: `*.render.test.tsx` for happy-dom, other `*.test.ts(x)`.
- Mirror every `Request`/`AgentCommand` change in `ui/src/api.ts` (the UI side lands in Task 4; the e2e helper posts in Task 5).
- Fluent: a removed key is removed from **all 20 locales**; English is the reference.
- A change that voids a rule edits the nearest `AGENTS.md` **in the same commit**; each `ARCHITECTURE.md` stays ≤ 50 lines, current, and committed with the code it describes.
- e2e: `just e2e <port>` only (it sets fresh data dirs); specs read French labels.
- Do not set `CARGO_TARGET_DIR`.

## File Structure

- `crates/oku-slides-session/src/agent/scope.rs`, `scope_tests.rs` — deleted.
- `crates/oku-slides-session/src/agent/conversation.rs` (+ `conversation_tests.rs`) — one conversation per session, no rename tracking.
- `crates/oku-slides-session/src/resume_agent.rs`, `resume_writer.rs` (+ tests), `resume.rs`, `undo.rs`, `widgets_edit.rs`, `agent/mod.rs`, `lib.rs` — follow.
- `crates/oku-slides-session/tests/regressions/issue_0613_annuler_un_renommage_ramene_la_conversation.rs` — deleted (behavior gone), with its `main.rs` line.
- `crates/oku-slides-mcp/src/hub_scopes.rs` → `hub_thread.rs` (+ `hub_scopes_tests.rs` → `hub_thread_tests.rs`); `hub.rs`, `hub_turns.rs`, `surface_tools.rs`, `tool_error.rs`; `tests/{receipts,surfaces,themes,turns}.rs`.
- `crates/oku-slides-agent/src/{keeper,tab_agent,turn,turns,events,tab_entry,stack}.rs` (+ their tests), `tests/{stack,turns,common/mod}.rs`, `tests/regressions/issue_0611…`, `issue_0612…`, `examples/smoke.rs`; `tests/scopes.rs` deleted.
- `ui/src/agent/scope.ts`, `scope.test.ts` deleted; `useAgentScope.ts` → `useAgentThread.ts`, `useFollowScope.ts` → `useFollowThread.ts` (+ render tests); `AgentPanel.tsx`, `App.tsx`, `AgentWindow.tsx`, `api.ts`.
- `ui/e2e/070-agent-scopes.cjs` deleted; `ui/e2e/080-agent-window.cjs` follows.
- Fluent: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-error-anchor-scope`), `*/ui.ftl` (`ui-agent-busy-elsewhere`, `ui-agent-scope-slides`, `ui-agent-scope-atelier`).
- Docs: `ARCHITECTURE.md` and `AGENTS.md` of `oku-slides-session`, `oku-slides-mcp`, `oku-slides-agent`, `ui`; `docs/superpowers/specs/2026-10-08-sessions-par-perspective-design.md` (superseded note).

---

### Task 1: One conversation per session (oku-slides-session) (t-1173)

**Files:**
- Delete: `crates/oku-slides-session/src/agent/scope.rs`, `crates/oku-slides-session/src/agent/scope_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/conversation.rs`, `conversation_tests.rs`, `agent/mod.rs`
- Modify: `crates/oku-slides-session/src/resume_agent.rs`, `resume_writer.rs`, `resume_writer_tests.rs`, `resume.rs` (restore ~line 679, prune call ~636, `persist_with` ~762), `lib.rs` (the `conversations` field)
- Modify: `crates/oku-slides-session/src/undo.rs:30-35`, `crates/oku-slides-session/src/widgets_edit.rs:382-431`
- Delete: `crates/oku-slides-session/tests/regressions/issue_0613_annuler_un_renommage_ramene_la_conversation.rs` and its declaration in `tests/regressions/main.rs`
- Modify: `crates/oku-slides-session/ARCHITECTURE.md` (the `agent/` bullet), `crates/oku-slides-session/AGENTS.md` (rule "Move a widget conversation on rename…")

**Interfaces:**
- Produces (used by Tasks 2 and 3):

```rust
/// Une conversation : son fil et sa session ACP.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedConversation {
    #[serde(default)] pub agent: Option<String>,
    #[serde(default)] pub session: Option<String>,
    #[serde(default)] pub thread: Vec<Surface>,
}

impl Session {
    pub fn conversation(&self) -> Option<&SavedConversation>;
    pub fn keep_thread(&mut self, thread: Vec<Surface>);
    pub fn keep_agent_session(&mut self, session: String, agent: String);
    /// Nouvelle conversation : oubliée, son fichier effacé ; rend sa session.
    pub fn forget_conversation(&mut self) -> Option<String>;
}
```

- Removed: `AgentScope`, `ScopeMove`, `Session::{conversations, take_scope_moves}`, and the crate-private `conversation_renamed`, `widgets_before`, `follow_widget_rename`, `prune_conversations`, `WidgetsBefore`, `Conversations`, `writable`, plus `widget_known`, `pack_files_known`, `pack_files_held` **if** grep shows no other caller after the removal (check, do not assume). `renamed_source` stays (used by `rename_widget`).

- [ ] **Step 1: Write the failing tests**

Replace `crates/oku-slides-session/src/agent/conversation_tests.rs` with tests on the new API (keep its existing imports/helpers that still apply; read the old file first for how a session with resume is built — it uses `test_support` and a resume dir):

```rust
#[test]
fn keep_thread_then_conversation_returns_the_thread() {
    let mut s = session();
    let thread = vec![thread_surface(1)];
    s.keep_thread(thread.clone());
    assert_that!(s.conversation().map(|c| c.thread.clone())).is_equal_to(Some(thread));
}

#[test]
fn keep_agent_session_records_session_and_agent() {
    let mut s = session();
    s.keep_agent_session("sess-1".into(), "agent-a".into());
    let c = s.conversation().cloned().unwrap();
    assert_that!((c.session, c.agent)).is_equal_to((Some("sess-1".to_owned()), Some("agent-a".to_owned())));
}

#[test]
fn forget_conversation_returns_its_session_and_clears_it() {
    let mut s = session();
    s.keep_agent_session("sess-1".into(), "agent-a".into());
    assert_that!(s.forget_conversation()).is_equal_to(Some("sess-1".to_owned()));
    assert_that!(s.conversation()).is_none();
}

#[test]
fn forget_conversation_without_one_returns_none() {
    let mut s = session();
    assert_that!(s.forget_conversation()).is_none();
}

/// Écrite dans `agent/conversation.json`, relue par un onglet rouvert.
#[test]
fn conversation_survives_resume_in_conversation_json() {
    // Build a session with resume enabled on a fresh dir (same helper the old tests used),
    // keep_thread + keep_agent_session, flush the resume writer, assert the file
    // `<dir>/agent/conversation.json` exists, reopen a session on the same dir and
    // assert conversation() equals what was kept.
}

/// Les fichiers par portée d'avant restent sur le disque, ignorés.
#[test]
fn resume_ignores_old_per_scope_files() {
    // Write `<dir>/agent/slides.json` (version 1, old shape with "scope") before reopening;
    // assert conversation() is None after reopening and the file is still there.
}
```

`thread_surface(n)` is a `Surface` with `place: Place::Thread`, `turn: Some(n)`, `live: false` — reuse whatever helper the old test file or `test_support` provides to build a thread surface; add a small local helper otherwise. Write the two resume tests' bodies fully, following the old file's resume tests (they did exactly this per scope).

In `resume_writer_tests.rs`, change the cases that built `Job { agent: BTreeMap… }` to the new `agent: Option<AgentWrite>` (Step 3) and keep their intent: a later job's conversation write wins in `merge`; a `Delete` after a `Write` deletes.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-session conversation resume_writer`
Expected: compile errors (`keep_thread` takes 2 arguments, `AgentWrite` unknown, …).

- [ ] **Step 3: Implement**

`agent/conversation.rs` — new content:

```rust
//! La conversation de l'agent gardée par la session : une par onglet.
//!
//! Écrite dans `agent/conversation.json` du dossier de reprise. La session est
//! le seul écrivain ; dans l'agent, le gardien de l'onglet le seul appelant.

use serde::{Deserialize, Serialize};

use super::Surface;
use crate::Session;

/// Une conversation : son fil et sa session ACP.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedConversation {
    /// Empreinte de l'agent qui tient la session (`Link::agent_id`) ; opaque ici.
    #[serde(default)]
    pub agent: Option<String>,
    /// Identifiant de la session ACP.
    #[serde(default)]
    pub session: Option<String>,
    /// Les surfaces figées du fil, sans actions.
    #[serde(default)]
    pub thread: Vec<Surface>,
}

impl Session {
    /// La conversation de l'onglet, si elle existe.
    pub fn conversation(&self) -> Option<&SavedConversation> {
        self.conversation.as_ref()
    }

    /// Le fil (déjà sans surface vivante ni action) ; écrit pour la reprise.
    pub fn keep_thread(&mut self, thread: Vec<Surface>) {
        self.conversation.get_or_insert_with(SavedConversation::default).thread = thread;
        self.persist_conversation();
    }

    /// La session ACP et l'empreinte de son agent ; écrites pour la reprise.
    pub fn keep_agent_session(&mut self, session: String, agent: String) {
        let c = self.conversation.get_or_insert_with(SavedConversation::default);
        c.session = Some(session);
        c.agent = Some(agent);
        self.persist_conversation();
    }

    /// Nouvelle conversation : oubliée, son fichier effacé ; rend sa session.
    pub fn forget_conversation(&mut self) -> Option<String> {
        let gone = self.conversation.take()?;
        self.persist_agent(None);
        gone.session
    }

    fn persist_conversation(&mut self) {
        let saved = self.conversation.clone();
        self.persist_agent(saved);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;
```

`Session` field: replace `conversations: Conversations` with `pub(crate) conversation: Option<SavedConversation>` (initialized `None`; every `self.conversations = Default::default()` becomes `self.conversation = None`).

`resume_agent.rs`:

```rust
//! La conversation de l'agent dans le dossier de reprise : `agent/conversation.json`.

/// Sous-dossier de la conversation, dans le dossier de reprise de l'onglet.
pub(crate) const AGENT_DIR: &str = "agent";
/// Fichier de la conversation de l'onglet.
const CONVERSATION_FILE: &str = "conversation.json";
/// Version du fichier de la conversation ; la 1, une par portée, n'est plus relue.
pub(crate) const CONVERSATION_VERSION: u32 = 2;

impl Resume {
    pub(crate) fn write_conversation(&self, c: &SavedConversation) -> Result<()> { /* write_json(AGENT_DIR/CONVERSATION_FILE, {version, ..c}) */ }
    pub(crate) fn delete_conversation(&self) -> Result<()> { /* storage.delete(dir/AGENT_DIR/CONVERSATION_FILE) */ }
    /// La conversation relue ; illisible, journalisée et ignorée.
    pub(crate) fn read_conversation(&self) -> Option<SavedConversation> { /* read_json::<SavedConversation>(…, CONVERSATION_VERSION): Ok(Some(c)) => Some(c), Ok(None) => None, Err(e) => warn + None */ }
}

impl Session {
    /// La conversation à écrire (`Some`) ou effacer (`None`), avec l'état courant.
    pub(crate) fn persist_agent(&mut self, saved: Option<SavedConversation>) {
        let write = match saved { Some(c) => AgentWrite::Write(c), None => AgentWrite::Delete };
        self.persist_with(false, Some(write));
    }
}
```

Keep the existing `ConversationOut { version, #[serde(flatten)] conversation }` writer shape.

`resume_writer.rs`:

```rust
/// Ce qu'il faut faire de la conversation de l'agent.
#[derive(Debug, Clone)]
pub(crate) enum AgentWrite {
    /// L'écrire.
    Write(SavedConversation),
    /// L'effacer.
    Delete,
}
// Job.agent: Option<AgentWrite>   (doc: "La conversation à écrire ou effacer ; `None` : inchangée.")
// merge: `agent: next.agent.or(prev.agent)`
// write(): match &job.agent { Some(AgentWrite::Write(c)) => resume.write_conversation(c), Some(AgentWrite::Delete) => resume.delete_conversation(), None => Ok(()) } then the same warn on error.
```

`resume.rs`: `persist_with(&mut self, documents: bool, agent: Option<AgentWrite>)`; `persist` passes `None`; `restore` sets `self.conversation = resume.read_conversation();`; delete the `self.prune_conversations();` call.

`undo.rs` `replay`: delete `let before = self.widgets_before();` and `self.follow_widget_rename(before);`.

`widgets_edit.rs` `rename_widget`: delete the `to_existed` computation and the `self.conversation_renamed(...)` call. Then grep `widget_known`, `pack_files_known`, `pack_files_held`: delete each that has no caller left.

`agent/mod.rs`: drop `pub mod scope;`, `pub use scope::AgentScope;`, and `ScopeMove` from `pub use conversation::{…}`; update its module doc if it names scopes.

- [ ] **Step 4: Delete the obsolete regression and run the crate**

Delete `issue_0613_annuler_un_renommage_ramene_la_conversation.rs` and its `mod` line in `tests/regressions/main.rs`. Grep the crate for `AgentScope`, `ScopeMove`, `scope_moves`, `conversations()`: none may remain.

Run: `cargo nextest run -p oku-slides-session`
Expected: PASS. (`oku-slides-mcp`, `oku-slides-agent` do not compile yet: Tasks 2-3.)

- [ ] **Step 5: Docs and rules (same commit)**

`crates/oku-slides-session/ARCHITECTURE.md`, `agent/` bullet: replace `` `scope` + `conversation` (per-scope threads; `ScopeMove` on widget rename, replayed by `undo.rs`) `` with `` `conversation` (one thread and ACP session per tab, `agent/conversation.json`) `` — line count unchanged (≤ 50). The `<data>/resume/` paragraph: `agent/<scope>.json` → `agent/conversation.json`.

`crates/oku-slides-session/AGENTS.md`: delete the rule "Move a widget conversation on rename, undo or redo, never overwriting one; refuse keep_thread and keep_agent_session for unknown Widget scopes. (plan:2026-10-08-sessions-par-perspective)". Keep "Define KEPT_TURNS only once…".

`cargo fmt -p oku-slides-session`; stage; message: `feat(agent)! : une conversation par onglet dans la session, AgentScope supprimé (t-1173)`.

---

### Task 2: One thread per hub (oku-slides-mcp) (t-1174)

**Files:**
- Rename: `crates/oku-slides-mcp/src/hub_scopes.rs` → `hub_thread.rs`, `hub_scopes_tests.rs` → `hub_thread_tests.rs` (`git mv`), module declaration in `hub.rs:338-341`
- Modify: `crates/oku-slides-mcp/src/hub.rs` (`McpEvent::Viewing` ~line 78, `HubState` ~144-146, `resync`/`viewing` ~247-260), `hub_turns.rs` (`begin_turn` ~29-40, end of turn ~208, `clear_thread` ~214-232), `surface_tools.rs:50-63,127`, `tool_error.rs:38,99`
- Modify: `crates/oku-slides-mcp/tests/{receipts,surfaces,themes,turns}.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (remove `agent-error-anchor-scope` in all 20 locales)
- Modify: `crates/oku-slides-mcp/AGENTS.md` (rule "Leave thread persistence to the tab keeper; the hub files turns by scope…"), `crates/oku-slides-mcp/ARCHITECTURE.md` if it names scopes or `hub_scopes`

**Interfaces:**
- Consumes: `SavedConversation` (no `scope`) from Task 1.
- Produces (used by Task 3):

```rust
// HubState: `thread: BTreeSet<u32>` (the turns of the tab's thread) replaces
// `turn_scopes: HashMap<u32, AgentScope>`; `viewing` is removed.
impl SurfaceHub {
    pub fn begin_turn(&self) -> u32;
    pub fn clear_thread(&self);
    pub fn resync(&self);                         // emits the shown list, no Viewing event
    pub fn in_thread(&self, turn: u32) -> bool;   // replaces turn_scope(turn).is_some()
    pub fn restore(&self, saved: Option<SavedConversation>);
    pub fn thread_snapshot(&self) -> Option<Vec<Surface>>;
    pub fn visible_surfaces(&self) -> Vec<Surface>; // unchanged
}
// Removed: McpEvent::Viewing, SurfaceHub::{viewing, turn_scope, rename_scope},
// ToolError AnchorScope and check_anchor_scope.
```

Semantics to keep, now without scopes: a surface is shown when it is live, outside a turn, or its turn is in `thread` (a turn the hub never filed is not shown); `begin_turn` files the turn in `thread` and closes the oldest turns of the thread beyond `KEPT_TURNS`; `clear_thread` releases the questions of the cleared turns and closes their surfaces; `restore` keeps only sanitized thread surfaces (`cleaned`), the last `KEPT_TURNS` turns, skips a turn already open and logs it; `thread_snapshot` returns the frozen surfaces of closed turns within `THREAD_BYTES`, `None` when nothing fits.

- [ ] **Step 1: Rename the module files** (`git mv` both) and adapt the `mod` line: `#[path = "hub_thread.rs"] mod thread; pub use thread::THREAD_BYTES;`. Update the module doc: «Le fil de l'onglet : surfaces montrées, restauration et image du fil.»

- [ ] **Step 2: Write the failing tests**

Rewrite `hub_thread_tests.rs` from the old scope tests, keeping every behavior that does not depend on two scopes, with the new signatures, and delete those that only made sense with two scopes (a turn of another scope hidden, rename_scope, viewing). The kept behaviors, one test each:

```rust
#[test] fn restore_shows_saved_thread_frozen_without_actions() { /* restore(Some(conv with a thread surface with an action)) → visible_surfaces() has it, live false, no actions */ }
#[test] fn restore_keeps_only_the_last_kept_turns() { /* KEPT_TURNS + 3 turns saved → only the last KEPT_TURNS restored */ }
#[test] fn restore_skips_a_turn_already_open() { /* begin_turn() then restore a surface with that turn → ignored */ }
#[test] fn restore_drops_surfaces_outside_the_thread_place() { /* place != Thread → dropped */ }
#[test] fn begin_turn_files_the_turn_in_the_thread() { /* n = begin_turn(); in_thread(n) */ }
#[test] fn begin_turn_beyond_kept_turns_closes_the_oldest() { /* KEPT_TURNS+1 begin/end → first not in_thread, its surfaces closed */ }
#[test] fn clear_thread_forgets_every_turn() { /* turns → clear_thread() → !in_thread(any), visible_surfaces() empty of thread surfaces */ }
#[test] fn thread_snapshot_returns_closed_turns_only() { /* open turn excluded */ }
#[test] fn thread_snapshot_over_budget_returns_none() { /* one turn bigger than THREAD_BYTES → None */ }
#[test] fn surface_outside_any_turn_is_shown() { /* turn None → visible */ }
```

Write each body fully from the corresponding old test (they exist with a scope argument); only the scope arguments and the scope-specific assertions go.

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-mcp hub_thread`
Expected: compile errors on the new signatures.

- [ ] **Step 4: Implement** the interface above in `hub.rs`, `hub_thread.rs`, `hub_turns.rs`. In `hub_thread.rs`: `shown(state, live, turn) = live || turn.is_none_or(|t| state.thread.contains(&t))`; `restore` iterates `saved.into_iter().flat_map(|c| c.thread)` with the existing `cleaned` / `KEPT_TURNS` / conflict logic where "taken" becomes `state.thread.contains(&t) || state.live_turns.contains(&t) || state.turn == Some(t)`; `thread_snapshot` filters on `state.thread.contains(&t)`. `resync()` keeps what it did minus setting `viewing` and recording `McpEvent::Viewing`.

`surface_tools.rs`: delete `check_anchor_scope` and its two calls; `check_anchor` only calls `self.with_session(move |s| s.check_anchor(&anchor))`; update the doc comment of `ask_rewrite`. `tool_error.rs`: delete the `AnchorScope` variant and its localize arm. Remove `agent-error-anchor-scope` from `agent.ftl` in every locale under `crates/oku-slides-i18n/src/locales/`.

Ruling recorded in the plan: until lot 3 the workshop's widget sub-tab shares the deck thread, so an anchored question asked while a widget sub-tab is shown may anchor on a slide the user does not see; `Session::check_anchor` still refuses an anchor the document cannot resolve. Lot 3 removes sub-tabs.

- [ ] **Step 5: Adapt the integration tests** `tests/receipts.rs`, `tests/surfaces.rs`, `tests/themes.rs`, `tests/turns.rs`: every `begin_turn(AgentScope::Slides)` → `begin_turn()`, `resync(AgentScope::Slides)` → `resync()`, drop `Viewing` expectations; delete tests whose subject is a second scope; delete any test of `AnchorScope`.

Run: `cargo nextest run -p oku-slides-mcp` — Expected: PASS. Run `cargo nextest run -p oku-slides-i18n` — Expected: PASS (locales stay in sync).

- [ ] **Step 6: Docs and rules (same commit)**

`crates/oku-slides-mcp/AGENTS.md`: replace "Leave thread persistence to the tab keeper; the hub files turns by scope and sends only the viewed thread. (plan:2026-10-08-sessions-par-perspective)" with "Leave thread persistence to the tab keeper; the hub keeps one thread per tab and sends only shown surfaces. (spec:2026-10-24-onglets-pack-theme-design, t-1154)". Fix `ARCHITECTURE.md` mentions of scopes / `hub_scopes` if any (≤ 50 lines).

`cargo fmt -p oku-slides-mcp`; stage; message: `feat(agent)! : un seul fil par hub, portées et refus d'ancre de widget supprimés (t-1174)`.

---

### Task 3: Keeper, actor, events and stack without scope (oku-slides-agent) (t-1175)

**Files:**
- Modify: `crates/oku-slides-agent/src/keeper.rs`, `tab_agent.rs` (Command/Instruction ~30-48, `prompt_in` ~126-131, `new_conversation` ~146-212, `prompt` ~215-275), `turn.rs` (~8-23, 50-120), `turns.rs` (~100-110, 340-372, 445-480), `events.rs` (~30-120), `tab_entry.rs` (~60-120, 200-240, 250-340, 360-370), `stack.rs` (~35-90, 235-340, 440-500)
- Modify tests: `src/events_tests.rs`, `src/turns_tests.rs`, `src/tab_entry_tests.rs`, `tests/stack.rs`, `tests/turns.rs`, `tests/common/mod.rs`, `tests/regressions/issue_0611_memory_lost_only_on_refused_resume.rs`, `tests/regressions/issue_0612_late_load_replay_ignored.rs`, `examples/smoke.rs`
- Delete: `tests/scopes.rs` (and its `[[test]]` entry in `Cargo.toml` if there is one)
- Modify: `crates/oku-slides-agent/AGENTS.md`, `crates/oku-slides-agent/ARCHITECTURE.md`

**Interfaces:**
- Consumes: Task 1 session API, Task 2 hub API.
- Produces (wire format the UI uses in Task 4 — serde as today, minus the fields):

```rust
pub enum AgentCommand {           // stack.rs
    Prompt { tab: String, text: String, ui: AgentUi, answer: Option<String>, images: Vec<…> },
    NewConversation { tab: String },
    Resync { tab: String },
    // other variants unchanged; the `scope` field and its `#[serde(default)]` disappear everywhere
}
pub enum AgentEvent {             // events.rs
    Status { state: AgentState },             // `scope` removed
    UserText { turn: u32, text: String, images: … },  // `scope` removed
    Conversation,                              // was `Conversation { scope }`: JSON {"kind":"conversation"}
    // others unchanged
}
pub enum Keep {                   // keeper.rs
    Thread(u32),
    Session { turn: u32, id: String, agent: String },
    Forget(oneshot::Sender<Option<String>>),
    Lookup(oneshot::Sender<Option<(String, Option<String>)>>),
    Show,
    Flush(oneshot::Sender<()>),
}
// Keeper::{lookup(&self), forget(&self)} without scope. Removed: Keep::{Replay, Sync}, all rename handling.
```

Check how `AgentEvent::Conversation` is serialized today (tag/rename attributes on the enum) and keep the tag; a unit variant must serialize as `{"kind":"conversation"}` — add a serde test for it in `events_tests.rs`.

- [ ] **Step 1: Write the failing tests**

- `events_tests.rs`: `status_serializes_without_scope` (`{"kind":"status","state":"idle"}` or the crate's exact casing — copy from the existing status test), `conversation_serializes_as_bare_kind`.
- `turns_tests.rs`: replace scope-based tests with `conversation_event_clears_every_turn` and `kept_turns_cap_counts_all_turns`; adapt every `UserText { scope, … }` / `Status { scope, … }`.
- `tests/stack.rs`: the restore-after-restart scenarios without scope — `resync_after_restart_shows_the_saved_thread`, `new_conversation_forgets_thread_and_session`, `prompt_after_restart_resumes_the_saved_session` (these exist per scope; keep one per behavior for the single thread). Delete `tests/scopes.rs`.
- `tests/common/mod.rs`: helpers lose their scope parameters.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-agent`
Expected: compile errors.

- [ ] **Step 3: Implement**

`keeper.rs`: the interface above. `run` restores with `t.with(|s| s.conversation().cloned())` then `h.restore(saved)`. `apply` loses the `moves`/`follow` prelude entirely:
- `Thread(turn)`: `if !hub.in_thread(turn) { Ok(()) }` (turn cleared meanwhile) `else match hub.thread_snapshot() { Some(thread) => tab.with(|s| s.keep_thread(thread)), None => { warn "fil trop lourd pour le budget : enregistrement sauté"; Ok(()) } }`.
- `Session { turn, id, agent }`: `if hub.in_thread(turn) { tab.with(|s| s.keep_agent_session(id, agent)) } else { debug; Ok(()) }`.
- `Forget(reply)`: `forget_conversation()`, then `hub.clear_thread()`, reply.
- `Lookup(reply)`: `s.conversation().and_then(|c| Some((c.session.clone()?, c.agent.clone())))`.
- `Show`: `hub.resync()`.
Module doc: «l'ordre de la file règle les courses (écriture puis oubli, oubli puis lecture)».

`tab_agent.rs`: `Command::NewConversation` (unit); `Instruction` without `scope`; `prompt_in` removed (callers use `prompt`); `new_conversation(&mut self)`; `prompt` uses `begin_turn()`, `keeper.lookup()`, events without scope.

`turn.rs`: `pending.new_conversation: bool` replaces the `BTreeSet<AgentScope>`; a `NewConversation` arriving during a turn takes today's same-scope path (the `elsewhere` distinction disappears) — read the current branch and keep its same-scope behavior exactly. `tab_agent::prompt` then calls `self.new_conversation().await` once if `pending.new_conversation`.

`turns.rs`: turn records lose `scope`; `AgentEvent::Conversation` clears all turns; the `KEPT_TURNS` cap counts all turns; the fallback `scope` at ~371 disappears.

`tab_entry.rs`: `status: Arc<Mutex<AgentState>>`; remove every `keeper.send(Keep::Sync)` and the `McpEvent::Viewing` emit (~232); the queued-prompt path uses `hub.begin_turn()`; `Command::NewConversation` arm without scope.

`stack.rs`: `AgentCommand` as above; `Resync { tab }` → `keeper.send(Keep::Show)`; the follow path that sent `Keep::Replay` sends `Keep::Show`; drop the `Keep::Sync` send (~446) and the `McpEvent::Viewing` emit (~500); every `Instruction { scope: AgentScope::Slides, … }` loses the field.

- [ ] **Step 4: Run**

Run: `cargo nextest run -p oku-slides-agent` — Expected: PASS.
Run: `cargo clippy --workspace --all-targets -- -D warnings` — Expected: clean (the workspace compiles again; `src-tauri` and `oku-slides-cli` must build — fix any caller of the removed APIs there).
Run: `cargo nextest run --workspace --exclude oku-slides-app` — Expected: PASS.

- [ ] **Step 5: Docs and rules (same commit)**

`crates/oku-slides-agent/AGENTS.md`:
- delete "Let the UI compute and send the agent scope; never derive it from the session mode on the host. (spec:2026-10-08-sessions-par-perspective-design)";
- replace "Keep one ACP session and thread per scope (Slides, Workshop, Widget name), restored on reopen. (t-598)" with "Keep one ACP session and thread per tab, restored on reopen. (spec:2026-10-24-onglets-pack-theme-design, t-1154)";
- replace "When the MCP relay lags, resend only visible_surfaces, the thread being viewed. (plan:…)" with "When the MCP relay lags, resend only visible_surfaces. (plan:2026-10-08-sessions-par-perspective)";
- keep the keeper, resume-routing and fingerprint rules.
`ARCHITECTURE.md`: remove scope wording (≤ 50 lines).

`cargo fmt`; stage; message: `feat(agent)! : gardien, acteur et commandes de l'agent sans portée, un fil par onglet (t-1175)`.

---

### Task 4: The UI talks to one thread (ui) (t-1176)

**Files:**
- Delete: `ui/src/agent/scope.ts`, `ui/src/agent/scope.test.ts`
- Rename + rewrite: `ui/src/agent/useAgentScope.ts` → `useAgentThread.ts`, `useAgentScope.render.test.tsx` → `useAgentThread.render.test.tsx`; `useFollowScope.ts` → `useFollowThread.ts`, `useFollowScope.render.test.tsx` → `useFollowThread.render.test.tsx`
- Modify: `ui/src/api.ts` (~569, 583, 625, 638-674), `ui/src/agent/AgentPanel.tsx` (~20-30, 85-90, 145-165, 225-265, 310-320), `ui/src/agent/AgentPanel.render.test.tsx`, `ui/src/App.tsx` (~232, 464 and where `scopeSource` is built), `ui/src/AgentWindow.tsx` (~251)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (remove `ui-agent-busy-elsewhere`, `ui-agent-scope-slides`, `ui-agent-scope-atelier` in all 20 locales)
- Modify: `ui/AGENTS.md` / `ui` docs only if they name scopes of the agent (the rule "Send surface actions to the scope of the displayed surface…" is about surfaces, keep it).

**Interfaces:**
- Consumes: Task 3 wire format.
- Produces:

```ts
// api.ts
| { kind: "status"; state: AgentState }                     // AgentEvent
| { kind: "conversation" }                                   // AgentEvent
// McpEvent: the "viewing" member is removed; `AgentScope` type removed.
| { cmd: "prompt"; tab: string; text: string; ui: AgentUiSent; answer: string | null; images: string[] }
| { cmd: "new_conversation"; tab: string }
| { cmd: "resync"; tab: string }
// agentApi(tab): prompt(text, ui, answer, images = []), newConversation(), resync()

// agent/useAgentThread.ts
export const IDLE: AgentState-based status;  // the reducer keeps only `state`
export interface AgentThreadState {
  state: AgentState;
  ready: boolean;          // last resync succeeded (unchanged meaning)
  onAgentEvent: (e: AgentEvent) => void;
  onReady: () => void;     // stable; first call arms, later calls resync
}
export function useAgentThread(agent: { resync: () => Promise<void> }, loaded: boolean, clearSurfaces: (e: McpEvent) => void): AgentThreadState;
// agent/useFollowThread.ts
export function useFollowThread(agent: Pick<ReturnType<typeof agentApi>, "follow">): AgentThreadState; // ready = followed
```

`useAgentThread`: resync once when the subscription is ready and the snapshot is loaded (surfaces cleared first), and on every reconnection (`onReady` after the first); there is no scope key any more, so nothing re-triggers on mode changes. `useFollowThread`: no `viewing` wait, `ready` is the last `follow()` success; it no longer needs `onMcp`.

`AgentPanel`: its `scope` prop becomes `thread: { kind: "drive"; loaded: boolean } | { kind: "follow" }`; every `elsewhere`/`elsewhereNotice` branch goes (prompts and suggested prompts are never refused for another scope; `cancellable = busy`; the thread's state is `agentState`). `App.tsx` builds `{ kind: "drive", loaded: snap !== null }` instead of a `ScopeView`; `AgentWindow.tsx` passes `{ kind: "follow" }`.

- [ ] **Step 1: Write the failing tests**

`useAgentThread.render.test.tsx` (from the old `useAgentScope` tests, scope cases removed):
- `useAgentThread resyncs once when ready and loaded`
- `useAgentThread does not resync before the snapshot is loaded`
- `useAgentThread resyncs again on reconnection`
- `useAgentThread ready is false when resync fails`
- `useAgentThread status follows status events`

`useFollowThread.render.test.tsx`:
- `useFollowThread is ready after follow succeeds`
- `useFollowThread a later follow failure makes it not ready`

`AgentPanel.render.test.tsx`: delete the cases about "répond ailleurs" and `viewing`; add `AgentPanel sends a prompt without scope` (the agent fake receives `prompt("…", ui, null, [])`) and `AgentPanel new conversation calls newConversation without arguments`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd ui && bun test src/agent` and the render runner (`bun run test:render` or the command `just test` uses for `*.render.test.tsx` — read `justfile`)
Expected: FAIL / type errors.

- [ ] **Step 3: Implement** the interface above; delete `scope.ts` and its test; remove the three Fluent keys from `ui.ftl` in every locale.

- [ ] **Step 4: Run**

Run: `cd ui && bun run typecheck` (or `tsc -b`) — Expected: clean.
Run: the two UI test processes — Expected: PASS.
Run: `cargo nextest run -p oku-slides-i18n` — Expected: PASS.

- [ ] **Step 5: Stage**; message: `feat(ui)! : le panneau de l'agent parle au fil de l'onglet, plus de portée ni de « répond ailleurs » (t-1176)`.

---

### Task 5: e2e, superseded spec, full gate (t-1177)

**Files:**
- Delete: `ui/e2e/070-agent-scopes.cjs`
- Modify: `ui/e2e/080-agent-window.cjs:26,68` (and any e2e or `ui/e2e/lib/` helper that posts `scope`; grep)
- Modify: `docs/superpowers/specs/2026-10-08-sessions-par-perspective-design.md` — add under the title: `> Superseded by docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md (t-1152): one conversation per tab, no scopes.`

- [ ] **Step 1:** Delete `070-agent-scopes.cjs`. In `080-agent-window.cjs`, post `{ cmd: 'new_conversation', tab: 't-1' }`. Grep `ui/e2e` for `scope` and `atelier` agent expectations (e.g. a note "L'agent répond dans…"): remove them. Later specs relied on earlier ones only through shared server state: check that no spec after 070 depended on state 070 created (read 070's setup; if a later spec used a pack or widget it created, move that setup into the later spec).

- [ ] **Step 2:** Run `just e2e 4999` — Expected: all specs pass.

- [ ] **Step 3:** Add the superseded note to the old spec.

- [ ] **Step 4:** Run `just check` and `just test` — Expected: PASS.

- [ ] **Step 5:** Stage; message: `test(e2e) : portées de l'agent retirées des specs, spec des sessions par perspective remplacée (t-1177)`.

---

## After lot 2

Lot 3 (t-1155, session and workspace: `TabDocument`, pack and theme tabs) gets its own plan, written on top of the merged lot 2: `docs/superpowers/plans/2026-10-24-onglets-pack-theme-lot3-<slug>.md`.
