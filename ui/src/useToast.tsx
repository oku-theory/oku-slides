// Un avis passager en bas de l'écran : partagé par `App` et la fenêtre de l'agent.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { REAL_TIMERS, type Timers } from "./debounce";

/** Durée d'un avis, et d'une erreur (plus longue à lire). */
const SHOW_MS = 2500;
const ERROR_MS = 6000;

/** Un avis passager : texte, erreur ou non, effacé après un délai. */
export function useToast(timers: Timers = REAL_TIMERS): { say: (text: string, error?: boolean) => void; node: ReactNode } {
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const handle = useRef<unknown>(null);
  useEffect(() => () => void (handle.current !== null && timers.clear(handle.current)), [timers]);
  const say = useCallback(
    (text: string, error = false) => {
      setToast({ text, error });
      // Un nouvel avis remplace le précédent : son minuteur ne doit pas l'effacer en avance.
      if (handle.current !== null) timers.clear(handle.current);
      handle.current = timers.set(() => setToast((cur) => (cur?.text === text ? null : cur)), error ? ERROR_MS : SHOW_MS);
    },
    [timers],
  );
  const node = toast ? <div className={`toast${toast.error ? " error" : ""}`} role={toast.error ? "alert" : "status"}>{toast.text}</div> : null;
  return { say, node };
}
