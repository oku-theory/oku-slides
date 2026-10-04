// Miniature de la slide d'un reçu (t-450). Le reçu ne porte pas le rendu de la
// slide, seulement une référence : son identifiant et l'empreinte de sa page au
// moment du reçu. L'interface la résout avec le cache des SVG (indexé par
// empreinte, partagé avec la pile) : dans Tauri comme dans le navigateur, le
// rendu passe par la même commande `svg` de l'hôte.

import { createContext } from "react";

/** La slide d'un reçu : identifiant et empreinte de sa page au moment du reçu. */
export interface SlideRef {
  id: string;
  hash: string;
}

/** Ce dont le reçu a besoin pour montrer une miniature ; fourni par l'app. */
export interface SlideThumbs {
  /** URL de l'image d'empreinte `hash` si elle est déjà reçue ; ne demande rien. */
  cached(hash: string): string | null;
  /** URL du rendu de la slide `id` à l'empreinte `hash` ; demandé à l'hôte au besoin (null en attendant). */
  svg(id: string, hash: string): string | null;
  /** Empreinte actuelle de la slide `id` ; undefined si elle n'existe plus. */
  current(id: string): string | undefined;
}

/** Absent (null) : les reçus ne montrent pas de miniature. */
export const SlideThumbsContext = createContext<SlideThumbs | null>(null);

/**
 * La miniature d'un reçu. L'état de la slide au moment du reçu si son image est
 * connue (en cache, ou slide inchangée depuis) ; sinon son état actuel, marqué
 * `changed` : l'hôte ne garde que le rendu courant des slides, l'ancien n'est plus
 * demandable. Rien si la slide n'existe plus ou que l'image n'est pas encore reçue.
 */
export function resolveSlideThumb(thumbs: SlideThumbs | null, ref: SlideRef): { url: string; changed: boolean } | null {
  if (!thumbs) return null;
  const hit = thumbs.cached(ref.hash);
  if (hit) return { url: hit, changed: false };
  const now = thumbs.current(ref.id);
  if (now === undefined) return null;
  const url = thumbs.svg(ref.id, now);
  return url ? { url, changed: now !== ref.hash } : null;
}
