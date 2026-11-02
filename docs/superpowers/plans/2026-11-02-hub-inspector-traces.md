# Hub inspector, data pane, popovers and traces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Empty the agent thread of object state: element settings and chart/table data move to an
inspector and a data pane, requested boxes become popovers, agent-opened surfaces become chips, and each
receipt lists clickable traces of the elements the agent changed.

**Architecture:** The session gains a `host-element` surface (like `host-data`) with value and list
writes, and records the elements the agent's tools change (`handle_by_agent` → `TurnTouches`, per turn,
after success), resolved as `Trace`s for receipts. `Place` gains `Inspector` and `Popover`; boxes opened
by agent tools always carry a turn and show as chips. The UI adds an inspector column and a data pane row
inside `.sheet-area`, saved in `UiState`, a popover frame anchored to its opener, and a trace handler
built on `useAgentFocus`.

**Tech Stack:** Rust (oku-slides-session, -mcp, -agent, -a2ui, -i18n), React/TS (Bun, happy-dom render
tests), Playwright e2e (`just e2e`).

**Spec:** `docs/superpowers/specs/2026-11-02-hub-inspector-traces-design.md` (read it first; this plan
was revised after a 15-reviewer pass, its findings are folded in).

## Global Constraints

- Read the `AGENTS.md` and `ARCHITECTURE.md` of every directory you touch (root, `crates/`,
  `crates/oku-slides-session/`, `crates/oku-slides-mcp/`, `crates/oku-slides-agent/`,
  `crates/oku-slides-i18n/`, `ui/`). `session`, `mcp` and `agent` `ARCHITECTURE.md` are at 50 lines:
  reword lines, never add. Same commit as the code.
- Agent provenance goes through `Session::*_by_agent` methods; never an origin flag on requests
  (`crates/oku-slides-session/AGENTS.md`). New `impl Session` code goes in its own module; `lib.rs`
  (937 lines) gets match arms and one field only.
- No `unwrap`/`expect` outside tests, no silent fallback; Snafu errors with backtrace, Fluent messages.
  Prefer core's own validation (`OpError`, already localized) to new session error variants.
- Every Fluent key goes into all 20 locales of `crates/oku-slides-i18n/src/locales/*/`, same order, each
  plural variant on its own line, numbers passed as numbers. Test messages on the key plus `fr` text.
- Rust unit tests in sibling `x_tests.rs`; integration tests in `crates/*/tests/`; speculoos; names
  `unit_scenario_result`; one behaviour per test; rstest for tables.
- UI: visible text through `t()`; `IconButton`, `IconToggle`, `Button` from `ui/src/primitives.tsx`;
  effects survive StrictMode; `*.render.test.tsx` for DOM tests; English comments and test names in new
  files; module header comment on every new file (first sentence < 15 words).
- No new A2UI component kind (`CATALOG_ID` unchanged). Regenerate the fixture with
  `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form_matches_the_ui_fixture`; its
  diff holds only what the task changed.
- Files ≤ 500 lines target, 1000 hard. `App.tsx` (541), `api.ts` (903), `styles.css` (940),
  `error.rs` (848): new UI wiring goes in hooks, new CSS in `ui/src/inspector.css`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.
  The first nextest run in this worktree builds cold (> 10 min): background.
- e2e specs share one server: a spec that opens the inspector or data pane closes them before it ends.
- Commits: `type(scope)!?: description (t-1502, t-N)`; `!` on Tasks 2, 4, 5 (public signatures and
  places change); message from a scratchpad file (`git commit -F`); SDD branch (`oku.review=deferred`).

---

### Task 1: `host-element` surface and element writes

**Files:**
- Create: `crates/oku-slides-session/src/element_panel.rs`, `element_panel_tests.rs`, `tests/element.rs`
- Modify: `crates/oku-slides-session/src/widget_params.rs` (extract the field loop), `panel.rs` (`HostParams::from_schema`)
- Modify: `crates/oku-slides-session/src/lib.rs` (mod), `request.rs` (variants, dispatch, the persisting list ~:470 — a `matches!`, edit by hand), `request_class.rs` (`changes_deck`, `edits` — `matches!`, by hand — `writes_edited_draft`, `theme_tab_allows`) and `request_class_tests.rs`
- Modify: `crates/oku-slides-session/tests/common/mod.rs` (move `ok` and `with_element` from `tests/data.rs`, which then imports them)
- Modify: `crates/oku-slides-session/src/agent/forms_tests.rs` (`every_form()` gains a `host-element`), the fixture
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl`, `ui/src/api.ts`, `crates/oku-slides-session/ARCHITECTURE.md`

**Interfaces:**
- Produces (Rust): `pub(crate) const HOST_ELEMENT: &str = "host-element";`
  `HostParams::from_schema(schema: &WidgetSchema, values: &Params, slide: &str, title_key: &str, skip_host: bool, l: &Localizer) -> HostParams`;
  `Session::element_panel(&self, slide: &str, element: &str) -> Option<Surface>`;
  `Session::element_param(&mut self, slide: &str, element: &str, key: &str, value: Value) -> Result<()>`;
  `Session::element_list(&mut self, slide: &str, element: &str, key: &str, change: ListOp) -> Result<()>`;
  `Request::{ElementPanel { slide, element }, ElementParam { slide, element, key, value }, ElementList { slide, element, key, change }}`.
- Produces (TS): `api.elementPanel(slide, element): Promise<Surface | null>`,
  `api.elementParam(slide, element, key, value: unknown): Promise<Snapshot>`,
  `api.elementList(slide, element, key, change: ListOp): Promise<Snapshot>`.

- [ ] **Step 1: Failing integration tests** in `tests/element.rs`. Use `kinds::base::CHART` (its
  `option` host param must not appear as a field) and a base kind with a settable colour or choice
  param (read `oku-slides-core/src/kinds/` and its schema; name it `KIND`, `PARAM`, `NEW`).

```rust
fn panel(s: &mut Session, slide: &str, element: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "element_panel", "slide": slide, "element": element }))));
    ok(serde_json::from_str(&text))
}
fn set(s: &mut Session, slide: &str, value: Value) -> oku_slides_session::Result<String> {
    s.handle(req(json!({ "cmd": "element_param", "slide": slide, "element": "e", "key": PARAM, "value": value })))
}

#[test]
fn element_panel_element_is_the_host_element_surface() { /* id == "host-element" */ }
#[test]
fn element_panel_shows_the_current_value() { /* set PARAM to NEW via apply, panel data model holds NEW */ }
#[test]
fn element_panel_unknown_element_returns_null() {}
#[test]
fn element_panel_chart_has_no_data_field() { /* no field key == "option" */ }
#[test]
fn element_panel_in_history_returns_null() { /* open history first (see tests/session.rs for the request) */ }
#[test]
fn element_param_undo_restores_the_previous_value() {
    // set NEW, then {"cmd":"undo"}, then read params back through {"cmd":"slide"} like tests/data.rs:141
}
#[test]
fn element_param_undo_label_is_the_param_label_in_fr() { /* common::snapshot_json(s)["undo_label"], fr session */ }
#[test]
fn element_param_unknown_key_is_a_core_refusal() {
    let r = set_key(&mut s, &slide, "zz", json!(1));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Op));
}
#[test]
fn element_param_unknown_element_is_a_core_refusal() {}
#[test]
fn element_list_insert_adds_one_item() { /* a base kind with a list param; ListOp insert; undo removes it */ }
```

  First write each test against the request shape, run it, and check it fails for the right reason
  (unknown `cmd`). `element_param_unknown_key_is_a_core_refusal` must be checked red against an
  implementation that returns a different error kind, not only against the missing command.

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session --test element` (background) → FAIL.
- [ ] **Step 3: Extract the field builder.** Move the loop of `widget_settings`
  (`widget_params.rs:~36-50`: fields, keys, fixed notes, `session-widget-no-params`) into
  `HostParams::from_schema` in `panel.rs`. `skip_host: true` silently skips `Host`, `Slot` and `Grid`
  params (the inspector's Data button covers host data). Missing values take `schema_default`
  (`param_fields.rs:217`), as `preview_params` does. Fixed notes use `session-widget-param-fixed` in
  pack mode and a new `session-element-param-fixed = { $name }: not adjustable here` (fr `{ $name } :
  non réglable ici`) for elements. `widget_settings` calls it with its example values; its existing
  tests stay green.
- [ ] **Step 4: Implement `element_panel.rs`** (module doc: "The host-element surface: a deck
  element's settings at its current values."):

```rust
impl Session {
    /// Settings of `element` on `slide`; `None` outside an editable deck or for an unknown element.
    pub(crate) fn element_panel(&self, slide: &str, element: &str) -> Option<Surface> {
        self.ensure_editable().ok()?;
        let deck = self.doc.sources.deck();
        let located = deck.slide(slide)?.locate(element)?;
        let schema = self.registry.get(&located.element.kind)?;
        let p = HostParams::from_schema(schema, &located.element.params, slide, "session-widget-params", true, &self.localizer);
        // `params_with` places it in the thread; the UI fetches it and renders it in the inspector.
        Some(params_with(HOST_ELEMENT, &p.spec, &p.extras(), &self.slide_palette(), &self.localizer))
    }

    /// Sets `key` of `element`; core validates the param and its value. One undo step.
    pub(crate) fn element_param(&mut self, slide: &str, element: &str, key: &str, value: Value) -> Result<()> {
        let target = Target { slide: slide.to_owned(), element: Some(element.to_owned()) };
        let label = self.element_param_label(slide, element, key);
        // `null` removes the param (`SetParam` semantics): the widget's default applies.
        self.apply(&label, vec![Op::SetParam { target, param: key.to_owned(), value }], None)
    }

    /// Applies one list change to `key` of `element` (`key` may be `param/item/column`). One undo step.
    pub(crate) fn element_list(&mut self, slide: &str, element: &str, key: &str, change: ListOp) -> Result<()> {
        let target = Target { slide: slide.to_owned(), element: Some(element.to_owned()) };
        let label = self.element_param_label(slide, element, key);
        self.apply(&label, vec![Op::EditList { target, param: key.to_owned(), change }], None)
    }

    /// `agent-undo-param` with the param's label, or its raw key (cleaned by `bounds::label`) if unknown.
    fn element_param_label(&self, slide: &str, element: &str, key: &str) -> String { /* param_label or label(key) */ }
}
```

  Before writing, confirm in `oku-slides-core/src/ops/` that `SetParam` with an undeclared param or a
  bad value, and with an unknown element, returns an `OpError` that `apply` maps to
  `SessionErrorKind::Op` (adjust the test's expected kind to the real one, never to a catch-all).
  `ensure_editable` is called by `apply` (check; else call it first).
- [ ] **Step 5: Wire the requests** (`request.rs`): doc-commented variants, dispatch
  `ElementPanel => return json(&self.element_panel(&slide, &element))`, the two writes like `DataEdit`.
  Copy `DataEdit`'s row in every classifier for both writes, `DataPanel`'s for `ElementPanel`; extend the
  rstest tables in `request_class_tests.rs` (help, theme tab, history, pack tab rows).
- [ ] **Step 6: Unit test** `element_panel_tests.rs`: `element_panel_fields_carry_param_keys`. Add a
  `host-element` form to `every_form()`, regenerate the fixture.
- [ ] **Step 7: Run** `cargo nextest run -p oku-slides-session` (background) → PASS.
- [ ] **Step 8: TS API** in `api.ts`: three command union members and `TabApi` methods
  (doc comments, `unknown` for `value`, `ListOp` for `change`). `cd ui && bunx tsc -b` → PASS.
- [ ] **Step 9: Commit** `feat(session,ui,i18n): host-element surface with value and list writes (t-1502, t-1503)`
  (ARCHITECTURE line for `element_panel.rs` reworded into the panels line).

---

### Task 2: Agent touches and receipt traces

**Files:**
- Create: `crates/oku-slides-session/src/agent/turn_touches.rs`, `turn_touches_tests.rs`, `agent/traces.rs`, `traces_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs` (mods, `pub use traces::{Trace, TraceName};`),
  `agent/context.rs` (`begin_turn` :117 / `end_turn` :131), `deck_widgets.rs` (`apply_ops` hook),
  `deck_file.rs:~135` (reset with `turn_steps`), the `Session` struct (field `turn_touches`)
- Modify: `crates/oku-slides-session/src/agent/receipt.rs` (`receipt`, `fallback`, `report_surface`, both calls that build `surface` and `plain`) and `receipt_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/surface_ops.rs` (`without_actions`: trace button → caption) and its tests
- Modify: `crates/oku-slides-mcp/src/edit_tools.rs` (`change`), `guarded_tools.rs:60,302`, `app_tools.rs:147`, `surface_tools.rs:155` → `handle_by_agent` / `apply_by_agent`
- Modify: `crates/oku-slides-mcp/src/hub.rs` (`HubState.traces: HashMap<u32, Vec<Trace>>`, `note_turn_traces`), `hub_turns.rs` (`end_turn` :345 uses them; `show_unlisted_guide_writes` :333 passes `&[]`)
- Modify: `crates/oku-slides-agent/src/tab_agent.rs` (`Ctx::end_turn` :121 pushes traces; memory-lost card :523 passes `&[]`)
- Modify: `crates/oku-slides-mcp/tests/{receipts,turns}.rs`, `crates/oku-slides-mcp/src/hub_thread_tests.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl`, the fixture
- Create: `ui/src/agent/trace.ts`, `trace.test.ts`, `ui/src/useTrace.ts`, `useTrace.render.test.tsx`
- Modify: `ui/src/agent/useAgentFocus.ts` (+ `onMissing`), `ui/src/App.tsx`, `ui/src/AgentWindow.tsx` (`onHostAction` :271), `ui/src/agent/link.ts` (+ `reveal` message), `ui/src/agent/useMainLink.ts`

**Interfaces:**
- Produces (Rust, `oku_slides_session::agent`):

```rust
/// The object a trace names, localized where it is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceName {
    /// The element's first text, plain, at most 40 characters.
    Text(String),
    /// No text: the element's kind, named through its schema label.
    Kind { kind: String, label: Option<Label> },   // `Label`: the type `kind_name` takes
}
/// One element the agent changed in a turn, ready for a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// Slide that holds the element now.
    pub slide: String,
    /// Element id.
    pub element: String,
    /// What the chip calls it.
    pub name: TraceName,
    /// Ops that touched it.
    pub changes: u32,
    /// Gone from the deck when the trace was built.
    pub removed: bool,
    /// Its chart or table data changed: the click opens the data pane.
    pub data: bool,
}
impl Session {
    /// `handle` for an agent tool: deck ops applied by it are recorded for the open turn's receipt.
    pub fn handle_by_agent(&mut self, req: Request) -> Result<String>;
    /// `apply` for an agent tool (hub-applied agent ops), recorded like `handle_by_agent`.
    pub fn apply_by_agent(&mut self, label: &str, ops: Vec<Op>, merge: Option<&str>) -> Result<()>;
    /// Traces of `turn`, resolved against the deck now; empty for an unknown turn.
    pub fn traces(&self, turn: u32) -> Vec<Trace>;
    /// Closes the turn (as today) and returns its traces.
    pub fn end_turn(&mut self) -> Vec<Trace>;
}
pub fn receipt(id: &str, spec: &Report, slide: Option<&SlideRef>, traces: &[Trace], l: &Localizer) -> Surface;
pub fn fallback(id: &str, narration: &str, written: &[String], traces: &[Trace], l: &Localizer) -> Option<Surface>;
pub const TRACE_EVENT: &str = "trace";
pub const TRACES_SHOWN: usize = 6;
```

- Internal: `TurnTouches` (`pub(crate)`) keeps `VecDeque<(u32, Vec<Touch>)>` bounded by `KEPT_TURNS`,
  plus `recording: bool` (set only inside `*_by_agent`) and `open: Option<u32>`; each list is ordered
  by first touch with a `HashMap<(String, String), usize>` index; `changes` add with `saturating_add`.
- Produces (TS): `traceTarget(a: QuickAction): TraceTarget | null` (`{ slide: string; element: string; pane: boolean }`),
  `useTrace({ snap, view, current, editing, show, select, onPane, onInspector, say }): (s: Surface, a: QuickAction) => boolean`,
  link message `{ to: "main"; kind: "reveal"; tab; target: TraceTarget }`.

- [ ] **Step 1: Failing `TurnTouches` tests** (`turn_touches_tests.rs`): `record_same_element_merges_changes`,
  `record_keeps_first_touch_order`, `record_removal_after_edit_marks_removed`,
  `record_data_after_edit_marks_data`, `begin_turn_starts_an_empty_list_and_keeps_the_previous`,
  `turns_beyond_kept_turns_are_dropped`. One assertion target per test; a `touch(element)` helper at the top.
- [ ] **Step 2: Failing `touches_of` table** (rstest in `turn_touches_tests.rs`), one case per `Op`
  variant of `oku-slides-core/src/ops/mod.rs`, expecting `(slide, element, data, removed)` or none:
  `SetParam`/`EditList`/`SetKind`/`SetStyle`/`ReplaceStyle`/`SetFrame`/`SetCell` on an element → one
  touch; `SetData`/`SetChartOption`/`SetTableOption` → one touch, `data: true`; `ReplaceElement` →
  `element.id`, `data` when the element is a chart or table (host param in its schema); `AddElement` →
  its `element.id` and kind; `RemoveElement` → `removed: true`; `MoveElement` → keyed on
  `to.owner.slide`; any op with `target.element: None` and every slide- or deck-level op → none. The
  implementation is a `match` with **no `_` arm**. `data` reads `schema.host_param(..)` names, never
  `data_target` (it clones the dataset). Kinds come from the deck before the batch, then from earlier
  `AddElement`s of the same batch.
- [ ] **Step 3: Run** `cargo nextest run -p oku-slides-session turn_touches` → FAIL.
- [ ] **Step 4: Implement** `turn_touches.rs` (module doc: "Elements the agent's tools changed, per
  turn, for the receipt's traces."). Hook: `begin_turn` → `turn_touches.begin(turn)`; `end_turn` →
  `end()` then `self.traces(turn)`; in `apply_ops` (`deck_widgets.rs:340`), when
  `turn_touches.recording()`, compute `touches_of` for the batch **before** the ops run and record them
  **only after** `apply_ops` succeeds:

```rust
let pending = self.turn_touches.recording().then(|| self.touches_of_batch(&ops));
self.apply_ops_inner(label, ops, merge)?;          // the current body
if let Some(p) = pending { self.turn_touches.record_all(p); }
```

  `handle_by_agent` / `apply_by_agent` set `recording` for the call's duration (a guard restores it on
  every return path, errors included). Integration tests (`tests/traces.rs`):
  `handle_by_agent_apply_records_one_trace`, `handle_apply_by_user_during_turn_records_nothing`,
  `handle_by_agent_refused_op_records_nothing`, `data_edit_by_agent_marks_data` (through
  `handle_by_agent(Request::DataEdit …)`), `end_turn_returns_the_turn_traces`.
- [ ] **Step 5: Traces** (`traces.rs`, `traces_tests.rs`): `Session::traces(turn)` resolves each
  touch: `removed` when `deck.slide(slide)?.locate(element)` fails now (covers `RemoveSlide` and parent
  removal); name = the first param whose schema type is text or rich text with a non-empty value,
  through `paragraphs_to_text`, trimmed, cut at 40 chars with "…"; else `TraceName::Kind`. Tests:
  `traces_title_from_first_text`, `traces_long_title_is_cut`, `traces_without_text_name_the_kind`,
  `traces_removed_element_is_marked`.
- [ ] **Step 6: Switch the MCP write paths** to the agent methods: `edit_tools.rs::change` (`s.handle` →
  `s.handle_by_agent`), `guarded_tools.rs:60,302`, `app_tools.rs:147` when it changes the deck,
  `surface_tools.rs:155` (`s.apply` → `s.apply_by_agent`). Leave `hub/actions.rs:145,166` on `apply`:
  those are the user's clicks on agent forms and variants (spec decision 3). Test in
  `crates/oku-slides-mcp/tests/edit.rs`: `apply_tool_edit_appears_in_the_turn_traces`.
- [ ] **Step 7: Receipt rendering, failing tests first** (`receipt_tests.rs`):
  `receipt_one_trace_button_per_element`, `receipt_removed_trace_is_caption`,
  `receipt_more_than_six_traces_adds_a_more_caption`, `receipt_plain_copy_keeps_trace_buttons`,
  `receipt_trace_labels_fr` (rstest: count 1 → "Ventes : 1 modification", 2 → "Ventes : 2
  modifications", data → "Ventes : données modifiées", removed → "Ventes : n'existe plus"; literal
  French, fr localizer). Implement: after the receipt card a `Row` `traces`; each live trace a button
  built like the receipt's `suggest` buttons, action `trace` with context `{slide, element, pane: data}`;
  removed ones `Text` captions; beyond `TRACES_SHOWN`, one caption `agent-trace-more`. `report_surface`
  passes `self.traces(current turn)` to **both** `receipt` calls (`surface` and `plain`, `receipt.rs:~197-205`).
  Fluent (all 20 `agent.ftl`, after `agent-report-why`; adapt plural categories per locale):

```ftl
agent-trace = { $name }: { $count ->
    [one] { $count } change
   *[other] { $count } changes
}
agent-trace-data = { $name }: data changed
agent-trace-removed = { $name }: no longer exists
agent-trace-more = { $count ->
    [one] and { $count } other
   *[other] and { $count } others
}
```

  fr: `{ $name } : { $count -> … [one] { $count } modification *[other] { $count } modifications }`,
  `{ $name } : données modifiées`, `{ $name } : n'existe plus`, `et { $count } autre(s)` as plural.
  `$count` is passed as a number (`i64::from(changes).into()`). `TraceName::Kind` is localized with
  `kind_name` at render time.
- [ ] **Step 8: Restore keeps text.** `Surface::without_actions` (`surface_ops.rs:57`) turns a `Button`
  whose action name is `TRACE_EVENT` into a caption `Text` with the button's label (confirm the
  `ComponentKind::Button` field names in `oku-slides-a2ui/src/component.rs`) instead of dropping it.
  Tests: `without_actions_keeps_trace_labels_as_text` (surface_ops_tests),
  `restored_receipt_keeps_traces_as_text` (`hub_thread_tests.rs`).
- [ ] **Step 9: Fallback receipt.** `Ctx::end_turn` (`tab_agent.rs:121`) gets the traces from
  `tab.with(Session::end_turn)` inside its `spawn_blocking` and calls `hub.note_turn_traces(turn, traces)`
  before `TurnEnded` is emitted (read `turn.rs:136-138` for the order). `SurfaceHub::end_turn` passes the
  stored traces to `fallback` and drops them; `show_unlisted_guide_writes` and `tab_agent.rs:523` pass
  `&[]`. `HubState.traces` is pruned with the turns it belongs to. Tests (`mcp/tests/turns.rs`):
  `fallback_receipt_lists_the_turn_traces`, `late_guide_card_has_no_traces`.
- [ ] **Step 10: Fixture.** `every_form()` gains a receipt with two traces and one removed; regenerate;
  `cd ui && bun test src/a2ui/catalog.schemas.render.test.tsx` → PASS.
- [ ] **Step 11: UI, failing tests first.** `trace.test.ts` (`describe("traceTarget")`): valid action →
  target; other action → null; missing element → null; non-string slide or element → null; non-boolean
  pane → null; empty strings → null. `useTrace.render.test.tsx`: a trace from a non-receipt surface is
  ignored (returns false); a trace to the shown slide selects the element; a trace to another slide shows
  it then selects once its view arrives; an unknown slide or element says `ui-trace-gone` and changes
  nothing; `pane` calls `onPane`, else `onInspector` (both after selection).
- [ ] **Step 12: Implement.** `trace.ts` (module doc: "The receipt's traces: the element a click
  shows."). `useAgentFocus` gains an optional `onMissing()` called where it now ignores an unknown slide
  or an element not found in a fresh view (existing tests stay green). `useTrace` (module doc: "Trace
  clicks: show the slide, select the element, open its place.") accepts only `isReceipt(s)` surfaces,
  uses `useAgentFocus` with `onMissing = () => say(t("ui-trace-gone"), true)` and calls
  `onPane`/`onInspector` once selected (Task 3 passes them; until then no-ops). `App.tsx`: `useTrace`
  first in the `onHostAction` chain. `AgentWindow.tsx:271`: a receipt `trace` is sent over the link as
  `reveal` and returns true; `useMainLink` handles `reveal` by calling the same `useTrace` handler
  (pass it in `MainLinkInput`). Fluent (20 `ui.ftl`): `ui-trace-gone = This element no longer exists`
  (fr `Cet élément n'existe plus`).
- [ ] **Step 13: Run** `just test` (background) → PASS. Commit
  `feat(session,mcp,agent,ui,i18n)!: receipts list traces of the elements the agent changed (t-1502, t-1504)`.

---

### Task 3: Inspector column, data pane, focus no longer selects

**Files:**
- Create: `ui/src/Inspector.tsx`, `Inspector.render.test.tsx`, `ui/src/DataPane.tsx`, `DataPane.render.test.tsx`,
  `ui/src/useElementSettings.ts`, `useElementSettings.render.test.tsx`, `ui/src/useSelectionSurface.ts`,
  `ui/src/inspector.css`, `ui/src/useInspector.ts`
- Modify: `ui/src/useDataSettings.ts` (built on `useSelectionSurface`; header no longer says "dock"), `ui/src/useHostSettings.ts` (export `routeListChange` if private)
- Modify: `ui/src/App.tsx` (hooks :257–262, `.sheet-area` :391–436, delete the detached `HostSettings` :442–444 and `hostSurfaces`), `ui/src/main.tsx` or wherever `styles.css` is imported (import `inspector.css` after it), `ui/src/styles.test.ts`
- Modify: `ui/src/agent/AgentPanel.tsx` (drop `hostSurfaces` :112/:177–178 and `NO_SURFACES` if unused), `AgentPanel.render.test.tsx` (:63, :126, :387, :397)
- Modify: `ui/src/useKeptUi.ts` + `useKeptUi.render.test.tsx`, `ui/src/api.ts` `UiState`, `crates/oku-slides-session/src/views.rs` `UiState` :186, `crates/oku-slides-session/tests/session.rs` (:308, :354, :363), `crates/oku-slides-session/src/resume_tests.rs` (:253, :287, :323), UiState literals in `ThemeTab`, `PackPanels`, `ThemePanels`, `usePackItem`, `useKeptUi.insecure` render tests
- Modify: `ui/src/ToolsTray.tsx` (inspector `IconToggle`)
- Modify: `crates/oku-slides-mcp/src/edit_tools.rs` `focus` (:~186), the `focus` tool description in `crates/oku-slides-mcp/src/tools.rs`, `ui/src/agent/AgentPanel.tsx` focus wiring (:189), `crates/oku-slides-mcp/tests/edit.rs:160`
- Modify: `ui/e2e/010-editor.cjs` (:102, :105), `ui/e2e/150-data-panel.cjs` (incl. :50), `ui/e2e/090-layout.cjs` (check)
- Modify: `ui/AGENTS.md` (host-data rule), README shortcut table, `docs/aide/en.json` and `fr.json`, `crates/oku-slides-i18n/src/locales/*/ui.ftl`

**Interfaces:**
- Consumes: Task 1 API; `useTrace` (`onPane`, `onInspector`); `useSessionSurface`, `routeListChange`, `ownPanel`.
- Produces:
  - `useSelectionSurface({ ids, api, snap, slide, selected, off, fetch, send, run, say }): HostSettings`
    (the shared active rule: a deck tab, not read-only, history or theming, a selected element; key
    `[slide, selected?.id, slide hash, undo_label, redo_label]` as `useDataSettings.ts:33`).
  - `export const HOST_ELEMENT = "host-element"`; `useElementSettings(api, snap, slide, selected, off, run, say): HostSettings`
    (any selected element; `param` with `value` → `api.elementParam`, list changes via `routeListChange` → `api.elementList`).
  - `export interface InspectorProps { element: HostSettings; data: boolean; onData(): void; deck: { theme: string | null; onPacks(): void }; offer?: ReactNode; onAction(s: Surface, a: A2uiClientAction): boolean; onError(e: unknown): void; onClose(): void }`
  - `export interface DataPaneProps { data: HostSettings; onAction(...): boolean; onError(e: unknown): void; onClose(): void }`
  - `UiState.inspector: boolean`, `UiState.data_pane: boolean` (Rust `#[serde(default)]`; TS required,
    the server always sends them); `KeptUi.inspector`, `setInspector`, `dataPane`, `setDataPane`;
    `inspectorToggle(e: KeyboardEvent): boolean` (⌘⇧E / Ctrl+Shift+E, matched by key, both cases).
  - `useInspector(...)`: the App wiring (open/close, `onPane`/`onInspector` for `useTrace`, the deck block).

- [ ] **Step 1: Failing tests.**
  - `useElementSettings.render.test.tsx` (model: `useDataSettings.render.test.tsx`): active for any
    selected element; inactive in history and theme tabs; a `param` value sends `elementParam`; a list
    change sends `elementList`; rereads after undo.
  - `Inspector.render.test.tsx`: nothing selected shows the theme name and Packs; an element shows
    `realSurface("host-element")`; `data` shows a Data button calling `onData`; an `offer` comes before the
    element's settings inside a frame labelled `t("ui-inspector-offer")`.
  - `DataPane.render.test.tsx`: shows `host-data`; close calls `onClose`.
  - `useKeptUi.render.test.tsx`, `describe("inspectorToggle")` next to `viewToggle`: ⌘⇧E (key `"E"`),
    Ctrl+Shift+E, ⌘E without Shift does not toggle, ⌘⇧E while editing text does not toggle; restore then
    save of `inspector` and `data_pane`.
  - Rust: `tests/session.rs` exact JSON gains both fields; `ui_state_without_inspector_reads_false`
    (an older `state.json`); `focus_tool_shows_the_slide_without_selecting` (`mcp/tests/edit.rs`).
  - `styles.test.ts`: `.sheet-area > .data-pane` exists and is never `position:absolute`; the
    `@media (max-width: 1100px)` inspector overlay rule exists.
- [ ] **Step 2: Run** `cd ui && bun test` on those files and the Rust tests → FAIL.
- [ ] **Step 3: Implement the hooks.** `useSelectionSurface`, then `useDataSettings` and
  `useElementSettings` on it (no duplicated active/scope/key code).
- [ ] **Step 4: Implement the components** (headers: "The inspector: the selection's settings, or the
  deck's." / "The data pane: the selected chart's or table's data, under the stage."), with `IconButton`
  close buttons and `HostSettings` for surfaces.
- [ ] **Step 5: Layout** in `inspector.css`: `.sheet-area` becomes
  `grid-template-columns: minmax(0, 1fr) var(--inspector-w, 0px); grid-template-rows: minmax(0, 1fr) auto auto;`
  with `.sheet-area.inspector-open { --inspector-w: 280px }`; stage panel and widget pane `grid-column: 1;
  grid-row: 1`; variant drawer `grid-column: 1; grid-row: 2`; `.data-pane { grid-column: 1; grid-row: 3;
  max-height: 40vh; overflow-y: auto }`; `.inspector { grid-column: 2; grid-row: 1 / 4; overflow-y: auto }`.
  Read every child of `.sheet-area` (`.slide-strip` included, `pack.css:117`) and give each its column;
  the `.table` grid, column dividers and `columns.ts` stay untouched. Below 1100 px: `--inspector-w: 0px`
  and `.inspector { position: absolute; inset-block: 0; inset-inline-end: 0; width: 280px; z-index: 5 }`.
- [ ] **Step 6: Saved state.** Rust `views.rs` fields with doc comments and `#[serde(default)]`; TS
  `UiState`; `useKeptUi` state, restore, save object and deps, `handleKey` branch for `inspectorToggle`
  gated to deck tabs (`appKeyScope` does not let it through while editing). Update every listed UiState
  literal (tsc skips tests: grep `preview_span:` under `ui/src` to find them all).
- [ ] **Step 7: Wire** through `useInspector` (keeps App's additions to a few lines): `.sheet-area` gets
  `inspector-open` when `deckMode && keptUi.inspector`; `<Inspector>` and `<DataPane>` render inside it;
  `onPacks` calls `agent.showPacks()` (same as the tray); `data = ownPanel(snap.host_params, kind) !== null`;
  `useTrace` gets `onPane = () => keptUi.setDataPane(true)` and `onInspector = () => keptUi.setInspector(true)`.
  Remove `hostSurfaces` and the detached `HostSettings`; the `agent-detached` class drops `!data.surface`.
  `ToolsTray` gains an inspector `IconToggle` (`t("ui-inspector")`, pressed = open), deck mode only.
- [ ] **Step 8: Focus.** Drop `element` from `FocusArgs`, from `McpEvent::Focus` and from the `focus`
  tool description (MVP rule, no shim); `AgentPanel`'s focus wiring only shows the slide, and
  `useAgentFocus` keeps its select path for `useTrace` only.
- [ ] **Step 9: Fluent** (20 `ui.ftl`): `ui-inspector = Inspector`, `ui-inspector-close = Close the inspector`,
  `ui-inspector-data = Data`, `ui-inspector-packs = Packs`, `ui-inspector-no-theme = No theme`,
  `ui-inspector-offer = Offered by the agent`, `ui-data-pane = Data`, `ui-data-pane-close = Close the data`;
  fr `Inspecteur`, `Fermer l'inspecteur`, `Données`, `Packs`, `Aucun thème`, `Proposé par l'agent`,
  `Données`, `Fermer les données`.
- [ ] **Step 10: e2e.** `010-editor.cjs:102,105` and `150-data-panel.cjs`: select the chart, click the
  inspector toggle, then "Données" with `{ exact: true }`, and read `'.data-pane [data-surface-id="host-data"]'`;
  150 :50 now asserts the chart panel is inside `.data-pane`; both close the pane and the inspector at the
  end. Run `just e2e 4999` (background) → PASS.
- [ ] **Step 11: Docs and commit.** README `| ⌘⇧E | Show or hide the inspector |`; same row in
  `docs/aide/en.json` and `fr.json` ("Afficher ou masquer l'inspecteur"); `ui/AGENTS.md`: "Edit chart and
  table data only through the data pane's host-data surface, sending DataEdit, never a patch." `just check`
  → PASS. Commit `feat(ui,session,mcp,i18n): inspector and data pane beside the stage, agent focus no longer selects (t-1502, t-1505)`.

---

### Task 4: Requested boxes become popovers, agent-opened ones chips

**Files:**
- Create: `ui/src/popoverPlace.ts`, `popoverPlace.test.ts` (cases moved from `a2ui/ListTable.render.test.tsx:14,447`), `ui/src/agent/AgentPopover.tsx`, `AgentPopover.render.test.tsx`
- Modify: `ui/src/a2ui/ListTable.tsx` (imports `popoverPlace`; no re-export), `ListTable.render.test.tsx`
- Modify: `crates/oku-slides-session/src/agent/forms.rs` (`Place`), `voice_panel.rs:91`, `deck_packs_panel.rs:360`, `theme_picker.rs:123`; `ui/src/api.ts` `Place` :705
- Modify: `crates/oku-slides-mcp/src/hub.rs` (`show_alone`), `hub/packs.rs` (`PacksShown.turn`, `apply_packs` :53, `refresh_packs` :73), `hub/themes.rs` (picker state `.turn`, refresh :46), `hub/settings.rs` (`SettingsShown.turn`, `send_settings` :57; `show_settings` returns `bool`)
- Modify: `crates/oku-slides-mcp/src/handler.rs` (:87, :88, :106 pass `turn`), `surface_tools.rs` (:200–236 docs and signatures), `voice_tools.rs` (:63–66), `tools.rs` (:351, :368, :369 descriptions)
- Modify: `crates/oku-slides-agent/src/stack.rs` (`AgentCommand::ShowThemes { tab, slide }` next to `ShowPacks` :130, `tab()` :161, handler near :284; :316, :370, :417 pass `None`), `crates/oku-slides-agent/tests/stack.rs`
- Modify: `crates/oku-slides-mcp/tests/{packs,themes,voice}.rs`
- Modify: `ui/src/agent/surfaces.ts` (`Routed`, `routeSurfaces`; delete `isRequestedBox`, `isHostSettings`), `surfaces.test.ts` (:191–218 replaced), `AgentDock.tsx`, `AgentThread.tsx`, `AgentPanel.tsx`, `ui/src/useAgentWiring.ts` (:55 slots), `ui/src/agent/useQuickKeys.ts` (`quickTarget` :27 and caller `AgentPanel.tsx:225`), `ui/src/agent/useRelocalizedBoxes.ts`, `ui/src/AgentWindow.tsx` (popover slot), `ui/src/ToolsTray.tsx` (`onThemes`, anchors), `ui/src/Dictation.tsx` (:360 anchor), `ui/src/App.tsx`, `ui/src/api.ts` (`showThemes(slide)`)
- Modify: e2e `210-settings-box`, `230-raccourci-theme`, `240-agent-themes`, `250-packs`, `160-widgets`, `330-guides`, `350-received-packs`, `ui/e2e/lib/atelier.cjs` (`closePacksPanel`)
- Modify: `ui/AGENTS.md`, `crates/oku-slides-mcp/AGENTS.md` / `ARCHITECTURE.md` (reworded lines), `crates/oku-slides-i18n/src/locales/*/ui.ftl`, the fixture

**Interfaces:**
- Produces (Rust): `Place::Popover` ("Popover next to the button that opens it."), `Place::Inspector`
  ("In the inspector, opened from its chip in the thread."); `SurfaceHub::show_packs(&self, turn: Option<u32>) -> Result<bool, HubError>`,
  `show_themes(&self, slide: Option<&str>, turn: Option<u32>) -> Result<bool, HubError>`, `show_settings(&self, turn: Option<u32>) -> bool`;
  `SurfaceHub::agent_turn(&self, turn: Option<u32>) -> u32` (the open turn, else the last one; used by
  the three tool paths so an agent box always carries a turn).
- Produces (TS): `Place` gains `"popover" | "inspector"`; `Routed` gains `popovers: Surface[]` and
  `offers: { turn: number; surface: Surface }[]`; `turns` entries exist for every turn that has an offer;
  `quickTarget(r: Pick<Routed, "modal" | "drawer" | "dock"> & { popover?: Surface })`;
  `<AgentPopover surface anchor: string onClose() />`; `api.showThemes(slide: string | null)`.

- [ ] **Step 1: Failing routing tests** (`surfaces.test.ts`, `describe("routeSurfaces")`), one per row:
  live turnless popover → `popovers`; live popover with turn → `offers` of its turn (and that turn exists
  in `turns`); live inspector with turn → `offers`; live inspector without turn → `loose`; frozen
  popover or inspector → its turn or `loose`; live non-question thread surface → `loose`; live question →
  `dock` (until Task 5 the dock still also keeps live non-question thread surfaces: write that row as a
  separate test Task 5 replaces).
- [ ] **Step 2: Failing Rust tests** (`mcp/tests/{packs,themes,voice}.rs`, `agent/tests/stack.rs`): each box
  `place == Place::Popover`; `show_*_from_tool_carries_the_open_turn`; `show_*_late_tool_call_carries_the_last_turn`;
  `show_*_from_command_has_no_turn`; `show_packs_refresh_after_action_keeps_turn` (and themes, settings);
  `show_themes_command_passes_the_slide`.
- [ ] **Step 3: Run** both → FAIL.
- [ ] **Step 4: Implement Rust.** Enum variants; the three builders' place; `turn` stored in each box's
  `Shown` state and stamped on every emit (first show, action, prune refresh); tool paths pass
  `Some(self.hub.agent_turn(turn))` from `handler.rs`; commands and the spoken shortcut pass `None`;
  `ShowThemes { tab, slide }` handled like `ShowPacks`. Rewrite the `surface_tools.rs:200–204`,
  `voice_tools.rs:63` and `tools.rs` descriptions: "Opens … as a card the user opens from your turn."
- [ ] **Step 5: Implement routing** in `surfaces.ts` per Step 1; delete `isRequestedBox`, `isHostSettings`;
  `AgentDock` drops the `aria-live` special case and folds every non-question while a question waits.
  `quickTarget` takes the theme picker only from the open popover.
- [ ] **Step 6: Popover, failing render tests first** (`AgentPopover.render.test.tsx`): Escape closes it
  and does not reach `useAppKeys` (`defaultPrevented`); an outside pointerdown closes it; a pointerdown on
  its anchor does not close it (the anchor's click toggles); an anchor id containing a quote does not
  throw; it renders nothing until its anchor exists. `popoverPlace.test.ts`: right-edge clamp (moved
  cases); an anchor near the bottom places the box above it.
- [ ] **Step 7: Implement.** Move `popoverPlace` to `ui/src/popoverPlace.ts` (header: "Places a floating
  box next to its anchor, inside the viewport."), add the flip-above mode with a vertical clamp (ListTable
  keeps its default). `AgentPopover` (header: "An agent box as a non-modal popover next to its opener.")
  wraps `AgentSurfaceBox` (`className="agent-popover" role="dialog" aria-modal="false"`), finds its anchor
  with `` `[data-popover-anchor="${CSS.escape(anchor)}"]` `` in a layout effect, recomputes on resize and
  scroll, focuses its first control on open and returns focus to the anchor on close, and handles Escape
  in the capture phase with `preventDefault` + `stopPropagation`. Add to `ui/AGENTS.md`: "Agent boxes open
  as non-modal popovers (AgentPopover) next to their opener; modals still use useModalFrame."
- [ ] **Step 8: Wire.** `AgentPanel`: a turnless popover surface is open while it exists (no local
  state); `offer: string | null` holds the chip-opened one; `onClose` sends the surface's `dismiss`
  action through `onSurfaceAction` (today's box close) and clears `offer`. Slots `popover` (in
  `useAgentWiring.ts:55` and `AgentWindow.tsx`) hold the open popover; App renders `<SlotView slot={agentSlots.popover} />`
  at the end of `.table`. Anchors (`data-popover-anchor`): `host-packs` on the tray's Packs button
  (`ToolsTray.tsx:106`, also on the inspector's Packs button: the popover takes the anchor that was
  clicked, kept by `AgentPanel` with the open id), `host-voice` on the gear (`Dictation.tsx:360`; in the
  detached window the gear and the popover slot are both in `AgentWindow`), `host-theme-picker` on a new
  Themes `IconButton` (`onThemes` prop, `t("ui-themes")`, deck mode, calls `agent.showThemes(view?.slide.id ?? null)`).
  `AgentThread` renders inside each turn's `li` one ghost `Button` per offer:
  `t("ui-offer-open", { title })` for popovers, `t("ui-offer-settings", { title })` for inspector offers
  (title from the surface's first `Text` component), calling `onOpenOffer(surface.id)`; in the detached
  window the opened offer renders under its chip. `useRelocalizedBoxes` re-shows only turnless boxes.
  Fluent (20 `ui.ftl`): `ui-themes = Themes`, `ui-offer-open = Open: { $title }`,
  `ui-offer-settings = Settings offered: { $title }`; fr `Thèmes`, `Ouvrir : { $title }`,
  `Réglages proposés : { $title }`. `$title` is isolated (the bundle runs with `useIsolating: false`,
  `ui/src/i18n.ts:103`: wrap with the existing FSI/PDI helper).
- [ ] **Step 9: Fixture and e2e.** Regenerate the fixture. User-opened boxes read
  `.agent-popover [data-surface-id="…"]`; tool-opened ones (240, 250 when the agent opens packs) first
  click the chip in `.agent-turn`. `closePacksPanel` presses Escape in the popover. Run `just e2e 4999` → PASS.
- [ ] **Step 10: Commit** `feat(session,mcp,agent,ui,i18n)!: requested boxes open as popovers, agent-opened ones as turn chips (t-1502, t-1506)`.

---

### Task 5: The agent's `show_params` goes to the inspector

**Files:**
- Modify: `crates/oku-slides-session/src/agent/variants.rs` (`params_surface` :139 sets the place), `crates/oku-slides-mcp/src/hub_turns.rs` (`freeze` :~235 resets `place` to `Thread`), `crates/oku-slides-mcp/tests/surfaces.rs`, `hub_thread_tests.rs`
- Modify: `ui/src/agent/surfaces.ts` (`dock` = live questions), `surfaces.test.ts`, `AgentDock.tsx` (no folding toggle), `AgentPanel.tsx` (slot `inspector`, props `selected`, `onOpenInspector`), `useAgentWiring.ts`, `ui/src/useInspector.ts` (offer from the slot)
- Modify: e2e specs that use `show_params` in the dock: grep `"show_params"` in `fixtures/agent/e2e*.json`, then the specs sending those prompts (list them in the commit message)
- Regenerate the fixture.

**Interfaces:**
- Consumes: `Place::Inspector`, `Routed.offers`, `AgentPanel.offer` (Task 4); `Inspector.offer` (Task 3).
- Produces: `slots.inspector` (the opened offer in an `AgentSurfaceBox`, actions through `onSurfaceAction`);
  `AgentPanel` props `selected: string | null` (a change closes the offer) and `onOpenInspector(): void`.

- [ ] **Step 1: Failing tests.** Rust: `params_surface_in_deck_mode_is_in_the_inspector`,
  `params_surface_in_pack_or_theme_tab_stays_in_the_thread`, `params_surface_without_turn_stays_in_the_thread`,
  `frozen_params_surface_returns_to_the_thread`, `thread_snapshot_keeps_frozen_params`. TS: "dock holds
  live questions only" (replaces Task 4's interim row); render: an offer chip click fills `slots.inspector`
  and calls `onOpenInspector`; a `selected` change empties it.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** `params_surface` sets `Place::Inspector` only in deck mode with an open turn,
  else `Thread`; `freeze` sets `place = Place::Thread`. `routeSurfaces`: `dock = thread.filter(s => s.live && isQuestion(s))`.
  `AgentPanel` fills `slots.inspector`; `useInspector` passes it to `Inspector.offer` and opens the
  inspector on `onOpenInspector`.
- [ ] **Step 4: Run** `just test` (background) → PASS; regenerate the fixture; update the e2e specs found
  in Files (click the chip, act in `.inspector`, close it at the end). `just e2e 4999` → PASS.
- [ ] **Step 5: Commit** `feat(session,mcp,ui)!: agent settings forms open in the inspector from their turn chip (t-1502, t-1507)`.

---

### Task 6: End-to-end trace spec and final docs

**Files:**
- Create: `ui/e2e/360-hub-traces.cjs`, `ui/e2e/lib/agent.cjs` (`prompt(p, text)`, moved from `060`, `210`, `240`, which then import it)
- Modify: `fixtures/agent/e2e.json` (new `by_prompt` entry)
- Modify: the spec (record any deviation met in Tasks 1–5), `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-mcp/ARCHITECTURE.md` (reworded lines)

- [ ] **Step 1: Scenario.** Read `150-data-panel.cjs:26–37` (it adds a chart through
  `post(p, '/api', { tab, req: { cmd: 'apply', … } })`) and copy that call into 360 with fixed ids: slide
  `s-constat`-like id of the deck 360 opens (read it from `activeSnapshot`), element id `e-ventes`. In
  `fixtures/agent/e2e.json` add `by_prompt["modifie le graphique des ventes"]`: `mcp` = one `apply` call
  with a `set_data` (or the op `data_edit` emits for a row change, `data_edit.rs:106`) on `e-ventes`, then
  `report` `{ "title": "Graphique mis à jour", "facts": ["2 séries"] }`. The scenario's slide id must equal
  the one the spec uses: make the spec create the chart on the slide id the fixture names.
- [ ] **Step 2: Write the spec:**

```js
// An agent edit opens nothing; its trace in the receipt opens the data pane on the edited chart.
const assert = require('node:assert');
const { chromium } = require('playwright');
const { post } = require('./lib/http.cjs');
const { prompt } = require('./lib/agent.cjs');
const { logOffset, mcpResults } = require('./lib/agentLog.cjs');
const { untilValue, stable } = require('./lib/until.cjs');
const URL = process.env.E2E_URL; const LOG = process.env.E2E_AGENT_LOG;
const SLIDE = 's-ventes'; const CHART = 'e-ventes';
(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = []; p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload(); await p.waitForSelector('.stage img');
  assert.equal(await p.locator('.inspector, .data-pane').count(), 0, 'inspector and pane start closed');
  await addChart(p, SLIDE, CHART);       // the 150-data-panel apply call, written out here
  const selectedBefore = await p.locator('.stage .target.selected').count();
  const from = logOffset(LOG);
  await prompt(p, 'modifie le graphique des ventes');
  await untilValue(p, () => mcpResults(LOG, from, 'report'), (o) => o.length > 0, 'report must answer', 15000);
  const receipt = p.locator('.agent-turn').last();
  await receipt.locator('.a2ui-receipt').waitFor();
  await stable(p, async () => p.locator('.inspector, .data-pane, .agent-popover').count(), 500);
  assert.equal(await p.locator('.inspector, .data-pane, .agent-popover').count(), 0, 'nothing opens by itself');
  assert.equal(await p.locator('.stage .target.selected').count(), selectedBefore, 'selection unchanged');
  await receipt.getByRole('button', { name: /données modifiées/ }).click();
  await p.locator('.data-pane [data-surface-id="host-data"]').waitFor();
  assert.equal(await p.locator(`.stage .target.selected[data-id="${CHART}"]`).count(), 1, 'chart selected');
  await closeInspectorAndPane(p); await removeChart(p, SLIDE, CHART);
  assert.deepEqual(errors, []); await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
```

  Write `addChart`, `removeChart` and `closeInspectorAndPane` in the spec from `150-data-panel.cjs`'s
  calls (check the selected-target selector and `data-id` attribute in `Stage.tsx` and adjust).
- [ ] **Step 3: Run** `just e2e 4999` (background) → PASS.
- [ ] **Step 4: Final gate** `just ci` (background) → PASS.
- [ ] **Step 5: Commit** `test(ui): a turn's trace opens the data pane, nothing opens by itself (t-1502, t-1508)`.

---

## Deferred (backlog candidates, from the review)

- Slide-level traces ("add a slide" leaves none).
- A merge key for slider and colour drags in `host-element` (one undo step per drag, like `data_edit`).
- Text params already edited on the stage: leave them out of `host-element`?
- Edits made after the agent's `report` call: a late trace card, like `show_unlisted_guide_writes`.

## Self-review notes

- Spec decisions → tasks: 1 → 3, 4, 5; 2 → 3 (focus), 4, 5, 6; 3 → 2; 4 → 1, 3, 5; 5 → 3; 6 → 5;
  7 → 4; 8 → 3; 9 → 3; 10 → 3; 11 → 2 (reveal), 4 (chips, gear).
- Every commit keeps every surface visible: dock surfaces leave only once their new place renders them;
  Task 4's interim dock row is replaced in Task 5.
