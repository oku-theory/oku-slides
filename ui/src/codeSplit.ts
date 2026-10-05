// Largeur du code d'un widget à droite de son aperçu, bornée pour que
// l'aperçu et le code gardent chacun de la place.

/** Place gardée au code, en px. */
export const CODE_MIN = 120;
/** Place gardée à l'aperçu, en px : ses pastilles et ses réglages restent lisibles. */
export const PREVIEW_MIN = 240;
/** Épaisseur du séparateur entre aperçu et code, en px. */
export const SPLIT = 12;

/**
 * Borne la largeur du code dans un sous-onglet de `total` px (séparateur compris) :
 * au moins `CODE_MIN` au code et `PREVIEW_MIN` à l'aperçu ; trop étroit pour les deux,
 * moitié / moitié ; sans place du tout (rien de mesuré), zéro.
 */
export function fitCode(total: number, size: number): number {
  const room = total - SPLIT;
  if (room < CODE_MIN + PREVIEW_MIN) return Math.max(0, Math.round(room / 2));
  return Math.min(Math.max(Math.round(size), CODE_MIN), room - PREVIEW_MIN);
}

/** Piste CSS du code pour une largeur choisie, bornée par la fenêtre ; `null` : proportion par défaut. */
export function codeTrack(size: number | null): string | null {
  return size === null ? null : `min(${size}px, calc(100% - ${PREVIEW_MIN + SPLIT}px))`;
}
