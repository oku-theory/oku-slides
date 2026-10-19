// Aperçu d'une variante sur la scène : la miniature survolée y montre son rendu.

import { createContext } from "react";

/** Pose (`on`) ou retire l'image `url` sur la scène, à la place de la slide. */
export type SlidePreview = (url: string, on: boolean) => void;

/** Absent (null) : les miniatures ne montrent rien sur la scène (fenêtre de l'agent détachée). */
export const SlidePreviewContext = createContext<SlidePreview | null>(null);

/**
 * L'aperçu après une demande : la dernière image posée gagne ; un retrait n'efface
 * que sa propre image, pas celle qu'une autre miniature a posée depuis.
 */
export function nextPreview(current: string | null, url: string, on: boolean): string | null {
  if (on) return url;
  return current === url ? null : current;
}
