// Bornes d'une taille réglée à un séparateur (Divider) : chaque panneau dit
// les siennes (bandBounds, codeBounds, columnBounds), le séparateur y borne le geste.

/** Taille la plus petite et la plus grande permises, en px ; `min <= max`. */
export interface Bounds {
  min: number;
  max: number;
}

/** Arrondit `size` au px, puis la borne à `bounds`. */
export function fitIn(bounds: Bounds, size: number): number {
  return Math.min(Math.max(Math.round(size), bounds.min), bounds.max);
}
