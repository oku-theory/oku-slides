# Pack editor layout: slim list, side-by-side preview and form

Task t-1392. A widget pack tab today stacks a 16:9 preview over a settings card, with
the code squeezed beside the card, under a four-line centred header. The left column shows
16:9 thumbnails that are mostly blank for element widgets. An element widget fills the
whole grid of the preview slide, so a card renders as a thin strip of tiny text in a large
white frame. The form shows raw parameter names (`kicker`, `highlight`) and a Yes/No
menu for flags.

This spec reorganises the left part of a **widget** pack tab (the agent column is
unchanged), adds a preview slot picker, and makes the form readable.

## Decisions (made on 2026-10-31)

1. **Agent column untouched (user).** Only the pile and the sheet area change.
2. **Widget packs only (user).** Layout packs keep their slide thumbnails, their foldable
   section and today's layout; the slot picker is not shown there.
3. **Slot picker: full, ½, ⅓ (user).** One choice for every widget, kept by the host like
   `code`; full by default.
4. **Form content in scope (user).** Localized labels and help per parameter; a switch for
   flags.
5. **Code replaces the form.** Shown, the code takes the form's track, and the form moves
   under the preview. Rejected: three tracks (preview, form, code) side by side, too narrow
   next to the agent column; hiding the form while coding, which would stop editing
   examples while writing code.

## Amendments (made while planning, 2026-10-31)

These override the sections below where they differ
(plan: `docs/superpowers/plans/2026-10-31-pack-editor-layout.md`).

1. **Built-in labels and help in Fluent, not in Typst.** Built-in widgets already take their
   names from Fluent in 20 locales (`session-widget-kind-<pack>-<name>`). Their parameters
   follow suit: `session-widget-param-label-<pack>-<widget>-<param>` and
   `session-widget-param-help-…`. The Typst `label:`/`help:` dictionaries serve other packs.
   The `base` pack's Typst sources do not change.
2. **Side by side in every pack tab.** `WidgetTab` is shared by widget and layout packs, so
   the new grid applies to both; only the slim list and the slot picker are for widget packs.
   Decision 2 above was about the left column.
3. **Slim list: name only, 180 px.** All items of a widget pack are element widgets, so a
   category icon tells nothing. The list keeps today's minimum pile width (`PILE_MIN`, 180 px)
   instead of a new bound; it is a `rows` variant of `SlidePile`, keeping drag and keys.
4. **Header stays above the sheet.** The table's grid has no row above the pile column; the
   one-line header fills the `label` area above the sheet, and the finish button stays in its
   `.tray` at the end of the same row.
5. **Settings title** uses the widget's shown name (`kind_name`) instead of its file name.
6. **The header wraps when it does not fit.** It is one row when it fits; on a narrow sheet the
   preview controls (« Aperçu : », theme and slot menus) wrap as one group to a second line, at its end.

## 1 · Layout (UI)

The sheet area of a widget pack tab (`.widget-tab`, `ui/src/pack.css`) becomes:

```
code hidden:  "preview split side"      columns: minmax(0,1fr) 12px var(--side-w, 300px)
code shown:   "preview split code"      columns: minmax(0,1fr) 12px var(--code-w, minmax(0,1fr))
              "side    split code"      rows: minmax(0,1fr) fit-content(40%)
```

- Preview on the left, full height (code hidden) or above the form (code shown).
- The divider (`CodeDivider`, `codeSplit.ts`) sizes the right track in both states. Code
  shown, it keeps `codeWidth`, as today. Code hidden, the form keeps a fixed 300 px: no new
  kept width.
- Hidden, the editor stays mounted and inert, as today.
- Commun (the shared file) has no form and no preview: its list of widgets in error takes
  the preview track, and the code its track when shown.

The pile of a widget pack (`PackPile` in `PileColumn`, `widgets` slot) becomes a **slim
list**, about 120 px wide:

- One row per item: a category icon, the item's name, the remove button shown on hover or
  focus only. The selected row has the accent background, like `.pack-pile-item` today.
- Commun first, then the items in pile order (t-1249), then the add button and its name
  field.
- No thumbnails: `PackPile` stops asking `svg` for widget packs. Gestures (select, rename
  with F2, duplicate, reorder by drag, remove with confirmation) keep working on rows.
- The column's width default shrinks for widget pack tabs only; layout packs keep theirs.

## 2 · One-line header

`PackPanels` renders one row, left-aligned above the list and the sheet area:

`[brand] [title field] [caption: N widgets · saved]  ……  Preview: [theme ▾] [slot ▾] [✓]`

- The finish button joins the row instead of floating in its own tray.
- "Preview:" labels the theme menu (new Fluent key, English reference plus French).
- The code switch stays in the bottom toolbar (`ToolsTray`).
- Layout pack tabs get the same row, without the slot picker.

## 3 · Preview slot picker (Rust + UI)

- `preview_widget` takes `span: "full" | "half" | "third"` (default `full`); the TS client
  passes it from the kept UI state.
- In `widget_preview.rs`, the preview element's cell becomes
  `Cell { col: 0, row: 0, cols: ceil(grid.columns × part), rows: grid.rows }`, with
  `part` 1, ½ or ⅓, and at least one column. The `bbox` crop follows unchanged.
- The agent's PNG preview stays full width.
- The picker is hidden for Commun and slide widgets; their previews ignore `span`.
- The kept UI state (`useKeptUi` and the state the host keeps for it) gains
  `previewSpan`, saved like `code` (ordered by page and seq).

## 4 · Form content (Typst + Rust + A2UI)

- **Typst.** The helpers of `typst/oku-slides/src/params.typ` take `help:` beside
  `label:`. Both accept a string or a dictionary by language (`(en: "…", fr: "…")`), as a
  widget's label does. The `base` pack declares English and French labels and help for
  every parameter, drawn from the `///` comments of each widget.
- **Rust.** `ParamSchema.label` becomes `Option<LocalizedText>`; `help: Option<LocalizedText>`
  is added, `skip_serializing_if = "Option::is_none"`. A plain string still parses, so
  existing packs and their digests are unchanged. `param_fields::label` resolves in the UI
  locale (`LocalizedText::pick`), then falls back to the name; `ParamField` gains `help`.
- **Form.** The help renders under its field, in muted text.
- **Switch.** A new A2UI component `Switch` (bound boolean, label, optional help). A
  `flag` maps to it instead of the Yes/No `Select`. `ComponentKind` in Rust and
  `ui/src/a2ui/catalog.tsx` change in the same commit, with its zod schema and a playbook
  variant. The `session-widget-param-yes` / `-no` keys go when nothing else reads them.

## Testing

- **UI render (happy-dom):** the widget tab's tracks with code hidden and shown; the slim
  list's rows, hover remove, selection; the header row with and without the slot picker.
- **Rust:** the preview cell for each `span` (rstest: full, half, third, with a 12-column
  and an odd grid); `span` ignored for slide widgets; `label` and `help` parsed as a string
  and as a dictionary; resolution in a non-English locale; `flag` mapped to `Switch`.
- **A2UI:** `Switch` round trip in Rust and zod.
- **e2e:** a new spec that switches the slot to ½ and checks the preview image narrows,
  then toggles the code and checks the form moves under the preview.

## Out of scope

- Layout pack tabs' left column and preview.
- Multiple example sets (variants) per widget.
- Per-widget default slot declared in Typst.
