// Hauteur de la bande de réglages sous la scène, bornée pour que la slide et la
// carte gardent chacune de la place.

import type { Bounds } from "./bounds";

/** Place gardée à la bande, en px : la rangée d'outils de la carte et un peu de contenu. */
export const BAND_MIN = 120;
/** Place gardée à la scène au-dessus, en px : la slide reste lisible. */
export const STAGE_MIN = 200;

/**
 * Bornes de la hauteur de la bande dans une colonne de `total` px (scène comprise) :
 * au moins `STAGE_MIN` à la scène, mais jamais moins de `BAND_MIN` à la bande, qui
 * l'emporte dans une colonne trop basse pour les deux, comme dans `bandHeight` ;
 * sans place du tout (rien de mesuré), zéro.
 */
export function bandBounds(total: number): Bounds {
  if (total <= 0) return { min: 0, max: 0 };
  return { min: BAND_MIN, max: Math.max(BAND_MIN, total - STAGE_MIN) };
}

/**
 * Hauteur CSS de la bande pour une hauteur choisie, bornée par la fenêtre comme `bandBounds`
 * (y compris une hauteur gardée hors bornes) ; `null` : hauteur par défaut.
 */
export function bandHeight(size: number): string;
export function bandHeight(size: number | null): string | null;
export function bandHeight(size: number | null): string | null {
  return size === null ? null : `max(${BAND_MIN}px, min(${size}px, calc(100% - ${STAGE_MIN}px)))`;
}
