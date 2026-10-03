// Largeurs des colonnes de la table (pile, fil de l'agent), bornées pour que
// la feuille garde toujours de la place entre les deux.

/** Largeurs choisies aux séparateurs, en px. */
export interface Columns {
  pile: number;
  notes: number;
}

export const PILE_MIN = 180;
export const PILE_MAX = 360;
export const NOTES_MIN = 300;
/** Place gardée à la feuille, entre la pile et le fil. */
export const SHEET_MIN = 320;

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));

/**
 * Borne les largeurs pour une table de `table` px : pile dans [180, 360],
 * fil d'au moins 300, et 320 gardés à la feuille ; la pile cède avant la feuille.
 */
export function fitColumns(table: number, pile: number, notes: number): Columns {
  const p = clamp(Math.round(pile), PILE_MIN, Math.min(PILE_MAX, table - NOTES_MIN - SHEET_MIN));
  return { pile: p, notes: clamp(Math.round(notes), NOTES_MIN, table - p - SHEET_MIN) };
}
