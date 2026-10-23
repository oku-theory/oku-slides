# Chart and table data as A2UI widgets — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Edit a selected chart or table from a host-only A2UI surface (`host-data`) in the agent
dock; every edit is an A2UI action the session applies as one undoable op.

**Architecture:** `oku-slides-a2ui` gains `ChartData` / `TableData` (host-only). The session
builds the surface from the deck (`DataPanel` request) and applies edits (`DataEdit` request),
computing the chart/table patches that the UI computed before. The UI fetches the surface with
`useSessionSurface`, shows it in the agent dock, and renders the two components with the
existing `ChartPanel` / `TablePanel` turned into prop-driven views.

**Tech Stack:** Rust (serde, snafu, speculoos, rstest), React + zod + `@a2ui/react` v0.9, Bun
tests, Playwright e2e, Fluent.

**Spec:** `docs/superpowers/specs/2026-10-23-chart-table-a2ui-design.md`

**Task:** t-1119 (sub-tasks t-1120 to t-1128, one per task below); the thumbnails strip that
precedes it is t-1118.

## Global Constraints

- Change the Rust `ComponentKind` catalog and `ui/src/a2ui/catalog.tsx` together, same commit.
- List every host-only component in `ComponentKind::agent_allowed` so `render` refuses it.
- Build settings forms only through `forms` (no second A2UI builder).
- Send surface actions to the scope of the displayed surface, never to the current selection.
- No `unwrap`/`expect` outside tests; Snafu errors with backtraces, Fluent messages.
- Rust unit tests in sibling `x_tests.rs`, speculoos, names `unit_scenario_result`.
- Every visible UI string through `t()`; English reference locale, every locale file updated.
- Files under 500 lines target, never over 1000.
- Commits `type(scope) : description (t-N)`, no Co-Authored-By line.

---

### Task 1: Catalog variants `ChartData` and `TableData`

**Files:**
- Modify: `crates/oku-slides-a2ui/src/message.rs` (enum, `actions`, `without_action`, `agent_allowed`)
- Test: `crates/oku-slides-a2ui/src/message_tests.rs`
- Modify: `crates/oku-slides-a2ui/ARCHITECTURE.md`, `crates/oku-slides-a2ui/AGENTS.md` (host-only list)

**Interfaces — Produces:**
```rust
/// Données d'un graphique, éditées dans le dock (hôte seul).
ChartData {
    #[serde(skip_serializing_if = "Option::is_none")] kind: Option<String>,
    source: Vec<Vec<Value>>,
    checkbox_columns: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")] stacked: Option<bool>,
    series: Vec<SeriesSwatch>,
    palette: Vec<String>,
    edit: Dynamic<Value>,
    action: Action,
},
/// Données et options d'un tableau, éditées dans le dock (hôte seul).
TableData {
    source: Vec<Vec<Value>>,
    header_row: bool, header_column: bool, striped: bool,
    align: Vec<String>,
    edit: Dynamic<Value>,
    action: Action,
},
pub struct SeriesSwatch { pub name: String, #[serde(skip_serializing_if = "Option::is_none")] pub color: Option<String> }
```

- [ ] Write failing tests: serde round trip of each variant (camel/snake case as the enum's
  `rename_all`), `agent_allowed()` false for both, `actions()` returns the one action,
  `without_action()` returns `None` (the component is useless without it).
- [ ] Run `cargo nextest run -p oku-slides-a2ui` — FAIL (unknown variants).
- [ ] Add the variants and helpers; `agent_allowed` lists them with `Prompt`, `Timeline`…
- [ ] Run again — PASS. Update ARCHITECTURE.md (component list, host-only list) and AGENTS.md rule.

### Task 2: Session builds the `host-data` surface (`DataPanel`)

**Files:**
- Create: `crates/oku-slides-session/src/data_panel.rs`, `data_panel_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod data_panel;`), `request.rs` (`DataPanel`)
- Modify: `crates/oku-slides-session/src/agent/forms.rs` (builder `data_surface`)
- Modify: Fluent `session.ftl` (all locales): `session-data-chart`, `session-data-table` (surface titles)

**Interfaces — Produces:**
```rust
pub(crate) const HOST_DATA: &str = "host-data";
impl Session {
    /// Surface `host-data` de l'élément ; `None` hors deck modifiable, élément absent ou sans graphique/tableau.
    pub(crate) fn data_panel(&self, slide: &str, element: &str) -> Result<Option<Surface>>;
}
// forms.rs
pub(crate) fn data_surface(title: String, data: ComponentKind) -> Surface; // column[title, data], Place::Thread, model {"edit": null}
```
Reading: the element's `host(chart)` param via `chart_param(&self.registry, &kind)`, parsed as
`oku_slides_chart::ChartOption` (series types, `dataset.source`, `stack`, `itemStyle.color`);
`host(table)` via `registry.get(kind).host_param(HostType::Table)` parsed with `parse_table`.
Port from `ui/src/chart/chartDeck.ts`: `checkboxColumns` (waterfall, width > 2 → `[2]`),
`stackable` (2+ series all bar/line) → `stacked: Some(all share one stack)`, `seriesNames`
(cartesian: header col i+1; scatter: header col 2i+1; else none), `hexColor` normalization
(`#rgb` → `#rrggbb`, lowercase), palette from `self.appearance` chart palette (valid hex only).

- [ ] Failing tests: chart element → surface id `host-data`, `ChartData.kind == "bar"`,
  source equals the deck's; stacked `Some(false)` for 2 bar series, `None` for 1; waterfall
  `checkbox_columns == [2]`; series colors normalized; table element → `TableData` flags and
  align strings; text element → `None`; missing element → `None`; history mode → `None`.
- [ ] Implement `data_panel.rs` and `Request::DataPanel { slide, element }` (read-only request,
  listed with the other read requests in `request.rs` classification matches).
- [ ] Tests PASS.

### Task 3: Session applies edits (`DataEdit`)

**Files:**
- Create: `crates/oku-slides-session/src/data_edit.rs`, `data_edit_tests.rs`
- Modify: `request.rs` (`DataEdit`), `lib.rs`
- Modify: Fluent `session.ftl` (all locales): `session-undo-chart-kind`, `-chart-data`,
  `-chart-color`, `-chart-stack` (`$stacked`), `-table-option`, `-table-align`,
  `-table-data`, `-show-as-table`, `-show-as-chart` — moved from the `ui-undo-*` keys with
  their translations.

**Interfaces — Produces:**
```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataEdit {
    Cells { change: oku_slides_chart::DataOp },
    Kind { kind: oku_slides_chart::SeriesType },
    Stack { stacked: bool },
    Color { index: usize, color: Option<String> },
    Flag { flag: TableFlag, on: bool },        // header_row | header_column | striped (kebab in JSON)
    Align { column: usize, align: oku_slides_chart::ColumnAlign },
    ShowAs { view: View },
}
impl Session { pub(crate) fn data_edit(&mut self, slide: String, element: &str, edit: DataEdit) -> Result<()>; }
```
Ops: `Cells` → `Op::SetData { target, param, change }` with merge key
`data:{slide}/{element}/{row}/{col}` for `SetCell` (else no merge); `Kind` →
`set_chart_kind`; `Stack` → `SetChartOption` patch `{series:[…]}` (stack `"total"` set or
removed on every series); `Color` → series patch with `itemStyle.color` set or removed (hex
validated, else `HostKey` error); `Flag` → `SetTableOption {"header-row": on}` etc.; `Align`
→ `{align: [...]}` with trailing `auto` trimmed, `null` when empty; `ShowAs` → `show_as`.

- [ ] Failing tests, one per edit: op applied (read the deck back), one undo step, label equals
  `localizer.tr("session-undo-…")`; two `SetCell` on the same cell → one undo step; a bad
  color → error; an unknown `edit` tag → deserialization error.
- [ ] Implement; `Request::DataEdit { slide, element, edit }` among the mutating requests.
- [ ] Tests PASS; the `fr` label test checks the key, not English text.

### Task 4: Remove the target settings path

**Files:**
- Delete: `crates/oku-slides-session/src/target_params.rs`, `target_params_tests.rs`, `target_grid.rs`, `tests/target.rs`
- Modify: `request.rs` (drop `TargetSettings`, `TargetParam`, `TargetList`), `panel.rs` (drop `HOST_TARGET`, doc), `lib.rs`, `lib_tests.rs`
- Modify: Fluent `session.ftl` keys no longer read (checked by the i18n crate tests)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md`, `AGENTS.md` where they cite target params

- [ ] Delete, build, fix every reference (`cargo build -p oku-slides-session --tests`).
- [ ] Remove helpers left dead in `param_fields.rs` (clippy `dead_code` tells).
- [ ] `cargo nextest run -p oku-slides-session -p oku-slides-i18n` PASS.

### Task 5: UI catalog components

**Files:**
- Modify: `ui/src/chart/ChartPanel.tsx` → `ChartDataView` props `{kind, source, checkboxColumns, stacked, series, palette, onEdit}`
- Modify: `ui/src/chart/SeriesColors.tsx` (props `series`, `palette`, `onColor`)
- Modify: `ui/src/table/TablePanel.tsx` → `TableDataView` props `{source, headerRow, headerColumn, striped, align, onEdit}`
- Modify: `ui/src/table/ShowAsButton.tsx` (unchanged API), `ui/src/chart/chartDeck.ts`, `ui/src/table/tableDeck.ts` (keep `CHART_KINDS`, starters, `ColumnAlign`; drop readers/patches)
- Create: `ui/src/a2ui/dataComponents.tsx` (zod APIs + `ChartData`, `TableData` implementations)
- Modify: `ui/src/a2ui/catalog.tsx` (register), `ui/src/a2ui/fixtures/agent-surfaces.json` (regenerated in Task 6)
- Test: `ui/src/a2ui/dataComponents.render.test.tsx`; update `ChartPanel`/`TablePanel` render tests

**Interfaces — Produces:**
```ts
export type DataEdit =
  | { edit: "cells"; change: DataOp } | { edit: "kind"; kind: ChartKind }
  | { edit: "stack"; stacked: boolean } | { edit: "color"; index: number; color: string | null }
  | { edit: "flag"; flag: TableFlag; on: boolean } | { edit: "align"; column: number; align: ColumnAlign }
  | { edit: "show_as"; view: View };
```
Renderer: `onEdit = (e) => { props.setEdit(e); return sendWithOutcome(outcome, () => props.action()); }`.

- [ ] Failing render tests: `ChartData` from fixture props shows the kind toggles pressed for
  its kind; clicking "line" writes `{edit:"kind",kind:"line"}` then fires the action; a cell
  edit writes `{edit:"cells",change:{op:"set_cell",…}}`; `TableData` flag toggle writes
  `{edit:"flag",flag:"striped",on:true}`; zod rejects an unknown prop.
- [ ] Implement; PASS.

### Task 6: Contract fixture

**Files:** `crates/oku-slides-session/src/agent/forms_tests.rs` (`every_form` adds a chart and a
table `data_surface`), `ui/src/a2ui/fixtures/agent-surfaces.json`, `ui/src/a2ui/catalog.test.tsx`

- [ ] Add both surfaces to `every_form`; regenerate with `OKU_UPDATE_FIXTURES=1`; the UI
  catalog test parses every fixture surface with the zod schemas — PASS on both sides.

### Task 7: UI wiring

**Files:**
- Create: `ui/src/useDataSettings.ts`, `ui/src/useDataSettings.render.test.tsx`
- Modify: `ui/src/useHostSettings.ts` (`send` gets the action name; `useHostSettings` keeps `param`)
- Modify: `ui/src/api.ts` (`dataPanel`, `dataEdit`; drop `chartKind`, `showAs`; `HOST_DATA` id)
- Modify: `ui/src/App.tsx` (hook, `onHostAction`, surface into `AgentPanel`), `ui/src/agent/AgentPanel.tsx` (`hostSurfaces` prop merged at the head of the dock; detached: rendered in the main window column)
- Modify: `ui/src/agent/surfaces.ts` (doc of `isHostSettings`: `host-data`)
- Test: `ui/src/agent/AgentPanel.render.test.tsx` (dock shows `host-data`, folded behind a question)

**Interfaces — Consumes:** `api.dataPanel(slide, element): Promise<Surface | null>`,
`api.dataEdit(slide, element, edit: DataEdit): Promise<Snapshot>`.

- [ ] Failing tests: hook inactive for a text element, read-only, history; active for a chart
  (fetches `dataPanel(slide, element)`); an action from a replaced surface sends nothing; a
  `data` action sends `dataEdit(scope.slide, scope.element, context.edit)`.
- [ ] Implement; PASS; `bunx tsc -b` clean.

### Task 8: e2e, help, docs

**Files:** `ui/e2e/030-panel.cjs`, `ui/e2e/100-fields.cjs`, `docs/aide/*.json` (if they mention
the settings band or the left thumbnails), `ui/AGENTS.md` (ownPanel rule now routes the
data surface), crate ARCHITECTURE.md files touched.

- [ ] Rewrite `030-panel`: select the chart, edit a cell and switch to "line" from the dock;
  select the table, toggle "Striped rows". Trim `100-fields` of the settings-band steps.
- [ ] `just e2e 4999` PASS.

### Task 9: Full gate and app check

- [ ] `just check` and `just test` green (note the known `$TMPDIR/x` environmental failure if
  it is still there).
- [ ] Run the app (`run-oku-slides`), select a chart and a table, edit from the dock, undo;
  screenshot.
