/** Couleurs : luminance, et fond clair ou foncé pour choisir une encre lisible. */

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
