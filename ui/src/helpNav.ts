// Aide en lecture seule : les touches qui passent d'une slide à l'autre.

type Keys = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey">;

/**
 * Rang `index + delta`, borné aux `count` slides ; `null` s'il ne change pas
 * (au bord, ou aucune slide). Un `index` inconnu (-1) part de la première.
 */
export function helpStep(index: number, delta: number, count: number): number | null {
  if (count === 0) return null;
  const from = Math.max(0, index);
  const to = Math.max(0, Math.min(count - 1, from + delta));
  return to === index ? null : to;
}

/**
 * Rang de la slide où mène la touche, depuis `index` sur `count` ; `null` :
 * la touche ne navigue pas (ou ne mène nulle part de nouveau).
 *
 * ← ↑ Page↑ : la précédente ; → ↓ Page↓ Espace : la suivante ; Début, Fin.
 * `onButton` : un bouton a le focus, Espace est à lui.
 */
export function helpTarget(e: Keys, index: number, count: number, onButton = false): number | null {
  if (e.metaKey || e.ctrlKey || e.altKey) return null;
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowUp":
    case "PageUp":
      return helpStep(index, -1, count);
    case " ":
      return onButton ? null : helpStep(index, 1, count);
    case "ArrowRight":
    case "ArrowDown":
    case "PageDown":
      return helpStep(index, 1, count);
    case "Home":
      return helpStep(index, -count, count);
    case "End":
      return helpStep(index, count, count);
    default:
      return null;
  }
}
