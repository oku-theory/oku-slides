# Data entities, phase 3 (tags and `AT (VERSION => …)`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A user tags the deck's state ("Rapport 2026-10"), and a query reads any data as it was
at a tag, `FROM ventes AT (VERSION => 'Rapport 2026-10')` or `AT (VERSION => PREVIOUS)`, so a
report can show what changed since the last one.

**Architecture:** Tags live in a new Loro root `tags` (outside `deck`, like `knowledge`). Each
holds the encoded frontier before the tagging op and the base ids it pins. The SQL front end
parses `AT (VERSION => …)` through a DuckDB dialect wrapper with timestamp versioning on, and
rewrites each versioned read to a synthetic frame name. The session replays the replica up to
the tag's frontier (never `checkout`), resolves each name to the data id it has now, recomputes
a derived input at that tag when needed, and hands the frames to the existing `run_query` as
extra inputs. Result keys include the frontiers read.

**Tech Stack:** Rust (sqlparser `=0.60.0` with a wrapped `DuckDbDialect`, polars `=0.53.0` in
the render worker, Loro 1.17.1), snafu, serde, Fluent; React/TS UI (A2UI catalog v7).

**Spec:** `docs/superpowers/specs/2026-11-21-data-entities-design.md` ("Tags", "Queries", phase 3
of "Phases"). Phase 2 plan: `2026-11-26-data-entities-phase2.md`. Out of scope: phase 1b
(pack-widget lists) and phase 4 (linked sources).

**Oku:** parent t-1797; Task N is t-(1797 + N): Task 1 t-1798 … Task 9 t-1806.

## Decisions (accepted by the user, 2026-11-28)

All seven proposals were accepted as written. Task 1 still runs, is measured, and does not stop
the plan; if its probes refute 1 or 4, it records the fallback in the spec and the later tasks
follow that fallback.

1. **Parsing: a dialect wrapper, no lexical pre-pass, no fork.**
   - `OkuDialect` wraps `DuckDbDialect`, forwards every method, and turns
     `supports_timestamp_versioning` on.
   - `dialect()` returns `TypeId::of::<DuckDbDialect>()`, so the parser's `dialect_of!` checks
     still see DuckDB. sqlparser's own `parse_with_wrapped_dialect` test uses this pattern.
   - `Parser::maybe_parse_table_version` then reads `AT(VERSION => …)` as
     `TableVersion::Function`.
   - The whitelist accepts only `AT(VERSION => '<tag>')` and `AT(VERSION => PREVIOUS)`.
   - Fallback if Task 1 refutes this: a lexical pre-pass, recorded in the spec.
2. **Storage.**
   - Tags live in a Loro root `tags`: a map of `TagId` → atom `{name, date, frontier, bases}`.
   - `prepare_ops` stamps the frontier and the date from the replica and the host clock. A stamp
     in external input is refused, like `PutViewData`.
   - There is no rename. `remove_tag` deletes the key and undo restores it.
   - Two tags with the same name after a merge get `issue-tag-duplicate`, and `AT` on that name
     puts the query in error. An unknown frontier gets `issue-tag-unreadable`.
3. **Base pinning.**
   - Each tag lists the base ids of the deck's manual data when it is made.
   - Known bases, save, leases and GC roots cover current ∪ pinned bases, and so does the
     512 MiB budget. Past the budget, compaction is skipped and reported on the existing path.
   - A missing pinned base gives `issue-data-base-missing` when that tag is read.
4. **`PREVIOUS`.**
   - "Latest" is causal: a tag whose frontier no other tag's frontier strictly includes. Among
     concurrent tags, the latest date wins, then the greater id.
   - If nothing outside the `tags` root changed since that latest tag T0, `PREVIOUS` is the
     latest tag before T0. Otherwise it is T0.
   - Task 1 picks the Loro 1.17.1 API that detects "only `tags` changed" without `checkout`.
5. **Derived data at a tag** are recomputed at that tag: same SQL, inputs at the tag, the whole
   needed sub-DAG at that tag.
   - An `AT` inside a query evaluated at a tag is refused ("nested version read", error state),
     so depth stays at one.
   - `AT` reads add no current dependency edge: no cycle, and a data may read its own past.
6. **Cache.**
   - Results at a tag are ordinary result entries in the sources zip, keyed with the frontiers
     read.
   - `PROTOCOL_VERSION` stays at 10: a frame at a tag is one more `(name, Data)` input of
     `run_query`.
7. **Agent, MCP, UI.**
   - New MCP tool `tags_list`; `data_read` gains `tag`. The `tag_version` and `remove_tag` ops
     go through the existing ops tool.
   - `deck.md` documents `AT` and `PREVIOUS`.
   - New A2UI `TagList` component in the Data panel (catalog v7), and a tag marker in the
     history bar. New e2e spec `152-tags.cjs`.

## Global Constraints

- sqlparser `=0.60.0` and polars `=0.53.0`, unchanged; no new dependency.
- Loro stays 1.17.1. Reading at a tag replays `ExportMode::updates_till` into a fresh document
  (`replica/history.rs`), never `checkout` (t-809).
- Parquet and query execution only in the render worker. The host parses SQL, replays Loro and
  sends frames.
- Tag limits:
  - `MAX_TAGS = 256` per deck;
  - a name is trimmed, 1 to 100 characters, with no control characters, and unique ignoring
    case;
  - an encoded frontier is at most `MAX_TAG_FRONTIER_BYTES = 4096`;
  - `bases` holds at most `MAX_DATA`.
  - Every limit is checked on read of a peer's replica, and on apply.
- Tag names in SQL match ignoring case, and only as a single-quoted string or the bare keyword
  `PREVIOUS`. Refused versions:
  - `AT(TIMESTAMP => …)`, `BEFORE(…)`, `FOR SYSTEM_TIME AS OF`, a number;
  - `AT` on a `WITH` name or on a subquery;
  - a second argument.
  Each refusal names the construct (`data-query-construct`).
- `tag_version` is alone in its batch (`op-tag-not-alone`), so "the frontier before the op" is
  the state the user saw.
- `remove_tag` is refused while a query names the tag (`op-tag-in-use`, listing the data);
  `PREVIOUS` counts as naming every tag.
- `set_query` is refused for an unknown tag name (`op-data-query-unknown-tag`), and for
  `PREVIOUS` when the deck has no tag (`op-data-query-no-previous`). After a merge or an undo,
  the same faults put the query in error state with an issue.
- A data missing at the tag puts the query in error (`data-query-missing-at-tag`, `$data`,
  `$tag`); no empty frame is substituted (no silent fallback).
- Versions:
  - `DECK_VERSION` 11 → 12, `REPLICA_FORMAT` 10 → 11, `SOURCES_VERSION` 16 → 17;
  - `WHITELIST_VERSION` 1 → 2, so the engine version changes and old cache entries are
    recomputed;
  - `CATALOG_ID` `…:catalog:v7`, Rust and TS in the same commit.
  - Old decks are refused, with no migration.
- Fluent keys:
  - `data-query-*` in `data.ftl`; `op-tag-*` and `op-data-query-*` in core; `issue-tag-*`;
    `session-tag-*`;
  - all 20 locales, in the same order.
  - Errors are tested on the key plus one non-English locale.
- Every commit cites its task: t-1798 to t-1806, children of t-1797.

## Review Focus

1. **A tag from a peer whose frontier this copy does not have** (forged or truncated replica).
   The tag shows as unreadable with one issue; queries reading it are in error; nothing panics;
   other tags still read. → Task 2, `read_tag_with_unknown_frontier_reports_unreadable`.
2. **Rename between the tag and now.** `ventes` renamed to `sales` after the tag, so the query
   says `sales AT (VERSION => 'T')` and reads the data id that was named `ventes` at T. A data
   added after T is in error, never an empty frame. → Task 5,
   `renamed_data_reads_its_old_rows_at_tag` and `data_added_after_tag_is_in_error`.
3. **Tag right after tagging.** `PREVIOUS` immediately after `tag_version`, with no edit in
   between, must not name the tag just made. → Task 5, `previous_skips_tag_with_no_edit_since`.
4. **Compaction after a tag.** Compaction must not delete the base the tag pins: a save, close,
   GC and reopen still read the tag. → Task 6,
   `tag_survives_compaction_save_gc_and_reopen`.
5. **Undo of `tag_version` while a query uses `PREVIOUS`.** The query falls back to the earlier
   tag, or to an error if there is none, never to stale rows. Its key changes and it recomputes.
   → Task 5, `undo_tag_recomputes_previous_query`.

---

### Task 1: Spike — `AT (VERSION => …)` parsing and Loro tag probes

**Files:**
- Throwaway branch for the parser probe.
- Keep: `crates/oku-slides-core/tests/loro_tags_probe.rs`, a Loro probe test like
  `loro_tree_probe.rs`.
- Output: a new "Phase 3 spike" section in the spec, and an update of the spec's "Phases"
  item 0 sentence "sqlparser 0.60 … does **not** parse".

- [ ] **Step 1: Wrapped-dialect probe.** Parse with `OkuDialect` (see Task 3 Interfaces) and
  record, for each query, whether it parses, the `TableVersion` value and the `Display` output:
  - `SELECT * FROM t AT (VERSION => 'x')`
  - `… AT (VERSION => PREVIOUS)`
  - `… AT (VERSION => 'x') AS a`
  - `… AS a AT (VERSION => 'x')`
  - `SELECT * FROM t at` and `SELECT * FROM t AS at`
  - `SELECT at FROM t`
  - a join with `AT` on both sides
  - `AT` inside a subquery and a CTE body
- [ ] **Step 2: Phase 2 corpus probe.** Parse every query of the phase 2 corpus (data crate
  `sql` tests, oracle cases and fixtures) with both `DuckDbDialect` and `OkuDialect`. The ASTs
  must be equal, apart from tables named or aliased `at`; list any difference.
- [ ] **Step 3: Loro probe (kept).** In `loro_tags_probe.rs`, with a doc holding roots `deck`
  and `tags` and 10,000 commits:
  1. `tags_only_change_since_frontier_is_detected`: compare `doc.diff(&f, &doc.state_frontiers())`
     with walking the changes after `f` (`export(ExportMode::updates(&vv))` + `ChangeMeta`
     container roots). Pick the one that is correct after a merge and an undo, and record its
     time.
  2. `frontier_inclusion_orders_tags`: decide `a ⊂ b` with `VersionVector` from
     `frontiers_to_vv`.
  3. `updates_till_tag_frontier_reads_deck`: replay to a tag frontier, then `compact_data`, then
     replay again; the result is the same.
- [ ] **Step 4: Record.** Write the spec's "Phase 3 spike" table: construct → parses / refused /
  needs fallback, the chosen diff API and its time. If Step 1 or 2 shows a construct that cannot
  be told apart (for example a table aliased `at` that no longer parses), record the decision:
  refuse the `at` alias with `data-query-construct`, or fall back to the lexical pre-pass.
- [ ] **Step 5: Commit** the spec update and the probe test, as
  `docs(data): phase 3 spike findings (t-1798)` with `Review-waived: docs-only` (the probe is a
  test-only file, also allowed under the waiver).

### Task 2: Tags in the model and the replica

**Files:**
- Create: `crates/oku-slides-core/src/tag.rs` and `tag_tests.rs`.
- Create: `crates/oku-slides-core/src/replica/tags.rs` and `tags_tests.rs`.
- Modify:
  - `core/src/model.rs:31` (`DECK_VERSION` 12) and `Deck`;
  - `core/src/replica/mod.rs:92` (`REPLICA_FORMAT` 11, and its format comment), `schema.rs`,
    `read.rs`, `write.rs`;
  - `core/src/sources.rs:56` (`SOURCES_VERSION` 17);
  - fixtures (`just oku-slides save` on each committed deck fixture);
  - `core/ARCHITECTURE.md` and `replica/ARCHITECTURE.md`.

**Interfaces:**
- Produces, in `oku_slides_core::tag`:
  ```rust
  pub struct TagId(String);                // same id form as DataId
  pub struct TagFrontier(Vec<u8>);         // Frontiers::encode(); hex in JSON; ≤ MAX_TAG_FRONTIER_BYTES
  pub struct Tag { pub name: String, pub date: OffsetDateTime /* UTC, RFC 3339 in JSON */,
                   pub frontier: TagFrontier, pub bases: BTreeSet<BaseId> }
  pub const MAX_TAGS: usize = 256;
  pub const MAX_TAG_FRONTIER_BYTES: usize = 4096;
  pub fn check_tag_name(name: &str) -> Result<String, OpError>; // trimmed name or op-tag-name
  ```
- `Deck.tags: BTreeMap<TagId, Tag>` (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`,
  bounded like `data`).
- `replica::tags::{read, write}`: root `TAGS = "tags"`, a `LoroMap` of atoms. Reading is bounded,
  and a malformed atom is dropped with `issue-tag-malformed`.
- `Replica::tag_frontier(&self) -> TagFrontier`: the current `state_frontiers`, encoded.
- `Replica::has_frontier(&self, f: &TagFrontier) -> bool`: decodes it and checks that it is
  included in `oplog_vv`.
- The read of a deck reports `issue-tag-unreadable` (`$tag`) for each tag where
  `has_frontier` is false, and `issue-tag-duplicate` (`$tag`) once per name shared ignoring case.

- [ ] **Step 1: Failing tests.**
  - `tag_tests.rs`, rstest `check_tag_name`: `"  Rapport  "` → `"Rapport"`; `""`, 101 chars and
    `"a\u{7}"` → `op-tag-name`.
  - `tags_tests.rs`:
    - `tags_round_trip_through_replica`;
    - `read_tag_with_unknown_frontier_reports_unreadable` (Review Focus 1);
    - `read_bounds_tags_at_max_tags` (257 atoms → 256 read plus one issue);
    - `read_drops_frontier_over_limit_with_issue`;
    - `merged_duplicate_names_report_once`;
    - `tags_root_absent_reads_empty`.
  - `sources_tests`: `sources_version_16_refused`.
- [ ] **Step 2:** `just test` → the new tests FAIL.
- [ ] **Step 3:** Implement the interfaces. The `tags` root is written only when `deck.tags`
  differs from the replica's, like `KnowledgeSync`.
- [ ] **Step 4:** `just test` → PASS. Update both ARCHITECTURE.md files (≤ 50 lines).
- [ ] **Step 5:** Stage, then commit `feat(core)!: tags in the deck model and a tags replica root (t-1799)`.

### Task 3: SQL front end accepts `AT (VERSION => …)`

**Files:**
- Create: `crates/oku-slides-data/src/sql/dialect.rs` and `dialect_tests.rs`.
- Modify:
  - `sql/mod.rs` (`parse_query`, `Query`, `EngineQuery`, `WHITELIST_VERSION` 2);
  - `sql/whitelist.rs`, near the `version.is_some()` refusal (~l. 486);
  - `sql/diagnose.rs`, `sql/rewrite.rs`;
  - `data.ftl` and all locales;
  - `data/ARCHITECTURE.md`.

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug)] pub(crate) struct OkuDialect(DuckDbDialect); // dialect() = TypeId::of::<DuckDbDialect>(),
                                                                // supports_timestamp_versioning() = true, rest forwarded
  #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub enum VersionRef { Tag(String) /* as written, unquoted */, Previous }
  #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub struct VersionedInput { pub name: String /* lowercased */, pub at: VersionRef }
  impl Query {
      pub fn inputs(&self) -> BTreeSet<String>;                 // unchanged: current-state reads only
      pub fn versioned_inputs(&self) -> BTreeSet<VersionedInput>;
      pub fn renamed(&self, old: &str, new: &str) -> Query;     // keeps each table's version
  }
  pub struct EngineQuery { pub sql: String, pub frames: Vec<(String, VersionedInput)> }
  // for_engine(): each `t AT (…)` becomes the frame `__at_<i>` (i = index in versioned_inputs order),
  // keeping the alias, or adding `AS t` when there was none so that `t.col` still resolves.
  ```
- Fluent keys `data-query-version-arg` (only `VERSION => '<tag>'` or `PREVIOUS`) and
  `data-query-version-cte`. `diagnose.rs` stops mapping `AT (VERSION` to a refusal and keeps
  `AT (TIMESTAMP`.

- [ ] **Step 1: Failing tests.**
  - `dialect_tests.rs`: `oku_dialect_parses_phase2_corpus_like_duckdb`, over the Task 1 corpus.
  - `sql/mod_tests.rs`, rstest:
    - `versioned_inputs` over `'x'`, `PREVIOUS`, `previous` (case-insensitive) and a mixed
      join;
    - `parse_refuses_version_forms`: `AT(TIMESTAMP => …)`, `BEFORE(…)`, `FOR SYSTEM_TIME AS OF`,
      `AT(VERSION => 3)`, `AT(VERSION => 'a', VERSION => 'b')`, `AT` on a CTE name → key
      asserted;
    - `renamed_keeps_version`;
    - `for_engine_replaces_versioned_reads_with_frames` (asserting the SQL text and `frames`);
    - `for_engine_keeps_alias_on_versioned_read`.
  - Any decision Task 1 recorded about the `at` alias gets its own case.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-data` → FAIL.
- [ ] **Step 3:** Implement. `parse_query` uses `OkuDialect`; the recursion limit and size
  limits are unchanged.
- [ ] **Step 4:** `just test` and `just test-oracle` → PASS (the oracle corpus has no `AT`, so it
  shows the dialect swap changed nothing).
- [ ] **Step 5:** Stage, then commit `feat(data): parse and rewrite AT (VERSION => …) (t-1800)`.

### Task 4: Tag ops and query validation in core

**Files:**
- Create: `crates/oku-slides-core/src/ops/tag.rs` and `tag_tests.rs`.
- Modify:
  - `ops/mod.rs` (`Op` at l. 89, apply dispatch around l. 821);
  - `prepare.rs` (`PrepareCtx`, `check_external`);
  - `ops/query.rs` (`set_query` checks);
  - `query_order.rs` (no edge for an `AT` read);
  - `validate_data.rs`;
  - core `.ftl` files and all locales.
- Create: `crates/oku-slides-core/tests/regressions/issue_1801_tag_frontier_before_op.rs`,
  declared in `main.rs`.

**Interfaces:**
- Consumes: Task 2 `Tag`, `TagId` and `check_tag_name`; Task 3 `versioned_inputs`.
- Produces:
  ```rust
  Op::TagVersion { name: String, #[schemars(skip)] #[serde(default, skip_serializing_if = "Option::is_none")] stamp: Option<TagStamp> }
  Op::RemoveTag { tag: TagId }
  pub struct TagStamp { pub id: TagId, pub date: OffsetDateTime, pub frontier: TagFrontier, pub bases: BTreeSet<BaseId> }
  // PrepareCtx gains: pub frontier: Option<&'a TagFrontier>, pub now: OffsetDateTime
  // prepare_ops fills `stamp` (bases = the ctx.deck manual data base ids); apply refuses TagVersion without stamp.
  pub fn Deck::previous_tag(&self, current_changed_since: impl Fn(&TagFrontier) -> bool) -> Option<&TagId>;
  pub fn Deck::tag_named(&self, name: &str) -> Result<&TagId, TagLookup>; // TagLookup::{Unknown, Ambiguous}
  ```
- `previous_tag` takes a closure, so core decides without a replica; the session passes the
  Task 1 diff check.
- Op errors:
  - `op-tag-name`, `op-tag-duplicate`, `op-tag-too-many`, `op-tag-not-alone`;
  - `op-tag-in-use` (`$data` list), `op-tag-stamp-external`;
  - `op-data-query-unknown-tag`, `op-data-query-no-previous`.

- [ ] **Step 1: Failing tests.**
  - `tag_tests.rs`:
    - `tag_version_records_frontier_given_by_prepare`;
    - `tag_version_with_external_stamp_refused`;
    - `tag_version_with_other_op_refused`;
    - `tag_version_duplicate_name_ignoring_case_refused`;
    - `tag_version_past_max_tags_refused`;
    - `remove_tag_used_by_query_refused_listing_data`;
    - `remove_tag_used_through_previous_refused`;
    - `undo_remove_tag_restores_it`;
    - `set_query_unknown_tag_refused`;
    - `set_query_previous_without_tags_refused`;
    - `query_order_has_no_edge_for_versioned_read` (a data reading itself at a tag is not
      broken);
    - `previous_tag_picks_causally_latest_then_date_then_id`;
    - `previous_tag_skips_latest_when_nothing_changed_since`.
  - Regression `issue_1801_tag_frontier_before_op`: the tag's frontier read back equals the
    replica frontier before apply.
- [ ] **Step 2:** `just test` → FAIL.
- [ ] **Step 3:** Implement. Every apply site passes `frontier` and `now`: the compiler lists
  them, since `PrepareCtx` is built whole.
- [ ] **Step 4:** `just test` → PASS.
- [ ] **Step 5:** Stage, then commit `feat(core): tag_version and remove_tag ops (t-1801)`.

### Task 5: Session reads data at a tag; `PREVIOUS`

**Files:**
- Create: `crates/oku-slides-session/src/tag_read.rs` and `tag_read_tests.rs`.
- Modify:
  - `derived.rs` (`keyed`, `Keyed`, `jobs`, `inputs`, `land`);
  - `history.rs` (expose replay at a frontier);
  - `core/src/replica/history.rs` (`deck_at_frontier`);
  - `data_read.rs` (read at a tag);
  - session `.ftl` files;
  - `session/ARCHITECTURE.md`.

**Interfaces:**
- Consumes: Task 3 `EngineQuery.frames`; Task 4 `previous_tag` and `tag_named`; the existing
  `run_query(worker, &EngineQuery, &[(String, Arc<Data>)])`.
- Produces:
  ```rust
  // core replica/history.rs
  pub fn Replica::deck_at_frontier(&self, f: &TagFrontier) -> Result<DeckRead, ReplicaError>; // fresh doc, updates_till
  pub fn Replica::changed_outside_tags_since(&self, f: &TagFrontier) -> bool;                 // Task 1 API
  // session tag_read.rs
  pub(crate) struct TagDecks { /* frontier → Arc<Deck>, LRU of 4, cleared when tags change */ }
  impl TagDecks { pub(crate) fn deck(&mut self, replica: &Replica, f: &TagFrontier) -> Result<Arc<Deck>, DataError>; }
  pub(crate) fn resolve(deck: &Deck, input: &VersionedInput, changed: impl Fn(&TagFrontier) -> bool)
      -> Result<(TagId, DataId), DataError>; // current name → current id; tag lookup; Ambiguous/Unknown → DataError
  ```
- `Keyed.inputs` becomes `Vec<(String /*frame or name*/, Input)>`, with
  `enum Input { Current(DataId), AtTag { tag: TagId, data: DataId } }`.
- The key of an `AtTag` input digests `frontier bytes`, `data id` and the input's key at the tag:
  `manual_key` of the tag deck's table, or the key of the derived data computed at the tag.
- A derived data at a tag is a job keyed in the tag deck, stored in the same
  `Derived.states`, with the id `DataId@frontier-hex`, internal and never shown.
- A query evaluated at a tag that itself has `AT` fails with `data-query-nested-version`.
- Errors: `data-query-missing-at-tag` (`$data`, `$tag`), `data-query-tag-unknown`,
  `data-query-tag-ambiguous`, `data-query-tag-unreadable`, `data-query-no-previous`.

- [ ] **Step 1: Failing tests** in `tag_read_tests.rs` (a session with a real replica):
  - `query_at_tag_reads_rows_as_tagged`;
  - `renamed_data_reads_its_old_rows_at_tag` (Review Focus 2);
  - `data_added_after_tag_is_in_error`;
  - `previous_skips_tag_with_no_edit_since` (Review Focus 3);
  - `previous_is_latest_tag_after_an_edit`;
  - `undo_tag_recomputes_previous_query` (Review Focus 5);
  - `derived_input_is_recomputed_at_tag`;
  - `nested_version_read_at_tag_is_in_error`;
  - `ambiguous_tag_after_merge_is_in_error`;
  - `result_key_changes_with_tag_frontier`;
  - `data_read_at_tag_returns_tagged_rows`.
  - Each error case is asserted on its key and in `fr`.
- [ ] **Step 2:** `just test` → FAIL.
- [ ] **Step 3:** Implement. Replay runs in `spawn_blocking`, like the other synchronous session
  work; no mutex guard is held across an await.
- [ ] **Step 4:** `just test` → PASS.
- [ ] **Step 5:** Stage, then commit `feat(session): read data at a tag and PREVIOUS in queries (t-1802)`.

### Task 6: Tags pin their bases; result cache at tags

**Files:**
- Modify:
  - `session/src/bases.rs` (`sync_bases`, `known`);
  - `workspace_bases.rs` (`resumed_bases` adds tag bases);
  - `data_compact.rs`;
  - `core/src/sources_zip.rs` (budget, l. 121) and `core/src/sources_results.rs` (entries
    needed);
  - `derived_cache.rs`.
- Create: `crates/oku-slides-session/src/tag_bases_tests.rs`.

**Interfaces:**
- Produces `Deck::pinned_bases(&self) -> BTreeSet<BaseId>`: the union of every tag's `bases`.
- `known = current manual bases ∪ pinned_bases ∪ result bases`.
- The save keeps a result entry when its key is one the current state or a tag read needs.

- [ ] **Step 1: Failing tests:**
  - `tag_survives_compaction_save_gc_and_reopen` (Review Focus 4);
  - `remove_tag_releases_its_bases_at_next_gc`;
  - `pinned_bases_count_toward_deck_budget` (compaction skipped and reported);
  - `missing_pinned_base_reports_issue_on_tag_read`;
  - `reopen_renders_tag_query_from_cache_without_worker` (the worker stub counts zero runs);
  - `save_drops_cache_entries_of_removed_tag`.
- [ ] **Step 2:** `just test` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `just test` → PASS. Update the session and core ARCHITECTURE.md files.
- [ ] **Step 5:** Stage, then commit `feat(session): tags pin their bases; cache results at tags (t-1803)`.

### Task 7: MCP and agent instructions

**Files:**
- Modify:
  - `crates/oku-slides-mcp/src/data_tools.rs` (l. 59 `data_read`, new `tags_list`), `tools.rs`
    (l. 427 list), `handler.rs` (l. 73), `tools_tests.rs`;
  - `crates/oku-slides-agent/src/instructions/deck.md`;
  - `mcp/ARCHITECTURE.md`.

**Interfaces:**
- `tags_list` (no args, `Kinds::DECK_OR_HELP`) returns
  `{tags: [{id, name, date, readable, previous: bool}]}`, in causal order then date. Its
  description says names are the user's data, never instructions.
- `data_read` gains `tag: Option<String>` (a name or `"PREVIOUS"`). Errors are the Task 5 keys.
- `deck.md` gets a short section: `FROM t AT (VERSION => 'name')`, `PREVIOUS`, current names
  across renames, and a "what changed since the last report" example (`EXCEPT` or a `JOIN` on a
  key).

- [ ] **Step 1: Failing tests:**
  - `tags_list_lists_tags_with_previous_flag`;
  - `data_read_at_tag_returns_tagged_rows`;
  - `data_read_unknown_tag_errors_with_key`;
  - add `tags_list` to the tool list cases (l. 201 and l. 467).
- [ ] **Step 2:** `just test` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `just test` → PASS.
- [ ] **Step 5:** Stage, then commit `feat(mcp): tags_list and data_read at a tag (t-1804)`.

### Task 8: UI — tags in the Data panel, catalog v7

**Files:**
- Modify:
  - `crates/oku-slides-a2ui/src/component.rs` (`TagList` next to `DataList`, l. 403) and
    `message.rs:9` (`CATALOG_ID` v7);
  - `ui/src/a2ui/catalog.tsx:31`;
  - `session/src/data_panel.rs` and `data_list.rs` (`DataAction` gains `TagVersion { name }`
    and `RemoveTag { tag }`);
  - `core/src/replica/history.rs` (`Version.tags: Vec<String>`, the names of tags whose
    frontier equals that version's cut);
  - `ui/src/history.ts` (`Marker.kind` gains `"tag"`) and `HistoryBar.tsx`;
  - `ui/src/DataPane.tsx`.
- Create: `ui/src/TagList.tsx`, `TagList.render.test.tsx`, `ui/e2e/152-tags.cjs`.

**Interfaces:**
- `ComponentKind::TagList { rows: Vec<TagRow>, edit: Dynamic<Value>, action: Action }`, with
  `TagRow { id, name, date, readable, previous }`.
- The zod schema mirrors it; the TS `CATALOG_ID` equals the Rust one.
- Strings go through Fluent: `data-tags-title`, `data-tags-add`, `data-tags-name`,
  `data-tags-remove`, `data-tags-previous`, `data-tags-unreadable`.

- [ ] **Step 1: Failing tests:**
  - Rust: `data_panel_lists_tags`; `tag_action_applies_tag_version`;
    `version_carries_tag_names`.
  - TS: `TagList renders rows with previous badge`; `TagList add sends name`;
    `markers include tag`.
  - Read `docs/agents/ui-e2e.md` first, then write the e2e test `152-tags.cjs`: tag, edit a
    cell, a query with `PREVIOUS` shows the difference, remove the tag, refused while used.
- [ ] **Step 2:** `just test` → FAIL.
- [ ] **Step 3:** Implement, within the Papier design system. Change the Rust catalog and
  `catalog.tsx` in the same commit.
- [ ] **Step 4:** `just test` → PASS. Then run
  `OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) just e2e 4999`
  → PASS.
- [ ] **Step 5:** Stage, then commit `feat(ui): tags in the Data panel, catalog v7 (t-1805)`.

### Task 9: Gate

- [ ] **Step 1:** Update ARCHITECTURE.md for data, core, replica, session, mcp and ui (each ≤ 50
  lines), and the nearest AGENTS.md if a rule changed. Add to the spec a "Plan calls (phase 3
  plan, 2026-11-28)" section, if not yet written.
- [ ] **Step 2:** Run `just ci`, `just test-oracle`, and `just e2e 4999` with fresh temp
  `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` → all PASS.
- [ ] **Step 3:** Run `/review` over the branch and fix its findings. Commit with a
  `Reviewed-by:` trailer naming the round and the gates.
- [ ] **Step 4:** Land per AGENTS.md Workflow. Mark t-1798 to t-1806 done as each lands, then
  t-1797.
