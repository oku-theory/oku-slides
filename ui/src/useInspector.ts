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
  /** The inspector column shows (a deck tab, kept open). */
  open: boolean;
  /** The data pane shows when the selection has data (a deck tab, kept open). */
  pane: boolean;
  /** Opens the data pane (a data trace, for `useTrace`). */
  onPane: () => void;
  /** Opens the inspector (any other trace, for `useTrace`). */
  onInspector: () => void;
  /** Closes the data pane. */
  closePane: () => void;
  /** The inspector's own props, besides the element's settings and the routing of actions. */
  props: Pick<InspectorProps, "data" | "onData" | "deck" | "offer" | "onClose">;
}

/**
 * The inspector and data pane of a deck tab (not a pack or theme tab, history or help), their open
 * state kept by the host (`keptUi`); `selected`'s own panel (`ownPanel`) gives the Data button;
 * `onPacks` opens the deck's packs, as the tools tray does; `offer`, the agent offer opened from its
 * chip (the agent's inspector slot), shows first; `onInspector` opens the inspector for it.
 */
export function useInspector({ snap, keptUi, selected, onPacks, offer }: {
  snap: Snapshot | null;
  keptUi: KeptUi;
  selected: Element | undefined;
  onPacks: () => void;
  offer?: ReactNode;
}): InspectorWiring {
  const deckMode = isDeckTab(snap);
  const openPane = () => keptUi.setDataPane(true);
  return {
    available: deckMode,
    open: deckMode && keptUi.inspector,
    pane: deckMode && keptUi.dataPane,
    onPane: openPane,
    onInspector: () => keptUi.setInspector(true),
    closePane: () => keptUi.setDataPane(false),
    props: {
      data: !!snap && ownPanel(snap.host_params, selected?.kind) !== null,
      onData: openPane,
      deck: { theme: snap?.theme_name ?? null, onPacks },
      offer,
      onClose: () => keptUi.setInspector(false),
    },
  };
}
