// L'issue de l'action d'un champ. Une action A2UI ne rend rien (`dispatchAction`
// jette sa promesse), mais elle arrive à son routeur pendant l'appel : le routeur
// qui sait l'issue (les réglages de l'hôte) pose ici, pendant l'envoi, la promesse
// de son succès ; le champ la reprend juste après. Sans fournisseur, ou si le
// routeur ne pose rien (agent), l'action est tenue pour acceptée.

import { createContext } from "react";

/** Issue posée par le routeur pendant l'envoi : vrai si la session a accepté. */
export interface ActionOutcome {
  current: Promise<boolean> | null;
}

export const ActionOutcomeContext = createContext<ActionOutcome | null>(null);

/** Envoie l'action (`send`) et rend son issue : celle posée par le routeur pendant l'envoi, sinon succès. */
export function sendWithOutcome(outcome: ActionOutcome | null, send: () => void): Promise<boolean> {
  if (!outcome) {
    send();
    return Promise.resolve(true);
  }
  outcome.current = null;
  send();
  const done = outcome.current as Promise<boolean> | null;
  outcome.current = null;
  return done ?? Promise.resolve(true);
}
