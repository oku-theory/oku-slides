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
 * Bounds of the code's width in an item view `total` px wide (divider included).
 *
 * At least `CODE_MIN` for the code and `SIDE_MIN` for the settings; too narrow for both, half
 * and half; with no room at all (nothing measured), zero.
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
