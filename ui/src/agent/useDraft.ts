// Le brouillon de l'onglet, tenu par l'hôte : il survit au détachement de l'agent. Chaque vue
// garde le sien en local, l'envoie dans l'ordre, et ignore l'écho de ses propres envois ; un rejeu
// de l'hôte qui n'a pas sa dernière écriture (écho perdu, envoi refusé) la lui renvoie.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { REAL_TIMERS, debounced, type Timers } from "../debounce";
import type { Attachment } from "../api";
import { afterSend, appendDraft } from "../draft";
import { randomId } from "../randomId";

/** Délai d'envoi de la frappe. */
export const DRAFT_TYPING_MS = 300;

export interface DraftState {
  text: string | null;
  dictations: number;
  typing: number;
  /** Attachments (images and documents), held by the host. */
  attachments: Attachment[];
}

export interface Draft {
  state: DraftState;
  /** Brouillon reçu de l'hôte ; l'écho d'un envoi de cette vue est ignoré. */
  onHost: (e: { text: string | null; origin: string | null; attachments: Attachment[] }) => void;
  /** Texte dicté ajouté (envoyé tout de suite) ; renvoie le brouillon à jour. */
  append: (text: string) => { text: string; attachments: Attachment[] };
  /** Ouvre le brouillon pour écrire (⌘K ; envoyé tout de suite). */
  type: () => void;
  /** Frappe (envoyée après DRAFT_TYPING_MS). */
  change: (text: string) => void;
  /** Ferme le brouillon (envoyé tout de suite). */
  close: () => void;
  /**
   * `text` went to the agent: removes it, keeping what was added since; closes when empty.
   * Returns `true` when text remains (the caller removes the attachments it sent).
   */
  drop: (text: string) => boolean;
  /** Envoie tout de suite la frappe en attente ; résolue une fois tous les envois partis. */
  flush: () => Promise<void>;
}

const EMPTY: DraftState = { text: null, dictations: 0, typing: 0, attachments: [] };

const sameAttachments = (a: Attachment[], b: Attachment[]) => a.length === b.length && a.every((x, i) => x.id === b[i]?.id);

export function useDraft(setDraft: (text: string | null, origin: string) => Promise<void>, timers: Timers = REAL_TIMERS): Draft {
  const [state, setState] = useState<DraftState>(EMPTY);
  const current = useRef(state);
  const write = useCallback((next: DraftState) => {
    current.current = next;
    setState(next);
  }, []);
  // Un identifiant de vue ; chaque envoi y ajoute son numéro (`v-xxx:n`) pour reconnaître ses échos.
  const [view] = useState(() => randomId("v"));
  const push = useRef(setDraft);
  push.current = setDraft;
  // Un envoi à la fois : l'hôte reçoit les écritures dans l'ordre où elles ont été faites.
  const queue = useRef(Promise.resolve());
  // Numéro du dernier envoi, et le plus grand revenu (écho ou refus) : tant que le dernier n'est pas
  // revenu, ce qui vient d'ailleurs est plus ancien que la dernière écriture de cette vue.
  const sent = useRef(0);
  const settled = useRef(0);
  // Dernière écriture refusée (onglet pas encore démarré) : l'hôte n'a pas ce brouillon.
  const refused = useRef(false);
  const send = useCallback(
    (text: string | null) => {
      const n = ++sent.current;
      refused.current = false;
      queue.current = queue.current
        .then(() => push.current(text, `${view}:${n}`))
        .catch((e) => {
          // Pas d'écho pour un envoi refusé : il est revenu.
          settled.current = Math.max(settled.current, n);
          if (n === sent.current) refused.current = true;
          console.warn("brouillon", String(e));
        });
    },
    [view],
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
    (e: { text: string | null; origin: string | null; attachments: Attachment[] }) => {
      // Attachments come from the host alone: every event, echo included, gives them.
      if (!sameAttachments(current.current.attachments, e.attachments)) write({ ...current.current, attachments: e.attachments });
      const own = e.origin?.startsWith(`${view}:`) ? Number(e.origin.slice(view.length + 1)) : null;
      if (own !== null) {
        settled.current = Math.max(settled.current, own);
        return;
      }
      const waiting = settled.current < sent.current;
      // Un rejeu de l'hôte (`follow`, `resync`) quand cette vue a écrit en dernier mais que l'hôte
      // ne l'a pas : écho perdu (relais en retard, vue sans abonnement) ou écriture refusée (onglet
      // pas encore démarré). Elle renvoie son texte, au lieu d'ignorer ou de subir le rejeu.
      if (e.origin === null && (waiting || refused.current) && !pending.current) {
        send(current.current.text);
        return;
      }
      if (pending.current || waiting) return;
      refused.current = false;
      if (current.current.text !== e.text) write({ ...current.current, text: e.text });
    },
    [view, write, send],
  );
  const append = useCallback(
    (text: string) => {
      const d = current.current;
      const next = appendDraft(d.text, text);
      write({ ...d, text: next, dictations: d.dictations + 1 });
      now(next);
      return { text: next, attachments: d.attachments };
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
    write({ ...current.current, text: null, attachments: [] });
    now(null);
  }, [write, now]);
  const drop = useCallback(
    (text: string) => {
      const d = current.current;
      if (d.text === null) return false;
      const left = afterSend(d.text, text);
      if (left === null) {
        close();
        return false;
      }
      if (left === d.text) return true;
      write({ ...d, text: left });
      now(left);
      return true;
    },
    [write, now, close],
  );
  const flush = useCallback(() => {
    typing.flush();
    return queue.current;
  }, [typing]);
  return useMemo(
    () => ({ state, onHost, append, type, change, close, drop, flush }),
    [state, onHost, append, type, change, close, drop, flush],
  );
}
