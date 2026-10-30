# Packs and themes as tabs — lot 7: the agent's tools by tab kind — Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each tab's agent sees only the MCP tools of its tab's kind (deck, help, pack, theme). A deck's agent opens a pack or a theme in its own tab with `open_pack` / `open_theme`, at most one tab move per turn. A pack tab's agent selects items, chooses the preview theme and saves copies. A theme tab's agent renames the theme (one undo step) and saves copies. Each kind gets its own instructions paragraph.

**Architecture:** The kind is fixed when a session is created. The agent stack resolves it once per tab (`Sessions::document_kind`, which both workspace hosts answer through `Workspace::document_kind` without waking the tab), then builds that tab's `Tools` and its turns with it. `tools::list_all()` is one table of `(Tool, Kinds)`; `list_for(kind)` filters it, and `Tools::call` refuses an unlisted tool with `UnknownTool`. The agent gate of lot 3 (`PackGate`, `reserve_pack_tab`, `Verdict`) becomes a tab gate shared by packs and themes (`TabGate`, `reserve_tab`, `TabVerdict`): one claim per turn spent by a new tab or by a focus, and its three known bugs are fixed. `edit_pack` is renamed `open_pack`; `open_theme` reaches the workspace through a gated `Sessions::open_theme`.

**Tech Stack:** Rust 2024 (snafu, speculoos, rstest, nextest, rust-mcp-sdk 1.x), Tauri 2 host (`src-tauri`), Playwright e2e with the fake ACP agent, Fluent (20 locales). No React change in this lot.

**Spec:** `docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md`, sections "Agent" ("What the agent sees", the per-kind lists and `open_pack` / `open_theme`), "Testing" (`tool_list_filtered_by_document_kind`) and "Lots" (lot 7, the lot 3 note on agent-opened tabs). This plan's "Decisions" amend the spec where they differ; the plan commit adds "Implementation note (lot 7)" to it. Oku: t-1152 › t-1191; Task 1 t-1268, Task 2 t-1269, Task 3 t-1270, Task 4 t-1271, Task 5 t-1272, Task 6 t-1273, Task 7 t-1274, Task 8 t-1275, Task 9 t-1276.

**Base:** branch `fervent-mayer-9923eb` = `main` `3624341d` (lot 6 merged).

**Review:** v1 was reviewed by the 15-agent team (8 common reviewers, 4 Rust lenses, 3 TypeScript lenses) on 2026-10-29. v2 folds in every CRITICAL and MAJOR finding and the user's two answers: the theme rename is an undo step, and the focus policy is "active tab + one move".

**Out of this lot:** handing the rest of a request to the new tab's agent (the open stops there; follow-up ticket filed in Task 9); the pack and theme agent guides (t-1267); pack-tab session undo (t-1265); the other lot 6 follow-ups (t-1249..t-1255, t-1266); trimming app-wide tools from pack and theme tabs; the local agent's routing in pack and theme tabs (both noted in Task 9).

## Global Constraints

- **Commits:** Conventional Commits in English, `type(scope): description (t-N)`; every commit cites t-1191 and its task's sub-task; never a `Co-Authored-By` line. Under SDD implementers **stage only**. The controller commits after the task review, writing the message with Write to the session scratchpad and running a bare `git commit -F <file>`, with a trailer `Reviewed-by: SDD task review (spec + quality), round N, approved`. `oku.review=deferred` is set with `git config --worktree` at SDD setup and unset at the end. The first commit holds this plan and the spec note alone: `docs(plan): lot 7 of pack and theme tabs, the agent's tools by kind (t-1191)`, trailer `Reviewed-by: plan review team (15 agents), 2026-10-29, v2 approved by the user`.
- **Language:** code comments and docs in English. A French doc block in a file this lot edits is translated whole when it is touched (`agent/tabs.rs`, `edit_tools.rs`, `hub_turns.rs`, `tools.rs`, `tab_entry.rs`, `tests/common/mod.rs`, `src-tauri/src/state.rs` methods touched). User strings go through Fluent: a new key goes into **all 20 locales**, translated (never English copied), English first, same key order everywhere. A renamed or removed key leaves all 20 in the same commit (`crates/oku-slides-i18n/src/lib_tests.rs` `every_locale_has_exactly_the_reference_keys`). Agent-facing tool errors go through `Tools::fail_with` / `ToolError` (`crates/oku-slides-mcp/AGENTS.md`).
- **No manifest change** (no new dependency).
- **Rust:**
  - no `unwrap`/`expect` outside tests, no silent fallback (no `.ok().flatten().unwrap_or_default()`: propagate with `fail_with`);
  - Snafu errors with backtrace, localized, tested on the key plus `fr` (`Localizer::new(Some("fr"))`, `e.localize(&l)`);
  - `lock().unwrap_or_else(PoisonError::into_inner)`;
  - never a `std::sync::Mutex` guard across an await (session work goes in `spawn_blocking`);
  - pub items documented (first sentence under 15 words), new modules get a `//!` header, with `# Errors` / `# Blocking` headings;
  - `#[expect(…, reason)]`, never `allow`; new integration test files start with `#![expect(clippy::panic, reason = "test failures panic")]` as their siblings do;
  - files under 1000 lines (aim for 500). Current sizes: `tools.rs` 327, `edit_tools.rs` 376, `hub_turns.rs` 370, `workspace_themes.rs` ~140, MCP `tests/common/mod.rs` **836**. New MCP test helpers therefore go into `tests/common/workspace_harness.rs` or a new `tests/common/kinds.rs` submodule, never into `mod.rs`.
- **Lock order unchanged:** `OPENING` → workspace → one session (under the workspace, a session is only `try_lock`ed) → `DRAFTS` → `PACK_TABS`. The hub's state lock is a leaf. The gate closure takes it with only the opening lock held and never blocks. **The gate is never asked while the workspace guard is held:** read the holder in its own `let`, drop the guard, then ask. `Workspace::document_kind` takes the workspace lock alone and wakes nothing.
- **Rust tests:**
  - sibling `x_tests.rs` via `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`;
  - speculoos, `unit_scenario_result`, one behavior per test, rstest for tables;
  - MCP behaviour is tested on a real server on 127.0.0.1 port 0 with a rust-mcp-sdk client (`crates/oku-slides-mcp/tests/common`);
  - regressions go in `tests/regressions/issue_1191_<topic>.rs`, declared in `tests/regressions/main.rs`. Two exceptions: the session's bug-1 regression stays a unit test in `workspace_packs_race_tests.rs`, because its fixtures are `pub(in crate::workspace)` (Task 2); the MCP bug-3 regression is the unit test `tab_gate_tests::verdict_after_a_refused_focus_then_a_granted_new_tab_reports_no_refusal`, because `TabVerdict` is crate-private (Task 3). The plan records both;
  - temp dirs with `tempfile`;
  - reuse the real fixtures: session `packs()` / `Packs`, `packs_resumed()` + `restart`, `themes()` / `Themes`, `is_dormant`, `test_support::{session, session_on}`; MCP `common::guarded(h, tool, args, confirmed)`, `start_theme_tab`, `start_pack_tab`, `workspace_harness::{start_on_workspace, start_in_pack_tab, open_pack_tab, document_of, active_tab}`.
- **Mirrors:** no `McpEvent` variant is added (the `ui/src/api.ts` union is unchanged); the `ComponentKind` catalog is untouched.
- **Rules and docs travel with the code:** a change that creates, changes or voids a rule edits the nearest `AGENTS.md` in the same commit. The `edit_pack` invariants of `crates/oku-slides-mcp/AGENTS.md` change in Task 5. Every `ARCHITECTURE.md` this lot touches is **already at 50 lines**: each docs step condenses an existing line for every line it adds (`wc -l` ≤ 50 checked in the step). `just agents-lint` passes at each commit.
- **Consumers:** every rename is searched with `rg -n '<old>' crates src-tauri ui fixtures docs/aide` before staging. The real app host `src-tauri/src/state.rs` (`impl Sessions for AppSessions`) is a consumer of every `Sessions` change.
- **Gates.**
  - End of each task: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo check -p oku-slides-app`, then `cargo nextest run --workspace --exclude oku-slides-app`. Run them detached, with logs under `.superpowers/sdd/2026-10-29-onglets-pack-theme-lot7-mcp/`. Under load, re-run timed-out tests with `--retries 2` before blaming a change.
  - Tasks 5 and 9: `just e2e 4999`. A failed spec is re-run with all specs before it, never alone.
  - Task 9: `just ci`.
  - The first cold build in this worktree takes over 10 min: run it in the background at SDD setup.
- Runs that start the app outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir. Do not set `CARGO_TARGET_DIR`.

## Decisions (1-9 and 20-21 are the user's, 2026-10-29; the rest are settled by this plan and its review)

1. **The open stops there (user).** `open_pack` / `open_theme` end the deck agent's work on that request. The tool result's `note` tells it to ask the user to continue in the other tab, whose thread starts empty. The instruction is not handed off (follow-up ticket, Task 9).
2. **Names and reach (user).** `edit_pack` is renamed `open_pack { pack?, widget? }`, and `open_theme { theme }` is new. Both are listed for **deck and help** tabs only. A pack tab's agent changes its selected item with `select_item { name }`. Pack and theme agents never open other tabs.
3. **One gate (user).** At most one tab move per turn, pack or theme alike (`SurfaceHub::reserve_tab(turn, TabTarget)`), and only inside a turn. Once a turn has opened or focused another tab, its widget writes in the requesting tab stay refused (`writes_elsewhere`, unchanged).
4. **No confirmation before an open (user).** No `SurfaceHub::confirm` is asked before an open; Decision 20 sets which tab is shown.
5. **Saves are copies (user).** `save_pack` loses `as_new` and always sends `SavePack { as_new: true }`. `save_theme` (new, theme tabs) always sends `SaveTheme { as_new: true }`. The user confirms both first. The plain save is the tabs' autosave.
6. **Extra tools (user).** Theme tab: `rename_theme { name }` (Decision 21). Pack tab: `set_preview_theme { theme }` (`SetPreviewTheme`, no undo step). `show_params` and `show_variants` are listed in pack and theme tabs too. The session's per-kind op filter keeps refusing what does not fit (`WrongDocument`).
7. **`edit_theme` stays (user)** for decks (confirmed, no tab).
8. **Gate bugs fixed here (user)**, each with a regression test (Tasks 2, 3, 5).
9. **Per-kind instructions (user).** `instructions/agent.md` keeps the shared core, and `deck.md`, `pack.md`, `theme.md` each hold one kind's tools paragraph. The first turn sends the core, then the kind's paragraph, then `widgets.md` (deck, help and pack only), then `import.md` when the turn tunes an import (deck only, as today). A test checks that every tool named in backticks in the texts a kind receives is in that kind's list (Task 8).
10. **Kind lists: one table.** `tools::list_all()` builds one `Vec<(Tool, Kinds)>` in a `OnceLock`; `list_for(kind)` filters it and `kinds_of(name)` looks a name up in it. No second table can drift. `Kinds` is a bitset with `const fn or`: `ALL`, `DECK_OR_HELP`, `PACK`, `THEME`. The final lists (help = deck):
    - **all kinds:** `deck_outline`, `widgets`, `theme_tokens`, `slide`, `render_slide`, `packs`, `settings`, `apply`, `focus`, `ask_choice`, `ask_confirm`, `ask_rewrite`, `report`, `render`, `set_interface`, `show_dictation`, `set_dictation`, `download_dictation_model`, `cancel_dictation_download`, `select_agent`, `download_agent_model`, `cancel_agent_model_download`, `set_setting`;
    - **deck:** `chart_kind`, `show_as`, `open_pack`, `open_theme`, `add_pack`, `remove_pack`, `import_report`, `import_diff`, `show_themes`, `show_packs`, `save`, `open`, `move`, `save_copy`, `merge`, `new`, `history`, `present`, `delete_pack`, `edit_theme`;
    - **deck + pack:** `layouts`, `widget_sources`, `widget_source`, `set_widget`, `remove_widget`, `preview_widget`;
    - **deck + pack + theme:** `show_params`, `show_variants`;
    - **pack:** `set_widget_trial`, `select_item`, `set_preview_theme`, `save_pack`;
    - **theme:** `set_theme_extends`, `save_theme`, `rename_theme`.

    The descriptions of tools listed in several kinds must make sense in each one: `deck_outline`, `theme_tokens`, `packs` and `apply` are reworded (Task 4) so they name no tool their reader's kind lacks. The instructions test checks them (Task 8).
11. **Where the kind comes from.** `Sessions::document_kind(tab) -> Result<DocumentKind>` is a **required** trait method. The single-session impl reads `Session::document_kind()`. Both workspace hosts (`Mutex<Workspace>` in `workspace_agent.rs` and `AppSessions` in `src-tauri`) call the one `pub fn Workspace::document_kind(ws, tab)`. It reads `TabInfo.document.kind()` under the workspace lock alone, wakes nothing, and refuses an unknown tab or a closed workspace (`is_closed`). `TabEntry::start` (sync, under the agent stack's table lock) no longer builds `Tools`. `Agent::run` resolves the kind once in `spawn_blocking`. It builds `Tools::new(…, kind)` with it and passes it through `ActorParts` into `Ctx` for `turn.rs`. If the kind cannot be read (tab gone, task lost), `run` takes the existing "tools unavailable" path. Its message is a localized `ToolError::Task` / session error. `Tools` has no default kind. A test checks that `TabDocument::kind()` agrees with `Session::document_kind()` for every mode.
12. **Unlisted tool.** `Tools::call` first checks the tool's `Kinds` against `self.kind`. An unlisted name gets `UnknownTool { name }`, as the agent never saw it listed; this covers another kind's tool and an unknown name.
13. **Gate types renamed.** `agent::PackGate` → `agent::TabGate` (`Focus`, `NewTab`). `agent::PackOpen` → `agent::TabOpen` with `Here`, `Opened { tab: String, shown: bool }`, `Focused { tab: String, shown: bool }` and `Refused`. `Opened` and `Focused` are the two **moves** and keep the turn's claim. `Here`, `Refused` and an error give it back (the bug 2 fix: lot 3 leaked the claim on `Here` and on a lease refusal). The doc of `Sessions::open_pack` / `open_theme` states the gate order: at most one `Focus`, then at most one `NewTab`, then possibly one `Focus` again after a lease refusal (Decision 14).
14. **Bug 1 (session side).** In `open_new` (packs) and `open_theme` (themes), a granted `NewTab` whose lease is refused, with a live holder tab, asks `Focus` before focusing it. Refused → `Refused`; granted → `Focused { holder, shown }`. The existing `open_pack_refused_by_a_lagging_pack_tab_focuses_it` asserts `[NewTab, Focus]`. The gate is never asked with the workspace guard held: the holder is read in its own `let`, the guard dropped, then the gate asked (`focus_theme_tab` splits into a check and an activation around it).
15. **Bug 3.** `TabVerdict::ask` clears `refusal` whenever it grants. A `NotThisTurn` on `Focus` followed by a granted `NewTab` therefore never reports a stale refusal. `Verdict` becomes `TabVerdict` in `crates/oku-slides-mcp/src/tab_gate.rs`, shared by both tools.
16. **`open_theme` resolves its argument** in the requesting tab's session with `Session::find_theme_info(&theme)` (namespace, id or name), then opens by id. An unknown theme fails before any gate is asked.
17. **`select_item` uses the host path.** `Session::select_item` becomes `pub`. It validates the name and persists `UiState.widget`. The tool calls it in the session, then emits `McpEvent::Changed`. The UI re-reads the snapshot, and its existing host-value effect in `usePackItem` selects a host `ui.widget` that is not its own echo, while `select()` flushes pending typing. **No UI change, no new `McpEvent`.**
18. **Fluent keys.**
    - Renamed for both kinds, with all 20 locales re-translated:
      - `agent-error-pack-tab-no-turn` → `agent-error-tab-no-turn`;
      - `agent-error-pack-tab-limit` → `agent-error-tab-limit`;
      - `agent-error-continue-in-pack-tab` → `agent-error-continue-in-other-tab`.
    - New: `agent-theme-opened-in-tab` (`$tab`, `$shown`), `agent-confirm-copy-pack` (`$name`, replaces `agent-confirm-save-pack`), `agent-confirm-copy-edited-theme` (`$name`) and `session-undo-theme-rename`.
    - `agent-pack-opened-in-tab` gains a `$shown` selector (Decision 20).
    - The English texts are in Tasks 3, 5, 6 and 7.
19. **e2e.** The fake agent's `ouvre l'atelier` turn calls `open_pack` (`fixtures/agent/e2e.json`, `ui/e2e/lib/atelier.cjs`). A new turn `ouvre le thème` calls `open_theme` with `{ "theme": "papier" }`, checked in `ui/e2e/200-theme-editor.cjs` (a theme tab opens, is active, and `theming.theme.base.id` is the papier theme's). One shared helper waits on an MCP result and fails fast on an error result.
20. **Focus: the active tab and one move (user).** The opened or focused tab becomes its window's active tab only if the requesting tab is **its window's active tab** at that moment. Otherwise a new tab is inserted without being activated, and an existing one is left where it is. `TabOpen` reports this as `shown: false`, and the note tells the agent the tab is not shown. A user's open (`Opener::Ui`) always activates, as today. A turn has **one move**: a `Focus` claims the turn as a `NewTab` does. Both gates of one call share one claim (`TabVerdict` claims on its first grant, and later asks in the same call are granted while it holds the claim). A second open or focus in the same turn is `Taken` (`agent-error-tab-limit`).
21. **The theme rename is an undo step (user).** `Session::rename_edited_theme` becomes a theme-tab draft step, like `set_edited_theme_extends`:
    - a `DRAFT_SWITCH_VAR` rank op through `apply_draft_step(label, ops)`;
    - a pushed `ThemeSwitch { rank, from, to }`;
    - the label `session-undo-theme-rename`.

    `Request::RenameEditedTheme` moves into the same classification lists as `SetEditedThemeExtends` (Task 7). The UI's rename (`ThemePanel`, sent once per committed name on blur or Enter) becomes undoable with no UI change. The name and the empty name (base name) keep their rules. The agent's `rename_theme` checks its input with `agent::bounds::check_text("name", &name, LABEL_MAX)`.
22. **Confirmation names.** `save_pack` and `save_theme` quote the name through `agent::bounds::label` (made `pub`), so a name cannot break the confirm text. An unreadable name fails the tool (`fail_with`); it never becomes an empty string.
23. **Pack tab least privilege.** Every request a pack or theme tab's agent can reach is a tool listed for that kind. The session refuses any request outside that kind, whoever sends it (a pack tab's deck requests with `EditingPack`, before `ensure_fits`; a theme tab's through `theme_tab_allows`). Task 4 adds a test asserting that a pack tab refuses `HistoryOpen`, `Save` and `Move` (`EditingPack`) from the session itself, not only from the tool list (amended during Task 4: the session already refused them that way).

## File Structure

- `crates/oku-slides-session/src/tab_document.rs` — `TabDocument::kind()`.
- `crates/oku-slides-session/src/agent/tabs.rs` — required `Sessions::document_kind` and `Sessions::open_theme`, `TabGate`, `TabOpen`; `TabSession::document_kind`, `TabSession::open_theme`.
- `crates/oku-slides-session/src/workspace_agent.rs` — `Workspace::document_kind` (pub, shared by both hosts), the workspace's `Sessions` impl.
- `crates/oku-slides-session/src/workspace_packs.rs` — `PackOpened` → `TabOpen` mapping, bug 1 in `open_new`, `shown`.
- `crates/oku-slides-session/src/workspace_themes.rs` — `open_theme` takes an `Opener` (`Ui` or `Agent(gate)`), bug 1, `focus_theme_tab` split around the gate.
- `crates/oku-slides-session/src/workspace_layout.rs`, `workspace_tabs.rs` — `insert_tab(…, activate: bool)`, `Pending.activate`.
- `crates/oku-slides-session/src/pack_tab.rs` — `select_item` public.
- `crates/oku-slides-session/src/theme_edit.rs`, `request.rs`, `request_class.rs` — the rename as an undo step.
- `crates/oku-slides-session/src/agent/bounds.rs` — `label` public.
- `src-tauri/src/state.rs` — `AppSessions::document_kind`, `AppSessions::open_theme`, `TabOpen` match.
- `crates/oku-slides-mcp/src/hub_turns.rs` — `TabTarget`, `TabClaim`, `reserve_tab`, `release_tab`.
- `crates/oku-slides-mcp/src/tab_gate.rs` (new) — `TabVerdict`, `Refusal`, moved out of `edit_tools.rs`.
- `crates/oku-slides-mcp/src/tool_kinds.rs` (new) — `Kinds`.
- `crates/oku-slides-mcp/src/tools.rs` — `list_all()` as `(Tool, Kinds)`, `list_for(kind)`, `kinds_of`, `tool_names_for`, `Tools.kind`, `Tools::new(…, kind)`.
- `crates/oku-slides-mcp/src/open_tools.rs` (new) — `open_pack`, `open_theme`, `main_pack` (moved from `edit_tools.rs`).
- `crates/oku-slides-mcp/src/tab_tools.rs` (new) — `select_item`, `set_preview_theme`, `rename_theme`, `save_theme`.
- `crates/oku-slides-mcp/src/guarded_tools.rs` — `save_pack` copy-only.
- `crates/oku-slides-agent/src/tab_entry.rs`, `tab_agent.rs` (`ActorParts.kind`, `Ctx.kind`), `turn.rs`.
- `crates/oku-slides-agent/src/instructions/{agent,deck,pack,theme}.md`, `instructions.rs`.
- `crates/oku-slides-local-agent/src/families.rs`.
- `crates/oku-slides-session/src/agent/context.rs` — the context states the kind, the edited pack or theme's name and the preview theme.
- `fixtures/agent/e2e.json`, `ui/e2e/lib/atelier.cjs`, `ui/e2e/lib/agent-tools.cjs` (new helper, or the existing lib that holds `mcpResults`), `ui/e2e/200-theme-editor.cjs`.
- Docs: `crates/oku-slides-mcp/{AGENTS,ARCHITECTURE}.md`, `crates/oku-slides-session/{AGENTS,ARCHITECTURE}.md`, `crates/oku-slides-agent/ARCHITECTURE.md`, `src-tauri/ARCHITECTURE.md` if it names `PackOpen`, and `docs/aide` (help slides that name `edit_pack`: `rg -n edit_pack docs/aide`).

---

### Task 1: The tab's document kind, read without waking it (session, app host)

**Files:**
- Modify: `crates/oku-slides-session/src/tab_document.rs`, `agent/tabs.rs`, `workspace_agent.rs`, `src-tauri/src/state.rs`, and every other `impl Sessions` (`rg -n 'impl Sessions for' crates src-tauri`: the MCP, agent and local-agent test fakes, among them `OtherTabs`, `ClosedTabs`, `GatedTabs`).
- Test: `crates/oku-slides-session/src/tab_document_tests.rs` (create if absent, else extend), `crates/oku-slides-session/src/document_kind_tests.rs` (extend), `crates/oku-slides-session/src/workspace_agent_tests.rs` (create, declared in `workspace_agent.rs`).

**Interfaces:**
- Produces:
  - `TabDocument::kind(&self) -> DocumentKind`;
  - required `Sessions::document_kind(&self, tab: &str) -> crate::Result<DocumentKind>`;
  - `pub fn Workspace::document_kind(ws: &Mutex<Workspace>, tab: &str) -> Result<DocumentKind>`;
  - `TabSession::document_kind(&self) -> crate::Result<DocumentKind>` (blocking).

- [ ] **Step 1: Write the failing tests**

```rust
// tab_document_tests.rs
#[rstest]
#[case(TabDocument::Deck { path: None, deck_uid: None }, DocumentKind::Deck)]
#[case(TabDocument::Help { locale: "fr".into() }, DocumentKind::Help)]
#[case(TabDocument::Pack { pack: Uuid::nil() }, DocumentKind::Pack)]
#[case(TabDocument::DeckPack { deck_tab: "t-1".into(), pack: Uuid::nil() }, DocumentKind::Pack)]
#[case(TabDocument::Theme { theme: Uuid::nil() }, DocumentKind::Theme)]
fn kind_of_each_document_is_its_session_kind(#[case] document: TabDocument, #[case] kind: DocumentKind) {
    assert_that!(document.kind()).is_equal_to(kind);
}

// document_kind_tests.rs: the cached document and the live session agree, mode by mode.
// Build one session per mode with the fixtures the file already uses for
// `document_kind` (deck: test_support::session(); help, pack, deck-pack, theme:
// the same helpers its existing `document_kind_of_*` tests call).
#[rstest]
#[case::deck(Mode::Deck)]
#[case::help(Mode::Help)]
#[case::pack(Mode::Pack)]
#[case::deck_pack(Mode::DeckPack)]
#[case::theme(Mode::Theme)]
fn tab_document_kind_matches_session_document_kind(#[case] mode: Mode) {
    let s = session_in(mode);
    assert_that!(s.tab_info().document.kind()).is_equal_to(s.document_kind());
}

// workspace_agent_tests.rs
#[test]
fn document_kind_of_a_dormant_pack_tab_is_pack_and_keeps_it_dormant() {
    let f = packs_resumed();
    let tab = f.pack_tab();
    let f = f.restart(|_, _| {}); // tabs come back dormant
    assert_that!(Workspace::document_kind(&f.ws, &tab)).is_ok().is_equal_to(DocumentKind::Pack);
    assert_that!(f.is_dormant(&tab)).is_true();
}

#[test]
fn document_kind_of_an_unknown_tab_is_refused() {
    let f = packs();
    let err = Workspace::document_kind(&f.ws, "t-99").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn document_kind_after_settle_is_refused() {
    let f = packs();
    Workspace::settle(&f.ws);
    assert_that!(Workspace::document_kind(&f.ws, "t-1")).is_err();
}

#[test]
fn document_kind_through_the_sessions_trait_reads_the_workspace() {
    let f = packs();
    let tab = f.pack_tab();
    let sessions: &dyn Sessions = &*f.ws;
    assert_that!(sessions.document_kind(&tab)).is_ok().is_equal_to(DocumentKind::Pack);
}
```

`Mode` and `session_in` are local to `document_kind_tests.rs`. Write them over the helpers its tests already use, and do not add new fixtures. Check the `restart` signature in `workspace_packs_fixtures.rs:155` and the error-kind accessor that `workspace_packs_race_tests.rs` uses (`err.kind()`).

- [ ] **Step 2: Run them, expect compile failures** (`kind`, `document_kind` missing): `cargo nextest run -p oku-slides-session -E 'test(document_kind) | test(kind_of_each)'`.

- [ ] **Step 3: Implement**

```rust
// tab_document.rs
impl TabDocument {
    /// The kind of session this document opens in.
    #[must_use]
    pub fn kind(&self) -> DocumentKind {
        match self {
            Self::Deck { .. } => DocumentKind::Deck,
            Self::Help { .. } => DocumentKind::Help,
            Self::Pack { .. } | Self::DeckPack { .. } => DocumentKind::Pack,
            Self::Theme { .. } => DocumentKind::Theme,
        }
    }
}

// agent/tabs.rs, in trait Sessions (required: every host answers)
    /// The kind of `tab`'s document; wakes no dormant tab.
    ///
    /// # Errors
    ///
    /// Fails if tab `tab` is unknown to the host or the host is closing.
    ///
    /// # Blocking
    ///
    /// Takes the host's lock.
    fn document_kind(&self, tab: &str) -> crate::Result<DocumentKind>;

// impl Sessions for Mutex<Session>
    fn document_kind(&self, _tab: &str) -> crate::Result<DocumentKind> {
        Ok(lock(self).document_kind())
    }

// TabSession
    /// The kind of this tab's document (see [`Sessions::document_kind`]); blocking.
    ///
    /// # Errors
    ///
    /// As [`Sessions::document_kind`].
    pub fn document_kind(&self) -> crate::Result<DocumentKind> {
        self.sessions.document_kind(&self.tab)
    }

// workspace_agent.rs
impl Workspace {
    /// The kind of `tab`'s document, read from its cached info: a dormant tab stays asleep.
    ///
    /// # Errors
    ///
    /// `UnknownTab`, or `UnknownWindow` once the workspace is closed.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone.
    pub fn document_kind(ws: &Mutex<Self>, tab: &str) -> Result<DocumentKind> {
        let w = Self::lock(ws);
        // Use the same selector `finish` uses for a closed workspace.
        ensure!(!w.is_closed(), UnknownWindowSnafu { label: tab });
        Ok(w.tabs.get(tab).context(UnknownTabSnafu { id: tab })?.info.document.kind())
    }
}
// in impl Sessions for Mutex<Workspace>
    fn document_kind(&self, tab: &str) -> Result<DocumentKind> {
        Workspace::document_kind(self, tab)
    }

// src-tauri/src/state.rs, impl Sessions for AppSessions
    /// Read from the workspace's cached tab: wakes nothing.
    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<DocumentKind> {
        Workspace::document_kind(&self.workspace, tab)
    }
```

Test fakes: `OtherTabs`, `GatedTabs` and the like answer the kind of the session they wrap (`lock(&self.session).document_kind()`). `ClosedTabs` answers `UnknownTab`, like its `with`. Check how `finish` builds the closed-workspace error (`UnknownWindowSnafu { label: … }`) and reuse that selector. If `UnknownWindow` reads badly in a log, add no new variant: the error only reaches the "tools unavailable" path.

- [ ] **Step 4: Run the tests, expect PASS;** then run clippy on the crate and `cargo check -p oku-slides-app`.
- [ ] **Step 5: Docs.** In `crates/oku-slides-session/ARCHITECTURE.md`, the agent's `Sessions` line names `document_kind`. Condense another line so the file stays at 50.
- [ ] **Step 6: Stage** (`git add` the files above). Controller commits: `feat(session,app): a tab's document kind, read without waking it (t-1191, t-1268)`.

---

### Task 2: A gated agent open of a theme tab, one gate type, and the focus policy (session, app host)

**Files:**
- Modify:
  - in `crates/oku-slides-session/src/`: `agent/tabs.rs`, `agent/mod.rs` (re-exports), `workspace_agent.rs`, `workspace_packs.rs`, `workspace_themes.rs`, `workspace_tabs.rs` (`Pending.activate`, `finish`), `workspace_layout.rs` (`insert_tab(…, activate)`), `workspace_packs_fixtures.rs` (`open_by_agent` signature);
  - `src-tauri/src/state.rs`;
  - every `PackGate` / `PackOpen` user (`rg -n 'PackGate|PackOpen' crates src-tauri`).
- Test: `crates/oku-slides-session/src/workspace_themes_tests.rs`, `workspace_packs_race_tests.rs`, `workspace_packs_tests.rs` (or wherever the agent-open tests of `open_by_agent` live: `rg -n open_by_agent crates/oku-slides-session/src`).

**Interfaces:**
- Consumes: Task 1.
- Produces:
  ```rust
  pub enum TabGate { Focus, NewTab }
  pub enum TabOpen { Here, Opened { tab: String, shown: bool }, Focused { tab: String, shown: bool }, Refused }
  // Sessions (both required)
  fn open_pack(&self, tab: &str, pack: Uuid, widget: Option<&str>, gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen>;
  fn open_theme(&self, tab: &str, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen>;
  // TabSession
  pub fn open_theme(&self, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> crate::Result<TabOpen>;
  // Workspace
  pub fn open_theme_for_agent(ws: &Mutex<Self>, tab: &str, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen>;
  ```

- [ ] **Step 1: Rename** `PackGate` → `TabGate` and `PackOpen` → `TabOpen` everywhere (mechanical; doc comments say "a pack or theme tab"). Split `TabOpen::Tab(t)` into `Opened { tab, shown: true }` and `Focused { tab, shown: true }` for now, mapped from `PackOpened::Opened` and `PackOpened::Focused`. Update `src-tauri/src/state.rs`: `TabOpen::Opened { .. } | TabOpen::Focused { .. } => windows::sync(&self.app, None, true)`. Run the session tests and `cargo check -p oku-slides-app`. Everything passes once the tests that match `PackOpen::Tab` are updated to the exact variant they expect.

- [ ] **Step 2: Write the failing theme-gate tests** in `workspace_themes_tests.rs`. They use the `themes()` fixture, and `PAPIER` is the library theme `open_theme_places_the_tab_after_the_requester_and_activates_it` opens. Add `Themes::open_by_agent(tab, theme, gate)`, which calls `Workspace::open_theme_for_agent`.

```rust
#[test]
fn open_theme_for_agent_asks_new_tab_then_opens_it_after_the_requester() {
    let f = themes();
    let mut asked = Vec::new();
    let opened = f.open_by_agent("t-1", PAPIER, &mut |g| { asked.push(g); true });
    let tab = match opened { Ok(TabOpen::Opened { tab, shown: true }) => tab, other => panic!("{other:?}") };
    assert_that!((asked, f.window(), f.active())).is_equal_to((vec![TabGate::NewTab], vec!["t-1".to_owned(), tab.clone()], tab));
}

#[test]
fn open_theme_for_agent_refused_by_new_tab_opens_nothing() {
    let f = themes();
    let opened = f.open_by_agent("t-1", PAPIER, &mut |_| false);
    assert_that!((opened.ok(), f.window())).is_equal_to((Some(TabOpen::Refused), vec!["t-1".to_owned()]));
}

#[test]
fn open_theme_for_agent_of_an_open_theme_asks_focus_and_focuses_it() {
    let f = themes();
    f.opened("t-1", PAPIER);
    let tab = f.theme_tabs()[0].clone();
    f.activate("t-1");
    let mut asked = Vec::new();
    let opened = f.open_by_agent("t-1", PAPIER, &mut |g| { asked.push(g); true });
    assert_that!((asked, opened.ok(), f.active()))
        .is_equal_to((vec![TabGate::Focus], Some(TabOpen::Focused { tab: tab.clone(), shown: true }), tab));
}

#[test]
fn open_theme_for_agent_refused_by_focus_activates_nothing() {
    let f = themes();
    f.opened("t-1", PAPIER);
    f.activate("t-1");
    let opened = f.open_by_agent("t-1", PAPIER, &mut |_| false);
    assert_that!((opened.ok(), f.active())).is_equal_to((Some(TabOpen::Refused), "t-1".to_owned()));
}

#[test]
fn open_theme_for_agent_from_a_help_tab_opens_a_theme_tab() {
    // Same setup as open_theme_from_a_help_tab_opens_a_theme_tab, through the agent path.
}

/// Decision 20: a requester that is not its window's active tab opens behind.
#[test]
fn open_theme_for_agent_from_a_background_tab_opens_it_without_activating_it() {
    let f = themes();
    let other = f.new_tab();
    f.activate(&other);
    let opened = f.open_by_agent("t-1", PAPIER, &mut |_| true);
    assert_that!((matches!(opened, Ok(TabOpen::Opened { shown: false, .. })), f.active())).is_equal_to((true, other));
}

#[test]
fn open_theme_for_agent_from_a_background_tab_focuses_nothing() {
    let f = themes();
    f.opened("t-1", PAPIER);
    let other = f.new_tab();
    f.activate(&other);
    let opened = f.open_by_agent("t-1", PAPIER, &mut |_| true);
    assert_that!((matches!(opened, Ok(TabOpen::Focused { shown: false, .. })), f.active())).is_equal_to((true, other));
}

#[test]
fn open_theme_for_agent_with_a_refused_lease_asks_focus_first() { /* the holder setup of open_theme_twice_focuses_existing_tab, its cached document forged to Deck as the race test does; gate = |g| g == NewTab: asked == [NewTab, Focus], Refused, active == t-1 */ }

#[test]
fn open_theme_for_agent_with_a_refused_lease_reports_focused_not_opened() { /* same setup, gate always true: Focused { holder, shown: true } */ }

#[test]
fn open_theme_ui_asks_no_gate_and_still_activates() { /* from a background tab via f.open(): the new tab is active (Opener::Ui unchanged) */ }
```

Write the bodies marked with `/* */` in full, in the shape of the test above them; each comment states its assertion.

Bug-1 regression in `workspace_packs_race_tests.rs`, next to the test it extends (an exception to `tests/regressions`: the `Packs` fixtures are `pub(in crate::workspace)`):

```rust
/// t-1191 (lot 3 review, bug 1): a granted NewTab whose lease is refused asks Focus before focusing the holder.
#[test]
fn issue_1191_refused_lease_with_focus_refused_activates_nothing() {
    let f = packs();
    let p = f.pack_tab();
    f.activate("t-1");
    Workspace::lock(&f.ws).tabs.get_mut(&p).unwrap().info.document = TabDocument::Deck { path: None, deck_uid: None };
    let mut asked = Vec::new();
    let r = f.open_by_agent("t-1", f.acme, &mut |g| { asked.push(g); g == TabGate::NewTab });
    assert_that!((outcome(r), asked, f.active(0))).is_equal_to(("Refused".to_owned(), vec![TabGate::NewTab, TabGate::Focus], "t-1".to_owned()));
}
```

Also update `open_pack_refused_by_a_lagging_pack_tab_focuses_it` to expect `vec![TabGate::NewTab, TabGate::Focus]`. Add `open_pack_by_agent_from_a_background_tab_opens_it_without_activating_it` (packs, as for themes). `outcome` prints `PackOpened`; once `open_by_agent` returns `TabOpen`, update it to print `TabOpen`.

- [ ] **Step 3: Run, expect FAIL** (missing `open_theme_for_agent`, wrong ask order, always-active insertion).

- [ ] **Step 4: Implement.**
  - **Insertion without focus.** `Pending` gains `activate: bool` (`true` from `pending()`). `finish` passes it to `insert_tab(…, activate)`, which sets `win.active = id` only when `activate` is set. `insert_tab`'s doc changes from "Inserts an active tab…" to "Inserts a tab, made active when `activate` is set…".
  - **Shown.** `Workspace::is_active_in_its_window(&self, tab) -> bool` (private; `window_of(tab)` then `windows[w].active == tab`). For an `Opener::Agent`, read it once under the lock in `candidates` and keep it in `OpenCtx { shown, … }`; for `Opener::Ui`, `shown = true`. `focus_candidate` and `focus_theme_tab` call `w.activate(candidate)` only when `shown`; `open_new` / `open_theme` set `pending.activate = shown`. `PackOpened::{Opened, Focused}` carry `shown`.
  - **Themes.** `workspace_themes.rs` `open_theme(ws, tab, theme, created, opener: Opener<'_>) -> Result<PackOpened>`, reusing `PackOpened` and `Ask` (renamed if needed, still crate-private):
    - Before focusing a candidate other than `tab`, ask `Focus` once (as `focus_existing` does). Before building a new tab, ask `NewTab`.
    - On a refused lease, read `live_tab` in its own `let` (guard dropped), then if there is a holder: `ask(Focus)`; refused → `Refused`; granted → focus it → `Focused`.
    - `focus_theme_tab` splits into `theme_tab_edits(ws, candidate, wanted) -> bool` (wakes, checks) and `activate_theme_tab(ws, tab, candidate, shown)`, so no gate is asked inside a guard.
    - A candidate equal to `tab` gives `Here` without asking.
    - `open_theme_ui` and `new_theme` pass `Opener::Ui`; `open_theme_for_agent` passes `Opener::Agent(gate)` inside `with_opening(theme, …)` and maps to `TabOpen`.
  - **Packs.** In the `Err(refused)` arm of `workspace_packs.rs` `open_new`: `let lagging = Self::lock(ctx.ws).live_tab(…);` (its own statement), then `if lagging.is_some() && !ask(TabGate::Focus) { return Ok(PackOpened::Refused) }`, then `focus_candidate`.
  - **Workspace host.** `workspace_agent.rs`: `fn open_theme(&self, tab, theme, gate) -> Result<TabOpen> { Workspace::open_theme_for_agent(self, tab, theme, gate) }`.
  - **Single session.** `agent/tabs.rs`, `impl Sessions for Mutex<Session>`: `open_theme` answers `Here` if the session is that theme's tab (`tab_info().document == Theme { theme }`), else `SingleDocument`. Fakes in the MCP, agent and local-agent tests: `ClosedTabs` → `UnknownTab`; others → `SingleDocument` unless the test needs more (Task 5 extends `OtherTabs`).
  - **App host.** `src-tauri/src/state.rs` `AppSessions::open_theme`: `Workspace::open_theme_for_agent(&self.workspace, tab, theme, gate)`, then `Opened` / `Focused` → `windows::sync(&self.app, None, shown)`, `Here` → `emit_view`, `Refused` → nothing. `open_pack` matches the same way. Translate its French doc comments.
  - **Docs.** `Sessions::open_pack` and `open_theme` docs give the gate order of Decision 13 and the `shown` rule.
- [ ] **Step 5: Run the session crate's tests, expect PASS;** then run the workspace nextest and `cargo check -p oku-slides-app`. For now, the MCP crate's `edit_pack` code maps `TabOpen::Opened { .. } | TabOpen::Focused { .. }` to today's behaviour; Task 5 rewrites it.
- [ ] **Step 6: Docs.**
  - The session `AGENTS.md` lease invariant is unchanged. Add a rule: `- Ask an agent's tab gate with no workspace guard held; activate an agent-opened tab only if its requester is its window's active tab. (t-1191)`.
  - Session `ARCHITECTURE.md` workspace themes line: "agents open through the tab gate".
  - In the module doc of `workspace_themes.rs`, replace "Unlike a pack, a theme tab has no preview seed and no agent gate." with "Unlike a pack, a theme tab has no preview seed; an agent's open asks the same gate (`TabGate`)."
- [ ] **Step 7: Stage.** Commit: `feat(session,app)!: agents open theme tabs through one tab gate, shown only from the active tab (t-1191, t-1269)`.

---

### Task 3: One claim per turn for packs and themes, and the verdict fixes (MCP hub)

**Files:**
- Modify: `crates/oku-slides-mcp/src/hub_turns.rs`, `hub.rs` (state field), `edit_tools.rs` (uses), `tool_error.rs` (renamed variants), `widget_tools.rs` (`ContinueInPackTabSnafu` → `ContinueInOtherTabSnafu`), `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 files).
- Create: `crates/oku-slides-mcp/src/tab_gate.rs`, `tab_gate_tests.rs`.
- Test: `crates/oku-slides-mcp/src/hub_turns_tests.rs`, `tab_gate_tests.rs`, `tool_error_tests.rs`, `hub_tests.rs`, `hub_thread_tests.rs` (renamed calls).

**Interfaces:**
- Consumes: Task 2's `TabGate`, `TabOpen`.
- Produces:
  ```rust
  pub(crate) enum TabTarget { Pack(Uuid), Theme(Uuid) }
  impl SurfaceHub {
      pub(crate) fn reserve_tab(&self, turn: u32, target: TabTarget) -> Reserve;
      pub(crate) fn release_tab(&self, turn: u32, target: TabTarget);
  }
  // tab_gate.rs
  #[derive(Debug, Default)]
  pub(crate) struct TabVerdict { refusal: Option<Refusal>, claimed: bool }
  impl TabVerdict {
      /// Grants a gate: the first grant of a call claims the turn's one move.
      pub(crate) fn ask(&mut self, hub: &SurfaceHub, turn: Option<u32>, target: TabTarget, gate: TabGate) -> bool;
      pub(crate) fn refusal(&self) -> ToolError;
      /// Gives back the claim unless the open moved a tab; notes writes elsewhere after a move.
      pub(crate) fn settle(&self, hub: &SurfaceHub, turn: Option<u32>, target: TabTarget, outcome: &oku_slides_session::Result<TabOpen>);
  }
  // ToolError variants renamed: TabNoTurn, TabLimit, ContinueInOtherTab
  ```

- [ ] **Step 1: Failing hub tests** (`hub_turns_tests.rs`). Adapt the existing `reserve_pack_tab` tests by renaming them, then add:

```rust
#[test]
fn reserve_tab_of_a_theme_after_a_pack_in_one_turn_is_taken() {
    let hub = hub();
    let turn = hub.begin_turn();
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(ACME))).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}

#[test]
fn reserve_tab_of_the_same_target_twice_needs_two_releases() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Theme(PAPIER));
    let _ = hub.reserve_tab(turn, TabTarget::Theme(PAPIER));
    hub.release_tab(turn, TabTarget::Theme(PAPIER));
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(ACME))).is_equal_to(Reserve::Taken);
}

#[test]
fn release_tab_of_a_pack_never_releases_a_theme_claim_of_the_same_uuid() {
    let hub = hub();
    let turn = hub.begin_turn();
    let id = Uuid::from_u128(7);
    let _ = hub.reserve_tab(turn, TabTarget::Theme(id));
    hub.release_tab(turn, TabTarget::Pack(id));
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(ACME))).is_equal_to(Reserve::Taken);
}
```

- [ ] **Step 2: Failing verdict tests** (`tab_gate_tests.rs`), bugs 2 and 3 and the one-move rule. The bug-3 regression stays here (an exception to `tests/regressions`: `TabVerdict` is crate-private):

```rust
/// t-1191 bug 3: a refusal noted on Focus is cleared by a later grant.
#[test]
fn verdict_after_a_refused_focus_then_a_granted_new_tab_reports_no_refusal() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, Some(turn + 1), TabTarget::Pack(ACME), TabGate::Focus)).is_false();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::NewTab)).is_true();
    assert_that!(v.refusal.is_none()).is_true();
}

/// Decision 20: a Focus spends the turn's one move.
#[test]
fn verdict_focus_granted_claims_the_turn() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::Focus)).is_true();
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}

/// The second gate of one call rides on the first one's claim.
#[test]
fn verdict_new_tab_after_a_granted_focus_in_the_same_call_is_granted() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    let _ = v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::Focus);
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::NewTab)).is_true();
}

#[test]
fn verdict_outside_a_turn_refuses_with_no_turn() {
    let hub = hub();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, None, TabTarget::Theme(PAPIER), TabGate::NewTab)).is_false();
    assert_that!(v.refusal().kind_key()).is_equal_to("agent-error-tab-no-turn"); // use the file's key accessor
}

/// t-1191 bug 2: an outcome that moved no tab gives the claim back.
#[rstest]
#[case::here(Ok(TabOpen::Here))]
#[case::refused(Ok(TabOpen::Refused))]
#[case::error(Err(unknown_tab_error()))]
fn settle_without_a_move_releases_the_claim(#[case] outcome: oku_slides_session::Result<TabOpen>) {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::NewTab)).is_true();
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &outcome);
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Granted);
}

#[rstest]
#[case::opened(TabOpen::Opened { tab: "t-2".into(), shown: true })]
#[case::focused(TabOpen::Focused { tab: "t-2".into(), shown: false })]
fn settle_with_a_move_keeps_the_claim_and_notes_writes_elsewhere(#[case] moved: TabOpen) {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    let _ = v.ask(&hub, Some(turn), TabTarget::Pack(ACME), TabGate::NewTab);
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &Ok(moved));
    assert_that!((hub.reserve_tab(turn, TabTarget::Theme(PAPIER)), hub.writes_elsewhere(turn)))
        .is_equal_to((Reserve::Taken, true));
}

#[test]
fn settle_without_a_claim_releases_nothing() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(ACME)); // another call's claim
    let v = TabVerdict::default();
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &Ok(TabOpen::Here));
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}
```

`unknown_tab_error()` builds a session error the way `edit_tools_tests.rs` does (read it; reuse).

- [ ] **Step 3: Failing localized-error test** (`tool_error_tests.rs`, rstest over the renamed variants, key plus `fr`):

```rust
#[rstest]
#[case(TabNoTurnSnafu.build(), "agent-error-tab-no-turn")]
#[case(TabLimitSnafu.build(), "agent-error-tab-limit")]
#[case(ContinueInOtherTabSnafu.build(), "agent-error-continue-in-other-tab")]
fn tab_gate_errors_say_their_key(#[case] e: ToolError, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr(key));
    assert_that!(fr.tr(key)).is_not_equal_to(Localizer::new(Some("en")).tr(key));
}
```

- [ ] **Step 4: Run, expect FAIL.**
- [ ] **Step 5: Implement.**
  - `TabTarget` derives `Debug, Clone, Copy, PartialEq, Eq` and has a `Display` for tracing (`pack <id>` / `theme <id>`). `PackTabClaim` → `TabClaim { turn, target, claims }`; `HubState.pack_tab` → `tab_claim`.
  - Move `Verdict` / `Refusal` from `edit_tools.rs` into `tab_gate.rs` (`//! The agent's open verdict: one tab move per turn, asked by the session's gate.`) as `TabVerdict`. The `focused` field is gone, since a move is read from the outcome.
  - `ask`: if `self.claimed`, grant only while `hub.turn_is_open(turn)`. Otherwise both gates `reserve_tab`: `Granted` → `claimed = true`; `NotThisTurn` → `Refusal::NoTurn`; `Taken` → `Refusal::Limit`. Every `true` clears `refusal`.
  - `settle`: on `Opened` / `Focused`, `note_elsewhere(turn)`; on anything else, `release_tab` **only if `self.claimed && turn.is_some()`**.
  - A `spawn_blocking` that panics (JoinError) never runs `settle`. The claim then stays spent for the rest of that turn: the turn fails closed, and the doc of `settle` says so.
  - Fluent, English (reference) texts:
    ```
    agent-error-tab-no-turn = Open or show a pack or theme tab only while answering the user.
    agent-error-tab-limit = You already opened or showed a pack or theme tab in this turn; finish there, or ask the user before another.
    agent-error-continue-in-other-tab = This request continues in the tab you opened or showed: ask the user to go on there; this tab changes no widget for now.
    ```
    Re-translate the three into the 19 other locales, starting with `fr` (fresh translations; each locale keeps its own word for "agent"), and remove the old keys at the same position.
- [ ] **Step 6: Run the MCP crate unit tests and the i18n crate tests, expect PASS.** Update the French constants in `tests/edit_pack_tabs.rs` (`NO_TURN_FR`, `LIMIT_FR`, `CONTINUE_FR`) and the key names in `tests/widgets.rs` to the new texts.
- [ ] **Step 7: Docs.** Translate the French doc comments of `hub_turns.rs` that this task touches. In `crates/oku-slides-mcp/ARCHITECTURE.md`, the `hub_turns` line says `reserve_tab` (one pack or theme tab move per turn); fold `tab_gate` into the same line, so the file stays at 50.
- [ ] **Step 8: Stage.** Commit: `refactor(mcp): one tab move per turn for packs and themes, verdict fixes (t-1191, t-1270)`.

---

### Task 4: Tool lists filtered by the tab's kind (MCP, agent)

**Files:**
- Create: `crates/oku-slides-mcp/src/tool_kinds.rs` (`Kinds` only), `tool_kinds_tests.rs`, `crates/oku-slides-mcp/tests/kinds.rs`, `crates/oku-slides-mcp/tests/common/kinds.rs` (`list_tool_names`, `start_help_tab`).
- Modify:
  - `crates/oku-slides-mcp/src/tools.rs` (the table, `list_for`, `kinds_of`, `tool_names_for`; translate its French docs), `handler.rs`, `lib.rs` (re-export `tool_names_for`);
  - `crates/oku-slides-mcp/src/tools_tests.rs` (`list`, `edit_pack_description` → `list_all`);
  - `crates/oku-slides-agent/src/tab_entry.rs`, `tab_agent.rs` (`ActorParts.kind`, `Ctx.kind`), `crates/oku-slides-agent/src/tab_entry_tests.rs:93`;
  - the harnesses that call `Tools::new`: `crates/oku-slides-mcp/tests/common/mod.rs:208` (`start_with`), `crates/oku-slides-agent/tests/common/mod.rs:141`, `crates/oku-slides-local-agent/tests/common/mod.rs:105`;
  - `crates/oku-slides-mcp/tests/server.rs:35` (tool count or list);
  - `crates/oku-slides-session/src/document_kind_tests.rs` (Decision 23).

**Interfaces:**
- Consumes: Task 1's `DocumentKind` reads.
- Produces:
  ```rust
  // tool_kinds.rs
  //! Which tab kinds list a tool: a small bitset over DocumentKind.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(crate) struct Kinds(u8);
  impl Kinds {
      /// Deck and help tabs (a help tab is a read-only deck).
      pub(crate) const DECK_OR_HELP: Self;
      pub(crate) const PACK: Self;
      pub(crate) const THEME: Self;
      pub(crate) const ALL: Self;
      pub(crate) const fn or(self, other: Self) -> Self;
      pub(crate) fn has(self, kind: DocumentKind) -> bool;
  }
  // tools.rs
  pub(crate) fn list_all() -> &'static [(Tool, Kinds)];
  pub(crate) fn list_for(kind: DocumentKind) -> Vec<Tool>;
  pub(crate) fn kinds_of(tool: &str) -> Option<Kinds>; // a lookup into list_all
  /// The names of the tools a tab of `kind` lists.
  pub fn tool_names_for(kind: DocumentKind) -> Vec<String>;
  impl Tools { pub fn new(session: TabSession, hub: Arc<SurfaceHub>, settings: Arc<dyn SettingsPort>, platform: &Platform, kind: DocumentKind) -> Self; }
  // tab_agent.rs
  pub(crate) struct ActorParts { …, pub kind: DocumentKind }
  pub(crate) struct Ctx { …, pub kind: DocumentKind }
  ```
  `open_theme`, `select_item`, `set_preview_theme`, `save_theme` and `rename_theme` join the table in Tasks 5-7. In this task, `edit_pack` is `DECK_OR_HELP`.

- [ ] **Step 1: Failing unit tests** (`tool_kinds_tests.rs` for the bitset, `tools_tests.rs` for the lists):

```rust
// tool_kinds_tests.rs
#[rstest]
#[case(Kinds::DECK_OR_HELP, DocumentKind::Help, true)]
#[case(Kinds::DECK_OR_HELP, DocumentKind::Pack, false)]
#[case(Kinds::PACK.or(Kinds::THEME), DocumentKind::Theme, true)]
#[case(Kinds::PACK.or(Kinds::THEME), DocumentKind::Deck, false)]
#[case(Kinds::ALL, DocumentKind::Pack, true)]
fn kinds_has_exactly_its_kinds(#[case] k: Kinds, #[case] kind: DocumentKind, #[case] has: bool) {
    assert_that!(k.has(kind)).is_equal_to(has);
}

// tools_tests.rs: the exact sorted list per kind (this task's names; Tasks 5-7 extend them).
const DECK_TOOLS: &[&str] = &[/* Decision 10's all + deck + deck/pack + deck/pack/theme names, edit_pack in place of open_pack and open_theme, sorted */];
const PACK_TOOLS: &[&str] = &[/* all + deck/pack + deck/pack/theme + set_widget_trial, save_pack, sorted */];
const THEME_TOOLS: &[&str] = &[/* all + deck/pack/theme + set_theme_extends, sorted */];

#[rstest]
#[case::deck(DocumentKind::Deck, DECK_TOOLS)]
#[case::help(DocumentKind::Help, DECK_TOOLS)]
#[case::pack(DocumentKind::Pack, PACK_TOOLS)]
#[case::theme(DocumentKind::Theme, THEME_TOOLS)]
fn list_for_a_kind_is_exactly_its_tools(#[case] kind: DocumentKind, #[case] expected: &[&str]) {
    let mut names: Vec<String> = list_for(kind).into_iter().map(|t| t.name).collect();
    names.sort();
    assert_that!(names).is_equal_to(expected.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>());
}

/// Every tool is listed somewhere: no table entry is dead.
#[test]
fn list_all_every_tool_is_listed_for_some_kind() {
    for (tool, _) in list_all() {
        let somewhere = [DocumentKind::Deck, DocumentKind::Pack, DocumentKind::Theme]
            .into_iter()
            .any(|k| list_for(k).iter().any(|t| t.name == tool.name));
        assert_that!(somewhere).named(&tool.name).is_true();
    }
}

#[test]
fn kinds_of_an_unknown_name_is_none() {
    assert_that!(kinds_of("nope")).is_none();
}
```

Write the three constants in full, sorted; Decision 10 gives the names.

- [ ] **Step 2: Failing server tests** `tests/kinds.rs` (the spec's name):

```rust
//! Each tab's agent lists only its kind's tools (spec: tool_list_filtered_by_document_kind).
#![expect(clippy::panic, reason = "test failures panic")]

/// The spec's test, on a real server per kind; the exact lists are `tools_tests.rs`'s.
#[rstest]
#[case::deck(Kind::Deck, &["apply", "save", "edit_pack", "show_packs"][..], &["set_widget_trial", "set_theme_extends", "save_pack"][..])]
#[case::help(Kind::Help, &["apply", "edit_pack", "history"][..], &["set_widget_trial", "set_theme_extends"][..])]
#[case::pack(Kind::Pack, &["set_widget", "set_widget_trial", "save_pack", "show_params"][..], &["save", "history", "edit_pack", "set_theme_extends"][..])]
#[case::theme(Kind::Theme, &["apply", "set_theme_extends", "theme_tokens", "show_variants"][..], &["set_widget", "save", "edit_pack", "save_pack"][..])]
#[tokio::test(flavor = "multi_thread")]
async fn tool_list_filtered_by_document_kind(#[case] kind: Kind, #[case] has: &[&str], #[case] lacks: &[&str]) {
    let h = kind.start().await; // start(), start_help_tab(), start_pack_tab(SET), start_theme_tab(PAPIER_THEME)
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    for n in has { assert_that!(names).named(n).contains(n.to_string()); }
    for n in lacks { assert_that!(names).named(n).does_not_contain(n.to_string()); }
}

#[tokio::test(flavor = "multi_thread")]
async fn call_of_a_tool_another_kind_lists_is_unknown() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    let r = call(&c, "set_widget", json!({ "name": "x", "source": "" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let fr = Localizer::new(Some("fr"));
    // Name the key and check the non-English text: the harness localizer is fr.
    assert_that!(text_of(&r)).is_equal_to(fr.tr_with("agent-error-unknown-tool", &[("name", "set_widget".into())]));
}

#[tokio::test(flavor = "multi_thread")]
async fn call_of_a_listed_tool_in_its_kind_is_served() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    assert_that!(call(&c, "theme_tokens", json!({})).await.is_error).is_not_equal_to(Some(true));
}
```

`Kind` (an enum with `start()`), `list_tool_names` (`c.request_tool_list(None)` as `tests/server.rs` does) and `start_help_tab` go into `tests/common/kinds.rs`. For `start_help_tab`, the session comes from the helper the session crate's help tests use, if the MCP harness has none; read `tests/common/mod.rs` first. Check the harness localizer's language: if it is not `fr`, use `h.localizer` and add one `fr` assertion through `Localizer::new(Some("fr"))` on the error's key.

**Decision 23 test** (`crates/oku-slides-session/src/document_kind_tests.rs`, unit):

```rust
#[rstest]
#[case::history(Request::HistoryOpen)]
#[case::save(Request::Save { path: None })]
#[case::move_(Request::Move { /* fields as request.rs defines */ })]
fn pack_tab_refuses_a_deck_request_itself(#[case] req: Request) {
    let mut s = pack_tab_session(); // the file's pack fixture
    let err = s.handle(req).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}
```

If one of these is not refused today, **stop and surface it** (G9). That would be a session hole, so do not paper over it in MCP.

- [ ] **Step 3: Run, expect FAIL.**
- [ ] **Step 4: Implement.**
  - `tool_kinds.rs`: bits `DECK = 1`, `HELP = 2`, `PACK = 4`, `THEME = 8`. `DECK_OR_HELP` is `DECK | HELP` computed in `const` with `or`, since `|` is not const on a newtype.
  - `tools.rs`: `list()` → `list_all()` (`pub(crate)`, still a `OnceLock`). Each entry is `(Tool, Kinds)`, grouped by Decision 10's groups with a short comment per group. `list_for(kind)` clones the tools whose kinds `has(kind)`. `kinds_of` is `list_all().iter().find(|(t, _)| t.name == name).map(|(_, k)| *k)`. `tool_names()` keeps returning all names. `tool_names_for(kind)` returns `list_for`'s names. `Tools` gains `pub(crate) kind: DocumentKind`, and `Tools::new` takes it.
  - Reword the shared descriptions so each reads true in every kind that lists it:
    - `deck_outline`: "The document of this tab: a deck's slides, a pack's items or a theme's specimen slides, with its theme";
    - `theme_tokens`: name no deck-only op;
    - `packs`: "open_pack and delete_pack take its id" → "a deck's agent opens one with open_pack";
    - `apply`: list the op families without naming deck-only tools.

    The exact wording is checked by Task 8's test.
  - `handler.rs`: `tools: list_for(self.kind)`. `Tools::call` starts with `if !kinds_of(name).is_some_and(|k| k.has(self.kind)) { return self.fail_with(&UnknownToolSnafu { name }.build()); }`; the `_` arm stays (`UnknownTool`).
  - `tab_entry.rs`: `TabEntry::start` stops building `Tools` and passes `ports.settings` and a `Platform` clone to `Agent::run`, which first does:
    ```rust
    let kind = {
        let session = self.session.clone();
        tokio::task::spawn_blocking(move || session.document_kind()).await
    };
    let kind = match kind {
        Ok(Ok(kind)) => kind,
        Ok(Err(e)) => return self.serve_without_tools(e.localize(&self.localizer.current()), cancel, rx, relay).await,
        Err(e) => {
            let e = ToolError::from(TaskSnafu.into_error(e)); // the crate's JoinError mapping
            return self.serve_without_tools(e.localize(&self.localizer.current()), cancel, rx, relay).await;
        }
    };
    let tools = Tools::new(self.session.clone(), self.hub.clone(), settings, &platform, kind);
    ```
    `serve_without_tools(message: String, …)` is the existing `ToolServer::start` error branch, extracted into a method. `run` calls it for both failures: a failed kind read and a failed server start (`e.localize(…)`). The kind goes into `ActorParts { kind, … }` and from there into `Ctx`. If `run` grows past ~60 lines, extract `start_tools(&self, settings, platform) -> Result<(ToolServer, DocumentKind), String>`. Translate the French doc comments of the methods touched.
  - `tab_entry_tests.rs:93`: pass `DocumentKind::Deck`. Add `agent_run_on_a_closed_tab_answers_tools_unavailable` if the file's harness can close the tab before the run; otherwise test `serve_without_tools` through the existing tools-unavailable test.
  - Harnesses: `start_with` reads `let kind = lock(&session).document_kind();` from the concrete session it already holds (no trait call, so `ClosedTabs` and `start_closed` keep working), then calls `Tools::new(…, kind)`. The agent and local-agent harnesses do the same from their session.
- [ ] **Step 5: Run** the MCP, agent and local-agent crates' tests. The existing tests that call a tool outside their tab's kind now fail with `UnknownTool`. For each one, decide which case it is:
  - it was testing a refusal that is now "not listed": rewrite the assertion to `agent-error-unknown-tool`;
  - it was a stray: move it to a harness of the right kind;
  - it is vacuous now: delete it and name it in the task report. For example, `edit_pack_tabs.rs:324` (`set_widget_trial` in a deck) and the in-pack-tab `edit_pack` tests (`edit_pack_in_place_needs_no_turn` and the like) get `UnknownTool` because pack tabs no longer list it. Rewrite them in Task 5 as "pack tab does not list open_pack".

  `rg -n '"set_theme_extends"|"set_widget_trial"|"save_pack"|"edit_pack"' crates/oku-slides-mcp/tests crates/oku-slides-agent/tests crates/oku-slides-local-agent/tests` lists the candidates.
- [ ] **Step 6: Docs.** In `crates/oku-slides-mcp/ARCHITECTURE.md`, replace the `tools` line: "`list_for(kind)`: the tab's kind, read once when its agent starts; an unlisted tool is `UnknownTool`". Its extension point becomes "New tool: an entry `(Tool, Kinds)` in `list_all`". Keep the file at 50 lines. `crates/oku-slides-mcp/AGENTS.md` gets a new invariant: `- List and serve only the tools of the tab's document kind, read once when its agent starts; refuse any other with UnknownTool. (spec:2026-10-24-onglets-pack-theme-design, t-1191)`. In `crates/oku-slides-agent/ARCHITECTURE.md`, the tools are built in `Agent::run` from the tab's kind (condense a line).
- [ ] **Step 7: Stage.** Commit: `feat(mcp,agent)!: each tab's agent lists only its kind's tools (t-1191, t-1271)`.

---

### Task 5: `open_pack` and `open_theme` for a deck's agent (MCP, e2e)

**Files:**
- Create: `crates/oku-slides-mcp/src/open_tools.rs`, `open_tools_tests.rs` (holding the `main_pack` tests moved from `edit_tools_tests.rs`), `crates/oku-slides-mcp/tests/open_theme_tabs.rs`, `crates/oku-slides-mcp/tests/regressions/issue_1191_open_claim_release.rs`, the e2e helper (`ui/e2e/lib/` file that holds `mcpResults`, extended).
- Modify:
  - in `crates/oku-slides-mcp/src/`: `edit_tools.rs` (remove `edit_pack`, `default_pack`, `main_pack`, `EditPackArgs`; translate its touched French docs), `tools.rs` (entries and the `packs` description), `handler.rs`;
  - `crates/oku-slides-local-agent/src/families.rs` (+ tests);
  - in `crates/oku-slides-agent/src/`: `instructions/agent.md` and `import.md` (`edit_pack` → `open_pack`; the full per-kind split is Task 8), `instructions_tests.rs`;
  - `crates/oku-slides-i18n/src/locales/*/agent.ftl`;
  - `fixtures/agent/e2e.json`, `ui/e2e/lib/atelier.cjs` (header comment too), `ui/e2e/200-theme-editor.cjs`;
  - `crates/oku-slides-mcp/tests/edit_pack_tabs.rs` → `git mv` to `open_pack_tabs.rs`, and the other tests calling `edit_pack` (`rg -ln edit_pack crates src-tauri ui fixtures docs/aide`);
  - `crates/oku-slides-mcp/tests/common/workspace_harness.rs` (`selected_item`), the `OtherTabs` fake (`open_theme`, `HERE_PACK`);
  - `crates/oku-slides-mcp/{AGENTS,ARCHITECTURE}.md`, `docs/aide` slides naming `edit_pack`.

**Interfaces:**
- Consumes: Tasks 2-4.
- Produces: tools `open_pack { pack?: string, widget?: string }` and `open_theme { theme: string }`, both `Kinds::DECK_OR_HELP`. They reply `{ ok, tab, shown, note }` (`note` from `agent-pack-opened-in-tab` / `agent-theme-opened-in-tab`), or `{ ok: true }` for `Here`.

- [ ] **Step 1: Move and rename the pack tests** (`git mv tests/edit_pack_tabs.rs tests/open_pack_tabs.rs`). The `edit` / `edit_main` helpers call `open_pack`, and the `edit_pack_…` names become `open_pack_…`. Rewrite the in-pack-tab ones (Task 4 Step 5) as `pack_tab_does_not_list_open_pack`. Add:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_widget_selects_it_in_the_new_tab() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let turn = h.hub.begin_turn();
    let r = call(&c, "open_pack", json!({ "pack": ACME.to_string(), "widget": ACME_WIDGET })).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(selected_item(&ws, &tab)).is_equal_to(Some(ACME_WIDGET.to_owned()));
    h.hub.close_turn(turn);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_then_open_pack_of_another_pack_in_one_turn_is_refused_by_the_limit() { /* ACME then BETA: agent-error-tab-limit (fr); one new tab */ }

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_of_an_open_pack_tab_focuses_it_and_spends_the_turn() { /* open ACME (turn 1), activate t-1, turn 2: open ACME → same tab, active; then open_theme papier in turn 2 → agent-error-tab-limit */ }

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_from_a_help_tab_opens_a_pack_tab() { /* help tab in the workspace (open help via the workspace request), its agent opens ACME */ }
```

`ACME_WIDGET` is a widget the harness's ACME pack holds; `selected_item` reads the tab's `UiState.widget` through `Workspace::with_session` and goes into `workspace_harness.rs`.

- [ ] **Step 2: Failing `open_theme` tests** `tests/open_theme_tabs.rs` (real workspace, `start_on_workspace`):

```rust
//! `open_theme` opens a library theme in its own tab from a deck tab, one move per turn (t-1191).
#![expect(clippy::panic, reason = "test failures panic")]

#[tokio::test(flavor = "multi_thread")]
async fn open_theme_in_a_turn_opens_a_theme_tab_after_the_deck_and_activates_it() {
    let (h, ws) = start_on_workspace().await;
    let c = server(&h).await;
    let _turn = h.hub.begin_turn();
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    let tab = json_of(&r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned();
    assert_that!(document_of(&ws, &tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
    assert_that!(active_tab(&ws)).is_equal_to(tab.clone());
    let note = h.localizer.tr_with("agent-theme-opened-in-tab", &[("tab", tab.as_str().into()), ("shown", "yes".into())]);
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned)).is_equal_to(Some(note));
}
```

Then these, each written in full in the shape of the first test (each comment states its assertion):
- `open_theme_outside_a_turn_is_refused_and_opens_nothing`: no `begin_turn`; `agent-error-tab-no-turn` plus its fr text; the tab ids are unchanged.
- `open_theme_after_open_pack_in_one_turn_is_refused_by_the_limit`: open_pack ACME, then open_theme papier → `agent-error-tab-limit`; one new tab.
- `open_theme_of_an_open_theme_tab_focuses_it_without_a_new_tab`: open via the workspace `open_theme` request, activate t-1, `begin_turn`, call → same tab id, active, tab count unchanged, `shown: true`.
- `open_theme_from_a_background_deck_tab_opens_it_unshown`: a second blank tab active; the agent of t-1 opens papier → `shown: false`, active tab unchanged, the note is the `shown = no` variant.
- `open_theme_of_an_unknown_theme_is_refused_before_the_gate`: "nope" → the session's unknown-theme key (fr); then open_pack in the same turn is still granted (no claim spent).
- `set_widget_after_open_theme_in_the_same_turn_is_refused`: `agent-error-continue-in-other-tab`.
- `open_theme_by_id_and_by_name_opens_the_same_tab`: the id string in turn 1, "Papier" in turn 2 → same tab (focused).
- `theme_tab_does_not_list_open_theme`: `start_theme_tab`; `list_tool_names` lacks `open_theme` and `open_pack`.

Then the concurrency test of the shared gate, written in full: two concurrent calls in one turn, `open_pack` ACME and `open_theme` papier. Use the harness's `hold_next` to park the first inside its blocking open; read `tests/common` for its use in `edit_pack_tabs.rs`. Assert: exactly one new tab, one result ok, one result `agent-error-tab-limit`.

- [ ] **Step 3: Failing regression** `issue_1191_open_claim_release.rs` (bug 2 end to end). `OtherTabs::open_pack` returns `Here` for `OtherTabs::HERE_PACK` after calling the gate with `NewTab`. `OtherTabs::open_theme` asks `NewTab`, records the theme, and answers `Opened { tab: "t-9", shown: true }`.

```rust
//! t-1191 (lot 3 review, bug 2): an open that lands Here after claiming gives the claim back.
#![expect(clippy::panic, reason = "test failures panic")]

#[tokio::test(flavor = "multi_thread")]
async fn issue_1191_open_pack_landing_here_leaves_the_turn_free_to_open_a_theme() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let _turn = h.hub.begin_turn();
    let here = call(&c, "open_pack", json!({ "pack": OtherTabs::HERE_PACK.to_string() })).await;
    assert_that!(here.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!((r.is_error, tabs.opened_themes())).is_equal_to((None, vec![PAPIER_THEME]));
}
```

`OtherTabs` lives in `tests/common/mod.rs`. Its new code is about 20 lines; if the file would pass 850 lines, move `OtherTabs` into `tests/common/other_tabs.rs` first, as its own staged move.

- [ ] **Step 4: Run, expect FAIL.**
- [ ] **Step 5: Implement** `open_tools.rs`:

```rust
//! Opening a pack or a theme in its own tab, for a deck's agent (`open_pack`, `open_theme`).

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenPackArgs {
    /// Id of a pack (UUID, see `packs`); absent: the deck's main pack.
    #[serde(default)]
    pub pack: Option<String>,
    /// A widget or layout of the pack to select in its tab.
    #[serde(default)]
    pub widget: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenThemeArgs {
    /// The theme: its namespace (as `deck_outline` lists it), id or name.
    pub theme: String,
}

impl Tools {
    /// `open_pack`: the pack (or the deck's main one) in its own tab, gated.
    pub(crate) async fn open_pack(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let OpenPackArgs { pack, widget } = match self.parse(args) { Ok(a) => a, Err(r) => return r };
        let pack = match pack {
            Some(id) => match parse_pack_id(&id) { Ok(id) => id, Err(e) => return self.fail_with(&e) },
            None => match self.with_session(|s| Ok(main_pack(&s.snapshot()))).await {
                Ok(id) => id,
                Err(r) => return r,
            },
        };
        self.open_tab(turn, TabTarget::Pack(pack), move |tab, gate| tab.open_pack(pack, widget.as_deref(), gate), "agent-pack-opened-in-tab").await
    }

    /// `open_theme`: a library theme in its own tab, gated; resolved before any gate.
    pub(crate) async fn open_theme(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let OpenThemeArgs { theme } = match self.parse(args) { Ok(a) => a, Err(r) => return r };
        let info = match self.with_session(move |s| s.find_theme_info(&theme)).await {
            Ok(info) => info,
            Err(r) => return r,
        };
        let id = info.id;
        self.open_tab(turn, TabTarget::Theme(id), move |tab, gate| tab.open_theme(id, gate), "agent-theme-opened-in-tab").await
    }

    /// One gated open: the verdict, the open in a blocking task, then the claim settled and the reply.
    async fn open_tab(
        &self,
        turn: Option<u32>,
        target: TabTarget,
        open: impl FnOnce(&TabSession, &mut dyn FnMut(TabGate) -> bool) -> oku_slides_session::Result<TabOpen> + Send + 'static,
        note_key: &'static str,
    ) -> CallToolResult {
        let (tab, hub) = (self.session.clone(), Arc::clone(&self.hub));
        let opened = tokio::task::spawn_blocking(move || {
            let mut verdict = TabVerdict::default();
            let result = open(&tab, &mut |gate| verdict.ask(&hub, turn, target, gate));
            verdict.settle(&hub, turn, target, &result);
            (result, verdict)
        })
        .await;
        let (result, verdict) = match opened {
            Ok(done) => done,
            // The claim stays spent: the turn fails closed (TabVerdict::settle's doc).
            Err(e) => return self.fail_with(&TaskSnafu.into_error(e)),
        };
        match result {
            Ok(TabOpen::Refused) => self.fail_with(&verdict.refusal()),
            Ok(TabOpen::Here) => self.json_result(&json!({ "ok": true })),
            Ok(TabOpen::Opened { tab, shown } | TabOpen::Focused { tab, shown }) => {
                self.hub.emit(McpEvent::Changed);
                self.prune_surfaces().await;
                let shown_arg = if shown { "yes" } else { "no" };
                let note = self.localizer.current().tr_with(note_key, &[("tab", tab.as_str().into()), ("shown", shown_arg.into())]);
                self.json_result(&json!({ "ok": true, "tab": tab, "shown": shown, "note": note }))
            }
            Err(e) => self.fail_with(&SessionSnafu.into_error(e)),
        }
    }
}
```

Check `with_session`'s actual return shape (a `Result<T, CallToolResult>` or similar) and the crate's pack-id parse helper (`PackIdArgs::id` in `edit_tools.rs`), and adapt these lines to them. Inside a fn returning `CallToolResult`, never use `?`: match explicitly as shown.

  - `main_pack` moves here unchanged. `default_pack` drops its `editing` branch, because pack tabs no longer list `open_pack`.
  - Tool descriptions, in `tools.rs`. The pack one is adapted from `edit_pack`'s, minus "In a pack's own tab…".
    - `open_pack`: "Open a pack in its own tab to edit it there. The tab is shown if this tab is the one shown in its window; else it opens behind. One tab move (opening or showing a pack or theme tab) per turn, and only while answering the user. Editing happens in that tab, in its own conversation: tell the user to continue there; this request stops here. pack: its id (see packs); absent, the deck's main pack: its own layout pack, else its own widget pack, else a library pack (layouts first), else essentiel. widget: an item of the pack to select. A deck-only pack (an imported deck's own) opens in a tab linked to this deck, from the version the deck embeds; its saves go back to the deck. Not for themes: use open_theme."
    - `open_theme`: "Open a library theme (namespace as deck_outline lists it, id or name) in its own tab to edit its tokens and parents there; shown as open_pack's tab is. Same limit as open_pack: one tab move per turn, only while answering the user; the request stops here, tell the user to continue in that tab. For a quick change of this deck's theme without a tab, use edit_theme; for this deck only, set_token ops on deck.tokens."
  - Fluent, in 20 locales, after `agent-pack-opened-in-tab`, whose text gets the same `$shown` selector:
    ```
    agent-theme-opened-in-tab = { $shown ->
        [yes] The theme opened in tab { $tab }, now shown.
       *[no] The theme opened in tab { $tab }, behind: the user's window shows another tab.
    } Tell the user to continue there: that tab's agent does not know this request. This tab stays on its deck.
    ```
  - `families.rs`: `"open_pack" | "open_theme"` go into the `Packs` arm, and `edit_pack` is removed. The families tests are updated (the `rstest` case list).
  - `agent.md`: "Packs: open_pack (…); themes: open_theme opens a library theme in its own tab (one tab move per turn, shared with open_pack)". `import.md`: "Never use open_pack on an imported deck." The `instructions_tests.rs` cases follow.
  - `docs/aide`: replace `edit_pack` wherever it appears.
  - e2e:
    - `fixtures/agent/e2e.json`: `"ouvre l'atelier": { "mcp": [{ "tool": "open_pack", "args": {} }] …}`, and a new turn `"ouvre le thème": { "mcp": [{ "tool": "open_theme", "args": { "theme": "papier" } }], "chunks": ["Thème ouvert."] }`.
    - The shared helper `waitForTool(p, from, tool)` sits beside `mcpResults` and resolves on the tool's first result. It throws at once with the result's text when `is_error` is set.
    - `atelier.cjs` `openAtelier` uses it on `'open_pack'`; its header comment says `open_pack`.
    - `200-theme-editor.cjs` sends `ouvre le thème` from its deck tab before its first error assert. It waits for an active tab whose view has `document.kind === 'theme'`, asserts `theming.theme.base.id` is the papier theme's id (read from the view, never hard-coded), and closes that tab before the spec's own steps.
- [ ] **Step 6: Run** the MCP, agent, local-agent and i18n tests, then `just e2e 4999` (detached).
- [ ] **Step 7: Docs.**
  - `crates/oku-slides-mcp/AGENTS.md`: the two `edit_pack` invariants become:
    - "…only open_pack and open_theme may open or focus a pack or theme tab, through Sessions::open_pack / Sessions::open_theme";
    - "Let open_pack and open_theme make at most one tab move (new or focused) per agent turn through SurfaceHub::reserve_tab, and give the claim back when no tab moved (Here, Refused, error)…".

    Keep their sources and add `t-1191`.
  - `ARCHITECTURE.md`: the `edit_tools` line loses `edit_pack`, and `open_tools` takes its place, so the line count is unchanged.
- [ ] **Step 8: Stage.** Commit: `feat(mcp)!: open_pack and open_theme for a deck's agent (t-1191, t-1272)`.

---

### Task 6: Pack tab tools: `select_item`, `set_preview_theme`, copy-only `save_pack` (MCP, session)

**Files:**
- Create: `crates/oku-slides-mcp/src/tab_tools.rs`, `crates/oku-slides-mcp/tests/pack_tab_tools.rs`.
- Modify: `crates/oku-slides-session/src/pack_tab.rs` (`select_item` → `pub`, doc), `crates/oku-slides-session/src/agent/bounds.rs` (`label` → `pub`), `crates/oku-slides-mcp/src/tools.rs`, `handler.rs`, `edit_tools.rs` (`change` → `pub(crate)`), `guarded_tools.rs` (`SavePackArgs` removed), `crates/oku-slides-i18n/src/locales/*/agent.ftl`, `crates/oku-slides-local-agent/src/families.rs`.
- Test: `crates/oku-slides-mcp/tests/pack_tab_tools.rs`.

**Interfaces:**
- Consumes: Task 4's table.
- Produces: tools `select_item { name }` (`PACK`), `set_preview_theme { theme }` (`PACK`) and `save_pack {}` (`PACK`); `pub fn Session::select_item(&mut self, widget: &str) -> Result<()>`; `pub fn agent::bounds::label(text: &str) -> String`.

- [ ] **Step 1: Failing server tests** (`start_in_pack_tab` from `workspace_harness.rs`; `guarded` from `tests/common`):

```rust
//! A pack tab's own tools: selection, preview theme, copies (t-1191).
#![expect(clippy::panic, reason = "test failures panic")]

#[tokio::test(flavor = "multi_thread")]
async fn select_item_of_an_item_selects_it_and_emits_changed() {
    let (h, ws, tab) = start_in_pack_tab().await;
    let mut events = h.hub.subscribe();
    let c = server(&h).await;
    let r = call(&c, "select_item", json!({ "name": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(selected_item(&ws, &tab)).is_equal_to(Some(ACME_WIDGET.to_owned()));
    assert_that!(matches!(next_event(&mut events).await, McpEvent::Changed)).is_true();
}
```

Then these, each written in full:
- `select_item_of_an_unknown_item_is_refused_and_selects_nothing`: the session's unknown-widget key, plus fr; the selection is unchanged.
- `set_preview_theme_changes_the_preview_and_leaves_the_tab_clean`: after `set_preview_theme { theme: "pro" }`, the tab's `snapshot().theme` (namespace) is `"pro"` and the tab is not dirty. Read both as `preview_theme_tests.rs` does (`theme_of`, `dirty`), through `Workspace::with_session`.
- `save_pack_confirmed_saves_a_copy_under_a_new_id`: `guarded(&h, "save_pack", json!({}), true)`; the tab now edits a pack id ≠ ACME, and the library holds both.
- `save_pack_denied_saves_nothing`: `guarded(…, false)`; the agent-denied text; the library is unchanged.
- `save_pack_asks_with_the_pack_name_as_a_label`: the confirm surface text equals `tr_with("agent-confirm-copy-pack", name = label(pack name))`.
- `save_pack_in_a_deck_pack_tab_saves_a_copy_into_the_library`: the deck-pack tab from `workspace_harness` (`open_pack_tab` on the deck's deck-only pack); confirmed. The library gains a new pack, and the owner deck's embedded version is unchanged.

`next_event` (the hub subscription's next event, with a timeout) goes into `workspace_harness.rs` unless one exists.

- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement.**
  - `tab_tools.rs` (`//! Tools of a pack or theme tab's own document: selection, preview theme, rename, copies.`):
    - `SelectItemArgs { name: String }`, documented as "A widget or layout of this pack, as widget_sources lists it, `_commun` included.";
    - `select_item` runs `s.select_item(&name)` in `with_session`, then `self.hub.emit(McpEvent::Changed)` and replies `{ ok: true }`;
    - `set_preview_theme` sends `Request::SetPreviewTheme { theme }` through `self.change(…)`.
  - `pack_tab.rs`: `select_item` is `pub`, with the doc "Selects `widget` in this pack tab and persists it; the host's view follows." plus a `# Errors` heading.
  - `guarded_tools.rs` `save_pack`:
    - no args (`NoArgs`);
    - the name comes from `with_session(|s| s.snapshot().editing.map(|e| e.name.clone()).context(…))`, and a missing one fails through `fail_with` with the session's `WrongDocument`;
    - the message is `tr_with("agent-confirm-copy-pack", &[("name", label(&name).into())])`, sending `Request::SavePack { as_new: true }`;
    - description: "Save a copy of the pack this tab edits, under a new id and name; the tab then edits the copy. The pack itself saves on its own as you edit. The user confirms first."
  - Descriptions:
    - `select_item`: "Select an item (widget or layout) of this pack: the tab shows it, as a click in the pile does. Its trial values then appear in the context.";
    - `set_preview_theme`: "Choose the theme this pack tab previews its items with (namespace, id or name, see deck_outline). Changes no pack and is no undo step."
  - Fluent: `agent-confirm-copy-pack = Save a copy of the pack “{ $name }”? The pack itself stays as it is.` in 20 locales, replacing `agent-confirm-save-pack`.
  - `families.rs`: `select_item` and `set_preview_theme` → `Packs`.
- [ ] **Step 4: Run** the MCP, session and i18n tests. No UI gate is needed: no UI file changes. Check this with `git diff --stat -- ui` (empty).
- [ ] **Step 5: Docs.** In MCP `ARCHITECTURE.md`, fold the `tab_tools` family into the tool-families line (no new line). Session and UI `AGENTS.md`: no rule change.
- [ ] **Step 6: Stage.** Commit: `feat(mcp,session)!: a pack tab's agent selects items, picks the preview theme, saves copies (t-1191, t-1273)`.

---

### Task 7: Theme tab tools: `rename_theme` as an undo step, `save_theme` (session, MCP)

**Files:**
- Modify:
  - session: `crates/oku-slides-session/src/theme_edit.rs` (`rename_edited_theme`), `request.rs` (classification ~405-436), `request_class.rs` (`changes_deck`, `edits`, `writes_edited_draft`, `theme_tab_allows`);
  - i18n: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`session-undo-theme-rename`, after `session-undo-theme-extends`), `crates/oku-slides-i18n/src/locales/*/agent.ftl`;
  - MCP: `crates/oku-slides-mcp/src/tab_tools.rs`, `tools.rs`, `handler.rs`;
  - `crates/oku-slides-local-agent/src/families.rs`.
- Test: `crates/oku-slides-session/src/theme_edit_switch_tests.rs` (the rename tests already live there), `crates/oku-slides-mcp/tests/theme_tab.rs`.

**Interfaces:**
- Produces: `rename_theme { name }` (`THEME`), `save_theme {}` (`THEME`). `Session::rename_edited_theme` now pushes an undo step.

- [ ] **Step 1: Failing session tests** (`theme_edit_switch_tests.rs`, with its theme-tab fixture and `PRO_THEME`):

```rust
#[test]
fn rename_edited_theme_then_undo_restores_the_name() {
    let mut s = pro_theme_tab(); // the file's fixture, as rename_edited_theme_request_shows_the_new_name_in_theming uses
    s.handle(Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    s.handle(Request::Undo).unwrap();
    let snap = snapshot_json(&s);
    assert_that!(snap.pointer("/theming/theme/name")).is_equal_to(Some(&json!("Pro")));
}

#[test]
fn rename_edited_theme_undo_then_redo_renames_again() { /* undo, redo: name == "Pro à moi" */ }

#[test]
fn rename_edited_theme_undo_step_has_its_label() { /* the undo label (as theme_extends tests read it) == tr("session-undo-theme-rename") in fr */ }

#[test]
fn rename_edited_theme_with_an_empty_name_takes_the_base_name_in_one_step() { /* rename "X", rename "  " → "Pro"; undo → "X" */ }

#[test]
fn rename_edited_theme_after_set_edited_theme_extends_undoes_in_order() { /* extends change, rename, undo → old name kept extends; undo → old extends */ }
```

Write the four `/* */` bodies in full, reading the undo label the way the `set_edited_theme_extends` tests in this crate do (`rg -n session-undo-theme-extends crates/oku-slides-session/src`).

- [ ] **Step 2: Failing MCP tests** (`theme_tab.rs`, `start_theme_tab(PRO_THEME)`; read the file's helpers first):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn rename_theme_renames_the_draft_in_one_undo_step() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("{e}"));
    let _ = call(&c, "rename_theme", json!({ "name": "Mon pro" })).await;
    assert_that!(theming_name(&h)).is_equal_to("Mon pro".to_owned());
    lock(&h.session).handle(Request::Undo).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(theming_name(&h)).is_equal_to("Pro".to_owned());
}
```

Then these, each written in full:
- `rename_theme_with_a_hidden_control_is_refused_and_renames_nothing`: `"Pro\u{202E}"` → the `AgentForm` hidden-characters key (fr); the name is unchanged.
- `rename_theme_too_long_is_refused`: 61 characters → the `AgentForm` too-long key.
- `save_theme_confirmed_saves_a_copy_the_tab_then_edits`: `guarded(&h, "save_theme", json!({}), true)`; `theming.theme.base.id` ≠ `PRO_THEME`, and the library lists both.
- `save_theme_denied_saves_nothing`.
- `save_theme_asks_with_the_edited_theme_name`: the confirm text equals `tr_with("agent-confirm-copy-edited-theme", name = label(theming name))`.
- `show_params_in_a_theme_tab_is_listed`: `list_tool_names` contains `show_params` and `show_variants`.

`theming_name(&h)` reads `lock(&h.session).snapshot().theming` → `theme.name`. Put it in this file, or in `tests/common/kinds.rs` if another file needs it.

- [ ] **Step 3: Run, expect FAIL.**
- [ ] **Step 4: Implement.**
  - **Session.** `rename_edited_theme` mirrors `set_edited_theme_extends` (`theme_extends.rs:106`):
    ```rust
    /// Renames the edited theme in one undo step; an empty name takes back its base's name.
    ///
    /// # Errors
    ///
    /// Fails outside a theme tab (`WrongDocument`), or if the draft step is refused.
    pub fn rename_edited_theme(&mut self, name: &str) -> Result<()> {
        let Mode::Theme { edit, switches, .. } = &self.mode else { return self.ensure_theme_tab() };
        let from = edit.draft.clone();
        let rank = next_switch_rank(self.doc.sources.deck(), switches);
        let name = match name.trim() {
            "" => self.base_manifest().map(|m| m.name).context(…)?, // no unwrap_or_default: a missing base is an error
            trimmed => trimmed.to_owned(),
        };
        let to = ThemeEdit { name, ..from.clone() };
        let label = self.localizer.tr("session-undo-theme-rename");
        self.set_theme_edit(to.clone());
        let ops = vec![Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(rank) }];
        if let Err(e) = self.apply_draft_step(&label, ops) {
            self.set_theme_edit(from);
            return Err(e);
        }
        if let Mode::Theme { switches, .. } = &mut self.mode {
            switches.push(ThemeSwitch { rank, from, to });
        }
        Ok(())
    }
    ```
    - Check what `base_manifest()` returns. If it is a `Result`, propagate it; if it is an `Option`, map it with the selector that `ensure_theme_tab` uses for a missing base. Today's `unwrap_or_default` is a silent fallback, which the root invariant forbids. If the borrow checker refuses `self.mode` borrowed across `next_switch_rank`, clone `switches` the way `set_edited_theme_extends` does.
    - Drop `self.doc.dirty = true` if `apply_draft_step` already sets it (as it does for extends; check).
    - `request.rs`: add `RenameEditedTheme` to the first `!matches!` list (beside `SetEditedThemeExtends`: the step writes its own state), and remove it from the `documents` exclusion list.
    - `request_class.rs`: move `RenameEditedTheme` from the `false` arm of `changes_deck` to the `true` arm, next to `SetEditedThemeExtends`, and update its doc comment. In `edits`, `writes_edited_draft` and `theme_tab_allows`, keep or place it wherever `SetEditedThemeExtends` is.
    - Fluent `session-undo-theme-rename = Rename the theme` in 20 locales, after `session-undo-theme-extends`.
  - **MCP.**
    - `rename_theme`: `check_text("name", &name, LABEL_MAX)` first (→ `fail_with`), then `self.change(Request::RenameEditedTheme { name })`.
    - `save_theme`: the name from `with_session(|s| s.snapshot().theming.map(|t| t.theme.name.clone()).context(…))`, propagated through `fail_with`. The message is `tr_with("agent-confirm-copy-edited-theme", &[("name", label(&name).into())])`, sending `self.guarded(message, false, Request::SaveTheme { as_new: true })`.
    - Descriptions: `rename_theme` "Rename the theme this tab edits (one undo step); the name shows in the theme list once it saves."; `save_theme` "Save a copy of the theme this tab edits under a new id; the tab then edits the copy. The theme itself saves on its own as you edit. The user confirms first."
    - Fluent: `agent-confirm-copy-edited-theme = Save a copy of the theme “{ $name }”? The theme itself stays as it is.` (20 locales).
    - `families.rs`: both → `Packs`.
- [ ] **Step 5: Run** the session, MCP and i18n tests. Also run the UI render tests that cover `ThemePanel` rename (`cd ui && bun test ThemePanel`): they must still pass unchanged.
- [ ] **Step 6: Docs.**
  - Session `ARCHITECTURE.md`: the theme-edit line says "rename and extends are draft steps".
  - MCP `ARCHITECTURE.md`: the families line names `rename_theme` and `save_theme`.
  - MCP `AGENTS.md` settled decision: `- Save a pack or theme tab through its autosave; the agent's save_pack and save_theme only save confirmed copies. (t-1191)`.
- [ ] **Step 7: Stage.** Commit: `feat(session,mcp)!: theme renames are undo steps; a theme tab's agent renames and saves copies (t-1191, t-1274)`.

---

### Task 8: Instructions and context by tab kind (agent, session)

**Files:**
- Create: `crates/oku-slides-agent/src/instructions/deck.md`, `pack.md`, `theme.md`.
- Modify: `instructions/agent.md` (the core only), `instructions.rs` (translate its module doc), `turn.rs`, `instructions_tests.rs`, `crates/oku-slides-agent/tests/stack.rs` (prompt block expectations), `crates/oku-slides-session/src/agent/context.rs` (+ its tests), `crates/oku-slides-agent/ARCHITECTURE.md`.

**Interfaces:**
- Consumes: Task 4 (`oku_slides_mcp::tool_names_for(kind)`, `oku_slides_mcp::tool_names()`, the descriptions through a new `pub fn oku_slides_mcp::tool_descriptions_for(kind) -> Vec<(String, String)>`), `Ctx.kind`.
- Produces:
  - `instructions::for_kind(kind: DocumentKind) -> &'static str` (the kind paragraph; help → deck's);
  - `turn::run` sends `INSTRUCTIONS`, `for_kind(ctx.kind)`, `WIDGETS` unless the kind is `Theme`, then `IMPORT` when the turn tunes an import.
  - `agent_context` states the kind (help included), the edited pack's or theme's name, and the preview theme in a pack tab.

- [ ] **Step 1: Failing tests** (`instructions_tests.rs`):

```rust
/// Backticked words that name a tool: `open_pack`, `apply` …
fn backticked_tools(text: &str) -> Vec<String> {
    let all = oku_slides_mcp::tool_names();
    text.split('`').skip(1).step_by(2).filter(|w| all.iter().any(|t| t == w)).map(str::to_owned).collect()
}

/// Every tool a kind's texts name in backticks is a tool that kind lists.
#[rstest]
#[case::deck(DocumentKind::Deck)]
#[case::help(DocumentKind::Help)]
#[case::pack(DocumentKind::Pack)]
#[case::theme(DocumentKind::Theme)]
fn instructions_for_a_kind_name_only_its_tools(#[case] kind: DocumentKind) {
    let listed = oku_slides_mcp::tool_names_for(kind);
    let mut text = format!("{INSTRUCTIONS}\n{}", for_kind(kind));
    if kind != DocumentKind::Theme { text.push_str(WIDGETS); }
    if matches!(kind, DocumentKind::Deck | DocumentKind::Help) { text.push_str(IMPORT); }
    for (_, description) in oku_slides_mcp::tool_descriptions_for(kind) { text.push_str(&description); }
    for tool in backticked_tools(&text) {
        assert_that!(listed.contains(&tool)).named(&tool).is_true();
    }
}

#[rstest]
#[case::deck(DocumentKind::Deck, &["open_pack", "open_theme", "edit_theme", "history"][..])]
#[case::pack(DocumentKind::Pack, &["select_item", "set_widget_trial", "set_preview_theme", "save_pack"][..])]
#[case::theme(DocumentKind::Theme, &["set_theme_extends", "rename_theme", "save_theme", "theme_tokens"][..])]
fn instructions_for_a_kind_name_its_own_tools(#[case] kind: DocumentKind, #[case] tools: &[&str]) {
    let named = backticked_tools(for_kind(kind));
    for t in tools { assert_that!(named).named(t).contains(t.to_string()); }
}

#[test]
fn instructions_for_help_are_the_deck_ones() {
    assert_that!(for_kind(DocumentKind::Help)).is_equal_to(for_kind(DocumentKind::Deck));
}
```

The texts and descriptions name tools in backticks (`` `open_pack` ``) wherever they mean the tool. Prose words such as "history" or "layouts" are then free in every kind. Update the tool descriptions to backtick their tool names, everywhere they name one, so the test sees them. Keep the existing tests, re-pointed: the app-tool cases look in `for_kind(DocumentKind::Deck)`, or in the core for the shared ones (`set_interface`, dictation); `instructions_mention_pack_tabs` becomes `deck_instructions_mention_open_pack_and_open_theme`.

Context tests (`context.rs`'s test file):

```rust
#[rstest]
#[case::deck(deck_session(), "kind: deck")]
#[case::help(help_session(), "kind: help")]
#[case::pack(pack_tab_session(), "kind: pack")]
#[case::theme(theme_tab_session(), "kind: theme")]
fn agent_context_states_the_tab_kind(#[case] s: Session, #[case] line: &str) {
    assert_that!(agent_context(&s)).contains(line);
}

#[test]
fn agent_context_of_a_pack_tab_names_the_pack_and_its_preview_theme() { /* contains "pack: <name>" and "preview theme: pro" after SetPreviewTheme pro */ }

#[test]
fn agent_context_of_a_theme_tab_names_the_theme() { /* contains "theme: <draft name>" */ }
```

Write the last two in full, with the session fixtures of `context.rs`'s existing tests. The exact line format follows the context's current `key: value` lines; keep `mode: … theme_tab(id=…)` if other tests read it, or replace it and update them.

- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Write the texts.**
  - Move from `agent.md` into `deck.md` everything about files, history, presenting, the deck's theme (`set_theme` op, `edit_theme`, `deck.tokens`), packs of the deck (`show_packs`, `add_pack`, `remove_pack`, `open_pack`, `open_theme`) and the theme-token families. `agent.md` keeps the role, the report/ask/render rules, the interface and dictation tools, and the languages.
  - `pack.md` (new, one paragraph): "This tab edits a pack: its items (widgets or layouts) fill the pile, one is selected and shown (the context's widget line). Read `widget_sources` and `widget_source`; write with `set_widget` and `remove_widget`, check with `preview_widget`; `set_widget_trial` sets an item's test values; `select_item` shows another item; `set_preview_theme` changes the theme items are previewed with (no undo step). A layout pack's slides are its layouts' examples: change them with set_param ops through `apply`. The pack saves itself as you edit; `save_pack` only saves a copy, after the user confirms. This tab opens no other tab, file or history."
  - `theme.md`: "This tab edits a theme: its specimen slides show it. Change its tokens with set_token and unset_token ops through `apply` (read `theme_tokens` first, to restore earlier values), its parents with `set_theme_extends`, its name with `rename_theme` (one undo step). The theme saves itself as you edit; `save_theme` only saves a copy, after the user confirms. This tab opens no other tab, file or history."
  - The files keep no final newline (see the `instructions.rs` module doc).
  - `instructions.rs`: three `include_str!` constants and `for_kind`. `turn.rs`: on the first turn, push `INSTRUCTIONS`, `for_kind(ctx.kind)`, then `WIDGETS` unless `ctx.kind == DocumentKind::Theme`, then `IMPORT` when tuning (unchanged condition).
  - `tools.rs`: `tool_descriptions_for(kind)` (re-exported from `lib.rs`, documented).
  - `context.rs`: the kind line, the name line and the preview-theme line.
- [ ] **Step 4: Run** the agent crate's tests (`tests/stack.rs`'s scenarios that assert prompt blocks: update their expected block count and order) and the session tests.
- [ ] **Step 5: Docs.** Agent `ARCHITECTURE.md`: the instructions line ("a shared core and one paragraph per tab kind, sent in that order"), condensed in place. Session `ARCHITECTURE.md`: the context line names the kind.
- [ ] **Step 6: Stage.** Commit: `feat(agent,session): instructions and context by tab kind (t-1191, t-1275)`.

---

### Task 9: Final review, follow-ups, full gate

- [ ] **Step 1:** Whole-branch review (SDD final review with the full team: the common reviewers plus the Rust lenses, and the TypeScript lenses for the e2e files). Fix findings in their own commits.
- [ ] **Step 2:** File the follow-ups under t-1152:
  - "Hand a request's rest to the agent of the tab it opened" (Decision 1);
  - "Trim app-wide tools from pack and theme tabs" (all-kinds tools such as `select_agent` and the dictation tools, low);
  - "Route the local agent's families in pack and theme tabs" (check `families_of` against `list_for`).

  Comment on t-1191 which lot 3 review items are fixed (bugs 1-3), with the regression test names.
- [ ] **Step 3:** `just ci` and `just e2e 4999`, detached, with logs under `.superpowers/sdd/2026-10-29-onglets-pack-theme-lot7-mcp/`.
- [ ] **Step 4:** Mark each sub-task done as its task lands (not in a batch). Unset `oku.review` (`git config --worktree --unset oku.review`). Ask the user before merging into or pushing to main; mark t-1191 done once they agree.

## Self-review

- **Spec coverage:**
  - per-kind filtering when the agent starts (Task 4);
  - the deck list without workshop tools (Decision 10: `set_widget_trial`, `set_theme_extends` and `save_pack` are out of decks);
  - the pack list: widget tools, layout tools via `apply` / `layouts`, `save_pack` as a copy, reads, surfaces;
  - the theme list: `set_theme_extends`, token tools through `apply`, `save_theme` as a copy, `rename_theme`, reads, surfaces;
  - `open_pack` / `open_theme` routed by the workspace (Tasks 2, 5), including from a help tab;
  - `tool_list_filtered_by_document_kind` (Task 4, four kinds);
  - agent-opened tabs: no confirmation, focus only from the active tab, one move per turn (Decisions 3, 4, 20);
  - the context states the kind, the name and the preview theme (Task 8);
  - the lot 3 review comments on t-1191 (Tasks 2, 3, 5);
  - the spec's "edit_theme_extends" is today's `set_theme_extends`.
- **v1 review findings folded in:**
  - the app host (`AppSessions`) in Tasks 1 and 2, plus `cargo check -p oku-slides-app` in every gate;
  - `document_kind` and `open_theme` as required trait methods;
  - one tool table;
  - the kind resolved once and carried into turns;
  - the select_item host path (no UI change);
  - the rename as an undo step;
  - the bug-1 unit regression and the lagging test's new order;
  - backticked tool names in the instructions test, which also covers WIDGETS, IMPORT and descriptions;
  - `label` / `check_text` on agent text, with no `unwrap_or_default`;
  - real fixture names, harness kind read from the concrete session, and the missing consumers;
  - `settle` releasing only its own claim, the JoinError behaviour documented, the gate never asked under the workspace guard;
  - vacuous tests named for rewrite;
  - the concurrency test, the help cases, the deck-pack `save_pack` test, the least-privilege test;
  - file and doc size limits and the e2e helper that fails fast.
- **Placeholders:** each test body marked with a `/* … */` comment states its exact assertion and follows a fully written sibling in the same file. Harness helpers (`selected_item`, `next_event`, `list_tool_names`, `start_help_tab`, `theming_name`, `Kind`) are created in the task that first uses them, in `workspace_harness.rs` or `tests/common/kinds.rs`, unless an equivalent already exists (reuse it). Where a snippet depends on a current signature (`with_session`'s return, `base_manifest`, `restart`), the step names the place to read and says how to adapt.
- **Type consistency:** these names are used identically in every task: `TabGate`, `TabOpen::{Here, Opened { tab, shown }, Focused { tab, shown }, Refused}`, `TabTarget::{Pack, Theme}`, `TabVerdict::{ask, settle, refusal}`, `Kinds::{ALL, DECK_OR_HELP, PACK, THEME, or, has}`, `list_all`, `list_for`, `kinds_of`, `tool_names_for`, `tool_descriptions_for`, `Tools::new(…, kind)`, `Workspace::document_kind`, `Workspace::open_theme_for_agent`, `ActorParts.kind`, `Ctx.kind`, `session-undo-theme-rename`.
