// Le brouillon de l'onglet, tenu par l'hôte : il survit au détachement de l'agent. Chaque vue
// garde le sien en local, l'envoie dans l'ordre, et ignore l'écho de ses propres envois.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { REAL_TIMERS, debounced, type Timers } from "../debounce";
import { appendDraft } from "../draft";
import { randomId } from "../randomId";

/** Délai d'envoi de la frappe. */
export const DRAFT_TYPING_MS = 300;

export interface DraftState {
  text: string | null;
  dictations: number;
  typing: number;
}

export interface Draft {
  state: DraftState;
  /** Brouillon reçu de l'hôte ; l'écho d'un envoi de cette vue est ignoré. */
  onHost: (e: { text: string | null; origin: string | null }) => void;
  /** Texte dicté ajouté (envoyé tout de suite). */
  append: (text: string) => void;
  /** Ouvre le brouillon pour écrire (⌘K ; envoyé tout de suite). */
  type: () => void;
  /** Frappe (envoyée après DRAFT_TYPING_MS). */
  change: (text: string) => void;
  /** Ferme le brouillon (envoyé tout de suite). */
  close: () => void;
  /** Envoie tout de suite la frappe en attente ; résolue une fois tous les envois partis. */
  flush: () => Promise<void>;
}

const EMPTY: DraftState = { text: null, dictations: 0, typing: 0 };

export function useDraft(setDraft: (text: string | null, origin: string) => Promise<void>, timers: Timers = REAL_TIMERS): Draft {
  const [state, setState] = useState<DraftState>(EMPTY);
  const current = useRef(state);
  const write = useCallback((next: DraftState) => {
    current.current = next;
    setState(next);
  }, []);
  // Un identifiant de vue, pour reconnaître ses propres échos.
  const [origin] = useState(() => randomId("v"));
  const push = useRef(setDraft);
  push.current = setDraft;
  // Un envoi à la fois : l'hôte reçoit les écritures dans l'ordre où elles ont été faites.
  const queue = useRef(Promise.resolve());
  // Envois dont l'écho n'est pas revenu : tant qu'il y en a, ce qui vient d'ailleurs est plus ancien.
  const inflight = useRef(0);
  const send = useCallback(
    (text: string | null) => {
      inflight.current++;
      queue.current = queue.current
        .then(() => push.current(text, origin))
        .catch((e) => {
          // Pas d'écho pour un envoi refusé : il ne compte plus.
          inflight.current = Math.max(0, inflight.current - 1);
          console.warn("brouillon", String(e));
        });
    },
    [origin],
  );
  const pending = useRef(false);
  const typing = useMemo(
    () =>
      debounced(
        (text: string) => {
          pending.current = false;
          send(text);
        },
        DRAFT_TYPING_MS,
        timers,
      ),
    [send, timers],
  );
  useEffect(() => () => typing.flush(), [typing]);
  const now = useCallback(
    (text: string | null) => {
      typing.cancel();
      pending.current = false;
      send(text);
    },
    [typing, send],
  );
  const onHost = useCallback(
    (e: { text: string | null; origin: string | null }) => {
      if (e.origin === origin) {
        inflight.current = Math.max(0, inflight.current - 1);
        return;
      }
      // Un rejeu de l'hôte (`follow`, `resync`) pendant qu'un écho manque : l'écho est perdu
      // (relais en retard, vue sans abonnement). Cette vue a écrit en dernier : elle se resynchronise
      // en renvoyant son texte, au lieu d'ignorer pour toujours ce qui vient d'ailleurs.
      if (e.origin === null && inflight.current > 0 && !pending.current) {
        inflight.current = 0;
        send(current.current.text);
        return;
      }
      if (pending.current || inflight.current > 0) return;
      if (current.current.text !== e.text) write({ ...current.current, text: e.text });
    },
    [origin, write, send],
  );
  const append = useCallback(
    (text: string) => {
      const d = current.current;
      const next = appendDraft(d.text, text);
      write({ ...d, text: next, dictations: d.dictations + 1 });
      now(next);
    },
    [write, now],
  );
  const type = useCallback(() => {
    const d = current.current;
    const next = d.text ?? "";
    write({ ...d, text: next, typing: d.typing + 1 });
    now(next);
  }, [write, now]);
  const change = useCallback(
    (text: string) => {
      pending.current = true;
      write({ ...current.current, text });
      typing(text);
    },
    [write, typing],
  );
  const close = useCallback(() => {
    write({ ...current.current, text: null });
    now(null);
  }, [write, now]);
  const flush = useCallback(() => {
    typing.flush();
    return queue.current;
  }, [typing]);
  return useMemo(() => ({ state, onHost, append, type, change, close, flush }), [state, onHost, append, type, change, close, flush]);
}
