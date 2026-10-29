// The kind of a tab: what its document is, for the bar and for the panels it mounts.

import type { Snapshot, TabDocument } from "./api";

/** What a tab holds: a deck, a pack, a theme, the app's help, or a deck's history. */
export type TabKind = "deck" | "pack" | "theme" | "help" | "history";

/** The kind of the tab a snapshot describes: history, then help, then theme, then pack, else deck. */
export function tabKind(snap: Snapshot): TabKind {
  if (snap.history) return "history";
  if (snap.read_only) return "help";
  if (snap.theming) return "theme";
  if (snap.editing) return "pack";
  return "deck";
}

/** The kind a tab's document gives it in the bar; a deck-only pack is a pack, an unknown kind a deck. */
export function documentKind(doc: TabDocument | undefined): "deck" | "pack" | "theme" | "help" {
  switch (doc?.kind) {
    case "pack":
    case "deck_pack":
      return "pack";
    case "theme":
      return "theme";
    case "help":
      return "help";
    default:
      return "deck";
  }
}
