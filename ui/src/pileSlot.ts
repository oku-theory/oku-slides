// Place d'insertion d'une miniature glissée dans la pile, verticale ou en bande horizontale.

/** Axe de la pile : colonne (`y`) ou bande (`x`). */
export type PileAxis = "x" | "y";

/** Boîte d'une miniature à l'écran (`getBoundingClientRect`). */
export interface ThumbRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

/**
 * Place d'insertion pour un pointeur en `pos` le long de `axis` : avant la première miniature
 * (dans l'ordre de la pile) dont on n'a pas dépassé le milieu ; après la dernière sinon.
 * Une bande de droite à gauche (`rtl`) se lit depuis la droite.
 */
export function dropSlot(thumbs: readonly ThumbRect[], pos: number, axis: PileAxis, rtl: boolean): number {
  const i = thumbs.findIndex((r) => {
    if (axis === "y") return pos < r.top + r.height / 2;
    const middle = r.left + r.width / 2;
    return rtl ? pos > middle : pos < middle;
  });
  return i < 0 ? thumbs.length : i;
}
