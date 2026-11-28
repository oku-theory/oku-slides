// The deck's data in a deck tab: the list in the side panel's Data tab (`deck-data`) and, opened
// from it or from a view's data name, one data's rows in the data pane (`data-sheet`). Opening
// the pane or selecting a view is done here; every other action goes to the session.

import { useEffect, useState } from "react";
import type { Snapshot, Surface, TabApi } from "./api";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { QuickAction } from "./agent/quick";
import { type DataRoute, isPanelAction, routeDataAction } from "./dataActions";
import { snapNumber } from "./snapNumber";
import { isDeckTab } from "./tabKind";
import { type HostSettings, useSessionSurface } from "./useHostSettings";

/** Surface of the deck's data list. */
export const DECK_DATA = "deck-data";
/** Surface of one data's rows. */
export const DATA_SHEET = "data-sheet";

/** The data and first row the data pane shows. */
interface SheetScope {
  data: string;
  offset: number;
}

/** What `useDeckData` gives App. */
export interface DeckData {
  list: HostSettings;
  sheet: HostSettings;
  /** A data is open in the pane: it shows the sheet instead of the selection's data. */
  sheetOpen: boolean;
  /** Opens `data` in the pane (a view's data name). */
  openSheet: (data: string) => void;
  /** Takes an action of the list or the sheet; `true` if it was theirs. */
  onAction: (s: Surface, a: A2uiClientAction) => boolean;
}

/**
 * The list, read while `listOpen`, and the sheet, read while `paneOpen` with a data open; both
 * reread on each new snapshot. `onPane` opens the data pane; `select` shows a view's element.
 * A new selection (`selected`) or a closed pane gives the pane back to the selection's data.
 */
export function useDeckData({ api, snap, listOpen, paneOpen, selected, onPane, select, run, say }: {
  api: TabApi;
  snap: Snapshot | null;
  listOpen: boolean;
  paneOpen: boolean;
  selected: string | null;
  onPane: () => void;
  select: (slide: string, element: string) => void;
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  say: (m: string, error?: boolean) => void;
}): DeckData {
  const [open, setOpen] = useState<SheetScope | null>(null);
  const rev = snapNumber(snap);
  const deck = isDeckTab(snap);
  useEffect(() => setOpen(null), [selected]);
  useEffect(() => {
    if (!paneOpen) setOpen(null);
  }, [paneOpen]);
  const openSheet = (data: string, offset = 0) => {
    setOpen({ data, offset });
    onPane();
  };
  const take = (route: DataRoute): Promise<Snapshot> | null => {
    if (route.to === "session") return api.dataAction(route.action);
    if (route.to === "sheet") openSheet(route.data, route.offset);
    else select(route.slide, route.element);
    return null;
  };
  const send = (_: unknown, { name, context }: QuickAction) =>
    name === "data" && isPanelAction(context.edit) ? take(routeDataAction(context.edit)) : null;
  const list = useSessionSurface({
    ids: [DECK_DATA],
    active: deck && listOpen,
    scope: null,
    key: String(rev),
    fetch: () => api.dataList(),
    send,
    run,
    say,
  });
  const sheet = useSessionSurface({
    ids: [DATA_SHEET],
    active: deck && paneOpen && open !== null,
    scope: open ?? { data: "", offset: 0 },
    key: [open?.data, open?.offset, rev].join("|"),
    // A data removed since (an undo, the agent, a peer) has no sheet: the pane goes back to
    // the selection's data rather than stay empty.
    fetch: (scope) =>
      api.dataSheet(scope.data, scope.offset).then((read) => {
        if (!read) setOpen((cur) => (cur?.data === scope.data ? null : cur));
        return read;
      }),
    send,
    run,
    say,
  });
  return {
    list,
    sheet,
    sheetOpen: open !== null,
    openSheet: (data) => openSheet(data),
    onAction: (s, a) => list.onAction(s, a) || sheet.onAction(s, a),
  };
}
