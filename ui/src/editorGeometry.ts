/**
 * Géométrie de l'éditeur et des placeholders d'un champ : posés exactement
 * sur le texte rendu (police, taille, graisse, couleur, interlignage,
 * alignement, première ligne sur la ligne de base Typst).
 */
import type { CSSProperties } from "react";
import type { FieldView, TextStyle } from "./api";
import { editorBackground } from "./color";
import { baselineIn, capHeight, cssFont, textWidth } from "./fontMetrics";
import { fieldHitBox, lineHeight, unplaced } from "./stageFields";

/** Marge intérieure de l'éditeur (px écran) : le curseur et la sélection ne collent pas au bord. */
const PAD = 5;

/** Typographie CSS d'un style Typst à l'échelle de la scène. */
export function typography(st: TextStyle, scale: number) {
  const size = st.size * scale;
  const line = lineHeight(st, scale);
  const { weight, family } = cssFont(st);
  const tracking = st.tracking * scale;
  const align = st.align ?? "left";
  return {
    size,
    line,
    weight,
    family,
    tracking,
    align,
    css: {
      fontFamily: family,
      fontSize: size,
      fontWeight: weight,
      lineHeight: `${line}px`,
      letterSpacing: tracking || undefined,
      // Typst règle la taille optique sur la taille en pt ; CSS (auto) sur les px
      // à l'écran : sans cela, une scène réduite prend une autre coupe de la police.
      fontVariationSettings: `'opsz' ${st.size}`,
      textAlign: align,
      textTransform: st.case === "upper" ? "uppercase" : st.case === "lower" ? "lowercase" : undefined,
    } satisfies CSSProperties,
  };
}

/** Placeholder d'un champ vide : la typographie du champ, dans sa boîte cliquable. */
export function placeholderStyle(f: FieldView, scale: number): CSSProperties {
  if (!f.style) return {};
  return { ...typography(f.style, scale).css, color: f.style.fill ?? undefined };
}

/**
 * Éditeur posé sur le champ. `plain` épouse son texte et s'élargit en
 * tapant, jusqu'à sa zone (`ax`/`aw`) ; `rich` prend sa zone, largeur où
 * Typst revient à la ligne. Sans typographie connue : sur la boîte
 * cliquable, dans la police de l'interface. `fields` : les champs de la vue
 * (un champ vide s'empile avec ses voisins sur la même ancre).
 */
export function editorGeometry(f: FieldView, draft: string, scale: number, fields: readonly FieldView[]): CSSProperties {
  const st = f.style;
  const hit = fieldHitBox(f, scale, fields);
  if (!st) {
    return { left: hit.left - PAD, top: hit.top - PAD, width: Math.max(hit.width, 240) + 2 * PAD, minHeight: hit.height + 2 * PAD, padding: PAD };
  }
  const ty = typography(st, scale);
  // Haut de la première ligne : la boîte rendue, ou la ligne dessinée à côté de l'ancre.
  const y = unplaced(f) ? hit.top : f.y * scale;
  // Ligne de base de la première ligne : Typst la place à y + hauteur de capitale ;
  // CSS à mi-interligne + (ascendante − descendante) / 2, arrondis du moteur compris.
  // Mesurées pour la police ; sans mise en page, une police à capitale de 0,7 em.
  const font = { weight: ty.weight, family: ty.family, size: ty.size, opsz: st.size };
  const cap = capHeight(font) ?? ty.size * 0.7;
  const top = y + cap - (baselineIn(font, ty.line) ?? ty.line / 2 + ty.size * 0.325);
  const zoneLeft = f.ax * scale;
  const zoneWidth = f.aw * scale;

  let width: number;
  let x: number;
  if (f.kind === "rich") {
    // La largeur où Typst revient à la ligne : la zone, pas la plus longue ligne rendue.
    width = zoneWidth;
    x = zoneLeft;
  } else {
    const shown = st.case === "upper" ? draft.toUpperCase() : st.case === "lower" ? draft.toLowerCase() : draft;
    const natural = (textWidth(font, shown, ty.tracking) ?? 0) + ty.size * 0.35; // place du curseur
    width = Math.min(Math.max(natural, ty.size * 3), zoneWidth);
    x = ty.align === "center"
      ? zoneLeft + (zoneWidth - width) / 2
      : ty.align === "right"
        ? zoneLeft + zoneWidth - width
        : unplaced(f) ? zoneLeft : f.x * scale;
  }
  return {
    ...ty.css,
    left: x - PAD,
    top: top - PAD,
    width: width + 2 * PAD,
    minHeight: ty.line + 2 * PAD,
    padding: PAD,
    color: st.fill ?? undefined,
    // Le fond cache le texte rendu sous le brouillon, sur ses lignes seulement :
    // la marge déborderait sur les champs voisins (le titre sous son surtitre).
    backgroundColor: editorBackground(st.fill, st.bg),
    backgroundClip: "content-box",
  };
}
