/** Couleurs : luminance, contraste, fond clair ou foncé pour une encre lisible, fond de l'éditeur. */

/** Luminance relative (WCAG) d'une couleur #rgb ou #rrggbb, null sinon. */
export function luminance(hex: string): number | null {
  const m = /^#?(?:([0-9a-f]{6})|([0-9a-f]{3})$)/i.exec(hex);
  if (!m) return null;
  const digits = m[1] ?? [...m[2]].map((c) => c + c).join("");
  const n = parseInt(digits, 16);
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => lin(v / 255));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/**
 * Seuil de luminance au-dessus duquel un fond est clair (encre foncée).
 *
 * Le même que `ink-on` de Typst (`tokens.typ`) : luma > 60 %, converti en WCAG.
 */
export const LIGHT_BACKGROUND = ((0.6 + 0.055) / 1.055) ** 2.4;

/** Rapport de contraste (WCAG) entre deux luminances, dans un sens ou l'autre. */
export function contrast(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** Contraste minimal texte / fond de l'éditeur : 3, le seuil WCAG du grand texte. */
export const EDITOR_MIN_CONTRAST = 3;

/** Luminance au-delà de laquelle un texte est clair (il lui faut un fond sombre). */
export const LIGHT_INK = 0.4;

/** Fond de repli de l'éditeur sous un texte clair, et sous un texte sombre. */
export const EDITOR_DARK = "#1d2230";
export const EDITOR_LIGHT = "#ffffff";

/**
 * Fond de l'éditeur : celui de la slide sous le texte ; s'il ne contraste pas
 * assez avec le texte (texte clair sur un bandeau que Typst ne voit pas), un
 * fond sombre ou clair selon le texte, pour qu'il reste lisible.
 */
export function editorBackground(fill: string | null, bg: string | null): string | undefined {
  const lf = fill ? luminance(fill) : null;
  const lb = bg ? luminance(bg) : null;
  if (lf === null) return bg ?? undefined;
  if (lb !== null && contrast(lf, lb) >= EDITOR_MIN_CONTRAST) return bg!;
  return lf > LIGHT_INK ? EDITOR_DARK : EDITOR_LIGHT;
}
