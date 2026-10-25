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
import type { PackWidgetEditor } from "./usePackWidgetEditor";

/** Ce que l'agent lit de l'onglet. */
interface AgentWiringContext {
  tab: string;
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  selected: string | null;
  editing: ReturnType<typeof useFieldEditing>["editing"];
  editField: ReturnType<typeof useFieldEditing>["editField"];
  widgets: PackWidgetEditor;
  /** Un widget de l'atelier est ouvert : la scène est cachée. */
  widgetActive: boolean;
  importJob: ImportJob | null;
  showSlide: (id: string) => void;
  setSelected: (id: string | null) => void;
}

/** Où les fiches ancrées se posent : sur la slide affichée, nulle part si la scène est cachée. */
export function stageAnchors(view: SlideView | null, current: string | null, widgetActive: boolean) {
  return view && view.slide.id === current && !widgetActive
    ? { slide: view.slide.id, has: (a: Anchor) => anchorBox(view, a) !== null }
    : null;
}

/** L'état de l'agent pour cet onglet, et la poignée vers son panneau. */
export function useAgentWiring({
  tab, snap, view, current, selected, editing, editField, widgets, widgetActive, importJob, showSlide,
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
  // Sous un widget de l'atelier la scène est cachée : les fiches vont alors au fil.
  const shown = useMemo(() => stageAnchors(view, current, widgetActive), [view, current, widgetActive]);
  const route = useCallback((list: Surface[]) => routeSurfaces(list, shown), [shown]);
  const agentUiNow = useMemo(
    () => agentUi(current, selected, editField, widgets.activeName),
    [current, selected, editField, widgets.activeName],
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
  widgetActive: boolean;
  deckLook: string;
}

/** Le lien avec la fenêtre détachée de l'agent. Elle apprend d'ici l'onglet, et y écrit. */
export function useAgentLink({
  tab, agentDetached, editing, dictation, agentUiNow, agentStatus, dialogOpen, widgetActive,
  deckLook,
}: AgentLinkContext) {
  const [transport] = useState(() => linkTransport());
  const linkContext = useMemo<LinkContext>(
    () => ({
      editing: editing !== null,
      ui: agentUiNow,
      staged: agentStatus.sheet ? agentStatus.sheetId : null,
      keysOff: !!editing || dialogOpen || widgetActive,
      look: deckLook,
    }),
    [
      editing, agentUiNow, agentStatus.sheet, agentStatus.sheetId, dialogOpen,
      widgetActive, deckLook,
    ],
  );
  useMainLink({ tab, detached: !!agentDetached, context: linkContext, insert: dictation.insertText, transport });
}
