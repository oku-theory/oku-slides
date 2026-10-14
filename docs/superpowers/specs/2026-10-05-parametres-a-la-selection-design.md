# Parameters on selection — design

Date: 2026-10-05. Parent task: t-548 (subtasks t-549 to t-553) (see the plan `docs/superpowers/plans/2026-10-05-parametres-a-la-selection.md`).

## Goal

A widget's parameters are edited like a chart's data: when a
widget is the target, a panel on the right shows all its settable parameters, a
list as a table (`ListTable`). Adding an item on the slide (the "+") goes away:
items are added through the table.

## Behavior

- **Target**: the selected element; with nothing selected, the slide's widget.
- **Panel**: the "Paramètres de *nom*" (Parameters of *name*) card, at the top of the notes column
  (`aside.notes`), where `ChartPanel` mounts. One field per settable parameter:
  `plain` (text), `rich` (rich text), `number` (slider if bounds are valid, otherwise
  text), `choice` and `flag` (select), `list` (table).
  - Values read from the deck; when absent, the schema's `default`; never a sample
    or neutral value. A list absent from the deck is empty (as in rendering): empty table.
  - Parameter bound to a deck variable (`bind`): no field, a note line
    "*label* follows the variable *key*".
  - Non-settable types (`asset`, `slot`, `grid`, `host`): omitted, without a note.
  - No field (notes alone do not count): no panel.
  - A chart (widget with a `host(chart)` parameter) keeps `ChartPanel`, without a parameters panel.
  - Stale selection (element gone): the interface resolves the selection in the view;
    a vanished element is no longer selected and the panel shows the slide's one,
    without error. The session, for its part, returns no surface for an unknown element id.
  - Name shown: the `label` of the widget and of each parameter; without a `label` (base
    widgets), a translated name (`session-widget-kind-<kind>`, `session-param-<kind>-<nom>`),
    otherwise the raw name.
  - The selected chart does not ask the session for a parameters surface.
- **Modes**: deck and Templates (template slides). In Templates mode the "Ce
  modèle" (This template) card stays below the stage; the parameters panel is on the right. The widget
  workshop (`host-widget`) does not change and, when open, hides the right panel. No panel in read-only mode nor in
  the history.
- **Stage and workshop preview**: no more "+" (between items, before, after, in the
  center of an empty list). The "×" and ⋮⋮ handles remain.
- **Undo**: each setting is one step, "Réglage : *label*" (Setting: *label*); only successive
  settings of the same slider (dragged) merge into one step. Other values and
  list replacements never merge.
- **Insertion**: the table adds at the end of the list; insertion at a precise position
  (the "+" between items) goes away, the row is dragged afterwards.

## Architecture

Chosen approach: a settings surface built by the session (like
`host-template` and `host-widget`), a single type → field mapping.

### Session (Rust, `oku-slides-session`)

- `param_fields.rs` (new): `field()`, `column()`, `label()`, `slider_step()`,
  `settable()` moved from `widget_params.rs`, plus `coerce()` (pure conversion extracted from
  `checked_value`) and `schema_default()` (extracted from `preview_params`), shared between
  the workshop (sample values) and the target (deck values). No copy.
- New `target_params.rs`:
  - `Session::target_settings(slide, element) -> Result<Option<Surface>>`: surface
    `host-target` (title `session-widget-params`); `None` outside editing (history,
    help), target gone, without schema, chart, without field.
  - `Session::target_param(slide, element, key, value)`: `Op::SetParam`, value
    converted by `coerce` (empty text: clears); a list expects an array of items
    (paste); undo merging for a slider only (`FieldKind::Slider`).
  - `Session::target_list(slide, element, key, change: ListOp)`: `Op::EditList`.
  - Key: that of a shown field (`bind`, non-settable, unknown → `HostKey`); unknown
    slide → `NoSlide`. Validation by `apply` (core `Op` rejections, localized). One step,
    label `session-undo-param`. No new error variant.
- `panel.rs`: `HOST_TARGET = "host-target"`, `slide_palette()`. `Request::HostSettings`
  and `HostParam` unchanged; new `Request::TargetSettings { slide, element? }`,
  `TargetParam { slide, element?, key, value }`, `TargetList { slide, element?, key,
  change }`.
- i18n: `session-undo-param`, `session-target-bound` in the 20 languages.

### Interface (`ui/src`)

- `useHostSettings.ts`: the common cycle (read, keep if same content, read again after
  rejection, ignore a replaced surface) becomes `useSessionSurface`, used by
  `useHostSettings` and by `useTargetSettings.ts`, which reads the target's surface
  (element resolved in the view, otherwise the slide; inactive in read-only mode, history,
  workshop open) and sends to the target **of the displayed surface**: value →
  `targetParam`, `ListOp` → `targetList`, replacement → `targetParam` with the items.
- `TargetPanel.tsx`: the card at the top of `aside.notes` — `ChartPanel` for a
  chart, otherwise the `host-target` surface (`HostSettings.tsx`). `App.tsx` only
  wires.
- `api.ts`: `targetSettings`, `targetParam`, `targetList`.

### Removing the "+"

- `ItemLayer.tsx`: `plus`, `insert`, `INSERT_SIZE`, the "+" of an empty list.
- `stageItems.ts`: `within`, `onInserted`, `onEditWhenShown`; `insertPoints`
  stays (drag marker, `dropTarget`).
- `Stage.tsx`, `App.tsx`, `useFieldEditing.ts` (`editKeyWhenShown`),
  `WidgetTab.tsx`: props and state that existed only to open the inserted item.
- Rust: `first_text` of the list views (only reader: insertion).
- `styles.css`: `.item-insert`. i18n: `ui-undo-list-add` (20 languages, and the list
  in `oku-slides-i18n/src/lib_tests.rs`). Help `docs/aide/{en,fr}.json`.
- Tests and e2e: `ItemLayer`, `Stage`, `WidgetTab`, `stageItems`, `useFieldEditing`;
  `lists.cjs` adds through the panel's table, `lists-dock.cjs` loses the preview's "+".

## Errors

Rejected value (type, list bounds, size): `apply` rejects it (core `Op`
error), the panel reads the surface again (the field takes back the deck's value), error
message like the other settings. Unknown, bound or non-settable key, or vanished
element: `HostKey`; unknown slide: `NoSlide`. Reading a vanished target is not an
error: no panel.

## Tests

- Rust (`target_params_tests.rs`): fields by type for an element and for a
  slide; `bind` as a note; omitted types; `None` for a chart and without a parameter;
  `target_param`: value, flag, number, clearing, each `ListOp`, `replace`, a
  single labeled undo step, rejections (invalid value, unknown key, full list).
- UI (render): panel follows the selection; slide when nothing is selected;
  chart → `ChartPanel` alone; no panel in read-only mode; routing to
  `targetParam` and read-back after rejection; `ItemLayer` / `Stage` / `WidgetTab` without
  "+".
- e2e: `lists.cjs`, `lists-dock.cjs`; an e2e `target-panel.cjs` (selecting a
  timeline, adding a step in the table, the slide shows it, ⌘Z undoes it).
