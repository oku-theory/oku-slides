// Width of a widget's code, on the right of the preview and settings column, bounded
// so the preview column and the code each keep some room.

import type { Bounds } from "./bounds";

/** Place gardée au code, en px. */
export const CODE_MIN = 120;
/** Room kept for the preview and settings column, in px: the fields stay readable. */
export const SIDE_MIN = 240;
/** Thickness of the divider between that column and the code, in px. */
export const SPLIT = 12;

/**
 * Bounds of the code's width in an item view `total` px wide (divider included).
 *
 * At least `CODE_MIN` for the code and `SIDE_MIN` for the preview column; too narrow for both, half
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
