// Aperçu sur la scène : une miniature survolée (variante du tiroir, thème du sélecteur de
// l'hôte) y montre son rendu ; seule la couche d'aperçu de la scène s'y abonne, l'onglet ne
// se re-rend pas.

import { createContext, useSyncExternalStore } from "react";

/**
 * Le rendu montré sur la scène. `slide` : la slide qu'il rend (thème), montré sur elle
 * seule et hors édition ; absent (variante) : couvre la slide montrée quoi qu'il arrive.
 */
export interface StagePreview {
  url: string;
  slide?: string;
}

/** L'aperçu de la fenêtre et ses abonnés. */
export interface StagePreviewStore {
  get(): StagePreview | null;
  /** Pose `preview` : le dernier posé gagne. */
  show(preview: StagePreview): void;
  /** Retire `preview` s'il est encore montré ; celui qu'une autre miniature a posé depuis reste. */
  hide(preview: StagePreview): void;
  subscribe(listener: () => void): () => void;
}

// Par valeur : le retrait d'un nettoyage d'effet, objet neuf, retrouve l'aperçu qu'il avait posé.
const same = (a: StagePreview | null, b: StagePreview | null) => a?.url === b?.url && a?.slide === b?.slide;

/** Un magasin d'aperçu vide. */
export function createStagePreviewStore(): StagePreviewStore {
  let value: StagePreview | null = null;
  const listeners = new Set<() => void>();
  const set = (next: StagePreview | null) => {
    if (same(next, value)) return;
    value = next;
    for (const listener of listeners) listener();
  };
  return {
    get: () => value,
    show: (preview) => set(preview),
    hide(preview) {
      if (same(preview, value)) set(null);
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** Le magasin de la fenêtre ; absent (fenêtre de l'agent détachée) : pas d'aperçu. */
export const StagePreviewContext = createContext<StagePreviewStore | null>(null);

/** L'aperçu courant de `store`, suivi à chaque changement. */
export function useStagePreview(store: StagePreviewStore): StagePreview | null {
  return useSyncExternalStore(store.subscribe, store.get);
}
