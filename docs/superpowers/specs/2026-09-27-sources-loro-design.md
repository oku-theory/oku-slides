# Deck sources as a Loro CRDT: history and merge — design

Date: 2026-09-27 · Task: t-188

## Goal

The sources blob stored in the PDF becomes a **Loro** document (CRDT):

1. **History in the file**: any past version of a deck can be reviewed
   and restored from the PDF alone, with a time slider.
2. **Merge**: two copies of the same deck modified separately (mail,
   shared folder) merge without a conflict dialog.
3. **Real time later**: the format does not prevent it (Loro updates
   exchanged between peers). Real time itself is out of scope.

The full history **always** travels with the PDF (no "clean" export, no
flattening).

Out of scope: real time, visual diff of slides, flattening the
history, reading PDFs in the v1 format.

## Decisions

- **Loro rather than Git or a home-made op log.**
  - `git bundle` + three-way merge: structural merge and conflict resolution
    to write ourselves, no path to real time.
  - Replayed `Op` log: ops do not commute (move ‖ delete,
    edit the same text); that would be reinventing a CRDT.
  - Loro (Rust, 1.x): history, merge, `checkout`, per-peer undo
    and sync protocol in a single format.
- **Reconciliation rather than translating ops.** `Op`s apply
  to `Deck` as today; the `LoroDoc` is then reconciled with the
  new `Deck` by structural diff. `Op` variants are not
  translated one by one.
- **Empty = absent** for all prose (see "Schema").
- **Undo through Loro's `UndoManager`**, not persisted: the undo
  stack starts from scratch on relaunch (accepted regression on t-145).
- **v1 PDFs ignored**: a v1 blob is treated as absent. No
  conversion.

## Spike (throwaway, outside the repo)

Loro 1.16.2, `fixtures/roadmap.json` (8 slides) and a synthetic deck of
40 slides:

| Measure | Result |
|---|---|
| Round trip `Deck → Loro → Deck` | identical |
| Reconcile the same deck | 0 ops, ~1 ms (40 slides) |
| Merge of two branches (title edited on both sides, move, delete, add) | convergence, `Brouillon : Contexte (révisé)`, `validate` without issues |
| Back to the original version from the snapshot | identical |
| `UndoManager` after merge | undoes the local change, keeps the remote one |
| 539 one-character commits | +~330 B, ~40 µs per commit |
| Snapshot with history | 5.5 KB (roadmap), versus 1 KB for the v1 zip |

Observed limits, reproduced on raw Loro:

1. Rich text runs reconciled by index: bold ‖ typing gives
   `Le  [ajout B]**marché** a doublé…`. **Verified workaround**: one
   `LoroText` with marks per block of paragraphs.
2. Concurrent creation of the same absent text field: only one text
   survives. A pre-existing text, even empty, merges (`AB`). **Workaround**:
   prose fields always created.
3. Move ‖ delete: the element reappears.
4. Edit ‖ delete: the delete wins.
5. Two adds with the same `id`: two elements; `validate` reports it.

## `LoroDoc` schema

Root `deck` (`LoroMap`), plus two root maps:

- `meta`: `deck_uid` (random, set at creation, never modified).
- `peers`: `{ <PeerID>: "<author name>" }`, one entry per peer, registered
  before its first edit, in the same commit (`claim_author`). The name
  comes from the `[author] name` setting, otherwise from the system user
  (`USER`/`USERNAME`); blank on both sides → peer without an entry, its
  changes stay anonymous in a `MergeReport`. The registration is
  part of the undoable step: undoing a peer's first edit also removes
  its entry (a fully undone edit left nothing to
  sign), until its next edit, which registers it again.

Each model field belongs to a category:

| Category | Container | Fields |
|---|---|---|
| Ordered collection with `id` | `LoroMovableList` of `LoroMap` | `slides`, `blocks`, `elements`, `children` |
| Plain prose | `LoroText` | slide: `name`, `description`, `title`, `subtitle`, `kicker`, `text`, `notes`; `params`: `title`, `subtitle`, `author`, `organization`, `date`; card: `title`, `kicker` |
| Rich prose | one `LoroText` with marks | `text: Vec<Paragraph>` of the `text` and `card` elements |
| LWW scalar | value | `id`, `kind`, `layout`, `theme`, `templates`, `version`, `params.lang`, `asset`, `fit`, `shape`, each field of `frame`, `cell`, `style`, `grid`, group `layout`; each entry of `vars`; each leaf of `tokens` and of `params.extra` |
| LWW JSON atom | one value (whole JSON) | `chart.option` |

- **Rich prose.** Paragraphs joined by `\n`. Run marks: `bold`,
  `italic`, `color`, `size`, `highlight` (`expand: After`), `link`
  (`expand: None`). A paragraph's `list` is a mark on its `\n`.
  Reading rebuilds the `Run`s from `to_delta()`.
- **Prose and collections always present.** When the deck, a
  slide or an element is created, all its prose `LoroText`s and all its
  `id` collections (`slides`, `blocks`, `elements`, `children`) are
  created, empty if need be. A value absent from the JSON empties the container,
  without ever removing the key. Otherwise, two peers that create the same key
  concurrently (first element of an empty slide, first title) each make
  their own container, and only one survives.
- **Paragraph separator**: `U+2029` (and not `\n`, which a run can
  contain). Each paragraph ends with this separator, the last one
  included. Adjacent runs with the same formatting are merged on
  read (canonical form).
- **Empty = absent, a model invariant.** `opt_string` (`ops.rs`) converts
  an empty or whitespace-only string to `None` (same rule as the UI) (covers `SetSlideField` and `SetParam`); the Loro read
  turns an empty text into `None`. The UI normalizations
  (`deck.ts:76`, `a2ui/panelActions.ts:25`, `App.tsx:494`) go away.
  Audit: no template or test relies on `Some("")`; the only effect
  lost is forcing an empty title on a title slide.
- **Chart as an atom**: `consistent_chart` rejects inconsistent
  options; a field-by-field merge could produce some. General
  rule: a subtree that carries a global invariant is an atom.
- **`frame` field by field**: move ‖ resize keeps both.

## Code

- **`oku-slides-core::replica`** (`replica.rs`, tests `replica_tests.rs`):
  `DeckReplica`, wrapper around a `LoroDoc`.
  - `DeckReplica::new(deck)` (new `deck_uid`), `from_snapshot(bytes)`
    (also returns the first read, done under guard), `snapshot()`.
  - `reconcile(&Deck)` then `commit(label)`.
  - `deck() -> (Deck, Vec<Issue>)`: **total read**.
  - `merge(&DeckReplica)`: checks `deck_uid`, imports.
  - History: `versions()` (topological order, session and merge
    markers), `deck_at(index)`, `restore(index)`.
- **`SourceBundle`** gains `replica: DeckReplica`. `deck` stays a
  materialized view used by rendering, `validate`, `locate` and the ops.
  **Invariant: the view equals the read-back of the replica**
  (`replica().deck().0 == *deck()`). Since the read-back is canonical, the view
  must be too: `oku_slides_core::canonicalize(&mut Deck)` (`canonical.rs`)
  is called at the end of `apply_all` and in `SourceBundle::new`. Rules:
  empty runs removed, neighbouring runs with the same format merged, `U+2029` in
  a run's text replaced by `\n` (the content is kept, without a new
  paragraph); optional whitespace-only prose → `None`, other prose
  trimmed (`opt_string` rule); whitespace-only notes → `""`; JSON
  integer beyond `i64` (`vars`, `params` extras, `tokens`) → float,
  as Loro stores it; empty `tokens` → `null`. `SourceBundle::apply`
  expects a canonical deck and checks it with `debug_assert!` (debug
  builds). A property (`canonical_tests.rs`, proptest) guarantees it:
  for any canonical deck `d`, `DeckReplica::new(&d).deck().0 == d`.
  Still outside the invariant: a value nested beyond
  `MAX_DEPTH` (64) in `vars`, `tokens` or `params`, truncated on read.
- **`oku-slides-core` removes** `History` (`history.rs`) and the inverses:
  `apply_all` applies atomically and returns nothing.
- New dependency: `loro` (workspace). Dev: `proptest`.

## Write flow

`Session::apply`:

1. `canonicalize_charts(ops)`.
2. `deck' = apply_all(clone(deck), ops)` (canonical form); error → nothing moved.
3. Render `deck'`; error → nothing moved.
4. `replica.apply(&deck', label)`, `deck = deck'`.

Rendering happens **before** the commit: the Loro history is final, a
failed render must not leave a trace in it.

- **One commit = one Loro *change*.** `set_change_merge_interval(-1)` disables
  the automatic merging of local commits within a time window (1000 s by
  default); without it, Loro would group several commits into a single
  *change* and "N changes" (merge, history mode slider) would
  no longer mean anything. The application label (`apply(deck, label)`)
  is also set as the *change*'s message (`set_next_commit_message`), so
  that an external tool reading the Loro history finds the intent of
  each step without going through the app.
- **One `mov` per commit** (t-263). Loro 1.16.2 badly undoes an undo
  item that carries several `mov`s on a list of containers: an
  element is deleted (durably, in the oplog), or Loro panics. The
  reconciliation therefore commits just before each `mov`; an application
  that reorders `k` times makes `k + 1` *changes*. The intermediate
  commits carry the message `oku-part:<label>`: the state that
  follows them is not a deck version (history mode skips them).
- **Peer**: random `PeerID` on each load (open, resume,
  new tab), never persisted. Two writers of the same peer
  would corrupt the document.
- **Timestamps**: `set_record_timestamp(true)`.
- **Undo / redo**: one `UndoManager` per session, `merge_interval
  = 0`, under a stack of our own **steps** (`undo.rs`): a step = one
  application (or a group), made of the Loro items of its commits (counted
  by `on_push`), undone or redone all together. Labels, groups (an
  agent turn) and limit (`settings.history.limit` steps) live in
  this stack; never Loro's `group_start`, which would merge the items
  of a group into one and bring back the `mov` bug. Loro keeps at
  most 64 items per step of the limit; a step it has partly
  discarded is no longer undoable (never half undone).
  After an undo or a redo: deck read-back, render; a failed render keeps
  the state and surfaces the error.
- **Templates mode**: same `SourceBundle`, replica never saved to PDF.

## Resume

`tabs/t-<n>/deck/`:

```
deck.loro       snapshot, atomic write
document.json   { path, dirty, saved_hash }
assets/<hex>    unchanged
fonts/<name>    unchanged
```

`history` disappears from `document.json` and from `resume.rs`. The resume
spec (`2026-09-26-resume-state-design.md`) is updated: the undo
stack no longer survives relaunch. `TabInfo` gains `deck_uid`
(in `workspace.json`), to detect an already open deck without waking
a tab (ADR-0001).

## Format in the PDF

Envelope unchanged (`/PieceInfo/OkuSlides/Private`), `/Version 2`.
`extract_sources`: version < 2 → `Ok(None)` (open: `NotOkuSlides`);
version > 2 → `UnsupportedVersion`.

v2 sources zip:

```
manifest.json    { format: "oku-slides-sources", version: 2 }
deck.loro        full snapshot (ExportMode::Snapshot)
assets/<hex>     as is
fonts/<name>     as is
```

- No more `deck.json`: the deck is rebuilt from `deck.loro`.
- **Assets and fonts append-only**: never removed, an old
  version must still render. Merge = union.
- **Untrusted blob**: import **and first complete read**
  (`deck_uid`, the whole `deck` tree) under `catch_unwind`, because Loro only
  decodes a container's state on first access (`from_snapshot` returns
  this read, which `SourceBundle::from_replica` takes over without reading again);
  maximum size of `deck.loro` (`[sources] max_replica_mib = 32`);
  panic or rejected bytes → `Corrupted`. After a panic, the doc
  (poisoned locks) is abandoned without drop. `merge` tries the import and
  the read-back under guard on a copy before importing. Still outside the guard:
  the containers the first read does not reach (outside `deck`/`meta`,
  beyond `MAX_DEPTH`).
- **Untrusted zip**: each entry is read with a capped initial capacity
  (1 MiB, the announced size is only indicative) and a bounded
  read (`take`); the total decompressed size is bounded by `[sources]
  max_sources_mib = 256` (anti-bomb), overflow → `SourcesErrorKind::TooLarge`
  (`sources-too-large`). Unknown entries are not read.
- **Total read, three levels**: an unreadable element is discarded
  (`issue-unreadable-element`), then a slide (`issue-unreadable-slide`),
  then a top-level deck field (`theme`, `params`, `vars`,
  `templates`), reset to its default value (`issue-unreadable-field`).

## Merge

- **Kinship**: two replicas merge only if their `deck_uid` is equal;
  otherwise `Unrelated` ("these two presentations have no common
  origin").
- **Steps** (`SourceBundle::merge`, called by `Session::absorb`): read the
  incoming blob, check `deck_uid`, `import`, union of assets and fonts,
  read back (total read), render. The notice and `dirty` are set as soon as the
  merge is done, before the render: a failed render must not let merged
  content pass for saved. A merge with nothing new (0
  changes, no asset or font added) does not mark the document
  modified; when requested (`merge_file`), it still sets the "0" notice; on
  save, it sets none (the "Enregistré" (Saved) toast is enough).
- **Notice**: `notice-merged` ("Fusionné avec *x.pdf* — modifications
  importées : N (auteurs)", i.e. "Merged with *x.pdf* — imported changes: N (authors)")
  when the merge imports at least one named peer,
  `notice-merged-anonymous` (without the author list) otherwise — peers without an
  entry in `peers`, or a merge with 0 changes. N counts the imported *changes*,
  not only the edits: undos and redos,
  silent canonicalizations and author registrations count too.
  The notice names only registered authors and does not mention anonymous
  peers (the `MergeReport` counts them, `anonymous`). A notice is appended to
  the one still pending (dormant tab woken by the merge and read back
  with its own notice, file opened with unreadable parts…),
  separated by a blank line; each is shown only once.
- **Triggers**, as built (three deviations from the initial
  design, below):
  1. *Fichier → Fusionner avec un PDF…* (File → Merge with a PDF…) on the active tab
     (`merge_file`, refused in templates mode).
  2. **On save** (`save_checked`): if the file on disk has
     changed since opening or the last save (`saved_hash` fingerprint,
     compared on the bytes before any decoding) and carries the same
     `deck_uid`, merge then write. If it carries another deck, no longer has
     Oku Slides sources, or is unreadable: nothing is written, the next
     snapshot carries `disk_replaced: true` (once, like `notice`); the UI
     offers to overwrite or save under another name (`ui/src/merge.ts`,
     `saveGuarded`). Both choices call `save` again with `overwrite: true`:
     after "Enregistrer sous…" (Save As…), the system save dialog has already
     had the replacement of the chosen file confirmed, even when it is the
     same file as before. A response that still carries `disk_replaced`
     (the user targeted a path that was also replaced in the meantime) means
     "nothing was saved" for `saveGuarded`. Only a missing file
     (`StorageErrorKind::NotFound`) counts as unchanged — any other
     read error (permissions, network drive, file too large) is
     treated as a replacement, out of caution. Accepted limit: the
     check and the write (`write_atomic`) are not atomic
     together; on a synced drive, another writer that drops its
     version between the two is overwritten without merge.
  3. **On open**, two paths depending on whether the host can ask a question:
     - *Ouvrir…* (Open…), drag and drop onto the window: the read-only
       request `SameDeck { path }` (`Workspace::same_deck`, no side effect) looks for
       the non-blank tab that already holds this `deck_uid` under another
       path (a tab in templates mode does not count: it would refuse the
       merge). If there is one, the UI asks "Fusionner dans cet onglet" (Merge into
       this tab) or "Ouvrir à part" (Open separately) then, depending on the choice,
       sends `Request::Merge` to that tab or opens a new tab. The merge notice is
       shown in the merged tab; if it is in another window, in the active tab
       of the window from which the file was opened (`mergedNotice`), without
       bringing the other window to the front.
     - **Files handed over by the OS** (Finder, drag and drop onto the icon,
       command line — `open_files`): the host cannot ask a
       question on open, the file always opens separately; the notice
       `notice-same-deck-open` is set on the new tab and names
       the already open tab ("Cette présentation est déjà ouverte dans
       l'onglet « … »", i.e. "This presentation is already open in the tab '…'"),
       without merging anything.
- Under Tauri, this detour through a read-only request is needed
  because a workspace request that would change the structure returns
  the synced view instead of its response
  (`src-tauri/src/commands.rs:77-95`): `SameDeck` changes nothing, only the
  merge that follows (on the tab found) does.
- No conflict dialog: each conflict has a deterministic outcome;
  one fixes it afterwards, or restores the version from before the merge.

## History mode

`Mode::History { stash, history }`, on the pattern of `Mode::Templates`: the
current deck waits aside, the chosen version is shown read-only
(`history` carries the versions and the cut shown, `HistoryMode`).

- **Order**: topological sort of the changes (Lamport, then peer). Every
  prefix is a valid causal cut (`vv_to_frontiers` of the prefix's version
  vector, not only the causal history of the last
  change): each slider step is a consistent state, including a
  concurrent branch that is older in the order — a deviation from the initial
  design (cutting the causal history alone would have made it disappear from one
  step to the next). The time is displayed, never used for sorting
  (diverging clocks).
- **Editing session**: a peer's first commit in the order — its
  creation of the deck as well as its first edit — carries the session start
  marker; creation followed by edits of the same replica count as only
  one session.
- **Slider**: one step per edit (application, restore, silent
  commit): the intermediate `oku-part:` commits of an edit that
  moves slides have no step, the step of its final commit
  contains them. Markers at the start of each session and at each merge (an
  edit one of whose commits joins branches). ←/→: one step; ⇧←/⇧→: one
  session. No tooltip
  on the slider thumb: the banner label follows the slider during
  the drag (author, time, label) and stands in for it.
- **Banner**: "Version du … · auteur · libellé" (Version of … · author · label),
  **Restaurer** (Restore), **Revenir** (Go back).
- **Pile**: slides added, deleted, modified relative to the current
  deck — comparison of slides by `id` between the shown version and the
  current deck (`slide_changes`, both already read), not `doc.diff`: same
  result, without going up from Loro containers to slides.
- **Rendering**: the preview (`HistoryPreview`) is a fork made on entry;
  `checkout` from one step to another. During the drag, only the last
  requested step is rendered (pattern of `ui/src/latest.ts`); the SVG cache by
  fingerprint avoids redrawing unchanged pages. Measured (task 3,
  `oku-slides bench fixtures/roadmap.json --iterations 20`, 50 title
  changes, read + render per step): 5.3 ms on average, p50 5.3 ms — under
  the ~100 ms threshold, so rendering happens during the drag, without a
  low-resolution thumbnail.
- **Restore**: the version's deck (`deck_at`, the one the preview
  shows) is applied like any edit — author registration, then
  `apply(deck, "Restaurer la version du …")` (Restore the version of …): a single new
  undoable step (a single slider step, even across a reordering), whose
  result is exactly that version.
  Deviation from the initial design (`revert_to(frontiers)`): on the
  `slides` list (`LoroMovableList`, whose slides are moved by `mov`),
  `revert_to` lost slides or failed (`ContainerDeleted`) as soon as
  a move followed the target version. The reconciliation writes only
  the `deck` root: the `peers` map is not touched, the names of
  more recent sessions remain. All or nothing: Loro does not undo a
  commit, and `apply`, which commits before each `mov`, is not atomic;
  the reconciliation is therefore first tried under guard on a copy; a failure leaves the replica equal to its view, nothing
  pending. A partly read version is restored as read (as on
  open or merge), its unreadable parts logged. Restoring
  the current version makes no commit. `$date` is the timestamp of the
  restored version, in local time (UTC fallback), YYYY-MM-DD HH:MM.
- **Requests**: `HistoryOpen`, `HistorySeek { index }`, `HistoryRestore`,
  `HistoryClose`, `HistoryVersions` (the full list of versions is sent
  once; a step's snapshot carries only its index, its version and its
  slide changes).
- **Dictation**: stays mounted during history; dictated text follows the
  normal write path and receives the session's `InHistory` refusal,
  displayed (no silent loss, microphone visible and stoppable).

## Errors

| Where | Type | `kind` |
|---|---|---|
| `oku-slides-core::replica` | `ReplicaError` | `Corrupted`, `TooLarge`, `Write`; `Unrelated` with the merge |
| `oku-slides-core::sources` | `SourcesError` | existing ones; `MissingDeck` covers the absence of `deck.loro` |
| `oku-slides-session` | session error | `MergeUnrelated`, `HistoryNotOpen`, `InHistory` (history mode, plan 3) |

- **`MergeUnrelated`** reuses the existing message `replica-unrelated`
  ("these two presentations have no common origin"): no new
  Fluent key.
- **`HistoryNotOpen`**: a history request (`HistorySeek`,
  `HistoryRestore`, `HistoryClose`, `HistoryVersions`) outside history
  mode. **`InHistory`**: editing, saving or merging while
  the history is open.
- **Reading a version** (`HistoryPreview::deck_at`) can fail
  (`ReplicaError`); on the session side, this internal variant maps to the
  existing `kind` `InvalidDocument`, with no new `kind` or new Fluent
  key — same family of errors as "unreadable sources or deck".
  Going back to the version (`checkout`) and reading it are guarded
  (`catch_unwind`); after a panic, the preview is poisoned and every
  later read fails. A seek or a restore that fails
  leaves the history open, the deck set aside intact, images and fonts
  in the preview.
- **`DiskReplaced` is not an error.** Session errors reach
  the UI only as already translated text (`state.refuse` → `localize`),
  which cannot be told apart from another message; yet the UI must react
  differently (offer to overwrite or save as). The replaced
  disk therefore becomes the `disk_replaced: bool` flag of the `Snapshot`
  (`views.rs`), set once by `save_checked` and cleared on the next
  render — same pattern as `notice`.

Not errors: a subtree unreadable after merge (discarded,
reported by an `Issue`); a failed render during an apply (nothing is
committed, the error surfaces as today); a disk replaced by another
deck on save (`disk_replaced`, above). Messages in
Fluent: English first, then the existing languages.

## Tests (TDD)

The spike scenarios are the first red tests.

- **`replica_tests.rs`**: round trip; reconciliation without change =
  0 ops; empty = absent; prose always present (concurrent creation without
  loss); rich prose (bold ‖ typing, link that does not extend, `list` on
  the `\n`); chart atom; total read (unknown `kind`, inconsistent
  chart → `Issue`, rest of the deck read); different `deck_uid` →
  `Unrelated`.
- **`tests/convergence.rs`** (proptest): N peers, random ops, merges
  in random orders → all converge, the read never
  panics.
- **`tests/untrusted.rs`**: truncated, random, huge snapshot →
  `Corrupted` / `TooLarge`, never a panic.
- **`oku-slides-pdf`**: v2 round trip; v1 blob → `Ok(None)`; v3 →
  `UnsupportedVersion`.
- **`replica/history_tests.rs`** (core): order (creation, then one step per
  edit, label and author); a reordering of several slides
  (several *changes*) makes a single step, at the state after it; session start at a peer's first step,
  creation included (ruling 1); `deck_at` reads the state after each version;
  a merge marks its step and each prefix before it keeps both
  concurrent branches (not only the ancestors of the last change);
  restore = a new undoable commit (`undo` brings back the old title);
  restore keeps the authors of more recent sessions; restoring without
  a prior edit leaves no stray step without a label;
  restore returns exactly `deck_at(i)` (slide count included), under
  the given label, `undo` going back to the previous deck — after a slide
  move, after delete then move (each earlier version),
  after a round trip through the zip; after delete then swap of two
  slides, each earlier version is returned exactly; restoring
  across a slide swap undoes without loss and makes a single step; a poisoned
  preview makes the restore fail without leaving anything pending;
  `slide_changes` compares slides by `id` (added, deleted,
  modified).
- **Session**: failed render → replica intact; undo labels;
  resume without an undo stack; merge on open and on
  save (replaced file: `Saved::DiskReplaced`, flag
  `disk_replaced`); history mode — `tests/history.rs`: opening shows the
  latest version read-only and refuses an edit (`InHistory`);
  `seek` shows an earlier version, clamps beyond the last one, marks
  the slides added since the first version; closing returns to the current
  deck intact (theme, `dirty`, undo stack); restore = a
  new undoable commit, `undo` goes back to it; history requests
  outside the mode return `HistoryNotOpen`; in the mode, editing,
  saving, merging, opening or editing the templates return `InHistory`
  without changing anything; `history_versions` lists the labels in order;
  images and fonts survive open/seek/close and
  open/seek/restore without loss; the tab bar does not change
  during history; `history_tests.rs`: a seek to a version
  whose render fails (`Render`) keeps the shown version, images and
  fonts in the preview; a restore that fails (poisoned preview,
  `InvalidDocument`) keeps the history open and the deck set aside equal
  to the read-back of its replica.
- **UI (Bun, `history.test.ts`)**: `markers` places a marker at each session
  start and at each merge; `step` moves one step (clamped) or, with
  ⇧, to the previous or next session start; `pileMarks` marks the
  deleted and modified slides, not the added ones; `Seeker` displays only
  the response of the last requested step (outdated responses, arriving
  later, are ignored) and `cancel` makes a response still in flight
  no longer display afterwards (Go back, Restore).
- **Measure in the plan (task 3)**: cost of a render in history mode — read in
  task 3 (`oku-slides bench`, 50 title changes) at 5.3 ms on
  average, p50 5.3 ms, under the ~100 ms threshold: decision and detail in
  "History mode".

## Known limits

- Move ‖ delete: the element reappears.
- Edit ‖ delete: the edit is lost.
- Two concurrent adds with the same `id`: duplicate, reported by `validate`.
  UI ids are random (`deck.ts:newId`); to be checked for the
  ids produced by the voice agent.
- Moving an element from one group or slide to another = delete +
  insert: possible duplicate under concurrency (not tested in the spike).
- Text inserted at the edge of a mark: inherits it.
- The PDF never shrinks (history and assets append-only).
- **Privacy**: each shared PDF carries all the past content of the
  deck (deleted slides, elements and texts, old versions of
  texts, removed assets), the timestamp of each change and the author
  name of each peer that edited (`peers` map: `[author]
  name` setting, otherwise the system user name). Any recipient can
  read it.
  Accepted by design (no history flattening).
  Mitigation: export without sources, `oku-slides strip <entrée> <sortie>`
  (`oku_slides_pdf::strip_sources`), for now from the command line
  only.
- The undo stack is lost on relaunch (regression on t-145).
- Silent commits (canonicalization on load, saving
  templates) show up in the history as steps without a label.
- History mode: a step of another peer, concurrent with a
  reordering and sorted (Lamport, peer) between its intermediate
  commits, shows that reordering half done. The step of
  the edit itself, and all those after, are complete.
- v1 PDFs no longer open.
- Loro's decoder itself can overflow the stack on a forged
  snapshot, with a flat value nested tens of thousands of
  levels deep (a few KiB compressed): Loro decodes it recursively on
  first access and frees it on drop, not on `import`, outside our code,
  out of reach of `catch_unwind` and of the byte cap. Local denial of service
  when opening a forged PDF. Tracked as a separate fix
  (lead: access and free on a thread with a large stack, or an upstream
  Loro fix).
- Resume writes `deck.loro` then `document.json`: an abrupt stop between
  the two can leave a new replica next to an older `document.json`
  (stale `dirty` / `saved_hash`). The old format wrote
  only one file.
- A peer that undoes its first edit appears anonymous in the history
  (its only commit, the one that carried its name in `peers`, disappears).
- The time displayed in the history is that of the author's machine,
  never corrected: two peers with offset clocks may see times
  that do not match the order of the versions (the order itself stays
  correct, see "History mode").

The first three points are accepted regressions, to be recorded in an ADR.
