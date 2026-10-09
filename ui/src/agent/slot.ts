// Une place de l'interface remplie par un autre composant : le conteneur de l'agent y pose une
// surface (fiche sur la scène, tiroir, fenêtre) sans que le composant qui tient la place se
// re-rende. Seule la vue de la place (`SlotView`) suit ce qui y est posé.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";

/** Une place : ce qui y est posé, et de quoi suivre ses changements. */
export interface Slot {
  get: () => ReactNode;
  set: (node: ReactNode) => void;
  subscribe: (listener: () => void) => () => void;
}

/** Une place vide. */
export function createSlot(): Slot {
  let node: ReactNode = null;
  const listeners = new Set<() => void>();
  return {
    get: () => node,
    set: (next) => {
      if (next === node) return;
      node = next;
      listeners.forEach((l) => l());
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
}

/** Ce qui est posé dans `slot`, re-rendu seul à chaque changement. */
export function SlotView({ slot }: { slot: Slot }) {
  return useSyncExternalStore(slot.subscribe, slot.get, slot.get);
}
