/**
 * Police CSS d'un style Typst et ses métriques verticales, mesurées dans le
 * DOM : elles dépendent de la police (Newsreader et Instrument Sans n'ont ni la
 * même capitale ni les mêmes ascendante et descendante). Sans mise en page
 * (tests), les mesures valent `null` et l'appelant garde une valeur par défaut.
 */
import type { TextStyle } from "./api";
import { fontStack } from "./appearance";

const WEIGHTS: Record<string, number> = {
  thin: 100, extralight: 200, light: 300, regular: 400, medium: 500, semibold: 600, bold: 700, extrabold: 800, black: 900,
};

/** Graisse et pile de polices CSS d'un style Typst. */
export function cssFont(st: TextStyle): { weight: number; family: string } {
  const weight = typeof st.weight === "number" ? st.weight : WEIGHTS[String(st.weight).replace(/[^a-z]/g, "")] ?? 400;
  return { weight, family: fontStack(st.font, "serif") };
}

export interface Font {
  weight: number;
  family: string;
  /** Taille (px). */
  size: number;
}

const cache = new Map<string, number>();

/**
 * Mesure `read` sur une sonde invisible à la typographie de `font` ; gardée en
 * cache une fois la police chargée (avant : la police de repli, à remesurer).
 */
function measure(font: Font, key: string, build: (probe: HTMLElement) => () => number): number | null {
  const shorthand = `${font.weight} ${font.size}px ${font.family}`;
  const id = `${shorthand}/${key}`;
  const known = cache.get(id);
  if (known !== undefined) return known;
  if (typeof document === "undefined" || !document.body) return null;
  const probe = document.createElement("div");
  probe.style.cssText = "position:absolute;left:0;top:0;visibility:hidden;white-space:pre;pointer-events:none";
  Object.assign(probe.style, { fontFamily: font.family, fontSize: `${font.size}px`, fontWeight: String(font.weight) });
  const read = build(probe);
  document.body.append(probe);
  const value = read();
  probe.remove();
  if (!(Number.isFinite(value) && value > 0)) return null;
  if (document.fonts?.check(shorthand)) cache.set(id, value);
  return value;
}

/** Hauteur de capitale (`1cap`, px) : Typst y pose le haut de chaque ligne. */
export function capHeight(font: Font): number | null {
  return measure(font, "cap", (probe) => {
    const box = document.createElement("div");
    box.style.height = "1cap";
    probe.append(box);
    return () => box.getBoundingClientRect().height;
  });
}

/** Ligne de base (px depuis le haut) d'une ligne CSS de hauteur `line`, arrondis du moteur compris. */
export function baselineIn(font: Font, line: number): number | null {
  return measure(font, `base/${line}`, (probe) => {
    const row = document.createElement("div");
    row.style.lineHeight = `${line}px`;
    // Un inline-block vide pose son bas (ici son haut) sur la ligne de base.
    const mark = document.createElement("span");
    mark.style.cssText = "display:inline-block;width:0;height:0";
    row.append("H", mark);
    probe.append(row);
    return () => mark.getBoundingClientRect().top - row.getBoundingClientRect().top;
  });
}
