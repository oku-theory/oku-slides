# replica

The deck in a `LoroDoc` (CRDT): full history, merge, undo. `DeckReplica` is authoritative.

## Modules

- `mod.rs`: `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`).
- `schema`: each field's `Kind` by context (`Ctx`) and registry; `write`: `Writer`, reconciles
  the containers towards a JSON `Deck`; `tree`: elements by (parent, slot); `tree/deck`: slides.
- `read`: total read, without the registry; what cannot be read is dropped and reported (`Issue`).
- `host`: `host(chart|table)` option, naming deck data; `rich`: paragraphs as a marked `LoroText`.
- `data`: `deck.data` (`data/<id>` → `kind`; a query's `sql` atom, else `table` per generation → `columns`, `rows`, `layer` per epoch
  {`epoch`, `base`, `cells`}), keyed diff, a removed data marked, never compacted; `data_read`: bounded read;
  `data_lost`: edits lost to a replaced table or layer; `compact_data`: a compaction's silent commit (new epoch).
- `undo`: undo, groups, merge keys, top step rank, data each step touches (`touches`, `drop_data`:
  cuts both stacks, Loro drops no single item); `authors`: peers, `MergeReport`.
- `compact`: compaction on open (ADR-0007), the only `tree.delete`. `entry`: replay, single tree, bounded depth (t-809); `Ancestry`: one walk up per node (t-865).
  `history`: one `Version` per edit, read by replay (`updates_till`), never `checkout` (t-809).

## Categories (`Kind`)

- `Prose`: `LoroText`, diffed per char, per line past 64 Ki chars, whole past a time budget. `Rich`: `LoroMap { rich: LoroText }`.
- `Slides`: a single `LoroTree` (`slides`); roots = slides (meta = the slide, `@slide`),
  descendants = elements; fractional order (ADR-0006).
- `Slot`: element nodes; meta = the element without its slots, `@slot` = mergeable map
  { parent `TreeID` → slot }. `List`: `LoroMap { list: LoroMovableList }` (also as a column).
- `Host`: `LoroMap`, keys LWW, objects opaque; a `dataset` stored at format 9 is dropped on read
  (`issue-data-inline-dropped`, ADR-0015). `Atom`: whole JSON, LWW (a `widgets`, `layouts` or `themes` pin; the `theme` namespace). `Nested`: `LoroMap`.

## Rules

- **Diff of the deck JSON**: `apply` writes only changes; `KnowledgeSync::Unchanged` skips `knowledge` (raw entries the read drops stay until the bundle changes, t-1731).
- **Soft delete**: a removed node gets `@deleted/<peer>` = the Unix time it was set and hides
  with its subtree; undo removes it; a re-added id gets a fresh node; an undoable creation is
  born marked (t-776); only compaction does `tree.delete`.
- **Strict input**: `@slide` off a root, a malformed mark or `@slot`: refused. The state of a
  received snapshot is never read: its operations are replayed (shallow refused), then any tree
  outside `deck.slides` and any node, deleted ones included, beyond 1024 levels are refused (t-809),
  and `snapshot` refuses (`TooDeep`) to save such a tree (t-866).
- **Registry-free read**: the type is read from the container; sorted keys (canonical form).
- **Repair on read**: deck data per `data_read` (ids repaired, `oku_slides_data::limits` bounds,
  `issue-data-truncated`; unknown `kind` dropped, `issue-data-kind-unknown`, missing one `issue-unreadable-field`; `check_kinds` refuses a write switching a kind in place); depth by `MAX_DEPTH`, elements by `MAX_SLOT_DEPTH`. Slides, elements or items with the same id
  (t-652): exact copy dropped, other occurrence renamed `{id}-{n}`. Without the parent's
  `@slot`: a sibling's slot, else the recorded slot, else `orphans` (`issue-slot-repaired`).
  Empty node (born from an undo): ignored.
- **Guards**: t-188, containers, tree, `data` and the `widgets` / `layouts` / `themes` pins (never removed) up front.
- **Knowledge**: root map `knowledge` (`id → { path, text }`), never under `deck`; text read verbatim up to `READ_FACTOR` (4)
  × each limit and 32 unreadable issues, the rest dropped unread and counted (`issue-knowledge-entries-dropped`), then checked by `knowledge::normalize`; `canonicalize` runs `knowledge::repair`.
- **Format**: `meta.format = 10` (9: `deck.data`; 10: query data), checked on import and merge;
  any other, 8 and 9 included, is refused (`UnsupportedVersion`); change `SOURCES_VERSION` with it.
