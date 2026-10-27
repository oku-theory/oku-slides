# ADR-0012: Theme tabs on a specimen deck, the theme draft out of pack mode

- **Status:** accepted
- **Date:** 2026-10-27
- **Tasks:** t-1156 (t-1218 to t-1222)
- **Spec:** [`2026-10-24-onglets-pack-theme-design.md`](../superpowers/specs/2026-10-24-onglets-pack-theme-design.md)
  (lot 4 and its implementation note); plan
  [`2026-10-26-onglets-pack-theme-lot4-theme-tabs.md`](../superpowers/plans/2026-10-26-onglets-pack-theme-lot4-theme-tabs.md)
- **Amends:** [ADR-0011](0011-tab-document-kinds-and-preview-decks.md) (a fourth document kind,
  editor state version 5, theme leases)

## Context

Until lot 4 a theme was edited in the "Thème" sub-tab of pack mode, on the pack's working deck:
editing a widget pack also edited the deck's theme, and saving a pack could save a theme. Lot 4
opens a library theme in its own window tab, as lot 3 did for library packs. That tab needs slides
to show the theme on, a persisted state, and the same one-editor rule as packs.

## Decision

- **A theme tab works on a specimen deck.** `TabDocument::Theme` and `Mode::Theme` hold the draft
  (`ThemeTab`) beside a read-only working deck: one slide per layout of the built-in Essentiel
  (`layout-<name>`, its localized examples), pinning built-in `base` and `essentiel` only, no
  theme, the draft as its tokens. The opener's slides were set aside: they would tie a library
  document to one deck, bring that deck's assets, fonts and packs into the tab, and change with it.
  A request or op a kind does not take is refused with `WrongDocument { kind }`; in a theme tab
  only token ops apply, and the filter sits in `Session::apply`, which the agent calls too.
- **The theme draft leaves pack mode**, the in-deck workshop included. A pack working deck keeps
  its deck's theme, pins and overrides and refuses token ops; `save_pack` no longer saves a theme.
- **The base is fixed per theme tab.** A theme tab never changes which theme it edits
  (`SetEditedTheme` and the panel's base picker are removed); to start from another theme the user
  opens that theme's tab. A save made as a new theme moves the tab to the new id.
- **Saving follows the library's draft rule** (`draft_write` then `save_draft` under one
  `draft_lock`): an unadopted draft is replaced in place, an adopted one gets a child, a built-in
  or a "save as new" makes a new theme whose lease the tab takes first. Saving pins nothing in any
  deck: decks see the new current version through "Packs du deck".
- **Freezing.** Opening a theme tab on a user theme, or resuming one that has not written yet,
  freezes its base from the library's bytes; built-in and received versions are already frozen.
- **One editor per theme.** `LeaseScope` gains a `Themes` scope in the shared `PACK_TABS` registry,
  so packs and themes follow one lock order and one opening lock (their ids never collide). A theme
  tab without the lease is read-only (`ThemeOpenInTab`), and `edit_theme` in deck mode refuses a
  leased theme. Opening is serialized per theme, and `OpenTheme` focuses the tab already editing it.
- **Persistence is `state.json` only**: a theme tab writes no `deck/` or `pack/`; its specimen deck
  is rebuilt on resume after the base, parent, `extends` and draft tokens are checked against the
  library. Every pack-mode state (both hosts) and theme state is written at
  `EDITOR_STATE_VERSION` 5, which 6.0.0 refuses rather than misreads (6.0.0 would read a pack
  working deck's tokens as a theme draft and save a corrupt version). Version-3 workshops and
  version-4 pack tabs are still read, their legacy theme draft dropped with a notice when it
  differed from its base. `workspace.json` stays layout version 2.

## Consequences

- Downgrade cost: after a downgrade to 6.0.0, pack and theme states written at version 5 are set
  aside, a deck tab left in an in-deck workshop included (its deck file, if any, is untouched).
- At version 5, a hand-forged pack-tab state with `host: deck` cannot be told apart from an
  in-deck workshop: it would resume the preview copy as a user deck (needs data-directory write
  access).
- If the app dies between writing `state.json` (with the new id after a "save as new") and writing
  `workspace.json`, the tab is kept aside on resume; nothing is lost.
- Known limits:
  - leases stay process-local;
  - the holder label in an "…open in a tab" refusal is the theme's name when the lease was taken,
    stale after a rename, as in pack mode;
  - the specimen deck's token schema is narrower for the agent than pack mode's was: global tokens
    plus `base` and `essentiel`; `component.<ns>` tokens of other packs are kept but not shown;
  - a new theme saved in a theme tab is not offered to the deck that opened it (t-1207);
  - a theme tab opened by the agent waits for lot 7 (t-1191).
