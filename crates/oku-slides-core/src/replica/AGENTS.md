# crates/oku-slides-core/src/replica — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Treat DeckReplica as the source of truth; Deck is only a view equal to rereading the replica. (ADR-0002)
- Write empty or blank prose as an empty LoroText, never by removing the key; read it back as absent. (ADR-0002)
- Create prose, rich, list and tree containers with their parent, even empty, so concurrent creations merge. (ADR-0002, plan:2026-09-27-sources-loro-fondation)
- Keep DeckReplica deck() infallible: drop what cannot be read and report an Issue instead of an error. (ADR-0002)
- Keep the Loro change merge interval disabled so that each commit is exactly one labelled change. (ADR-0002, spec:2026-09-27-sources-loro-design)
- Build each history step as a causal prefix of the version vector, not one change's causal history. (ADR-0002, plan:2026-09-27-sources-loro-historique)
- When reordering tree children, keep a longest increasing subsequence in place and move only the rest. (ADR-0005, plan:2026-10-09-elements-en-arbre)
- Bound slot depth by MAX_SLOT_DEPTH on read and write, return SlotsTooDeep before any deck write, and refuse saving a tree past MAX_TREE_DEPTH with TooDeep. (ADR-0005, ADR-0006, plan:2026-10-10-deck-en-arbre, t-866)
- Never drop an element because of its slot; repair it, falling back to orphans, and emit issue-slot-repaired. (ADR-0005, t-694)
- Keep apply of an unchanged deck write-free; only read repairs may write, once, on the first apply. (ADR-0005, plan:2026-10-10-deck-en-arbre)
- Remove nodes with this peer's deletion mark in their meta, never tree.delete; reconciling never clears a mark. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Create a fresh node when a deleted id is added again; never match or revive a marked node. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Never read, index or write below a marked node; a marked node hides its whole subtree. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Create the slot map only with ensure_mergeable_map, never get_or_create_container. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Key slot entries by the parent TreeID string, and ignore keys that are not canonical TreeIDs. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- On import and merge, refuse as corrupted only tree states no code path produces, undo and redo included. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Identify slides and elements by the id in their meta, never by their TreeID. (ADR-0006, plan:2026-10-09-elements-en-arbre)
- Pair elements across slides only via move_element, or an id unique in two kept slides with the same kind. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Never index with lengths read from the replica; use get and fail with TreeOrder, never a silent fallback. (plan:2026-10-10-deck-en-arbre)
- Keep reading, writing and input checks of an untrusted replica iterative, never unboundedly recursive. (plan:2026-10-10-deck-en-arbre)
- Keep reading the replica independent of the widget registry; only writing may consult it. (plan:2026-09-28-widgets-typst-2)
- Build the UndoManager after the initial commit so that creating the deck is not undoable. (plan:2026-09-27-sources-loro-fondation)
- Separate rich prose paragraphs in the replica text with U+2029. (plan:2026-09-27-sources-loro-fondation)
- Cap replica dataset reads at 10000 rows and 1000 columns, truncating and reporting the excess. (plan:2026-10-08-fusion-cellules)
- Use a fresh random PeerID on every load and never persist it; two writers sharing a peer corrupt the doc. (spec:2026-09-27-sources-loro-design)
- Import untrusted bytes and do the first full read inside guarded; after a panic, forget the doc, never drop it. (spec:2026-09-27-sources-loro-design)
- Merge two replicas only when their deck_uid is equal; otherwise fail with Unrelated. (spec:2026-09-27-sources-loro-design)
- Never record a history version for a change whose message starts with PART (oku-part:). (t-290)
- Write choice, asset and other non-prose params as Loro atoms, never merged per character. (spec:2026-09-27-widgets-typst-design)

## Pitfalls

- Do not rely on catch_unwind or byte caps against deeply nested Loro values; a stack overflow aborts. (ADR-0002, t-809)
- Silently skip, with their subtree, roots without the slide flag and nodes with empty meta; undo creates them. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Expect undoing an element or slide add to be a real Loro delete that loses subtrees a peer moved under it. (ADR-0006, t-776)
- Mark only visible nodes; Loro skips an equal insert, which would leave undo nothing to revert. (plan:2026-10-10-deck-en-arbre)
- On import, check deck_uid, then the replica format, then check_trees, so old replicas read as unsupported. (plan:2026-10-10-deck-en-arbre)
- Group undo steps in our own stack or by merge key; never use Loro group_start. (spec:2026-09-27-sources-loro-design)
- Never call get_deep_value on a received Loro document; its recursion can overflow the stack. (spec:2026-10-10-arbre-deck-robustesse-design)

## Testing

- Do not count issue-slot-repaired as a read loss in assertions or unreadable-issue filters. (plan:2026-10-09-elements-en-arbre)

## Settled decisions

- Apply edits by diffing the Deck into the replica; never translate Op variants into Loro mutations. (ADR-0002)
- Never persist the undo stack; it belongs to the session and starts empty on every open. (ADR-0002)
- Restore a version as one undoable apply, tried first on a fork; never use Loro revert_to. (ADR-0002, spec:2026-09-27-sources-loro-design)
- Store host datasets as row, column and cell containers matched by id, never as one JSON atom. (ADR-0004, plan:2026-10-08-fusion-cellules)
- Keep the whole deck in one LoroTree whose roots, and only they, are slides; elements are descendants. (ADR-0006, plan:2026-10-10-deck-en-arbre)
- Store one deletion mark key per peer, never a shared boolean, so one undo cannot cancel a peer's removal. (plan:2026-10-10-deck-en-arbre)
- Commit before each mov on movable lists such as items and dataset; never split commits for deck tree moves. (ADR-0006, t-269)
- Use silent commits (oku-silent origin) for non-undoable changes such as load-time canonicalization. (plan:2026-09-27-sources-loro-fondation)
