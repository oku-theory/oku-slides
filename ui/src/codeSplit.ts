// Largeur du code d'un widget, sous son aperçu à droite des réglages, bornée
// pour que les réglages et le code gardent chacun de la place.

import type { Bounds } from "./bounds";

/** Place gardée au code, en px. */
export const CODE_MIN = 120;
/** Place gardée aux réglages, en px : leurs champs restent lisibles. */
export const SIDE_MIN = 240;
/** Épaisseur du séparateur entre réglages et code, en px. */
export const SPLIT = 12;

/**
 * Bornes de la largeur du code dans un sous-onglet de `total` px (séparateur compris) :
 * au moins `CODE_MIN` au code et `SIDE_MIN` aux réglages ; trop étroit pour les deux,
 * moitié / moitié ; sans place du tout (rien de mesuré), zéro.
 */
export function codeBounds(total: number): Bounds {
  const room = total - SPLIT;
  if (room < CODE_MIN + SIDE_MIN) {
    const half = Math.max(0, Math.round(room / 2));
    return { min: half, max: half };
  }
  return { min: CODE_MIN, max: room - SIDE_MIN };
}

/** Piste CSS du code pour une largeur choisie, bornée par la fenêtre ; `null` : proportion par défaut. */
export function codeTrack(size: number | null): string | null {
  return size === null ? null : `min(${size}px, calc(100% - ${SIDE_MIN + SPLIT}px))`;
}
