// Le contenu d'un élément pendant son déplacement sur la scène : l'image de la
// slide recadrée sur sa place d'origine suit le pointeur, et cette place s'estompe.
// Le recadrage emporte aussi le fond de la slide et ce qui chevauche l'élément ;
// le voile d'origine prend la couleur de fond du thème, pas une image de fond.

import type { ObjectTarget } from "./api";

interface Props {
  /** Image rendue de toute la slide. */
  svgUrl: string;
  /** L'élément déplacé, à sa place d'origine (en pt). */
  target: ObjectTarget;
  scale: number;
  /** Taille de la scène (px), celle de l'image de la slide. */
  size: { width: number; height: number };
}

/** Fantôme du contenu, posé dans la boîte qui suit le pointeur. */
export function DragGhost({ svgUrl, target, scale, size }: Props) {
  return (
    <div className="drag-ghost" aria-hidden style={{ width: target.w * scale, height: target.h * scale }}>
      <img src={svgUrl} alt="" draggable={false} style={{ left: -target.x * scale, top: -target.y * scale, ...size }} />
    </div>
  );
}

/** Voile sur la place d'origine du contenu déplacé. */
export function DragOrigin({ target, scale }: Pick<Props, "target" | "scale">) {
  const { x, y, w, h } = target;
  return <span className="drag-origin" style={{ left: x * scale, top: y * scale, width: w * scale, height: h * scale }} />;
}
