# Widgets as sub-tabs in the templates perspective — design

Date: 2026-10-03 · Task: t-394

## Goal

In template mode, each widget file of the pack opens as a tab:
1 widget = 1 tab. This replaces the modal panel "Widgets du pack" (Pack widgets)
(`ui/src/WidgetWorkbench.tsx`, delivered by t-329, fixed by t-392 and t-390).

Prerequisite: t-390, merged into `main` (9bf45b3).

Out of scope: reordering sub-tabs by dragging, detaching a widget
into another window, keeping the editor's undo history after
a restart.

## Decisions

1. **Sub-tabs inside the deck's tab**, not a new kind of
   `Workspace` tab. A tab remains a `Session`; template mode is one of its
   states (`TemplateMode`). In template mode, the table gains a bar of
   sub-tabs: "Modèles" (Templates; always there, not closable), then the
   open widgets. `Workspace` and `TabInfo` do not change.
2. **`_commun` is a sub-tab like the others**, labeled "Commun" (Common).
   In place of the preview, it lists the widgets in error (those it may have
   broken), each one clickable. It cannot be renamed.
3. **Open**: from a "Widgets" section of the pile (under the
   template thumbnails), or from the label of the element selected
   on the stage when its widget comes from the pack. Double-clicking an
   element keeps editing its main field.
4. **Create, rename, delete**: all in the "Widgets" section of the
   pile. "+" in its header; rename by double-clicking the name or
   F2; delete with a button on hover, with confirmation.
5. **Sitting and versions unchanged**: the sitting is template mode; all
   sub-tabs write into the same sitting version. Closing a
   sub-tab ends nothing (its pending keystrokes are sent). **Undo /
   Redo** in a widget sub-tab act on the CodeMirror history of that
   file; each step goes through `set_widget` again.
6. **No more panel in deck mode**: ☰ "Éditer les widgets" (Edit widgets) and
   the label of a pack widget element enter template mode
   (`edit_templates`). The deck-mode widget sitting
   (`widgets_deck.rs`) stays for the agent and MCP.
   In deck mode, the label knows that an element comes from the pack through
   the snapshot's `pack_widgets`; the statuses (and their probe) are only
   loaded on click, when entering template mode.
7. **Sub-tabs kept in `UiState`**: restored on restart, they
   follow the deck's tab from one window to another.

## Session (Rust)

- `UiState` gains two fields, `#[serde(default)]`:
  - `widgets: Vec<String>`: open sub-tabs, in opening
    order;
  - `widget: Option<String>`: active sub-tab; `None`: "Modèles".
- `UiState::sanitized(templates)` normalizes these fields: outside template
  mode, empty; in template mode, valid names (`WidgetFile::parse`),
  without duplicates, at most `MAX_WIDGET_TABS` (64), active among them. Called on
  the `ui` command (`Request::Ui`) and on resume. A deck therefore never
  resumes sub-tabs, even if the resumed templates are unreadable.
- `close_templates`, `edit_templates` (entry, or switch to another pack)
  and `load_document` empty both fields; the UI then sets its
  sub-tabs through `Request::Ui`.
- The snapshot gains `pack_widgets`: the widget files of the current
  pack, without `_commun`, read without a probe. The stage label uses it
  in deck mode.
- No new operation: `widget_status`, `widget_source`,
  `set_widget`, `rename_widget`, `remove_widget` and `preview_widget`
  are enough. The UI removes names that no longer exist in the pack.

## UI

### Breakdown

| Unit | Role |
|---|---|
| `useWidgetFiles` | Shared state of the pack, **active only in template mode** (no probe in deck mode): statuses and the `revision` for which they were read, **single queue** of operations (writes, renames, deletes, inherited from t-390), 400 ms debounce per file, "unsaved" marks, in-flight renames, trial statuses and keystroke counters **per file**. Reset on every pack change. Extracted from `WidgetWorkbench`. |
| `widgetTabs.ts` | Pure sub-tab logic: open (activates it if already open, otherwise appends it at the end), close (back to the left; left of the first, "Modèles"), rename (same id), prune, resume merged with the sub-tabs already open. |
| `useTemplateWidgets` | Assembles files and sub-tabs for `App`: resume once the pack statuses are read, pruning only on statuses of the current `revision`, `kept` for `UiState` (empty outside template mode from the same render), entry from the deck, ✓ (keystrokes written before `close_templates`), the sub-tabs' ⌘W and ⌘Z shortcuts. |
| `WidgetPane.tsx` | The mounted sub-tabs, only the active one visible. |
| `TemplateTabs.tsx` | Secondary bar (`tablist` pattern: ←/→, Home/End, Delete closes): click to activate, × or middle click to close. "Unsaved" mark on the label. |
| `WidgetTab.tsx` | One file: `TypstEditor`, cropped preview (`croppedSvg`) and diagnostics; for Commun, the list of widgets in error. Only the active sub-tab rereads its source and requests a preview; a hidden one rereads when it becomes active. |
| `PileWidgets.tsx` | "Widgets" section of the pile: Commun first, valid/error badges, "+" and inline name (`nameAvailable`), rename (double-click, F2), confirmed delete. |

The queue stays **single** for the whole pack (not one per sub-tab): a
rename from the pile must wait for a sub-tab's keystrokes, as
in the t-390 panel.

The open sub-tabs stay **mounted but hidden**: each keeps its
CodeMirror state, hence its undo history. Accepted limits:
the history is lost when switching deck tab (only the `App` of
the active tab is mounted), on restart, and when the agent changes the
file (the editor is recreated on its source). A replacement of the declared
name after a rename does not enter the history.

### Behaviors

- **Open** (pile, label): activates the sub-tab, or appends it at the end
  of the bar and activates it. From a deck: `edit_templates`, then opening
  of the sub-tab (label) or focus on the Widgets section (☰).
- **Active widget sub-tab**: the stage (hidden, kept mounted) and the
  add tray give way to the editor and the preview; the pile
  stays. Undo, Redo and ⌘Z/⌘⇧Z act on the file's editor;
  ⌘W closes the sub-tab (on "Modèles", ⌘W keeps its meaning). The
  stage shortcuts (Delete, arrows, Enter) are inactive. The title
  and ☰ do not change.
- **Close a sub-tab**: `flush` of its keystrokes, then the sub-tab on the
  left becomes active.
- **✓ Finish**: `flush` of all files, wait for the queue, then
  `close_templates`.
- **Rename**: the open sub-tab keeps its place under the new name;
  keystrokes typed during the rename follow (t-390 `moves` rule).
- **Delete**: confirmation ("Supprimer le widget « x » ?", Delete widget "x"?), then `remove_widget`; the open
  sub-tab closes. `WidgetInUse`: toast that names the templates, nothing
  closes.
- **Agent**: same operations. A `revision` change rereads statuses and
  sources, the local keystrokes in progress keeping priority (t-390 rules).
  A file deleted or renamed by the agent closes its sub-tab, without
  noise.
- **☰ menu**: in template mode, the "Widgets" entry disappears; in deck
  mode, it becomes "Éditer les widgets".

### Edge cases

- Unreadable source or statuses: error toast, the sub-tab stays.
- History open (read-only): sub-tab bar hidden.
- Restore: names absent from the statuses removed; active sub-tab
  gone → "Modèles".
- Pack without widgets: "Widgets" section with its "+" and the text
  `ui-widgets-empty`.

### Removals

`WidgetWorkbench.tsx`, its tests (`WidgetWorkbench*.render.test.tsx`) and
the helper `test/workbench.ts` (ported to the new units), the
`.workbench*` styles and the Fluent keys no longer needed (`ui-widgets-title`,
`ui-widgets-close`, `ui-widgets-pick`…), in the 20 languages.

## Tests

In build order, in TDD:

1. **Session**: `UiState` round trip with `widgets` and `widget`, default
   values for an old state; `close_templates` empties both
   fields; resume in template mode that returns them.
2. **UI, logic (bun)**: `useWidgetTabs` (open, reactivate, close with
   return to the left, rename followed, cleanup of vanished names, active
   vanished → Modèles); `useWidgetFiles` (write, trial and
   rename tests carried over from `WidgetWorkbench.writes`/`.trials`).
3. **UI, render**: `TemplateTabs` (activate, close, ⌘W); `PileWidgets`
   (badges, creation, rename, confirmed delete, `WidgetInUse`
   toast); `WidgetTab` (diagnostics at the line, Commun and its
   widgets in error, undo through the editor); stage label
   only for a pack widget; deck-mode ☰ that enters template
   mode.
4. **Regression**: pending keystrokes in a hidden sub-tab are
   saved by ✓ before `close_templates`.
5. **i18n**: new keys in the 20 languages, old ones removed
   (`i18n-keys.test.ts`).
