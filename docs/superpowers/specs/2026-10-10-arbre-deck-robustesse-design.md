# Deck tree: undoing an add, compaction, depth — design

- **Date:** 2026-10-10, revised on 2026-10-11 after two reviews of the plan and the Loro probes
- **Tasks:** t-776 (high), t-770, t-809 (high, security)
- **Context:** [ADR-0006](../../adr/0006-deck-en-un-seul-arbre-suppression-douce.md)
- **Batch order:** t-776, then t-770, then t-809. The t-809 probe opens its batch;
  a stop there does not block the first two.

## Scoping decisions (user)

1. Compaction: on open, only the nodes whose mark is more than
   N days old, N = 30 (setting).
2. Loro history kept whole: no *shallow* snapshot produced by the app.
3. Empty nodes (born from an undone `mov`) are compacted by the same rule.
4. t-809, option D: the state of a received snapshot is never read; the document
   is replayed from its operations, then bounded. (Option 1, a large stack
   and a byte cap, is rejected: a Loro *shallow* snapshot already holds
   a chain at 7.5 B per level, a forged state much less.)
5. Maximum tree depth: 1024 (a deletion keeps its
   hidden subtree, which a `mov` carries along: 64 was reachable through use).
6. Clocks: `limit = min(now, most recent change of the document) − N days`.
   Intended consequence: a mark is compacted only after N days
   of document activity. A zero or negative date is never old.
7. A mark restored by ⌘Z or redo keeps its original date: a
   documented limit, no redating.

## Batch 1 — t-776: undoing an add no longer loses the subtree

**Problem.** Loro inverts `create` with `delete`: undoing an add takes away
the subtree a peer moved under it (B creates a slide, A moves an
element into it with `move_element`, B presses ⌘Z: the element is lost, A's ⌘Z
does not bring it back).

**Born marked, together.** In an undoable reconciliation (`apply`,
`apply_moving`), between marking the removals and placement, all the
nodes to create (unmatched slides and elements, owners first) are
created at the end of their parent, each with the current peer's mark, in
**a single** commit with origin `SILENT` (excluded from the `UndoManager`), preceded
by a part that commits what comes before. Placement then treats them
as existing nodes (moved to their rank) and removes their mark, in
the undoable step.

Undoing the add puts the marks back (soft delete): the nodes are
hidden, their subtrees too, nothing is lost. A's ⌘Z brings back its
element; B's redo gives back the slide and what is under it. `new` and
`apply_silent` create directly, with no mark or commit; the trial run of
`restore` follows the same path as the `apply` that follows it.

**Side effect.** No code path does a `tree.delete` any more outside
compaction. Birth commits are parts: no history step,
no count in the merge report (which no longer counts any part).

**Cost.** At most one extra part and one extra silent commit per
application, whatever the number of nodes created. A node-by-node
birth was measured: one undo item per node as soon as there is nesting, ⌘Z of
500 nested stacks 82.5 s (versus 41.8 ms before t-776). Control
measurement: pasting 50 and 500 elements, flat and nested; a ⌘Z of 500
above 100 ms stops the batch.

**Tests.** Regression `issue_0776_undo_add_keeps_moved_subtree.rs` (precondition
checked; slide node not deleted by Loro after B's ⌘Z; A's ⌘Z;
B's redo). Existing tests inverted (`soft_delete_tests`). A paste is
undone and redone as one block, nodes marked and not deleted. `apply_silent`
adds only one change; an `apply` that creates nodes adds at most two (a part and the birth commit), whatever their number.

## Batch 2 — t-770: compaction of dead nodes

**Dated mark.** The value of `@deleted/<pair>` changes from `true` to the Unix
time it was set (`i64`, the author's clock; Loro only exposes the last
author of a key, not its op). `REPLICA_FORMAT` 3 → 4, `SOURCES_VERSION`
9 → 10, with no compatibility. `check_trees` rejects a mark that is not an
integer.

**Empty node dated** by `get_last_move_id` then `get_change(id).timestamp`.

**Rule** (`compact_doc`, entirely in `replica/compact.rs`): with
`limit = min(now, most recent change timestamp) − N days`, the following is
deleted by `tree.delete`, with its already hidden subtree:
- any node that carries at least one mark dated in `]0, limit]`;
- any empty node dated in `]0, limit]`.
A node taken away by a compacted ancestor is not counted. Deterministic
deletion order.

**Where.** In opening (`from_snapshot`), under the guard, after the first
read, before `Undo::new`. Silent commit, message `oku-compact`: no
history step, no count in the merge report. The compaction commit
is dated like the most recent change (`set_next_commit_timestamp`), so
that opening a file does not count as activity. No public method compacts during a session.

**Accepted risks:** another copy open for more than N days of activity,
which then undoes its mark or the `mov` of an element under this node, loses it on
merge; an author clock running behind shortens the window; a mark
restored by ⌘Z keeps its date; a forged file can get a node compacted
that it could already hide; a forged `oku-compact` message hides a change from
the history and the report (the content stays in later versions).

**Setting.** `compact_after_days = 30` in `[sources]` (0: never).
`ReplicaLimits { max_bytes, compact_after_secs }` replaces the bytes
argument of `from_snapshot` and of `Resume::read_doc` (`From<usize>`: without
compaction, for tests and tools); `SourceLimits` gains
`compact_after_secs` and `replica()`.

**Measurement** (ADR-0007): `deck()`, `apply`, `from_snapshot`, `merge`, size,
before and after compaction, 40×20 with ~10,000 marked nodes.

**Tests.** Dated through the mark values and through
`set_next_commit_timestamp` (change dates do not go backwards): old mark
compacted, recent one kept, exactly at the limit compacted and one
second after kept; one old among recent ones; local clock ahead
bounded by the most recent change; empty node compacted after
N days of activity; age predicate on 0 and negative; dead parent and child
counted once; no commit when nothing is to be compacted; opening not
undoable, with no step; convergence with an uncompacted copy; empty merge
report; non-integer mark rejected; `from_zip` and resume.

**ADR-0007** "Compaction of dead nodes" (partially supersedes ADR-0006:
"never `tree.delete`").

## Batch 3 — t-809: very deep forged tree chain

**Problem.** Loro 1.16.2 recurses over a tree's depth: on the first
access to a snapshot's decoded state, then, even once deleted, in
`is_node_deleted`, export and `fork`. A stack overflow aborts the
process; `catch_unwind` does not catch it. A snapshot's state is not
checked by Loro against its history: it can be forged, dense, cyclic.

**Probe of 2026-10-11** (release, 256 KiB stack, 100,000 levels):
- imported snapshot: 1st access, `is_node_deleted`, export, `fork`, drop
  overflow;
- **replayed document** (import of the updates exported from a document whose state
  is never read): 1st access, walking up `parent()`, drop pass; only
  exporting a deep tree overflows;
- deep past version read back by replay (`updates_till`): passes, 2 ms;
- 50,000 nested maps replayed: import, export, `fork`, drop pass
  (`get_deep_value` overflows; never called outside tests); beyond 65,535,
  Loro panics (caught);
- *shallow* snapshot: `export(all_updates)` impossible → rejected by
  `is_shallow()`;
- replay cost: chain of 100,000 ops 60 ms (direct 4 ms); 1 M ops 0.8 s
  (5 ms); wide tree of one million nodes ~10 s (review #3).

**Opening by replay.** Under `guarded` (`catch_unwind`, unchanged):
1. the snapshot is imported into a document A whose state is **never read**;
   rejected if it is *shallow*; its operations are exported
   (`all_updates`), then A is released (its drop does not recurse: state not
   decoded; after a panic, A is forgotten);
2. the operations are imported into the replica's new document;
3. **entry checks, in this order, even without `deck.slides`**: no
   tree other than `deck.slides` in the containers touched by the history
   (any tree if it is missing); depth of **all** nodes, deleted ones
   included (walking up `parent()`, never `is_node_deleted`), at most
   `MAX_TREE_DEPTH = 1024`. The same linear pass returns the set of nodes
   deleted by Loro: `check_trees` and compaction read it instead of
   calling `is_node_deleted` in a loop (O(nodes × depth): ~29 s for a
   broom of one million leaves). Then the existing checks, the first
   read, the compaction.

An accepted document has only one tree, of at most 1024 levels (export,
`fork` and drop fit in ~1 MiB of stack, under the app threads' 2 MiB;
merge, restore and history at the bound tested on 2 MiB):
nothing overflows afterwards. Rejected: `Corrupted`.

**Merge.** The trial (copy of `self` + the other's updates) passes the same
checks (`check_trees` carries the two new ones).

**Past version.** `deck_at` no longer does a `checkout`: it exports the
operations up to the version (`updates_till`), replays them in a new
document, checks it as on opening, reads it and releases it. A rejected
version returns `Corrupted`; the others stay readable.

**Documented leftovers.** Nested flat values (t-223); never call
`get_deep_value` on a received document; replay cost on opening (a
hostile file of ~18 MiB can freeze opening for ~10 s, without a crash).

**Tests.** Committed probe (`tests/loro_depth_probe.rs`). Regression
`issue_0809_deep_tree_chain.rs` on a 256 KiB stack: live chain, chain with a
deleted head, *shallow* snapshot, tree outside `deck.slides` (with or without
`deck.slides`) rejected; chain at the bound accepted then saved, merged,
restored and released on a 2 MiB stack; file with a deep history and a
flat state opened on 2 MiB. Merge
of a forged peer (deep chain, foreign tree) rejected, `self` intact.
Deep past version rejected, first and last read identically.

## Out of scope

- *Shallow* snapshot produced by the app; compaction during a session.
- Nested flat values (t-223); integer overflow on a change counter
  in `history::build` (pre-existing); pinning `loro = "=1.16.2"` in
  the workspace: separate tasks.
