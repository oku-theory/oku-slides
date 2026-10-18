# oku-slides-session

Editing session of one deck and its workspace (windows, tabs, resume), and the PowerPoint import.
The Tauri app and the dev server (`oku-slides-cli` `serve`) expose the same API.

## Session

- `Session` (`lib.rs`) answers `Request` through `Session::handle` (`request.rs`): one deck
  (`SourceBundle`, Loro replica, undo by the non-persisted `UndoManager`), its Typst rendering,
  templates, resume state (`resume.rs`, `resume_writer.rs`), file (`deck_file.rs`, `files.rs`, `merge.rs`).
- `Mode`: `Deck`, `Help` (read-only, `help.rs`), `Templates`, `History`; the last two set the deck aside (`stash`) in
  `templates_mode.rs` (`templates_library.rs`: base, library writes; a deck-only pack edits its embedded version) and `history.rs`.
- Packs (`deck.widgets`, one pin per namespace) and themes: embedded versions are authoritative; a deck-only one never reaches
  the library. `deck_widgets.rs` builds the registry per pack, `deck_fallback.rs` disables a faulty pack alone (bisection),
  `deck_closure.rs` adds `requires`, lists dependents (also `probed_widgets`); `deck_pack.rs` (`check_pack_id`), `deck_theme.rs` embed defaults;
  `library.rs` (namespace unique, kept by every version; `namespace_slug.rs`), `library_kind.rs`, `library_index.rs` (in-memory, reset on `generation`) hold `Library<K>`; `theme.rs`, `pack.rs`.
- Theme editing: `theme_edit.rs` (`ThemeEdit` draft in templates mode, `save_theme_edit`), `deck_theme_edit.rs` (`edit_theme`
  in deck mode), `theme_schema.rs` (token controls, bounds), `theme_panel.rs` + `theme_controls.rs` (`host-theme` surface),
  `appearance.rs`; `deck_themes.rs`: deck-only themes the sources embed (`Snapshot.deck_themes`), picked after the library.
- Widget workshop: `widgets_edit.rs`, `widget_preview.rs`, `widget_examples.rs`, `widget_params.rs`,
  `param_fields.rs`, `items.rs`, `widgets_deck.rs`; host settings: `panel.rs`, `target_params.rs`.
- Views: `SlideView.fields` (`fields.rs`), markdown (`text.rs`, `inline.rs`), `search.rs`, `tables.rs`.
- Rendering: one `RenderWorker` per `Session`, one `RenderSlot` per render; a tab session starts
  unrendered (`Session::unrendered`), its worker process at the first render (`ensure_rendered`).
- `import/`: PowerPoint import, CLI and app (one history step; a retrofit keeps the library pack until its first widget
  write makes the deck-only `child_pack`), `render`, `note_overflows`, `reference`, `state` (`ImportState`, tab memory),
  `score`, `geometry`, tuning (`queue`, `sample`, `diff`, `tune`). Also for the CLI: `library_lookup`, `probed_widgets`.
- `agent/`: what the session offers the agent. `tabs` (`Sessions`, `TabSession`), turns as undo
  groups (`begin_turn`; `turn_undo` names the turn atop the deck undo stack for its receipt), A2UI
  surfaces (`forms`, `turn_view`, `receipt`, `render`, `surface_ops`), `address`, `variants`,
  `context`, `scope` + `conversation` (per-scope threads; `ScopeMove` on widget rename, replayed by
  `undo.rs`), `reason` (`AgentFormReason`).

## Workspace

`Workspace` (`workspace.rs`), `WorkspaceRequest` / `WorkspaceView` (`workspace_request.rs`): windows `w-<n>`, tabs `t-<n>` (`_ids`),
detached agent. `Slot` (`workspace_slot.rs`: `Ready`, `Dormant`, `Closed`) holds a tab's session (`Tab`); other `workspace_*.rs`: `session`
(locking path, `relock`), `locale`, `scheme`, tabs (`Opening`; `ImportTab` builds an imported tab, outcome `Tune` names it for the agent), windows, layout
(`Layout`, `Rect`), resume, same deck, agent, help, `import_pick` (a `.pptx` given to `open_files` becomes the focused window's
`WorkspaceView.import_pick`, its import dialog, not a tab, until `ImportPicked`). Lock order: `Mutex<Workspace>`, then a tab's `Arc<Mutex<Slot>>`,
never the reverse; only `try_lock` a session under it. `apply`, `with_session`, `open_files`, `settle`, `post_notice` take both in sequence, never nested; slow work outside.

`<data>/resume/`: `workspace.json` (layout version 2, counters, `kept`, per-tab `info`) and
`tabs/t-<n>/{state.json, deck/, templates/, agent/<scope>.json}`, each written by its session only;
`deck/` holds `deck.loro`, `document.json`, `assets/`, `fonts/`, `packs/`. Unreadable parts are set
aside as `*.failed-…`. Tabs restore lazily: the tab bar comes from `info` (ADR-0001).

## Dependencies

`oku-slides-`: `core`, `render`, `render-worker`, `pdf`, `platform`, `i18n`, `chart`, `a2ui`, `import`, `reference`; `fontdb`.
