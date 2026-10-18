---
task: t-936
---

# Backlog of 17 October — lot 5 known limits

Tickets checked against main @ 84ebfaa2. Closed without code: t-896 (shipped in 916e2ad8), t-910
(duplicate of t-912: the witness runs in the Seatbelt-sandboxed render worker). Still blocked
upstream: t-893 (Loro still 1.16.2), t-899 (umya-spreadsheet 3.1.1 still on zip ^8.6). Deferred:
t-909 (needs downloading third-party .pptx files: the user approves each source). Not doable here:
t-213, t-648 (manual checks in the Tauri app), t-179, t-341 (Windows), t-466, t-700, t-343 (native
speakers), t-886 (no PowerPoint), t-223 (upstream Loro), t-635 (watch). New: t-937 (the deck-only
pack in the pack picker, the t-903 gap on the pack side).

## Rules

- One lot at a time, on branch `backlog-2a6252` (this worktree), in the order below: the lots
  share `App.tsx`, `views.rs`, `ui.ftl` and `config.rs`, and the cargo target is shared.
- Each lot: TDD (red test first, run), implemented by one subagent that stages only; the
  controller reviews the staged diff, then commits with `git commit -F <file>` and the trailer
  `Review-waived: revue par lot et revue finale (oku.review=deferred, t-936)`.
- Cargo runs plainly (`cargo …`, `just …`): the global cargo config gives each worktree its own
  `build-dir`. Lots 1–9 ran with a shared `CARGO_TARGET_DIR` and a `touch` of every source
  first; that override was dropped on 2026-10-18.
- Data dirs: `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` set to a fresh `mktemp -d`
  for every run outside `just e2e`; e2e on a free port (not 4870–4872, 5173).
- New Fluent keys go in all 20 locales; a removed key is removed from all 20.
- Each lot updates the `ARCHITECTURE.md`/`AGENTS.md` it invalidates, and removes its bullet from
  "Known limits" in `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.
- End: one review of the whole branch by the team, `just ci`, squash onto main.

## Lot 1 — `[import] max_input_mib` (t-908)

- `oku-slides-platform/src/config.rs` `ImportSettings`: `pub max_input_mib: NonZeroU64` (0 refused at
  load, like `tune_*`) and `fn max_input_bytes(&self) -> u64` (saturating MiB → bytes, like
  `SourcesSettings::max_sources_bytes`). `config.default.toml` `[import]`: `max_input_mib = 256`,
  commented (largest .pptx accepted, checked before reading, minimum 1).
- `oku-slides-session/src/import/input.rs` `read_input`: `max_input_bytes` from the settings; its doc
  updated. `ReadLimits::default()` keeps 256 MiB for the reader's own tests, commented.
- Tests: `config_tests.rs` default (256 and 256 << 20), env override, 0 refused, saturation;
  `import/mod_tests.rs` a file over a 1 MiB limit gives `ImportErrorKind::TooLarge`.
- Docs: README `[import]`, `oku-slides-session/AGENTS.md` mention of `max_input_bytes`.

## Lot 2 — A .pptx given by the OS opens the import dialog (t-906)

- Session: `Workspace::open_files` (`workspace_resume.rs`) no longer opens a `.pptx` (extension,
  any case) as a tab; it records it as `WorkspaceView.import_pick: Option<ImportPick { window,
  path, id }>` for the focused window (the view survives a webview that is not listening yet and
  works in web mode). `id` increases; a newer pick replaces an older one.
- Host: nothing new beyond `windows::sync` after `open_files` (already there); the view carries it.
- UI `Workspace.tsx`: when the view's `import_pick` targets this window and its `id` is new,
  `imports.pick(path)` once (same once-per-id pattern as `mergedNotice`).
- Tests: `crates/oku-slides-session/tests/workspace.rs` next to `open_files_*`: a .pptx is not opened
  and sets no notice, it is the focused window's pick, a mixed list still opens the decks, `.PPTX`
  is recognized, a second pick gets a higher id; UI render test that a view with `import_pick`
  opens the dialog (`ui-import-title`) once. No e2e (no OS delivery in web mode).
- As built: the page acknowledges a pick it took with `WorkspaceRequest::ImportPicked { id }`; the
  host forgets that pick only, so a newer pick survives (one taken during an import waits for it to
  end). Lot 2 and lot 5 tests live in `session/src/workspace_import_pick_tests.rs` and
  `agent/tests/availability.rs`.

## Lot 3 — Edit a deck-only pack in templates mode (t-902)

- `templates_mode.rs` `edit_templates`: base from a helper `templates_base(id)`: the deck's embedded
  version when the deck's pack is `id` and deck-only, else the library's current version. A deck-only
  id found only in history stays refused (`session-deck-only`). `TemplateMode` gets
  `#[serde(default)] deck_only: bool`.
- `adopt_session_version`, `realign_templates`: for a deck-only pack, no library write at all
  (`add_version`, `replace_version`, `set_current`, re-add); embedding in the working doc and the
  set-aside deck, the merge key and the removal of intermediate versions stay. Factor the library
  writes into one function, as `widgets_deck.rs` does. `edited_manifest` keeps the base scope instead
  of hardcoding `Library`. "Save as copy" of a deck-only pack stays deck-only (new id, never the
  library).
- UI: remove the four deck-only guards (`App.tsx` `editTemplates`, menu item and hint,
  `onOpenWidget`); remove `ui-menu-workshop-deck-only` (20 locales); add `EditingView.deck_only` and a
  templates-mode note `ui-templates-deck-only` ("Saved in this presentation only", 20 locales).
- MCP: `tools.rs` edit_templates description and `mcp/tests/import.rs` follow; `import.md` keeps
  forbidding edit_templates during the tune turn.
- Tests: `templates_mode_tests.rs` (starts from the embedded version; save embeds a deck-only
  version, library unchanged; second save replaces in sources only; undo writes no library;
  history-only id refused); `widgets_edit_tests.rs` (set_widget in templates mode on a deck-only pack
  writes no library); `resume_tests.rs` (resumes deck-only; a state without the field reads);
  regression `tests/regressions/issue_0902_deck_only_workshop.rs`; e2e `220-import.cjs` opens the
  Workshop after the hi-fi import, saves, checks `deck_pack` still true and no new library pack.

## Lot 4 — Re-pick the deck-only theme (t-903)

- Session, new `deck_themes.rs`: `deck_only_themes() -> Vec<ThemeInfo>` (deck-scope themes embedded
  in the sources, one per id: the version the deck uses if it uses that id, else the leaf of the
  `parent` chain, ties by newest `created`), `deck_only_theme_ref(id)`.
- `Snapshot.deck_themes: Vec<ThemeInfo>` (and `api.ts`). `set_theme`, `set_edited_theme` and
  `theme_base` resolve library first, then deck-only; else `UnknownTheme`.
- UI `ThemeMenu.tsx`: `deckThemes` prop rendered as an enabled `<optgroup>`
  `ui-toolbar-theme-deck-group` ("This presentation", 20 locales); passed from `App.tsx` and
  `ThemePanel.tsx`. `ImportDialog` keeps filtering deck-only themes out.
- Tests: `deck_themes_tests.rs` (listed after another theme is chosen; one per id; re-picking it
  re-adopts the same `ThemeRef` and leaves the library unchanged; unknown id refused);
  `theme_edit_tests.rs` (edit starts from the embedded version); regression
  `issue_0903_repick_deck_only_theme.rs` (import theme, pick Papier, save, reopen, re-pick);
  `ThemeMenu.render.test.tsx`; e2e `220-import.cjs` picks Papier then the imported theme back.
- As built: MCP `deck_outline` lists the deck-only themes too, next to the library's.

## Lot 5 — "Fine-tune with the agent" disabled when no agent is usable (t-901)

- Platform, new `program.rs`: `find_program(storage, name, path_var) -> Option<PathBuf>` (a name
  with a separator is checked as is, a bare name is searched on PATH); `find_soffice` reuses it.
- Agent: the link remembers its last connection failure (`NotFound`, `Spawn`, `NoHttp`; not
  `Stopped`), cleared on success and on switch. `Stack::availability()` gives a localized reason or
  none: preset command not found on PATH (`agent-not-found`), else the last failure. New
  `AgentCommand::Availability` / `AgentEvent::Availability { unusable }`, also emitted after a
  failure. `Stack::tune` refuses (logs, sends nothing) when unusable.
- UI: `agentApi(tab).availability()`; a small hook `ui/src/agent/useAgentAvailability.ts`
  (App.tsx is near its cap); `ImportDialog` prop `agentUnusable`: box disabled and unchecked, note
  `ui-import-tune-unavailable { $reason }` (20 locales), submit sends `tune=false`.
- Tests: `program_tests.rs` (found on a temp PATH, absolute path, missing); `link_tests.rs`
  (failure recorded, cleared by switch); `agent/tests/stack.rs` (bogus command → not-found reason;
  tune with an unusable agent sends nothing); `ImportDialog.render.test.tsx`.

## Lot 6 — kitchen p. 2 at 8.7 (t-904)

Investigation first (systematic-debugging): zip `fixtures/import/kitchen`, import hi-fi with the CLI
and a report, find which region scores 8.7, render Oku's and the witness's page 2 and compare.
Hypotheses in order: the title in Calibri Light (no Carlito Light, different fallbacks); the
grouped text boxes "Gauche"/"Droite" (position in `grpSp`, first baseline at 16 pt in a 40 pt box);
the picture. Outcome: an Oku fix with a regression test, or a new `WitnessLimit` with a
`witness_tests.rs` case, and kitchen added to `crates/oku-slides-cli/tests/import_reference.rs` with
its expected bound. Update the 8.7 mentions (`config.default.toml`, `oku-slides-reference/ARCHITECTURE.md`).

## Lot 7 — LibreOffice witness in the app (t-905)

- Measure: run `--reference soffice` (with `OKU_SLIDES_TEST_SOFFICE=1` for the tests) on alto,
  kitchen and libreoffice; list LibreOffice's systematic gaps; add `Witness::LibreOffice` with its own
  `witness_limit` arm in `metrics/witness.rs`; `limits_of` maps Soffice to it.
- Offer it: `[import] witness = "office2pdf" | "soffice"` (default `office2pdf`, validated,
  documented). The app path (`workspace_tabs.rs` → `ImportRequest` → `ImportState`) honors it; a
  missing or refused soffice is reported exactly as in the CLI (`witness_missing`/`witness_refused`),
  never a silent fallback to office2pdf.
- Tests: `witness_tests.rs` per new limit; config tests; a session test that the setting selects
  the witness; the real-LibreOffice test stays opt-in.
- As built: the CLI `--reference` default (`auto`) now follows `[import] witness`.

## Lot 8 — Help decks present the .pptx import (t-907)

Only `docs/aide/fr.json` and `docs/aide/en.json` exist (other locales fall back to English), so the
page goes in those two. New slide `s-import` after `s-fichiers`, same id in both: Open a .pptx,
"Copie fidèle" vs "Utiliser mes modèles", "Peaufiner avec l'agent" (and when it is disabled, lot 5),
the report and frozen elements, editing the imported templates and re-picking the imported theme
(lots 3–4). Wording matches the Fluent strings. `help_tests.rs` checks ids and validity.

## Lot 9 — Split workspace.rs and App.tsx (t-911)

Pure moves, last (after every lot that touches them), each its own commit, no test changed except
`use` paths, suites green before and after.

- `workspace.rs` → `workspace_request.rs` (requests, outcomes, views), `workspace_ids.rs`,
  `workspace_locale.rs`, `workspace_session.rs` (locking path); `Rect`/`Layout` into
  `workspace_layout.rs`, `Tab` into `workspace_slot.rs`. Target under 350 lines.
- `App.tsx` → `useClipboard.ts`, `useAppKeys.ts`, `useSlideOps.ts`, `useTemplatesMode.ts`,
  `AppMenu.tsx`, `useAgentWiring.ts`, and the layout JSX into components. Target under 500 lines.

## Final review

The whole-branch review was addressed in two follow-up commits: `fix(import) : suites de la revue
finale du backlog, côté Rust (t-936)` and `fix(ui) : suites de la revue finale du backlog,
interface et docs (t-936)`. Findings deferred to tickets t-956 to t-961; those that concern import
are in the spec's "Known limits": t-956 (app launched from the Finder: GUI `PATH`, the agent's `npx`
not found), t-957 (witness reason not localized in the report), t-958 (scope of `DefaultFont` to
measure), t-961 (only the last of several OS-opened .pptx is offered). `App.tsx` ends at 519 lines once its
long hook and prop lines are wrapped.
