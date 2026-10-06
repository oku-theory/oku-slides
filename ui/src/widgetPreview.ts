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
 * champs et items du widget `widget` seulement, dans le repère de `box`
 * (pt), à sa taille ; la slide entière sans boîte. Pas de zone ; pour
 * cibles, l'élément d'aperçu seul, sur toute l'image : le cadre qui donne
 * l'axe d'un item seul de ses listes.
 */
export function croppedView(view: SlideView, box: Frame | null, widget: string): SlideView {
  const dx = box?.x ?? 0;
  const dy = box?.y ?? 0;
  return {
    ...view,
    width: box?.w ?? view.width,
    height: box?.h ?? view.height,
    targets: [...new Set(view.lists.filter((l) => l.owner_kind === "element").map((l) => l.owner))].map((id) => ({
      id, kind: widget, free: false, x: 0, y: 0, w: box?.w ?? view.width, h: box?.h ?? view.height,
    })),
    zone: null,
    items: view.items.map((it) => ({ ...it, x: it.x - dx, y: it.y - dy })),
    fields: view.fields
      .filter((f) => f.widget === widget)
      .map((f) => ({ ...f, x: f.x - dx, y: f.y - dy, ax: f.ax - dx })),
  };
}
