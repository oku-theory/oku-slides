// Aperçu d'un widget : recadré sur ce que dessine un widget d'élément.

import type { Frame, SlideView } from "./api";

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

/**
 * La vue de la slide d'aperçu sur l'image recadrée (`croppedSvg`) : les
 * champs du widget `widget` seulement, dans le repère de `box` (pt), à sa
 * taille ; la slide entière sans boîte. Ni cibles ni zone : la scène en
 * aperçu n'a que ses champs.
 */
export function croppedView(view: SlideView, box: Frame | null, widget: string): SlideView {
  const dx = box?.x ?? 0;
  const dy = box?.y ?? 0;
  return {
    ...view,
    width: box?.w ?? view.width,
    height: box?.h ?? view.height,
    targets: [],
    zone: null,
    fields: view.fields
      .filter((f) => f.widget === widget)
      .map((f) => ({ ...f, x: f.x - dx, y: f.y - dy, ax: f.ax - dx })),
  };
}
