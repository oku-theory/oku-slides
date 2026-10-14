# In-place editable fields per widget — design

Date: 2026-09-28 · Task: t-271 · Follows: t-270 (Typst widgets)

## Goal

Every text parameter (`plain`, `rich`) of a widget, slide included,
is edited in place, at the exact position of its rendered text. Empty fields
show a placeholder. Tab and Shift-Tab go through the fields of the slide.

Sub-project 2 of the program of spec `2026-09-27-widgets-typst-design.md`.
Sub-project 3 (t-272) hides the markdown marks and takes up per-run
style.

## Decisions

- **Empty fields take no room in the render.** An empty `field()` emits a
  marker; `fields-stack` turns it into its box (`empty`, `side`). An empty
  field does not change the layout: the export stays identical. The UI
  draws the placeholder on top.
- **Tab across the whole slide, in model order**: the fields of the
  slide in schema order, then the elements in slot order
  (depth first), and for each one its fields in schema order.
  Shift-Tab goes back. The order does not depend on the on-screen
  position.
- **One commit per field, one undo step per typing session.**
  Each exit from a field applies `set_param` and re-renders. The applications
  of the same session merge into one undo step.
- **Double-click to enter, click to move on.** A click on an
  element field selects its element; on a slide field, it
  selects nothing. A double-click edits the targeted field. While editing, a
  click on another field moves to it.
- **The editor exactly covers the rendered text.** t-271 guarantees the
  geometry: origin, baseline of each line, wrapping
  width, typography. A `plain` field, or a `rich` without marks,
  matches to the pixel. Text following a markdown mark drifts until
  t-272.
- **Two layers in the view**: `targets` (object boxes, to
  select and move) and `fields` (fields, to edit).
- **Selection**: the `@<parameter>` ids disappear from `targets`. The
  agent protocol keeps them to address a slide field
  (`ask_rewrite`, card anchors): the UI resolves them, by listing
  `@<field>` for each field of the slide among the possible anchors, and
  places the card under that `FieldView` (ruling R5).
- **Main field of an object**: where one writes without targeting a field
  (Enter, double-click outside a field, dictation outside editing, copy): the first
  non-empty field, otherwise the first `rich`, otherwise the first (Tab
  order). A card's kicker, often empty, does not come before the
  title. Dictation into a non-empty `rich` field is appended.

## Typst

### `field()` and empty fields

`field(ctx, name, body, style:)` accepts `body == none` or `""`. An empty
`field()` emits no box: it emits an unlabeled `metadata` marker
(`oku-empty-field: true, owner, field, style`), inert outside
`fields-stack`. `fields-stack` turns it into a `<oku-slides-field>` box
with `empty: true`, `h: 0` and `side`. A rendered but empty field
(`field(…, empty: true)`) keeps its measured box.

### `fields-stack`

A child of `stack`, even a zero-size `place()`, receives the
`spacing` (measured: 34.48 pt without a marker, 54.48 pt with a marker as a
separate item, 34.48 pt with the marker glued to the next item).

`fields-stack(spacing:, ..items)` (in `widget.typ`, exported) glues
each marker to a neighboring item: at the head of the next item, with
`side: "before"`; in last position, at the tail of the previous one, with
`side: "after"`. The empty box is placed out of flow, at the current vertical
position, by `place(left, layout(…))`: the `layout` only serves to
read the available width (`aw`). The anchor of an empty field is therefore the top
(or the bottom) of its neighbor: `y` of that item, `x`/`ax`/`aw` of the available
area. No measuring of the items, and no `layout` around the `stack`:
wrapping the `stack` shifted the render by a fraction of a pixel (antialiasing
in the golden test).

Several consecutive markers (card without kicker or title, content
slide without title, cover without kicker or title) share the same
anchor: same `y`, same `side`. The UI stacks them (see Layers).

The card (`slides.typ`, `card`), `_slide-title`, `title-slide`, `chapter`
and `end-slide` replace their `if x != none { wrap(…) }` with an
unconditional `wrap` passed to `fields-stack`. The `typst/oku-slides` README
documents the rule for packs.

### `FieldBox`

`oku_slides_render::FieldBox` gains:

```rust
pub empty: bool,              // #[serde(default)]
pub side: Option<FieldSide>,  // Before | After; only if `empty`
```

## Session

### `Target`

`views::Target` becomes an element box again: `id`, `kind: &str` (the
element's widget; no slide variant), `free`, `x`, `y`, `w`, `h`.
It loses `param`, `text`, `ax`, `aw`, `style`. `Target::field` and the
`@<parameter>` ids disappear from `targets` (the agent keeps them, see
Decisions).

### `FieldView`

```rust
pub struct FieldView<'a> {
    pub owner: &'a str,           // id of the slide or the element
    pub owner_kind: OwnerKind,    // Slide | Element
    pub field: &'a str,           // parameter name
    pub kind: FieldKind,          // Plain | Rich
    pub widget: &'a str,          // kind of the object
    pub label: Option<&'a str>,   // schema label (the localized label
                                  // is computed in the UI)
    pub text: Cow<'a, str>,       // text as typed (bind resolved)
    pub empty: bool,
    pub side: Option<FieldSide>,
    pub x: f64, pub y: f64, pub w: f64, pub h: f64,
    pub ax: f64, pub aw: f64,
    pub style: Option<&'a TextStyle>,
}
```

`SlideView.fields: Vec<FieldView>` is stored in Tab order. The
reading-order traversal (slide, then slots depth first) is a
core function, next to `Slide::locate`, reusable by the agent and the
panel. The text of a `plain` comes from `field_text` (which resolves `bind`),
that of a `rich` from `paragraphs_to_text`.

A `FieldBox` with no `plain` or `rich` parameter of the same name in the schema
of its `owner` is ignored (`tracing::debug!`).

### Labels

The placeholder follows the rule: `ui-field` (UI), then the schema `label`,
then the name. `ui-field` is completed with `body`, `author`, `date`,
`organization` in the 20 languages.

### Merging undo steps

`Request::Apply` gains `merge: Option<String>`, a typing session key
created by the UI. In `replica/undo.rs`, an application with key `k`
is added to the top step if that step carries key `k` **and**
no other step has been pushed since; otherwise, it opens a
new step, marked `k`. The merged step keeps the label of its
first application; the UI always passes `ui-undo-edit-slide`. A merged
step grows with the session; Loro keeps `limite × 64` items, and a
step from which it dropped items goes away entirely.

Merging does not use `begin_group` / `end_group`: a group opened by
the UI could swallow an agent turn or stay open if the window
closes. The key makes each `Apply` self-contained. An interleaved `Apply` (agent,
another gesture) breaks the merge: ⌘Z never undoes a typing and an
agent change together.

## UI

### Stage layers (`Stage.tsx`)

1. Rendered SVG.
2. Objects layer (`targets`): selection, drag, handles, label. The
   sort by area (`ordered`) disappears.
3. Fields layer (`fields`), on top: transparent click zones. An
   empty field is one line high (line pitch from its `TextStyle`, see
   Geometry), above the anchor (`before`) or below it (`after`).
   Consecutive empty fields of the same object on the same anchor (same
   `y`, same `side`) stack top to bottom in Tab order:
   before the anchor, the first ones go up above the following ones; after it,
   the following ones go down below the first ones. Greyed placeholder, in the
   field's typography, drawn for the empty fields of the
   selected object or the one being edited; everywhere in templates mode. The
   resize handles go above this layer.
4. Editor: a `TextEditor`, `key` = `owner/field`, and its mask.

The open history (read-only) does not mount the fields layer.

### Gestures

| Gesture | Effect |
|---|---|
| Click (outside editing) | selects the element, even on one of its fields; a slide field selects nothing |
| Double-click on a field | edits it |
| Double-click on an object, outside a field | edits its main field (see Decisions) |
| Enter (element selected, outside editing) | edits its main field |
| Click on another field (while editing) | commits, moves to it, same session; the selection follows the element |
| Click outside a field (while editing) | commits and exits |
| Tab / Shift-Tab | commits, next / previous field (the selection follows the element); at the end, commits and exits |
| Enter (`plain`) | commits, next field |
| ⌘Enter | commits and exits |
| Escape | cancels the current field and exits |

### Sequence

1. On entering editing, the UI creates a session key.
2. On leaving a field: if the draft differs from the original text,
   `onApply(label, textOps, merge)` (`set_param`).
3. The targeted field is retained by its `(owner, field)` key, not by an index.
4. The targeted editor opens right away; its geometry follows each
   `SlideView`, it realigns when the render arrives.
5. On exit, the session key is forgotten.
6. Before a slide change, the stage commits the open typing (handle
   `StageHandle.commitEditing`), then editing closes.

### Geometry

`editorGeometry` always starts from a `FieldView`. `plain`: natural
width of the draft, bounded by `aw`, aligned within `ax`/`aw`. `rich`:
width of the area (`aw`), the one where Typst wraps. Line
pitch, Typst's: cap height of the font, measured per
font (`fontMetrics`, 0.7 em without layout), plus the leading.
First line on the Typst baseline (cap height), the CSS
baseline measured for the field's font. Without `TextStyle`, the editor
sits on the clickable box, in the interface font.

## Edge cases

- **Field gone from the same slide during editing** (deletion by
  the agent, merge, `SetKind`): nothing is committed, the draft is lost
  and a toast says so (`ui-toast-field-gone`). The editor closes.
- **Slide changed during editing** (agent focus, thumbnail): the
  draft is committed first, with the session key, editing
  closes, then the slide changes. No toast.
- **Concurrent editing of the same field**: Loro merge per character (`Prose`).
- **Bound field** (`bind`): written into `deck.params`; the slides that
  show it re-render.
- **Field overflowing** its object: edited at its position; the stage does not clip
  the editor.
- **Free canvas**: the Tab order stays the model's.
- **Dictation**: inserted into the open field, unchanged.
- **Third-party pack without `fields-stack`**: its empty fields
  cannot be reached (the marker is inert outside
  `fields-stack`); nothing detects it automatically, the rule is
  documented in the README.

## Out of scope

- Hiding markdown marks, per-run style (t-272).
- On-stage editing of non-text parameters (`flag`, `choice`,
  `number`…): inspector.
- `author` and `date` of the cover (single line, inspector).

## Tests

In TDD, in build order:

1. **Typst**: an empty `field()` emits an `empty` box without changing the
   height of a `fields-stack`; a card without kicker emits three
   `FieldBox`es, an empty `kicker` anchored `before` the title; the golden test stays
   identical.
2. **Core**: reading order on a slide with a nested `stack`.
3. **Undo**: two `Apply`s with the same key → one step, one `undo` undoes them;
   an interleaved `Apply` breaks the merge; a different key opens a
   step.
4. **Session**: `SlideView.fields` in Tab order; empty field listed
   with `empty` and its schema label; `bind` resolved; orphan
   `FieldBox` ignored.
5. **UI, pure tests** (`stageFields.test.ts`): navigation by key, edges,
   main field of an object, stacking of the empty fields of an anchor.
6. **UI, render**: double-click on a card title → editor on the
   title; Tab → the editor opens on the body and realigns on the
   next `SlideView`.
7. **Alignment** (Playwright, theme fonts loaded): on the
   fixtures, for each `plain` field and each `rich` without marks,
   the open editor overlaid on the SVG: left edge and baseline of
   each line within ≤ 1 px at scale 1, identical line breaks.
