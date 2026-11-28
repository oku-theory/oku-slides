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
Tag { id, name (unique ignoring case), date (UTC, `time`), frontier (Loro frontiers, encoded),
      bases (base ids of the manual data at the tag) }
```

- `tag_version { name }` records the replica's frontier **before** the op that writes the tag,
  so the tag names exactly the state the user saw. Undoable; a tag never moves; `remove_tag`
  deletes it.
- `tag_version` is alone in its batch, so the frontier is the state the user saw; `remove_tag`
  is refused while a query names the tag (`PREVIOUS` names them all).
- Tags live in a Loro map (atoms) at the root `tags`, outside `deck`, merged across copies; the history they point into is already
  kept whole (ADR-0007), so a tag stays readable after compaction.
- Reading a data at a tag replays `updates_till(frontier)` into a fresh document
  (`replica/history.rs`, never `checkout`, t-809) and reads its `data` map; the base it names
  is kept for as long as the tag exists.
- **Previous tag**: the causally latest tag (no other tag's frontier strictly includes its
  frontier; concurrent ones by latest `date`, then id). If nothing outside the `tags` root
  changed since that tag, `PREVIOUS` is the tag before it, so a report tagged a moment ago
  compares with the report before it.
- **Derived data at a tag** are recomputed at that tag (same SQL, inputs at the tag); an
  `AT` inside a query evaluated at a tag is refused (error state), so depth stays one. `AT`
  reads add no current dependency edge: a data may read its own past.
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
Phase 2 accepts only the subset fixed by the spike (see "Phase 2 spike"); the rest of this list is
deferred.

### Execution

- **polars** (features `dtype-date`, `dtype-datetime`, `parquet`, `lazy`, `sql`: the chosen
  engine; its measured cost in "Phase 2 spike" awaits the user's re-confirmation) reads the
  bases, applies the overlay, runs the queries and writes the compacted bases.
- A new crate **`oku-slides-data`**: data types, SQL validation, AST rewrites, and (behind an
  `engine` feature) the polars code. Core depends on it without the engine feature, so
  `apply_all` validates ops and queries without polars; only the render worker enables it (the
  session reaches it through the worker link), so Parquet is never parsed in the host.
- Rewrites before polars: `AT (VERSION => …)` → a registered frame read at the tag (phase 3);
  the phase 2 rewrites listed in "Phase 2 spike", all plain SQL (no direct polars call, so the
  engine request is SQL text only). Rewrites marked (s) need input schemas or expression types
  and run where those are known.
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
  A worker lost, replaced or timed out is not the query's failure: the query stays pending and
  runs again, three times at most for the same inputs, then fails with the worker's error (t-1794).
  The host cannot tell a hung worker from a slow query, so a query over the timeout fails after
  three timeouts.
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
   Phase 2 spike, done (t-1770, "Phase 2 spike" below): `polars-sql` runs the core of the
   whitelist; silent differences from DuckDB (NULL sums, integer division, NULL ordering, rounding,
   window frames) get AST rewrites or are deferred. sqlparser 0.60 `DuckDbDialect` does **not**
   parse `AT (VERSION => …)`, which phase 3 must solve first.
1. Data entity, table and chart views (manual data only).
1b. Pack-widget `list` params as views. Its own plan: list columns may be `rich` (markdown
   cells, Typst needs paragraphs back) and lists nest one level, which a scalar data cannot hold;
   a nested-list column keeps its list inline or is refused at bind (decided in that plan).
2. Derived data: crate, validation, worker, recompute, cache.
3. Tags and `AT (VERSION => …)`.
4. Linked sources (separate design): source spec, declared key column, refresh as a diff by key
   written to Loro (so tags capture it), network outside the worker.

## Phase 2 spike

Measured on 2026-11-26 (t-1770): every query ran in DuckDB 2.0.1 (the oracle) and in polars-sql
0.53 (features `lazy`, `sql`) on the same four frames, results compared row by row (floats to 10
significant digits). sqlparser 0.60 `DuckDbDialect` parses the text; polars runs its `Display`.
Spike code and raw results: `.superpowers/sdd/2026-11-26-data-entities-phase2/` (not committed).

Verdicts: **native** (same rows and names as DuckDB), **rewrite** (an AST rewrite before polars
gives the same rows; tested), **deferred** (refused in phase 2 with `data-query-construct`).
(s) marks a rewrite that needs the input schemas or expression types.

| Construct | Parses | Verdict | Rewrite or reason |
|---|---|---|---|
| SELECT, WHERE, DISTINCT, DISTINCT ON, LIMIT/OFFSET, FETCH FIRST, GROUP BY, HAVING, CASE, IN list, BETWEEN, [I]LIKE, IS [NOT] DISTINCT FROM, VALUES, `\|\|` | yes | native | |
| `SUM` (aggregate and window) | yes | rewrite | `CASE WHEN COUNT(x) = 0 THEN NULL ELSE SUM(x) END` (polars sums no value to 0) |
| `a / b` | yes | rewrite | `CAST(a AS DOUBLE) / b` (polars divides integers) |
| `a % b`, `mod(a, b)`, both operands integer | yes | rewrite (s) | `CASE WHEN a % b <> 0 AND (a < 0) <> (b < 0) THEN a % b - b ELSE a % b END` (polars floors, DuckDB truncates); integer-exact, no DOUBLE detour |
| `a % b`, `mod(a, b)`, either operand a float type | yes | deferred (s) | silent wrong results (below); refused from the input schemas |
| `a // b`, both operands integer | yes | rewrite (s) | `CASE WHEN a % b <> 0 AND (a < 0) <> (b < 0) THEN a // b + 1 ELSE a // b END`; integer-exact |
| `a // b`, either operand a float type | yes | deferred (s) | DuckDB 2.0.1 divides without flooring (`5.3 // 0.1` = 52.99999999999999), polars floors (52), and the CASE test uses polars' inexact float `%`; refused from the input schemas |
| `round(x)` (n = 0) | yes | rewrite (s) | float `x`: `CASE WHEN abs(x) - floor(abs(x)) = 0.5 THEN sign(x) * (floor(abs(x)) + 1) ELSE round(x, 0) END` (polars rounds half to even); integer `x` native |
| `round(x, n)`, n a literal > 0 | yes | native for integer `x`; deferred (s) for float `x` | integer `x` native; float `x` refused: no tested form matches DuckDB on binary edge values (below) |
| `CAST(x AS <integer type>)` written by the user | yes | rewrite (s) | float `x`: `CAST(round(x) AS …)` with polars' half-even `round` (polars truncates, DuckDB rounds half to even) |
| `ORDER BY` | yes | rewrite | explicit `NULLS LAST` on every key without one (polars puts NULLs first in DESC) |
| SELECT without FROM | yes | rewrite | add `FROM (VALUES (1)) AS __one(__x)` (polars returned 0 rows for some multi-column ones) |
| `1e3` literals | yes | rewrite | plain decimal literal (polars refuses the exponent) |
| `IN (… NULL …)`, `NOT IN (subquery)` | yes | deferred | polars is two-valued here: different rows, silently |
| IN / NOT IN list without NULL, IN (subquery) | yes | native | |
| Uncorrelated scalar subquery whose SELECT is one ungrouped aggregate, in WHERE, the SELECT list or HAVING | yes | rewrite | the k-th one becomes `CROSS JOIN (SELECT agg AS __vk …) AS __sk` (`__s1` / `__v1`, `__s2` / `__v2`, …) and is read as `__sk.__vk`; in HAVING as `MAX(__sk.__vk)` (polars refuses all three forms natively) |
| Any other scalar subquery (grouped, non-aggregate, correlated) | yes | deferred | may return zero or several rows, which a CROSS JOIN would turn into dropped or repeated rows |
| EXISTS, correlated subqueries | yes | deferred | polars refuses them |
| Derived tables, `WITH` (chained, column lists) | yes | native | a derived table needs an alias (polars) |
| `WITH RECURSIVE` | yes | refused | |
| INNER, LEFT, RIGHT, FULL, CROSS, NATURAL joins ON equalities; USING for INNER and LEFT | yes | native | |
| FULL / RIGHT JOIN USING | yes | rewrite | `ON` plus `COALESCE(l.k, r.k) AS k` (polars leaves the key NULL) |
| Comma join | yes | rewrite | `CROSS JOIN` |
| INNER join ON with a non-equality term | yes | rewrite | move the term to `WHERE` |
| Outer join on a non-equality condition | yes | deferred | polars refuses |
| SEMI / ANTI join | yes | deferred | polars refuses; native with the `semi_anti_join` feature |
| ASOF join | no (without MATCH_CONDITION) | deferred | DuckDB's `ASOF JOIN … ON a >= b` form does not parse |
| POSITIONAL join | yes | refused | polars refuses; out of scope (Language) |
| `*` over a join | yes | rewrite (s) | expand to columns, DuckDB names (`product_1`; a USING key once) — polars names it `product:p` |
| UNION [ALL], set op + ORDER BY / LIMIT | yes | native | |
| EXCEPT, INTERSECT | yes | deferred | native with polars `semi_anti_join` (not enabled); EXCEPT ALL refused even then |
| UNION [ALL] BY NAME | yes | deferred | native with polars `diagonal_concat` (not enabled) |
| ROW_NUMBER, LAG/LEAD (1–2 args), FIRST_VALUE, aggregates OVER (PARTITION BY) | yes | native | |
| Window `ORDER BY` on a nullable key | yes | rewrite | derived table adds `k IS NULL AS __nK` (DESC: `IS NOT NULL`), order by it first (polars ignores NULLS FIRST/LAST in OVER) |
| Window `ORDER BY` with mixed directions | yes | deferred | polars refuses |
| Running SUM / COUNT / AVG with `ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW` | yes | rewrite | SUM rule with `SUM(COALESCE(x, 0))`; AVG = running SUM / `NULLIF(running COUNT, 0)` |
| Aggregate OVER (ORDER BY) without that frame (RANGE) | yes | deferred | polars treats ties as ROWS: different rows, silently |
| Running MIN / MAX, LAST_VALUE, other frames, LAG with default, RANK, DENSE_RANK, NTILE, PERCENT_RANK, CUME_DIST | yes | deferred | wrong rows on NULLs, or refused; RANK / DENSE_RANK run with polars `rank` (not enabled) but need the NULL-ordering rewrite; `ROWS n PRECEDING` stays refused with `dynamic_group_by` |
| Named `WINDOW` | yes | native | SUM rule applies |
| `GROUP BY ALL`, `ORDER BY ALL`, GROUP BY position / alias | yes | native | |
| GROUP BY an expression repeated from the select list | yes | rewrite | group by its select-list position (polars fails on `EXTRACT`; the position, unlike the spike's alias, cannot be shadowed by a source column) |
| Select alias in WHERE | yes | rewrite (s) | substitute the expression unless an input column has that name |
| Select alias in GROUP BY / HAVING / ORDER BY | yes | native | |
| `* EXCLUDE`, `* REPLACE`, `COLUMNS('re')` in the select list | yes | native | |
| `* RENAME`, `COLUMNS(* …)`, `COLUMNS` inside a function or WHERE | no / yes | deferred | does not parse, or polars refuses |
| QUALIFY | yes | rewrite | derived table with the window as `__qN`, filter outside (polars needs every referenced column selected) |
| `min_by` / `max_by` | yes | deferred | rewrite via ROW_NUMBER works only as the sole aggregate |
| Aggregate `FILTER (WHERE c)` | yes | rewrite | `agg(CASE WHEN c THEN x END)`; `COUNT(*)` → `COUNT(CASE WHEN c THEN 1 END)` |
| `PIVOT (agg(x) [AS a] FOR col IN (v [AS n], …))`, one aggregate, literal list | yes | rewrite (s) | `GROUP BY` the other source columns, one `agg(CASE WHEN col = v THEN x END) AS v` (or `n_a`) per value |
| `UNPIVOT [INCLUDE NULLS] (v FOR n IN (c, …))` | yes | rewrite (s) | `UNION ALL` of one SELECT per column, `WHERE c IS NOT NULL` unless INCLUDE NULLS |
| DuckDB `PIVOT t ON … USING …`, `UNPIVOT t ON … INTO …`, `IN (ANY)`, several aggregates | no / yes | deferred | DuckDB forms do not parse in sqlparser 0.60; dynamic columns |
| `AT (VERSION => …)`, `AT (TIMESTAMP => …)` | **no** | phase 3 | "Expected: ), found: =>": phase 3 needs a parser change or a pre-parse rewrite |
| Unquoted identifiers in another case | yes | not rewritten | polars is case-sensitive: the query fails (`not found`), never wrong rows |

Rewrite notes:

- One pass: every rule matches only nodes the user wrote, and rewriter output is never rewritten
  again. So the casts the rewriter emits (`/`, SUM, `d1 - d2` and `date_diff('day', …)`'s
  `CAST(d AS INTEGER)`) escape the CAST rule, and the `round(x, 0)` the CAST rule emits stays
  polars' half-even round: `CAST(2.5 AS INTEGER)` gives 2, as in DuckDB, whereas feeding it to the
  `round` rule would give 3. The `%` and `//` forms emit no cast and
  stay exact on BIGINT above 2^53, where a DOUBLE detour (`CAST(CAST(a AS DOUBLE) / b AS
  BIGINT)`) gave wrong digits. They repeat their operands, which is safe because every whitelisted
  function is deterministic.
- `%` and `//` by zero: DuckDB 2.0.1 fails the query ("Division by zero"); polars, and so the
  rewrite, return NULL. Phase 2 accepts NULL (DuckDB's `error_on_division_by_zero=false` result).
- Float `%` is deferred: polars computes the remainder as `a - b * floor(a / b)`, DuckDB uses
  `fmod`, and the error grows with `|a / b|` (4e-16 on `5.3 % 0.1`, but 0.875 against 0.825 on
  `1000000000000000.8 % 0.9`), a silent wrong result. Float `//` is deferred too: DuckDB 2.0.1
  returns the unfloored quotient on doubles while polars floors, and the `//` form tests the
  inexact float `%`. The whitelist and Task 3 refuse `%`, `mod` and `//` when either operand has
  a float type, known from the input schemas in the worker.
- `round` on floats: DuckDB rounds the binary value half away from zero. The n = 0 form matches
  DuckDB on ties (±0.5, 2.5), on 0.49999999999999994 (0, where `floor(x + 0.5)` gives 1), on
  2^52 + 1, on NaN, ±inf (returned unchanged by both) and NULL. For n > 0, every form tried
  (`sign(x) * floor(abs(x) * 10^n + 0.5) / 10^n`, then a tie test on `x * 10^n`) differs on
  binary edge values: DuckDB gives `round(2.675, 2)` = 2.67 and `round(1.45, 1)` = 1.4, both forms
  give 2.68 and 1.5 (`1.005` and `0.285` happen to match). Float `round(x, n > 0)` is therefore
  deferred; Task 5 fuzzes the n = 0 form against the DuckDB oracle (ties, values next to ties,
  large magnitudes, NaN, ±inf) and may enable n > 0 only with a form that passes the same fuzz.

**Cost** (macOS arm64, release profile, load average 45 to 80 from other sessions, so times are
indicative only):

| Measure | Before | With `lazy` + `sql` (+ `dtype-datetime`) |
|---|---|---|
| Probe binary, stripped: reads and writes one Parquet; "after" adds one `SQLContext::execute` call | 20.7 MB | 71.1 MB (+50.4 MB) |
| Same probe, unstripped | 29.3 MB | 102.3 MB (+73.0 MB) |
| Probe cold build, no sccache: wall / CPU | 77 s / 600 s | 164 s / 1,365 s |
| `oku-slides-app` release binary, features on but no SQL call site, unstripped / stripped | 222.1 / 169.7 MB | 227.1 / 172.9 MB (+5.0 / +3.2 MB) |
| `oku-slides-app` rebuild after the feature switch (sccache on, 134 crates) | n/a: no like-for-like figure | 8 min 29 s wall, 227 s CPU |

The probe rows are a probe figure (one standalone binary with one SQL call), not the app delta.
The app itself was only measured with the features on and no caller, where the linker drops the
SQL engine: +3 to 5 MB. Task 5 re-measures the app release binary once `run_query` exists, and
that figure, not the probe's, is the app cost. There is no like-for-like app cold build figure:
the one cold release build of the app took 97 min under that load and is not usable, and the
8 min 29 s rebuild has no "before" run to compare with. The earlier "+31 MB, about 6 min" is
superseded.

Build findings: polars-io fails to compile with `lazy` + `sql` + `dtype-date` alone (the `csv`
writer needs `chrono`), so `dtype-datetime` is required; it enables polars' own `chrono`, not a
direct dependency. `lazy` pulls polars-stream, which always enables `cloud`: `object_store`
0.13.2 enters the app (reqwest 0.12.28 is already there through rust-mcp-sdk). The worker
sandbox (no network) still holds.

Not enabled, measured with them on: `semi_anti_join`, `diagonal_concat`, `rank` would make
EXCEPT, INTERSECT, SEMI / ANTI joins, UNION BY NAME and RANK native; adding them is a later call.

Refused probes `DuckDbDialect` parses (the whitelist must refuse them): `read_csv(…)`,
`'x.parquet'`, FROM-first, POSITIONAL, lists, structs, lambdas, dot calls, DESCRIBE, ATTACH,
`WITH RECURSIVE`. `USING SAMPLE` and `SUMMARIZE` do not parse.

**Functions** (`whitelist.rs`), native unless a rewrite is given:

- Text: `upper`, `lower`, `length`, `char_length`, `substr` / `substring` (also `FROM … FOR …`;
  start an integer literal, a negative one without length → `right(s, -start)`), `left`,
  `right`, `replace` (pattern a non-empty literal), `trim`, `ltrim`, `rtrim`, `concat`,
  `concat_ws`, `starts_with`, `ends_with`, `strpos`, `position`, `reverse`, `split_part`;
  rewrites `contains` → `strpos(…) > 0`, `lcase`/`ucase` → `lower`/`upper`, `prefix`/`suffix`
  → `starts_with`/`ends_with`, `regexp_matches` → polars `regexp_like`.
- Numbers: `abs`, `round` (rewrite above; float with n > 0 deferred), `floor`, `ceil`,
  `ceiling`, `sqrt`, `power`, `pow`, `exp`, `ln`, `log10`, `log2`, `sign`, `greatest`, `least`,
  `mod` (integer operands, rewrite above), `pi`, `cbrt`, `degrees`, `radians`, `sin`, `cos`;
  `log(x)` → `log10(x)`.
- Conditions and casts: `coalesce`, `nullif`, `ifnull`, `if`, `CASE`, `CAST` / `::` / `TRY_CAST`
  to DOUBLE, REAL, BIGINT, INTEGER (rewrite above), VARCHAR, TEXT, BOOLEAN, DATE.
- Dates: `EXTRACT` / `date_part` of year, month, day, quarter, week, dow, isodow, doy, isoyear;
  `strftime`, `strptime` (cast to DATE), DATE literals, `date('…')`, date comparisons.
  Rewrites: `year`/`month`/`day`/`quarter`/`week`/`dayofweek`/`dayofmonth`/`dayofyear`/
  `isodow`/`isoyear`/`datepart` → `EXTRACT`; `dayname`/`monthname` → `strftime('%A'/'%B')`;
  `date_trunc('year'|'month', d)` → `date(strftime(d, '%Y-01-01'|'%Y-%m-01'))`;
  `INTERVAL n UNIT` → `INTERVAL 'n units'` (unit plural unless n is 1, as polars
  needs); `date_add(d, i)` → `d + i`; `d + n` → `d + INTERVAL 'n days'`; `d1 - d2`
  and `date_diff('day', …)` → `CAST(d1 AS INTEGER) - CAST(d2 AS INTEGER)`;
  `date_diff('month'|'year', …)` → `EXTRACT` arithmetic; `last_day`, `make_date` via `strftime`
  / `date`. Date + interval and `date_trunc` give a DATE where DuckDB gives a TIMESTAMP at
  midnight (same value; the oracle casts DuckDB's to DATE, phase 2 has no timestamp type).
- Aggregates: `count(*)`, `count`, `count(DISTINCT)`, `sum` (rule above), `avg`, `min`, `max`,
  `median`, `stddev`, `stddev_samp`, `variance`, `var_samp`, `quantile_cont`, `quantile_disc`,
  `covar_samp`, `covar_pop`; `mean` → `avg`. `covar_pop` matched DuckDB natively, whereas
  polars-sql 0.53 refuses `stddev_pop` and `var_pop` ("unsupported function"), hence deferred.
- Deferred functions: `corr` (NaN where DuckDB gives NULL), `count_if`, `string_agg` /
  `listagg` / `group_concat`, `bool_and` / `bool_or`, `first` / `last` / `any_value` /
  `arbitrary`, `sum` / `avg (DISTINCT)`, `product`, `mode`, `approx_count_distinct`, `bit_*`,
  `geomean`, `kurtosis`, `skewness`, `percentile_cont`, `stddev_pop`, `var_pop`, `lpad`, `rpad`,
  `repeat`, `regexp_replace`, `SIMILAR TO`, `strlen`, `ascii`, `trunc`, `even`, `isnan`,
  negative `round` digits, float `%` / `mod` / `//`, DECIMAL, `date_trunc('week'|'quarter')`,
  `epoch`, `yearweek`, `current_date`.

## Testing

- Unit tests per the crate rules; regression tests in `tests/regressions`.
- **DuckDB as oracle**: `duckdb` (bundled) as a test-only dependency of `oku-slides-data`; the
  same queries on the same data must give the same results. It never ships in the app. Floats
  compare to 10 significant digits.
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

## Plan calls (phase 2 plan, 2026-11-26)

Choices the phase 2 plan made within this design:

- Queries run through polars lazy plus its SQL context in the render worker; sqlparser parses
  and whitelists in the host, which never executes. Polars and sqlparser are pinned.
- Inputs are Parquet bases (overlay folded) and a query result is stored as a base and cached in
  the deck under `data/<key>.json`; a failed query leaves the data in error state with no last
  good result.
- DuckDB is a test-only oracle behind the `oracle` feature of the data crate; `just test-oracle`
  compares polars results to it and runs in `just ci`.
- A cross-deck paste of a view over derived data copies the data as manual data.
- The element panel shows a view over a query read-only with its view controls; Kind and ShowAs
  stay disabled (visible): the session refuses those conversions over a query, because converting
  copies the query's rows.

## Plan calls (phase 3 plan, 2026-11-28)

Choices the phase 3 plan made within this design:

- `AT (VERSION => …)` is parsed by a `DuckDbDialect` wrapper with timestamp versioning on (its
  `dialect()` still reports DuckDB); only `VERSION => '<tag>'` and `VERSION => PREVIOUS` pass
  the whitelist, and each versioned read becomes a synthetic frame the session fills.
- Tags pin the bases of the manual data at the tag; the deck base budget counts current and
  pinned bases together, and compaction past it is skipped and reported.
- Results at a tag are ordinary result cache entries; the worker protocol is unchanged.
- MCP gains `tags_list` and `data_read` at a tag; the Data panel lists tags (catalog v7) and the
  history bar marks tagged versions.

## Left to the builder

Type and module names, the exact function whitelist, cache file format details, the worker link
lifecycle, recompute debouncing, the Data panel layout (within the Papier design system).
