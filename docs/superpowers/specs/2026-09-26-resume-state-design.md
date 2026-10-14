# Resuming the application state — design

Date: 2026-09-26

## Goal

Quitting then relaunching the app brings back exactly the same state: open deck
(even unsaved), current slide, theme (carried by the deck), template
editing mode, grid preference, window size and position.
The state is saved on every change.

> Update (t-188, [ADR-0002](../../adr/0002-sources-du-deck-en-crdt-loro.md)):
> the undo stack is no longer part of what comes back on relaunch.
> The deck becomes the Loro replica again (`deck.loro`); its `UndoManager` is
> rebuilt empty on every open, as when opening a PDF. The
> sections below still describe the original JSON history (v1) where
> the rest has not changed; the format of the files in `deck/` is that of
> the replica (see below).

Out of scope: dictation preferences (already in `<models>/config.json`),
language (`config.toml`, read-only).

## Architecture

- New module `crates/oku-slides-session/src/resume.rs` (tests:
  `resume_tests.rs`). Persistence is optional: only the Tauri app
  enables it, via `Session::resume_from(dir)`. The development server and
  the existing tests do not change.
- `Session::handle` saves the state after each successful command
  (`New`, `Open`, `Save`, `Apply`, `ChartKind`, `Undo`, `Redo`,
  `EditTemplates`, `SaveTemplates`, `CloseTemplates`, `Ui`). A failed
  write is logged (`warn`); the request is not failed.
- ~~`oku-slides-core`: `History` and `Transaction` derive `Serialize` /
  `Deserialize`~~ — obsolete since t-188: `History` is gone, replaced
  by the Loro replica (`DeckReplica`), serialized as a binary snapshot
  (`deck.loro`), not as JSON.

## Files

Under `<data>/resume/` (`<data>` = `settings.paths.data()`), written via
`Storage::write_atomic`:

```
resume/
  state.json        { version, mode, ui: { slide, grid } }
  deck/             the deck document (or the deck set aside while editing templates)
    deck.loro       full snapshot of the replica (oku-slides-core::DeckReplica), atomic write
    document.json   { path, dirty, saved_hash, blank } — written after deck.loro
    assets/<hex>    written if it does not exist; orphans deleted
    fonts/<nom>     same
  templates/        same shape; read only when `mode` is template editing
```

> Update (t-188): `deck/` no longer holds the deck or its history
> as JSON in `document.json`; the deck becomes a view of the replica read
> from `deck.loro` again. The write order (`deck.loro` then `document.json`)
> leaves a crash window: a new `deck.loro` can end up next to
> an older `document.json` (old `dirty` / `saved_hash`), since the old
> format wrote a single file. Minor consequence: at worst, a
> `dirty` or a `saved_hash` one write out of date; the deck itself stays
> consistent (`deck.loro` is always written in full).

- `mode`: `"deck"` or `{ "templates": { set, name, builtin } }`.
- `saved_hash`: sha256 (`asset_id`) of the `path` file at the time of
  opening or of the last save.
- `version`: `1`. An unknown version is treated as an unreadable state.
- Notices (`notice`) are not persisted: they are shown only once.
- Write: documents first (`deck.loro` then `document.json`),
  `state.json` next. A `Ui` request writes only `state.json`.

## Restore (app startup)

1. Read `state.json`, then `deck/` (and `templates/` if the mode says so).
2. Unmodified document, `path` present and file hash different from
   `saved_hash`: reopen from disk. Missing or unreadable file: the
   working copy is kept.
3. Otherwise: deck (replica read back from `deck.loro`) + `dirty` + `path` from
   the working copy. The undo stack is not part of the working
   copy: it always starts from zero (t-188, ADR-0002).
4. Unreadable templates document: back to the deck. Unreadable state, `deck.loro` or
   `document.json`: logged, blank deck.
5. File passed as an argument ("open with…"): it is opened, **unless**
   the restored state contains unsaved changes. In that case the
   restored work is kept and a notice (`notice`) reports it.

## UI ↔ session

- New request `Request::Ui { slide: Option<String>, grid: bool }`:
  the UI sends it when the current slide or the grid changes. No
  history, no render; returns the snapshot.
- `Snapshot` gains `ui: { slide, grid }`. On mount, `App.tsx` initializes
  `current` and `gridAlways` from this field.
- `localStorage["oku-slides.grid"]` is removed (no migration).
- Without resume (dev server), `Ui` keeps the state in memory only.

## Window

`tauri-plugin-window-state` (size, position, maximized). Saved on
`Moved` / `Resized` in addition to close.

## Tests (TDD)

- Core: replica round trip (`deck.loro`), see
  `crates/oku-slides-core/src/replica/`. ~~serde round trip of
  `History`; identical undo / redo afterwards~~ (obsolete, `History`
  disappeared in t-188).
- `resume_tests`: full round trip (deck, dirty, path, slide,
  grid); template editing mode; file modified elsewhere → read again
  from disk; corrupted state → blank deck; corrupted `deck.loro` →
  blank deck; orphan assets deleted; argument ignored if unsaved
  work.
- Integration `tests/session.rs`: session A modifies then is dropped;
  session B on the same directory finds the same snapshot; `undo` finds
  nothing to undo there (stack rebuilt empty, t-188).
- UI (`bun test`): initialization from `snap.ui`.
