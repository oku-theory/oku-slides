// La révision de la dictée et de l'agent vue de la page, hors de l'état d'App.

import { useSyncExternalStore } from "react";

let value = 0;
const listeners = new Set<() => void>();

/** La dictée ou l'agent ont changé : les abonnés relisent. */
export function bumpVoiceSeq(): void {
  value += 1;
  for (const l of listeners) l();
}

/** Abonne `l` aux changements ; rend de quoi le désabonner. */
export function subscribeVoiceSeq(l: () => void): () => void {
  listeners.add(l);
  return () => void listeners.delete(l);
}

/** La révision courante. */
export function voiceSeq(): number {
  return value;
}

/** La révision courante, relue par React à chaque changement. */
export function useVoiceSeq(): number {
  return useSyncExternalStore(subscribeVoiceSeq, voiceSeq);
}
