# Lot 7 follow-ups (t-1277 to t-1281) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Parent task:** t-1152. **Sub-tasks:**

| Task | Sub-task |
|---|---|
| 1 | t-1278 |
| 2 | t-1279 |
| 3 | t-1280 |
| 4 | t-1277 |
| 5 | t-1281 |
| 6 | t-1282 |

**Goal:** Close the five follow-ups that lot 7 (t-1191) left under t-1152.

**Architecture:** Five changes on top of lot 7:
- A theme tab stops listing three shared tools that cannot work there.
- The local agent routes only to the families its tab's own tools give.
- A help tab opens library packs and themes.
- A deck's agent passes the user's request to the tab it opens, as a prefilled draft, through a new MCP port that the agent stack implements.
- The last task closes small test and doc gaps.

**Tech Stack:** Rust workspace (`oku-slides-mcp`, `-session`, `-agent`, `-local-agent`, `-i18n`), React UI (`ui`),
Fluent in 20 locales, Playwright e2e (`ui/e2e`).

**Spec:** `docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md`, lot 7 implementation note.

**User decisions** (2026-10-30, binding):
- **t-1277:** prefill the opened tab's draft and never start a turn. The theme tab shows its composer, so the draft can be seen there.
- **t-1278:** drop only the tools that cannot work.
- **t-1280:** allow library packs from help.
- **t-1279:** count only kind tools when offering families.

**v2** follows a 15-agent plan review: `.superpowers/sdd/2026-10-30-lot7-follow-ups/plan-review-findings.md`.

## Global Constraints

- Every root `AGENTS.md` invariant applies, in particular:
  - no unwrap or expect outside tests;
  - Snafu errors carry a backtrace and a Fluent message;
  - every pub item is documented, with a first sentence under 15 words;
  - files aim at 500 lines and never exceed 1000;
  - each `ARCHITECTURE.md` is at most 50 lines;
  - no `std::fs` in production code;
  - mutex locks use `unwrap_or_else(PoisonError::into_inner)`;
  - no std mutex guard is held across an await.
- New user-facing strings go through Fluent in all 20 locales, with English as the reference. Tests check the key or variant, plus `fr`.
- Tests:
  - name them `unit_scenario_result`, one behaviour per test, with `rstest` for tables;
  - assert with speculoos;
  - put unit tests in a sibling `*_tests.rs`;
  - create temp dirs with `tempfile` and keep the guard alive.
- A rule change updates the nearest `AGENTS.md` in the same commit. Each rule ends with its source, `(t-N)`. `just agents-lint` must stay green, and both the mcp and session `AGENTS.md` files are at the 60-line cap.
- Comments and docs are in English. French text in a block you edit is translated.
- Commits are `type(scope): description (t-1152, t-N)`, with `!` when a tool list or a public signature changes. Implementers stage only; the controller commits with `-F`.

---

### Task 1: A theme tab drops widgets, packs and ask_rewrite (t-1278)

**Files:**
- `crates/oku-slides-mcp/src/tools.rs` (`list_all`);
- `crates/oku-slides-mcp/src/tools_tests.rs` (the exact per-kind lists and `THEME_TOOLS`);
- `crates/oku-slides-mcp/tests/theme_tab.rs`;
- `crates/oku-slides-agent/src/instructions/agent.md`, `deck.md` and `pack.md`;
- `crates/oku-slides-agent/src/instructions_tests.rs`;
- the mcp `AGENTS.md` and `ARCHITECTURE.md` where they describe the theme list.

Steps:
- In `list_all`, change `widgets`, `packs` and `ask_rewrite` from `Kinds::ALL` to `Kinds::DECK_OR_HELP.or(Kinds::PACK)`.
- Every other shared tool stays listed for every kind, the app settings tools included (user decision).
- Fix the text that names the dropped tools and that a theme tab still reads:
  - **agent.md** (the shared core): line 1 backticks `ask_rewrite`, and the Languages paragraph names it in prose. Move those sentences into `deck.md` and `pack.md`.
  - **`apply` description:** it says "see widgets for its schema". Rewrite it so it does not name `widgets` for a theme tab, for example "(a deck or pack tab reads its schema with widgets)", unbackticked.
  - **`packs` description:** it now reads "A deck or help tab opens one in its own tab." Task 3 makes help able to open packs.
  - Grep `theme_tokens`, `deck_outline`, `show_params` and `show_variants` for the three names too.
- Tests:
  - In `tests/theme_tab.rs`, a theme tab's agent that calls `widgets`, `packs` or `ask_rewrite` by name, through `call` rather than `tools/list`, gets `UnknownTool`. Use an rstest table.
  - The deck, help and pack lists are unchanged.
  - Fix the per-kind lists in `tools_tests.rs`.
  - `instructions_for_a_kind_name_only_its_tools` passes for every kind.

### Task 2: The local agent routes only to the families its tab's own tools give (t-1279)

**Files:**
- `crates/oku-slides-local-agent/src/families.rs`, `engine.rs` (`Completion`), `turn.rs`, `gguf.rs`, `fake.rs`;
- `families_tests.rs`, `turn_tests.rs`, `gguf_tests.rs`, `fake_tests.rs` and `chatml_tests.rs`;
- `crates/oku-slides-local-agent/ARCHITECTURE.md`.

**Rule.** A routable family is offered when one of the tab's listed tools that is *not shared by every kind* belongs to it. The tools shared by every kind are those in `oku_slides_mcp::tool_names_for(k)` for all four `DocumentKind`s; compute that set once. `Configure` is always offered, because its tools are all shared. `AskReport` is always exposed and never routed.

Expected sets, tested against the real `tool_names_for` lists:

| Kind | Families offered |
|---|---|
| deck, help | all six |
| pack | `edit`, `packs`, `configure` |
| theme | `edit`, `packs`, `configure` |

Steps:
- Add `offered: Vec<Family>` to `Completion`, in `Family::ROUTABLE` order and without duplicates. Update every `Completion { .. }` literal.
- `route_grammar(&[Family])` and `parse_route(reply, &[Family])` take the offered set. The `turn.rs:154` prompt lists only offered families. Keep the `expect(dead_code)` on `route_grammar`.
- `play` reads `all` before routing. **If the offered set has one family or none, skip the Route step** and call `exposed(&all, &offered)`. This avoids an empty GBNF rule.
- `parse_route` drops a family that is not offered. If nothing offered remains, the turn ends as today on `NoRoute`, with no fallback to another family.
- Translate all of `families.rs`'s French docs.
- Tests:
  - the table above, as an rstest over the four kinds;
  - the grammar for a pack set lists exactly its keys;
  - the grammar is never built for a set of one or none (route skipped);
  - `parse_route` ignores `present` when it is not offered;
  - a `turn_tests` fake-engine turn on a theme-tab list exposes no `files` tools;
  - update `gguf_tests.rs:50,72`.
- This is not a bug fix (no wrong output today: `deck_outline` and `focus` are reachable), so it needs no `tests/regressions` file.

### Task 3: A help tab opens library packs and themes in their own tab (t-1280)

**Files:**
- `crates/oku-slides-session/src/pack_tab.rs` (`route_pack`, `take_preview`, a new private check), `pack_mode.rs` (the comment at :287);
- `deck_pack_link.rs` (read only, to confirm the refusal);
- `pack_tab_route_tests.rs` and `pack_tab_tests.rs`;
- `crates/oku-slides-session/tests/agent_pack_tabs.rs` (the help `open_pack` test becomes a success);
- `crates/oku-slides-agent/src/instructions/deck.md`;
- the session `AGENTS.md` and `ARCHITECTURE.md` if they state the help refusal.

**Today:**
- `can_open_pack`, `take_preview` and `preview_seed` call `ensure_editable`, which refuses help through `ensure_not_help` (`history.rs:187`).
- `can_open_pack` also guards `new_pack` (`workspace_packs.rs:179`).
- `route_theme` and `open_theme` already let help open a theme (`workspace_themes.rs:70`).

Steps:
- Leave `can_open_pack`, `ensure_editable` and `preview_seed` unchanged, so `new_pack` stays refused from help.
- Add a private `can_open_library_pack`. It refuses `InHistory` and theme tabs as today, and allows `Mode::Help`.
  - `route_pack` and `take_preview` use it.
  - `route_pack` gets an explicit `Mode::Help` arm: a deck-only pack, or any route other than `PackRoute::Tab`, is refused with `DeckOnly` (or `UnknownPack` for an id the help deck does not pin). A help deck never gets a linked deck-pack tab.
  - From help, `take_preview` copies the help deck through a private helper that ends no widget session and writes nothing.
- An absent `pack` from help resolves through `main_pack` as for a deck (essentiel or base, library packs). That is allowed.
- Fix the `pack_mode.rs:287` comment, which assumes help is refused earlier.
- Themes: add tests only.
- Tests:
  - from help, a library pack opens a pack tab whose preview renders;
  - the help deck's snapshot or revision is unchanged afterwards;
  - from help, an absent `pack` opens essentiel or base;
  - an rstest refusal table:
    - help `new_pack` → `ReadOnly`;
    - help deck-only or unpinned pack → the variant;
    - history → `InHistory`;
    - theme tab → `WrongDocument`;
  - one `fr` refusal check;
  - from help, a library theme opens a theme tab;
  - from help, an unknown theme is refused;
  - the MCP agent-path test in `tests/agent_pack_tabs.rs` now expects success.
- `deck.md`: keep the substrings pinned by `deck_instructions_name_the_app_tools_that_replace_the_menu`.

### Task 4: The opened tab gets the user's request as a prefilled draft (t-1277)

**Design** (from the architecture review). mcp cannot reach the agent stack, because `oku-slides-agent` depends on `oku-slides-mcp`. The work splits into five parts.

1. **Port in mcp:** `crates/oku-slides-mcp/src/draft.rs`.
   - `pub trait DraftPort: Send + Sync + Debug`, documented "Implement this trait to …", like `SettingsPort`.
   - It has one method, `fn prefill(&self, tab: &str, text: String) -> Prefill`.
   - `pub enum Prefill { Set, Kept, Unavailable }`:
     - `Kept`: the target already has a non-empty draft;
     - `Unavailable`: no such tab, or no port wired.
   - Add `NoDrafts` behind `test-support`, returning `Unavailable`. Re-export everything at the crate root.
   - `Tools::new` / `ToolServer::start` gain an `Arc<dyn DraftPort>`. Update every caller:
     - `agent/src/tab_entry.rs` (:327, :353, :372);
     - `mcp/src/hub_tests.rs:49` and `hub_thread_tests.rs:50`;
     - `mcp/tests/common`;
     - the agent and local-agent `tests/common`;
     - the CLI `serve` and `src-tauri` wiring, if they build `Tools`.
2. **Request argument.** `open_pack` and `open_theme` take an optional `request: String`, described as "the user's request, restated for the opened tab; it waits in that tab's draft for the user to review and send".
   - Normalize it before the gate: trim it, drop control characters other than `\n` and `\t`, and drop the bidi overrides U+202A–U+202E and U+2066–U+2069.
   - An empty result is treated as no request.
   - Cap it at `MAX_REQUEST_BYTES = 2048` bytes, a constant in `draft.rs` counted in bytes. Over the cap, the open still happens, the draft is not set, and the outcome is `too_long`. Never truncate.
3. **Call site.** In `open_tab`, call the port only on `TabOpen::Opened` or `Focused`, after `spawn_blocking` returns and the verdict is settled.
   - Never call it inside the gate closure, under the opening lock or under the hub lock.
   - Run it in `spawn_blocking`, because the stack takes its table lock and may start an entry.
   - `Here` and `Refused` never reach the port. The one-tab-move gate is unchanged.
4. **Agent side:** `crates/oku-slides-agent/src/drafts_port.rs`, a new module, because `stack.rs` is at 537 lines.
   - `StackDrafts` holds a `Weak` handle to the stack's table and `out_tx`; it is `Weak` because Stack → TabEntry → Tools → port. It is wired through `TabPorts` beside `settings` and `voice`.
   - `prefill` starts the target's entry with `Stack::open` (tools and keeper, **never the ACP agent**), then calls a new `TabLink::prefill_draft(out, text) -> bool`.
   - `prefill_draft` takes only the `draft` mutex, once. It sets the text and emits `AgentEvent::Draft { origin: None, .. }` under that lock, but only when the draft is empty. A draft counts as non-empty when its text is non-blank or it holds images. A draft of `None` (closed) is empty and is opened.
   - `prefill_draft` does not take the `paste` lock.
   - Reword `crates/oku-slides-agent/AGENTS.md:15` to: the host prefill may start a tab's entry, never its ACP agent, and never overwrites a non-empty draft (t-1277). The UI's `set_draft` still refuses a tab with no entry.
5. **Result and note.**
   - The result JSON gains `draft: "set" | "kept" | "too_long" | "unavailable"`. The field is absent when no request was given.
   - The request text is never echoed, either in the result or as a Fluent argument.
   - The `agent-pack-opened-in-tab` and `agent-theme-opened-in-tab` notes gain a `$draft` select (`none`, `set`, `kept`, `too_long`, `unavailable`) in all 20 locales. For `set`, the note says the request waits in that tab's draft and the user should review it before sending.
   - Update the exact-note tests (`tests/open_pack_tabs.rs:38`, `tests/open_theme_tabs.rs:22`).
6. **Theme tab composer (UI).** `ui/src/pack.css:146` hides every child of `.notes.theme-active` except `.theme-panel`. Keep `DictationDraft` (the composer) visible below the theme panel; the agent thread itself may stay hidden.
   - Change the CSS and `App.tsx:452` as needed, and fix the `ThemePanels` comment.
   - Add a `*.render.test.tsx` showing the composer in a theme tab.
   - Update `ui/AGENTS.md`.
7. **Instructions.** `deck.md` tells the agent to pass `request` when the user asked for a change to make in the opened tab. Document the trust boundary in the mcp `AGENTS.md`: the request is agent-written data, shown as a draft and only sent by the user (t-1277).

**Tests:**
- mcp, with a recording `DraftPort`, in `tests/open_pack_tabs.rs` and `tests/open_theme_tabs.rs`:
  - each of Opened, Focused, Here and Refused, with the port called only for the first two;
  - request omitted, empty and whitespace-only: the port is not called and no `draft` field appears;
  - exactly 2048 bytes is accepted;
  - 2049 bytes gives `too_long` with the open still `ok`;
  - multibyte text is counted in bytes;
  - control and bidi characters are stripped;
  - the request text is absent from the result JSON;
  - the note key and `$draft` value, in `fr`;
  - `NoDrafts` gives `unavailable`.
- Agent, in `drafts_port_tests.rs` and `tab_entry_tests.rs`:
  - `prefill_draft` on an empty or closed draft sets it and emits `Draft { origin: None }`;
  - a non-blank draft is kept, and a draft holding images only is kept;
  - two concurrent prefills leave exactly one winner, and a user `set_draft` after a prefill wins;
  - a prefill on a never-started tab starts its entry but no ACP agent (no `session/new` or `session/prompt`);
  - prefill, then `Follow`, replays the draft;
  - no turn starts after a prefill.
- e2e:
  - Add a new prompt entry in `fixtures/agent/e2e.json`, `"ouvre le thème avec une demande"`, that calls `open_theme` with `{theme: "papier", request: "Passe les titres en bleu."}`. Check whether `e2e-bis.json` needs a matching entry.
  - In `200-theme-editor.cjs`, after the theme tab opens: reload, wait for the theme tab's `.draft-text` to be visible with that value, and assert the fake-agent log has no `session/prompt` for the theme tab.
  - Closing the tabs uses `discard: true`, so no dialog fires.
- Before `just e2e`, run `bun run build` in `ui/`.

### Task 5: Test and doc gaps (t-1281)

All the items are small; send them in one dispatch.

1. Add an agent-path `Here` test for opening a missing theme. Put it in `crates/oku-slides-mcp/tests/open_theme_tabs.rs`.
2. `crates/oku-slides-agent/src/tab_entry_tests.rs`:
   - Fix the `closed_tab` doc, which says t-1 while the code uses t-404.
   - Rename `agent_run_on_a_closed_tab_answers_tools_unavailable` to `agent_run_on_a_closed_tab_answers_unknown_tab`.
3. In `document_kind_tests`, in the `Move` case, use a `tempfile` dir with its guard kept alive, not the shared `temp_dir()`.
4. `crates/oku-slides-mcp/tests/common/kinds.rs` `kind_of`:
   - Keep the closed-tab stand-in (`start_closed`).
   - For any other host error, panic with the error rather than falling back silently. Keep `#![expect(clippy::panic, reason)]`.
5. `waitForTool` (`ui/e2e/lib/agentLog.cjs:37`) already returns once the tool is seen; make it fail fast when the turn ends without calling the tool.
   - `crates/oku-slides-agent/src/bin/fake_agent.rs` logs a `turn/end` line when it answers a `session/prompt`.
   - `waitForTool` fails with a French message as soon as that prompt's `turn/end` appears without the tool.
   - Keep its signature (callers: `atelier.cjs:32`, `200-theme-editor.cjs:48`).
   - Add `ui/e2e/lib/agentLog.test.ts`, written like `run.test.ts`.
6. New tests:
   - `select_item` puts the item's trial values in the context: `crates/oku-slides-mcp/tests/pack_tab.rs`;
   - `save_pack` outside a pack tab is refused, asserting the variant: same file;
   - `Session::edited_pack_name` returns the name in a pack tab and `NotEditingPack` elsewhere: a sibling `*_tests.rs` of the module that defines it.
7. `undo_label`: merge the copies that share a signature into one helper, in the same test crate. `deck_packs_tests.rs:98` and `theme_edit_switch_tests.rs:39` are both in-crate unit tests. Leave `tests/data.rs:148`, which returns a different type. If the two in-crate copies differ in signature, leave them and say so in the report.
8. `insert_tab` (`workspace_layout.rs:130`) activates a tab in a fallback new window, yet `shown` stays false.
   - Set `shown` to whether the tab ends up its window's active tab (`tab_shown`), and return it to `TabOpen`.
   - Add a test on the fallback-window path.
   - Consumers: `open_tools.rs:119-131`, which the note's `$shown` follows.
9. Fix-wave leftovers (comment added to t-1281):
   - Arm the `FAIL_FINISH_VIEW` failpoint in its test only, and make the test fail if the failpoint was never hit.
   - Add a comment on `finish_shown`'s already-open branch, which ignores `Activate::IfShown`; only file and help openings reach it.
10. Split the `AGENTS.md` rules that were merged to fit the 60-line cap:
    - session: the line pairing `is_window_label` with `KEPT_TURNS`;
    - mcp: the merged settled decision.

    Do this last, once Tasks 1-4's rules are in. Each split rule stays in its crate's `AGENTS.md`, the deepest anchor that covers it. If a split would pass 60 lines, leave the rules merged, say so in the report, and keep `just agents-lint` green.

**Dropped from t-1281, recorded on the ticket:**
- the help `open_pack` refusal test, which Task 3 replaces;
- the plain-word backtick check: whole-word matching of English words such as `new`, `open`, `save`, `move` and `present` is noise, and a hand check found the texts clean;
- removing the `200-theme-editor` reload: the reload is needed because a non-active tab's thread is attached only on `Follow`.

### Task 6: Final review and gates

- Run the whole-branch review on the most capable model.
- Then do one fix wave and one scoped re-review. The re-review stops the loop: any remaining minors become a ticket.
- Run `bun run build` in `ui/`, then `just ci` and `just e2e 4999`.
- Mark t-1277 to t-1282 done, then ask the user before merging to main.
