# replica

The deck in a `LoroDoc` (CRDT): full history, merge, undo. `DeckReplica` is authoritative.

## Modules

- `mod.rs`: `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`).
- `schema`: each field's `Kind` by context (`Ctx`) and registry; `write`: `Writer`, reconciles
  the containers towards a JSON `Deck`; `tree`: elements by (parent, slot); `tree/deck`: slides.
- `read`: total read, without the registry; what cannot be read is dropped and reported (`Issue`).
- `host`: `host(chart|table)` option; `rich`: paragraphs as a marked `LoroText`; `undo`: undo,
  groups, merge keys, top step rank; `authors`: peers, `MergeReport`.
- `compact`: compaction on open (ADR-0007), the only `tree.delete`. `entry`: replay, single tree, bounded depth (t-809); `Ancestry`: one walk up per node (t-865).
  `history`: one `Version` per edit, read by replay (`updates_till`), never `checkout` (t-809).

## Categories (`Kind`)

- `Prose`: `LoroText`, diffed per char, per line past 64 Ki chars, whole past a time budget. `Rich`: `LoroMap { rich: LoroText }`.
- `Slides`: a single `LoroTree` (`slides`); roots = slides (meta = the slide, `@slide`),
  descendants = elements; fractional order (ADR-0006).
- `Slot`: element nodes; meta = the element without its slots, `@slot` = mergeable map
  { parent `TreeID` → slot }. `List`: `LoroMap { list: LoroMovableList }` (also as a column).
- `Host`: `LoroMap`; `dataset` as `{columns, rows}` (`rows` of `{id, cells}`), other keys
  LWW; any other `dataset` shape: opaque. `Atom`: whole JSON, LWW (a `widgets`, `layouts` or `themes` pin; the `theme` namespace). `Nested`: `LoroMap`.

## Rules

- **Diff of the deck JSON**: `apply` brings each container to the state of the serialized
  `Deck`; nothing is written without a change.
- **Soft delete**: a removed node gets `@deleted/<peer>` = the Unix time it was set and hides
  with its subtree; undo removes it; a re-added id gets a fresh node; an undoable creation is
  born marked (t-776); only compaction does `tree.delete`.
- **Strict input**: `@slide` off a root, a malformed mark or `@slot`: refused. The state of a
  received snapshot is never read: its operations are replayed (shallow refused), then any tree
  outside `deck.slides` and any node, deleted ones included, beyond 1024 levels are refused (t-809),
  and `snapshot` refuses (`TooDeep`) to save such a tree (t-866).
- **Registry-free read**: the type is read from the container; `{columns, rows}` is read only
  under the `dataset` of a map-valued param; sorted keys (canonical form).
- **Repair on read**: duplicate column or row (same cells) dropped; missing cell read as
  `null`; other duplicate or invalid id replaced (`r{i}`/`c{j}`, then `-{n}`); bounds 10,000
  rows, 1,000 columns, 1,000,000 cells (`issue-data-truncated`), beyond that `Opaque`; depth
  by `MAX_DEPTH`, elements by `MAX_SLOT_DEPTH`. Slides, elements or items with the same id
  (t-652): exact copy dropped, other occurrence renamed `{id}-{n}`. Without the parent's
  `@slot`: a sibling's slot, else the recorded slot, else `orphans` (`issue-slot-repaired`).
  Empty node (born from an undo): ignored.
- **Guards**: t-188, containers, tree, `widgets` / `layouts` / `themes` pins (never removed) up front.
- **Knowledge**: root map `knowledge` (`id → { path, text }`), never under `deck`; text read verbatim up to `READ_FACTOR` (4)
  × each limit and 32 unreadable issues, the rest dropped unread and counted (`issue-knowledge-entries-dropped`), then checked by `knowledge::normalize`; `canonicalize` runs `knowledge::repair`.
- **Format**: `meta.format = 8` (8: root `knowledge`), checked on import and merge; 7 is upgraded
  by a silent `oku-compact` commit; change `SOURCES_VERSION` with it.
