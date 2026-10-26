# ADR-0011: Tab document kinds, pack tabs beside a preview deck, pack leases

- **Status:** accepted
- **Date:** 2026-10-25
- **Tasks:** t-1155 (t-1192 to t-1197)
- **Spec:** [`2026-10-24-onglets-pack-theme-design.md`](../superpowers/specs/2026-10-24-onglets-pack-theme-design.md)
  (lot 3 and its implementation note); plan
  [`2026-10-25-onglets-pack-theme-lot3-pack-tabs.md`](../superpowers/plans/2026-10-25-onglets-pack-theme-lot3-pack-tabs.md)
- **Amends:** [ADR-0001](0001-espace-de-travail-session-par-onglet.md) (a tab's info says what it edits)

## Context

Until lot 3 a library pack was edited in its deck tab's workshop, which set the deck aside. Lot 3
opens a library pack in its own window tab. That tab needs a deck to preview the pack with, a way to
say what it edits, a persisted state the previous release can still read, and a rule that keeps two
editors from writing the same pack. A deck-only pack (an import's hi-fi pack) keeps its in-deck
workshop until lot 5.

## Decision

- **A tab says what it edits.** `TabInfo.document` is a `TabDocument` (`Deck`, `Help`, `Pack`). A
  cached document is a display hint: it never grants anything, it only finds candidates that are
  re-checked on the live session, or tells the UI that the live state must be read.
- **Formats stay readable by the previous release.** `workspace.json` stays version 2: each entry
  carries `document` and the legacy keys derived from it. `state.json` stays version 3 for deck,
  help and in-deck workshop states; only a pack tab's state is version 4. A downgrade sets aside
  only pack tabs.
- **A pack tab edits beside a preview deck.** The preview is a never-saved copy of the opener deck
  (its deck value, the pack and theme versions its pins reach, its fonts; no file, identity, assets
  or registry). It never adopts, and it is persisted and resumed with the tab, so a copy of the
  opener rests under app support. A smaller ephemeral preview was weighed and set aside: it would
  lose the opener's tokens and fonts after a restart.
- **Freezing.** Versions freeze when a real deck adopts them; a pack tab freezes its base, from the
  library's own bytes, once it holds the lease. Another deck may still adopt a version a pack tab
  wrote; the tab's next save becomes a child.
- **The opener is memory only.** Closing a pack tab that wrote a version offers that exact version to
  the deck that opened it; a refocused tab's opener may be offered a version previewed with another
  deck. Save-as-new copies and theme versions are not offered (t-1207).
- **One editor per pack: the lease.** `PACK_TABS` holds one lease per library pack, keyed by the
  editor's own library directories, taken after `DRAFTS`, by awake editors only (no dormant lease),
  with the holder's label frozen when taken. While held, no other tab writes the pack in deck mode,
  makes another version current, deletes it, enters its workshop or opens a second editor of it.
  The two pack-mode library writers take the lease as a witness and re-check that the pack is still
  listed. An editor without the lease is read-only: it retries at each editing request, never
  autosaves meanwhile, and once the holder closes, its next write makes its own version current
  (last writer). A lease refused at resume restores the tab read-only instead of setting work aside.
- **Orphaned tabs.** A library pack deleted while its editor was dormant or awake — or deleted and
  recreated with other versions — leaves that pack tab or workshop read-only for the rest of the
  session; asking the agent to save it as a new pack keeps its work. A deck-only workshop is never
  orphaned.
- **Opening is serialized per pack** by an opening lock, so two opens of one pack give one tab. The
  agent's gate is asked before any change and never blocks (lot 7 asks the user before the opening
  lock); an agent opens at most one new pack tab per turn, and a turn that opened one writes no
  widget in its own tab.
- **Badges are lazy:** recounted at a deck's next snapshot from a process-wide library write count,
  without adopting.

## Consequences

- Resume validates a pack tab's persisted state against the library and its own `pack/` and
  `deck/` (id, kind, scope, base, parent, session version); a forged state directory can at worst
  bring its own pack bytes into the user's next save, or replace an unfrozen version of the pack it
  claims, which needs write access to the data directory anyway.
- Themes are not leased in this lot (lot 4).
- What the lease does not cover, accepted:
  - it is process-local (`just serve` and the app on one data directory are not guarded) and its
    scope lexical (a symlinked spelling of a library gives a second scope);
  - `receive_version` still adds non-current versions of a leased pack when a received deck opens,
    and can bring a deleted pack back (an orphaned tab stays read-only);
  - the undo path of a forged `pack/` can add a non-current version;
  - in deck mode a widget write can still recreate a deleted pack (today's behavior, t-1157);
  - a holder label goes stale after a rename.
- Known limits:
  - a discard close of a tab whose session is stuck waits for the stuck call and keeps its pack
    leased until then (t-1206); a write already past the witness lands before that close returns;
  - a dirty pack tab whose save keeps failing for a real fault stays open until the fault clears or
    the app quits (t-1206);
  - typing refused in a read-only tab is lost when it closes, with or without a dialog (t-1157);
  - the agent reads `agent-error-pack-id` for an orphaned tab, its `Focus` moves the focus without
    consuming a claim and picks the first candidate in window order (t-1191);
  - a forged cached `Pack` document over a deck state with a failing render offers the pack discard;
    a tab whose session panicked keeps its lease until closed with discard; a forged dirty `pack/`
    autosaves once (all need data-directory write access);
  - plain-word spoofing within 60 characters stays possible in the two native confirms, which skip
    the 500 ms arming until lot 6 (t-1157).
- `PackTabNotFresh` is an internal error (a precondition of building a pack tab, never shown for a
  user action).
