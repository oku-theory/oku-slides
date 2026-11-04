# Hands-free control, lots 2 to 8 (batch M) — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or
> superpowers:executing-plans). Steps use `- [ ]` checkboxes. Each task is RED → GREEN → REFACTOR.

**Status:** approved by the user on 2026-11-01 (draft 3, every recommended answer, Q11 included). It takes in two rounds of a 15-reviewer plan review (8 common
reviewers and 7 Rust and TS lenses each). The consolidated report is in the session notes folder
(`lot-m-plan-review.md`).

**Goal:** a whole Oku Slides session can be driven by voice: a listening loop that sends each
utterance, voice answers to confirmations, presenting by voice, the missing agent tools, and
read-aloud with the OS voice. Tickets t-996, t-997, t-998, t-999, t-1000, t-1001, t-1002.

**Architecture:**
- **Utterances stay in the host.** The STT crate gets a continuous *listening* state that closes
  utterances (numbered, timed by sample offsets) and hands them to the host on a channel. A
  host-side voice router sends each one to the active tab of the focused window. Utterance text
  never reaches the page or the agent's tools, except as the prompt it becomes. So the host alone
  knows a text came from the microphone. That keeps voice confirmations out of the agent's reach,
  and it lets local lexicons (stop, presentation commands, wake phrase) run before any model.
- **The page only reports.** It reports its context (selection, editing, presenting) and shows the
  indicator. Reports carry a page id and a seq, and the host accepts them only from the window
  that holds the tab. The window focus comes from the host's own window events, never from a page.
- **New tools reuse existing requests.** Workspace actions go through a new `AgentWorkspace` port;
  quit and updates go through a small host `AppPort`, run after the turn. Both traits live in
  `session::agent`, which src-tauri, cli and mcp already depend on.
- **The OS voice lives next to dictation** in the STT crate, so one coordinator mutes the
  microphone while Oku speaks.

**Tech stack:** Rust (stt, i18n, session, mcp, agent, cli, src-tauri), Swift (OkuSpeech bridge),
`windows` 0.62 WinRT (already locked), `spd-say` child process, React/TS (Bun), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-pilotage-sans-mains-design.md`, section Lots, as
amended on 2026-11-01. Its "Amendments" section is at the end. Until now the spec lived only on
the unmerged branch `vigilant-pasteur-ff1b06`; this batch brings it onto `main`.

## Global constraints

- **AGENTS.md rules.** Every rule applies: the root, `crates/`, `ui/` and `src-tauri/` files, and
  the stt, i18n, session, mcp, agent, local-agent and platform crate files. A rule this batch
  changes is edited in the same commit (each task's **Rules** line lists them).
- **Errors.** No `unwrap`/`expect` outside tests. Each refusal is a Snafu error with a backtrace in
  every variant and a Fluent key. MCP refusals go through `Tools::fail_with` with safe arguments
  only. Tests check the key plus a non-English locale (fr).
- **Arithmetic.** Use checked arithmetic on every agent-, page- or engine-supplied number
  (indices, ordinals, `Instant`/`Duration`, `f32`/`f64` rates and timestamps), and route casts
  through the `num` modules.
- **Locks.** Never hold a lock across an await or across a call into another crate's lock
  (session, workspace, `Stt`). The hub decides under its lock and acts after releasing it. Run
  blocking work (engines, `spd-say`, WinRT `.get()`, imports) on its own thread or in
  `spawn_blocking`, never on a tokio worker.
- **Files.** No `std::fs` in production; fakes included, everything goes through `Storage`.
  Files are capped at 1000 lines. Already near it: `stt/src/lib.rs` 889, `ui/src/api.ts` 862,
  `session/src/agent/forms.rs` 698, `agent/src/voice_port.rs` 599, so each task splits before it
  adds. The mcp, session and stt `ARCHITECTURE.md` are at 50 lines, so a task that adds modules
  trims in the same commit.
- **Strings.** Every new user-facing string, and every `voice-*` lexicon list, goes in all 20
  locales. English is the reference, and French uses real translations, which the e2e specs read.
  `voice.ftl` is loaded before `ui.ftl`.
- **Mirrors.**
  - New `Request`, `WorkspaceRequest` and `AgentCommand` variants are mirrored in TS. Voice and
    dictation types move to a new `ui/src/voiceApi.ts` (named re-exports, no `export *`).
  - New MCP events get a zod schema in `ui/src/agent/events.ts`; invalid events are dropped
    without logging.
  - New tool argument structs use `#[serde(deny_unknown_fields)]`; every new optional field has a
    serde default.
- **New MCP tool checklist.** Each new tool needs:
  - an entry in `list_all` with its `Kinds`, and an arm in `Tools::call`;
  - its line in the per-kind lists of `tools_tests.rs`;
  - a family in `crates/oku-slides-local-agent/src/families.rs` (the `families_every_mcp_tool_has_a_family` test);
  - a sentence in `crates/oku-slides-agent/src/instructions/agent.md` (paragraph 2), or in the kind's file (`deck.md` …) for a kind-scoped tool;
  - its family listed in `crates/oku-slides-mcp/ARCHITECTURE.md`;
  - scripted turns for e2e in `fixtures/agent/e2e.json`.
- **Runs.** Every run uses fresh `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` temp dirs.
  The first `just test` builds cold, so run it in the background. e2e specs are numbered after
  330 and read French labels; a negative check is always followed by a sentinel utterance that
  must appear, and never uses fixed sleeps.
- **Batch L.** It also edits `families.rs`, the tool list and the agent stack. Merge `main` at the
  start of every task.
- **Types and threads.**
  - Every new public type derives `Debug`; a type holding a closure gets a manual `Debug` impl.
  - Every new pub item has a doc comment, first sentence under 15 words.
  - Each host thread this batch adds (voice router, speaker, reader, after-turn runner) wraps its
    loop in `catch_unwind`, logs the panic and ends cleanly. It holds `Weak` references to the
    stack and exits when an upgrade fails.
  - Window actions go through `windows::sync` off the main thread, as `open_tab` already does (R32).
- **Local HTTP.** Every new serve route (context, presenting, close decision, listen) goes
  through `guard::check` (ADR-0003). Page-supplied structs use `deny_unknown_fields` and size
  limits checked in Rust.
- **Never logged.** Utterance text, spoken text and tab titles are never logged at any level;
  logs carry seq numbers and kinds only.

## Open questions for the user (my recommendation first)

Each answer changes a task. The plan is written for the recommendation.

1. **Who routes an utterance.** (a) *Recommended:* the host routes it.
   - The host takes each closed utterance from `Stt` and sends it to the active tab of the focused
     workspace window (the main window's active tab when the agent window is detached).
   - It resolves the open question itself and uses the last context that tab's page reported.
   - (b) The page polls and sends the utterance number back (draft 1). The reviewers showed that
     (b) breaks: `App` remounts on each tab switch, it is absent in read-only and help tabs, a
     background webview throttles its timers, and its list of open questions lags the hub's.
2. **What may answer a guarded confirmation.** *Recommended:* only a microphone utterance through
   the host lexicon. A typed reply leaves the question open and says "say it or click". It does not
   become a prompt either, so it no longer cancels the turn. Today a typed reply counts as a
   refusal. Typed text cannot be told apart from a draft the agent prefilled, so it cannot be
   trusted to confirm.
3. **Permissions by voice match the option kind, never its label.**
   - *Recommended:* "yes" picks the allow-once option, "no" picks reject-once, and anything else
     stays open. Allow-always and reject-always are never picked by voice.
   - The spec says "ordinal or label", but the agent writes both labels and order, so it could
     label allow-always "no". This is recorded in the spec amendments. Ordinals and labels stay
     for the variant drawer, which is undoable.
4. **The danger button shows its verb** ("Merge", "Delete", "Install and restart") instead of
   "Yes, do it". The verb you read is the verb you say. *Recommended.* The agent's own
   `ask_confirm` schema stays the same.
5. **Lexicons in 20 locales.** *Recommended:* I write English and French. The 18 others are my
   translations, to be reviewed by native speakers later (ticket "Review the voice lexicons per
   locale", created when the sub-tasks are mirrored). Alternative: ship en and fr only, and grey
   out hands-free in the other languages.
6. **Gating by focus.** *Recommended:*
   - While a workspace window has the focus, every utterance goes to its active tab (an utterance
     during a turn interrupts it, as the spec says).
   - While no workspace window has the focus (the app is in the background), only the stop word
     and wake-phrase utterances go through.
   - Danger confirmations by voice need a focused window. This guards against ambient speech
     (calls, videos, the audience).
   - **Where focus comes from.** The app tracks one focused flag from Tauri `Focused(true)` and
     `Focused(false)` on every window, the agent window included. Today `Workspace::focused()`
     always names the last active window and the host ignores `Focused(false)`.
   - **In serve** there are no OS windows. The page reports `document.hasFocus()` in its context,
     and that is used for routing only. Danger confirmations by voice are never accepted in
     serve; the user clicks.
   - **Known limit, documented in the help:** while the app has focus, ambient audio (a video)
     can still run tools that need no confirmation (edits, `undo_turn`, `set_interface`). Each of
     these can be undone.
7. **Who may turn hands-free on.** *Recommended:* only the user, from the settings box, a click or
   Voice Control. The agent may turn it off, never on. A prompt injection must not be able to
   start an always-on microphone.
   - The setting is saved, but every launch starts **paused**. Listening begins only after a user
     gesture: the indicator, ⌘D or the box switch. An agent that can write the config file
     therefore cannot get a microphone that starts on its own at the next launch.
8. **Tab titles shown to the agent** (`tabs`). An invariant (t-1191) says "never show the agent
   another tab's title".
   - *Recommended:* list titles cleaned with `agent::bounds::label`, clipped to 80 characters and
     framed as data. Narrow the invariant to hub errors and `open_pack`/`open_theme` replies.
     Titles are never read aloud.
   - Without titles, "go to the sales deck" cannot work. Alternative: `tab activate {title}`, where
     the host matches titles and only positions are shown.
9. **Which tools may move tabs.** *Recommended:* besides `open`, `open_pack` and `open_theme`,
   also `import`, `help`, `tab activate|move|reorder`. All of them go through
   `SurfaceHub::reserve_tab` (one move per turn) and the new `AgentWorkspace` port.
10. **Closing by voice.** *Recommended:*
    - The hub asks the confirmation. Unsaved work gets the `Discard` verb, and the message names it.
    - The page receives a close event with a one-time decision id. It redeems the id from the host
      and runs `closeTabFlow`/`closeWindowFlow` as usual, with saves included. The decision only
      answers the discard question, and only if the live dirty revision is the confirmed one.
    - A forged event without a redeemable id gets the normal dialog.
    - Ids are UUID v4 (the workspace `uuid` crate, a CSPRNG). Each is bound to its target, may be
      redeemed only from the window that holds that target, and expires after 30 s.
11. **CI for the Windows and Linux voices.** No workflow builds Windows or Linux before a release
    (`release.yml` only). *Recommended:* add a `voice-check.yml` workflow (pull requests and
    `main`; `windows-latest` and `ubuntu-latest`; `cargo check -p oku-slides-stt` plus the Linux
    `spd-say` tests). This is a CI change: I will make it only with your OK. Alternative: the first
    Windows compile happens at release time, plus a ticket.
12. **Voice language.** *Recommended:* use the voice of the interface locale only, and drop "or of
    the text's language". Detecting the language of agent text needs a new dependency, and the
    agent answers in the user's language anyway. Recorded in the spec amendments.
13. **Commits.** As asked, *recommended:* one squash commit on `main`, with WIP commits on the
    branch per task. Alternative (devil's advocate): three squashes, so a revert costs less and
    batch L conflicts stay smaller: (A) Tasks 6, 8–13, the agent tools; (B) Tasks 1–5 and 7, voice;
    (C) Tasks 14–15, read-aloud.
14. **Tab reorder and move to a window** (spec lot 7, "if still wanted"). *Recommended:* include
    them; the requests exist, so each is one arm.
15. **Wake phrase.** *Recommended:* a Fluent list of accepted forms per locale ("oku", "okou", …),
    with "Oku" passed in the STT vocabulary. Matching uses the folded text, but what goes to the
    agent is a slice of the original text.
16. **e2e fakes.** *Recommended:* a scripted utterance source and a logging voice, compiled only
    with a cargo feature `e2e-fakes` that `just e2e` turns on. Builds without that feature refuse
    the settings. Under Q2, a fake left in a release would let any process that can write a file
    act as the microphone.
    - The feature is chained `oku-slides-cli/e2e-fakes` → `oku-slides-stt/e2e-fakes` →
      `oku-slides-platform/e2e-fakes`, and it is enabled only on the `just e2e` command line,
      never in a `[dev-dependencies]` entry.
    - `stt` has `#[cfg(all(feature = "e2e-fakes", not(debug_assertions)))] compile_error!(…)`.
    - `just check` greps `cargo tree -e features -p oku-slides-app --release` and fails if the
      feature shows up.

## Sub-tasks (mirrored in Oku under each ticket)

| Task | Ticket | Sub-task | Title |
|---|---|---|---|
| 1 | t-996 | t-1473 | Listening state and utterances in the STT crate |
| 2 | t-997 | t-1474 | Voice lexicon: forms in i18n, matching in mcp |
| 3 | t-996 | t-1475 | Host voice router, stop word, hands-free setting |
| 4 | t-996 | t-1476 | Page context reports, indicator and e2e fakes |
| 5 | t-997 | t-1477 | Voice answers to confirmations, permissions and variants |
| 6 | t-998 | t-1478 | Escape regression test and fix |
| 7 | t-998 | t-1479 | Presentation by voice |
| 8 | t-999 | t-1480 | Undo, redo and undo_turn tools; focus deselect |
| 9 | t-999 | t-1481 | AppPort: update and quit after the turn; pack_update |
| 10 | t-999 | t-1482 | AgentWorkspace port and the import tool |
| 11 | t-1000 | t-1483 | tabs, tab and window tools |
| 12 | t-1000 | t-1484 | help and new_conversation tools |
| 13 | t-1001 | t-1485 | Layout through set_interface |
| 14 | t-1002 | t-1486 | OS voice engines in the STT crate |
| 15 | t-1002 | t-1487 | Read-aloud: speaker, reader, settings and box |
| 16 | all | t-1488 | Docs, instructions, review round, CI, squash |

**Order:**
1. Tasks 1 → 2 → 3 → 4 → 5, with Task 6 alongside.
2. Task 7.
3. Tasks 8 → 9 → 10 → 11 → 12 → 13. They do not need a microphone, so a subagent can run them
   while 1–5 run. Task 3 creates the `AgentWorkspace` trait with `focused_tab` alone and
   threads it through `Stack::start`, `TabPorts` and `Tools::new`. Task 10 adds the other
   methods, so the parallel track starts after Task 3 lands.
4. Tasks 14 → 15.
5. Task 16.

---

### Task 1: Listening state and utterances in the STT crate (t-996)

**Files:**
- Refactor first: move `listen_loop` and `pull` from `crates/oku-slides-stt/src/lib.rs` into
  `listen.rs`, with no behaviour change, tests green.
- Create: `utterance.rs` (+ `utterance_tests.rs`), `listen_tests.rs`.
- Modify: `lib.rs` (`State::Listening`, `SttRequest::Listen`, `Status`), `dictation_whisper.rs`
  (a bounded flush), `results.rs` (Apple finals), `error.rs`.
- Modify: `swift/Sources/OkuSpeech/OkuSpeech.swift`: the missing `errDenied` code.
- Docs: `ARCHITECTURE.md` (trimmed to 50), `AGENTS.md`.

**Interfaces:**
```rust
/// Number of a closed utterance, unique for the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub struct UtteranceSeq(pub u64);
/// One closed stretch of speech in listening mode; not Clone, not Serialize.
#[derive(Debug)] pub struct Utterance { pub seq: UtteranceSeq, pub text: String,
    pub started: Instant, pub ended: Instant, pub speech_secs: f32 }
pub enum State { Idle, Downloading, Recording, Transcribing, Listening }
pub enum SttRequest { Status, Start { vocabulary }, Listen { #[serde(default)] vocabulary }, Stop, Cancel }
impl Stt {
    /// Hands out the utterance receiver once; a second call is an error (`ReceiverTaken`).
    pub fn utterances(&self) -> Result<std::sync::mpsc::Receiver<Utterance>, SttError>;
    /// Drops captured samples and the utterance in progress until the guard drops.
    pub fn suspend(&self) -> SuspendGuard;
}
/// Keeps listening suspended while alive; nested guards are counted.
#[must_use] #[derive(Debug)] pub struct SuspendGuard { /* Weak<Inner>, epoch */ }
// Status gains `suspended: bool`; listening is `state == State::Listening` (one source of truth).
// Status never carries utterance text.
```
- The single `Sender` lives in `Inner` and survives a configure restart.
- `SuspendGuard` is the only suspend type in the crate. Task 15's speaker uses it too.

**Behaviour:**
- **Starting and stopping.** `Listen` enters `Listening`; `Start` is refused meanwhile, and a
  second `Listen` is a no-op. Listening ignores `nothing_heard_secs` and `stop_in`. `Stop` and
  `Cancel` end listening and discard the utterance in progress.
- **Closing an utterance** is edge-triggered. It closes only when a phrase arrived after the last
  close (`closed_upto`, a sample position) and the silence since then has reached
  `config.silence`.
- **Timing.** `started` and `ended` come from sample offsets: `recording_start + offset / 16 kHz`,
  computed with checked math. They are rebased at each resume. A suspend epoch drops late
  `Job::Final` results from a discarded utterance.
- **Whisper.** `Job::Flush { upto }` closes once the phrases before `upto` are transcribed and
  keeps the later ones. `upto` is an absolute phrase index, not a sample position.
  - `phrases` becomes a `VecDeque` with a `base` index.
  - A late `Final` looks itself up with `index.checked_sub(base).and_then(|i| phrases.get_mut(i))`;
    an index below `base` is dropped.
- **Apple.** An utterance is the finals since the last close; the stream is not finished.
- **Noise.** An utterance whose speech is shorter than `stt.min_phrase_secs` (0.3 s), or that is
  empty after trimming, gets no number. `stt.vad.min_phrase` (2.5 s) is the phrase segmenter's
  rule, not this one. The new key is an addition to the spec, recorded in its amendments.
- **Settings.** `configure` and `download` are accepted while listening: capture stops, the change
  applies, and capture starts again, with the utterance in progress discarded.
- **Every Idle-gated consumer accepts `Listening`.** Each needs a test:
  - `Stt::configure` and `Stt::download` (lib.rs:491, :568);
  - `check_patch`, `check_download` and the `busy` check (voice_port.rs:471);
  - `idle` in voice_port.rs:96, which feeds the download button in voice_panel.rs:252, so the
    button stays while listening;
  - the `stt_watch.rs` `Fingerprint`, which gains the state and `suspended`, so the box refreshes
    when listening pauses;
  - in the UI, `Dictation.tsx:254,334` and the `fakeStt` of `Dictation.render.test.tsx`.

- [ ] RED: `utterance_tests.rs`: `utterance_seq_never_reused_across_suspend`,
  `short_utterance_is_dropped_without_seq` (rstest: empty, spaces, 0.2 s),
  `utterance_times_from_sample_offsets_are_checked`.
- [ ] RED: `listen_tests.rs` on Whisper with `test_support` samples and generous `wait_until`
  deadlines (`stt.apple = false`):
  - `listen_two_phrases_with_silence_closes_two_utterances`
  - `listen_long_silence_after_utterance_closes_once`
  - `listen_phrase_after_flush_belongs_to_next_utterance`
  - `listen_suspend_discards_speech_in_progress`
  - `listen_stop_discards_utterance_in_progress`
  - `listen_start_while_listening_is_refused`
  - `listen_configure_while_listening_restarts_capture`
  - `listen_configure_restart_keeps_utterance_sender`
  - `listen_late_final_below_base_is_dropped`
  - `utterances_second_call_is_refused`
  - `suspend_nested_guards_resume_after_last_drop`
  - `voice_port_listening_keeps_download_button` and `stt_watch_suspended_change_refreshes_box`
- [ ] RED: Apple closing rule on the fake recognizer in `test_support`, on any host:
  `apple_finals_after_silence_close_one_utterance`.
- [ ] RED (regression, G16): `swift_error_codes_match_stream_raw` in `stream_tests.rs`.
  `stream::raw` is `pub(crate)`, so it reads `OkuSpeech.swift` with `include_str!` and compares.
  This is a unit test rather than a `tests/regressions` file, because only a unit test can see the
  codes. The deviation is recorded here.
- [ ] GREEN, REFACTOR, then `cargo nextest run -p oku-slides-stt`.
- [ ] Rules: stt AGENTS.md: "Hand each utterance out once, only to the host voice router; never
  put utterance text in Status." Testing: "Time utterances by sample offsets; test closing on
  silence with sample fixtures, never wall clock."

### Task 2: Voice lexicon: forms in i18n, matching in mcp (t-997)

**Files:**
- Create: `crates/oku-slides-i18n/src/locales/*/voice.ftl` (20 locales).
- Modify: `crates/oku-slides-i18n/src/lib.rs`: a `voice_forms(locale, key) -> Vec<String>`
  accessor, folded with `fold`.
- Create: `crates/oku-slides-mcp/src/lexicon.rs` (+ `lexicon_tests.rs`).
- Modify: the i18n `AGENTS.md`: forms that stay equal across locales, such as "oku", are allowed;
  no placeables in `voice-*` messages.

**Interfaces:**
```rust
/// Spoken command words of the user's languages, folded once per utterance.
#[derive(Debug)] pub struct Lexicon { /* per kind: Vec<Vec<String>> token forms */ }
pub enum YesNo { Yes, No, Unclear }
pub enum Nav { Next, Previous, First, Last, Exit }
impl Lexicon {
  /// Builds from the live UI and dictation locales; duplicates merged.
  pub fn for_locales(ui: &str, dictation: Option<&str>) -> Self;
  pub fn yes_no(&self, said: &str, verb: Option<&[String]>) -> YesNo;
  pub fn ordinal(&self, said: &str, count: usize) -> Option<usize>;   // 0-based, checked
  pub fn label(&self, said: &str, labels: &[&str]) -> Option<usize>;  // unique match only
  pub fn stop(&self, said: &str) -> bool;
  pub fn nav(&self, said: &str) -> Option<Nav>;
  /// The original text after the wake phrase, accents and case kept.
  pub fn wake<'a>(&self, said: &'a str) -> Option<&'a str>;
  /// The spoken forms of a confirmation verb, from `voice-verb-<name>`.
  pub fn verb(&self, verb: ConfirmVerb) -> &[Vec<String>]; }
```
- **Tokens.** Each token is folded on its own and keeps its byte range in the original text.
  `wake` slices the original at the range of the first token after the wake form. It never takes
  folded offsets, which differ in length once accents and case are folded.
- **Shortcut words.** The theme shortcut route reuses `oku_slides_i18n::theme_shortcuts()`; the
  lexicon does not copy it.
- `ConfirmVerb` is defined here in mcp (Task 5 uses it), with one `voice-verb-<name>` message per
  variant, holding comma-separated forms (`voice-verb-merge = fusionner, fusionne`).

**Message format and matching:**
- Messages hold comma-separated forms: `voice-yes`, `voice-no`, `voice-negation`, `voice-stop`,
  `voice-ordinals` (one message per position, up to 9), `voice-next`, `voice-previous`,
  `voice-first`, `voice-last`, `voice-exit`, `voice-wake`, `voice-filler`.
- An utterance is folded, then punctuation and filler words are dropped. The whole utterance must
  match.
- `yes_no` with a verb needs exactly one yes form and the verb, in any order. Any negation form
  makes it `No` or `Unclear`, never `Yes`. A bare verb is not enough.
- `label` returns a match only if exactly one label matches.

- [ ] RED (rstest tables in en, fr, de and ja):
  - `yes_no_yes_plus_verb_returns_yes`
  - `yes_no_negated_verb_never_returns_yes` ("ne fusionne pas", "non, fusionner", "pas
    fusionner", "don't merge", "nicht zusammenführen")
  - `yes_no_bare_verb_in_sentence_is_unclear`
  - `yes_no_yes_and_no_is_unclear`
  - `ordinal_zero_or_out_of_range_is_none`
  - `label_ambiguous_match_is_none`
  - `label_match_ignores_accents`
  - `wake_returns_original_slice` ("Oku, Écris le titre" → "Écris le titre")
  - `wake_false_positive_is_none` ("ok, où en étions-nous")
  - `wake_slice_after_multibyte_folding_is_on_char_boundary` ("ÖKU, Ärger" and "Oku、次へ")
  - `verb_forms_every_confirm_verb_defined` (every locale)
  - `for_locales_merges_ui_and_dictation`
- [ ] RED (i18n): `voice_forms_every_locale_defines_every_key`,
  `voice_forms_kinds_are_disjoint_per_locale` (no form is both yes and no, or both stop and nav,
  or a filler).
- [ ] GREEN, REFACTOR, then `cargo nextest run -p oku-slides-i18n -p oku-slides-mcp`.

### Task 3: Host voice router, stop word, hands-free setting (t-996)

**Files:**
- Create: `crates/oku-slides-agent/src/voice_router.rs` (+ tests). It owns the `Stt` utterance
  receiver on a dedicated thread and sends each routed utterance to the stack.
- Modify: `crates/oku-slides-agent/src/stack.rs`:
  - `AgentCommand::Context { tab, page, ui, seq, has_focus }`, the page's last context per tab,
    ordered by `(page, seq)`. A new page id resets the order, and its first report is accepted.
  - `AgentCommand::Presenting { tab, page, #[serde(default)] mode: Option<PresentMode>, seq }`;
  - both are added to `AgentCommand::tab()`;
  - both are handled inside the stack and never forwarded to the tab agent as a `Command`, since
    any command during a turn cancels it (turn.rs:118-126);
  - each is accepted only from the window that holds `tab`, and refused otherwise; `seq` is
    bounded below `u64::MAX`;
  - `AgentUi` gets `deny_unknown_fields` and size limits (selection count, text lengths), checked
    in Rust;
  - the host clears a tab's context and presenting state on a new page id, a tab close or a
    window destroy;
  - `route_prompt` keeps the typed path.
- Create: `crates/oku-slides-session/src/agent/workspace_port.rs`, holding the `AgentWorkspace`
  trait with `focused_tab` only (Task 10 adds the rest) and `NoWorkspace`. Thread it through
  `Stack::start(.., workspace: Arc<dyn AgentWorkspace>)`, `TabPorts` (drafts_port.rs:62) and
  `Tools::new`. Callers to update:
  - `tab_entry.rs:146,393`, and the mcp, agent and local-agent test commons (mcp:241, agent:143
    and :625, local-agent:107);
  - `src-tauri/src/main.rs:222`, `cli/src/serve.rs:369`, `serve_tests.rs:71`,
    `examples/smoke.rs:41`.
- Focus: `src-tauri/src/main.rs:139` handles `Focused(false)` too. `AppSessions` keeps one
  app-level `focused: Option<WindowId>`, cleared when no window of the app has the focus.
  `focused_tab()` returns the active tab of that window (the main window's when the agent window
  has the focus), and `None` when the app is in the background. The serve impl
  (`Mutex<Workspace>`) returns the tab of the last context whose `has_focus` is true, flagged
  `PageReported`, which danger confirmations refuse (Q6).
- Modify: `crates/oku-slides-stt/src/lib.rs` `Config.hands_free: bool` (serde default false);
  `crates/oku-slides-session/src/voice_view.rs` (`DictationPatch.hands_free`,
  `DictationView.hands_free`, the box action `hands_free {on}`); `voice_panel.rs` (a switch row);
  the hands-free patch logic moved to `crates/oku-slides-agent/src/voice_port_hands_free.rs` to
  keep `voice_port.rs` under 600 lines.
- Regenerate: `ui/src/a2ui/fixtures/agent-surfaces.json` (session rule).
- Modify: `crates/oku-slides-mcp/src/voice_tools.rs`: `SetDictationArgs.hands_free`, which accepts
  only `false` (Q7).

**Interfaces:**
```rust
/// Where a closed utterance goes, decided before any agent launch.
pub(crate) enum VoiceRoute { Stop { tab: String }, Answer { tab: String, heard: HeardUtterance },
    Nav { tab: String, nav: Nav }, Shortcut { tab: String, text: String }, Prompt { tab: String, text: String }, Drop(DropReason) }
pub(crate) fn route_voice(u: &Utterance, target: Option<&VoiceTarget>, lexicon: &Lexicon) -> VoiceRoute;
/// Why an utterance went nowhere; logged with the seq only.
pub(crate) enum DropReason { NoTarget, Background, Presenting, Noise }
/// The listening tab and its state, read from the workspace and the hub.
pub(crate) struct VoiceTarget { tab: String, focus: FocusSource, presenting: bool, open_question: bool, drawer: bool }
// session::agent: the tab that has the focus, and who says so.
pub struct FocusedTab { pub tab: String, pub source: FocusSource }
pub enum FocusSource { Host, PageReported }
pub trait AgentWorkspace: Send + Sync + std::fmt::Debug { fn focused_tab(&self) -> Option<FocusedTab>; }
// session::agent: how a turn began, kept with the turn (the reader uses it).
pub enum TurnOrigin { Typed, Voice, Wake }
// mcp: an utterance as the hub sees it (no stt dependency in mcp)
#[derive(Debug, Clone)] pub struct HeardUtterance { pub seq: u64, pub text: String, pub started: Instant }
```
- **The router thread** uses `std::sync::mpsc` with `recv_timeout(250 ms)`. It holds a
  `Weak<Stack>` and exits when the upgrade fails or `closing` is set; holding an `Arc` would form
  a cycle through `Stt`.
- **The routing order**, one table in `voice_router_tests.rs`:

| Utterance | Focused, idle | Focused, question open | Presenting | Background |
|---|---|---|---|---|
| stop word | Stop | Stop | Stop | Stop |
| answer form | Prompt | Answer | Drop | Drop |
| nav word | Prompt | Answer or Prompt | Nav | Drop |
| wake + rest | Prompt(rest), Wake | Prompt(rest), Wake | Prompt(rest), Wake | Prompt(rest), Wake |
| shortcut word | Shortcut | Shortcut | Drop | Drop |
| other | Prompt | Answer (kept open) | Drop | Drop |

**Routing order** (agent invariant amended): stop, then answer (Task 5), then presentation (Task
7), then the theme shortcut, then the prompt.
- **Stop** sends `Cancel` to that tab only.
- **No focused window** (Q6): only stop and wake-phrase utterances go through.
- **No target**: the utterance is dropped. This covers no window, a tab whose agent cannot start,
  and serve without `Stt`. The drop is logged at debug with its seq only, never its text.
- **The lexicon** is built per utterance from `LiveLocalizer::current()` and the dictation
  language.
- **The prompt** carries the tab's last reported `AgentUi`.

- [ ] RED: `voice_router_tests.rs`:
  - `stop_word_returns_stop` (rstest en, fr, de)
  - `stop_inside_a_sentence_is_a_prompt`
  - `background_utterance_without_wake_is_dropped`
  - `background_wake_phrase_prompts_rest`
  - `no_target_drops_without_logging_text`
  - `route_voice_table` (rstest, one case per row and column of the table above)
  - `context_older_seq_is_ignored`
  - `context_new_page_id_resets_seq`
  - `context_from_other_window_is_refused`
  - `context_oversized_ui_is_refused`
  - `router_exits_after_stack_shutdown`
  - `router_panic_is_logged_and_thread_ends`
- [ ] RED: stack tests against the fake agent, each waiting for an event (agent testing rule):
  - `voice_utterance_during_turn_cancels_and_replaces_it`
  - `stop_cancels_without_new_turn`
  - `context_during_turn_does_not_cancel_it`
  - `utterance_goes_to_focused_window_active_tab` (two windows)
  - `two_utterances_between_reads_both_prompt`
  - `wake_turn_is_marked_wake`
- [ ] RED (src-tauri): `focus_lost_by_every_window_has_no_focused_tab`,
  `agent_window_focus_targets_main_active_tab`. (serve): `serve_focus_is_page_reported`.
- [ ] RED: `voice_port` tests:
  - `hands_free_on_from_agent_is_refused_localized` (key + fr)
  - `hands_free_off_from_agent_applies`
  - `hands_free_patch_while_listening_applies`
  - `hands_free_saved_on_starts_paused_at_launch` (Q7)
- [ ] RED: `voice_view` test: the switch and its accessible label key.
- [ ] RED (regression, G16): `issue_0996_agent_cannot_enable_hands_free.rs` in mcp
  `tests/regressions`, declared in its `main.rs`.
- [ ] GREEN, REFACTOR, then `cargo nextest run -p oku-slides-stt -p oku-slides-session -p
  oku-slides-agent -p oku-slides-mcp`.
- [ ] Rules:
  - agent AGENTS.md: "Route utterances only in the host voice router: stop, then an answer, then
    the presentation lexicon, then a shortcut word, before any agent launch."
  - mcp AGENTS.md: "set_dictation may turn hands-free off, never on."

### Task 4: Page context reports, indicator and e2e fakes (t-996)

**Files:**
- Create: `ui/src/voiceApi.ts`. As a refactor first, move the stt and dictation types out of
  `api.ts`, and update every importer (no re-export from `api.ts`). Then add `listen` and
  `listening`.
- Modify: `api.ts`. The `AgentCommand` union (api.ts:715) gains `context` and `presenting`, both
  with a `page` id; this union is where agent commands live, not `voiceApi.ts`.
- Create: `ui/src/voiceContextStore.ts` (+ `voiceContextStore.test.ts`), a per-window store.
  `App` writes the `AgentUi` it already builds (App.tsx:441) and the `document.hasFocus()` flag.
- Create: `ui/src/useVoiceContext.ts` (+ `useVoiceContext.render.test.tsx`), mounted once per
  window in `ui/src/Workspace.tsx`. It reads the store for `win.active` and sends `Context` on a
  selection, editing or slide change (debounced by 150 ms, with a page id and a seq). It asks
  `Listen` at mount when hands-free is on and the user has resumed it.
  - Only the window that holds the tab reports its context.
  - `AgentWindow.tsx` never reports context; it sends `Listen` only.
- Modify: `ui/src/Dictation.tsx`: a steady listening indicator (`IconToggle` from
  `primitives.tsx`, labels `t("ui-hands-free-on")`, `t("ui-hands-free-pause")`). Its
  `aria-live` text is a fixed status string, never utterance text. ⌘D and the mic button pause or
  resume listening while hands-free is on (`suspend`). Push-to-talk is unchanged when hands-free
  is off.
- Create: shared test helper `ui/src/test/sttSpy.ts` (the folder `tsconfig` excludes). It is
  the `spyOn(stt, …)` pattern of `Dictation.render.test.tsx`, extended with `listen` and
  `listening`.
- Create: `crates/oku-slides-stt/src/fake_source.rs` behind cargo feature `e2e-fakes`. It injects
  closed utterances at the listen-loop level, reading lines appended to `stt.fake_utterances`
  through `Storage`. It starts at the end of the file, emits only complete lines, drops empty
  lines and lines received while suspended, and stops with `Stop` or on drop.
- Modify:
  - `crates/oku-slides-platform/src/config.rs`: `SttSettings.fake_utterances` behind the
    platform's own `e2e-fakes` feature, refused without it;
  - the feature chain and the release guard of Q16;
  - the `justfile`: the e2e recipe gets `--features oku-slides-cli/e2e-fakes`, the file goes
    under the run's temp dir, and `E2E_UTTERANCES` is set. `just test` and `just check` also run
    stt with the feature, so `fake_source_tests.rs` builds and runs.
- Create:
  - `ui/e2e/lib/say.cjs` (+ `say.test.ts`). It waits until the page shows the listening state,
    appends a line, then waits with `until` and `agentLog.logSince`.
  - `ui/e2e/340-hands-free.cjs`.
- Fluent `ui.ftl` (20 locales): `ui-hands-free-on`, `ui-hands-free-pause`, `ui-hands-free-paused`.

- [ ] RED: `useVoiceContext.render.test.tsx`:
  - `useVoiceContext selection change reports context once`
  - `useVoiceContext remount sends no Stop`
  - `useVoiceContext hands-free off asks no listen`
  - `useVoiceContext hands-free on and resumed asks listen at mount`
  - `useVoiceContext new page sends a new page id`
  - `voiceContextStore write from App is read by the hook` (in `voiceContextStore.test.ts`)
- [ ] RED: `Dictation.render.test.tsx`:
  - `Dictation listening shows steady indicator with label`
  - `Dictation live region never holds utterance text`
  - `Dictation mic while listening pauses`
- [ ] RED: `fake_source_tests.rs` (feature on):
  - `fake_source_starts_at_end_of_file`
  - `fake_source_partial_line_waits`
  - `fake_source_empty_line_dropped`
  - `fake_source_suspended_line_dropped`
- [ ] RED: platform test `fake_utterances_without_feature_is_refused`.
- [ ] RED: `340-hands-free.cjs` covers routing only (the real VAD is covered by Task 1):
  - turn hands-free on in the box;
  - say "ajoute une diapositive titre" → the prompt is logged;
  - with a turn the fake agent blocks until cancelled, say "stop", then the sentinel "bonjour"
    → the turn is cancelled, and only "bonjour" was prompted after it;
  - reload the page → listening is host state, so the indicator still shows it and the next
    utterance is prompted. The paused-at-launch rule is covered by the Rust test in Task 3, since
    the e2e server is shared and never relaunched.
- [ ] GREEN, REFACTOR, `just test` (background), `just e2e 4999`.
- [ ] Rules: ui AGENTS.md: "Never log utterance text in the page or send it from there;
  hands-free reports context and shows a fixed status." The one exception is the heard line of
  Task 5. It is sent only to the window that holds the question and shown as a `Text` component,
  never in a live region.

### Task 5: Voice answers to confirmations, permissions and variants (t-997)

**Files:**
- Create: `crates/oku-slides-mcp/src/hub/voice_answer.rs` (+ tests). Modify: `hub/asks.rs`:
  pending asks keep their kind, `shown_at`, verb forms and permission option kinds; `answer`
  returns `AnswerOutcome`.
- Modify: `hub/actions.rs` (variant by voice), `guarded_tools.rs` (`guarded_with`, the call sites
  at :71, :187, :299, :322), `voice_tools.rs` (`api_question`, `download_question`, :112, :147,
  :184, :203), `surface_tools.rs:109` (`ask_confirm`, the `forms::confirm` caller).
- Update the `answer()` callers: `mcp/src/hub_tests.rs:89,104,107,108`, `tests/surfaces.rs:92`,
  and `stack.rs:361` (a three-way match instead of `is_some_and`).
  `tests/regressions/issue_0222_permission_primary_never_allow_always.rs:69` changes meaning: a
  typed reply to a permission is now `KeptOpen`, not a refusal. Rewrite the test to say so and
  keep its point (allow-always is never the primary).
- Create: `crates/oku-slides-session/src/agent/forms_confirm.rs` (`confirm(message, danger_label:
  Option<String>)`, the heard line as a `Text` component). It is split out of `forms.rs`;
  `AskConfirm` stays unchanged.
- Modify: `crates/oku-slides-agent/src/stack.rs`. With `KeptOpen`, a typed reply stops there: no
  prompt, no turn cancel (Q2).
- Update: `crates/oku-slides-mcp/tests/surfaces.rs:584`, `tests/common/mod.rs`
  `confirmation_of`, `forms_tests.rs`, `turn_view_tests.rs`, `surface_ops_tests.rs`, and the
  existing tests that assert a typed reply refuses (renamed `…_keeps_it_open`). Regenerate the
  a2ui fixture.
- Fluent `agent.ftl`: one verb per `ConfirmVerb`, `agent-surface-heard`,
  `agent-surface-say-or-click`, `agent-surface-said-too-early`.
- Create: `crates/oku-slides-mcp/tests/regressions/issue_0997_agent_text_never_confirms.rs` and
  `issue_0997_permission_label_never_grants.rs`, declared in `main.rs`.

**Interfaces:**
```rust
/// The verb a danger confirmation's button shows and the user must say.
pub enum ConfirmVerb { Replace /*save over a file*/, Merge, Discard /*new over a dirty deck, close with unsaved work*/,
    Delete /*delete_pack*/, UseApi /*set_dictation to the API*/, Install /*update install*/, Quit, Close /*another tab*/, Import }
impl SurfaceHub {
  pub async fn confirm(&self, message: String, danger: Option<ConfirmVerb>) -> bool;
  /// Answers a typed reply; never confirms a guard or picks a permission.
  pub fn answer(&self, text: &str) -> AnswerOutcome;
  /// Answers the open question of this tab from a microphone utterance.
  pub fn answer_voice(&self, heard: HeardUtterance, lexicon: &Lexicon, focus: Option<FocusSource>) -> AnswerOutcome;
}
/// What a reply did to the open question; one enum for typed and voice replies.
pub enum AnswerOutcome { Answered, KeptOpen, NotAQuestion }
/// True when speech began at least ARMING after the question became answerable.
pub(crate) fn armed(armed_from: Instant, started: Instant) -> bool; // checked_add; None → false
```
- **`armed_from`** is `shown_at`, pushed back to the end of the question's speech when read-aloud
  speaks it (Task 15 sets it through `SurfaceHub::spoken_until(id, Instant)`).
- **Atomic voice answer.** The voice router calls `answer_voice` first and falls through to the
  other routes only on `NotAQuestion`. The question is looked up and answered under one hub lock,
  so a question that closes in between cannot take a prompt meant for the next one.

**Behaviour:**
- **Target.** The host takes the target from its own open list, with the same priority as the
  page's `quickTarget`: modal, then sheet, then dock question, then drawer. The page id is never
  needed.
- **Decide, then act.** The answer is decided under the state lock; the tx is sent and variant
  ops are applied after the lock is released.
- **Guarded confirm.**
  - `Yes` with the verb, armed, and with `FocusSource::Host` (Q6) confirms. `PageReported` focus
    (serve) or no focus keeps it open, with "click to confirm".
  - `No` refuses.
  - Anything else keeps it open with the heard text, clipped and labelled (`bounds::clip` and
    `bounds::label`).
  - An utterance that is not armed keeps it open with "said too early".
- **Plain hub confirms** (`confirm(.., None)` at guarded_tools.rs:187/299/322 and
  voice_tools.rs:147/184/203). A bare `Yes` or `No`, armed and from a focused window, answers
  them; there is no verb to say. Anything else keeps the question open.
- **Permission.** `Yes` picks the allow-once option (by kind), `No` picks reject-once. Anything
  else keeps it open (Q3).
- **Typed replies.** `answer` returns `KeptOpen` for a guard or a permission, and the stack shows
  "say it or click" without a prompt or a cancel. The agent's own `ask_confirm` and `ask_choice`
  are still answered by text.
- **Arming.** `ARMING` (500 ms) is one Rust const. It is tested equal to `ARMING_MS` in
  `ui/src/modalFrame.ts` through the shared fixture `fixtures/ui/arming.json`, read by a Rust test
  and by a new `ui/src/modalFrame.test.ts`. The modal sets `data-armed` once armed, and e2e waits
  on that attribute, never on a timer.
- **The agent's own questions.** `ask_choice` and `ask_confirm` keep `answer_text` (they are not
  guards).
- **Variant drawer.** An ordinal or a unique label applies the variant; otherwise the utterance is
  an ordinary prompt.

- [ ] RED (regressions, first):
  - `issue_0997_agent_text_never_confirms`: a typed `answer`, or an agent-prefilled draft sent, is
    `KeptOpen` and never confirms. Only `answer_voice` with an armed utterance does.
  - `issue_0997_permission_label_never_grants`: an allow-always option labelled "non" and placed
    first. Saying "non" picks reject-once, and "le premier" keeps the question open.
- [ ] RED: `voice_answer_tests.rs`:
  - `armed_boundaries` (rstest 499, 500 and 501 ms; started before shown)
  - `confirm_danger_without_verb_keeps_open_with_heard`
  - `confirm_unfocused_window_keeps_open`
  - `confirm_page_reported_focus_keeps_open` (serve)
  - `plain_confirm_bare_yes_confirms`
  - `answer_voice_on_closed_question_is_not_a_question`
  - `armed_from_end_of_speech`
  - `permission_yes_picks_allow_once`
  - `permission_always_never_by_voice`
  - `variant_ordinal_applies_one_undo_step`
  - `variant_non_matching_is_a_prompt`
  - `answer_decides_under_lock_acts_after` (no deadlock with a session lock held elsewhere)
- [ ] RED: stack test `typed_reply_to_guard_keeps_turn_running`. forms test
  `confirm_danger_button_label_is_verb` (key + fr).
- [ ] GREEN, REFACTOR. `350-voice-confirm.cjs`:
  - the fake agent calls `merge`;
  - wait until the modal has been shown longer than 500 ms;
  - say "oui" → still open, heard line shown;
  - say "oui, fusionner" → merged.
- [ ] Rules:
  - mcp AGENTS.md: "Answer a guarded confirmation or a permission only from a host-routed
    microphone utterance, through the lexicon: the verb for danger, the option kind for
    permissions; never from prompt text or agent-written labels."
  - The settled decision on `SurfaceHub::confirm` names `ConfirmVerb`.
  - ui AGENTS.md: the confirmation rule adds that the danger button shows the verb.

### Task 6: Escape regression test and fix (t-998)

**Files:** create `ui/src/presentDictation.render.test.tsx` (the Presenter-plus-Dictation
integration only); then modify `ui/src/Presenter.tsx` and `ui/src/Dictation.tsx`; create
`ui/src/dictationActive.ts` (+ `dictationActive.render.test.tsx`). It is a module-level counter,
the `voiceSeq.ts` pattern with `useSyncExternalStore`. Each Dictation increments it in an effect
and decrements it in the cleanup; `resetDictationActive()` is for tests.

- [ ] RED first (G13), named for t-998:
  - `t-998 Escape while push-to-talk in a presentation cancels only the dictation`, run in both
    mount orders. The Presenter-first order exists only to prove the fix does not depend on order;
    in the app, Dictation always registers first.
  - `t-998 Escape with no dictation exits the presentation`
  - `t-998 Escape while hands-free listening exits the presentation` (listening is not
    push-to-talk)
  - `t-998 Escape in a presentation does not dismiss an armed modal` (useQuickKeys,
    useModalFrame)
  - `dictationActive survives StrictMode remount` (in `dictationActive.render.test.tsx`)
- [ ] Diagnose with the failing tests (G14). The planned fix: Presenter yields Escape while
  `dictationActive()` is above zero, and the quick keys ignore Escape while presenting. If the
  tests show another cause, the plan is updated before the fix.
- [ ] GREEN; add one e2e step in `360-present-voice.cjs` (happy-dom does not reproduce listener
  phases faithfully). Then `just test`.
- [ ] Rules: ui AGENTS.md pitfall "Do not capture Escape in Dictation…" gains: "…; in a
  presentation an active push-to-talk takes Escape first, through dictationActive."

### Task 7: Presentation by voice (t-998)

**Files:**
- Modify: `crates/oku-slides-mcp/src/app_tools.rs` (`present` actions), `hub.rs`
  (`McpEvent::Present { command }`). The hub's presenting state comes only from the page's
  `Presenting` reports, ordered by `(page, seq)`. It is cleared on a tab close, a window destroy
  and a new page id (Task 3).
- Modify: `voice_router.rs` (`Nav` → `PresentCommand`).
- Create: `ui/src/usePresentReports.ts` (+ `usePresentReports.render.test.tsx`). The present
  wiring goes there, not in `App.tsx`, which is already at 534 lines.
- Modify: `ui/src/usePageEvents.ts`, `ui/src/Presenter.tsx`, `ui/src/agent/events.ts` (zod; the
  `McpEvent` union at api.ts:697-702 gains the present payload).
- Update: `fixtures/agent/e2e.json:33`, `crates/oku-slides-mcp/tests/app_tools.rs:102,111,581`,
  `ui/src/agent/AgentPanel.tsx:187`, `AgentPanel.render.test.tsx:218`.
- Create: `ui/e2e/360-present-voice.cjs`.

**Interfaces:**
```rust
pub enum PresentCommand { Start { mode: PresentMode }, Show { slide: String }, Next, Previous, First, Last, Exit }
impl From<Nav> for PresentCommand { /* Next, Previous, First, Last, Exit */ }
// present tool: { action?: "start" (default) | "show" | "next" | "previous" | "first" | "last" | "exit", mode?, slide? }
impl SurfaceHub { pub fn set_presenting(&self, mode: Option<PresentMode>, seq: u64); pub fn presenting(&self) -> Option<PresentMode>; }
```

**Behaviour:**
- **Actions.** Every action but `start` is refused with a localized `NotPresenting` when the hub
  has no presentation. `NotPresenting` is a variant of the mcp tool error enum, with a backtrace.
  `show` refuses an unknown slide, and the page also ignores an id missing from its snapshot. In
  zod, `show.slide` is a bounded string (≤ 64 characters).
- **Page reports.** The page reports from an effect on `presenting` and reports the end in that
  effect's cleanup (tab switch, window close, `screen.onLeft`). A report carries a seq.
- **Voice while presenting.** The order is stop, then answer, then `nav` (emitted at once, with no
  agent), then `wake` (the rest goes to the agent). Anything else is dropped. `first` and `last`
  are an addition to the spec's list: one arm each.

- [ ] RED:
  - mcp: one test per action; `present_without_action_starts` (the fixture stays valid);
    `present_next_not_presenting_refused_localized` (key + fr).
  - router: `presenting_nav_word_emits_nav`, `presenting_wake_phrase_prompts_rest`,
    `presenting_other_speech_is_dropped`.
  - hub: `presenting_older_seq_ignored`, `presenting_cleared_on_tab_close`,
    `presenting_cleared_on_window_destroy`, `presenting_cleared_on_new_page_id`.
  - Presenter: a `test.each` table `Presenter <command> event <result>`.
  - `usePresentReports.render.test.tsx`: `presenting end is reported on unmount`.
  - zod: `present show with oversized slide id is dropped`.
- [ ] GREEN. `360-present-voice.cjs`, in window mode:
  - say "suivant" → slide 2;
  - say "bonjour à tous", then the sentinel "Oku, quelle heure est-il" → only the sentinel is
    prompted;
  - say "quitter" → the presentation closes;
  - the Escape step from Task 6.
- [ ] Rules:
  - mcp AGENTS.md: the page-events invariant now covers the present commands, and presenting
    state comes only from page reports.
  - agent AGENTS.md: "While a tab presents, voice reaches its agent only after the wake phrase."

### Task 8: Undo, redo and undo_turn tools; focus deselect (t-999)

**Files:** create `crates/oku-slides-mcp/src/undo_tools.rs` (+ `undo_tools_tests.rs`); modify
`tools.rs`, `handler.rs`, `tools_tests.rs`, `edit_tools.rs` (`FocusArgs.slide: Option`,
`McpEvent::Focus.slide: Option`), `ui/src/agent/useAgentFocus.ts`, `ui/src/agent/events.ts`;
`families.rs` (Edit). Also update the `Focus.slide` consumers: `mcp/tests/edit.rs:160-161`, the
`api.ts:698` union, and the `onFocus` signature (`AgentPanel.tsx:114,185`, `App.tsx:446`).

**Interfaces:**
- `undo {steps?: 1..=20}` and `redo {steps?}` call `Request::Undo` or `Request::Redo` steps
  times, stopping at the first refusal and returning how many were done.
- `undo_turn {turn}` calls `Request::UndoTurn`.
- None of them asks a confirmation. Their kinds are `Kinds::DECK_PACK_OR_THEME`; help tabs are
  read-only.
- `focus { slide?, element? }`: with no slide and no element it deselects and keeps the slide.
  An `element` without a `slide` is refused.

- [ ] RED first, the risk check: session test `undo_inside_agent_turn_undoes_previous_step`
  (`begin_turn`, Undo, `end_turn`: the turn's group stays one step). If Loro's open group swallows
  the undo, the session ends the turn group before an agent undo and reopens it after. That
  deviation goes into this plan before any code.
  - **Deviation (risk confirmed, 2026-11-02).** The test fails: the undo itself works (it undoes
    the step below the turn), but `DeckReplica::undo` and `redo` close the open group, so the
    turn's later edits become one step each. Loro is not involved (groups are our own steps).
    Fix: `Session`'s undo and redo note the open group's label and its own step
    (`SourceBundle::group`, `group_step`, new, on the document and on the deck set aside) before
    the step. After it, they reopen any group the step closed with `resume_group`, whatever the
    outcome. The turn's next edit joins the turn's own step when that step is back on top (a redo
    of it, or a refused undo taken back), and otherwise opens a fresh step under the turn's label.
    Either way the turn stays one step. This applies to every undo or redo while a turn is open
    (the session cannot tell an agent undo from the user's), so a user's undo during a turn keeps
    the turn grouped too.
- [ ] RED:
  - `undo_steps_three_undoes_three`
  - `undo_steps_beyond_stack_stops_and_reports_count`
  - `undo_turn_not_on_top_refused_localized` (key + fr)
  - `undo_turn_of_current_turn_refused`
  - `focus_without_slide_deselects`
  - `focus_element_without_slide_refused`
  - TS: `useAgentFocus focus without slide deselects only`
- [ ] GREEN, REFACTOR.

### Task 9: AppPort: update and quit after the turn; pack_update (t-999)

**Files:**
- Create: `crates/oku-slides-session/src/agent/app_port.rs` (trait, `AppRefusal`, `UnavailableApp`).
  It lives in session because src-tauri and cli depend on session, not on mcp.
- Create: `crates/oku-slides-mcp/src/host_tools.rs` (+ tests). Modify: `hub_turns.rs`
  (`AfterTurn` actions).
- Modify: `crates/oku-slides-agent/src/tab_entry.rs` (`TabPorts.app`), `stack.rs`
  (`Stack::start(.., app: Arc<dyn AppPort>)`, next to Task 3's `workspace`).
- Update the same callers as Task 3: `examples/smoke.rs:41`, `tests/common/mod.rs:625` (agent),
  `cli/src/serve.rs:369`, `cli/src/serve_tests.rs:71`, `src-tauri/src/main.rs:222`, and the
  `Tools::new` callers (`tab_entry.rs:393`, the mcp, agent and local-agent test commons).
- Create: `src-tauri/src/app_port.rs`.
  - It holds an `AppHandle` clone and reaches `AppState` through `try_state`. Tauri has no weak
    handle; the cycle ends at exit.
  - Quit runs `state::shutdown`.
  - Install runs `updates::install`, made window-independent, so only the commands check
    `workspace_window`. `install` is async, so the after-turn thread calls it through
    `tauri::async_runtime::block_on`, as `state::settle` does.
  - Check and later reuse the guts of `check_update` and `later_update`.
- Modify: `crates/oku-slides-cli/src/serve.rs`: it passes `UnavailableApp`.
- Modify: `crates/oku-slides-session/src/deck_packs_panel.rs` (a public `packs_event_for(pack,
  action)` builds the same `PacksEvent` as the click), `crates/oku-slides-mcp/src/hub/packs.rs`.

**Interfaces:**
```rust
/// Implement this trait to let agent tools ask the host for app-level actions.
#[async_trait] pub trait AppPort: Send + Sync + std::fmt::Debug {
  async fn update(&self, action: UpdateAction) -> Result<UpdateState, AppRefusal>;
  /// Quits once the turn has ended; never called inside a tool.
  fn quit(&self) -> Result<(), AppRefusal>;
  /// Installs and restarts once the turn has ended; never called inside a tool.
  fn install(&self) -> Result<(), AppRefusal>;
}
pub enum UpdateAction { Check, Later }
pub enum UpdateState { UpToDate, Offer { version: String, installable: bool }, Unavailable }
/// Why the host refused (opaque Snafu with a Fluent key: unavailable here, closing, no offer).
pub struct AppRefusal(AppRefusalKind); impl AppRefusal { pub fn unavailable() -> Self; pub fn closing() -> Self; pub fn no_offer() -> Self; }
pub enum AfterTurn { NewConversation, CloseOwn { decision: String }, Quit, Install }
impl SurfaceHub { pub fn after_turn(&self, a: AfterTurn); pub fn take_after_turn(&self) -> Vec<AfterTurn>; pub fn drop_after_turn(&self); }
```
- **One drain point.** `tab_agent.rs` takes the list once, at the end of the turn, after the
  receipt. `NewConversation` goes into `Pending` (agent/src/turn.rs:22). The others go to the
  stack's after-turn runner, a `std::thread`.
- **Duplicates.** At most one `Quit` or `Install` per turn: the first one recorded wins, and later
  ones are refused at tool time. `CloseOwn` and `NewConversation` are kept once each.
- **Stop.** The stop word, `Cancel` and a failed turn call `drop_after_turn`, so nothing queued
  runs.
- **Checked again.** Right before it runs, `Quit` checks for unsaved work again, and `CloseOwn`
  checks its dirty revision again. Either one is dropped, with a localized notice in the thread,
  if the state changed during the turn.

**Tools:**
- `update {action: check|later|install}`:
  - `check` and `later` call the port; `later` needs a current offer.
  - `install` asks `ConfirmVerb::Install` (its message says the app restarts), then records
    `AfterTurn::Install`, which runs on a thread once the turn ends.
  - Serve and debug builds answer `Unavailable`.
- `pack_update {pack, action: use_mine|make_current}`: the panel's `PacksEvent` through
  `Session::packs_action`, then `prune` (mcp rule t-1133). It is refused when the panel offers no
  such update. Family: Packs.

- [ ] RED (`host_tools_tests.rs`, fake `AppPort`):
  - `update_check_returns_state`
  - `update_later_without_offer_refused_localized`
  - `update_install_asks_then_runs_after_turn`
  - `update_install_refused_does_nothing`
  - `update_on_serve_unavailable_localized`
  - `pack_update_use_mine_applies_and_prunes`
  - `pack_update_without_offer_refused`
- [ ] RED (stack):
  - `after_turn_quit_runs_after_reply_without_deadlock`
  - `after_turn_dropped_on_stop`
  - `after_turn_quit_dropped_when_tab_became_dirty`
  - `after_turn_second_install_refused`
  - `after_turn_runner_panic_is_logged`
- [ ] RED (src-tauri): `app_port_refuses_while_closing`.
- [ ] GREEN, REFACTOR; families (Configure: update; Packs: pack_update).

### Task 10: AgentWorkspace port and the import tool (t-999)

**Files:**
- Extend: `crates/oku-slides-session/src/agent/workspace_port.rs` (from Task 3) with the methods
  below, and add `WorkspaceRefusal`, an opaque Snafu error with a backtrace in every variant and
  a Fluent key (`Unavailable`, `UnknownTab`, `IndexOutOfRange`, `Closing`). `Sessions` stays
  unchanged; its 12 impls are untouched. The plumbing through `Stack::start`, `TabPorts` and
  `Tools::new` is already in place from Task 3.
- Modify: `workspace_agent.rs` (`impl AgentWorkspace for Mutex<Workspace>`, for serve),
  `src-tauri/src/state.rs` (`AppSessions` syncs windows after each structural change, on the main
  thread).
- **Lock order.** A tool reserves the turn's tab move (`reserve_tab`) before it takes the
  workspace lock, and it never calls the hub while holding that lock.
- Modify: `crates/oku-slides-mcp/src/import_tools.rs` (the `import` tool next to `import_report`
  and `import_diff`).
- Create: the `TunePort` trait in `crates/oku-slides-mcp/src/tune_port.rs`, with a
  `TuneRefusal` Snafu error. Its impl is `crates/oku-slides-agent/src/tune_port.rs`, holding a
  weak `Stack` (the `StackDrafts` pattern), so `tune` works in serve too.

**Interfaces:**
```rust
/// Implement this trait to let a tab's agent act on its window's tabs and windows.
pub trait AgentWorkspace: Send + Sync + std::fmt::Debug {
  fn focused_tab(&self) -> Option<FocusedTab>; // from Task 3
  fn tabs(&self, tab: &str) -> Result<Vec<TabListing>, WorkspaceRefusal>;
  fn activate(&self, tab: &str, target: &str, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen, WorkspaceRefusal>;
  fn reorder(&self, tab: &str, target: &str, index: usize) -> Result<(), WorkspaceRefusal>; // index checked ≤ len
  fn move_tab(&self, tab: &str, target: &str, window: Option<&str>, index: Option<usize>) -> Result<(), WorkspaceRefusal>;
  fn new_window(&self, tab: &str) -> Result<(), WorkspaceRefusal>;
  fn detach_agent(&self, attach: bool) -> Result<(), WorkspaceRefusal>;
  fn open_help(&self, tab: &str, slide: Option<&str>, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen, WorkspaceRefusal>;
  fn import_tab(&self, tab: &str, path: &Path, target: ImportTarget, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen, WorkspaceRefusal>;
  /// Dirty revision of each tab, for close decisions (Task 11).
  fn dirty_revisions(&self, tabs: &[String]) -> Result<Vec<(String, Option<u64>)>, WorkspaceRefusal>;
}
pub struct TabListing { pub tab: String, pub title: String, pub active: bool, pub dirty: bool, pub kind: TabKind }
pub enum TabKind { Deck, Help, Pack, Theme }
// mcp: pub trait TunePort: Send + Sync + Debug { fn tune(&self, tab: &str) -> Result<(), TuneRefusal>; }
// import tool: { path, mode: "hifi"|"retrofit", pack?: [id|name] (→ ImportTarget::Retrofit.layouts), theme?: id|name, tune?: bool }
```

**Behaviour:**
- `NoWorkspace` refuses everything with a localized `Unavailable`; the per-tab test commons use
  it.
- The arg is named `pack` as in the ticket and maps to the retrofit `layouts`; the spec amendment
  records it.
- Paths are checked like `open`: absolute, normalized, no `..`, `.pptx`.
- The import asks `ConfirmVerb::Import` with the full normalized path, takes the turn's one tab
  move, and runs `import_tab` in `spawn_blocking`.
- `tune` goes through `TunePort` (a Tune turn starts only from the host, agent rule t-692).

- [ ] RED:
  - `import_hifi_opens_a_new_tab`
  - `import_relative_path_refused`
  - `import_second_tab_move_in_turn_refused`
  - `import_tune_starts_tune_through_port`
  - `import_retrofit_unknown_pack_refused_localized`
  - `import_retrofit_with_pack_uses_its_layouts`
  - `import_with_theme_applies_it`
  - `import_unknown_theme_refused_localized`
  - `no_workspace_refuses_localized`
  - `reserve_tab_before_workspace_lock` (a workspace fake that asserts no hub call under its lock)
- [ ] GREEN. Then `370-agent-app-tools.cjs`, import step with `fixtures/import/*.pptx`: confirm
  by voice, and the new tab shows.

### Task 11: tabs, tab and window tools (t-1000)

**Files:**
- Create: `crates/oku-slides-mcp/src/workspace_tools.rs` (+ tests).
- Modify: `hub.rs` (`McpEvent::Close { target: CloseTarget, decision: String }`, plus
  `take_close_decision`). In `record_and_send` (hub.rs:226), the `Close` arm is sent live only:
  it is never added to `state.open` and never replayed.
- **Redeeming.** `Stack::handle` returns no value (stack.rs:242) and `agentCall` is `host<void>`
  (api.ts:729), so the decision gets its own host command:
  - Tauri: `take_close_decision(window, id) -> Option<CloseDecision>`;
  - serve: a route through `guard::check`;
  - TS: `takeCloseDecision` in `api.ts`.
- **The dirty revision.**
  - `Session` gains a `dirty_revision() -> Option<u64>` counter that grows at each edit and is
    `None` while clean.
  - `TabInfo` carries it to the page, and its TS mirror gains `dirtyRevision`.
  - The hub stores one revision per tab of the target. A window close covers several tabs.
- Create: `ui/src/closeDecision.ts`, holding the redemption and the revision check, called from
  `closeFlow.ts`. `closeFlow.ts` is at 474 lines.
- Modify: `ui/src/agent/AgentPanel.tsx`, `ui/src/agent/events.ts` (zod, a kind→schema map), the
  `McpEvent` union in `api.ts`.
- Tests: `ui/src/closeFlow.test.ts`, with `ui/src/test/closeFlowFake.ts` extended by a fake
  decision redeemer.

**Interfaces:**
```rust
pub enum CloseTarget { Tab { tab: String }, Window { window: String } }
/// A hub decision a page redeems once to answer its own discard question.
pub struct CloseDecision { pub discard: bool, pub revisions: Vec<(String, Option<u64>)> }
// stored under a UUID v4 id with its target, its window and an expiry (30 s); taken once.
// A revision of None means "only while still clean".
// tabs {} -> { tabs: [TabListing] }        (title through bounds::label, ≤ 80 chars, framed as data)
// tab { action: "activate"|"close"|"reorder"|"move", tab, index?, window?: "new"|"w-N" }
// window { action: "new"|"close"|"detach_agent"|"attach_agent" }
```

**Behaviour:**
- **activate, move, reorder** go through `reserve_tab` and the activation rule (t-1191). Out of
  range or `usize::MAX` indices are refused.
- **Closing a tab.**
  - Closing another tab asks `ConfirmVerb::Close`; a tab with unsaved work asks
    `ConfirmVerb::Discard`, naming it.
  - The hub stores a `CloseDecision` under a one-time id and emits `Close`.
  - The page redeems the id, runs `closeTabFlow` with its saves, and uses the decision only to
    answer the discard question, only if the live dirty revision matches. Otherwise it asks as
    usual.
  - The agent's own tab closes after the turn ends (`AfterTurn::CloseOwn`), with the dirty
    revision checked again.
- **Windows.** `window close` uses the same path with `closeWindowFlow`. `new`, `detach_agent`
  and `attach_agent` go through `AgentWorkspace`; serve answers `Unavailable` for detach and
  attach.

- [ ] RED (mcp):
  - `tabs_lists_only_requester_window`
  - `tabs_titles_are_labelled_and_clipped`
  - `tab_close_other_asks_then_emits_close`
  - `tab_close_dirty_asks_discard_verb`
  - `tab_close_own_waits_for_turn_end`
  - `tab_activate_counts_as_turn_tab_move`
  - `tab_reorder_moves_tab_to_index`
  - `tab_reorder_index_out_of_range_refused` (rstest len+1, usize::MAX)
  - `tab_move_to_new_window`
  - `window_new_through_port`
  - `window_close_emits_close_event`
  - `window_attach_agent_through_port`
  - `window_detach_on_serve_unavailable_localized`
  - `close_decision_redeemed_once`
  - `close_decision_from_other_window_refused`
  - `close_decision_expired_refused`
  - `close_decision_window_stores_revision_per_tab`
  - `close_event_is_not_replayed`
  - session: `dirty_revision_grows_on_edit_and_clears_on_save`
- [ ] RED (TS, `closeFlow.test.ts`):
  - `closeTabFlow with redeemed decision answers discard and still saves`
  - `closeTabFlow forged close without decision shows the dialog`
  - `closeTabFlow decision with stale dirty revision asks again`
  - `closeWindowFlow decision with one stale tab asks for that tab`
- [ ] RED (zod): `close event with invalid ids is dropped`.
- [ ] GREEN. `370-agent-app-tools.cjs`: list tabs, activate the second one, close it after
  "oui, fermer".
- [ ] Rules:
  - mcp AGENTS.md: the scope invariant and the t-1191 title rule are amended (Q8, Q9).
  - ui AGENTS.md: "close a tab only through closeTabFlow …, or after the agent's close event,
    whose decision the page redeems from the host."

### Task 12: help and new_conversation tools (t-1000)

**Files:** `workspace_tools.rs`; `WorkspaceRequest::OpenHelp { window, #[serde(default)] slide }`
(mirrored in `workspaceApi.ts`); `workspace_tabs.rs` `open_help` and the `Opening::Help` path.
The help slide is set on both reuse paths: the early `Step::Done` return (:398) and the build-path
dedupe branch (:450). Also `crates/oku-slides-agent/src/turn.rs` (`Pending`, :22) and
`tab_agent.rs` (:321, which consumes it). Task 9's single drain point puts `NewConversation` into
`Pending`; nothing sends `Command::NewConversation`. `quit {}` is in this task's tool file and uses
Task 9's `AfterTurn::Quit`.

**Tools:**
- `help { query? }` (the ticket's `help {open, query?}`: opening is implied; see the spec
  amendments).
  - It opens or reuses the window's help tab, as the turn's one tab move.
  - With `query`, the host runs `Request::Search` on the help session and returns at most 5 hits
    `{slide, title}`; the tab opens at the first hit.
  - Paging is the help tab's own agent calling `focus`.
- `new_conversation {}` sets `AfterTurn::NewConversation`.
- `quit {}`:
  - it is refused while any tab holds unsaved work, which it reports by count, never by title;
  - otherwise it asks `ConfirmVerb::Quit`, then sets `AfterTurn::Quit`;
  - serve answers `Unavailable`.

- [ ] RED:
  - `help_query_opens_at_first_hit`
  - `help_unknown_query_returns_no_hits_and_opens_first_slide`
  - `help_reused_tab_on_dedupe_path_sets_slide`
  - `focus_in_help_tab_pages_it`
  - `quit_with_dirty_tab_refused_localized`
  - `quit_asks_then_runs_after_turn`
- [ ] RED (regression, G16): `crates/oku-slides-agent/tests/regressions/issue_1000_new_conversation_keeps_turn.rs`,
  declared in that folder's `main.rs`. The turn's receipt still shows, then the thread is new.
- [ ] GREEN. `370-agent-app-tools.cjs` help step: "Oku, ouvre l'aide sur la dictée" → the help
  tab shows the dictation slide (title read from the page).

### Task 13: Layout through set_interface (t-1001)

**Files:** `crates/oku-slides-mcp/src/app_tools.rs` (`InterfaceChange` and its `is_empty`;
`tests/app_tools.rs:472`); `ui/src/interfaceChange.ts`; the bounds from `ui/src/columns.ts`
(`PILE_MIN`, `PILE_MAX`, `NOTES_MIN`; `columnBounds`) and `ui/src/codeSplit.ts` (`CODE_MIN`;
`codeBounds`); a shared fixture `fixtures/ui/layout-bounds.json`, read by Rust and TS tests; the
`api.ts` `InterfaceChange` mirror.

**Interface:** `InterfaceChange` gains `folded?: { slides?: bool, layouts?: bool }`, `pile?: u32`,
`notes?: u32`, `code?: bool` and `code_width?: u32`. They are integers in pixels, so `Eq` holds,
and they keep the `UiState` names.
- `UiState.folded` is a `Vec<PileSection>` (views.rs:211). `folded.slides = true` adds
  `PileSection::Slides` if it is missing, and `false` removes it; likewise for `layouts`. An
  absent key leaves its section as it is.
- Rust refuses values outside the static bounds in the fixture.
- The page clamps to the live, window-dependent bounds (`columnBounds`, `codeBounds`) and saves
  through the normal `Ui` request, ordered by seq.

- [ ] RED:
  - Rust: `set_interface_layout_only_is_not_empty`,
    `set_interface_pile_out_of_static_bounds_refused`,
    `set_interface_folded_partial_keeps_other`, `layout_bounds_fixture_matches_rust`.
  - TS, in `interfaceChange.test.ts`: `applyInterface folds slides and keeps layouts`,
    `applyInterface pile beyond live bound is clamped`. In `columns.test.ts`: `layout bounds
    fixture matches columns`. In `codeSplit.test.ts`: `layout bounds fixture matches codeSplit`.
- [ ] GREEN. `380-agent-layout.cjs`: fold the slides, set the code pane on; the state survives a
  reload.

### Task 14: OS voice engines in the STT crate (t-1002)

**Files:**
- Create: `crates/oku-slides-stt/src/speak/mod.rs` (`SpeechEngine` trait, `SpeechStatus`),
  `speak/apple.rs` (`cfg(target_os = "macos")`, safe calls only).
- Create: `speak/apple_ffi.rs` (`cfg(any(test, target_os = "macos"))`). It holds the externs, the
  trampolines with `catch_unwind`, and every unsafe block, each with a `// SAFETY:` comment.
  - **Ownership.** Only Rust frees the callback context. The Swift side never frees it.
  - **One DONE.** DONE takes the context from a `Mutex<Option<Ctx>>` and checks its generation.
    A second call, or a call for an older generation, does nothing.
  - **MIRI.** The trampoline tests run under `cargo +nightly miri test -p oku-slides-stt
    speak::apple_ffi`, which Task 16 lists.
- Create: `speak/windows.rs` (`cfg(windows)`).
  - The dependency is declared `windows = { version = "0.62", default-features = false, features
    = [...] }` in `[workspace.dependencies]`. The features are `Media_SpeechSynthesis`,
    `Media_Playback`, `Media_Core`, `Storage_Streams`, `Foundation` and `Foundation_Collections`.
    It is pulled into the crate under `[target.'cfg(windows)'.dependencies]`.
  - It speaks on its own thread with `SynthesizeTextToStreamAsync`, never SSML.
  - The `MediaEnded` and `MediaFailed` handlers share one take-once DONE, and both event tokens
    are revoked when the speech ends or is cut.
- Create: `speak/linux.rs` (`cfg(any(test, target_os = "linux"))`). It runs `spd-say` with:
  - an injected program path and the client name `oku-slides`;
  - `-w`, so the child lives as long as the speech; never `-C`, which would cancel other
    clients' speech;
  - `.arg`s with `--` before the text, and stdin null.
  It checks the voice against `-L`. It keeps the `Child`, and on cut kills it and reaps it on its
  own thread.
- Create: `speak/fake.rs` (feature `e2e-fakes`, through `Storage`). Tests go in sibling
  `*_tests.rs` files.
- Create: `swift/Sources/OkuSpeech/OkuVoice.swift`: `oku_voice_list`, `oku_voice_speak`,
  `oku_voice_stop`, `oku_voice_release`. It uses `AVSpeechUtterance(string:)`, never
  `ssmlRepresentation`. The handle keeps the synthesizer, and the delegate maps finish and cancel
  to one DONE, followed by a single `free`.
- Modify: `error.rs` (`SttErrorKind::VoiceUnavailable { reason }`, `VoiceFailed`, localized).
- Update the README crate map and the stt ARCHITECTURE.md: "speech: dictation and OS voice".

**Interfaces:**
```rust
/// Implement this trait to give Oku a voice on one OS.
pub trait SpeechEngine: Send + Sync + std::fmt::Debug {
  fn status(&self, lang: &str) -> SpeechStatus;   // blocking: call it off async workers
  /// Starts speaking under an id the caller allocated; on Err, `done` is dropped uncalled.
  fn speak(&self, id: SpeechId, text: &str, voice: Option<&str>, rate: SpeechRate, done: SpeechDone) -> Result<(), SttError>;
  fn stop(&self, id: SpeechId) -> Result<(), SttError>;
}
// The speaker allocates SpeechId from one AtomicU64 and records it as current *before* speak, so
// a done that fires before speak returns is still matched.
pub enum SpeechStatus { Ready { voices: Vec<VoiceInfo> }, Unavailable { reason: UnavailableReason } }
pub struct VoiceInfo { pub id: String, pub name: String, pub lang: String }
pub enum UnavailableReason { NoEngine, NoVoiceForLanguage, DaemonSilent }
/// Speaking rate, finite, within 0.5..=2.0 of the OS default.
pub struct SpeechRate(f32); impl SpeechRate { pub fn new(r: f32) -> Result<Self, SttError>; }
/// Called once, with the speech's generation id; a late call for an older id is ignored.
pub struct SpeechDone(Box<dyn FnOnce(SpeechId) + Send>); // manual Debug impl
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct SpeechId(u64);
```
- When there is no engine, no voice for the language, or `spd-say` is missing or its daemon is
  silent, the engine reports `Unavailable` with a localized reason. Nothing falls back silently to
  another language.
- AppImage and Flatpak builds that cannot see a host `spd-say` are therefore `Unavailable`, which
  is the right answer.

- [ ] RED:
  - `fake_engine_logs_text_and_calls_done_once`
  - `apple_callbacks_called_twice_free_once` (calls the extern trampolines directly, any host)
  - `apple_done_for_older_generation_is_ignored`
  - `speak_error_drops_done_uncalled`
  - `done_before_speak_returns_is_matched`
  - `linux_missing_program_is_unavailable_localized`
  - `linux_spd_say_argv_has_wait_double_dash_and_client_name` (fake script records argv; no `-C`)
  - `linux_text_starting_with_dash_is_spoken_not_parsed`
  - `linux_stop_kills_own_child_only`
  - `speech_rate_nan_or_out_of_range_refused`
  - `voice_for_other_language_is_unavailable`
- [ ] RED: macOS-only `apple_lists_at_least_one_voice` (`cfg(target_os = "macos")`).
- [ ] RED: `speak/windows.rs` compiles in the `voice-check.yml` job (Q11). The job is described
  in Task 16.
- [ ] GREEN, then check by hand on this Mac. Windows and Linux get a hands-on check under the
  ticket "Check the OS voices on Windows and Linux", created at step 2.
- [ ] Rules:
  - stt AGENTS.md: "Speak only through SpeechEngine; never bundle a voice, call a cloud service
    or pass SSML; report Unavailable, never fall back to another language; free each speech
    callback once."
  - Root AGENTS.md gotcha: the `windows` features list.

### Task 15: Read-aloud: speaker, reader, settings and box (t-1002)

**Files:**
- Create: `crates/oku-slides-stt/src/speaker.rs` (+ tests). There is one per process (the spec's
  `say` becomes this `Speaker`, recorded in the amendments).
  - It speaks on its own thread and holds Task 1's `SuspendGuard` while a speech plays.
  - A watchdog releases the guard after `chars ÷ (15 × rate) + 3 s` if DONE never comes. A higher
    rate means a shorter speech, so the length is divided by the rate, with checked math.
  - A new speech cuts the current one. No lock is held across `engine.speak`.
  - After DONE it waits a 500 ms guard before resuming listening. An utterance that started
    before the end of speech plus the guard is dropped.
  - It takes an injected clock, so the lifecycle tests never sleep.
- Modify: `crates/oku-slides-mcp/src/hub.rs`.
  - `McpEvent::Surface` and `CloseSurface` get a host-only `#[serde(skip)] spoken:
    Option<SpokenText>`. The name avoids the existing TS `Spoken` type. `SpokenText` derives
    what `McpEvent` derives (`Clone`, `PartialEq`, `Debug`).
  - The new constructors `McpEvent::surface(s)` and `McpEvent::close(id)` replace the about 30
    literal constructions, and patterns use `..`. Sites: `tab_entry.rs:295`, `hub/asks.rs:107`,
    `packs.rs:54,84`, `hub_turns.rs:221`, `actions.rs:114,213`, the hub tests, and the
    local-agent `tests/common:157`.
  - `spoken` is built where `forms` builds the surface: a receipt's title and at most 4 facts, a
    question and its options in order, a user-facing tool error, and for a confirmation the
    template and the verb only. The confirmation's message is never read, since it may name a tab
    or hold a path (Q8).
  - Spoken text comes from host templates, `bounds::clip` (400 chars) and `bounds::label`, with
    FSI and PDI stripped. Permission and guard readouts never use agent text.
  - When a question's speech ends, the reader calls `SurfaceHub::spoken_until(id, ended)`, which
    moves its arming start (Task 5).
- Create: `crates/oku-slides-agent/src/reader.rs`.
  - It subscribes to the hub's event stream through a channel; nothing is called from `emit`.
  - It speaks only for the focused tab (`AgentWorkspace::focused_tab`, read with no hub lock
    held). The workspace is the one passed to `Stack::start` in Task 3.
  - It is muted while that tab presents, unless `in_presentation` is on and the turn began with a
    wake phrase. The voice router marks the turn with `TurnOrigin::Wake`.
  - It cuts speech at a new question or a new turn.
- Modify: `crates/oku-slides-session/src/voice_view.rs`:
  - `SpeechPatch { read_aloud, voice, rate, in_presentation }`;
  - `SpeechView` with its status and reason;
  - `read_aloud: Option<bool>` in the stt `Config`, where `None` means never set. It becomes
    `Some(true)` when hands-free is first turned on.
- Modify: `voice_panel.rs` (a "Read aloud" section, disabled with its reason when unavailable),
  `crates/oku-slides-agent/src/voice_port.rs` (`VoiceEvent::Speech(SpeechPatch)`),
  `crates/oku-slides-mcp/src/voice_tools.rs` (`set_speech`, through `VoicePort`), and the
  `settings` reply (speech status and reason).
- Modify: `crates/oku-slides-cli/src/serve.rs` and `src-tauri/src/main.rs`: wire the speaker and
  the reader.
- Fluent `voice.ftl`:
  - spoken templates: `voice-say-question`, `voice-say-option`, `voice-say-confirm`, and the
    turn failures `voice-say-agent-*` (see the deviations below);
  - box labels: `voice-read-aloud`, `voice-read-aloud-voice`, `voice-read-aloud-rate`,
    `voice-read-aloud-in-presentation`;
  - unavailable reasons: `voice-unavailable-no-engine`, `voice-unavailable-no-voice`,
    `voice-unavailable-daemon`.
- Create: `ui/e2e/390-read-aloud.cjs`.

- [ ] RED (`reader_tests.rs`, fake engine, `done` driven by hand):
  - `receipt_reads_title_and_four_facts`
  - `question_reads_options_in_order` (key + fr)
  - `confirmation_reads_verb`
  - `tool_error_is_read`
  - `progress_and_history_are_silent`
  - `presenting_tab_is_muted`
  - `presenting_wake_turn_reads_with_in_presentation`
  - `new_turn_cuts_speech`
  - `other_tab_is_not_read`
  - `agent_text_in_permission_is_never_read`
  - `confirmation_message_is_never_read` (a Discard naming a tab, an Import holding a path)
  - `agent_why_and_slide_content_are_never_read`
  - `new_question_cuts_speech`
  - `question_speech_end_moves_arming`
  - `reader_panic_is_logged_and_thread_ends`
- [ ] RED (`speaker_tests.rs`, injected clock):
  - `listening_suspended_while_speaking_then_resumed`
  - `utterance_starting_before_speech_end_plus_guard_dropped`
  - `watchdog_resumes_when_done_never_comes`
  - `watchdog_shorter_at_higher_rate`
  - `inline_done_does_not_deadlock`
  - `cut_then_late_done_of_old_speech_is_ignored`
  - `speech_of_confirmation_never_confirms_it` (fake engine plus fake source)
- [ ] RED (other):
  - voice_view: `speech_unavailable_disables_switch_with_reason`,
    `read_aloud_on_at_first_hands_free_only`
  - set_speech: `set_speech_rate_out_of_range_refused`
  - settings: `settings_reports_speech_status`
- [ ] GREEN. `390-read-aloud.cjs` (fake voice): a scripted receipt is in the speech log. While
  presenting, nothing is logged before the sentinel speech that follows the presentation.
- **Deviations (accepted in review r0, R44).**
  - The box labels live in `session.ftl` beside `voice-hands-free`; the `voice-error-*` keys of
    `set_speech` live in `agent.ftl`.
  - The surface constructor is `McpEvent::surface_aloud`; `CloseSurface` says nothing, so it has no
    `spoken` field and there is no `McpEvent::close`.
  - Fallback receipts (the agent's free narration) are never read aloud.
  - An agent `ask_confirm` reads its message as a question, then `voice-say-confirm-yes`, or
    `voice-say-confirm` with the danger label when `danger` is set.
  - A turn failure reads a host sentence chosen by the error's variant, with no argument
    (`voice-say-agent-refused`, `-stopped`, `-no-start`, `-no-tools`); `voice-say-error` is gone.
  - The e2e is `ui/e2e/410-read-aloud.cjs` (R43).

### Task 16: Docs, instructions, review round, CI, squash (all)

- [ ] Docs:
  - `crates/oku-slides-agent/src/instructions/agent.md` paragraph 2: one sentence per new tool,
    English only.
  - `crates/oku-slides-local-agent/src/instructions/local.md`: the families changed in Tasks 8
    to 12.
  - The `ARCHITECTURE.md` files stay within 50 lines.
  - `docs/aide`: a hands-free page in every shipped help locale (docs AGENTS.md).
  - Spec: status "implemented", the inventory rows of lots 2 to 8 marked yes.
- [ ] Review round: `common:review` on `main..HEAD` with the Rust and TS lenses. The security
  lenses re-check Q2, Q3, Q7, Q10, Q16, the `spd-say` argv and the callback lifetimes. Findings
  are fixed, or ticketed with their t-N.
- [ ] `voice-check.yml`, only if you approve it (Q11). It runs on pull requests and on `main`,
  with actions pinned by SHA.
  - `windows-latest`: CMake and NASM (whisper and the audio crates build C), the `+crt-static`
    target feature as in `release.yml`, then `cargo check -p oku-slides-stt`.
  - `ubuntu-latest`: `libasound2-dev` and `speech-dispatcher`, then `cargo check -p
    oku-slides-stt` and `cargo test -p oku-slides-stt speak::`.
- [ ] MIRI on the Apple trampolines (Task 14), run by hand on this Mac and noted in the review.
- [ ] Release guard (Q16): `just check` fails if `e2e-fakes` shows up in the release feature
  tree.
- [ ] Tickets to create at step 2, each cited by t-N in the commit body:
  - "Review the voice lexicons per locale", t-1489 (Q5);
  - "Check the OS voices on Windows and Linux", t-1490 (Task 14);
  - "Barge-in: speak over Oku to cut its speech", t-1491 (the spec's open question).
- [ ] Gate:
  - `git fetch`, merge `main`, and run `bun install` if needed.
  - Run `just ci` in the background (nohup, logs under
    `.superpowers/sdd/2026-11-01-lot-m-hands-free/`), and run `just e2e` with the feature.
- [ ] Ship:
  - Squash onto `main` per Q13: `feat(stt,i18n,session,mcp,agent,ui,app)!: hands-free voice
    control, lots 2 to 8 (t-996, t-997, t-998, t-999, t-1000, t-1001, t-1002)`.
  - It is breaking (`!`) because the agent-facing `present` schema and the confirmation buttons
    change.
  - Fetch and check `main` again right before, then push.
  - `oku cli task done` for each ticket and sub-task.

## Self-review

**Spec coverage:**

| Lot | Tasks | Notes |
|---|---|---|---|
| 2 | 1, 3, 4 | |
| 3 | 2, 5 | permissions by kind, Q3 |
| 4 | 6, 7 | |
| 5 | 8, 9, 10 | |
| 6 | 11, 12 | `select_agent` and downloads exist |
| 7 | 13, plus 11 | 11 covers reorder and move |
| 8 | 14, 15 | voice of the UI locale, Q12 |

The spec's own open questions (barge-in, a wake phrase outside presentations, the bootstrap
gesture) stay open.

**Additions beyond the spec, each a decision:**
- `present first|last`
- verb buttons (Q4)
- typed replies keep the question open (Q2)
- the Swift `errDenied` fix
- tab reorder and move (Q14)
- background gating (Q6), with the window focus taken from the host only
- hands-free can be turned on only by the user, and every launch starts paused (Q7)
- `stt.min_phrase_secs`, a new noise floor for utterances (Task 1)
- `quit` refused while a tab holds unsaved work, and checked again after the turn (Task 12)
- the spec's `say` becomes the stt `Speaker` (Task 15)

**Round 2 of the review** (15 reviewers, draft 2) is folded in:
- page ids on reports;
- host-only focus;
- the `AgentWorkspace` plumbing moved into Task 3;
- after-turn drain, stop and re-check rules;
- close decisions bound, expiring and per tab, redeemed through their own host command;
- caller-allocated speech ids;
- a `Weak` router;
- absolute Flush indices;
- token-range wake slices;
- the feature chain and its release guard;
- the consumer lists for every changed type;
- test file names.

**Risks named before coding:**
- undo inside an open turn group (Task 8)
- the Apple path: unit-tested on the fake recognizer, then checked by hand
- Windows and Linux voices: CI build only, plus a hands-on ticket
- 18 machine-translated lexicons (Q5)
- the size of one squash (Q13)

## Execution rulings

Decisions taken while executing this plan (ledger of the SDD run). Where a task above
shows an earlier shape, the ruling wins.

- **R1:** `ConfirmVerb` lives in mcp `lexicon.rs` (Task 2); Task 5 imports it, does not redefine — avoids two definitions — cost if wrong: a move between modules.
- **R2:** Task 3 adds the Rust `AgentCommand::Context/Presenting` handling and its Tauri path; Task 4 adds the serve route (with guard::check) and the TS side — the page side lives in Task 4 — cost if wrong: one route moved between tasks.
- **R3:** Task 5 stores `armed_from` (= `shown_at`) on each pending ask; Task 15 adds `SurfaceHub::spoken_until(id, Instant)` that moves it, with the `question_speech_end_moves_arming` test — no dead code between tasks — cost if wrong: Task 5's `armed_from_end_of_speech` test moves to Task 15.
- **R4:** Task 6 writes its Escape e2e step in `360-present-voice.cjs` as the file's first step; Task 7 extends the file — keeps the regression with its fix — cost if wrong: a reordered e2e file.
- **R5:** SuspendGuard epoch lives in Stt Inner, not in the guard — guard stays a counter handle, same semantics — cost if wrong: field moved.
- **R6:** a download started while listening resumes listening whatever its result (error still surfaced); resume-on-success only for downloads started from Idle — the brief says capture starts again, and no re-download loop is possible since fetch skips installed models — cost if wrong: one branch in finish_download.
- **R7:** `yes_no(&self, said, verb: Option<ConfirmVerb>)` instead of the brief's `Option<&[String]>` — the brief's two signatures disagree, and taking the enum rules out agent text structurally (spec: a confirmation reads its template and verb only) — cost if wrong: one parameter type in Task 5 call sites.
- **R8:** "annule"/"cancel" stay no-words, not stop-words — stop is routed before answers, so a stop form would turn a refusal into a cancel of the turn — cost if wrong: one form moved between messages.
- **R9:** unspaced scripts (ja, zh-Hans, zh-Hant, th) get longest-first segmentation of a token into lexicon forms, keeping the original byte offsets; a token not fully covered stays whole — spec requires all 20 locales — cost if wrong: segmentation code removed.
- **R10:** Task 3 builds route_voice with the whole table (Answer and Nav included). In the stack, the Answer arm goes through the hub's existing typed-answer path if one exists, else is dropped with a debug log naming the seq only; the Nav arm is dropped the same way. Task 5 replaces the Answer arm (answer_voice) and Task 7 the Nav arm (PresentCommand) — keeps the router table tested once, in one place — cost if wrong: two match arms rewritten in Tasks 5 and 7.
- **R11:** voice routing order is stop, wake, focus gate, shortcut (unless presenting), answer (open question or drawer), presentation (nav, else drop), prompt — follows every cell of the plan's table and keeps Task 5's "answer before any prompt" and Task 7's "answer before nav"; amends Task 5's "answer_voice first" (stop and wake come before it) and Task 7's order (wake before answer) — cost if wrong: a reorder of route_voice and its table.
- **R12:** the workspace port gives the listening tab with a focused flag even in the background (last focused workspace window's active tab, serve: last page-focus tab); voice_tab is dropped — cost if wrong: one port method shape.
- **R13:** answer(id, text) keeps the question id — the existing invariant names the target and a typed reply must never guess it — cost if wrong: one parameter dropped.
- **R14:** a voice "no" refuses whatever the focus; only "yes" needs FocusSource::Host && focused — refusing grants nothing, and the router's focus gate already drops background utterances — cost if wrong: one focus check added.
- **R15:** 350-voice-confirm ends with a click — serve focus is PageReported, which never confirms (Q6); host-focus voice confirmation is covered by the agent test — cost if wrong: an e2e step changed.
- **R16:** plain confirms stay strict (yes needs FocusSource::Host && focused) — matches Task 3's carry-over and grants nothing to a page-reported focus — cost if wrong: one focus check relaxed.
- **R17:** fix the sheet target now (staged slide from PageReports passed to answer_voice) rather than park — a wrong answer steers the agent and the rule was voided for this case — cost if wrong: one parameter and a PendingAsk field.
- **R18:** dictationActive.ts is a plain counter without useSyncExternalStore — nothing subscribes, and a hook would be dead code — cost if wrong: one hook added when a subscriber appears.
- **R19:** while presenting, an utterance an open question or drawer does not take is nav or dropped, never a prompt — follows R11 (presentation: nav, else drop) — cost if wrong: one fall-through arm.
- **R20:** "quitter" (fr) and "quit" (en) join the exit forms — the plan's e2e says "quitter" — cost if wrong: two forms removed.
- **R21:** accept the plan deviation — Session::replay reopens the turn's undo group after an undo/redo that closed it (replica end_group, not Loro) — the spec needs one undo step per turn — cost if wrong: the reopen removed and the turn split.
- **R22:** keep the extras — undoable_turn line in the agent context (undo_turn is unusable otherwise), two Fluent keys, 370-undo-tools e2e — cost if wrong: one context line and a spec removed.
- **R23:** AfterTurn::CloseOwn moves to Task 11 — it needs Task 11's dirty revision and close path; no dead variant meanwhile — cost if wrong: one variant added earlier.
- **R24:** SurfaceHub::after_turn(turn, action) -> bool (must_use), refused outside the open turn and for a second quit/install — the turn number pins the action to its turn — cost if wrong: one parameter dropped.
- **R25:** AppRefusal::failed() (agent-error-app-failed) is added — a failed check or download is none of unavailable/closing/no_offer — cost if wrong: one variant folded.
- **R26:** update install checks again first, which forgets a "Later" like the menu's check — the offer must be fresh before installing — cost if wrong: a kept put-off version.
- **R27:** tests/common re-exports keep #[allow(unused_imports, reason)] — the existing tests/common convention, and #[expect] goes unfulfilled in the binaries that use them — cost if wrong: an attribute swap.
- **R28:** Task 10's e2e is 390-agent-app-tools.cjs (370/380 taken by Tasks 8, 9); its voice confirm under serve ends with a click (R15) — cost if wrong: a renamed spec.
- **R29:** open_help moves to Task 12 and dirty_revisions to Task 11, with their plumbing; TabKind is the existing DocumentKind — no unused trait methods meanwhile — cost if wrong: two methods added earlier.
- **R30:** WorkspaceRefusal gains Session (import/build failures) — none of the four fits — cost if wrong: one variant folded.
- **R31:** Task 10's six workspace change methods (tabs, activate, reorder, move_tab, new_window, detach_agent) stay; Task 11 consumes them and re-adds none — both hosts implement them and session tests cover them — cost if wrong: they move to Task 11.
- **R32:** window actions go through windows::sync off the main thread, as open_tab/open_pack/open_theme already do (Tauri 2 proxies to the event loop; blocking the main thread is forbidden) — amends the plan's global constraint "run_on_main_thread" — cost if wrong: a main-thread dispatch added.
- **R33:** change methods keep their documented Unavailable default (Nit 8 declined) — fakes stay small and NoWorkspace documents it — cost if wrong: required methods.
- **R34:** a close from a tab that is not shown is refused (ensure_shown), not routed to the shown page — the page subscribes only for the shown tab; a workspace-wide listener is a larger change — cost if wrong: the agent must activate before closing.
- **R35:** the e2e does not cover activating another tab, since headless hands-free keeps listening to the previously shown tab — Rust tests cover activate — cost if wrong: one e2e step added later.
- **R36:** paging stays on DraftPort for now and the port rename (a tab port) is weighed at the final review — it works, is documented and tested on both sides — cost if wrong: one rename across mcp/agent.
- **R37:** Task 13's e2e is 400-agent-layout.cjs (380 is taken by Task 9's update-serve) — cost if wrong: a renamed spec.
- **R38:** no code_width field and no codeSplit fixture test — codeSplit.ts/codeBounds were removed by t-1419 and `code` now picks a pack item's Code or Settings tab — cost if wrong: one field restored.
- **R39:** notes have only a static minimum (300), since columns.ts defines no maximum; pile is refused outside 180–360 — cost if wrong: a max added later.
- **R40:** macOS speech DONE needs the main run loop (AVFoundation delegate on the main queue); the app has one, serve relies on Task 15's watchdog — cost if wrong: serve speech ends late.
- **R41:** Task 15 reuses stt.ftl stt-voice-no-engine/no-voice/daemon via UnavailableReason::localize instead of adding voice-unavailable-* keys — one text per reason — cost if wrong: keys moved.
- **R42:** the macOS router is one-at-a-time — each speak stops the current speech first and the last into the bridge wins; the synthesizer is called outside the router lock — simplest correct semantics for a shared synthesizer — cost if wrong: callers serialise instead.
- **R43:** Task 15's e2e is 410-read-aloud.cjs (390 taken) — cost if wrong: a renamed spec.
- **R44:** deviations accepted — voice_port.rs 594 lines (shrank), box labels in session.ftl, constructor surface_aloud with no McpEvent::close, fallback receipts silent, e2e 340 turns read-aloud off — recorded in the plan amendments at the final squash — cost if wrong: renames.
- **R45:** focus keeps deselect with no slide but never selects (main's t-1502 user decision wins over the hands-free spec's element select) — deselect is the voice Esc the user asks for, not the agent changing the view on its own — if wrong, drop the deselect callback and the no-slide branch; flagged for the user.
- **R46:** local-agent focus stays in Edit only, Configure summary shortened — main's eval-tuned test is the authority — costs a missed route for "next slide" while presenting.
- **R47:** AgentError::spoken reads main's new launch variants as "could not start" — they are launch failures — costs a coarser spoken message.
- **R48:** permissions by voice keep yes = allow once (spec lines 126, 334, Q3) — the spoken template is the bug and says yes/no now — if wrong, permissions go click-only by dropping the YesNo::Yes arm.
- **R49:** deferred to follow-up tickets: DraftPort/TunePort merge + Tools ports struct (architect M1/M2), shortcut matcher vs Lexicon (M3), hand voice dedupe (M4), self-echo filter (DA Medium), action noun in plain confirmations (sec M2), spd-say stdin + SSIP cancel (t-1490) — none is a correctness break in shipped flows — costs a later refactor.
- **R50:** Windows WinRT compile stays unverified until voice-check.yml runs after the push (CI runs only after the push) — recorded in stt ARCHITECTURE.md and t-1490 — costs a possible Windows fix-up commit on main.
- **R51:** dropped invalid MCP events log their kind (no payload) with console.warn — the fix brief wins over global-constraints' "without logging" — costs one console line per drift.
