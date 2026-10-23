# replica

The deck in a `LoroDoc` (CRDT): full history, merge, undo. `DeckReplica` is authoritative.

## Modules

- `mod.rs`: `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`).
- `schema`: the category (`Kind`) of each field, by its context (`Ctx`) and the registry.
- `write`: `Writer`, reconciles the containers towards a JSON `Deck`.
- `tree`: elements, placed by (parent, slot); `tree/deck`: slides, pairing.
- `read`: total read, without the registry; what cannot be read is dropped and reported (`Issue`).
- `host`: `host(chart|table)` option; `rich`: paragraphs as a marked `LoroText`.
- `undo`: undo, groups, merge keys, rank of the top step (`undo_step`); `authors`: peers, `MergeReport`.
- `compact`: compaction on open (ADR-0007), the only code doing a `tree.delete`.
- `entry`: replay, single tree, bounded depth (t-809); `Ancestry`: one walk up per node (t-865).
- `history`: one `Version` per edit; a past version is read by replay (`updates_till`
  into a fresh, checked document), never by `checkout` (t-809).

## Categories (`Kind`)

- `Prose`: `LoroText`, merged per character. `Rich`: `LoroMap { rich: LoroText }`.
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
  with its subtree; undo removes the mark; a re-added id gets a fresh node; an undoable
  creation is born marked (t-776); only compaction does `tree.delete`.
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
- **Guards**: t-188, containers, tree and `widgets` / `layouts` / `themes` pin maps (never removed) created up front; t-263, one commit per list `mov`.
- **Format**: `meta.format = 7` (7: `theme` a namespace atom, `themes` pins), checked on import and merge; change `SOURCES_VERSION` with it.
