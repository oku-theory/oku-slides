# Widget workshop: Scene and Board — design

> **Note of 2026-10-05 (t-541)**: the example cases are removed. No more
> pills above the preview (`ExamplePills`), no more named cases or
> kept current case (`UiState.case`), no more `add_example`,
> `rename_example`, `remove_example` requests. Each widget keeps **a single** set
> of example values in the pack (`pack.json`:
> `"examples": {"<widget>": {"<param>": valeur}}`), used by the editable
> preview, the "Paramètres de …" (Parameters of …) settings and the `set_widget_trial` tool.
> A `pack.json` with the old list of cases can still be read: the first case
> gives the values. The text below describes the state of t-497.

> **Note of 2026-10-04 (t-526)**: the Board is removed. No more Board
> view or Scene / Board toggle (`ExampleBoard`, `BoardSwitch`,
> `UiState.board`), no more preview look (`PreviewLook`: light / dark,
> 16:9 / 4:3) or "Champ vide" (Empty field) / "Déborde" (Overflows) alerts (`ExampleAlert`).
> What remains: the Scene (editing on the preview), the case pills and the
> settings under the preview. The text below describes the state of t-497.

Date: 2026-10-04 · Task: t-497

Mockup: https://claude.ai/artifact/E1NrXEUe4f5NFe5R4YJpRv (proposals A + C).

## Goal

In a widget sub-tab, the preview becomes the place where one sets
the example values, and one tests the widget on several **example
cases** at once:

- **Scene**: the preview at full size; its text fields are edited in place,
  with the field editing mechanism of the slide stage
  (double-click, `TextEditor`, Tab to move to the next one);
- **Cases**: named value sets ("Court" (Short), "Long", "Vide" (Empty)…),
  **saved in the pack** with the widget; one switches from one to
  the other through pills above the preview;
- **Board**: all the cases side by side, as thumbnails, with
  "Déborde" (Overflows) and "Champ vide" (Empty field) alerts; a click opens the case in the scene.

Prerequisites: t-474 (code placement and size), t-488 (settings under
the preview).

Out of scope: moving or resizing a field in the preview;
editing a non-text parameter in place (number, colour, boolean: it
stays in the settings card, t-488); several pack themes in
the board.

## Decisions

1. **Cases replace trial values.** The trials
   (`TemplateScratch`, lost when leaving templates mode) become the
   values of the **current case**, saved in the pack. A widget always
   has at least one case; the first is called "Exemple" (Example) and starts
   from the schema's neutral values (current behaviour).
2. **Storage in `pack.json`**, typed field `examples` of
   `PackManifest`: `{ "<widget>": [ { "name": "Court", "values":
   { "<param>": <valeur> } } ] }`. It is versioned, hashed and exported
   with the pack without changing the file format (separate files
   would break reading a deck, `sources.rs`). The values follow
   the `set_widget_trial` format (rich text: array of
   paragraphs). At most 12 cases per widget, unique names, 40 characters.
3. **Cases follow the widget**: renaming a widget renames its key,
   deleting it deletes its cases. A value the schema no longer accepts
   is ignored in the preview (like `trial_fits` today) and removed on the
   next save of the case.
4. **Current case outside the pack**: the displayed case is interface
   state, kept by the host in `UiState` (`case: Map<widget,
   nom>`), not in the pack.
5. **Writing**: each change to a case (value, name, addition,
   deletion) goes through the templates mode write path
   (`commit_files` → session version, one undo step).
   `commit_widgets` now also compares the manifest.
   `save_templates` keeps the session's `examples`.
6. **Preview with fields**: `preview_widget` also renders, for the requested
   case, a `SlideView` of the preview slide (`fields_on(1)` +
   `field_views` with the probed registry), and the crop offset
   (`bbox`) to place the fields on the cropped image.
7. **The scene reuses `Stage`** in "preview" mode: field layer
   and editing active, without object selection, dragging,
   resizing or grid. Applying a field sends
   `set_example_value(widget, cas, param, valeur)` instead of `Apply`.
   A field bound to the deck (`bind`) stays read-only: it shows the deck's
   value.
8. **Board**: a grid of thumbnails, one per case, rendered by
   `preview_widget` (one call per case, in parallel, at most 12); toggles
   Light / Dark and 16:9 / 4:3, specific to the view and not kept.
   Alerts computed by the host and returned with each preview:
   **Empty field** (text field without a value) and **Overflows** (content that
   exceeds the widget's frame: field box outside the area, or
   `layout` signalling an overflow).
9. **Scene / Board toggle** in the sub-tab toolbar,
   next to `</>`; Scene by default, choice kept in `UiState`
   (`board: bool`), shared by all widgets.

## Session (Rust)

- `PackManifest.examples: BTreeMap<WidgetFile, Vec<Example>>`,
  `#[serde(default)]`; `Example { name: String, values: Map<String, Value> }`.
- Requests: `add_example(widget, name, from?)`,
  `rename_example(widget, from, to)`, `remove_example(widget, name)`,
  `set_example_value(widget, name, param, value)`; they replace
  `set_widget_trial`, and `TemplateScratch.trials` goes away (the trials
  revision becomes the manifest's).
- `preview_widget(name, case, format)` → `WidgetPreview` gains
  `view: Option<SlideView>`, `offset: (x, y)` of the crop and
  `alerts: Vec<ExampleAlert>`.
- `UiState`: `case: BTreeMap<String, String>`, `board: bool`.

## Interface (TypeScript)

- `WidgetTab`: case pills (add, rename on double-click,
  delete on hover with confirmation); Scene = `Stage` in preview
  mode on the cropped image; Board = `ExampleBoard`.
- `useFieldEditing` reused as is; `Stage` gains a prop
  `mode: "deck" | "preview"` that turns off selection, dragging and grid.
- The settings card (t-488) stays for non-text parameters and
  follows the current case.

## Tests

- Rust: `examples` JSON round trip and version hashing; renaming
  and deleting a widget; out-of-schema values ignored; the
  requests refused outside templates mode; `preview_widget` renders
  fields with the right boxes and "vide" (empty) / "déborde" (overflows) alerts.
- UI: pills (add, rename, delete); editing a field
  in the scene → `set_example_value`; board: one thumbnail per case,
  alerts, click → scene on that case.
- e2e: create a widget, add a "Long" case, type a long title
  in the scene, see "Déborde" in the board; relaunch and
  find the cases again.

## Breakdown

1. Session: `examples` in the manifest, requests, tracking of
   renames and deletions, removal of the trials.
2. Session: preview with `SlideView`, offset and alerts.
3. UI: case pills and settings card wired to the case.
4. UI: `Stage` in preview mode, in-place editing.
5. UI: board, Scene / Board toggle, `UiState`.
6. Labels (20 languages), e2e.
