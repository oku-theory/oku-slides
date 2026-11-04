// Bounds of a size set at a divider (Divider): each panel gives its own (columnBounds),
// and the divider clamps the gesture to them.

/** Taille la plus petite et la plus grande permises, en px ; `min <= max`. */
export interface Bounds {
  min: number;
  max: number;
}

/** Arrondit `size` au px, puis la borne à `bounds`. */
export function fitIn(bounds: Bounds, size: number): number {
  return Math.min(Math.max(Math.round(size), bounds.min), bounds.max);
}
