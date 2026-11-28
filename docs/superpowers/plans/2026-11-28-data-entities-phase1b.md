# Data entities, phase 1b (pack list params as data views) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pack widget's flat `list` param is a view on a deck data, like a chart or a table:
its items are the data's rows, its columns the data's columns matched by name; a list with a
nested `list` column stays inline.

**Architecture:** Core gains the list binding `{data, fields}` (bound by name, stored by
column id) and plans list views next to chart and table views. One pure function in render,
`resolve_lists`, turns each list view into the inline item shape Typst already reads (row id
as item id, cells checked per column type); the render worker calls it for Typst, the session
calls it to show fields, item boxes and `ListTable` rows, so both see the same items. Only a
data's base file is out of the session's reach: the worker returns the base cells its list
views read, and the session caches them by base id (a base never changes). `prepare_ops` lifts
inline lists to data and rewrites `edit_list` on a bound list into `edit_data`. Rich cells hold
the editing markdown; core converts both ways, checked against the UI's reader by the shared
corpus.

**Tech Stack:** Rust (pulldown-cmark 0.13 and proptest, both already core dependencies: no
`Cargo.toml` change), snafu, serde, Loro 1.17.1; React/TS UI (lezer markdown, zod, A2UI catalog
v7); Typst widgets unchanged.

**Spec:** `docs/superpowers/specs/2026-11-21-data-entities-design.md` ("Views", "Phases" item
1b, "Plan calls (phase 1b plan)" added with this plan). Phase 1 plan:
`2026-11-21-data-entities-phase1.md`; phase 2 plan: `2026-11-26-data-entities-phase2.md`. Tags
(phase 3) and linked sources (phase 4) are out of scope.

**Oku:** parent t-1807; plan t-1808. Tasks: 1 t-1809 · 2 t-1810 · 3 t-1812 · 4 t-1813 ·
5 t-1811 · 6 t-1814 · 7 t-1815 · 8 t-1816 · 9 t-1817 (the order puts planning and resolving
before lifting, so every commit keeps `just test` green).

## Decisions (accepted by the user, 2026-11-28)

Impact measured first: no shipped pack has a `list` param (8 `base` widgets, `essentiel`
layouts); only the test widgets `fixtures/widgets/frise-test.typ` (flat) and `plan-test.typ`
(nested) do. User packs are the ones affected. `oku-slides-import` and `src-tauri` build no
list param (checked).

1. **Rich cells hold the editing markdown.** Core gets `markdown_to_paragraphs` (pulldown-cmark
   plus a `==…=={n}` pass); the writer (`session/src/inline.rs` + `paragraphs_to_text` of
   `session/src/text.rs`) moves to core as `paragraphs_to_markdown`. The existing shared corpora
   `fixtures/markdown.json` (paragraphs ↔ markdown, `lossy` with `reread`) and
   `fixtures/markdown-corpus.json` (runs ↔ markdown) stay the single corpus, now also read by
   the core reader. A property test: rewriting then reparsing loses no text; formatting is lost
   only on spaces and punctuation. Markdown the editing syntax lacks (HTML, images, code,
   tables, block quotes, nesting) is kept as literal text, never dropped. An invalid cell
   renders absent with an issue. Rejected: paragraph JSON in the cell, a Typst parser.
2. **Nested lists stay inline.** A `list` param whose item has a nested `list` column is never a
   view: `bind_data` refuses it (`op-data-list-nested`); its value stays the inline item tree
   edited by `edit_list` as today.
3. **Binding `{data, fields: {<schema column>: <column id>}}`**, a `ParamValue::Value` read
   only through `ListBinding::from_value`. Matched by name at bind, stored by id so a rename
   keeps the view; over a query data the ids are the result's column names. Compatible types:
   `plain`/`choice`/`asset` ← Text or Date, `flag` ← Bool, `number` ← Number or Integer, `rich`
   ← Text. A schema column with no data column of its name or type refuses the bind
   (`op-data-list-column`). The mapped ids are the view's projection (`ViewRef.columns`, an
   unordered set for a list), so removing a mapped column is refused like for a projected view;
   a later retype, or a query whose result drops the column, renders that field absent with an
   issue. Item fields follow schema order, never the map's.
4. **Bounds and cell checks at render.** Rows past the schema `max` (or `LIST_ITEMS_MAX`, 256)
   are cut with an issue; each cell is checked against its column param (`plain` ≤
   `PLAIN_MAX` bytes, `choice` among its options, `asset` an asset id of the deck, `rich`
   parsed and within `RICH_PARAGRAPHS_MAX`/`RICH_BYTES_MAX`, number finite); an invalid cell
   renders absent with `issue-list-cell-invalid`, never refusing the deck. Typst receives the
   inline item shape it reads today.
5. **Every flat list is a view.** An inline flat list is creation input only (`add_element`,
   `add_slide`, `replace_element`, `set_param`, schema defaults, paste, pack examples):
   `prepare_ops` lifts it to a sealed `PutViewData` named after the param (`etapes`, then
   `etapes_2`); seeding defaults of flat lists moves from `seed_lists` (apply) to `prepare_ops`.
   `edit_list` on a bound list becomes `edit_data` (`Insert` → `insert_row`, `SetField` →
   `set_cell`, `Move` → new `move_row`, `Remove` → `remove_row`). List views count as data
   users everywhere views do (`data_users`, paste copy, `remove_data` refusal,
   `dependent_views`).
6. **Format bumps, no migration:** `REPLICA_FORMAT` 10 → 11, `DECK_VERSION` 11 → 12,
   `SOURCES_VERSION` 16 → 17, render-worker `PROTOCOL_VERSION` 10 → 11; older decks are refused.
   The phase 3 plan (`2026-11-28-data-entities-phase3.md`) claims the same deck, replica,
   sources and catalog bumps: if it lands first, this plan bumps from what `main` carries then
   (deck 13, replica 12, sources 18, catalog v8) and its tests name those numbers. Read the
   constants on `main` before Tasks 4, 5 and 7.
7. **Packs unchanged.** No `widgets.json` change, no new `base` version. An inline flat list in
   a pack example is creation input, lifted when the example is added to a deck.
8. **Panel, A2UI, agent, MCP.** A bound list's panel keeps `ListTable` and gains a data header:
   name, views sharing it, bind picker. `CATALOG_ID` v6 → v7 in Rust and TS in one commit.
   The Data tab and MCP `data_list` list list views; tool descriptions say how to bind a list.
9. **Session reads lists without waiting for a render** (review, 2026-11-28): the session
   resolves list views itself with `resolve_lists`; only base cells come from the worker,
   cached by base id. A cell the cache lacks (a based data never rendered yet) makes that list
   `pending`: its items are absent with `issue-list-pending` until the next render, never an
   older value.
10. **A list over a query data is read-only** (review, 2026-11-28), like the query grid:
    `edit_list` on it is refused with the existing `op-data-derived`, the canvas and
    `ListTable` offer no edit.

## Global Constraints

- Loro pinned at 1.17.1; `move_row` uses `LoroMovableList::mov`, resolving the row by id at
  apply (a stale index never reaches `mov`).
- No silent fallback: an unreadable binding, missing data, pending base cells or an invalid
  cell give an issue and an absent value, never a default, an older value or a guessed column.
- Ids: a list item id becomes a row id. `edit_list insert` with an id that is not a valid row id
  (`valid_id`: 1–64 ASCII letters, digits, `-`, `_`) or already taken is refused
  (`data-id-invalid` / `data-id-taken`); lifting keeps a valid unique item id and draws one for
  an invalid or repeated id (the item's position, not its id, is what the user sees).
- Size: lifting refuses more than `LIST_ITEMS_MAX` items or a cell past its param bound
  (refusal at prepare: `op-data-list-too-large`). A rich cell over `RICH_BYTES_MAX` bytes is
  refused before parsing; the reader walks events iteratively. The resolved items of all list
  views of one render count in `MAX_RESOLVED_CELLS` and `MAX_RESOLVED_BYTES`.
- Links in a rich cell follow the rich param rules: written through `encode_link_url` (Rust)
  and `encodeLink` (TS). Restricting link schemes is a rich-text-wide change, out of this plan.
- Fluent keys, all in `crates/oku-slides-i18n/src/locales/<20 locales>/core.ftl`, same order,
  checked by the existing key parity test: `op-data-list-nested`, `op-data-list-column`
  (`$column`, `$expected`), `op-data-list-inline` (an inline flat list reaching apply),
  `op-data-list-too-large` (`$max`), `rich-markdown-invalid` (`$reason`: `link`, `highlight`,
  `size`), `issue-list-cell-invalid` (`$column`, `$row`, the row id cut to 64 chars),
  `issue-list-cut` (`$shown`, `$rows`), `issue-list-pending`, `issue-list-inline-dropped`
  (`$param`). Panel strings in `ui.ftl` (Task 7). Tests check the key or variant and `fr`.
- New `DataFault` variants get their `DataFaultKind` and `kind()` arms; `DataFault` stays
  `pub(crate)`, public functions return `OpError`.
- Unit tests in sibling `x_tests.rs` with speculoos; new feature, so no regression file unless a
  task fixes an existing bug.
- `ARCHITECTURE.md` files of core, render, session, data, replica and render-worker are at
  50 lines: each task trims what it adds there. A rule change edits the nearest `AGENTS.md`.
- Session state lives outside `session/src/lib.rs` (901 lines); its mutexes lock with
  `unwrap_or_else(PoisonError::into_inner)`, never across an await.
- Every commit cites its task id.

## Review Focus

1. A data shared by a table and a list, edited in the grid with a value a `choice` column does
   not offer: the table shows it, the list renders that field absent with exactly one issue,
   other fields and rows render. → Task 4 `list_view_choice_outside_options_is_absent_with_issue`.
2. Markdown lezer and pulldown-cmark could read differently (emphasis next to punctuation, `==`
   inside a link, a leading `<`, a backslash): both parsers give the corpus result. → Task 1
   corpus cases `emphasis_flanking_punctuation`, `highlight_in_link_text`, `link_leading_lt`,
   `link_backslash`, required by `markdown_corpus_has_required_cases`.
3. Two peers: one moves an item, the other edits its text, or both move the same row: both
   edits survive, one row, one position. → Task 5 `concurrent_move_row_and_set_cell_both_survive`,
   `concurrent_move_row_same_row_keeps_one_row`.
4. A list bound to a 10,000-row data: the view cuts at the schema `max`, exactly one issue, the
   plan reads `max × mapped` cells. → Task 3 `list_view_over_large_data_is_cut_to_max`.
5. An agent writes an inline list into a list bound to a shared data: a new data is made, the
   shared one keeps its rows; an inline data with one user is replaced in place and keeps its
   id. → Task 5 `inline_list_on_shared_binding_makes_new_data`,
   `inline_list_on_sole_user_binding_replaces_in_place`.

---

### Task 1: Rich markdown both ways in core (t-1809)

**Files:**
- Create: `crates/oku-slides-core/src/rich_markdown/{mod.rs,write.rs,read.rs}` with
  `write_tests.rs`, `write_oracle_tests.rs`, `read_tests.rs`. `write.rs` is
  `session/src/inline.rs` (403 lines) + `paragraphs_to_text` of `session/src/text.rs`; their
  tests `inline_tests.rs`, `inline_oracle_tests.rs`, `text_tests.rs` move with them (the corpus
  writer under `UPDATE_CORPUS=1` included). Check `unicode-properties` is a core dependency
  before moving; add it from the workspace if not (the one allowed `Cargo.toml` line).
- Create: `crates/oku-slides-session/src/interp.rs` (+ `interp_tests.rs`): `interp`, today in
  `session/src/text.rs:63`.
- Delete: `crates/oku-slides-session/src/{text.rs,inline.rs}` and their tests.
- Modify: `session/src/lib.rs:92,154,226` (drop `mod inline`, `mod text`, the
  `paragraphs_to_text` re-export; keep `interp` exported), `session/src/fields.rs:11,112`,
  `session/src/agent/forms.rs:20,507,567`, `session/src/example_text.rs`,
  `session/tests/session.rs:6,144`, `session/tests/regressions/issue_0630_link_angle_bracket.rs:7`
  (all import `oku_slides_core::paragraphs_to_markdown`); `core/src/lib.rs` exports;
  `fixtures/markdown.json`, `fixtures/markdown-corpus.json` (new cases), `fixtures/AGENTS.md`;
  `ui/src/markdown.test.ts`; core `ARCHITECTURE.md`, `AGENTS.md`; `core.ftl` ×20.

**Interfaces:**
- Produces: `pub fn paragraphs_to_markdown(paras: &[Paragraph]) -> String`;
  `pub fn markdown_to_paragraphs(text: &str) -> Result<Vec<Paragraph>, RichFault>` where
  `pub enum RichFault { Link, Highlight, Size }` (light, `Copy`; `impl From<RichFault> for
  OpError` gives `rich-markdown-invalid`). Highlight digits: at most 3, parsed as `u8`.
- Core `AGENTS.md` rule: "Read a rich data cell only through `markdown_to_paragraphs`, write one
  only through `paragraphs_to_markdown`. (plan:2026-11-28-data-entities-phase1b)"

- [ ] **Step 1: Failing tests** (`read_tests.rs` unless named):
  `markdown_corpus_matches_core_reader` (every `markdown.json` case: `markdown` reads to
  `paragraphs`; `lossy` cases read to `reread`); `markdown_corpus_has_required_cases` (the four
  Review Focus 2 names exist); proptest `rewrite_then_parse_keeps_text`;
  `rewrite_of_reread_is_a_fixed_point`; `unsupported_syntax_is_kept_as_text` (HTML, image, code
  span, table, `>`, nested list); `rich_over_bytes_is_refused_before_parse`;
  `deep_nesting_does_not_overflow` (100,000 `>`); `highlight_index_over_255_is_refused`;
  `highlight_index_huge_digits_is_refused`; `rich_markdown_refusal_is_localized` (key + `fr`);
  the moved writer tests pass unchanged in core. TS: the existing `textToParagraphs` loop over
  the shared fixture (`ui/src/markdown.test.ts:21`) covers the new cases, no second loop.
- [ ] **Step 2:** `just test` → the new tests FAIL. **Step 3:** Implement; regenerate
  `markdown-corpus.json` with `UPDATE_CORPUS=1`. **Step 4:** `just test` → PASS. Stage; commit
  `feat(core): rich markdown read and write in core (t-1809)`.

### Task 2: List bindings in core (t-1810)

**Files:**
- Create: `crates/oku-slides-core/src/list_binding.rs` (+ `list_binding_tests.rs`).
- Modify: `core/src/ops/data.rs:248-280` (`bind_data`: the `ParamType::List => NotView` arm at
  :259 becomes the list branch), `core/src/ops/data_tests.rs:237` (the `NotView` test now uses a
  non-view param), `core/src/data_users.rs`, `core/src/error/data.rs` (`ListNested { param }`,
  `ListColumn { column, expected }`, their `DataFaultKind` and `kind()` arms),
  `core/src/lib.rs` exports, `core.ftl` ×20.

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
  pub struct ListBinding { pub data: DataId, pub fields: BTreeMap<String, String> }
  impl ListBinding { pub fn from_value(value: &Value) -> Option<Self>; } // None: not a binding
  pub fn list_fields(item: &[ParamSchema], columns: &[Column]) -> Result<BTreeMap<String, String>, OpError>;
  pub fn list_bind_choices(deck: &Deck, item: &[ParamSchema], current: Option<&ListBinding>) -> Vec<DataId>;
  ```
- `bind_data` on a list param: nested → `ListNested`; `columns` given →
  `DataError::columns_invalid`; else `list_fields` over the data's columns (over a query data:
  `session/src/data_panel.rs` `check_query_binds`, Task 6) and the param set to the binding.
  `from_value` also rejects a `fields` map with more entries than schema columns.
  `list_bind_choices` keeps data whose columns pass `list_fields`, leaving out truncated data as
  `bind_choices` does. `data_users` reads a list param through `from_value`: a binding is a
  view with `columns: Some(mapped ids)`; an object with `data` that `from_value` rejects is a
  view with `columns: None` (it still blocks `remove_data`).

- [ ] **Step 1: Failing tests:** `bind_list_matches_columns_by_name`;
  `bind_list_with_missing_column_is_refused` (key + `fr`, `$column`, `$expected`);
  `bind_list_text_column_to_flag_param_is_refused`; `bind_list_date_column_to_plain_is_accepted`;
  `bind_nested_list_is_refused`; `bind_list_with_columns_is_refused`;
  `list_binding_is_a_data_user_with_mapped_columns`; `renamed_mapped_column_keeps_binding`;
  `remove_mapped_column_is_refused`; `remove_data_used_by_list_view_is_refused` (key + `fr`);
  `rebind_list_to_other_data_replaces_binding`; `list_bind_choices_excludes_incompatible_data`;
  `list_binding_with_unknown_field_is_rejected`; `malformed_list_binding_still_blocks_remove_data`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(core): bind list params to data (t-1810)`.

### Task 3: Plan list views (t-1812)

**Files:**
- Modify: `crates/oku-slides-core/src/views.rs` (`ViewPlan.host` → `kind: ViewKind`; a second
  loop branch for `ParamType::List` params), `core/src/ops/refs.rs:169` (`dependent_views`:
  `ViewKind::List` lists the view, no option patch), `session/src/data_edit.rs:305,392`
  (`DataTarget.host` → `kind`; the `_` arm becomes `ViewKind::Table`, `ViewKind::List` patches
  nothing: column follow skips list views), `session/src/charts.rs:133,161` and
  `session/src/data_panel.rs:14` (match `ViewKind`, list views skipped where only charts and
  tables apply), `render/src/views.rs` (chart and table branches match `kind`),
  `core/src/views_tests.rs`, core `ARCHITECTURE.md`.

**Interfaces:**
- Consumes: Task 2 `ListBinding::from_value`.
- Produces: `pub enum ViewKind { Chart, Table, List }` (`HostType` keeps `Path`, which is no
  view). A list view plans its mapped columns in schema order and rows
  `0..min(rows, max, LIST_ITEMS_MAX)`, `cut` past them; its cells and text bytes count in the
  budget; an unreadable binding plans `Unreadable`. Over a query data it plans `Derived` as
  charts do.

- [ ] **Step 1: Failing tests** (`views_tests.rs`): `list_view_plans_mapped_columns_in_schema_order`;
  `list_view_over_large_data_is_cut_to_max` (10,000-row data; exactly one `issue-list-cut`,
  `cells == max × mapped`); `list_view_at_max_is_not_cut`;
  `list_view_without_schema_max_cuts_at_list_items_max`; `list_view_counts_in_render_budget`;
  `list_view_missing_data_plans_missing`; `list_view_unreadable_binding_plans_unreadable`;
  `dependent_views_lists_list_views`; session `column_rename_does_not_patch_list_view`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(core): plan list views (t-1812)`.

### Task 4: Resolve list views; worker protocol 11 (t-1813)

**Files:**
- Create: `crates/oku-slides-render/src/list_views.rs` (+ `list_views_tests.rs`);
  `crates/oku-slides-core/src/list_view.rs` (+ tests): `ResolvedLists`, `fill_lists`.
- Modify: `render/src/views.rs` (`resolve_views` fills bound list params from
  `resolve_lists`), `render/src/lib.rs`, `render-worker/src/protocol.rs` (`PROTOCOL_VERSION =
  11`; `CompiledHeader.base_cells`), `render-worker/src/serve.rs`, `render-worker/src/compiled.rs`
  (`RemoteCompiled::base_cells`), `render-worker/src/serve_tests.rs:377`,
  `render-worker/src/data_worker.rs:26` (cache key follows the version: cached query results are
  recomputed once), `render/tests/lists.rs`, `render/tests/nested_lists.rs`, render and
  render-worker `ARCHITECTURE.md` (the protocol number at :32), `core.ftl` ×20.

**Interfaces:**
- Consumes: Task 3 plans; Task 1 `markdown_to_paragraphs`; `BaseRead` (`render/src/views.rs:28`).
- Produces:
  ```rust
  // core/src/list_view.rs
  pub type ResolvedLists = BTreeMap<ViewKey, ResolvedList>; // ViewKey = ViewRef without columns, Ord
  pub enum ResolvedList { Items(Vec<ListItem>), Pending }
  pub fn fill_lists<'a>(deck: &'a Deck, lists: &ResolvedLists) -> Cow<'a, Deck>; // never touches nested lists
  // render/src/list_views.rs
  pub fn resolve_lists(deck: &Deck, registry: &WidgetRegistry, plans: &[ViewPlan],
      bases: Option<&dyn BaseRead>, results: &Results) -> (ResolvedLists, Vec<Issue>);
  pub struct BaseCells { pub base: String, pub cells: BTreeMap<CellKey, DataCell> }
  ```
  Each planned row becomes a `ListItem` (id = row id) whose fields, in schema order, are the
  mapped cells converted to the column param; a cell failing decision 4 is left out with one
  `issue-list-cell-invalid`. A based data with `bases: None` gives `Pending` with
  `issue-list-pending`. A query data reads its result from `results` as charts do. The worker
  returns in `base_cells` the base cells its list views read (bounded by the plan) and checks
  the frame size as other header fields. Measure `fill_lists` on a 256-item list in the test
  run; it clones only the slides it fills.

- [ ] **Step 1: Failing tests** (`list_views_tests.rs` unless named):
  `frise_bound_list_renders_like_inline` (`render/tests/lists.rs`: same SVG as the inline
  render of the same items, and the same item ids); `list_view_choice_outside_options_is_absent_with_issue`;
  `list_view_plain_over_length_is_absent_with_issue`; `list_view_non_finite_number_is_absent_with_issue`;
  `list_view_rich_cell_renders_paragraphs`; `list_view_bad_rich_cell_is_absent_with_issue`;
  `list_view_missing_asset_is_absent_with_issue`; `list_view_asset_traversal_value_is_absent_with_issue`
  (`../x`); `list_view_retyped_column_is_absent_with_issue`;
  `list_view_after_column_rename_renders_same_items`; `list_view_over_empty_data_renders_empty_without_issue`;
  `list_view_with_huge_cells_is_cut_with_issue`; `list_view_over_query_result_resolves`;
  `list_view_query_dropping_column_is_absent_with_issue`;
  `list_view_over_base_without_reader_is_pending`; `list_view_cut_issue_is_localized` (key + `fr`);
  `fill_lists_leaves_nested_lists_alone`; `nested_plan_widget_still_renders_inline`
  (`render/tests/nested_lists.rs`); `worker_hello_with_protocol_10_is_refused`;
  `worker_returns_base_cells_of_list_views`. The issue tests assert key, `$column`, `$row`, and
  that the other fields and rows render.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(render)!: resolve list views, worker protocol 11 (t-1813)`.

### Task 5: Lift inline lists, rewrite list edits; format 12 (t-1811)

**Files:**
- Create: `crates/oku-slides-core/src/prepare_list.rs` (+ tests): the lift and the
  `edit_list` → `edit_data` rewrite; `prepare_view.rs` (297 lines) only calls it.
- Modify: `core/src/prepare.rs` (+ the weight match at :93), `core/src/ops/list.rs`
  (`seed_lists`/`seed_element`/`seed_slots` :480-501 keep nested lists only; slot recursion
  for flat lists moves to the lift), `core/src/ops/edit.rs:53,145` (callers),
  `core/src/ops/mod.rs` (apply refuses an inline flat list: `op-data-list-inline`),
  `core/src/validate.rs` (a loaded flat list holding inline items is dropped with
  `issue-list-inline-dropped` before reading its items),
  `crates/oku-slides-data/src/{table_op.rs,rows.rs}` (`MoveRow`; the four matches at
  table_op.rs:91,113,130,148), `core/src/replica/data.rs` (move in the `rows` movable list),
  `session/src/data_compact.rs:181`, `session/src/data_edit.rs:222` (coalescing key),
  `core/tests/convergence.rs:174` (`Edit::MoveRow`), `core/tests/lists.rs` (flat inline cases
  ported to bound lists, nested kept), `crates/oku-slides-mcp/src/tools.rs:396` (`move_row` in
  the `apply` description), the `TableOp` JSON schema, versions `core/src/model.rs:31`
  (`DECK_VERSION = 12`), `core/src/replica/mod.rs:92` (`REPLICA_FORMAT = 11`),
  `core/src/sources.rs:56` (`SOURCES_VERSION = 17`), `core/src/model_tests.rs`,
  `core/src/sources_tests.rs:711`, `replica/ARCHITECTURE.md:50`, data `ARCHITECTURE.md`,
  `core.ftl` ×20, `ui/src/slideTypes.ts:229` and `ui/src/chart/dataEdit.ts` (`move_row` in the
  `TableOp` union; the `edit_list` op type is unchanged).
- Fixtures: every deck fixture whose version changes is rewritten by its own test's update
  path; list them with `rg -l '"version": 11' fixtures crates` before editing.

**Interfaces:**
- Consumes: Task 1, Task 2 `ListBinding`, Task 3 `ViewKind::List`.
- Produces: `TableOp::MoveRow { row: String, after: Option<String> }` (refused on a derived
  data as other row ops). An inline flat list (or a missing one with a schema default) becomes
  a `PutViewData` whose table has one column per schema column (Text/Number/Bool by param
  type), named after it, in schema order, one row per item; the param becomes a `ListBinding`.
  In-place replace only for an inline data with one user. `edit_list` on a bound param becomes
  `Op::EditData`: `insert { index }` → `after` the row at `index - 1` among the shown rows
  (`index` 0 → `after: None`; past the shown count → refused), refused when the shown count is
  at `max`; `remove` refused at `min` (shown count); `move { to }` → `after` the row shown at
  `to - 1` after removal; a row id no longer in the data is refused, never a no-op;
  `set_field` converts per column (rich: paragraph array or markdown string, stored as
  `paragraphs_to_markdown` of its parse). Over a query data: `op-data-derived`.

- [ ] **Step 1: Failing tests** (`prepare_list_tests.rs` unless named):
  `add_element_with_inline_list_lifts_to_data`; `set_param_inline_list_lifts`;
  `replace_element_inline_list_lifts`; `add_slide_inline_list_lifts`;
  `new_element_list_default_becomes_data_named_after_param`; `second_lift_draws_etapes_2`;
  `lift_repeated_item_id_draws_new_id`; `inline_list_over_cap_is_refused_at_prepare`;
  `inline_list_on_shared_binding_makes_new_data` (shared rows unchanged);
  `inline_list_on_sole_user_binding_replaces_in_place`; `edit_list_set_field_becomes_set_cell`;
  `edit_list_rich_paragraphs_store_markdown`; `edit_list_rich_markdown_string_is_stored_parsed`;
  `edit_list_rich_bad_markdown_is_refused` (key + `fr`); `edit_list_insert_invalid_id_is_refused`;
  `edit_list_insert_taken_id_is_refused`; `edit_list_insert_after_row_orders_correctly`;
  `edit_list_insert_index_out_of_range_is_refused`; `edit_list_insert_past_max_is_refused`;
  `edit_list_remove_becomes_remove_row`; `edit_list_move_to_first_has_no_after`;
  `edit_list_move_to_same_place_keeps_order`; `edit_list_stale_row_is_refused`;
  `edit_list_on_query_list_is_refused` (`op-data-derived`); `nested_list_edit_list_is_unchanged`;
  `inline_flat_list_reaching_apply_is_refused`; `loaded_inline_flat_list_is_dropped_with_issue`
  (`validate_tests.rs`); `old_replica_format_10_is_refused`; `old_deck_version_11_is_refused`;
  `old_sources_version_16_is_refused`; data `move_row_after_unknown_row_is_refused`; replica
  `concurrent_move_row_and_set_cell_both_survive`, `concurrent_move_row_same_row_keeps_one_row`,
  `concurrent_move_and_remove_row_removes`; `undo_of_move_row_restores_order`;
  `undo_of_lift_removes_data_and_binding_together`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(core)!: inline lists lifted to data, list edits as data edits, deck format 12 (t-1811)`.

### Task 6: Session reads bound lists (t-1814)

**Files:**
- Create: `crates/oku-slides-session/src/list_cache.rs` (+ tests): base-cell cache by base id
  (a `BaseRead` over cached cells; a missing cell is a read error → `Pending`), filled from
  each `RemoteCompiled::base_cells`, emptied on deck switch.
- Modify: `session/src/lib.rs` (one field), `session/src/session_view.rs`,
  `session/src/fields.rs`, `session/src/items.rs`, `session/src/element_panel.rs` (reads go
  through `fill_lists` of `resolve_lists(…, Some(&cache), results)`; fields of a list over a
  query data are read-only), `session/src/data_panel.rs` (`check_query_binds` covers list
  binds), `session/src/data_list.rs` (list views among users), `session/src/widget_examples.rs`
  (adding an example to a deck goes through `prepare_ops`, so its inline list is lifted;
  `edit_example_list` at :232 stays on example JSON), `session/src/test_support.rs`,
  `mcp/tests/widgets.rs` and `render/tests/common/mod.rs` users of `frise-test`, session
  `ARCHITECTURE.md`.

**Interfaces:**
- Consumes: Task 4 `resolve_lists`, `fill_lists`, `RemoteCompiled::base_cells`; Task 5 rewrite.
- Produces: every read of list items for fields, item boxes or `ListTable` rows goes through
  the filled deck; writes stay ops on the real deck.

- [ ] **Step 1: Failing tests:** `canvas_edit_of_bound_list_field_sets_cell`;
  `canvas_edit_of_row_added_since_last_render_applies` (no render between);
  `bound_list_reads_after_failed_render` (fresh items, no stale value);
  `based_list_without_cached_cells_is_pending_with_issue`; `item_boxes_of_bound_list_carry_row_ids`;
  `element_list_insert_on_bound_list_adds_row`; `query_list_fields_are_read_only`;
  `bind_list_to_query_without_result_is_refused`; `paste_element_with_list_copies_its_data`;
  `paste_two_elements_sharing_list_data_copies_it_once`; `paste_list_with_taken_data_name_draws_new_name`;
  `paste_list_with_missing_asset_cell_keeps_issue`; `cut_paste_keeps_list_binding`;
  `pack_example_with_inline_list_is_lifted_on_add`; `data_tab_lists_list_views_as_users`;
  `undo_of_list_edit_restores_cell`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(session): bound lists in fields, items and panels (t-1814)`.

### Task 7: Panel data header, catalog v7 (t-1815)

**Files:**
- Modify: `crates/oku-slides-a2ui/src/{component.rs,message.rs,message_tests.rs:77}`
  (`ListTable.data: Option<ListData>`, skip if none; `CATALOG_ID` v7),
  `a2ui/ARCHITECTURE.md:4`, `session/src/element_panel.rs`, `ui/src/a2ui/catalog.tsx`
  (`CATALOG_ID`; `ListTableApi.data`), `ui/src/a2ui/ListTable.tsx` (header built from
  `ui/src/primitives.tsx`; names as text nodes only; `bind_action` through `sendWithOutcome`;
  read-only when `data.read_only`; no local state), `ui.ftl` ×20 (`list-data-header`,
  `list-data-shared` with `$count`, `list-data-bind`, `list-data-read-only`); remove the
  helpers of `ui/src/a2ui/listNested.ts` / `liftPatch.ts` that flat lists no longer use.
- Regenerate: `ui/src/a2ui/fixtures/agent-surfaces.json` (`OKU_UPDATE_FIXTURES=1`),
  `ui/src/a2ui/fixtures/playbook.json`, `session/src/agent/snapshots/*.snap` (`cargo insta`);
  `rg 'catalog:v6'` returns nothing afterwards.
- Tests: `ui/src/a2ui/ListTable.render.test.tsx`, `ListTableNested.render.test.tsx`,
  `catalog.schemas.render.test.tsx:110`; e2e `ui/e2e/155-list-data.cjs` (after the data-panel
  helpers of 150; read `docs/agents/ui-e2e.md` first).

**Interfaces:**
- Produces: `pub struct ListData { pub name: String, pub views: Vec<String>, pub choices:
  Vec<DataChoice>, pub read_only: bool, pub bind_action: Action }` (`DataChoice` as the chart
  picker's); zod: `z.object({name: z.string(), views: z.array(z.string()).max(64), choices:
  z.array(DataChoice).max(256), read_only: z.boolean(), bind_action: Action}).strict()`,
  optional.

- [ ] **Step 1: Failing tests:** session `list_panel_shows_data_header_and_choices`,
  `list_panel_bind_event_binds_data`, `list_panel_over_query_is_read_only`;
  `catalog_id_is_v7_in_rust_and_ts`; UI `ListTable shows the data name and shared views`,
  `ListTable bind picker emits bind`, `ListTable over a query offers no edit`,
  `ListTable renders a markup-like data name as text` (`<img onerror>`, `javascript:`),
  `nested ListTable shows no data header`, `ListTable schema refuses an unknown key in data`,
  `ListTable from the Rust session passes its catalog schema` (updated); e2e 155: add a frise,
  edit a date on the canvas, see the row changed in the Data tab, move and add an item, bind a
  second frise to the same data, edit one, see both change.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test`; `just e2e <free
  port>` with fresh temp `OKU_SLIDES_PATHS__DATA` / `__DECKS` (e2e 140 updated for bound
  frise lists) → green. Stage; commit `feat(ui)!: list data header, catalog v7 (t-1815)`.

### Task 8: MCP and agent (t-1816)

**Files:** Modify `crates/oku-slides-mcp/src/data_tools.rs` (views list list views with their
fields), `crates/oku-slides-mcp/src/tools.rs` (`bind_data` says nested lists are refused and
rich cells hold markdown), `crates/oku-slides-mcp/tests/widgets.rs`, MCP `AGENTS.md` if a rule
changes (else no change). The agent instructions name no view kinds (checked with
`rg -i 'chart.*table' crates/oku-slides-agent` before editing; edit only what it finds).

- [ ] **Step 1: Failing tests:** `data_list_names_list_views`;
  `agent_set_param_inline_list_creates_data`; `agent_set_param_inline_list_on_nested_param_stays_inline`;
  `agent_bind_list_by_name`; `agent_bind_nested_list_fails_with_localized_error` (key + `fr`).
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(mcp): list views for the agent (t-1816)`.

### Task 9: Gate (t-1817)

- [ ] **Step 1:** Each touched `ARCHITECTURE.md` ≤ 50 lines and current; core `AGENTS.md` list
  rule gains "a flat list param is a data view; a nested one stays inline";
  `typst/oku-slides/AGENTS.md` unchanged (widgets read the same shape). Spec "Plan calls (phase
  1b plan)" matches what landed.
- [ ] **Step 2:** `just ci`, `just e2e <free port>` with fresh temp dirs → green; whole-branch
  review; land per AGENTS.md; close t-1809 … t-1817 and t-1807.

## Deferred (backlog after landing)

- Restrict link schemes in all rich text (params and cells, Rust and TS).
- A rich cell rendered absent shows its issue in `ListTable`, so a first edit does not overwrite
  markdown the user cannot see.
- Two peers lifting at once may both name a data `etapes_2`; names are labels, ids stay unique.
- Undo of a list edit on a shared data also changes its other views (by design; document in help).
