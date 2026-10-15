# oku-slides-session

Editing session of one deck, and the workspace around it: windows, tabs (one session per tab),
resume. The Tauri app and the dev server (`oku-slides-cli` `serve`) expose the same API.

## Session

- `Session` (`lib.rs`) answers `Request` through `Session::handle` (`request.rs`): one deck
  (`SourceBundle`, Loro replica, undo by the non-persisted `UndoManager`), its Typst rendering,
  its templates and its resume state (`resume.rs`, written by `resume_writer.rs`).
- `Mode`: `Deck`, `Help` (read-only, `help.rs`), `Templates` and `History`; the last two set the
  deck aside (`stash`) in `templates_mode.rs` and `history.rs`. Edits go through `ensure_editable`.
- Files: `deck_file.rs`, `files.rs` (`read_document`), `merge.rs` (`save_checked`, `merge_file`).
- Packs and themes: the version embedded in the deck's sources is authoritative. `deck_widgets.rs`
  builds the registry from it (`fall_back_to_base` on a faulty pack), `deck_pack.rs` and
  `deck_theme.rs` embed defaults; `library.rs` + `library_kind.rs` hold the versioned `Library<K>`
  (`Templates`; a received deck's versions are marked `received`), `theme.rs` `Themes`, `pack.rs` `PackInfo`.
- Theme editing: `theme_edit.rs` (`ThemeEdit` draft in templates mode, `save_theme_edit`),
  `deck_theme_edit.rs` (`edit_theme` in deck mode), `theme_schema.rs` (token controls, bounds),
  `theme_panel.rs` + `theme_controls.rs` (the `host-theme` surface), `appearance.rs`.
- Widget workshop: `widgets_edit.rs`, `widget_preview.rs`, `widget_examples.rs`, `widget_params.rs`,
  `param_fields.rs`, `items.rs`, `widgets_deck.rs`; host settings: `panel.rs`, `target_params.rs`.
- Views: `SlideView.fields` (`fields.rs`), editing markdown (`paragraphs_to_text`, `text.rs`,
  `inline.rs`; contract `fixtures/markdown*.json`), `search.rs`, `show_as` (`tables.rs`).
- Rendering: one `RenderWorker` per `Session`, one `RenderSlot` per render; a tab session starts
  unrendered (`Session::unrendered`), its worker process at the first render (`ensure_rendered`).
- `agent/`: what the session offers the agent. `tabs` (`Sessions`, `TabSession`), turns as undo
  groups (`begin_turn`; `turn_undo` names the turn atop the deck undo stack for its receipt), A2UI
  surfaces (`forms`, `turn_view`, `receipt`, `render`, `surface_ops`), `address`, `variants`,
  `context`, `scope` + `conversation` (per-scope threads; `ScopeMove` on widget rename, replayed by
  `undo.rs`), `reason` (`AgentFormReason`).

## Workspace

`Workspace` + `WorkspaceRequest` / `WorkspaceView` (`workspace.rs`): windows `w-<n>`, tabs `t-<n>`,
detached agent. `Slot` (`workspace_slot.rs`: `Ready`, `Dormant`, `Closed`) holds a tab's session;
the other `workspace_*.rs` files cover tabs (`Opening`), windows, layout, resume, same deck, agent
and help. Lock order: `Mutex<Workspace>`, then a tab's `Arc<Mutex<Slot>>`, never the reverse;
only `try_lock` a session under the workspace lock. `apply`, `with_session`, `open_files`,
`settle` and `post_notice` take both in sequence, never nested; slow work runs outside them.

`<data>/resume/`: `workspace.json` (layout version 2, counters, `kept`, per-tab `info`) and
`tabs/t-<n>/{state.json, deck/, templates/, agent/<scope>.json}`, each written by its session only;
`deck/` holds `deck.loro`, `document.json`, `assets/`, `fonts/`, `packs/`. Unreadable parts are set
aside as `*.failed-…`. Tabs restore lazily: the tab bar comes from `info` (ADR-0001).

## Dependencies

`oku-slides-core`, `oku-slides-render`, `oku-slides-render-worker`, `oku-slides-pdf`,
`oku-slides-platform` (storage, settings), `oku-slides-i18n`, `oku-slides-chart`, `oku-slides-a2ui`.
