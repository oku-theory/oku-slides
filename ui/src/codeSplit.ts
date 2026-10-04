// Taille du code d'un widget à côté de son aperçu (dessous ou à droite), bornée
// pour que l'aperçu et le code gardent chacun de la place.

/** Axe du partage affiché : code sous l'aperçu ou à sa droite. */
export type CodeAxis = "below" | "right";

/** Place gardée au code, en px. */
export const CODE_MIN = 120;
/** Place gardée à l'aperçu, en px. */
export const PREVIEW_MIN = 120;
/** Épaisseur du séparateur entre aperçu et code, en px. */
export const SPLIT = 12;

/**
 * Borne la taille du code dans un sous-onglet de `total` px (séparateur compris) :
 * au moins 120 au code et 120 à l'aperçu ; trop petit pour les deux, moitié / moitié.
 */
export function fitCode(total: number, size: number): number {
  const room = total - SPLIT;
  if (room < CODE_MIN + PREVIEW_MIN) return Math.round(room / 2);
  return Math.min(Math.max(Math.round(size), CODE_MIN), room - PREVIEW_MIN);
}

/** Piste CSS du code pour une taille choisie, bornée par la fenêtre ; `null` : proportion par défaut. */
export function codeTrack(size: number | null): string | null {
  return size === null ? null : `min(${size}px, calc(100% - ${PREVIEW_MIN + SPLIT}px))`;
}
