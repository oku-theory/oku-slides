# ADR-0013: Deck-only pack tabs linked to their owner deck

- **Status:** accepted
- **Date:** 2026-10-28
- **Tasks:** t-1200 (t-1224 to t-1232)
- **Spec:** [`2026-10-24-onglets-pack-theme-design.md`](../superpowers/specs/2026-10-24-onglets-pack-theme-design.md)
  (lot 5 and its implementation note); plan
  [`2026-10-27-onglets-pack-theme-lot5-deck-pack-tabs.md`](../superpowers/plans/2026-10-27-onglets-pack-theme-lot5-deck-pack-tabs.md)
- **Amends:** [ADR-0011](0011-tab-document-kinds-and-preview-decks.md) (a fifth document kind, the
  in-deck workshop removed), [ADR-0012](0012-theme-tabs.md) (pack states written without `host`)

## Context

A deck-only pack (an import's layouts) lives only inside one deck: the library has no current
version of it. Until lot 5 it was edited in a workshop inside its deck tab (`EditPack`,
`ClosePack`, `PackHost::Deck`), the last use of the in-deck workshop. Lot 5 opens it in its own
tab, like a library pack, but its versions must still land in that one deck: the pack tab and its
owner deck are two sessions, each behind its own lock (ADR-0001), that must agree on which version
the deck pins, across saves, undo, closes, quits and restarts.

## Decision

- **The link's identity is in the workspace layout.** `TabDocument::DeckPack { deck_tab, pack }`
  (by pack uuid, not namespace) is the link's only persisted form. The tab opens right after its
  owner, the owner as its opener, with no lease; opening it again focuses it.
- **The link's state lives in the owner, keyed by tab id** (`DeckLink`: title, pack, `deck_uid`,
  expected pin, accepted parent, last seq, pending). Every session acquisition of a deck tab hands
  it the layout's links with the layout generation (`Slot::ready_with_links`, the only way in). A
  link is never persisted in the session state: the owner's pin is the truth, so a resumed link is
  **re-seeded** from it (`expect` = the pin, parent = the pin or its manifest's parent, deck-scoped
  with no library current, else the tab is set aside with `notice-tab-resume-failed`).
- **The layout generation keeps stale copies out.** The workspace bumps it on every layout change;
  an owner ignores links copied under an older one, so a copy taken before a close never brings a
  closed tab's link back. A link's title follows its tab, so refusals name its current title.
- **Saves go through an outbox.** A save in the tab writes nothing to the library: it fills the
  outbox (the latest files, the base of the oldest unacknowledged save). The outbox becomes due at a
  `SavePack`, a flush (closing the tab or its owner, quitting) or `end_turn`, and only then gets a
  send with a process-wide seq (`DECK_PACK_SEQ`, from 1, never reset). One send is in flight per
  tab; a save meanwhile marks it `newer` and is sent after the answer. The owner answers `Applied`
  or `Delivered` (acknowledged: cleared), `Stale` (idle again until the next save, flush or turn
  end), `Busy` (**parked** until the owner's
  `end_turn` or history close pulls it) or an error (`Refused`: files kept, the error as the tab's
  notice, until the next save, flush or turn end). The tab is `unsent` until acknowledged: its
  `TabInfo.dirty` stays set and `ensure_closable` refuses it.
- **Forwards make the caller wait.** After a public call on the tab (`request`, `with_session`),
  the workspace takes the due send under the tab's lock, releases it, waits for the owner's lock
  like any request, releases it, then settles the answer under the tab's lock. Internal workspace
  calls use `with_quiet`, which never forwards, so a forward never runs under the opening lock.
- **Hosts learn the owner changed.** An applied forward queues its owner in `touched`; hosts drain
  it after every call (`drain_touched`: redraw the views, then tell each owner's agent). The dev
  server (`serve`) drains an agent turn's owners only at its next request.
- **The owner validates against what it holds**, never what the tab says: the link exists, the
  owner's `deck_uid` is the link's, the seq, its pin is the expected deck-scoped version with no
  library current, the send's base is the accepted parent, the manifest's scope, kind and
  namespace are the pin's, then the same pack or a fresh copy. A send carries no closure files: the
  closure resolves from the owner's sources and the library. The owner's sources plus the added
  files stay within its cumulative byte budget. Every change is decided on a clone of the deck, and
  refused (`DeckPackMismatch`, nothing written) if it would move any other pin already held; a newly
  required namespace that would move another link's pin answers `PackOpenInTab`. A refused send
  leaves the owner's widget session open (its next write still merges).
- **Copies keep the namespace.** "Save as copy" in the tab makes a deck-only copy, without parent,
  under the original's namespace; the owner removes the original's pins from both kinds and pins
  the copy there, and the workspace points the tab's document at the copy in the same forward,
  on `Applied` or on `Delivered` (a resend after a lost acknowledgement or a failed widget sync).
- **One undo step per editing session.** The owner applies under the merge key
  `pack-adopt:<tab>` (tab ids are never reused), so consecutive sends merge into one step.
- **The owner treats a linked pack as leased.** Every change of the pin of a linked pack, by id in
  either kind, is refused with `PackOpenInTab`: deck-mode widget writes, `Apply` ops, removing the
  pack, `merge_file` (tried on a copy first) and `history_restore`. Undo and redo are checked
  **after the fact**: a step that moved a linked pin is taken back by the opposite step and
  refused (`UndoInPackTab`); this covers steps from before the tab opened and `undo_agent_turn`.
  If the opposite step fails or does not restore the pins, `UndoCompensationFailed` is returned
  and the cause logged; the deck stays moved, so it is marked dirty, its widgets synced and rerendered. The take-back has two effects on merging: after a refused undo the next
  send opens a new step; after a refused redo the step underneath no longer merges.
- **Closing.** Closing a deck flushes its linked tabs first (outside the lock), then closes them
  with it, each passing `ensure_closable`; a discard discards them too. `CloseWindow` takes linked
  tabs in other windows. `LastWindow` (nothing closed, the host quits keeping everything) answers a
  close that takes linked deck-pack tabs and would empty the workspace, and, as before this lot, a
  `CloseWindow` of the only window; a plain last tab closes as before. A discarded refused close
  records its linked tabs so the quit does not flush them, and drops their unsent saves (their state
  written without `unforwarded`), so a resume never resends them. Closing the pack tab alone flushes it, unlinks it and activates its owner. `settle` flushes
  every awake deck-pack tab once, before setting `closed`.
- **Old workshops reopen on their deck.** A state at version 3, or version 5 with `host: deck`,
  resumes in deck mode from `deck/`, no lease taken. `notice-workshop-reopened-on-deck` shows when
  `pack/` is dirty or unreadable, when a library pack's pin moved off its base with no session
  version, or when a deck-only pack's edited version is not the one the deck pins.
- **`state.json`.** Pack states stay at version 5, now written **without `host`**, and gain
  `unforwarded` (the base of an unacknowledged save, absent otherwise). At resume an unforwarded
  save is rebuilt from the pack document (`pack/`): the files of its base, which resume requires,
  when that document still pins the base (an undo back to the parent leaves no session version but
  keeps that base), and made due; otherwise `notice-deck-pack-unforwarded-lost` is shown. The lot-4 build that wrote `host` was never released
  and is not a downgrade target; 6.0.0 refuses version 5. `workspace.json` stays at layout version 2.
- **Lock order:** the opening lock, then the workspace, then one session at a time (never two,
  never the workspace during a session call), then the draft and lease registries. **No `Drop`
  locks the workspace**: a drop runs wherever a guard happens to be released, possibly under the
  workspace or a session lock, and would deadlock. A pending link (`PendingLink`) is cancelled
  explicitly once the guard is released; its `Drop` is only a panic net using `try_lock`.
- **Both sides of a forward persist.** Acknowledge, park and each outbox fill write the tab's state;
  an applied send writes the owner's.

## Consequences

- A deck-only pack is edited with the same tab, pile and tools as a library pack; the in-deck
  workshop, `Request::{EditPack, ClosePack}`, `PackHost` and the MCP `close_pack` tool are gone.
- Recorded deviations from the plan: `forward` returns whether the owner applied the send (a close
  reports `DeckChanged` from it); `Closed::Gone` carries the removed slots, emptied once the lock is
  released. A deck-pack entry with an invalid owner is kept by `read_info` and set aside by the
  resume pre-pass, with its notice.
- Known limits:
  - an owner that absorbs an external change of the file on save keeps its links, and a moved pin
    refuses the tab's later sends; an owner that opened another document fails closed by `deck_uid`;
  - one step per editing session no longer holds when an owner edit lands between two sends;
  - an undo in the tab past a copy is refused by the owner;
  - a history point saved between two merged sends no longer renders the dropped intermediate
    version;
  - an agent task that dies before `end_turn` parks forwards, as it already blocks undo;
  - the dev server drains an agent turn's owners at its next request;
  - undo stops compare one pin per kind (`linked_pins` takes the first pin of the pack per kind);
  - the compensating undo's merge cut after a refused redo cannot be isolated by a test (any redo
    follows an undo that already cut the merge); it is covered only by a regression guard;
  - the byte budget counts the sent files, not closure versions the library adds (reopening counts
    them);
  - an agent turn's version in a pack tab is merged into a later save (predates this lot, t-1237);
  - a discarded `LastWindow` close drops the linked tabs' unsent saves but not their unsaved edits,
    which resume dirty and reach the owner at its next flush; under `serve` (409, still running)
    the tabs stay open without their dropped saves.
