# ADR-0006: Deck as a single Loro tree, soft delete

- **Status:** accepted
- **Partly superseded by:** [ADR-0007](0007-compaction-des-noeuds-morts.md)
  (compaction)
- **Date:** 2026-10-10
- **Tasks:** t-746, t-747, t-776, t-809
- **Partly supersedes:**
  [ADR-0005](0005-elements-d-une-slide-en-arbre-loro.md) (one tree per
  slide, `@slot` by parent id, deletion by `tree.delete`) and
  [ADR-0002](0002-sources-du-deck-en-crdt-loro.md) (slides as
  `LoroMovableList`)
- **Probe:** `crates/oku-slides-core/tests/loro_deck_tree_probe.rs`
  (complements `loro_tree_probe.rs`)

## Context

- **One tree per slide.** Moving an element from one slide to another is a
  copy followed by a deletion: the concurrent edit is lost.
  The interface, for that matter, never moved an element from one slide to another:
  cut removed the element, paste placed a copy with renewed ids.
- **`tree.delete`.** Loro never resurrects a node. Undoing a removal
  recreates a copy, and a peer's concurrent `mov` or edit is lost.
- **Slides as `LoroMovableList`.** Undoing a removal against a peer's `mov`
  leaves two slides (t-652), and the t-263 workaround requires one commit per `mov`.

## Decision

- **A single `LoroTree`**, under `deck.slides`. The roots, and only they,
  are the slides: their meta is the slide without its slots, plus
  `@slide = true`. The descendants are the elements. Fractional order,
  jitter 0.
- **`@slot`** is a **mergeable** map (`ensure_mergeable_map`):
  { parent `TreeID`, as `counter@peer` → slot name }, at every
  level (slide, element, nested element). Two peers that create it in
  parallel keep both their entries (t-188).
- **Soft delete.** Removing a node sets a mark `@deleted/<peer>` =
  the Unix time it was set (t-770) in its meta, never `tree.delete`. The mark
  is per peer, and not shared: with a common mark, A's undo would erase B's concurrent
  deletion (probe, `p1_concurrent_double_soft_delete_then_one_undo`
  versus `p1_per_peer_mark_keys_survive_a_concurrent_undo`). A node is
  deleted as long as it carries a mark. Reconciliation never removes the
  mark of a removal, and a re-added id creates a new node.
- **Marked birth (t-776).** A creation, within an undoable step, is born
  marked in a silent commit, then the mark is removed within the step:
  undoing the addition restores the mark instead of a Loro `tree.delete`. All the
  nodes of a reconciliation are born together, in a single commit (created at the end
  of their parent, in owner order), before placement; the
  placement treats them as present and moves them only if they are
  misplaced. Without a step (`new`, `apply_silent`), creation stays direct.
- **Reading and writing never descend below a marked node**: a
  live node under a marked parent is hidden with it.
- **Strict input.** On import and on merge, the following are rejected (`Corrupted`):
  a `@slide` outside the roots, a malformed mark or `@slot`. Only what
  no code path produces, undo and redo included, is
  rejected; each rejection is justified that way. A root without `@slide` is
  accepted and ignored with its subtree: it is the empty node that Loro creates when
  undoing a `mov` whose target has disappeared.
  The state of a received snapshot is never read: its operations are replayed
  into a new document (a *shallow* snapshot is rejected). On that document,
  before any other check: no tree other than `deck.slides`; all
  nodes, deleted ones included, at most 1024 levels deep (t-809, probe
  `tests/loro_depth_probe.rs`).
- **Matching.** An object's identity is the `id` of its meta, never the
  `TreeID`. Slides are matched first, then elements within their slide,
  then from one slide to another, **only** for a move announced by
  `move_element`, or inferred without ambiguity: an id unique on both sides, between
  two kept slides, with the same `kind`. A marked node is never matched.
- **An announced move is authoritative**, even if the source slide is removed
  in the same transaction (emptying a slide into another, then removing it):
  the element leaves it with the peer's concurrent edit, and its unwanted
  descendants are marked. A removed slide is never the source of an
  inferred move.
- **Paths that produce a move between slides:** the
  `move_element` op (the agent's `apply` tool, the interface's cut and paste), and
  an `apply` that removes an id from one slide and adds it to another, under the
  conditions of the inferred move.
- **The t-263 workaround remains only for lists** (items, `dataset`).

### Reading

- **Empty node.** A node whose meta has no key is ignored with its
  subtree, without an `Issue`: it is born from a normal path (two crossed
  undos) and carries no data.
- **Resolved slot.** The `@slot` keys are read back through `TreeID::try_from` and
  count only if they are canonical. When the parent has changed (parent
  recreated by "redo"), the repair is silent if the keys of the
  vanished parents all give the same slot; entries of live but stale
  parents are ignored. Otherwise `issue-slot-repaired` is emitted
  (probe, `p9_tree_id_string_key_is_stable_across_snapshot_and_peers`).
- **Slide order.** The unmarked roots that carry `@slide = true`,
  in Loro's order.

## Compatibility

None. `REPLICA_FORMAT` goes from 2 to 3 and `SOURCES_VERSION` from 8 to 9.

- A v8 `deck.loro` read back outside a zip is rejected
  (`replica-unsupported-version`), and the session resume is set aside
  (`.failed-*`).
- The deck JSON does not change (`DECK_VERSION` stays 5). The behavior of
  "cut" changes: cut marks the element, paste moves it.

## Consequences

- A move between slides, or between slots, keeps the concurrent
  edits.
- Undoing a removal brings back the same node, with a peer's `mov` and edit,
  even when received between the removal and the undo
  (`p1_undo_soft_delete_after_receiving_move_does_not_move_back`).
- A child that a peer moved under a removed parent comes back with it on
  undo (`p3_move_under_soft_deleted_parent_reappears_on_undo`).
- Reordering slides takes one commit.
- Cost measured by the probe (`p8_cost_single_tree_vs_tree_per_slide`, deck of
  40 slides × 20 elements, one debug run and one release run): build
  6-7 ms versus 4-15 ms for one tree per slide, raw tree read
  0.4-1.0 ms versus 0.6-1.3 ms, snapshot after 200 passes of about 100 KB
  versus 120 KB. These figures are indicative and do not cover marked
  nodes. A full deck read (`deck()`, 40×20, release) measures
  about 6.5 ms, as before the migration.

## Limits

- **Marked nodes remain until `compact_after_days` of activity
  (ADR-0007).** The snapshot grows in the meantime, and reading as well as each
  `apply` cost O(live + marked), because `children()` also returns the
  marked ones. `max_replica_bytes` is checked only on import: a heavily
  edited deck may one day be rejected on resume.
- **The input check (`check_trees`) costs O(all nodes)**, marked ones
  and descendants of marked ones included, on every import and every merge. Measurement
  from the final review, 40×20 after about 10,000 marked nodes:
  `from_snapshot` 69 ms versus 22 ms before the migration, `merge` 96 ms versus
  55 ms.
- **Opening replays the whole history** (chain of 100,000 ops: ~60 ms;
  1 M: ~0.8 s; wide tree of one million nodes: ~10 s). Nested flat
  values (t-223) remain unbounded.
- The replay on opening (t-809), like a merge, combines into a single change
  the consecutive changes of the same peer, with the same message, within the same
  second (Loro 1.16.2, fixed interval): two edits with the same label within
  the same second become a single version after reopening.
- A mark from a closed session can no longer be undone: the undo stack starts
  from scratch on resume.
- **Undoing an addition also hides what a peer moved beneath it**; the peer's
  ⌘Z, or redo, brings it back. Undoing the addition leaves the node, reduced to its
  mark (its `id` goes away with the step). An application that creates nodes costs
  one more silent commit (and one more part), whatever their number, and the step stays at one item. Measurement (release, medians of 5
  alternating, heavily loaded machine, load ~370), pasting n elements, without
  then with marked birth: n=50 flat apply 2.7 → 2.2 ms, ⌘Z 3.0 → 2.4 ms;
  n=50 nested 6.0 → 5.7 ms, 5.9 → 8.7 ms; n=500 flat 23 → 20 ms, 26 → 37 ms;
  n=500 nested 53 → 50 ms, 90 → 97 ms (⌘Z ×1.1 to ×1.5). Being born marked
  node by node split the step into one item per node (⌘Z of 500 nested:
  82 s): hence the grouped birth. The merge report does not count the
  parts.
- An element moved under an element, or into a slide, removed in parallel
  is hidden with it; undoing the removal brings it back.
- Without `move_element`, an id that appears in several slides is never
  matched from one slide to another: its concurrent edit follows the earlier
  rule (lost).
- Removing an id from one slide and adding it to another, in the same
  `apply`, counts as a move when the move is inferred: the peer's concurrent
  edit follows the node.
- A re-added id, followed by undoing the earlier removal, leaves two nodes
  with the same id, repaired on read (t-652).
- Jitter 0 and the implicit `Rearrange` now also affect **slide
  order**: two concurrent slide insertions at the same rank, followed by an
  insertion between them, can bring back a slide that a peer was moving.
- A forged replica whose tree is malformed is rejected as a whole, on import
  as on merge.
- **Hidden depth (t-866).** A marked subtree keeps its depth and follows its
  live ancestor when it moves down; writes only bound live nodes
  (`MAX_SLOT_DEPTH`). Removals followed by moves down can therefore pile up
  hidden levels past 1024. Saving checks the depth of every node, marked and
  deleted ones included, and refuses with `ReplicaErrorKind::TooDeep`
  (`replica-too-deep`) rather than write a file that would not reopen; undoing
  the move makes saving possible again. Compacting the deep marked subtree was
  rejected: `tree.delete` keeps its inner depth under Loro's deleted root, which
  the input check also measures, and it would break the undo of the mark.
  Merging two copies whose depths add up past the bound is already refused by
  the merge's input check (`Corrupted`).
- **Trust limit.** A peer can delete any node, slide
  or element, with a `@deleted/<peer>` mark, as it could with
  `tree.delete`. The mark is well formed, hence accepted. Only the undo
  of the peer that set it removes it: the mark of an absent peer, or of a
  closed session, hides the node for good, without an `Issue`.
- An empty node is born from an undone `mov` whose target was compacted (ADR-0007).
- Loro stays pinned by `Cargo.lock` (1.16.2). No `cargo update -p loro`
  without rerunning `tests/loro_tree_probe.rs` and
  `tests/loro_deck_tree_probe.rs`.
