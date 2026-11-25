# Data as deck entities: shared tables, SQL transforms, tagged versions — design

Date: 2026-11-21 · Task: t-1690

## Intent

Oku Slides is used to produce **periodic status reports**: the same deck, refreshed every
period, tagged, compared with the previous period. Today every table, chart and pack-widget list
carries its own copy of its data in its params, so the same figures are duplicated, edited in
several places, and nothing can be derived or compared.

Data becomes an entity of the deck. A table, a chart, a pack widget's list are **views** that
reference a data; several views can show the same data; a data is edited on its own. A data is
either entered by hand, or **derived** from other data by a SQL query, or (phase 2) **linked**
to a refreshable remote source. Report versions are **tagged** in the Loro history, and a query
can read a data as it was at a tag, so a deck compares itself with the previous period without
being rewritten.

## Goals

1. Deck-level data, referenced by views; one edit updates every view.
2. Derived data defined by a SQL query over other data of the deck, recomputed when an input
   changes.
3. Tags on report versions; queries read a data at a tag, including "the previous tag".
4. Every table/chart dataset and pack-widget `list` value is a data (lists in phase 1b).
5. Phase 2 (outlined here, designed later): linked sources, refreshed on demand, with a key
   column so a refresh updates rows instead of replacing them.

## Non-goals

- Remote sources in phase 1 (only the model leaves room for them).
- A graphical query builder: the agent writes queries; the UI shows the SQL, its result, its error.
- Sharing data between decks; writing back to a source.
- Nested types (lists, structs), `ASOF` / `POSITIONAL` joins, sampling.

## Anti-goals (a failure even if it works)

- A refresh or recompute that silently drops manual edits: derived data is read-only.
- A tagged report that cannot be reproduced: a tag never moves and its data stays readable.
- Storing a polars plan or any engine-internal format in the deck: decks must reopen years later.
- A query reaching files, network or the process: it sees only the deck's data.

## Approaches (transform language)

| | Standard | Trade-off |
|---|---|---|
| **A. SQL, DuckDB-aligned subset (picked)** | SQL + DuckDB extensions | polars runs it (`polars-sql`, on `sqlparser`); the agent writes it fluently; temporal reads via DuckDB's `AT (VERSION => …)`. Text, edited as a whole, last-writer-wins. |
| B. Vega-Lite transforms | JSON steps | Step-by-step UI editing; native dynamic `pivot`; but we write the polars mapping and an expression evaluator, joins and version comparison are awkward, agents know it less. |
| C. Substrait | protobuf relational plan | Engine-neutral and precise; polars does not consume it (would mean DataFusion); unreadable by user and agent. |

The user picked A, aligned on DuckDB, because it is the only standard polars runs directly,
the agent writes without help, and it can say "as of that tag". Standard SQL has no dynamic
pivot, so DuckDB's `PIVOT` / `UNPIVOT` are in.

## Approaches (storage)

| | Trade-off |
|---|---|
| Pure Loro, every cell a map entry | Simplest, per-cell merge; about 700 bytes of memory per cell, so 65,536 × 1,000 needs about 40 GB: capped near 2M cells. |
| **Parquet base + Loro overlay (picked)** | 65,536 × 1,000 fits; reads are projections; per-cell merge kept by the overlay. Costs a compaction step, a file store, and polars in the app. |
| Hybrid, Loro below a size threshold, Parquet above | Same reach, but two storage paths and a conversion at the threshold. The picked design gets the same small-table cost with one path (empty base). |

The user picked the Parquet base, with polars (`lazy`, `parquet`, `sql`) rather than the
lighter `arrow` + `parquet` crates (3 MB, same read speed), because queries need polars anyway.

## Model

### Data (`Deck.data`, new)

A map `DataId → Data`, outside the slides tree. Data, row and column ids are drawn when the ops
are prepared (`prepare_ops`, before `apply`), when a paste copies data (`copy_data_ops`) or by
the host's compaction commit (epochs), so applying the same ops twice gives the same deck.

```text
Data {
  name:   SQL identifier, unique in the deck (case-insensitive), what queries reference
  label:  optional display name
  inline: true while the data was made from one view's inline input and not edited or shared
  source: Manual { columns, rows }          // entered by hand
        | Query  { sql }                    // derived, read-only
        | Linked { … }                      // phase 2
}
Column { id, name, type }   type ∈ text | number | integer | bool | date
Row    { id, cells }        cells keyed by column id; null omitted
```

Stored rows and columns always carry an id; only op input (`NewTable`) may leave ids out for
`prepare_ops` to draw.

- **Storage: a Parquet base plus a Loro overlay** (spike t-1704, user decision). A `Manual`
  data is a map `data/<DataId>` (name, label, kind) in Loro holding a `table` child container
  per generation (`Replace` makes a new one), which holds:
  - `columns`: a movable list of `{id, name, type}` maps (rename, retype, order: Loro, LWW);
  - `rows`: a movable list of row ids (order and membership: inserts, removes and moves merge in
    Loro, with no position key to invent);
  - `layer`: a child container per epoch (each compaction makes a new one), holding `base`
    (`{hash, bytes, rows, columns: [{id, type, text_bytes}]}` of an immutable Parquet file, or
    empty) and `cells`, the overlay, a map `"<row id>/<column id>" → value` (per-cell merge; ids
    never contain `/`). A stored null in the overlay means "cleared": it hides the base value.

  Concurrent replaces or compactions resolve last-writer-wins: edits made to the losing
  container (generation or layer) are lost, deterministically, and reported on every peer
  (`MergeReport.lost`, `issue-data-edits-lost`).

  A cell's value is the overlay entry if any, else the base's value for that row id and column
  id, else null. The base holds only cells: a `__row_id` column plus one column per column id,
  in Parquet's own types (text→`Utf8`, number→`Float64`, integer→`Int64`, bool→`Boolean`,
  date→`Date32`). A row removed from `rows`, or a column removed from `columns`, hides its base
  cells until the next compaction drops them. A retype is lossless or refused (checked against
  the base in the worker); the base is converted to the column's current type on read, a cell
  that does not convert reads as text and is reported (`issue-data-cell-type`); the next
  compaction writes the converted base. A small data never leaves Loro: its
  cells stay in the overlay with an empty base, so a 5 × 4 table costs what it costs today.
- **Compaction**: only the host compacts, so no two copies compact the same data concurrently.
  When a data's overlay reaches 10,000 entries and no undo or redo step touches it, the host
  folds it at the current state on a dedicated worker link (`DataLink`, longer timeout and
  memory than rendering) off the edit path: the worker writes a new Parquet (base + overlay,
  hidden rows and columns dropped, rows in `rows` order), the host checks and stores it, and in
  one silent commit creates a new `layer` (new epoch, the new base, empty cells). The job
  carries `(generation, epoch, touch counter)` and is dropped if any changed; failures back off
  and are memoized. Compaction is not an undo step, and waiting for idle keeps every undo exact.
  If an edit would push the overlay past its cap first, the host folds synchronously, drops that
  data's undo and redo steps and tells the user (`session-data-undo-dropped`); still over after
  the fold, the edit is refused. A merge that loses edits drops the local undo steps of that data.
- **Parquet files** are content-addressed (`sha256`) assets in a store on disk under the app's
  data directory, never all loaded in memory. They travel in the sources zip as
  `assets/<sha>` entries, so they are embedded in the deck PDF; open stream-hashes them into the
  store and refuses a mismatch. A Parquet asset is untrusted bytes: it is parsed only inside the
  sandboxed worker (the engine feature is enabled by the render worker only), opened by handle,
  its footer checked before decoding (rows, columns, row groups, uncompressed size) and its
  schema checked against the recorded columns (a mismatch puts the data in error state,
  `issue-data-base-invalid`, no repair). Files are written atomically (temp file, hash, fsync,
  rename) before the Loro commit references them. Each process leases the bases it uses (lease
  files refreshed while alive); a GC under a store lock deletes files no session, resume
  snapshot or live lease holds, older than 10 minutes. The store has a quota
  (`sources.max_store_mib`); over it compaction is skipped and reported. Saving writes current
  bases only. Reading a version whose base is gone, or a base id the session never received,
  is reported as `issue-data-base-missing`; tags (phase 3) will pin their bases.
- **Bounds** (refused by ops past them; the replica read truncates and reports, and refuses
  edits to a truncated data until removed). Phase 1 keeps saving and opening in memory (user
  decision): the PDF and its sources zip are built whole, so the Parquet bounds are set for a
  save to cost about 1 GB of memory at worst. Streaming save and open, for decks of several GiB,
  is a backlog task.
  - per data: 65,536 rows, 1,000 columns, 16 KiB per text cell, 200,000 overlay entries (the
    host compacts at 10,000, so the cap means compaction failed), one base Parquet of at most
    512 MiB (a real 65,536 × 1,000 table fits; the spike's pessimistic one is 319 MB);
  - per deck: 256 data, 512 MiB of Parquet, 1,048,576 row ids, 300,000 overlay entries holding
    at most 8 MiB of text, 64 MiB of deck JSON. With these, the worst-case state fits the Loro
    snapshot bound and the worker's 512 MiB frame (a test builds it; the worker caches data, so a
    one-cell edit sends a frame under 64 KiB). Settings change:
    `[sources] max_sources_mib` 256 → 1024 and `max_replica_mib` 32 → 64;
  - per request: 8 MiB per HTTP request body on the apply routes, and 100,000 cells per batch of
    ops from an external ingress (host-built batches, import and paste, are exempt).
    A larger table comes in through an import (PPTX now; CSV, XLSX and Parquet files in a later
    phase), written straight to a base in the worker, never as op JSON. An import over a data
    bound keeps the first rows and columns and reports it (`ChartTruncated` today); over the
    deck bounds each table is cut to the remaining budget, and once none remains (or at 256
    data) the element is imported without data and reported. An import never fails on bounds;
  - while parsing, list and map lengths stop at their bound.

  A view reads a projection: the columns it uses and its first rows, never the whole data.
- **Untrusted values**: messages and issues name data, columns and rows, never quote a cell.
  Labels, column names and cell text read by the agent are cleaned with the existing
  `oku_slides_i18n` hidden-character predicates (extended with the tag block), which keep the
  ZWJ and ZWNJ that Persian, Indic scripts and emoji need.
- `sql`, `name`, `label`: LWW atoms. A new `Kind` entry in `replica/schema.rs`; replica
  `meta.format` 7 → 8 and `SOURCES_VERSION` with it.
- **Column types** are declared; a cell that does not fit its column is refused by the op
  (manual) or by the engine (query). `date` is an ISO `YYYY-MM-DD` string in Loro, a polars
  `Date` in the engine and `Date32` in Parquet. (Builder call: polars dtype mapping
  text→`String`, number→`Float64`, integer→`Int64`, bool→`Boolean`, date→`Date`.)
- **Rename** (`rename_data`) rewrites every dependent query (on the parsed AST, then printed
  back) in the same op, so queries reference data by name without breaking.
- **Dependencies** form a DAG computed from the queries; a cycle is refused by the op.
- **Derived schema**: a `Query` data's columns are those of its last successful result; their
  id is the output column name. Binding a view or list to a derived data checks that schema
  in the session (not in `apply_all`, which has no engine) and is refused while the data has no
  successful result.
- **Merged states the ops would refuse** (two copies merge into a cycle, or a view or query
  referencing a data the other copy removed) are kept as merged and reported: each data on the
  cycle or reading a missing data is in error state; a view over a missing data shows the
  placeholder and an issue. No repair rewrites them. Two deliberate exceptions lose concurrent
  cell edits: removing a column removes its cells, and replacing a table (`Replace`, or an
  inline re-set replacing its data in place) sets a new base, new `rows` and an empty overlay,
  dropping the rows it replaces. Two concurrent `Replace` resolve last writer wins, and undoing
  a local `Replace` restores the previous base even over a remote one (documented, not
  prevented).
- **`set_column_type`** converts every cell when each converts without loss, else it is
  refused with the first offending cells; one undo step. Text with more than 15 significant
  digits never converts to a number (an `f64` would round it).

### Views

- `host(table)` and `host(chart)` options lose their inline `dataset` and gain `data: <DataId>`
  plus an optional `columns: [<column id>…]` (projection and order; absent: all). Chart series
  map to columns as today (one series per value column). Display limits stay view limits
  (`MAX_TABLE_ROWS` 100 with the header, `MAX_TABLE_COLUMNS` 20 for a table view): a table
  view over a larger data shows its first rows and columns and reports an issue. A chart view
  counts only the columns it uses (its `columns`, else the columns its series encode plus the
  category column); over its value cap it is not cut (a partial chart misleads): it draws the
  failure frame and reports `N values, max M`. A render resolves at most 200,000 cells, each view
  counting its own (shared data counts once per view); a view past it draws the failure frame.
- An option edit without `data` keeps the previous ref; `"data": null` is refused (no unbinding
  in phase 1).
- A pack widget `list` param becomes a binding `{ "data": <DataId> }`. Its schema's columns map
  to data columns **by name**, with compatible types (`plain`/`choice`/`asset` → text, `flag` →
  bool, `number` → number/integer, `rich` → text holding the editing markdown). A data that does
  not match is refused at bind time; adapting one is a query (`SELECT nom AS name …`).
  - `rich` cells become markdown strings: the markdown round-trip invariant keeps their text,
    but they merge per cell, no longer per character.
- `show_as` (chart ↔ table) keeps the same `data` ref: no copy.
- Copying a view (copy-paste, in or across decks; the app has no duplicate command) copies its
  data: the copy gets a new data (`ventes_2`), never a silent link to the original. Pasting is
  one stateless host request, `paste_element` (formerly `carry_assets`): the host reads the
  source tab's assets and data, prepends an `add_data` per distinct source data (fresh data id,
  row and column ids, base and overlay kept so projections stay valid and no Parquet is
  rewritten; the target leases the base), rewrites the refs and applies the whole
  batch to the target tab as one undo step. Nothing is staged in a session. Two views of one
  data in one paste share one copy; a closed source or a removed data refuses the paste
  (`op-data-missing`). Cut-paste in one deck is a move and keeps the ref. Sharing is explicit
  (`bind_data`, the panel picker, an op naming an existing data id).
- Editing a view's data from its panel edits the data. A view over a derived data shows the grid
  read-only, with the query and a way to its inputs.
- Rendering is unchanged on the Typst side. The deck goes to the render worker (protocol
  6 → 8) with each data's columns, rows and overlay but not its base, which the worker reads
  from the store (read-only in its sandbox profile) with a polars Parquet reader on an open handle
  (never a path) that keeps only the projected columns. The renderer resolves the views itself (`views::resolve_views`, called by
  `Renderer::set_deck`), so every render path (session, previews, import, CLI, bench) resolves
  the same way inside the sandbox: a clone with `data` emptied and each view's bounded dataset
  written into its param, any stored `dataset` ignored. Typst reads `dataset` as today (render
  guard kept: invalid or failed → `null`, placeholder, `warn`). The session's validate reports
  the same view issues from the same planning step without converting. The chart fingerprint
  covers the dataset, so a cell edit redraws.

### Tags (`Deck.tags`, new)

```text
Tag { id, name (unique), date (UTC, `time`), frontier (Loro frontiers, encoded) }
```

- `tag_version { name }` records the replica's frontier **before** the op that writes the tag,
  so the tag names exactly the state the user saw. Undoable; a tag never moves; `remove_tag`
  deletes it.
- Tags live in a Loro map (atoms), merged across copies; the history they point into is already
  kept whole (ADR-0007), so a tag stays readable after compaction.
- Reading a data at a tag replays `updates_till(frontier)` into a fresh document
  (`replica/history.rs`, never `checkout`, t-809) and reads its `data` map; the base it names
  is kept for as long as the tag exists.
- **Previous tag**: the tag with the latest `date` among those not after the current state.
- **Names across tags**: a query names data by their **current** name; the session resolves it
  to the `DataId` and reads that id at the tag, so a later rename never breaks a comparison. A
  data that did not exist at the tag puts the query in error state ("`risques` did not exist at
  tag `Rapport 2026-10`"); no empty frame is substituted.

## Queries

### Language

A whitelisted DuckDB subset, parsed with `sqlparser` (DuckDB dialect). One `SELECT` (with
`WITH`, set operations, subqueries); tables are the deck's data names only. Accepted beyond
standard SQL:

- `PIVOT t ON col USING agg(x) GROUP BY …` and `UNPIVOT t ON … INTO NAME n VALUE v`
  (dynamic columns);
- `GROUP BY ALL`, `ORDER BY ALL`; `* EXCLUDE / REPLACE / RENAME`; `COLUMNS('regex')`;
- `QUALIFY`; `UNION [ALL] BY NAME`; `min_by` / `max_by`; aggregate `FILTER (WHERE …)`;
  select-list aliases reused in `WHERE` / `GROUP BY` / `HAVING`;
- `FROM t AT (VERSION => 'tag name')` and `AT (VERSION => PREVIOUS)`.

Refused: any table function or file path (`read_csv`, `'x.parquet'`), `ATTACH`, DDL/DML,
`ASOF` / `POSITIONAL` joins, `SAMPLE`, `SUMMARIZE` / `DESCRIBE`, FROM-first statements, lists,
structs, lambdas, dot-call chaining. Anything `sqlparser` reads but the whitelist does not name
is refused with a Fluent message naming the construct. The function whitelist is explicit.

### Execution

- **polars** (features `lazy`, `parquet`, `sql`; user decision after the spike: +31 MB of
  binary and about 6 min of cold build, accepted) reads the bases, applies the overlay, runs the
  queries and writes the compacted bases.
- A new crate **`oku-slides-data`**: data types, SQL validation, AST rewrites, and (behind an
  `engine` feature) the polars code. Core depends on it without the engine feature, so
  `apply_all` validates ops and queries without polars; only the render worker enables it (the
  session reaches it through the worker link), so Parquet is never parsed in the host.
- Rewrites before polars: `AT (VERSION => …)` → a registered frame read at the tag; whatever
  of the list `polars-sql` does not run natively (`PIVOT`, `UNPIVOT`, `QUALIFY`, `COLUMNS`… to be
  measured by the spike) → AST rewrite or a direct polars call (`DataFrame::pivot`,
  `unpivot`).
- Queries run in the **bounded, sandboxed worker** (same mechanism as the render worker: time,
  memory, no file, no network), as a new request kind on a **separate worker link** so a slow
  query never blocks slide rendering. It is the same app binary (`exit_if_worker` already
  dispatches it), so no new binary and no new `main` invariant; polars is in the app binary.
- The **session** gathers every input before dispatch: current data, and data read at tags
  (replay, `PREVIOUS` resolved to a frontier). The request carries the SQL and those frames,
  each within the stored-data bounds; the worker reads nothing else.
- **Recompute**: an edit of a data recomputes its dependents in topological order, off the
  edit path (`spawn_blocking` / worker), then re-renders their views. A query that fails puts the
  data in an error state: its views show the placeholder and an issue names the error. No last
  good result is shown in its place (no silent fallback).
- **Result cache**: computed results are not Loro state (they would bloat the history). They go
  in the sources zip as add-only, bounded, content-addressed entries `data/<hex>.json`, keyed by
  hash(sql, input result hashes, tag frontiers read, engine version). Opening a deck renders from
  the cache without recomputing; a cache entry is untrusted bytes, verified for shape and bounds on
  read, and recomputed whenever its key changes. Saving rewrites the cache with only the
  entries the current state needs, so it never accumulates; it counts toward `SourceLimits`
  under its own cap, and an entry over the cap is not written (recomputed on open).
- Results obey the same bounds as stored data.

## Operations, agent, UI

- **Ops** (core `Op`, all undoable, one step each): `add_data`, `remove_data` (refused while a
  view or query uses it; the error lists them), `rename_data`, `set_query`, `edit_data` (an
  id-based table edit, replacing today's positional `DataOp` and `set_data`), `set_column_type`,
  `bind_data` (view or list param → data), `tag_version`, `remove_tag`. No confirmation step:
  `remove_data` and `Replace` apply directly and undo restores.
- **MCP tools**: list data (name, kind, schema, row count, users), read a data (first N rows, or
  at a tag), preview a query without writing it, the ops above. The agent's instructions carry
  the SQL whitelist.
- **UI**: a Data panel (data list with kind, size and users; manual data in the existing grid;
  derived data as SQL + result + error; tags list with "tag this version"). The element panel
  shows which data a view uses, with a link to it. All strings through Fluent.

## Old decks and inline data

- **No migration** (user decision, per `crates/oku-slides-core/AGENTS.md`: refuse old deck,
  sources and replica formats). Format 7 decks are refused on open, like every earlier format
  bump; tags exist only from format 8, so every tag is readable.
- **Inline data as creation input**: an element added or replaced with an inline `dataset` in a
  `host(table|chart)` option (agent, import, pack examples) is accepted; `prepare_ops` strips it
  and appends an internal `PutViewData`, which at apply replaces the view's data in place when
  that data is still `inline` and has no other user (column ids kept by name), else adds a new
  data and binds it. `Op::apply` refuses inline data, which is never stored.
- **Default names** (for data created from an element, a param or an import): the element's
  name or the param name, lowercased, accents stripped, non-`[a-z0-9_]` runs replaced by `_`, a
  leading digit prefixed by `_`; empty → `data`; duplicates get `_2`, `_3`…; the original text
  becomes `label`. Two views never share a data created this way: sharing is a user choice.

## Phases

0. **Spike** (throwaway). Storage, done (t-1704): at 65,536 × 1,000 a Parquet base is 319 MB
   (pessimistic data), a 3-column projection reads in 2.5 ms in 40 MB, and with a 10,000-edit
   overlay in 11.5 ms; pure Loro would need about 40 GB of memory. Concurrent compaction by any
   copy converges only with a peer acknowledgement before cleanup, hence host-only compaction.
   Still to measure in phase 2: `polars-sql` coverage of the whitelist and `sqlparser` support
   for `AT (VERSION => …)` in the DuckDB dialect.
1. Data entity, table and chart views (manual data only).
1b. Pack-widget `list` params as views. Its own plan: list columns may be `rich` (markdown
   cells, Typst needs paragraphs back) and lists nest one level, which a scalar data cannot hold;
   a nested-list column keeps its list inline or is refused at bind (decided in that plan).
2. Derived data: crate, validation, worker, recompute, cache.
3. Tags and `AT (VERSION => …)`.
4. Linked sources (separate design): source spec, declared key column, refresh as a diff by key
   written to Loro (so tags capture it), network outside the worker.

## Testing

- Unit tests per the crate rules; regression tests in `tests/regressions`.
- **DuckDB as oracle**: `duckdb` (bundled) as a test-only dependency of `oku-slides-data`; the
  same queries on the same data must give the same results. It never ships in the app.
- Localized errors tested on keys plus a non-English locale.

## Plan calls (phase 1 plan, 2026-11-21)

Choices the phase 1 plan made within this design:

- Default names are cut to 60 chars so `_NN` suffixes stay within the 64-char name limit.
- `DECK_VERSION` 9 → 10 with the replica format and `SOURCES_VERSION`.
- MCP reads are capped: 200 rows, 10,000 cells, 256 KiB, 200 chars per cell.
- A2UI catalog `catalog:v5` (element panels show the shared data, `DataList`).
- `PutViewData` is an internal op: only `prepare_ops` produces it; any in its input is refused.
- The renderer resolves views (above); paste is the stateless `paste_element` (above).
- Whole-table op input (`add_data`, `Replace`, inline datasets) gives row cells by position.
- Binding a chart to another data (`bind_data`, panel picker) requires the columns the chart
  uses to exist there with the same ids; the picker lists only those data, `bind_data` refuses
  the rest (`op-data-column-missing`).
- Pack widget examples carry their own data: a pack's `widgets.json` gains a `data` map (at
  most 16 data, overlay-only) and its examples reference it, so previews and
  `commit_layout_examples` resolve like a deck. Packs have no format number (versions are
  content hashes): the base pack gets a new version with a new byte-copy fixture of the current
  base as its parent; the existing fixture of the older shipped base stays untouched, and an
  older pack's examples holding inline data are dropped with a diagnostic, like any malformed
  example.
- A data's table lives in a Loro child container per generation: `Replace` creates a new one, so
  concurrent replaces resolve last-writer-wins and edits to the losing generation are lost.
- Compaction folds at the current state into a new `layer` container (epoch) and runs only when
  no undo or redo step touches the data, so every undo stays exact; its commit is not an undo
  step. Overflow before an idle moment folds synchronously and drops that data's undo steps
  with a notice (user decision); a Loro probe decides whether the steps can be dropped
  selectively or the stack is cut. Concurrent edits to an old layer are lost and reported on
  both peers (user decision), as for a `Replace` generation.
- The base store (`<data dir>/bases/`) is shared by open decks: atomic adopt, per-process lease
  files, GC under a lock with a 10-minute grace, roots from open sessions and resume snapshots,
  a quota. Bases enter by compaction, by open (hash checked) and by paste.
- Phase 1 parses Parquet only in the render worker (polars `lazy`, `parquet`, `sql`, pinned at
  0.53; chrono arrives transitively through polars-arrow, never imported); bases are written
  only by compaction.
- Paste copies a data with its base and overlay (`NewTable` carries both from the host only).
- Import truncates to the per-data and remaining deck bounds and reports; it never fails on
  bounds (user decision).
- Deck JSON parses into plain `Data` fields with bounded visitors; a data truncated on read is
  marked and refuses edits other than remove and replace.
- `prepare_ops` (core) draws every id and turns inline data into an internal sealed op;
  external batches are capped at 100,000 cells, host batches are not.
- The body cap (8 MiB) applies to the apply routes only, checked by `serve.rs` after
  `guard::check`, and to the Tauri apply command; no new HTTP middleware.
- The worker reads a base through a polars reader on an open handle with a column projection,
  not a lazy scan (a scan takes paths).
- `Data` is flat: a `kind` plus a required `table` in phase 1; phase 2 makes `table` optional
  for `Query`.
- A growing op is refused at 90 % of the replica bound (`op-replica-full`).
- A table view reads the first 99 data rows and 20 columns; each view's resolved cells and
  bytes are budgeted; text is cut to 1,000 chars.
- The agent's destructive data ops apply without confirmation (D1); one agent turn is one undo
  group.

## Left to the builder

Type and module names, the exact function whitelist, cache file format details, the worker link
lifecycle, recompute debouncing, the Data panel layout (within the Papier design system).
