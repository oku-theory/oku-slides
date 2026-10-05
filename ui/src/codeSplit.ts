// Largeur du code d'un widget, sous son aperçu à droite des réglages, bornée
// pour que les réglages et le code gardent chacun de la place.

/** Place gardée au code, en px. */
export const CODE_MIN = 120;
/** Place gardée aux réglages, en px : leurs champs restent lisibles. */
export const SIDE_MIN = 240;
/** Épaisseur du séparateur entre réglages et code, en px. */
export const SPLIT = 12;

/**
 * Borne la largeur du code dans un sous-onglet de `total` px (séparateur compris) :
 * au moins `CODE_MIN` au code et `SIDE_MIN` aux réglages ; trop étroit pour les deux,
 * moitié / moitié ; sans place du tout (rien de mesuré), zéro.
 */
export function fitCode(total: number, size: number): number {
  const room = total - SPLIT;
  if (room < CODE_MIN + SIDE_MIN) return Math.max(0, Math.round(room / 2));
  return Math.min(Math.max(Math.round(size), CODE_MIN), room - SIDE_MIN);
}

/** Piste CSS du code pour une largeur choisie, bornée par la fenêtre ; `null` : proportion par défaut. */
export function codeTrack(size: number | null): string | null {
  return size === null ? null : `min(${size}px, calc(100% - ${SIDE_MIN + SPLIT}px))`;
}
