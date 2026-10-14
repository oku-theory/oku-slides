# ADR-0005: Slide elements as a Loro tree

- **Status:** accepted
- **Date:** 2026-10-09
- **Tasks:** t-694 (t-740 to t-745)
- **Partly supersedes:** [ADR-0002](0002-sources-du-deck-en-crdt-loro.md)
  (slots as `LoroMovableList`)
- **Partly superseded by:** [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md)
  (one tree for the whole deck, `@slot` by `TreeID`, soft delete)
- **Probe:** `crates/oku-slides-core/tests/loro_tree_probe.rs`

## Context

A slot (`blocks`, `elements`, `children`) is a `LoroMovableList` of
`LoroMap`. Moving an element to another slot, or another depth, therefore means
removing it from one list and inserting it into another: the Loro identity is
lost, and so is the concurrent editing of the moved element.

Moreover, Loro 1.16.2 badly undoes several `mov` from the same commit (t-263).
The replica therefore writes one commit per `mov`.

## Decision

- **One `LoroTree` per slide**, under the `@elements` key of the slide's
  map. The roots are the elements of the slide's slots, the children
  of a node are the elements of that element's slots.
- **A node's meta is the element's map**, without its slots.
- **`@slot`**, in the meta, is a map { parent id (`""`: the slide)
  → slot name }. There is one entry per parent, never erased, and only
  the actual parent's entry counts: the others serve as fallbacks.
- **Fractional ordering**, with a jitter of 0.
- **Placement by relative order** within each (parent, slot), via
  `mov_after` and `mov_before`. The nodes of a longest increasing
  subsequence do not move, so as not to touch those a peer is moving.
- **Reconciliation by element id**, never by `TreeID`. An element's identity
  is the `id` of its meta.
- **The deck JSON does not change.** Slides and list items stay as
  `LoroMovableList`.

### Repair on read-back

Reading is total and does not trust the registry. The resolved slot of a
node under its actual parent is, in order:

1. `@slot[pid]`, if it is a non-empty string;
2. otherwise, the slot of the first sibling that has one;
3. otherwise, the first valid value of `@slot`, in key order;
4. otherwise `orphans`.

Cases 2 to 4 emit `issue-slot-repaired`. An element is never dropped
because of its slot.

- **Duplicate ids.** `unique_ids` applies to the whole tree (t-652): an
  exact copy is dropped within the same slot, and an occurrence in another
  slot is renamed `{id}-{n}`.
- **Depth.** It is bounded by `MAX_SLOT_DEPTH` on read **and** on
  write: a deeper JSON deck is rejected with `SlotsTooDeep`, before
  any write into the slide. `issue-slot-too-deep` is emitted once per
  node.
- **Unrendered elements.** An element placed in `orphans`, or in a slot
  that its parent does not declare, is kept in the JSON but is not
  rendered: the layout reads only the declared slots. `validate` reports
  `issue-unknown-param`, and the next `apply` records the repair
  (`@slot[pid] = "orphans"`). A repair on read-back is therefore written once;
  the invariant "nothing is written when nothing changes" holds from
  the second `apply` on.

## Compatibility

None. `SOURCES_VERSION` goes from 7 to 8, and `DECK_VERSION` does not change
(5).

- The `meta.format = 2` marker is checked on import and on merge.
- A v7 resume `deck.loro` is rejected (`replica-unsupported-version`) and
  the session sets its folder aside (`.failed-*`). The unsaved changes of a
  resumed session are therefore kept, not read back, on the
  first launch after the update.
- Rule: changing `REPLICA_FORMAT` requires changing `SOURCES_VERSION`.

## Consequences

- A move between slots or between depths keeps the concurrent edits
  of the element.
- The t-263 workaround goes away for elements: the tree is written without a commit per
  `mov`. It remains for list items only.

## Limits

- An element moved under an element deleted in parallel is hidden with it;
  undoing the removal brings it back (ADR-0006) (`loro_tree_probe.rs`, point 3a).
- Two crossed nestings that would form a cycle: the first `mov`
  in (lamport, peer) order wins. The loser stays visible at its former
  place, because its `@slot` entry for the former parent is intact.
- With a jitter of 0, two concurrent insertions at the same place receive
  the same fractional index. An insertion between them makes Loro emit a
  `Rearrange`, that is implicit `mov` of siblings, which can
  bring back a sibling that a peer was moving in parallel.
- Loro is pinned by `Cargo.lock` (1.16.2): no `cargo update -p loro`
  without rerunning the probes (`tests/loro_tree_probe.rs`,
  `tests/loro_deck_tree_probe.rs`).
