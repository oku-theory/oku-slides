// Un avis passager en bas de l'écran : partagé par `App` et la fenêtre de l'agent.

import { useCallback, type ReactNode } from "react";
import { REAL_TIMERS, type Timers } from "./debounce";
import { useTransient } from "./useTransient";

/** Durée d'un avis, et d'une erreur (plus longue à lire). */
const SHOW_MS = 2500;
export const ERROR_MS = 6000;

/** Un avis passager : texte, erreur ou non, effacé après un délai. */
export function useToast(timers: Timers = REAL_TIMERS): { say: (text: string, error?: boolean) => void; node: ReactNode } {
  const [toast, show] = useTransient<{ text: string; error: boolean }>(timers);
  const say = useCallback((text: string, error = false) => show({ text, error }, error ? ERROR_MS : SHOW_MS), [show]);
  const node = toast ? <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div> : null;
  return { say, node };
}
