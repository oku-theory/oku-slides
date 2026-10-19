// Aperçu d'un thème sur la scène : le sélecteur de thème de l'hôte y pose la miniature
// survolée ; seule la couche d'aperçu de la scène s'y abonne, l'onglet ne se re-rend pas.

import { createContext, useSyncExternalStore } from "react";

/** La miniature d'un thème et la slide qu'elle montre. */
export interface StagePreview {
  slide: string;
  url: string;
}

/** Une valeur d'aperçu et ses abonnés. */
export interface StagePreviewStore {
  get(): StagePreview | null;
  set(preview: StagePreview | null): void;
  subscribe(listener: () => void): () => void;
}

/** Un magasin d'aperçu vide. */
export function createStagePreviewStore(): StagePreviewStore {
  let value: StagePreview | null = null;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set(preview) {
      if (preview?.slide === value?.slide && preview?.url === value?.url) return;
      value = preview;
      for (const listener of listeners) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** Le magasin de la fenêtre ; absent (fenêtre de l'agent) : pas d'aperçu. */
export const StagePreviewContext = createContext<StagePreviewStore | null>(null);

const noSubscribe = () => () => {};
const none = () => null;

/** L'aperçu courant de `store` ; `null` sans magasin. */
export function useStagePreview(store: StagePreviewStore | null): StagePreview | null {
  return useSyncExternalStore(store ? store.subscribe : noSubscribe, store ? store.get : none);
}
