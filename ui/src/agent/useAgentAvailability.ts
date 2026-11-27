// La disponibilité de l'agent vue de la page. Elle est demandée à l'hôte quand le dialogue
// d'import s'ouvre, et mise à jour par l'évènement `availability` (`onAvailability`
// d'`AgentPanel`), qui arrive aussi de lui-même après un lancement raté.

import { useCallback, useEffect, useRef, useState } from "react";
import type { AgentApi } from "./AgentPanel";

/** Ce que la page sait de la disponibilité de l'agent. */
interface AgentAvailability {
  /** Pourquoi aucun agent n'est utilisable, localisé ; `null` : utilisable ou pas encore su. */
  unusable: string | null;
  /** À donner à `AgentPanel` : une disponibilité reçue de l'hôte. */
  onAvailability: (unusable: string | null) => void;
}

/** Suit la disponibilité de l'agent ; la redemande chaque fois que `asking` devient vrai. */
export function useAgentAvailability(
  agent: Pick<AgentApi, "availability">,
  asking: boolean,
): AgentAvailability {
  const [unusable, setUnusable] = useState<string | null>(null);
  const latest = useRef(agent);
  latest.current = agent;
  useEffect(() => {
    // Sans réponse, la case reste active : l'hôte refuse quand même un peaufinage impossible.
    if (asking) void latest.current.availability().catch((e) => console.warn(e));
  }, [asking]);
  const onAvailability = useCallback((u: string | null) => setUnusable(u), []);
  return { unusable, onAvailability };
}
