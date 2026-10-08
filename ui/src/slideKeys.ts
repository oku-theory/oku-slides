// Les touches qui passent d'une slide à l'autre : dans l'aide, et dans l'éditeur.

type Keys = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey">;

/**
 * Rang `index + delta`, borné aux `count` slides ; `null` s'il ne change pas
 * (au bord, ou aucune slide). Un `index` inconnu (-1) part de la première.
 */
export function slideStep(index: number, delta: number, count: number): number | null {
  if (count === 0) return null;
  const from = Math.max(0, index);
  const to = Math.max(0, Math.min(count - 1, from + delta));
  return to === index ? null : to;
}

/** Ce que la navigation laisse à d'autres : Espace à un bouton, les flèches à la sélection. */
export interface SlideKeys {
  /** Espace passe à la suivante (défaut : oui). */
  space?: boolean;
  /** ← → ↑ ↓ naviguent (défaut : oui). */
  arrows?: boolean;
}

/**
 * Rang de la slide où mène la touche, depuis `index` sur `count` ; `null` :
 * la touche ne navigue pas (ou ne mène nulle part de nouveau).
 *
 * ← ↑ Page↑ : la précédente ; → ↓ Page↓ Espace : la suivante ; Début, Fin.
 */
export function slideTarget(e: Keys, index: number, count: number, { space = true, arrows = true }: SlideKeys = {}): number | null {
  if (e.metaKey || e.ctrlKey || e.altKey) return null;
  if (!arrows && e.key.startsWith("Arrow")) return null;
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowUp":
    case "PageUp":
      return slideStep(index, -1, count);
    case " ":
      return space ? slideStep(index, 1, count) : null;
    case "ArrowRight":
    case "ArrowDown":
    case "PageDown":
      return slideStep(index, 1, count);
    case "Home":
      return slideStep(index, -count, count);
    case "End":
      return slideStep(index, count, count);
    default:
      return null;
  }
}
