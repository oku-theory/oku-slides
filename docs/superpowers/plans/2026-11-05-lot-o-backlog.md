# Lot O: open backlog (host hardening, voice debt, macOS barge-in) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every open backlog task that can be done on this machine: t-1500, t-1472, t-1498,
t-1469, t-1513 (seven items) and t-1491 (barge-in through macOS echo cancellation).

**Architecture:** Independent tasks, one commit each, on branch `backlog-complet-a1c90a`. Host
checks live in `oku-slides-session` (testable with nextest) and `src-tauri` only calls them. Voice
work stays in `oku-slides-stt` (engines, speaker), `oku-slides-mcp` (lexicon, hub asks) and
`oku-slides-agent` (ports, router). Barge-in adds a macOS duplex audio engine (voice-processing I/O:
capture and speech playback in one `AVAudioEngine`, so the echo canceller has the speech as reference).

**Tech Stack:** Rust (session, mcp, agent, stt, i18n, src-tauri), Swift (stt bridge), React/TS (Bun,
happy-dom render tests).

**Spec:** task bodies (`oku cli task info --project slider --id t-N`); hands-free spec
`docs/superpowers/specs/2026-10-18-pilotage-sans-mains-design.md` (Open questions: barge-in).
User decisions (2026-11-05): t-1491 = macOS AEC (voice-processing audio unit), other OSes keep the
microphone muted while Oku speaks; t-1513 item 5 = host-side action noun.

**Oku sub-tasks:** Task 4 = t-1517, Task 5 = t-1518, Tasks 6–12 = t-1519…t-1525, Task 13 =
t-1526, Task 14 = t-1527. Tasks 1–3 are their parents (t-1500, t-1472, t-1498). Commits cite the
parent and the sub-task, e.g. `(t-1469, t-1517)`.

**Blocked, out of this plan:** t-968 (no Apple Developer account), t-179 and t-1490 (Windows/Linux
hands), t-886 (no PowerPoint), t-888 (nothing to do on Typst 0.15.1), t-899 (ndarray-npy still on
zip ^6), t-1179 (@a2ui 0.15 still on zod 3), t-1510 (no Xcode 27 on runner images), t-1470 and
t-1489 (native speakers), t-635 (watch only), t-909/t-958 (corpus sources await user approval).

## Global Constraints

- Read root `AGENTS.md`, then the `AGENTS.md` and `ARCHITECTURE.md` of every directory you touch.
  `ARCHITECTURE.md` stays at most 50 lines (reword, don't grow) and is updated in the same commit.
- No `unwrap`/`expect` outside tests, no silent fallback; Snafu errors with backtrace, opaque public
  types, Fluent messages. A new `SessionError` variant needs: `SessionInner` variant, `SessionErrorKind`
  variant, `kind()` arm, `localize` arm (`crates/oku-slides-session/src/error.rs`).
- Every Fluent key goes into all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same position
  as in `en`. Test localized errors on the key or variant plus `fr`.
- No `std::fs` in production code (Storage/OpenDAL). Locks: `lock().unwrap_or_else(PoisonError::into_inner)`.
  Never hold a std mutex guard across an await.
- Rust unit tests in sibling `x_tests.rs` (`#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`),
  speculoos, names `unit_scenario_result`, one behaviour per test, rstest for tables; regressions in
  `tests/regressions/issue_NNNN_topic.rs` declared in that crate's `tests/regressions/main.rs`.
- UI: English comments in new code, `*.render.test.tsx` for DOM tests, module header comment.
- Files: aim 500 lines, never above 1000. Document every pub item (first sentence < 15 words).
- Temp data dirs: any manual run sets `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a
  fresh `mktemp -d`.
- Implementers **stage only** (`git add`), never commit; the controller commits after review with
  `type(scope): description (t-N)` and a `Reviewed-by:` trailer naming the review round.
- Gates per task: `cargo nextest run -p <crates touched> --retries 2`, `cargo clippy -p <crates>
  --all-targets -- -D warnings`, `cargo fmt --check`; UI tasks also `cd ui && bun test <files> && bun run typecheck`.

---

### Task 1: Re-arm modal frames when their window comes back (t-1500)

**Files:**
- Modify: `ui/src/modalFrame.ts:83-135` (`useModalFrame`: `openedAt`, `armed`, guards)
- Test: `ui/src/modalFrame.render.test.tsx` (create)

**Interfaces:** Produces nothing new; `useModalFrame` keeps its signature. All modals using it
(ReceivedDialog, ConfirmDialog, LayoutPicker, AboutDialog, ImportDialog, WidgetPicker,
AgentSurfaceBox) gain the behaviour.

- [ ] **Step 1: Failing tests** in `modalFrame.render.test.tsx`, rendering a tiny component that uses
  `useModalFrame(ref, { onEscape })` with a button whose `onClick` counts, `Date.now` mocked as in
  `ReceivedDialog.render.test.tsx:18-22`:
  - `useModalFrame window focus after arming ignores an immediate click`: advance `now` past
    `ARMING_MS`, dispatch `window` `focus`, click → count 0; advance `ARMING_MS + 1`, click → 1.
  - `useModalFrame visibility change to visible ignores an immediate click` (set
    `document.visibilityState` via `Object.defineProperty`, dispatch `visibilitychange`).
  - `useModalFrame window focus re-arms escape` (Escape right after focus does not call `onEscape`).
  - `useModalFrame hidden visibility change keeps the frame armed` (visibility `hidden` → click counts).
- [ ] **Step 2:** `cd ui && bun test src/modalFrame.render.test.tsx` → FAIL.
- [ ] **Step 3: Implement**: one `rearm()` callback that sets `openedAt.current = Date.now()`, sets
  `armed` false and restarts the `ARMING_MS` timer (clear the previous timer). An effect adds
  `window` `focus` and `document` `visibilitychange` (only when `visibilityState === "visible"`)
  listeners calling `rearm`, removed on cleanup (StrictMode-safe).
- [ ] **Step 4:** tests pass, plus `bun test src/ReceivedDialog.render.test.tsx` and `bun run typecheck`.
- [ ] **Step 5:** stage; message `fix(ui): re-arm modal clicks when the window regains focus (t-1500)`.

---

### Task 2: Sources budget on in-app additions (t-1472)

**Files:**
- Create: `crates/oku-slides-session/src/sources_budget.rs`, `sources_budget_tests.rs`
- Modify: `deck_pack_receive.rs:76-95,168-172` (move `files_bytes`/`source_bytes` out, call the new
  check), `agent/attachments.rs:147-172` (`embed_op_assets`), `deck_pack.rs:154`
  (`embed_from_library`), `deck_theme.rs` (`embed_op_closures`), `import/mod.rs:210-236`, `merge.rs:29`,
  `error.rs`, `lib.rs` (module line), `oku-slides-i18n/src/locales/*/session.ftl`, session `ARCHITECTURE.md`
- Test: `sources_budget_tests.rs`, `crates/oku-slides-session/tests/regressions/issue_1472_sources_budget.rs`

**Interfaces:**
- Produces (crate-private):
  ```rust
  /// Bytes the bundle weighs against `SourceLimits::total_bytes`.
  pub(crate) fn source_bytes(sources: &SourceBundle) -> Result<u64, SessionError>;
  pub(crate) fn files_bytes(files: &PackFiles) -> u64;
  impl Session {
      /// Refuses an addition of `added` bytes that would pass the sources budget.
      pub(crate) fn ensure_sources_room(&self, added: u64) -> Result<(), SessionError>;
  }
  ```
- New error `SessionInner::SourcesOverBudget { limit_mib: u64, backtrace }`, kind
  `SessionErrorKind::SourcesOverBudget`, key `session-sources-over-budget` (`$limit` MiB, number).
  en: `This would make the deck's files larger than { $limit } MiB, the most it can reopen.`

- [ ] **Step 1: Failing tests** (`sources_budget_tests.rs`, set `session.source_limits.total_bytes`
  just above the current `source_bytes`):
  - `ensure_sources_room_within_budget_accepts`, `ensure_sources_room_over_budget_refuses_with_kind`
  - `embed_op_assets_over_budget_adds_nothing` (asset bytes absent from the bundle afterwards)
  - `embed_op_assets_known_asset_costs_nothing` (an asset id already in the bundle is not counted)
  - `merge_over_budget_refuses`; `import_over_budget_refuses` (use a small fixture from
    `fixtures/import/`, limit set below its size)
  - `sources_over_budget_localizes_in_fr` (key text in `fr`)
  - regression `issue_1472_pasted_image_past_budget_is_refused` through the public request path that
    applies an op with a pasted asset.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-session -E 'test(sources_budget) | test(issue_1472)'` → FAIL.
- [ ] **Step 3: Implement.** Move the two helpers to `sources_budget.rs`; `ensure_sources_room`
  computes `source_bytes(&self.doc.sources)?.saturating_add(added)` and refuses above
  `self.source_limits.total_bytes` (`limit_mib = total_bytes / 1 MiB`). Call it before any bundle
  mutation: in `embed_op_assets` with the sum of new (absent) assets, in `embed_from_library` and the
  theme embeds with `files_bytes` of packs/themes not yet present, in import after building the bundle
  (compare the new bundle's `source_bytes`), in merge on the union. `deck_pack_receive` uses the same
  call (keep its `Mismatch::OverBudget` mapping). The replica snapshot export in `source_bytes` is
  only paid on additions.
- [ ] **Step 4:** tests green; `cargo nextest run -p oku-slides-session -p oku-slides-i18n --retries 2`.
- [ ] **Step 5:** stage; `fix(session): refuse in-app additions past the sources budget (t-1472)`.

---

### Task 3: A work folder per conversation (t-1498)

**Files:**
- Modify: `crates/oku-slides-session/src/agent/conversation.rs` (`SavedConversation.work: Option<Uuid>`,
  `#[serde(default)]`), `agent/work_dir.rs:58-99`, `agent/keeper.rs:183-190` (Forget),
  `resume_agent.rs` (doc of `work/`), session and agent `ARCHITECTURE.md`, `crates/oku-slides-agent/AGENTS.md`
  (rule "Give the ACP session the tab's work dir" → "the conversation's work dir")
- Test: `agent/work_dir_tests.rs`, `crates/oku-slides-agent/tests/work_dir.rs`, `tests/common/mod.rs:70`

**Interfaces:**
- `Session::agent_work_dir(&mut self) -> AgentWorkDir` (or current receiver) returns
  `<resume>/work/<uuid>`; the uuid is minted (v4) and stored in `self.conversation` (created if absent)
  and persisted through the existing `AgentWrite` path on first call. `forget_conversation` drops it;
  `Keep::Forget` deletes that conversation's folder (`Storage::delete_dir`) instead of emptying `work/`.

- [ ] **Step 1: Failing tests** (session): `agent_work_dir_new_conversation_is_under_work_uuid`,
  `agent_work_dir_twice_returns_same_folder`, `agent_work_dir_after_resume_keeps_folder` (write,
  reopen the resume, same path), `forget_then_work_dir_gives_new_folder_and_removes_old`. Agent
  integration (`tests/work_dir.rs`): new/resume/load cwd equals the conversation folder; new
  conversation gives a different cwd. Update the `work_dir(tab)` helper to read the saved uuid.
- [ ] **Step 2:** run them → FAIL.
- [ ] **Step 3: Implement** as above; `prepare()` stays the only creator of the folder; tab close still
  removes the whole resume dir.
- [ ] **Step 4:** `cargo nextest run -p oku-slides-session -p oku-slides-agent --retries 2` green.
- [ ] **Step 5:** stage; `fix(agent): give each conversation its own work folder (t-1498)`.

---

### Task 4: Host-side allowlist for the agent window (t-1469, part 1)

**Files:**
- Create: `crates/oku-slides-session/src/agent_window_allow.rs`, `agent_window_allow_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (module + re-export), `error.rs`
  (`AgentWindowForbidden { op: String }`, key `session-agent-window-forbidden`), 20 `session.ftl`,
  `src-tauri/src/commands.rs:132,162` (add `window: tauri::WebviewWindow`), `crates/oku-slides-cli/src/serve*.rs`
  (same check when `x-oku-window` is the agent label), `src-tauri/AGENTS.md` (rule), session `ARCHITECTURE.md`

**Interfaces:**
```rust
/// Refuses a workspace request the detached agent window may not send.
pub fn check_agent_workspace(ws: &Workspace, req: &WorkspaceRequest) -> Result<(), SessionError>;
/// Refuses a tab request the detached agent window may not send.
pub fn check_agent_request(ws: &Workspace, tab: &str, req: &Request) -> Result<(), SessionError>;
```
Allowed workspace ops: `View`, `Messages`, `Scheme`, `AttachAgent`, and `OpenTheme`/`OpenPack` whose
`tab` is held by a workspace window (`tab_window(tab).is_some()`). Allowed tab requests: `Snapshot`
and `Svg`, on a tab held by a workspace window. Both predicates are exhaustive `match`es (no `_`
arm) so a new variant forces a decision. The agent label constant moves to (or is mirrored with a
test in) the session crate: `pub const AGENT_WINDOW_LABEL: &str = "agent";` used by src-tauri
`agent_window::LABEL` and serve.

- [ ] **Step 1: Failing tests** (rstest tables): every allowed workspace variant passes; each refused
  one (`Close`, `CloseWindow`, `OpenTab`, `ImportTab`, `SameDeck`, `CarryAssets`, `Activate`, …) fails
  with kind `AgentWindowForbidden`; `OpenTheme` on an unknown tab is refused; `Snapshot` passes and
  `Save`/`Move`/`SaveCopy`/`Merge`/`Apply` fail; `fr` localization test.
- [ ] **Step 2:** FAIL. **Step 3:** implement predicates; in `commands::request`/`workspace`, when
  `window.label() == AGENT_WINDOW_LABEL`, call the check inside the existing `blocking` closure and
  refuse through `state.refuse(&e)`. In serve, same with the header.
- [ ] **Step 4:** `cargo nextest run -p oku-slides-session -p oku-slides-cli --retries 2`;
  `cargo check -p oku-slides-app`; `cargo clippy -p oku-slides-app -- -D warnings`.
- [ ] **Step 5:** stage; `fix(app): allow the agent window only its own requests (t-1469)`.

---

### Task 5: Agent link through a host relay; narrow the agent capability (t-1469, part 2)

**Files:**
- Modify: `src-tauri/src/commands.rs` (new `agent_link` command), `src-tauri/src/lib.rs`/handler list,
  `src-tauri/build.rs` `COMMANDS`, `src-tauri/capabilities/agent.json`, `default.json`
  (`allow-agent-link`), `ui/src/agent/link.ts:98-115` (Tauri transport calls `invoke("agent_link", { msg })`),
  its test, `src-tauri/AGENTS.md`
- Test: `src-tauri/src/commands_tests.rs` (relay validation), `ui/src/agent/link.test.ts`

**Interfaces:** `#[tauri::command] pub fn agent_link(app: AppHandle, window: WebviewWindow, msg:
serde_json::Value) -> Result<(), String>`: refuses a non-object or a message over 64 KiB, then
`app.emit("agent-link", msg)` (all windows, the sender too — same delivery as the old `emit`).
`agent.json` replaces `core:default` with the explicit sets the agent window uses:
`core:app:default`, `core:window:default`, `core:webview:default`, `core:event:allow-listen`,
`core:event:allow-unlisten`, plus its existing explicit grants and `allow-agent-link`
(no `core:event:allow-emit`/`allow-emit-to`). Check every `@tauri-apps/api` call reachable from
`AgentWindow.tsx` (`rg "@tauri-apps/api" ui/src`) against the granted sets.

- [ ] **Step 1: Failing tests**: a pure `fn validate_link(msg: &Value) -> Result<(), &'static str>`
  in commands with tests `validate_link_object_accepts`, `validate_link_array_refuses`,
  `validate_link_oversized_refuses`; UI test that the Tauri transport invokes `agent_link`.
- [ ] **Step 2–4:** implement; `cargo test -p oku-slides-app commands_tests` (the app crate is outside
  `just test`; run it directly), `cargo clippy -p oku-slides-app -- -D warnings`, `bun test src/agent`,
  `bun run typecheck`. Manual: `just dev` with temp dirs, detach the agent, send a prompt, insert a
  draft into the deck, reattach (controller does this check).
- [ ] **Step 5:** stage; `fix(app): relay the agent link through the host and narrow the agent window's capability (t-1469)`.

---

### Task 6: One tab port and a ports struct for `Tools::new` (t-1513 item 1)

**Files:**
- Create: `crates/oku-slides-mcp/src/tab_port.rs` (+ `_tests.rs`), `crates/oku-slides-agent/src/tab_port.rs` (+ `_tests.rs`)
- Delete: `crates/oku-slides-mcp/src/draft.rs` port parts and `tune_port.rs` (keep `Prefill`/`TuneRefusal`
  types, moved into `tab_port.rs`), `crates/oku-slides-agent/src/drafts_port.rs` port impl and `tune_port.rs`
- Modify: `crates/oku-slides-mcp/src/tools.rs:33-90`, `lib.rs` re-exports, `open_tools.rs:164`,
  `workspace_tools.rs:294`, `import_tools.rs:229`, `crates/oku-slides-agent/src/tab_entry.rs:159-163,385-451`,
  `stack.rs:120`, test commons of mcp/agent/local-agent, `local-agent/src/test_tools.rs:49`

**Interfaces:**
```rust
/// Host actions one tab's tools take on another tab.
pub trait TabPort: Send + Sync {
    fn prefill(&self, tab: &str, text: String) -> Prefill;
    fn page(&self, tab: &str, text: String) -> bool;
    fn tune(&self, tab: &str) -> Result<(), TuneRefusal>;
}
/// The host ports a tab's tools use.
pub struct HostPorts { pub settings: Arc<dyn SettingsPort>, pub tabs: Arc<dyn TabPort>,
                       pub workspace: Arc<dyn WorkspacePort>, pub app: Arc<dyn AppPort> }
// Tools::new(session, hub, ports: HostPorts, platform, kind)  — no too_many_arguments expect
```
(Match the existing port trait names/receivers exactly; the struct holds whatever ports `Tools::new`
takes today.) Test-support: `NoTabs` replaces `NoDrafts`/`NoTune`; `RecordingTabs` replaces the two
recording ports. Agent side: one `StackTabs` holding `Weak<Stack>` (reach the `Starter` through the
stack; add a `Weak<Starter>` field only if the stack cannot).

- [ ] **Step 1:** move the existing port tests to the new files first (they fail to compile = RED).
- [ ] **Step 2–4:** implement; `cargo nextest run -p oku-slides-mcp -p oku-slides-agent -p oku-slides-local-agent --retries 2`; clippy with no new `expect`.
- [ ] **Step 5:** stage; `refactor(mcp): merge the draft and tune ports into one tab port (t-1513)`.

---

### Task 7: Spoken theme shortcut through the Lexicon (t-1513 item 2)

**Files:** `crates/oku-slides-mcp/src/lexicon.rs` (+ tests), `crates/oku-slides-agent/src/voice_router.rs:104,184-196`
(+ tests), `stack.rs:342` region, `crates/oku-slides-i18n` (only if `voice_forms` cannot read
`ui-shortcut-theme`: then add `voice-shortcut-theme` to the 20 `voice.ftl`).

**Interfaces:** `Lexicon { theme: Vec<Form>, .. }`; `pub fn theme_shortcut(&self, said: &str) -> bool`
= `parse_all(&self.content(said), &[((), &self.theme)])`; `theme` forms join `one_word_forms` (segmentation).
`VoiceRoute::Shortcut` shows the themes directly (`hub.show_themes` or the function `stack.rs:342`
calls) instead of re-matching raw text. Typed prompts keep `shortcut.rs` (mirrors `ui/src/agent/shortcut.ts`).

- [ ] **Step 1: Failing tests** (rstest): `theme_shortcut_with_filler_matches` (en "um, themes"),
  `theme_shortcut_fr_matches`, `theme_shortcut_ja_unspaced_matches` (ja form + polite suffix as in the
  ja lexicon tests), `theme_shortcut_missing_locale_does_not_fall_back_to_english`,
  `route_voice_filler_theme_shows_themes` in `voice_router_tests.rs`.
- [ ] **Step 2–4:** implement; nextest mcp + agent green.
- [ ] **Step 5:** stage; `fix(agent): match the spoken theme shortcut through the lexicon (t-1513)`.

---

### Task 8: Shared hand-driven test voice and clock (t-1513 item 3)

**Files:** create `crates/oku-slides-stt/src/hand_voice.rs` under
`#[cfg(any(test, feature = "test-support"))]`; stt `Cargo.toml` `[features] test-support = []`;
`speaker_tests.rs:14-122` uses it; delete `crates/oku-slides-agent/src/hand_voice.rs` and its
`lib.rs:27-28` module; agent dev-deps `oku-slides-stt = { workspace = true, features = ["test-support"] }`;
`reader_tests.rs`, `settings_port_tests.rs`, `voice_port_speech_tests.rs` import from stt.

**Interfaces:** `pub struct HandClock` (as today), `pub struct HandEngine` = union: voices `v-fr`,
`v-fr2`; `spoken: Vec<(SpeechId, String, String)>`; `inline`, `refuse`, `echo` switches; `listed`
counter; `texts()`, `finish_all()`, `done(id)`; a missing voice is `""` (adapt stt assertions that
expected a panic). Document each pub item.

- [ ] Steps: move code (tests compile = proof), run `cargo nextest run -p oku-slides-stt -p oku-slides-agent --retries 2`; check `cargo build -p oku-slides-stt` (no feature) does not compile the module in.
- [ ] Stage; `test(stt): share the hand-driven voice and clock through test-support (t-1513)`.

---

### Task 9: Self-echo filter (t-1513 item 4)

**Files:** `crates/oku-slides-stt/src/speaker.rs` (store recent speech), create `echo.rs` +
`echo_tests.rs` (pure matcher), `voice_router.rs:184-190` (agent), stt `ARCHITECTURE.md`.

**Interfaces:**
```rust
/// True when `said` repeats, in a row, words of a speech that ended within ECHO_HORIZON.
pub fn Speaker::echoes(&self, said: &str, started: Instant) -> bool;
pub(crate) fn repeats(said: &str, spoken: &str) -> bool; // echo.rs, folded, pure
pub const ECHO_HORIZON: Duration = Duration::from_secs(3);
```
`repeats`: fold both with `oku_slides_i18n` folding; for spaced scripts compare word sequences
(the said words, at least 2, or all if fewer, appear consecutively in the spoken words); for
unspaced scripts (any char in CJK/Thai ranges) compare characters (said ≥ 3 chars appearing as a
substring). The speaker keeps the last 3 speeches' folded texts with their end instant (in memory
only, never logged); `echoes` checks `started <= end + ECHO_HORIZON`. Watchdog: per-script rate —
`CHARS_PER_SEC` 15 for spaced scripts, 6 for unspaced (constant documented).

- [ ] **Step 1: Failing tests:** `repeats` table (en exact run, en partial run, en one shared word →
  false, fr accents folded, ja substring, unrelated → false); `speaker_echoes_within_horizon_true`,
  `speaker_echoes_after_horizon_false` (HandClock), `watchdog_unspaced_text_gets_longer_bound`;
  agent `on_utterance_echo_of_speech_is_dropped`.
- [ ] Steps 2–4: implement; nextest stt + agent.
- [ ] Stage; `feat(stt): drop utterances that repeat Oku's recent speech (t-1513)`.

---

### Task 10: Name the action in spoken confirmations (t-1513 item 5)

**Files:** `crates/oku-slides-mcp/src/hub/asks.rs:260-270`, `crates/oku-slides-session/src/agent/spoken.rs:115-121`
(+ `spoken_tests.rs`), call sites `guarded_tools.rs:71,171,187,251,299,322`, `voice_tools.rs:147,196,233,252`,
`crates/oku-slides-agent/src/fallback_turn.rs:215`, 20 `voice.ftl`.

**Interfaces:**
```rust
/// What a plain confirmation asks the user to approve, read aloud.
pub enum ConfirmAction { Open, Save, NewDeck, CopyTheme, ChangeSetting, ChangeDictation, Download, SelectAgent, /* fallback_turn's case */ }
/// How a confirmation is answered: a plain yes, or yes plus a danger verb.
pub enum ConfirmAsk { Plain(ConfirmAction), Danger(ConfirmVerb) }
// SurfaceHub::confirm(message, ask: ConfirmAsk)
```
Put the enums where `ConfirmVerb` lives today. Keys `voice-action-open`, `-save`, `-new-deck`,
`-copy-theme`, `-change-setting`, `-change-dictation`, `-download`, `-select-agent` (infinitive
phrase, e.g. en "open the file") and `voice-say-confirm-action = To { $action }, say yes, or say no.`
(drop `voice-say-confirm-yes` if no longer used). `yes_no` unchanged.

- [ ] **Step 1: Failing tests:** `spoken_confirm_plain_names_the_action` (en + fr text),
  `every_confirm_action_has_a_key_in_every_locale`, mcp `voice_answers` test that a plain confirm's
  spoken prompt contains the action.
- [ ] Steps 2–4; nextest session + mcp + agent + i18n.
- [ ] Stage; `fix(mcp): name the action in spoken plain confirmations (t-1513)`.

---

### Task 11: Linux voice over SSIP (t-1513 item 6)

**Files:** create `crates/oku-slides-stt/src/speak/ssip.rs` (+ `ssip_tests.rs`), `#[cfg(unix)]`;
rewrite `speak/linux.rs` to use it (keep its status/voice listing contract), `linux_tests.rs`,
stt `ARCHITECTURE.md` (drop the known gap), `.github/workflows/voice-check.yml` only if it calls spd-say.

**Interfaces:** `pub(crate) struct Ssip<S: Read + Write>`: `connect(stream)` sends
`SET self CLIENT_NAME <user>:oku-slides:voice` and `SET self NOTIFICATION END on`/`CANCEL on`;
`speak(text, voice, rate)` sends `SET self SYNTHESIS_VOICE`, `SET self RATE`, `SPEAK`, the text with
CRLF lines and dot-stuffing (a line starting with `.` gets an extra `.`), `\r\n.\r\n`, reads the
`225` reply and message id; `cancel()` sends `CANCEL self`; `read_event()` parses `701`/`702`/`703`
notifications into Begin/End/Cancel for an id; `voices()` parses `LIST SYNTHESIS_VOICES`. Socket:
`$SPEECHD_ADDRESS` (`unix_socket:<path>`) else `$XDG_RUNTIME_DIR/speech-dispatcher/speechd.sock`;
absent socket → status `Unavailable` with reason. **Verify every command and reply code against the
SSIP manual** (https://htmlpreview.github.io/?https://github.com/brailcom/speechd/blob/master/doc/ssip.html
or the speechd repo `doc/ssip.texi`) before coding; cite the section in comments. Text never goes
to argv or logs; NUL → space.

- [ ] **Step 1: Failing tests** on an in-memory duplex (`Cursor` pair or `UnixStream::pair`):
  handshake bytes, dot-stuffing, CRLF conversion, `225` parse, `702` → End for the right id,
  `cancel` bytes, error reply (`4xx`) → error, voices parse.
- [ ] Steps 2–4: `cargo nextest run -p oku-slides-stt` (runs on macOS: module is `cfg(unix)`);
  `cargo check -p oku-slides-stt --target x86_64-unknown-linux-gnu` if the target is installed,
  else the controller relies on `voice-check.yml` after push.
- [ ] Stage; `fix(stt): speak through SSIP on Linux and cancel with CANCEL self (t-1513)`.

---

### Task 12: Extract the snapshot pipeline from App.tsx (t-1513 item 7)

**Files:** create `ui/src/useTabSnapshot.ts` (+ `useTabSnapshot.render.test.tsx`), modify `ui/src/App.tsx:134-201`.

**Interfaces:** `useTabSnapshot(api, { nonce, notice, onChange, say }) => { snap, snapRef, run }`
— exactly the state, refs and effects at App.tsx 134, 150-179, 197-201, moved unchanged.

- [ ] Steps: render test (`run` updates `snap`; a `nonce` change refetches) first, then move;
  `bun test src/useTabSnapshot.render.test.tsx src/App*.test.tsx`, `bun run typecheck`; App.tsx < 540 lines.
- [ ] Stage; `refactor(ui): move the tab snapshot pipeline out of App (t-1513)`.

---

### Task 13: macOS duplex audio with echo cancellation (t-1491, part 1)

**Files:** create `crates/oku-slides-stt/swift/Sources/OkuSpeech/OkuDuplex.swift`,
`crates/oku-slides-stt/src/duplex.rs` (+ `duplex_tests.rs`), `src/duplex_ffi.rs` (all `unsafe`, with
SAFETY comments); modify `src/audio.rs` (a `Recorder` source fed by the duplex), `src/speak/mod.rs`
(`DuplexVoice` engine), `build.rs` if needed, stt `ARCHITECTURE.md`, `crates/oku-slides-stt/AGENTS.md`.

**Design:** one `AVAudioEngine` with `inputNode.setVoiceProcessingEnabled(true)` (macOS 10.15+);
an input tap delivers mono `Float32` frames and the device rate through a C callback (same
`Captured` sink as cpal); an `AVAudioPlayerNode` on the main mixer plays speech rendered by
`AVSpeechSynthesizer.write(_:toBufferCallback:)` (converted to the player format with
`AVAudioConverter`); the last scheduled buffer's completion reports DONE; `stop` stops the player and
the synthesizer and reports DONE once. Because speech plays through the voice-processing unit, the
canceller removes it from the captured signal. Errors (`setVoiceProcessingEnabled` throws, engine
start fails) return codes mapped to `SttError` variants with Fluent keys; no `try!`, no `!`.
Rust: `Duplex::start(sink) -> Result<Duplex, SttError>`, `Duplex::voice() -> impl SpeechEngine`
(same contract as `speak/apple.rs`: one DONE per id), `Duplex::available() -> bool` (macOS only;
`false` elsewhere via `cfg`).

- [ ] **Step 1: Failing tests** (Rust, no hardware): FFI code mapping, `DuplexVoice` one-DONE-per-id
  with a fake bridge trait, capture frames resampled to 16 kHz through the existing path. One
  `#[ignore]` hardware test `duplex_speaks_and_captures` (speak a short text, expect DONE within 10 s
  and a non-zero capture level) that the controller runs by hand on macOS.
- [ ] Steps 2–4; `cargo nextest run -p oku-slides-stt`; `cargo test -p oku-slides-stt -- --ignored duplex` on this Mac.
- [ ] Stage; `feat(stt): macOS duplex audio with voice-processing echo cancellation (t-1491)`.

---

### Task 14: Barge-in while Oku speaks (t-1491, part 2)

**Files:** `crates/oku-slides-stt/src/speaker.rs` (barge-in mode), `capture.rs`/`listen.rs` (pick the
duplex when barge-in is on and available), `SpeechConfig` (`barge_in: bool`, default `true`,
`#[serde(default)]`), speech status (`barge_in` availability + reason), `set_speech` tool and
`VoicePort` (agent/mcp), settings box row (`ui/src/…` dictation settings, render test), 20 locales
(`stt.ftl`/`ui.ftl`/`voice.ftl` as the neighbours), hands-free spec: replace the barge-in open
question with the decision (Amendments 2026-11-05), stt `ARCHITECTURE.md`.

**Behaviour:** with barge-in active (macOS, duplex running, setting on): no `SuspendGuard` and no deaf
window during speech; while `Speaking`, the speaker polls `Stt::user_speaking()` every 50 ms and cuts
the speech (engine `stop`) when the user speaks; the utterance then goes through the normal router
(the self-echo filter of Task 9 still applies). Elsewhere, or with the setting off: today's behaviour
(muted). Status reason off macOS: key `speech-barge-in-unavailable-os` (en "Talking over Oku needs
echo cancellation, available on macOS only.").

- [ ] **Step 1: Failing tests** (HandEngine/HandClock, fake `user_speaking`):
  `barge_in_user_speaks_cuts_speech`, `barge_in_off_keeps_mic_suspended`,
  `barge_in_unavailable_keeps_mic_suspended`, `speech_config_without_barge_in_defaults_true`,
  settings box render test (row disabled with reason when unavailable), `set_speech` accepts `barge_in`.
- [ ] Steps 2–4; nextest stt + agent + mcp; `bun test` for the box; manual on this Mac: hands-free on,
  ask a question, speak during the answer → speech stops and the new utterance is handled.
- [ ] Stage; `feat(agent): talk over Oku to cut its speech on macOS (t-1491)`.

---

## Closing

- Final review of the whole branch (`git diff origin/main...HEAD`), then `just ci` and `just e2e 4999`
  in the background with logs under `.superpowers/sdd/2026-11-05-lot-o-backlog/`.
- Close tasks with `oku cli task done --project slider --id t-N --result "…"`; leave a comment on the
  blocked ones with today's upstream check.
- Squash onto `main`, push (memory: finish-merge-push).
