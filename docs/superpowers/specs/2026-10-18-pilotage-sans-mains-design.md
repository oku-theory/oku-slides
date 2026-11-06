# Hands-free control through the agent — design

Tasks: t-988 (lot 1, done), t-996 to t-1001 (lots 2 to 7), t-1002 (lot 8). The user asked for
read-aloud on 2026-10-18, through the OS speech feature where the platform has one. Date: 2026-10-18.
Status: implemented (lots 1 to 8; the inventory rows show the routes as built). Amended 2026-11-01
against `main` at 88e5af3f (see "Amendments" at the end); plan:
`docs/superpowers/plans/2026-11-01-lot-m-hands-free.md`.

## Problem

The user set a principle on 2026-10-18: the whole app must be usable without touching the
keyboard or the mouse, by talking to the agent, app configuration included. Lot 1 (t-988)
reduces the main menu to Present, Open and Help and adds the MCP tools `move`, `merge`,
`history`, `present`, `set_interface` (scheme, locale, grid, guides) and `edit_templates`
without a set. This spec inventories every remaining UI action, says whether the agent can do
it with an MCP tool, and lists what still blocks a voice-only session.

The inventory was taken on `main` at f8b69ba3 plus the uncommitted lot 1 work tree (tools
listed in `crates/oku-slides-mcp/src/tools.rs`, dispatched in `handler.rs`).

## Goals

- A user can start, steer and end a whole session by voice: talk to the agent, answer its
  questions and confirmations, present, switch and close tabs, configure the app.
- Every UI action has an agent route, or a written reason why it needs none.
- Where the OS has a voice, the user can follow by ear: Oku reads receipts and questions.
- Confirmations keep their purpose: the agent can never confirm its own guarded call.

Non-goals: a speech engine of our own (read-aloud uses the OS voice where the OS offers one,
lot 8), remote control from another device, OS-wide voice control. macOS Voice Control stays a
fallback for the one bootstrap gesture (see "Open questions").

## Inventory

Legend: **yes** an existing tool does it; **partial** a tool covers part of it; **missing** no
tool; **n/a** no agent route needed (pure display, pointer feedback, or a fine-grained gesture
whose result an existing op already reaches). "Lot" points at the lot that closes the gap.

### Deck editing

| UI action (trigger) | Agent route | Status | Lot |
|---|---|---|---|
| Rename deck title (TitleField) | `apply` set_deck_param title | yes | |
| Go to a slide (pile click, chevrons, arrows, Home/End) | `focus` | yes | |
| Reorder slides (drag, ⌥↑/⌥↓) | `apply` move_slide | yes | |
| Delete a slide (pile trash) | `apply` remove_slide | yes | |
| Add a slide from a template (TemplatePicker) | `apply` add_slide, `templates` | yes | |
| Change the deck's template pack (picker select) | `apply` set_templates | yes | |
| Select / deselect an element (stage click, Esc) | `focus` with no slide deselects; selecting stays the user's | yes | 5 |
| Move / nudge an element (drag, arrows) | `apply` set_frame / set_cell / move_element | yes | |
| Resize an element (corner handles) | `apply` set_frame / set_cell | yes | |
| Delete an element (Delete key) | `apply` remove_element | yes | |
| Edit a field in place (Enter, double-click, TextEditor) | `apply` set_param / edit_list, `ask_rewrite` | yes | |
| Bold, italic, link, list item move while editing | rich runs in set_param, edit_list move | yes | |
| Add text, card, shape, chart, table or pack widget (WidgetPicker) | `apply` add_element, `widgets` | yes | |
| Copy / cut / paste elements (⌘C/X/V) | `apply` add_element / move_element | yes | |
| Paste external text as a new element | `apply` add_element | yes | |
| Element and slide params (TargetPanel, ListTable) | `apply` set_param / edit_list | yes | |
| Chart type, stack, series colours | `chart_kind`, `apply` set_chart_option | yes | |
| Chart / table data cells, rows, columns, TSV paste | `apply` set_data | yes | |
| Table header row/column, stripes, alignment | `apply` set_table_option | yes | |
| Show chart as table and back | `show_as` | yes | |
| Undo / redo (⌘Z, ⌘⇧Z, tray buttons) | `undo`, `redo` | yes | 5 |
| Undo the agent's turn (button under the receipt) | `undo_turn` | yes | 5 |
| Flush autosave (⌘S) | autosave runs anyway; `save` | n/a | |
| Save a copy elsewhere (⌘⇧S) | `save` with a path | yes | |
| Deck theme | `apply` set_theme, `edit_theme`, `theme_tokens` | yes | |
| Pack notice: use mine / make current / dismiss | `pack_update` (nothing to dismiss) | yes | 5 |

### Files, import, history, templates

| UI action (trigger) | Agent route | Status | Lot |
|---|---|---|---|
| Open a .oku-slides.pdf (menu, ⌘O) | `open` | yes | |
| Open a .pptx and import it (ImportDialog: mode, pack, theme, tune) | `import` (confirmed) | yes | 5 |
| New blank deck | `new` | yes | |
| Move the deck file, merge another copy | `move`, `merge` (lot 1) | yes | |
| History: open, seek (slider, ←/→), restore, back | `history` (lot 1) | yes | |
| Enter / leave templates mode, Finish | `edit_templates`, `save_templates`, `close_templates` | yes | |
| Widget files: create, edit, delete | `set_widget`, `remove_widget`, `widget_source(s)` | yes | |
| Rename a widget file (F2) | write new + remove old (settled decision) | yes | |
| Widget test values (preview editing) | `set_widget_trial` | yes | |
| Template and theme settings (HostSettings, ThemePanel) | `edit_theme`, `apply` set_token | yes | |
| Activate / close template sub-tabs, show code, resize code pane | `set_interface` code (templates mode is gone; no code width, R38) | yes | 7 |
| Delete a user template pack | `delete_template_set` | yes | |

### Presentation

| UI action (trigger) | Agent route | Status | Lot |
|---|---|---|---|
| Present full screen / in the window (menu, ⌘↩, ⌘⇧↩) | `present` (lot 1) | yes | |
| Next / previous / first / last slide (arrows, Space, click) | `present` next, previous, first, last, show; local voice words | yes | 4 |
| Leave the presentation (Escape, leaving OS full screen) | `present` exit; local voice word | yes | 4 |

### Tabs, windows, app

| UI action (trigger) | Agent route | Status | Lot |
|---|---|---|---|
| Activate a tab (click, ⌘1-9, Ctrl+Tab) | `tabs`, `tab` activate | yes | 6 |
| Close a tab, close others (×, ⌘W, context menu) | `tab` close (confirmed) | yes | 6 |
| New tab (+, ⌘T) | `new` (blank deck, confirmed) | yes | 6 |
| Reorder tabs, drag a tab out, move a tab to a window | `tab` reorder, move | yes | 7 |
| New window (⌘⇧N), close window, quit (⌘Q) | `window` new, close; `quit` (both confirmed) | yes | 6 |
| Detach / re-attach the agent window | `window` detach_agent, attach_agent | yes | 6 |
| Open help, page through it, search it (⌘?, ⌘F) | `help` with query; `focus` pages it | yes | 6 |
| Check for updates (macOS app menu only) | `update` check | yes | 5 |
| Install update / see release / later (UpdateBar) | `update` install (confirmed), later | yes | 5 |
| Scheme, locale, grid, guides (⌘', ⌘⇧G) | `set_interface` (lot 1) | yes | |
| Resize or fold pile, notes column | `set_interface` folded (slides, layouts), pile and notes widths | yes | 7 |

### Agent and dictation

| UI action (trigger) | Agent route | Status | Lot |
|---|---|---|---|
| Start / stop dictation (mic, ⌘D, ⌘⇧D) | hands-free listening, resumed by one gesture per launch; the stop word | yes | 2 |
| Send the draft to the agent (⌘↩, Send) | hands-free sends each utterance | yes | 2 |
| Edit / discard the draft (⌘K, trash) | n/a in voice mode (each utterance is sent) | n/a | 2 |
| Cancel the running turn (Esc, ⌘⇧D, stop button) | the stop word; a new utterance interrupts | yes | 2 |
| New conversation (thread header button) | `new_conversation` | yes | 6 |
| Choose the ACP agent (gear popover chips) | `select_agent` | yes | 6 |
| Dictation model, language, auto-stop, silence, VAD | `set_dictation` through `VoicePort` (confirmed; never `set_setting`) | yes | |
| Download a dictation model | `download_dictation_model` | yes | 6 |
| Answer ask_choice / ask_rewrite (click, digits) | dictated reply goes back as `answer_text` | yes | |
| Answer ask_confirm (modal, click) | dictated reply goes back as `answer_text` | yes | |
| Confirm a guarded tool (save, open, move, merge, new, set_setting…) | host lexicon: yes / no, the verb for danger | yes | 3 |
| Answer an ACP permission request | yes / no by option kind (allow / reject once) | yes | 3 |
| Pick a variant (VariantDrawer, digits) | an ordinal or a label | yes | 3 |
| Report action buttons (suggested next steps) | dictating the step is equivalent | yes | |
| Jump to latest, fold the dock | display only | n/a | |

## Hands-free obstacles

1. **Dictation needs a key or a click to start.** `stt.start` is called only from
   `Dictation.tsx` `toggle()` (mic, ⌘D, ⌘⇧D, relay from the agent window). No wake word, no
   continuous listening, no restart after the agent answers. Stopping is already hands-free
   with `dictation.auto_stop` (default on, 1.5 s of silence).
2. **The draft is never sent by itself.** Since t-953 dictated text lands in the draft and
   waits for ⌘↩ or Send (`draftActions.ts`).
3. **A turn cannot be cancelled by voice.** Send is disabled while the agent is busy
   (`DictationDraft.tsx`), although the host accepts a prompt that interrupts a turn
   (`oku-slides-agent` `stack.rs`).
4. **Guarded confirmations and permissions refuse dictated replies.** `SurfaceHub::confirm`
   accepts only a `confirm` click and `permission` only an option click; `answer` releases them
   as a refusal. Every guarded tool (save, open, move, merge, new, save_templates,
   delete_template_set, edit_theme on a library theme, set_setting) is therefore unreachable
   by voice, including turning dictation settings on.
5. **The presentation has no voice exit or navigation.** Escape or leaving OS full screen
   only; `present` says "the user leaves with Escape". During a talk every sentence would also
   reach the draft. Suspected bug (from reading, unverified): Escape while dictating both
   cancels the dictation and exits the presentation, because both listeners sit on `window`
   in the capture phase and `Dictation` calls `stopPropagation`, not
   `stopImmediatePropagation`.
6. **Tabs, windows, help, new conversation, agent choice and update check are UI-only.** The
   update check is in the macOS app menu only (none on Windows or Linux).

## Design principles

- **The agent never decides a confirmation.** A voice confirmation is matched by the host
  against the transcript the STT engine produced from the microphone; the agent does not see
  it before the decision. An imported file or a web page cannot make the agent self-confirm.
- **Latency-critical commands do not go through the model.** Presentation navigation and
  "stop" must react in well under a second; they use a small fixed command lexicon matched
  locally. Everything else goes through the agent.
- **Workspace tools touch tabs, never other tabs' documents.** Tools stay scoped to their tab
  (crate rule); new workspace tools list tab titles and act on tabs, they never read or edit
  another tab's deck. Closing or replacing another tab is confirmed.
- **The microphone is never on without a visible sign.** Continuous listening shows a steady
  indicator and stays on-device (local STT engines only).

## Lots

Numbering continues t-988 (lot 1). Priority: P0 makes a voice-only session possible at all,
P1 removes the frequent dead ends, P2 completes coverage, P3 is low-value layout.

### Lot 2 — Voice conversation loop (P0) — t-996

- A `dictation.hands_free` setting (set by `set_dictation` through `VoicePort`, never `set_setting`; persisted). When on: the
  mic starts when a tab gains focus and at launch, each utterance closed by auto-stop is sent
  to the agent at once (no draft review), and listening resumes right after.
- An utterance while the agent is busy interrupts the turn (the host already supports a prompt
  that cancels the running turn). A fixed localized "stop" lexicon (Fluent list, e.g. *stop*,
  *annule*) cancels without a new prompt and is matched locally.
- Utterances shorter than `stt.vad.min_phrase` or empty after trimming are dropped, so noise
  does not cancel turns.
- The draft keeps its role when hands-free is off; the mic button and a hands-free toggle carry
  accessible names so macOS Voice Control can press them once (the bootstrap gesture).
- Tests: Rust for the setting and its whitelist; UI render tests for the auto-send and resume
  loop with the fake STT; e2e on the fake agent.

### Lot 3 — Voice answers to confirmations, permissions and variants (P0) — t-997

- `SurfaceHub::confirm` and `permission` accept an `answer` whose text matches, after
  normalization (case, accents, punctuation), a localized lexicon: yes / no words per locale
  (Fluent lists, English reference). For a `danger` confirmation the yes side also requires the
  button's verb (e.g. *fusionner*, *supprimer*). Anything else leaves the question open and
  shows what was heard; it is never read as a refusal.
- Permissions and variant drawers accept an ordinal (*the first*, *option two*) or an option's
  label. A variant drawer becomes answerable by the same route (today a dictated reply becomes
  a new prompt).
- The 500 ms arming of modals also applies to voice: an utterance that began before the modal
  showed does not answer it.
- Update the crate rules: "a dictated reply answers a confirmation only through the host
  lexicon, never through the agent".
- Tests: hub unit tests per locale (key-based, plus a non-English locale), a regression test
  that an agent-originated text cannot confirm.

### Lot 4 — Presentation by voice (P1) — t-998

- `present` gains actions: `start {mode}` (today's call), `show {slide}`, `next`, `previous`,
  `exit`. Refused when nothing is presenting (except `start`).
- In a presentation, hands-free listening acts only on a local lexicon (*next*, *previous*,
  *first*, *last*, *leave the presentation*) and on utterances that start with a wake phrase
  (*Oku, …*) which go to the agent; all other speech is dropped, so the talk never reaches the
  agent. The wake phrase is a Fluent string per locale.
- Fix the Escape double action (5 above) with a regression test first.
- Tests: Presenter render tests for each command; MCP tests for each `present` action.

### Lot 5 — Missing deck and app tools (P1) — t-999

- `undo {steps?}`, `redo {steps?}` and `undo_turn {turn}` over the existing session requests;
  no confirmation (undoable by nature).
- `import {path, mode: hifi|retrofit, pack?, theme?, tune?}`: opens the import in a new tab like
  the dialog; confirmed; paths validated like `open`.
- `pack_notice {use_mine|make_current|dismiss}` over the existing session commands.
- `update {check|install|later}`: the host's updater commands; `install` confirmed (it
  restarts the app). Adds the missing in-page "check" route on every OS.
- `focus` with no slide deselects (the voice Esc); it never selects an element (amended below).

### Lot 6 — Tabs, windows, help, conversation (P2) — t-1000

- `tabs` lists the window's tabs `{tab, title, active, dirty, kind}`; `tab {activate|close,
  tab}` acts on one (closing another tab, or a dirty untitled one, is confirmed; never closes
  the agent's own tab mid-turn: it closes after the turn ends).
- `window {new|close|detach_agent|attach_agent}`; `quit` confirmed.
- `help {open, query?}` opens the help tab, searched when `query` is given; `focus` pages it.
- `new_conversation` takes effect once the current turn has ended.
- `select_agent {id}` (takes effect at the next turn) and `download_model {model}`; the agent
  choice is listed by `settings`.

### Lot 7 — Layout and view (P3) — t-1001

- `set_interface` gains `folded {pile, params, …}`, column widths and the template sub-tab /
  code pane state, through the same kept-UI path (`req ui`).
- Tab reorder and move-to-window by voice, if still wanted after lot 6.

### Lot 8 — Read-aloud with the OS voice (P1) — t-1002

The user can look away: Oku reads out what they would otherwise have to read on screen.

- **What is read:** the receipt of each turn (title, then facts; `why` stays folded and
  silent), every question as it opens (ask_* message and its options in order, so an ordinal
  answer makes sense), guarded confirmations (message and the verb to say), permissions (how
  to answer), and turn failures (a host template per failure kind, never the error message;
  tool errors are not read). Progress notes, thread history and slide content are never read.
- **Engines, OS only:** macOS `AVSpeechSynthesizer` (available since macOS 10.14, so it fits
  the 12.0 minimum) through the existing Swift bridge of `oku-slides-stt`; Windows
  `Windows.Media.SpeechSynthesis` (WinRT, SAPI voices otherwise); Linux speech-dispatcher when
  its daemon answers. The plan weighs the `tts` crate, which wraps these three, against the
  Swift bridge plus two small bindings (dep audit, macOS 12 support). No bundled voice, no
  cloud service.
- **Availability is explicit:** where no engine or no voice for the language exists, the
  status says "unavailable" with the reason in the dictation settings and `settings`; the
  read-aloud switch is disabled there. Nothing falls back silently to another language.
- **Voice and language:** the voice of the interface locale (or of the text's language when
  the agent wrote in another one), chosen among the OS voices; settings `speech.read_aloud`
  (default on when `dictation.hands_free` is first enabled, off otherwise), `speech.voice`,
  `speech.rate`, set by `set_speech` through `VoicePort` (never `set_setting`).
- **No echo loop:** listening is suspended while Oku speaks and resumes when the utterance
  ends; transcripts overlapping speech are dropped. A local "stop" word (lot 2) cannot be heard
  during speech, so speech stays short (receipt title plus at most 4 facts) and a new question
  or turn cuts the current speech.
- **Never during a presentation:** the audience must not hear Oku; read-aloud is muted while
  presenting, except the answer to a wake-phrase request (lot 4), and only when the user
  enables `speech.in_presentation`.
- **Where it runs:** in the host, next to dictation (one coordinator for mic and voice, shared
  by the main and the detached agent window), so `oku-slides serve` gets it on the dev machine
  too. A `say` request on the stt side, driven by the hub events the UI already receives.
- Tests: Rust unit tests on what is read for each surface kind (keys, plus a non-English
  locale), the mic suspension around speech with a fake engine; a manual check per OS.

## Open questions

- Barge-in while Oku speaks: decided, see "Amendments (2026-11-05)".
- Wake phrase outside presentations: lot 2 sends every utterance; a wake phrase may be needed
  in noisy rooms. Decide after lot 2 is in use.
- The bootstrap gesture (first enabling of hands-free) stays a click or macOS Voice Control
  unless hands-free becomes the default.

## Amendments (2026-11-01)

Checked against `main` at 88e5af3f before planning lots 2 to 8. Where this section and the text
above disagree, this section wins.

- **Lot 1 is done** (t-988 closed). `set_interface` has no grid any more: the stage grid shows only
  while dragging (t-1283); it takes `scheme`, `locale`, `guides`.
- **Dictation settings go through `VoicePort`**, never `set_setting` (settled decision t-1144):
  `dictation.hands_free` is a `set_dictation` field and a row of the settings box, stored in the
  STT `config.json`. `set_setting` only writes next-launch keys (`Applied::NextLaunch`), so the
  `speech.*` settings of lot 8 also go through `VoicePort` (a `set_speech` tool, a box section).
- **Already covered, dropped from lot 6:** `select_agent`, `download_dictation_model`,
  `download_agent_model` and their cancels are MCP tools (`voice_tools.rs`); the agent choice is
  listed by `settings`.
- **Check for updates exists on every OS** in the in-page main menu (`AppMenu`, app only); only
  the native macOS menu entry is macOS-only. Lot 5 still adds the agent route.
- **Pack notice:** the notice bar is gone; "use my version" and "make theirs my version" are
  actions of the `show_packs` panel (`PacksEvent::UseMine`, `MakeCurrent`). Lot 5's tool is
  `pack_update {pack, action: use_mine|make_current}`; there is nothing to dismiss.
- **Templates mode is gone** (pack tabs replaced it): lot 7 has no template sub-tabs. Kept UI
  state is `pile` and `notes` widths, `code`, `folded` (`slides`, `layouts`).
- **`focus` already deselects** with `element` absent or null, but needs a `slide`; lot 5 makes
  `slide` optional. Since t-1502 on `main` (the agent never changes what the user looks at),
  `focus` takes `slide` only and never selects an element; with no slide it still deselects.
- **Dictation is polled, never pushed:** each `Dictation` polls `stt.status()` and reads a new
  `seq` as the end of a dictation, after which the microphone is closed. Lot 2 needs a
  continuous mode in the STT crate, not a restart loop in the page.
- **The agent cannot tell typed from dictated text today:** a prompt carries only `text`. Lot 3
  identifies a voice reply by the STT utterance it came from, read by the host from `Stt`.
- **The Escape double action is confirmed by reading** (not yet by a test): `Dictation` and
  `Presenter` both listen on `window` in the capture phase, `Dictation` first, with
  `stopPropagation` only.
- **Presentation state is page-only:** the host does not know whether a tab presents. Lots 4 and
  8 need it, so the page reports it.
- **Linux voice:** the `speech-dispatcher` crate is LGPL-2.1, outside the `deny.toml` allow list,
  and the `tts` crate depends on it and on the old `objc` crates. Lot 8 drives `spd-say` as a
  child process on Linux, AVSpeechSynthesizer through the existing Swift bridge on macOS, and
  WinRT `SpeechSynthesizer` through the `windows` 0.62 crate already locked.

The plan review of 2026-11-01 (15 reviewers) changed these points. Each is subject to the
user's answer to the plan's open questions.

- **The host routes utterances.** The STT crate closes utterances in a listening state. The host
  sends each one to the active tab of the focused window, with the context the page last
  reported. The page never sends an utterance and never sees its text.
- **Permissions by voice match the option kind, never its label or position.** "Yes" picks allow
  once and "no" picks reject once; the *always* options are never picked by voice. The agent
  writes ACP option labels and their order, so a label could say "no" for allow-always. Ordinals
  and labels stay for the variant drawer, whose picks can be undone.
- **A typed reply to a guarded confirmation keeps it open.** It no longer counts as a refusal and
  no longer starts a prompt.
- **The agent may turn hands-free off, never on.**
- **While no workspace window has the focus**, only the stop word and wake-phrase utterances go
  through, and danger confirmations by voice need a focused window.
- **`import {path, mode, pack?, theme?, tune?}`**: `pack` names the retrofit layout pack(s)
  (`ImportTarget::Retrofit.layouts`).
- **`help {query?}`**: opening is implied, so the ticket's `open` is dropped.
- **`focus`** pages through the help tab from that tab's own agent.
- **Read-aloud uses the voice of the interface locale.** "Or of the text's language" is dropped:
  detecting it needs a new dependency, and the agent answers in the user's language.
- **Quit and update install run after the turn ends**, never inside the tool call.

The second plan review (15 reviewers, draft 2) adds these points:

- **Window focus comes from the host only.** The app tracks focus from Tauri's window events on
  every window. In serve, the page reports `document.hasFocus()` for routing only, and danger
  confirmations by voice are refused there.
- **Hands-free starts paused at every launch.** It listens only after a user gesture, so a
  written config cannot turn the microphone on at launch.
- **`stt.min_phrase_secs` (0.3 s)** is a new noise floor below which an utterance gets no number.
- **The spoken confirmation reads the template and the verb only**, never the message, which may
  name a tab or hold a path. The arming delay starts when the question's speech ends.
- **`quit` is refused while a tab holds unsaved work**, and that is checked again when it runs
  after the turn. Stop drops every action queued after the turn.
- **The spec's `say` is the STT crate's `Speaker`**, one per process.

## Amendments (2026-11-05)

- **Barge-in on macOS only** (t-1491, plan `docs/superpowers/plans/2026-11-05-lot-o-backlog.md`).
  Hands-free listening with the `speech.barge_in` setting on (default on) captures through one
  voice-processing `AVAudioEngine` that also plays Oku's speech, so Apple's echo canceller removes
  it. While Oku speaks, listening is not suspended and has no deaf window; the speaker polls
  `Stt::user_speaking` every 50 ms and cuts the speech when the user talks; the utterance then
  goes through the normal router, the self-echo filter included. Off macOS, with the setting
  off, or when the duplex does not start (no main run loop, as in `oku-slides serve`), the
  lot 8 behaviour stays (muted while speaking) and the settings box and `settings` tool say why.
  No WebRTC echo canceller elsewhere: it would add a native dependency for a gain not yet asked.
