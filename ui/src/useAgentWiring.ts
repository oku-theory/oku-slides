// L'agent vu d'un onglet. Il y tient son état, ses emplacements, son focus, et le lien
// avec sa fenêtre.

import { useCallback, useMemo, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Anchor, SlideView, Snapshot, Surface } from "./api";
import { agentApi } from "./api";
import { IDLE_AGENT, type AgentHandle, type AgentStatusView, type ThreadMode } from "./agent/AgentPanel";
import { linkTransport, type LinkContext } from "./agent/link";
import { createSlot } from "./agent/slot";
import { routeSurfaces } from "./agent/surfaces";
import { agentUi } from "./agent/ui";
import { useAgentAvailability } from "./agent/useAgentAvailability";
import { useAgentFocus } from "./agent/useAgentFocus";
import { useMainLink } from "./agent/useMainLink";
import type { ImportJob } from "./useImportJob";
import { anchorBox } from "./stageFields";
import type { useDictation } from "./useDictation";
import type { useFieldEditing } from "./useFieldEditing";

/** Ce que l'agent lit de l'onglet. */
interface AgentWiringContext {
  tab: string;
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  selected: string | null;
  editing: ReturnType<typeof useFieldEditing>["editing"];
  editField: ReturnType<typeof useFieldEditing>["editField"];
  /** A pack tab's selected item; `null` elsewhere or when none is selected. */
  packItem: string | null;
  /** A pack tab: no stage is shown. */
  packTab: boolean;
  importJob: ImportJob | null;
  showSlide: (id: string) => void;
  setSelected: (id: string | null) => void;
}

/** Where anchored cards land: on the slide shown, nowhere in a pack tab (no stage). */
export function stageAnchors(view: SlideView | null, current: string | null, packTab: boolean) {
  return view && view.slide.id === current && !packTab
    ? { slide: view.slide.id, has: (a: Anchor) => anchorBox(view, a) !== null }
    : null;
}

/** L'état de l'agent pour cet onglet, et la poignée vers son panneau. */
export function useAgentWiring({
  tab, snap, view, current, selected, editing, editField, packItem, packTab, importJob, showSlide,
  setSelected,
}: AgentWiringContext) {
  // --- Agent ------------------------------------------------------------------
  // Son état vit dans `AgentPanel` : un morceau de réponse ne re-rend pas `App`. `App` n'en reçoit
  // que des valeurs simples (`agentStatus`) et une poignée (consigne, action, arrêt du tour).
  const [agent] = useState(() => agentApi(tab));
  const [agentSlots] = useState(() => ({ sheet: createSlot(), drawer: createSlot(), modal: createSlot() }));
  const [agentStatus, setAgentStatus] = useState(IDLE_AGENT);
  const agentAvailability = useAgentAvailability(agent, importJob !== null);
  // Les fiches ancrées se posent sous leur élément, sur la slide affichée.
  // A pack tab shows no stage: the cards go to the thread.
  const shown = useMemo(() => stageAnchors(view, current, packTab), [view, current, packTab]);
  const route = useCallback((list: Surface[]) => routeSurfaces(list, shown), [shown]);
  const agentUiNow = useMemo(
    () => agentUi(current, selected, editField, packItem),
    [current, selected, editField, packItem],
  );
  const loaded = snap !== null;
  const agentMode = useMemo<ThreadMode>(() => ({ kind: "drive", loaded }), [loaded]);
  const agentHandle = useRef<AgentHandle>(null);
  const promptAgent = (text: string, images: string[]) => agentHandle.current?.prompt(text, images) ?? Promise.resolve(false);
  const cancelAgent = () => agentHandle.current?.cancel();
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => agentHandle.current?.onSurfaceAction(s, a);
  // Un focus de l'agent pendant l'édition d'un texte attend la fin de l'édition.
  const onAgentFocus = useAgentFocus({ slides: snap?.slides, view, current, editing, show: showSlide, select: setSelected });
  return {
    agent, agentSlots, agentStatus, setAgentStatus, agentAvailability, route,
    agentUiNow, agentMode, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
  };
}

/** Ce que la fenêtre de l'agent apprend de l'onglet. */
interface AgentLinkContext {
  tab: string;
  agentDetached?: boolean;
  editing: ReturnType<typeof useFieldEditing>["editing"];
  dictation: ReturnType<typeof useDictation>;
  agentUiNow: ReturnType<typeof agentUi>;
  agentStatus: AgentStatusView;
  dialogOpen: boolean;
  /** A pack tab: its editor takes the keys, the agent's quick keys stay off. */
  packTab: boolean;
  deckLook: string;
}

/** Le lien avec la fenêtre détachée de l'agent. Elle apprend d'ici l'onglet, et y écrit. */
export function useAgentLink({
  tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, packTab,
  deckLook,
}: AgentLinkContext) {
  const [transport] = useState(() => linkTransport());
  const linkContext = useMemo<LinkContext>(
    () => ({
      editing: editing !== null,
      ui: agentUiNow,
      staged: agentStatus.sheet ? agentStatus.sheetId : null,
      keysOff: !!editing || dialogOpen || packTab,
      look: deckLook,
    }),
    [
      editing, agentUiNow, agentStatus.sheet, agentStatus.sheetId, dialogOpen,
      packTab, deckLook,
    ],
  );
  useMainLink({ tab, detached: !!agentDetached, context: linkContext, insert: dictation.insertText, transport });
}
