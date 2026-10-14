# ADR-0007: Compaction of dead nodes in the deck tree

- **Status:** accepted
- **Date:** 2026-10-11
- **Tasks:** t-770
- **Partially supersedes:**
  [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md) ("never
  `tree.delete`")

## Context

Nodes marked `@deleted/<pair>` pile up (ADR-0006): reading and every `apply`
cost O(live + marked), and the snapshot keeps growing.

## Decision

- **Dated mark.** A mark's value is the Unix time it was set (`i64`, the
  author's clock): Loro only exposes the last author of a key, not the date of
  each mark.
- **Limit.** `limit = min(now, most recent change) − compact_after_days`.
  A mark is therefore compacted only after 30 days of document **activity**, not
  of mere clock time: a local clock far ahead compacts nothing more.
- **Compacted:** a node that carries a mark dated in `]0, limit]` (a single one
  is enough: only its peer could remove it, and its window has passed), or an
  empty node whose last `mov` is dated in `]0, limit]`. `tree.delete`
  takes it away with its subtree, already hidden.
- **When.** On opening a file, under the guard (Loro changes are not
  reliable), before the undo stack: compaction cannot be undone.
- **Silent commit** with message `oku-compact`, dated like the most recent
  change (`set_next_commit_timestamp`), so that opening a file does not count
  as activity. No history step, no count in the merge report.
  The history stays whole.

## Consequences

- The node count follows the live nodes and those with less than 30 days of
  activity.
- Measurements (release, 40 slides × 20 elements, 10,000 marked nodes, before
  then after compaction): opening (`from_snapshot`) 104 ms then 25 ms; `apply`
  37 ms then 11 ms; `merge` 151 ms then 83 ms; `deck()` unchanged (11 ms).
  The snapshot size does not drop (2.24 MB then 2.28 MB): the Loro history
  is kept, only the cost of walking the tree decreases.
- A compacted copy and a copy that is not converge on merge.

## Limits

- Another copy, open for longer, that then undoes its mark or the `mov` of an
  element under this node, loses this node.
- A mark restored by ⌘Z or redo keeps its original date.
- An author clock running behind shortens the window.
- A forged file can get a node compacted that it could already hide, or
  carry an `oku-compact` message that hides a change from the history and the
  report (its content stays visible in later versions).
- The snapshot does not shrink: the history is kept.
