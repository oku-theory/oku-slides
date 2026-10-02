// Les fantômes de la scène : contour annoté de la zone de corps, de chaque
// objet (sorte et case de grille) et de chaque champ (nom du paramètre).

import type { SlideView } from "./api";
import { findElement } from "./deck";
import { t } from "./i18n";
import { elementTag, fieldHitBox, fieldKey, fieldLabel } from "./stageFields";

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Place (px) d'une étiquette hors de sa boîte ; sans elle, la scène la rognerait. */
const TAG_ROOM = 20;

/** Un contour et son étiquette ; `inside` : l'étiquette rentre dans la boîte, au bord de la slide. */
function Ghost({ kind, box, tag, inside }: { kind: "zone" | "target" | "field"; box: Box; tag: string; inside: boolean }) {
  return (
    <div className={`stage-ghost stage-ghost-${kind}${inside ? " stage-ghost-in" : ""}`} style={box}>
      <span className="stage-ghost-tag">{tag}</span>
    </div>
  );
}

interface Props {
  view: SlideView;
  scale: number;
  /** Objet en cours de glisser : son fantôme, resté à l'ancienne place, se tait. */
  hidden: string | null;
}

/** Couche des fantômes : au-dessus du rendu, sous les objets, les champs et l'éditeur. */
export default function GhostLayer({ view, scale, hidden }: Props) {
  const px = (r: { x: number; y: number; w: number; h: number }): Box => ({
    left: r.x * scale, top: r.y * scale, width: r.w * scale, height: r.h * scale,
  });
  // Étiquettes au-dessus des boîtes ; celle de la zone, dessous.
  const above = (b: Box) => b.top < TAG_ROOM;
  const below = (b: Box) => b.top + b.height + TAG_ROOM > view.height * scale;
  const zone = view.zone ? px(view.zone) : null;
  const grid = view.zone?.grid ?? null;
  return (
    <div className="stage-ghost-layer" aria-hidden>
      {zone && <Ghost kind="zone" box={zone} tag={t("ui-ghost-zone")} inside={below(zone)} />}
      {view.targets.filter((tg) => tg.id !== hidden).map((tg) => {
        const box = px(tg);
        const tag = elementTag(tg.kind, grid ? findElement(view.slide, tg.id)?.cell : null);
        return <Ghost key={tg.id} kind="target" box={box} tag={tag} inside={above(box)} />;
      })}
      {view.fields.map((f) => {
        const box = fieldHitBox(f, scale, view.fields);
        return <Ghost key={fieldKey(f)} kind="field" box={box} tag={fieldLabel(f)} inside={above(box)} />;
      })}
    </div>
  );
}
