// La portée de l'agent dans l'interface et le rappel de son fil.

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentScope, McpEvent } from "../api";
import { t } from "../i18n";
import { type AgentStatus, type ScopeView, agentScope, busyElsewhere, scopeKey, scopePlace } from "./scope";

export const IDLE: AgentStatus = { state: "idle" };
/** L'état de l'agent d'après ses évènements : seul `status` le change. */
export function agentStatusReducer(cur: AgentStatus, e: AgentEvent): AgentStatus {
  return e.kind === "status" ? { state: e.state, scope: e.scope } : cur;
}

const NO_VIEW: ScopeView = { templates: false, history: false, pack: null, widget: null };

/** Ce que l'interface lit de la portée de l'agent. */
export interface AgentScopeState {
  scope: AgentScope;
  status: AgentStatus;
  /** La portée d'un tour en cours ailleurs, sinon `null`. */
  elsewhere: AgentScope | null;
  /** « L'agent répond dans … », sinon `null`. */
  notice: string | null;
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
 * Portée, état de l'agent, note « répond ailleurs » et rappel du fil.
 *
 * Le fil est demandé une fois l'abonnement prêt et la vue connue, puis à chaque
 * changement de portée (surfaces vidées d'abord) ; une reconnexion le redemande.
 */
export function useAgentScope(
  agent: { resync: (scope: AgentScope) => Promise<void> },
  view: ScopeView | null,
  clearSurfaces: (e: McpEvent) => void,
): AgentScopeState {
  const scope = agentScope(view ?? NO_VIEW);
  const key = scopeKey(scope);
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const [ready, setReady] = useState(false);
  const readyRef = useRef(false);
  const [available, setAvailable] = useState(false);
  // Le rappel du fil dit aussi si un agent répond.
  const resync = useCallback(
    () =>
      agent.resync(scopeRef.current).then(
        () => setAvailable(true),
        (err) => {
          setAvailable(false);
          console.warn("resync", err);
        },
      ),
    [agent],
  );
  const [status, onAgentEvent] = useReducer(agentStatusReducer, IDLE);
  const loaded = view !== null;
  useEffect(() => {
    if (!ready || !loaded) return;
    clearSurfaces({ kind: "surfaces", list: [] });
    void resync();
  }, [resync, key, ready, loaded, clearSurfaces]);
  // Ne lit que des références : `subscribeHost` garde la première version.
  const onReady = useCallback(() => {
    if (!readyRef.current) {
      readyRef.current = true;
      setReady(true);
      return;
    }
    void resync();
  }, [resync]);
  const elsewhere = busyElsewhere(status, scope);
  const notice = elsewhere ? t("ui-agent-busy-elsewhere", { place: scopePlace(elsewhere, t) }) : null;
  return { scope, status, elsewhere, notice, ready: available, onAgentEvent, onReady };
}
