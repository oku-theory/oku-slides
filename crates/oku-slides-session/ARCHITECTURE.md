# oku-slides-session

Editing session of one deck and its workspace (windows, tabs, resume), and the PowerPoint import.
The Tauri app and the dev server (`oku-slides-cli` `serve`) expose the same API.

## Session

- `Session` (`lib.rs`) answers `Request` through `Session::handle` (`request.rs`): one deck
  (`SourceBundle`, Loro replica, undo by the non-persisted `UndoManager`), its Typst rendering,
  templates, resume state (`resume.rs`, `resume_writer.rs`), file (`deck_file.rs`, `files.rs`, `merge.rs`).
- `Mode`: `Deck`, `Help` (read-only, `help.rs`), `Templates` and `History`; the last two set the
  deck aside (`stash`) in `templates_mode.rs` and `history.rs`. Edits go through `ensure_editable`.
- Packs and themes: the deck's embedded version is authoritative; a deck-only one (`PackScope::Deck`) never
  reaches the library (`add_version` refuses it). `deck_widgets.rs` builds the registry (`fall_back_to_base`),
  `deck_pack.rs`, `deck_theme.rs` embed defaults; `library.rs`, `library_kind.rs` (received marks), `library_index.rs`
  (in-memory index, reset when `generation` changes; loads re-hash) hold `Library<K>`; `theme.rs`, `pack.rs`.
- Theme editing: `theme_edit.rs` (`ThemeEdit` draft in templates mode, `save_theme_edit`),
  `deck_theme_edit.rs` (`edit_theme` in deck mode), `theme_schema.rs` (token controls, bounds),
  `theme_panel.rs` + `theme_controls.rs` (the `host-theme` surface), `appearance.rs`.
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

`Workspace` + `WorkspaceRequest` / `WorkspaceView` (`workspace.rs`): windows `w-<n>`, tabs `t-<n>`,
detached agent. `Slot` (`workspace_slot.rs`: `Ready`, `Dormant`, `Closed`) holds a tab's session;
the other `workspace_*.rs` files cover tabs (`Opening`; `ImportTab` builds an imported tab, outcome `Tune`
names it for the agent), windows, layout, resume, same deck, agent and help. Lock order: `Mutex<Workspace>`,
then a tab's `Arc<Mutex<Slot>>`, never the reverse; only `try_lock` a session under the workspace lock.
`apply`, `with_session`, `open_files`, `settle`, `post_notice` take both in sequence, never nested; slow work outside.

`<data>/resume/`: `workspace.json` (layout version 2, counters, `kept`, per-tab `info`) and
`tabs/t-<n>/{state.json, deck/, templates/, agent/<scope>.json}`, each written by its session only;
`deck/` holds `deck.loro`, `document.json`, `assets/`, `fonts/`, `packs/`. Unreadable parts are set
aside as `*.failed-…`. Tabs restore lazily: the tab bar comes from `info` (ADR-0001).

## Dependencies

`oku-slides-`: `core`, `render`, `render-worker`, `pdf`, `platform`, `i18n`, `chart`, `a2ui`, `import`, `reference`; `fontdb`.
