// Aperçu d'un widget : recadré sur ce que dessine un widget d'élément.

import type { Frame } from "./api";

/** Le SVG de la slide d'aperçu, cadré au plus près sur `box` (pt) ; tel quel sans boîte. */
export function croppedSvg(svg: string, box: Frame | null): string {
  if (!box) return svg;
  const [x, y, w, h] = [box.x, box.y, box.w, box.h].map((v) => Math.round(v * 100) / 100);
  return svg.replace(/<svg\b[^>]*>/, (tag) =>
    tag
      .replace(/\sviewBox="[^"]*"/, ` viewBox="${x} ${y} ${w} ${h}"`)
      .replace(/\swidth="[^"]*"/, ` width="${w}pt"`)
      .replace(/\sheight="[^"]*"/, ` height="${h}pt"`),
  );
}
