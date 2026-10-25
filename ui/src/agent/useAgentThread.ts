// Le fil de l'agent de l'onglet dans l'interface et son rappel.

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentState, McpEvent } from "../api";

/** L'état de l'agent d'après ses évènements : seul `status` le change. */
export function agentStateReducer(cur: AgentState, e: AgentEvent): AgentState {
  return e.kind === "status" ? e.state : cur;
}

/** Ce que l'interface lit du fil de l'agent. */
export interface AgentThreadState {
  state: AgentState;
  /**
   * Un agent répond : le dernier rappel du fil (`resync`) a réussi. Faux avant le premier, et
   * sans pile d'outils (le rappel échoue) : rien ne peut lui être envoyé.
   */
  ready: boolean;
  onAgentEvent: (e: AgentEvent) => void;
  /** Abonnement prêt ; stable d'un rendu à l'autre (lu une fois par `subscribeHost`). */
  onReady: () => void;
}

/**
 * État de l'agent et rappel du fil de l'onglet.
 *
 * Le fil est demandé une fois l'abonnement prêt et l'instantané chargé (surfaces vidées
 * d'abord) ; une reconnexion le redemande.
 */
export function useAgentThread(
  agent: { resync: () => Promise<void> },
  loaded: boolean,
  clearSurfaces: (e: McpEvent) => void,
): AgentThreadState {
  const [armed, setArmed] = useState(false);
  const armedRef = useRef(false);
  const [available, setAvailable] = useState(false);
  // Le rappel du fil dit aussi si un agent répond.
  const resync = useCallback(
    () =>
      agent.resync().then(
        () => setAvailable(true),
        (err) => {
          setAvailable(false);
          console.warn("resync", err);
        },
      ),
    [agent],
  );
  const [state, onAgentEvent] = useReducer(agentStateReducer, "idle" as AgentState);
  useEffect(() => {
    if (!armed || !loaded) return;
    clearSurfaces({ kind: "surfaces", list: [] });
    void resync();
  }, [resync, armed, loaded, clearSurfaces]);
  // Ne lit que des références : `subscribeHost` garde la première version.
  const onReady = useCallback(() => {
    if (!armedRef.current) {
      armedRef.current = true;
      setArmed(true);
      return;
    }
    void resync();
  }, [resync]);
  return { state, ready: available, onAgentEvent, onReady };
}
