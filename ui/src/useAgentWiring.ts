// L'agent vu d'un onglet. Il y tient son état, ses emplacements, sa portée, son focus, et le lien
// avec sa fenêtre.

import { useCallback, useMemo, useRef, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Anchor, SlideView, Snapshot, Surface } from "./api";
import { agentApi } from "./api";
import { IDLE_AGENT, type AgentHandle, type AgentStatusView, type ScopeSource } from "./agent/AgentPanel";
import { linkTransport, type LinkContext } from "./agent/link";
import { createSlot } from "./agent/slot";
import { routeSurfaces } from "./agent/surfaces";
import { agentUi } from "./agent/ui";
import { useAgentAvailability } from "./agent/useAgentAvailability";
import { useAgentFocus } from "./agent/useAgentFocus";
import { useAgentPicker } from "./agent/useAgentPicker";
import { useMainLink } from "./agent/useMainLink";
import type { ImportJob } from "./useImportJob";
import { anchorBox } from "./stageFields";
import type { useDictation } from "./useDictation";
import type { useFieldEditing } from "./useFieldEditing";
import type { TemplateWidgets } from "./useTemplateWidgets";

/** Ce que l'agent lit de l'onglet. */
interface AgentWiringContext {
  tab: string;
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  selected: string | null;
  editing: ReturnType<typeof useFieldEditing>["editing"];
  editField: ReturnType<typeof useFieldEditing>["editField"];
  widgets: TemplateWidgets;
  importJob: ImportJob | null;
  say: (text: string, error?: boolean) => void;
  showSlide: (id: string) => void;
  setSelected: (id: string | null) => void;
}

/** L'état de l'agent pour cet onglet, et la poignée vers son panneau. */
export function useAgentWiring({
  tab, snap, view, current, selected, editing, editField, widgets, importJob, say, showSlide,
  setSelected,
}: AgentWiringContext) {
  // --- Agent ------------------------------------------------------------------
  // Son état vit dans `AgentPanel` : un morceau de réponse ne re-rend pas `App`. `App` n'en reçoit
  // que des valeurs simples (`agentStatus`) et une poignée (consigne, action, arrêt du tour).
  const [agent] = useState(() => agentApi(tab));
  const [agentSlots] = useState(() => ({ sheet: createSlot(), drawer: createSlot(), modal: createSlot() }));
  const [agentStatus, setAgentStatus] = useState(IDLE_AGENT);
  // Le panneau des agents, dans le menu des réglages de la dictée.
  const agentPicker = useAgentPicker(agent, (m) => say(m, true));
  const agentAvailability = useAgentAvailability(agent, importJob !== null);
  // Les fiches ancrées se posent sous leur élément, sur la slide affichée.
  const shown = useMemo(
    () =>
      view && view.slide.id === current
        ? { slide: view.slide.id, has: (a: Anchor) => anchorBox(view, a) !== null }
        : null,
    [view, current],
  );
  const route = useCallback((list: Surface[]) => routeSurfaces(list, shown), [shown]);
  const agentUiNow = useMemo(
    () => agentUi(current, selected, editField, widgets.activeName),
    [current, selected, editField, widgets.activeName],
  );
  const templates = !!snap?.editing;
  const inHistory = !!snap?.history;
  const pack = snap?.editing?.set ?? null;
  const scopeSource = useMemo<ScopeSource>(
    () => ({ kind: "drive", view: { templates, history: inHistory, pack, widget: widgets.activeName } }),
    [templates, inHistory, pack, widgets.activeName],
  );
  const agentHandle = useRef<AgentHandle>(null);
  const promptAgent = (text: string) => agentHandle.current?.prompt(text) ?? Promise.resolve(false);
  const cancelAgent = () => agentHandle.current?.cancel();
  const onSurfaceAction = (s: Surface, a: A2uiClientAction) => agentHandle.current?.onSurfaceAction(s, a);
  // Un focus de l'agent pendant l'édition d'un texte attend la fin de l'édition.
  const onAgentFocus = useAgentFocus({ slides: snap?.slides, view, current, editing, show: showSlide, select: setSelected });
  return {
    agent, agentSlots, agentStatus, setAgentStatus, agentPicker, agentAvailability, route,
    agentUiNow, scopeSource, agentHandle, promptAgent, cancelAgent, onSurfaceAction, onAgentFocus,
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
