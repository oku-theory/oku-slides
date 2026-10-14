# Position and size of a widget's code, kept by the host — design

> **Note of 2026-10-04 (t-526)**: the code is now always to the right
> of the preview. The placement toggle (`CodePlaceSwitch`, `UiState.code_place`),
> the code below and its height (`code_height`), the fallback below for lack of
> width and the divider axis are removed; what remains is the width set with the
> divider (`code_width`) and the `</>` switch; the preview keeps at least
> 240 px (the code, 120). The text below
> describes the state of t-474.

Date: 2026-10-04 · Task: t-474

## Goal

In a widget sub-tab, the expanded code (t-461) today always takes
the bottom three quarters, below the preview. We want to give it more
room, on demand:

- place it **below** (as today) or **to the right** of the preview;
- set its **size** with the divider between the preview and the code;
- **keep** position and size in `UiState`, restored at launch,
  like the widths of the stack and the thread (`pile`, `notes`).

Prerequisite: t-461 (code collapsed by default, `UiState.code` field).

Out of scope: the "Stage + Board" redesign mocked up on 2026-10-04
(editing on the preview, example cases); a size per widget; a
size setting when the code is collapsed.

## Decisions

1. **A choice shared by all widgets**, like `code`: a single
   placement and one size per placement, not a per-file setting.
2. **Two sizes kept, one per placement**: the height of the code
   below and its width on the right. Switching from one to the other restores the
   size already chosen for that placement.
3. **In px, like `pile` and `notes`**; `None`: the default proportion
   (below: preview 1 / code 3, as in t-461; on the right: half /
   half). The kept value is clamped on display: the preview and the
   code each keep at least 120 px, whatever the window.
4. **Toggle in the toolbar**, right next to `</>` and
   visible only when the code is expanded: a toggle button
   « Code à droite » ("Code on the right", `aria-pressed`). No extra menu entry.
5. **Divider between preview and code**, built like `ColumnDivider`:
   pointer dragging (no HTML5 drag and drop), keyboard arrows
   in steps of 16 px, double-click to go back to the default size.
   Horizontal below (↑/↓), vertical on the right (←/→, reversed in RTL).
   While dragging, the size only touches the DOM; it is only
   committed (and saved) on release. Absent when the code is collapsed.
6. **Fallback for lack of room**: if the sub-tab is less than 560 px
   wide, "on the right" is displayed below (container query). The kept
   choice does not change; the divider follows the displayed layout.
   (Threshold first planned at 720 px: between the stack and the thread, the sheet is
   only 644 px wide in a 1600 px window, so "on the right" would
   almost never have been used.)
7. **Transition**: expanding / collapsing keeps the t-461 animation;
   changing the placement is not animated (no grid animation between
   rows and columns).

## Session (Rust)

`UiState` (`crates/oku-slides-session/src/views.rs`) gains three fields,
`#[serde(default)]` — an old `state.json` is read back as is:

- `code_place: CodePlace` — `Below` (default) or `Right`, serialized as
  `"below"` / `"right"`;
- `code_height: Option<u32>` — height of the code below, in px;
- `code_width: Option<u32>` — width of the code on the right, in px.

Nothing else on the host side: no bounds (they depend on the window and
are applied by the UI), `sanitized` unchanged.

## Interface (TypeScript)

- `api.ts`: `UiState` takes over the three fields (`code_place: "below" |
  "right"`, `code_height` / `code_width: number | null`).
- `useViewOptions`: state `codeLayout: { place, height, width }`, restored
  from the first snapshot, saved with the rest; `setCodeLayout`.
- `CodePlaceSwitch.tsx`: the toggle button (decision 4), label
  `ui-widgets-code-right`.
- `CodeDivider.tsx`: the divider (decision 5), label
  `ui-divider-code`. It sets `--code-h` / `--code-w` on the
  `.widget-tab`, measures the computed grid to know the displayed axis
  (decision 6) and clamps the size (decision 3) with a pure function
  `fitCode(total, size)` tested on its own.
- `WidgetTab`: `data-place="below|right"` next to `data-code`, and the
  divider between preview and code when the code is expanded.
- `styles.css`: `.widget-tab` becomes `container-type: inline-size`;
  below, `grid-template-rows: minmax(120px, 1fr) var(--code-h, minmax(0, 3fr))`;
  on the right, columns `minmax(120px, 1fr) var(--code-w, minmax(0, 1fr))`;
  `@container (min-width: 560px)` allows the right-hand layout.
- Labels, in the 20 languages: `ui-widgets-code-right` (« Code à
  droite », "Code on the right"), `ui-divider-code` (« Taille du code », "Code size").

## Tests

- Rust (`views_tests.rs`, `resume_tests.rs`): a `state.json` without the
  fields is read back with `Below` / `None` / `None`; the three fields make
  the JSON round trip with the values `"below"` / `"right"`.
- `fitCode`: lower bounds on both sides, window too small for
  both minimums (half / half), `null` value untouched.
- `CodeDivider.render.test.tsx`: orientation and `aria-orientation`
  according to the axis; arrows → `onChange` with ±16 px; double-click →
  `null`; drag → DOM only, then `onChange` on release.
- `CodePlaceSwitch.render.test.tsx`: `aria-pressed`, toggle.
- `useViewOptions`: restoring the three fields and saving after a
  change.
- `WidgetTab`: `data-place` follows the choice; no divider when the code is collapsed.
- e2e (with temporary `OKU_SLIDES_PATHS__DATA` / `__DECKS`): switch
  to the right, drag the divider, relaunch; placement and width
  restored.
