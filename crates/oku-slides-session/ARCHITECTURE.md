# oku-slides-session

Editing session of one deck, its workspace (windows, tabs, resume), the PowerPoint import; the app and `serve` expose it.

## Session

- `Session` (`lib.rs`) answers `Request` through `Session::handle` (`request.rs`): one deck (`SourceBundle`, Loro replica, undo by
  the non-persisted `UndoManager`), its rendering, pack mode, resume (`resume.rs`, `resume_writer.rs`), file (`deck_file.rs`, `files.rs`, `merge.rs`).
- `Mode`: `Deck`, `Help` (`help.rs`), `Pack`, `History`; the last two set the deck aside (`stash`): `history.rs`, `pack_mode.rs` (a slide
  `layout-<name>` per layout, preview of its example), `layout_examples.rs` (slide op → example), `pack_mode_save.rs`, `pack_library.rs`, `pack_mode_previews.rs`.
- Packs (`deck.widgets`, `deck.layouts`; a namespace in one kind only) and themes: embedded versions are authoritative, a deck-only
  one never reaches the library. Built-ins `base` (widgets) and `essentiel` (layouts), sources from render, pinned by producers
  (new deck with no pin: `adopt_default_packs`; help; imports, built-in version; pack mode), never on open; current built-ins
  resolve in-process (`deck_widgets.rs`, `probed_widgets`: `builtin_registry` for exactly both), no packs `WidgetRegistry::default()`.
  `deck_fallback.rs` never disables a shipped built-in; `deck_closure.rs` (`requires`, dependents, `examples_requires`); `deck_pack.rs`,
  `deck_theme.rs` embed defaults; `library_packs.rs` (`PackLibrary`: one `Library<K>` per kind of `library_kind.rs`, namespace
  unique across kinds, `deck`/`deck-layouts` reserved, built-in ones never owned by a user copy; `namespace_slug.rs`), `library.rs`, `library_index.rs` (in-memory).
- Theme editing: `theme_edit.rs` (pack-mode draft, `save_theme_edit`), `deck_theme_edit.rs` (`edit_theme` in deck mode),
  `theme_schema.rs` (controls, bounds), `theme_panel.rs` + `theme_controls.rs` (`host-theme`), `theme_picker.rs` (`host-theme-picker`:
  bounded thumbnails, swatches, `pick_theme`), `appearance.rs`; `deck_themes.rs`: deck-only themes (`Snapshot.deck_themes`), after the library.
- Workshop: `widgets_edit.rs`, `widget_preview.rs` (working deck's packs), `widget_examples.rs`, `widget_params.rs`, `param_fields.rs`
  (Fluent names of built-in kinds), `items.rs`, `widgets_deck.rs` (`deck`/`deck-layouts` by category); `panel.rs`, `target_params.rs` + `target_grid.rs` (pack-mode slide grid).
- Views: `SlideView.fields` (`fields.rs`), markdown (`text.rs`, `inline.rs`), `search.rs`, `tables.rs`; `Snapshot.host_params`.
- Rendering: one `RenderWorker` per `Session`, one `RenderSlot` per render; a tab session starts unrendered, its worker at the first render.
- `import/`: PowerPoint import, CLI and app (one history step; a retrofit pins `base` and its layout packs, scored by their
  profiles; hi-fi makes a deck-only `deck-layouts`), `render`, `note_overflows`, `reference`, `state` (`ImportState`, tab memory),
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
`tabs/t-<n>/{state.json, deck/, pack/, agent/<scope>.json}`, each written by its session only;
`deck/` holds `deck.loro`, `document.json`, `assets/`, `fonts/`, `packs/`. Unreadable parts are set
aside as `*.failed-…` (state version 2; an older unsaved tab: `notice-tab-set-aside`). Tabs restore lazily from `info` (ADR-0001).

## Dependencies

`oku-slides-`: `core`, `render`, `render-worker`, `pdf`, `platform`, `i18n`, `chart`, `a2ui`, `import`, `reference`; `fontdb`.
