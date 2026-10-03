// Aperçu d'un widget : recadré sur ce que dessine un widget d'élément.

import type { Frame } from "./api";

/** Marge autour de la boîte, en part de son plus grand côté. */
const MARGIN_RATIO = 0.08;
/** Marge minimale, en pt. */
const MARGIN_MIN_PT = 12;

/** Le SVG de la slide d'aperçu, cadré sur `box` (pt) avec une marge ; tel quel sans boîte. */
export function croppedSvg(svg: string, box: Frame | null): string {
  if (!box) return svg;
  const m = Math.max(MARGIN_MIN_PT, MARGIN_RATIO * Math.max(box.w, box.h));
  const [x, y, w, h] = [box.x - m, box.y - m, box.w + 2 * m, box.h + 2 * m].map((v) => Math.round(v * 100) / 100);
  return svg.replace(/<svg\b[^>]*>/, (tag) =>
    tag
      .replace(/\sviewBox="[^"]*"/, ` viewBox="${x} ${y} ${w} ${h}"`)
      .replace(/\swidth="[^"]*"/, ` width="${w}pt"`)
      .replace(/\sheight="[^"]*"/, ` height="${h}pt"`),
  );
}
