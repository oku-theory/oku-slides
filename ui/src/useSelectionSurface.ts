// A session surface about the selected element of a deck tab: read for the selection, each
// action sent to the element of the surface shown, never to the current selection.

import type { Element, Snapshot, Surface, TabApi } from "./api";
import type { QuickAction } from "./agent/quick";
import { isDeckTab } from "./tabKind";
import { type HostSettings, useSessionSurface } from "./useHostSettings";

/** The element a selection surface was read for. */
export interface ElementScope {
  slide: string;
  element: string;
}

/**
 * The surface `ids` of `selected` on `slide`, read in a deck tab only: not read-only, not in
 * history or a theme tab, with an element selected; `off` removes it (a pack tab, an element
 * the surface does not cover). Reread when the element, its slide's render or undo change.
 */
export function useSelectionSurface({ ids, api, snap, slide, selected, off, fetch, send, run, say }: {
  ids: readonly string[];
  api: TabApi;
  snap: Snapshot | null;
  slide: string | null;
  selected: Element | undefined;
  off: boolean;
  fetch: (api: TabApi, scope: ElementScope) => Promise<Surface | null>;
  /** Sends the shown surface's action to its element; `null`: nothing to send. */
  send: (api: TabApi, scope: ElementScope, action: QuickAction) => Promise<Snapshot> | null;
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  say: (m: string, error?: boolean) => void;
}): HostSettings {
  const hash = snap?.slides.find((s) => s.id === slide)?.hash;
  return useSessionSurface({
    ids,
    active: isDeckTab(snap) && !!slide && !!selected && !off,
    scope: { slide: slide ?? "", element: selected?.id ?? "" },
    // A refusal forces a reread (`sync`): the model holds the refused value.
    key: [slide, selected?.id, hash, snap?.undo_label, snap?.redo_label].join("|"),
    fetch: (scope) => fetch(api, scope),
    send: (scope, action) => send(api, scope, action),
    run,
    say,
  });
}
