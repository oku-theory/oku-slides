// A deck tab's inspector and data pane: whether they show, how traces open them, the deck block.

import type { ReactNode } from "react";
import type { Element, Snapshot } from "./api";
import { ownPanel } from "./hostParams";
import type { InspectorProps } from "./Inspector";
import type { KeptUi } from "./useKeptUi";
import { isDeckTab } from "./tabKind";

/** The App's wiring of the inspector and the data pane. */
export interface InspectorWiring {
  /** The tab can show the inspector (`isDeckTab`): an agent offer opens there, else under its chip. */
  available: boolean;
  /** The inspector is kept open in a deck tab (its side panel shows it unless the packs do). */
  open: boolean;
  /** The data pane shows when the selection has data (a deck tab, kept open). */
  pane: boolean;
  /** Opens the data pane (a data trace, for `useTrace`). */
  onPane: () => void;
  /** Opens the inspector tab, closing the packs tab (any other trace, the side panel). */
  onInspector: () => void;
  /** Folds the side panel: the inspector and the packs tab close. */
  onFold: () => void;
  /** Closes the data pane. */
  closePane: () => void;
  /** The inspector's own props, besides the element's settings and the routing of actions. */
  props: Pick<InspectorProps, "data" | "onData" | "theme" | "offer">;
}

/**
 * The inspector and data pane of a deck tab (not a pack or theme tab, history or help), their open
 * state kept by the host (`keptUi`); `selected`'s own panel (`ownPanel`) gives the Data button;
 * `closePacks` closes the side panel's packs tab, which shows over the inspector; `offer`, the agent
 * offer opened from its chip (the agent's inspector slot), shows first; `onInspector` opens the
 * inspector for it.
 */
export function useInspector({ snap, keptUi, selected, closePacks, offer }: {
  snap: Snapshot | null;
  keptUi: KeptUi;
  selected: Element | undefined;
  closePacks: () => void;
  offer?: ReactNode;
}): InspectorWiring {
  const deckMode = isDeckTab(snap);
  const openPane = () => keptUi.setDataPane(true);
  return {
    available: deckMode,
    open: deckMode && keptUi.inspector,
    pane: deckMode && keptUi.dataPane,
    onPane: openPane,
    onInspector: () => {
      closePacks();
      keptUi.setInspector(true);
    },
    onFold: () => {
      closePacks();
      keptUi.setInspector(false);
    },
    closePane: () => keptUi.setDataPane(false),
    props: {
      data: !!snap && ownPanel(snap.host_params, selected?.kind) !== null,
      onData: openPane,
      theme: snap?.theme_name ?? null,
      offer,
    },
  };
}
