# Data entities, phase 2 (derived data) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A data can be derived from other data by a SQL query (DuckDB-aligned subset), run by
polars in the sandboxed worker, recomputed when its inputs change, and cached in the deck.

**Architecture:** `oku-slides-data` gains a SQL front end (sqlparser, no engine: parse,
whitelist, input names, rename rewrite, engine rewrites) that core uses to validate ops, and an
engine `run_query` (polars, render worker only) that writes the result as a Parquet base. The
session keeps each derived data's result out of Loro (`QueryState`), recomputes dependents in
topological order on a dedicated query link, feeds results to the renderer like manual data, and
saves them as content-addressed cache entries in the sources zip.

**Tech Stack:** Rust (sqlparser 0.60 with the DuckDB dialect, polars 0.53 + `lazy` + `sql` if Task 1
confirms it), snafu, serde, Loro 1.17.1; React/TS UI (A2UI catalog v6); DuckDB as a test-only oracle.

**Spec:** `docs/superpowers/specs/2026-11-21-data-entities-design.md` (phase 2 of "Phases":
"Derived data: crate, validation, worker, recompute, cache"). Tags and `AT (VERSION => …)` are
phase 3; pack-widget lists are phase 1b. Phase 1 plan: `2026-11-21-data-entities-phase1.md`.

## Decisions (accepted by the user, 2026-11-26)

All four proposals below were accepted as written. Task 1 still runs: it fixes the whitelist
(native constructs plus cheap rewrites; the rest deferred and named in the spec) and records the
cost, without a further stop.


1. **polars `lazy` + `sql`** (left open by phase 1, which kept only `dtype-date` and `parquet`).
   Accepted: `lazy` + `sql` are enabled (Task 5); Task 1 measures binary size, cold build and
   coverage.
2. **Results as Parquet bases** (deviates from the spec's `data/<hex>.json` result entries):
   a result of up to 65,536 × 1,000 cells does not fit a JSON entry. This plan stores each result
   as a base in the store and in the zip (`bases/<hex>.parquet`, as phase 1), with a small
   `data/<key hex>.json` entry naming it. Views and downstream queries then read results exactly
   like manual data.
3. **DuckDB oracle cost**: `duckdb` (bundled) is a large C++ build. Proposed: a test-only
   dependency behind feature `oracle` of `oku-slides-data`, run by a `just test-oracle` recipe
   that `just ci` calls, not by `just test`.
4. **Cross-deck paste of a view over derived data**: its inputs are not in the target deck.
   Proposed: a paste within the same deck copies the query (`ventes_2`, same SQL); a paste into
   another deck copies the current result as manual data and says so in the paste reply.

## Global Constraints

- polars pinned `=0.53.0`; sqlparser pinned `=0.60.0` (the version polars-sql already locks); no
  new duplicate in `cargo tree -d`; `cargo tree -e features -i polars` → only data via render-worker.
- Parquet and query execution only in the render worker; the host parses SQL text (sqlparser)
  but never runs it.
- SQL: one `SELECT` (with `WITH`, set operations, subqueries); tables are deck data names only.
  Refused: table functions and file paths, `ATTACH`, DDL/DML, `ASOF` / `POSITIONAL`, `SAMPLE`,
  `SUMMARIZE` / `DESCRIBE`, FROM-first, lists, structs, lambdas, dot-call chaining. Anything the
  whitelist does not name is refused with a Fluent message naming the construct. Phase 2 also
  refuses `AT (VERSION => …)` (`data-query-construct`, phase 3).
- `MAX_SQL_BYTES = 16_384`; sqlparser recursion limit 64; a parse error names line and column,
  never quotes the SQL.
- Results obey the stored-data bounds (`MAX_ROWS`, `MAX_COLUMNS`, `MAX_TEXT_BYTES`): past them the
  query fails (`data-query-too-large`), never truncated.
- Result column types: polars `String`→text, `Float32/64`→number, `Int8..Int64`, `UInt8..UInt32`
  →integer, `UInt64`→integer if every value fits `i64` else refused, `Boolean`→bool, `Date`→date,
  `Null`→text; any other dtype refused (`data-query-column-type`, `$column`). Column ids are the
  output names; duplicate output names refused (`data-query-duplicate-column`).
- A failed query puts the data in error state: views show the placeholder and one issue; no last
  good result is shown (no silent fallback).
- Versions: `DECK_VERSION` 10 → 11, `REPLICA_FORMAT` 9 → 10, `SOURCES_VERSION` 15 → 16,
  render-worker `PROTOCOL_VERSION` 8 → 9, `CATALOG_ID` `…:catalog:v6` (Rust and TS, same commit).
  Old decks refused, no migration.
- Fluent keys: data model and query errors `data-query-*` in `data.ftl`; op refusals `op-data-*`
  (core); issues `issue-data-query-*`; session `session-data-query-*`; all 20 locales, same order.
- Query link settings (`[render]`): `query_timeout_ms = 30000`, `query_memory_mib = 4096`.
- Every commit cites its task (`t-N`, one per Task, children of t-1690).

## Review Focus

1. A query over a data the other copy removed or renamed after a merge: error state with an issue
   naming the missing input, no panic, other data still render. → Task 4 test
   `merged_query_reading_removed_data_is_in_error_with_issue`.
2. Deeply nested or huge SQL from a peer or the agent (parsed in the host): refused with
   `data-query-too-deep` / `data-query-too-long`, never a stack overflow. → Task 3 rstest
   `parse_refuses_nesting_past_limit` (65 nested subqueries) and `parse_refuses_sql_over_16_kib`.
3. An edit storm on an input (typing in the grid): one recompute in flight per data, the last
   input state wins, no result older than the inputs is shown as current. → Task 6
   `recompute_after_burst_of_edits_matches_last_inputs`.
4. A query that never ends or eats memory: the query link times out, the data is in error
   (`data-query-timeout`), slide rendering is not blocked. → Task 5
   `slow_query_times_out_without_blocking_render`.
5. A tampered cache entry (wrong columns, missing base, base hash mismatch): ignored and
   recomputed, reported once in the log, never shown. → Task 7
   `tampered_cache_entry_is_recomputed`.

---

### Task 1: Spike — polars-sql coverage and cost (throwaway; decision gate)

**Files:** throwaway branch only; output: update the spec's "Phases" item 0 and a new
"Phase 2 spike" table in the spec.

- [ ] **Step 1:** Enable `lazy` + `sql` on polars in a throwaway branch; record release binary
  delta and cold build time (`cargo build --release -p oku-slides-app` vs main).
- [ ] **Step 2:** For each construct of the spec's language list (standard SELECT, joins,
  window functions, `WITH`, set ops, `PIVOT`, `UNPIVOT`, `GROUP BY ALL`, `ORDER BY ALL`,
  `* EXCLUDE/REPLACE/RENAME`, `COLUMNS('re')`, `QUALIFY`, `UNION BY NAME`, `min_by`/`max_by`,
  aggregate `FILTER`, select aliases reused in `WHERE`/`GROUP BY`/`HAVING`, date functions) run
  one query in DuckDB and in polars-sql 0.53 on the same frames; note: native / rewrite needed /
  unsupported, and whether sqlparser 0.60 `DuckDbDialect` parses it.
- [ ] **Step 3:** Propose the phase 2 whitelist (native + cheap rewrites; the rest deferred, named
  in the spec). The whitelist is final once recorded (decision 1 accepted). Commit the spec
  update `docs(data): phase 2 spike findings (t-N)` (`Review-waived: docs-only`).

### Task 2: `Data` gets a query kind; replica and formats

**Files:**
- Modify: `crates/oku-slides-data/src/data.rs` (+ `data_tests.rs`), `ARCHITECTURE.md`
- Modify: every `DataKind::Manual` site: `core/src/ops/data.rs:42`, `core/src/ops/view_data.rs:73`,
  `core/src/replica/data_read.rs:144`, `core/src/replica/data.rs:153`, `session/src/data_list.rs:138`,
  `mcp/src/data_tools.rs:98`; every `data.table` reader (compiler-guided).
- Modify: `core/src/model.rs:31`, `core/src/replica/mod.rs:91`, `core/src/sources.rs:52`, fixtures.

**Interfaces:**
- Produces:
  ```rust
  pub enum DataKind { Manual, Query }                       // serde "manual" | "query"
  pub enum Source { Manual(Table), Query { sql: String } }  // data.rs
  pub struct Data { pub name, pub label, pub inline, pub source: Source }
  impl Data { pub fn kind(&self) -> DataKind; pub fn table(&self) -> Option<&Table>;
              pub fn table_mut(&mut self) -> Option<&mut Table>; pub fn sql(&self) -> Option<&str> }
  ```
  JSON stays flat: `{"name","label"?,"inline"?,"kind":"manual","table":{…}}` or
  `{"name",…,"kind":"query","sql":"…"}`, through a `DataRepr` (`deny_unknown_fields`,
  `try_from`): `table` without `kind: manual` or `sql` without `kind: query` is refused
  (`data-source-mismatch`). `sql` bounded by `MAX_SQL_BYTES` while parsing.
- Replica: data map keeps `kind` (now read, not assumed) and gains atom `sql` (LWW); a query
  data has no `table` container. An unknown `kind` string reads as an issue
  (`issue-data-kind-unknown`) and the data is dropped, per the `deck()` infallible rule.

- [ ] **Step 1: Failing tests:** `query_data_json_round_trips_without_table`;
  `data_with_table_and_kind_query_is_refused`; `sql_over_max_bytes_is_refused_while_parsing`;
  replica `query_data_round_trips_through_replica`; `concurrent_set_sql_resolves_last_writer`;
  `unknown_kind_is_dropped_with_issue`; `old_replica_format_9_is_refused`;
  `every_task2_key_formats_in_all_locales`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-data -p oku-slides-core` → FAIL.
- [ ] **Step 3:** Implement; manual-only paths (`edit_data`, `set_column_type`, compaction,
  paste, views) take `table()` and treat a query data as Task 4/6 define (until then: refused
  with `op-data-derived` / not compacted).
- [ ] **Step 4:** `just test` → PASS. Stage; commit `feat(data)!: query kind in the data model, format 10 (t-N)`.

### Task 3: SQL front end (`oku-slides-data::sql`, no engine)

**Files:** Create `crates/oku-slides-data/src/sql/{mod.rs,whitelist.rs,rename.rs,rewrite.rs}`
with side-by-side `_tests.rs`; Modify `error.rs`, `limits.rs`, `data.ftl` (20 locales), `Cargo.toml`
(`sqlparser = "=0.60.0"`, workspace).

**Interfaces:**
- Produces:
  ```rust
  pub struct Query { /* parsed, whitelisted AST */ }
  pub fn parse_query(sql: &str) -> Result<Query, DataError>;
  impl Query {
      pub fn inputs(&self) -> BTreeSet<String>;           // table names, lowercased
      pub fn renamed(&self, old: &str, new: &str) -> Query;
      pub fn to_sql(&self) -> String;                       // sqlparser Display
      pub fn for_engine(&self) -> Result<EngineQuery, DataError>; // Task 1's rewrites applied
  }
  pub struct EngineQuery { pub sql: String, pub pivots: Vec<PivotStep> } // serde, sent to worker
  ```
  `DataErrorKind` gains `QueryParse { line, column }`, `QueryConstruct { construct }`,
  `QueryTooLong`, `QueryTooDeep`, `QueryUnknownFunction { function }`; Task 5 adds the runtime ones.
- Whitelist walks the AST with an explicit stack (never recursion past the parser's limit);
  functions are an explicit list (`whitelist.rs`, from Task 1).

- [ ] **Step 1: Failing tests:** rstest `parse_accepts_whitelisted` (one case per Task 1
  construct); rstest `parse_refuses_construct_named` (`read_csv('x')`, `'x.parquet'`, `ATTACH`,
  `INSERT`, `CREATE`, `SAMPLE`, `ASOF JOIN`, `FROM t SELECT *`, `[1,2]`, `{'a':1}`, `x -> x`,
  `t.col.upper()`, `AT (VERSION => 'x')`, two statements) → `QueryConstruct` naming it;
  `parse_refuses_unknown_function_named`; `parse_refuses_nesting_past_limit`;
  `parse_refuses_sql_over_16_kib`; `parse_error_names_line_and_column_not_text_fr`;
  `inputs_are_lowercased_table_names_not_cte_names`;
  `renamed_rewrites_tables_only_not_columns_or_ctes_of_same_name`;
  `to_sql_of_renamed_parses_back_equal`; one test per rewrite of Task 1.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-data` → FAIL. **Step 3:** Implement.
- [ ] **Step 4:** `just test` → PASS; `cargo tree -i sqlparser` → one version. Stage; commit
  `feat(data): SQL front end with whitelist and rename (t-N)`.

### Task 4: Query ops in core

**Files:** Modify `core/src/ops/{mod.rs,data.rs}`, `core/src/prepare.rs`, `core/src/data_users.rs`,
`core/src/error/data.rs`, `core/src/validate_data.rs`, `core/src/views.rs`, `core.ftl`,
`session/src/data_ops.rs` (undo label), `session.ftl`; tests side by side and a regression file.

**Interfaces:**
- Consumes: Task 2 `Source`, Task 3 `parse_query`, `Query::{inputs, renamed, to_sql}`.
- Produces:
  - `Op::AddData { id, name, label, table: Option<NewTable>, sql: Option<String> }` — exactly one
    of `table`/`sql` (`op-data-source`); `Op::SetQuery { data: DataId, sql: String }`.
  - `impl Deck { pub fn data_readers(&self, id: &DataId) -> Vec<DataId>; pub fn query_order(&self) -> QueryOrder }`
    with `pub struct QueryOrder { pub order: Vec<DataId>, pub broken: BTreeMap<DataId, QueryFault> }`,
    `pub enum QueryFault { Cycle, MissingInput(String), Invalid(DataError) }` (merged states).
  - `DataFault` gains `Derived { data }` (edit/retype/compact of a query data), `Cycle { data }`,
    `UnknownInput { name }`, `NotQuery { data }`; `InUse` also lists reading queries.
- Rules: `add_data`/`set_query` parse, check inputs exist and no cycle; `rename_data` rewrites
  every reader's SQL in the same op (reader's `sql` atom rewritten); `remove_data` refused while a
  view or query reads it; `check_external` weight of a query op = 1 + `sql.len() / 1024`.
- Validate: `issue-data-query-cycle`, `issue-data-query-missing-input` (`$data`, `$input`),
  `issue-data-query-invalid`; `plan_views` gives a view over a query data `PlanOutcome::Derived`
  (resolved by Task 6).

- [ ] **Step 1: Failing tests:** `add_query_data_reading_existing_data_applies`;
  `add_data_with_table_and_sql_is_refused`; `add_query_with_unknown_input_is_refused_fr`;
  `set_query_making_cycle_is_refused_and_names_data`; `set_query_on_manual_data_is_refused`;
  `edit_data_on_query_data_is_refused`; `rename_data_rewrites_reader_sql_one_undo_step`;
  `remove_data_read_by_query_is_refused_and_lists_it`;
  `merged_query_reading_removed_data_is_in_error_with_issue` (two replicas);
  `merged_cycle_marks_each_data_on_it`; `undo_set_query_restores_sql`;
  `ops_schema_validates_add_data_with_sql_and_set_query`; `every_task4_key_formats_in_all_locales`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-core -p oku-slides-session` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage; commit
  `feat(core): query data ops, dependency order and merged-state issues (t-N)`.

### Task 5: Engine `run_query`, worker request, query link

**Files:** Create `crates/oku-slides-data/src/engine/query.rs` (+ tests, oracle tests under
`tests/oracle.rs`, feature `oracle`); Modify root `Cargo.toml` (polars features per Task 1),
`render-worker/src/{protocol.rs,serve.rs,data_worker.rs}`, `platform/src/config.default.toml`,
`platform/src/config.rs`, `session/src/data_link.rs`, `justfile` (`test-oracle`, in `ci`).

**Interfaces:**
- Consumes: Task 3 `EngineQuery`.
- Produces:
  ```rust
  pub struct QueryInput<'a> { pub name: &'a str, pub table: &'a Table, pub base: Option<File> }
  pub fn run_query(q: &EngineQuery, inputs: Vec<QueryInput<'_>>, out: &mut File) -> Result<QueryOut, DataError>;
  pub struct QueryOut { pub base: BaseRef, pub columns: Vec<Column> }   // ids = output names
  ```
  Protocol 9: `Request::RunQuery { query: EngineQuery, inputs: Vec<QueryInputRef>, out: PathBuf }`
  (`QueryInputRef { name, base: Option<PathBuf> }`, each input's `Data` JSON as a frame, as
  `CompactData`); reply `Response::QueryDone { out: QueryOut }` or `DataRefused`.
  `DataWorker::run_query(&self, q: &EngineQuery, inputs: &[(String, Arc<Data>)]) -> DataResult<QueryOut>`
  (adopts the result base into the store like `compact`). `DataLink` gains a second worker,
  `query()`, built with `query_timeout_ms` / `query_memory_mib`, so a query never waits on a
  compaction or a render.
- Runtime errors: `QueryRun { reason }` (polars message cut to 200 chars, never cell values),
  `QueryTooLarge { what }`, `QueryColumnType { column }`, `QueryDuplicateColumn { column }`,
  `QueryTimeout` (host side, from the link timeout).

- [ ] **Step 1: Failing tests:** `run_query_joins_two_inputs_overlay_over_base`;
  `run_query_hidden_rows_and_columns_are_not_seen`; rstest `result_dtype_maps_to_column_type`;
  `datetime_result_column_is_refused_and_named`; `result_over_max_rows_is_refused`;
  `duplicate_output_names_are_refused`; worker `run_query_request_writes_base_in_store`;
  `slow_query_times_out_without_blocking_render`; oracle (feature `oracle`):
  `oracle_whitelist_queries_match_duckdb` (every Task 1 construct, same frames, same rows).
- [ ] **Step 2:** `cargo nextest run -p oku-slides-data --features engine -p oku-slides-render-worker -p oku-slides-session` → FAIL.
- [ ] **Step 3:** Implement; record binary size and cold build in `docs/development.md`.
- [ ] **Step 4:** `just test`, `just test-oracle` → PASS; `cargo deny check`, `cargo machete` →
  clean. Stage; commit `feat(data): run queries in the sandboxed worker (t-N)`.

### Task 6: Session results, recompute and rendering

**Files:** Create `session/src/derived.rs` (+ tests); Modify `session/src/lib.rs`,
`workspace_session.rs` (schedule after each request, like compactions), `data_panel.rs`
(bind check), `render/src/views.rs` (`dataset` reads results), `render-worker` (`PutResult`),
`session.ftl`, `core.ftl`.

**Interfaces:**
- Consumes: Task 4 `query_order`, Task 5 `DataWorker::run_query`.
- Produces:
  ```rust
  pub enum QueryState { Pending, Ready { key: ResultKey, out: Arc<QueryOut> }, Failed { key: ResultKey, error: DataError } }
  pub struct ResultKey(pub [u8; 32]);   // sha256(engine version, sql, input keys in name order)
  impl Session { pub fn query_state(&self, id: &DataId) -> Option<&QueryState>; }
  ```
  Input key: a manual data hashes its base id + canonical overlay JSON; a query data uses its
  `ResultKey`. Render: `views::resolve_views(deck, registry, bases, results: &Results)` where
  `Results = BTreeMap<DataId, Option<Table>>` (a ready result as a `Table` over its base, empty
  overlay; `None` = pending or failed); protocol `PutResult { slot, id }` with that `Table`
  frame, `DropData` drops it.
- Rules: after a request, dirty queries (key changed) recompute in `query_order`, one job per
  data in flight, a job whose inputs changed meanwhile is rerun and its result discarded;
  results land via the same off-lock thread pattern as compactions, then views re-render.
  Pending: placeholder, no issue. Failed: placeholder + `issue-data-query-failed` (`$data`,
  `$reason` = the error's localized key text). `bind_data` to a query data is checked against
  its ready result's columns (`session-data-query-no-result` while pending or failed).

- [ ] **Step 1: Failing tests:** `edit_input_recomputes_dependent_then_rerenders_view`;
  `chain_recomputes_in_topological_order`; `recompute_after_burst_of_edits_matches_last_inputs`;
  `failed_query_view_shows_placeholder_and_issue_fr`; `pending_query_view_has_no_issue`;
  `unchanged_inputs_do_not_recompute`; `bind_to_query_without_result_is_refused`;
  `bind_to_query_with_missing_column_is_refused`; `undo_of_input_edit_recomputes_back`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-session -p oku-slides-render -p oku-slides-render-worker` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage; commit
  `feat(session): derived data results, recompute and rendering (t-N)`.

### Task 7: Result cache in the sources zip

**Files:** Modify `core/src/{sources.rs,sources_zip.rs}`, `session` save/open paths; tests.

**Interfaces:**
- Produces: `SourceBundle.results: BTreeMap<ResultKeyHex, CachedResult>`,
  `pub struct CachedResult { pub key: String, pub columns: Vec<Column>, pub base: BaseRef }`,
  zip entries `data/<key hex>.json` (≤ 64 KiB each, ≤ `MAX_DATA` entries); result bases go in
  `bases/` with the manual ones and count toward `MAX_DECK_BASE_BYTES` (an entry that would pass
  it is not written and is recomputed on open).
- Rules: save writes only the entries of current `Ready` states; open verifies shape, bounds,
  base presence and hash, then seeds `QueryState::Ready` when the key recomputed from the
  opened deck matches; otherwise the data recomputes.

- [ ] **Step 1: Failing tests:** `save_then_open_renders_query_view_without_recompute`;
  `stale_cache_entry_is_recomputed`; `tampered_cache_entry_is_recomputed`;
  `save_writes_only_current_results`; `cache_over_base_budget_is_skipped_and_recomputed`;
  `old_sources_version_15_is_refused`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(core): cache derived results in the deck (t-N)`.

### Task 8: Paste of views over derived data

**Files:** Modify `session` paste (`paste_element`), `core` `copy_data_ops`; tests.

- [ ] **Step 1: Failing tests:** `paste_query_view_same_deck_copies_query_as_name_2`;
  `paste_query_view_into_other_deck_copies_result_as_manual_data` (per decision 4; the reply
  carries `session-data-query-pasted-as-values`); `paste_pending_query_view_into_other_deck_is_refused`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS. Stage;
  commit `feat(session): pasting a view over derived data (t-N)`.

### Task 9: MCP, agent instructions, local agent

**Files:** Modify `mcp/src/{data_tools.rs,tools.rs,handler.rs,limits.rs}`,
`agent/src/instructions/deck.md` (+ test), `local-agent/src/families.rs`,
`local-agent/eval/requests.json`, `docs/superpowers/plans/2026-11-01-lot-l-local-agent-eval.md`.

**Interfaces:**
- `data_list` entries gain `sql` and `state` (`ready` | `pending` | `failed` + error key);
  `data_read` reads a ready result; new tool `query_preview { sql, limit? }`: parse, check inputs,
  run on the query link without writing, reply like `data_read` (same caps), refusals untrusted.
- `deck.md` lists the whitelist (from `whitelist.rs`, a test checks every listed function is in it).
- `families_of("query_preview") → &[Edit]`; eval set gains at least 2 fr + 2 en data requests
  (list, read, make a summary query); the run is recorded in the eval doc (crate rule).

- [ ] **Step 1: Failing tests:** `query_preview_returns_rows_without_changing_deck`;
  `query_preview_refusal_is_wrapped_untrusted`; `data_list_shows_query_state_and_sql`;
  `instructions_whitelist_matches_code`; `families_route_query_preview`;
  `eval_set_data_requests_expect_data_tools`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `just test` → PASS; manual eval
  run recorded. Stage; commit `feat(mcp): query preview and derived data for the agent (t-N)`.

### Task 10: UI — derived data in the Data panel, catalog v6

**Files:** Modify `a2ui/src/{component.rs,data.rs,message.rs}`, `ui/src/a2ui/catalog.ts(x)`,
`ui/src/{DataList.tsx,DataSheet.tsx,dataActions.ts}`, `session/src/data_list.rs`
(`DataAction::AddQuery { name? }`, `SetQuery { data, sql }`), `ui.ftl`; e2e `ui/e2e/151-derived-data.cjs`.

- `DataSheet` gains `query: Option<QueryView { sql, state, error? }>`: a read-only grid of the
  result, the SQL in an editor applied on submit (one undo step), the error under it, links to
  its inputs. `DataList` shows kind and state.

- [ ] **Step 1: Failing tests:** session `add_query_action_creates_query_data`;
  `set_query_action_refusal_shows_on_sheet`; `catalog_id_is_equal_in_rust_and_ts` (v6);
  UI `DataSheet shows SQL, result and error for a query`, `SQL submit emits set-query`;
  e2e 151: create a query over a manual data, edit an input cell, see the chart over the query change.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement; `cargo insta review` (catalog id only).
- [ ] **Step 4:** `just test` → PASS; `just e2e <free port>` → green. Stage; commit
  `feat(ui)!: derived data in the Data panel, catalog v6 (t-N)`.

### Task 11: Gate

- [ ] **Step 1:** Update each touched `ARCHITECTURE.md` (≤ 50 lines) and `AGENTS.md` rule
  (data crate: SQL parsed in host, executed only in the worker), the spec's "Plan calls" with
  this plan's choices.
- [ ] **Step 2:** `just ci`, `just test-oracle`, `just e2e <free port>` with fresh temp
  `OKU_SLIDES_PATHS__DATA` / `__DECKS` → green; whole-branch review; land per AGENTS.md.
