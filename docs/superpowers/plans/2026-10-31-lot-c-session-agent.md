# Lot C: session and agent follow-ups of the t-988 review — Implementation Plan

**Parent task:** t-1326. **Tickets:** t-1004, t-1007, t-1003, t-1072, t-1073, t-956, t-1005, one task each, plus a review
and gates task. Batch of 2026-10-31 (lots A to G in parallel worktrees).

**Goal:** Close the session and agent follow-ups left by the t-988 review (main menu reduced to
Present, Open and Help; the rest goes through agent tools).

## Task 1: Remove `Request::SetTheme` from the session (t-1004)

- Delete the variant, its `handle` arm and its arms in `request_class.rs`; `Session::set_theme`
  stays (the theme picker calls it). `SetPreviewTheme`'s doc stops citing it.
- The UI never sent `set_theme` as a command (only the `set_theme` op): no `api.ts` change.
- Tests that used the request as a shortcut go through `Session::set_theme` or the op
  (`theme_tab_ops_tests`, `request_class_tests`, `deck_theme_tests`, `pack_mode_tests`,
  `test_support_packs`).

## Task 2: `Session::can_present` holds the presentation rule (t-1007)

- `history.rs` beside `ensure_deck_mode`: `can_present()` fails with a distinct error per cause:
  read-only help (`ReadOnly`), pack mode (`EditingPack`), theme tab (`WrongDocument`), history
  (`InHistory`), no slide (new `NothingToPresent`, Fluent key in 20 locales).
- `Snapshot.can_present: bool` (`can_present().is_ok()`); `App.present` reads it instead of
  repeating the rule (`api.ts` mirrors the field).
- The MCP `present` tool calls `can_present` and tells the agent the session's localized reason;
  `ToolError::NothingToPresent` and `agent-error-present` go.
- Tests: each refusal (help tab, pack tab, theme tab, history, empty deck) and the deck-mode
  success in the session; the MCP tool on a help tab.

## Task 3: Paginate the `history` tool's version list (t-1003)

- `history_page.rs` in `oku-slides-mcp`: `HistoryPage { total, before, versions, milestones }`
  from `&[Version]`, `before` (exclusive rank, default `total`) and `count` (default 20, max 100).
  Each version carries its `index`. `milestones`: the session-start and merge versions older than
  the page, the 20 most recent. `next_before` names the next page's `before` (absent at the start).
- `HistoryArgs` gains `before` and `count`; `open` and `list` both answer the page. The UI keeps
  `history_versions` (whole list, for its own bar).
- Tests: default page, `before`/`count` bounds, clamped `count`, milestones beyond the page.

## Task 4: One accent fold in `oku-slides-i18n` (t-1072)

- `oku_slides_i18n::fold`: lowercase, NFD, drop U+0300–U+036F, `œ`→`oe`, `æ`→`ae`, `ø`→`o`,
  `ß`→`ss`, `’`→`'`, then NFC (Hangul and Indic vowel signs come back whole).
- `shortcut::normalize` (agent), `search::fold` (session; an excerpt matches as a title does,
  on the whole line's fold) and `namespace_slug`'s table (session)
  use it; the session's two Latin tables go. The UI's `normalizeShortcut` spells the same letters.
- Tests in `oku-slides-i18n` (table cases), the existing shortcut, search and slug tests stay green.

## Task 5: Shortcuts recognized in the dictation language (t-1073)

- Host: `is_spoken_theme_shortcut` checks the interface language, then (asked lazily) the
  dictation's (`AppVoice::dictation_locale` over `Stt::languages`: the chosen language, else the
  detected one; shipped only), whose words come from `theme_shortcuts()`.
  `THEME_SHORTCUT` moves to `oku-slides-i18n`, with `theme_shortcuts()` (every locale's words, built once).
- UI: `Messages.shortcuts` carries `ui-shortcut-theme` of every shipped locale;
  `isShortcut(text, dictationLanguage)` also accepts that language's words. `DictationDraft`
  gets the dictation language from the dictation status.
- Tests: Rust (a German dictation with a French interface opens the picker), TS `shortcut.test.ts`.

## Task 6: App launched from Finder takes the login shell's PATH (t-956)

- `oku-slides-platform`: `Platform.program_path: ProgramPath` (where external programs are found
  and launched; `with_settings` fixes the process's). `login_path.rs`: `$SHELL -ilc` (interactive
  too, so rc-file tools such as nvm count) prints `PATH` after a marker, 3 s at most, read up to the
  marker line only (a background job may hold the pipe); its entries come first, then the
  process's missing ones, absolute entries only. Resolved on a thread (`ProgramPath::from_login_shell`,
  `get` waits; the thread fills the cell even if
  resolving panics) so the window never waits. `Platform::with_login_path()` in the app only (src-tauri).
- The agent's availability check and spawn (`choice.rs`; `Launch.path`, read in `link.rs` off the
  runtime threads, is the child's `PATH` and its lookup) and the soffice lookup
  (`import/reference.rs`) use `program_path`.
- `agent-not-found` says the command exists neither at that path nor in the app's PATH and
  suggests an absolute command (20 locales).
- Tests: marker parsing, merge order, a fake shell script, timeout; `find_program` on the merged
  path.

## Task 7: Page tests for agent events (t-1005)

- `useFileActions`: autosave restarts on a path change (render test).
- App applies `present`, `set_interface` and `open_history` events (render or unit tests on the
  handlers).
- e2e `300-agent-page-events.cjs`: the fake agent calls `present` and `set_interface`.

## Task 8: Lot C review and gates

- Multi-angle review (`common:review`), fixes, `just ci`, `just e2e`, AGENTS.md / ARCHITECTURE.md
  updates, one squash commit citing every ticket, push to `main`.
