// Largeurs des colonnes de la table (pile, fil de l'agent), bornées pour que
// la feuille garde toujours de la place entre les deux.

import { type Bounds, fitIn } from "./bounds";

/** Largeurs choisies aux séparateurs, en px. */
export interface Columns {
  pile: number;
  notes: number;
}

// The static bounds, equal to fixtures/ui/layout-bounds.json and the Rust `set_interface` (tests tie them).
export const PILE_MIN = 180;
export const PILE_MAX = 360;
export const NOTES_MIN = 300;
/** Place gardée à la feuille, entre la pile et le fil. */
export const SHEET_MIN = 320;

/** Bornes de `lo` à `hi`, sans jamais descendre sous `lo`. */
const span = (lo: number, hi: number): Bounds => ({ min: lo, max: Math.max(lo, hi) });

/**
 * Bornes de la largeur d'une colonne pour une table de `table` px et une pile de
 * `pile` px : pile dans [180, 360], fil d'au moins 300, et 320 gardés à la feuille ;
 * la pile cède avant la feuille, et le fil se borne contre la pile bornée.
 */
export function columnBounds(side: keyof Columns, table: number, pile: number): Bounds {
  const piles = span(PILE_MIN, Math.min(PILE_MAX, table - NOTES_MIN - SHEET_MIN));
  if (side === "pile") return piles;
  return span(NOTES_MIN, table - fitIn(piles, pile) - SHEET_MIN);
}

/** The grid's live widths: the table, its column 1 (pile) and 3 (thread); 0 without a table. */
export function measureColumns(el: HTMLElement | null): Columns & { table: number } {
  const cols = el ? getComputedStyle(el).gridTemplateColumns.split(" ").map(parseFloat) : [];
  return { table: el?.clientWidth ?? 0, pile: cols[0] || 0, notes: cols[2] || 0 };
}
