// Le fil suivi par une autre vue de l'onglet (fenêtre de l'agent).

import { useCallback, useReducer, useRef, useState } from "react";
import type { AgentEvent, AgentState, agentApi } from "../api";
import { agentStateReducer, type AgentThreadState } from "./useAgentThread";

/** Le fil suivi : prêt quand le dernier `follow` a réussi. */
export function useFollowThread(agent: Pick<ReturnType<typeof agentApi>, "follow">): AgentThreadState {
  const [followed, setFollowed] = useState(false);
  const [state, onStatus] = useReducer(agentStateReducer, "idle" as AgentState);
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
  const onAgentEvent = useCallback((e: AgentEvent) => onStatus(e), []);
  return { state, ready: followed, onAgentEvent, onReady };
}
