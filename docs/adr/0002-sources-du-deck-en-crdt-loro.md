# ADR-0002: Deck sources as a Loro CRDT, history and merge

- **Status:** accepted
- **Date:** 2026-09-27
- **Tasks:** t-198 … t-207 (plan 1: foundation); t-243 … t-248 (plan 3:
  history mode)
- **Partly superseded by:** [ADR-0004](0004-donnees-des-hotes-fusionnees-par-cellule.md),
  [ADR-0005](0005-elements-d-une-slide-en-arbre-loro.md),
  [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md)
- **Spec:** [sources-loro-design.md](../superpowers/specs/2026-09-27-sources-loro-design.md)

## Context

The sources blob stored in the PDF (`/PieceInfo/OkuSlides/Private`)
was a plain zip of the `Deck` as JSON: no past version could be found
in it, and two copies of the same deck modified separately (sent
by email, shared in a folder) could not be recombined other
than by overwriting one of the two. Three goals arose at once:

1. **History in the file**: review and restore any past version
   of a deck from the PDF alone, with a time cursor.
2. **Merge**: two copies modified separately merge without a
   conflict dialog.
3. **Real time later**: the format must not prevent it (updates
   exchanged between peers), even though real time itself stays out of
   the scope of this plan.

## Decision

**The deck is a Loro document (CRDT), not a frozen JSON.**

- `oku-slides-core::replica::DeckReplica` wraps a `LoroDoc`. It is
  authoritative; the `Deck` (the existing application model, unchanged) is only a
  materialized **view** of it, kept up to date by `SourceBundle`, and always equal
  to the read-back of the replica. This read-back is canonical (empty runs
  removed, neighboring runs with the same format merged, blank prose absent):
  `canonicalize` puts every deck produced by `apply_all` or received by
  `SourceBundle::new` in this form.
- **Reconciliation, not translation of `Op`s.** Editing operations
  apply to `Deck` as before (`apply_all`). The `LoroDoc` is
  then reconciled with the resulting `Deck` by structural diff
  (`DeckReplica::reconcile`). The `Op` variants (move, insert,
  change a field…) are not translated one by one into Loro
  mutations: smaller surface to maintain, a single path of truth.
- **Four field categories**, set once and for all in the
  replica schema (`replica::schema`):
  - ordered collections keyed by `id`: `slides` and slots (`blocks`, `elements`,
    `children`) → nodes of a single `LoroTree` (slides at the root), see
    [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md);
  - plain prose → `LoroText`;
  - rich prose (`text: Vec<Paragraph>`) → `LoroText` with marks per
    paragraph block (`bold`, `italic`, `color`, `size`, `highlight`,
    `link`, `list`);
  - the rest → scalar or JSON atom, last writer wins (LWW), except
    `chart.option`:
    [ADR-0004](0004-donnees-des-hotes-fusionnees-par-cellule.md) partly
    replaces this rule, by merging host data per cell.
- **Empty = absent**, a model invariant: any empty prose (empty string or
  made of spaces) reads as `None`, is written as an empty `LoroText`,
  never as a removed key. Prose and `id`-keyed collections are **always
  present** from the creation of their parent (deck, slide, element), even
  empty: without this, two peers creating the same key concurrently
  (first element of an empty slide, first title) would each create their own
  container, and only one would survive — silently.
- **Undo through Loro's `UndoManager`**, one per session, never
  persisted: the stack starts from zero at each opening (accepted drawback, see
  Consequences). An undo step (one application, or one agent
  turn) groups several Loro items in a stack of our own, with its
  label: Loro 1.16.2 undoes an item carrying several `mov`s badly, hence
  one commit per `mov` (t-263); no longer needed for the deck tree since
  ADR-0006 (still used for item and data lists).
- **Total read.** `DeckReplica::deck()` never returns an error: an
  unreadable subtree (element, slide, top-level field) is dropped and
  reported by an `Issue`, reset to its default value if needed. A
  partly corrupted deck, or one merged with a malicious peer, stays
  openable.
- **v1 PDFs ignored.** A `/Version 1` envelope (`deck.json`) is treated
  as absent by `extract_sources` (`Ok(None)`), never converted. The
  zip format moves to version 2 (`manifest.json`, `deck.loro` replacing
  `deck.json`, `assets/`, `fonts/`).
- **History mode shipped (plan 3).** The full history of the replica is
  browsed and restored directly, without conversion: each notch of the
  cursor is a causal prefix (`vv_to_frontiers` of the version vector,
  not only the causal history of the last change, otherwise a concurrent
  branch earlier in the order would disappear from one notch to the next);
  restoring a version makes a **single** new undoable step (one
  notch), by reconciling the deck of that version like any edit (the
  `peers` map is not touched). No `revert_to` (observed on
  container `LoroMovableList`s, which still carry the list items).
  Details in the spec, section “Mode historique”.

## Consequences

**Positive**

- History and merge in a single format, without a conflict dialog:
  every concurrency situation has a deterministic outcome (see
  “Limites connues” in the spec), fixable afterwards or by going back to
  an earlier version.
- Real time (Loro updates exchanged between peers) becomes an addition,
  not a rewrite of the format.
- A partly unreadable deck (corruption, hostile merge) stays
  openable: total read by construction.
- **An application commit is a Loro *change***, with its label as
  message (`set_change_merge_interval(-1)`, `set_next_commit_message`):
  the history keeps a fine grain (one undo step = one *change*),
  instead of Loro grouping several in its default merge
  window. A count of “N modifications” (merge, history mode
  cursor) therefore stays meaningful.

**Accepted drawbacks**

- **The undo stack no longer survives an app relaunch** (a step back
  from t-145, which had made it persistent). It starts from zero at each
  opening of a tab, as if the deck had just been opened.
- **v1 format PDFs no longer open**: their sources blob is
  treated as absent, with no possible conversion. A v1 deck without a copy of the
  `Deck` elsewhere (e.g. in the application history removed in t-206)
  loses its editing — the rendering frozen in the PDF pages stays viewable
  like any PDF.
- **The PDF never shrinks**: the Loro history and the assets are
  append-only (never removed, so that an old version always
  renders). A heavily edited deck can see its file grow without bound.
- **Confidentiality: every shared PDF carries its whole past.** The
  full embedded snapshot contains all content that ever existed — deleted slides,
  elements and texts, earlier versions of each text,
  removed assets — as well as the timestamp of each modification
  (`set_record_timestamp`). Author names travel in the
  document too, since plan 2: the root `peers` map associates the `PeerID` of
  each peer with the name taken from `[author] name` or from the system user, at
  its first edit. Any recipient can read this
  history back with a Loro tool. Accepted by design: no
  flattening (snapshot without history) on save, which
  would break history and merge. Mitigation: the export without sources,
  `oku-slides strip <entrée> <sortie>` (`oku_slides_pdf::strip_sources`),
  which keeps only the rendered pages; for now on the command line
  only, not yet in the app.

**Neutral / known limits** (see the spec for full details)

- Move ‖ delete an element concurrently: the element reappears.
  Edit ‖ delete: the edit is lost. Two concurrent additions with the
  same `id`: duplicate reported by `validate`.
- The Loro decoder itself can overflow the stack on a forged
  snapshot, with a flat value nested tens of thousands of
  levels deep (a few KiB compressed): Loro decodes it recursively on
  first access and frees it on drop, not at `import`, outside our code,
  out of reach of `catch_unwind` and of the byte cap. Local denial of service
  when opening a forged PDF. Tracked as a separate fix.
- Resume writes `deck.loro` then `document.json`: an abrupt stop between
  the two can leave a new replica next to an older `document.json`
  (stale `dirty` / `saved_hash`) — the old format wrote
  only one file.

## Rejected alternatives

- **`git bundle` + three-way merge.** Structural merge (which
  element moved, which field changed) and conflict resolution
  would have to be written by us on top of a text format; no path
  to real time (no incremental updates exchangeable between
  peers).
- **`Op` log replayed between peers.** `Op`s do not commute
  (moving an element while another peer deletes it, editing the same
  text in two places): reconciling them correctly would amount to
  reinventing a CRDT, with fewer guarantees than Loro already offers.
- **Flattening the history on export.** Rejected along with the goal:
  the full history must **always** travel with the PDF, not
  only on demand.
