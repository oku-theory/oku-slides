# Chart and table data as A2UI widgets in the agent area

## Goal

The settings band under the slide is gone (thumbnails strip in its place). A selected chart or
table still needs its editor: kind, stacking, series colours, table flags, column alignment,
chart/table switch and the data grid. It moves into the agent area as a host-only A2UI
surface, built by the session with the data inside; every edit goes back as an A2UI action
that the session turns into one undoable op.

Other elements (text, card, a slide's widget) keep no settings card: they are edited by
talking to the agent. The `target_*` session requests and the `host-target` surface go away.

## Catalog (`oku-slides-a2ui`, `ui/src/a2ui/catalog.tsx`)

Two `ComponentKind` variants, host-only (`agent_allowed` false, listed in the crate rule),
added to the Rust enum and the TS catalog in the same commit:

- `ChartData { kind, source, checkbox_columns, stacked, series, palette, waterfall_hint }`
  - `kind`: the first series type (`bar`, `line`, `pie`, `scatter`, `radar`, `waterfall`),
    or absent when the chart has no dataset.
  - `source`: `dataset.source` (row 0 = headers), cells `number | string | bool | null`.
  - `checkbox_columns`: columns edited as checkboxes (waterfall Total).
  - `stacked`: `Some(bool)` when stacking makes sense (2+ bar/line series), else absent.
  - `series`: `{ name, color }` per colourable series (`color` absent: palette).
  - `palette`: the theme's chart palette, valid hex only.
- `TableData { source, header_row, header_column, striped, align }`, `align` one of
  `auto | start | center | end` per column, trailing `auto` omitted.

Both carry `edit: Dynamic<Value>`, bound to `/edit` in the surface's data model, and one
`action: Action` named `data` whose context is `{ "edit": { "path": "/edit" } }`, the way
`ListTable` sends its `change`. The UI writes one edit at `/edit`, then fires the action; the
session refuses any edit that does not parse (`deny_unknown_fields`). Edits, tagged by `edit`:

| `edit` | context | op |
|---|---|---|
| `cells` | `change`: a `DataOp` | `SetData` |
| `kind` | `kind` | `set_chart_kind` (data converted) |
| `stack` | `stacked` | `SetChartOption` series patch |
| `color` | `index`, `color` (hex or null) | `SetChartOption` series patch |
| `flag` | `flag`, `on` | `SetTableOption` |
| `align` | `column`, `align` | `SetTableOption` align patch |
| `show_as` | `view` (`chart` / `table`) | `show_as` |

Patches (stack, colour, align) are computed in Rust from the deck, ported from today's
`chartDeck.ts` / `tableDeck.ts`; the UI computes no patch. Undo labels come from the session
(Fluent `session-undo-chart-*`, `session-undo-table-*`).

## Session (`oku-slides-session`)

- `HOST_DATA = "host-data"`, built by `forms` (one builder, per the crate rule): a column with
  the element's name and the `ChartData` or `TableData` component; `Place::Thread`.
- `Request::DataPanel { slide, element }` → `Option<Surface>`: `None` when the element is gone,
  has no chart/table host param (`host_params`), or the deck is not editable (history, help).
- `Request::DataEdit { slide, element, edit }` → applies the op, one undo step; consecutive
  `cells` edits of one cell merge like today's slider (`merge` key per cell).
- Removed: `TargetSettings`, `TargetParam`, `TargetList`, `target_params.rs`, `target_grid.rs`,
  `HOST_TARGET`, their tests, and Fluent keys left unused.

## UI

- `useDataSettings` (replaces `useTargetSettings`) on `useSessionSurface`: active when the
  selection's own panel (`ownPanel(host_params, kind)`) is `chart` or `table`, not read-only,
  not history, not the Theme tab. Key: slide, element, slide hash, undo labels. Every action
  goes to the surface's own scope, never the current selection (existing rule).
- `useSessionSurface` passes the action name to `send` so it can route `data` as well as
  `param`; `useHostSettings` keeps sending only `param`.
- The `host-data` surface joins the agent dock (live, host settings: not announced, folded
  behind "Settings" while a question waits). With the agent detached, the main window shows
  it in its right column, which is otherwise empty.
- Renderers: `ChartPanel` and `TablePanel` become views of the component's props (no
  `Element`), dispatching the action with one edit; `ChartDataGrid` is unchanged.
  `chartDeck.ts` keeps `starterChart`, `tableDeck.ts` `starterTable`; the readers and patches
  move to Rust. `api.ts` drops `chartKind`, `showAs`, `target*`; gains `dataPanel`, `dataEdit`.

## Errors

A refused edit (bad op, stale element) shows the session's localized error as a toast and
re-reads the surface (`useSessionSurface` sync), so the grid shows the deck's value again.

## Tests

- Rust: serde round trip and `agent_allowed` false for both variants; `render` refuses them;
  `DataPanel` for chart, table, other element, missing element, history; one test per `edit`
  (op applied, one undo step, label key); cell merge; the forms fixture contract
  (`every_form_matches_the_ui_fixture`) with a chart and a table surface.
- UI: catalog renders both components from the fixture and dispatches each edit's context;
  `useDataSettings` activation and scope; `AgentDock` shows `host-data` folded behind a
  question.
- e2e: `030-panel` edits a chart's data and kind and a table's flag from the agent area;
  `100-fields` loses its settings-band steps.
