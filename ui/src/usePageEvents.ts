// The page events the toolbar, the menu and the agent share: present the deck, open the history.

import { useRef, useState } from "react";
import type { RefObject } from "react";
import type { Snapshot, TabApi } from "./api";
import type { Autosave } from "./autosave";
import type { PresentMode } from "./present";
import type { StageHandle } from "./Stage";
import type { FieldKey } from "./stageFields";

/** What the page events need from the tab. */
export interface PageEventsContext {
  api: TabApi;
  /** The last accepted snapshot, current before the render that follows. */
  snapRef: RefObject<Snapshot | null>;
  /** The stage handle: commits the open text edit. */
  stage: RefObject<StageHandle | null>;
  autosave: Pick<Autosave, "flush">;
  /** Applies a tab request; `null`: refused (already said) or abandoned. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  setSelected: (id: string | null) => void;
  setEditing: (key: FieldKey | null) => void;
}

/** Presenting the deck and opening the history, for App and the agent's page events. */
export function usePageEvents({ api, snapRef, stage, autosave, run, setSelected, setEditing }: PageEventsContext) {
  // --- Presenting: the slide alone, full screen or full window ---------------
  const [presenting, setPresenting] = useState<PresentMode | null>(null);
  /** Presents the deck from the shown slide; the open text edit is committed first. Reads the
   *  last snapshot, not the render's: the agent may present right after a change. */
  const present = (mode: PresentMode) => {
    const s = snapRef.current;
    if (!s?.can_present) return;
    stage.current?.commitEditing();
    setSelected(null);
    setEditing(null);
    setPresenting(mode);
  };

  /** Opens the history (toolbar button or the agent's `history` tool), one opening at a time.
   *  The open text edit is committed first, then pending changes go to the file: the history
   *  refuses writes, and the deck set aside goes back to the file only when it closes. */
  const openingHistory = useRef(false);
  const openHistory = async () => {
    if (openingHistory.current) return;
    openingHistory.current = true;
    try {
      stage.current?.commitEditing();
      await autosave.flush();
      if (await run(api.historyOpen())) {
        setSelected(null);
        setEditing(null);
      }
    } finally {
      openingHistory.current = false;
    }
  };

  return { presenting, setPresenting, present, openHistory };
}
