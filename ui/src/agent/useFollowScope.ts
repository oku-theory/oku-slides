// La portée suivie par une autre vue de l'onglet (fenêtre de l'agent) : celle que montre l'hôte.

import { useCallback, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentScope, McpEvent, agentApi } from "../api";
import { t } from "../i18n";
import { busyElsewhere, scopePlace } from "./scope";
import { IDLE, agentStatusReducer, type AgentScopeState } from "./useAgentScope";

const SLIDES: AgentScope = { kind: "slides" };

/** La portée suivie : celle que l'hôte annonce (`viewing`) après `follow`. */
export function useFollowScope(agent: Pick<ReturnType<typeof agentApi>, "follow">): AgentScopeState & { onMcp: (e: McpEvent) => void } {
  const [viewing, setViewing] = useState<AgentScope | null>(null);
  const [followed, setFollowed] = useState(false);
  const [status, onStatus] = useReducer(agentStatusReducer, IDLE);
  // Seule la réponse du dernier `follow` compte : une reconnexion en relance un.
  const asked = useRef(0);
  const onReady = useCallback(() => {
    const n = ++asked.current;
    agent.follow().then(
      () => {
        if (n === asked.current) setFollowed(true);
      },
      (err) => {
        if (n === asked.current) setFollowed(false);
        console.warn("follow", err);
      },
    );
  }, [agent]);
  const onMcp = useCallback((e: McpEvent) => {
    if (e.kind === "viewing") setViewing(e.scope);
  }, []);
  const onAgentEvent = useCallback((e: AgentEvent) => onStatus(e), []);
  const scope = viewing ?? SLIDES;
  const elsewhere = busyElsewhere(status, scope);
  const notice = elsewhere ? t("ui-agent-busy-elsewhere", { place: scopePlace(elsewhere, t) }) : null;
  return { scope, status, elsewhere, notice, ready: followed && viewing !== null, onAgentEvent, onReady, onMcp };
}
