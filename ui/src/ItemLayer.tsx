// Poignées des items de liste sur la scène : « + » entre les items, « × »
// et ⋮⋮ sur l'item survolé ; « + » au centre d'un objet dont la liste est
// vide. Montrées pour l'élément sélectionné ou en édition, sinon pour la slide.
// Chaque geste rend un changement de liste (`onListOp`) : la scène du deck
// l'applique au deck, l'aperçu d'un widget à ses valeurs d'exemple.

import { Fragment, useEffect, useRef, useState } from "react";
import type { ItemView, ListView, SlideView } from "./api";
import { newId } from "./deck";
import { t } from "./i18n";
import type { FieldKey } from "./stageFields";
import { fieldKey } from "./stageFields";
import type { Axis, OnListOp } from "./stageItems";
import { dropTarget, groupAxis, groups, insertPoints, itemName, moveItemTo, within } from "./stageItems";

/** Marge autour d'un item où ses poignées restent visibles (px écran). */
const HOVER_SLACK = 16;
/** Côté d'un « + » (px écran, voir `.item-insert`). */
const INSERT_SIZE = 24;

// Les ids semés (d0…) se répètent d'une liste à l'autre : la liste fait partie de la clé.
const handleKey = (it: ItemView) => `${it.list}/${it.item}`;

interface Drag {
  group: ItemView[];
  axis: Axis;
  it: ItemView;
  /** Position du pointeur le long de l'axe (pt). */
  at: number;
}

/** Ces poignées ne commencent ni sélection ni glisser d'objet en dessous. */
const stop = (e: React.PointerEvent) => e.stopPropagation();

export function ItemLayer({ view, scale, active, onListOp, onInserted }: {
  view: SlideView;
  scale: number;
  /** Élément sélectionné, ou propriétaire du champ en édition. */
  active: string | null;
  /** Un changement de liste ; rend éventuellement une promesse : `false` si l'hôte l'a refusé. */
  onListOp: OnListOp;
  /** Champ du nouvel item à ouvrir, dès qu'une vue le montre. */
  onInserted: (key: FieldKey) => void;
}) {
  const layer = useRef<HTMLDivElement>(null);
  const [hover, setHover] = useState<string | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const own = active === null ? [] : view.items.filter((it) => it.owner === active);
  const ownRef = useRef(own);
  ownRef.current = own;

  // L'item sous le pointeur, suivi sur toute la scène : la couche ne prend pas les clics.
  useEffect(() => {
    const stage = layer.current?.parentElement;
    if (!stage || active === null) return;
    const move = (e: PointerEvent) => {
      const r = layer.current?.getBoundingClientRect();
      if (!r) return;
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      // Distance au rectangle de l'item (0 dedans) : l'item sous le pointeur gagne sur un voisin proche.
      const gap = (it: ItemView) =>
        Math.max(it.x * scale - x, 0, x - (it.x + it.w) * scale) + Math.max(it.y * scale - y, 0, y - (it.y + it.h) * scale);
      const near = ownRef.current.filter((it) =>
        x >= it.x * scale - HOVER_SLACK && x <= (it.x + it.w) * scale + HOVER_SLACK &&
        y >= it.y * scale - HOVER_SLACK && y <= (it.y + it.h) * scale + HOVER_SLACK);
      const hit = near.reduce<ItemView | undefined>((best, it) => (best === undefined || gap(it) < gap(best) ? it : best), undefined);
      setHover(hit ? handleKey(hit) : null);
    };
    const leave = () => setHover(null);
    stage.addEventListener("pointermove", move);
    stage.addEventListener("pointerleave", leave);
    return () => {
      stage.removeEventListener("pointermove", move);
      stage.removeEventListener("pointerleave", leave);
    };
  }, [active, scale]);

  const px = (v: number) => v * scale;

  const insert = (l: Omit<ListView, "len" | "min" | "max">, index: number) => {
    const id = newId("i");
    const done = onListOp(l, l.list, { op: "insert", index, id }, t("ui-undo-list-add", { item: itemName(l) }));
    // Le champ du nouvel item ne s'ouvre qu'une fois l'insertion acceptée.
    void Promise.resolve(done).then((ok) => {
      if (ok !== false && l.first_text) onInserted(fieldKey({ owner: l.owner, field: `${l.list}/${id}/${l.first_text}` }));
    });
  };
  const move = (it: ItemView, to: number) => moveItemTo(onListOp, it, to);
  const along = (e: React.PointerEvent, axis: Axis) => {
    const r = layer.current?.getBoundingClientRect();
    return axis === "x" ? (e.clientX - (r?.left ?? 0)) / scale : (e.clientY - (r?.top ?? 0)) / scale;
  };
  const endDrag = () => {
    if (!drag) return;
    setDrag(null);
    const target = dropTarget(drag.group, drag.it, drag.axis, drag.at);
    if (target) move(drag.it, target.index);
  };

  const plus = (key: string, x: number, y: number, label: string, onClick: () => void) => (
    <button
      key={key}
      type="button"
      className="item-insert"
      data-item-insert
      // Centré sur son point, sans sortir de la page.
      style={{ left: within(px(x), px(view.width), INSERT_SIZE), top: within(px(y), px(view.height), INSERT_SIZE) }}
      aria-label={label}
      title={label}
      onPointerDown={stop}
      onClick={onClick}
    >
      +
    </button>
  );

  // Repère du glisser : le point d'insertion du rang où le lâcher mettrait l'item.
  const marker = (() => {
    const best = drag && dropTarget(drag.group, drag.it, drag.axis, drag.at);
    if (!drag || !best) return null;
    const size = drag.axis === "x" ? { width: 3, height: px(drag.it.h) } : { width: px(drag.it.w), height: 3 };
    return <span className="item-drop" style={{ left: px(best.x), top: px(best.y), ...size }} />;
  })();

  const empty = view.lists.filter((l) => l.owner === active && l.len === 0 && l.max > 0);

  return (
    <div className="items" ref={layer}>
      {groups(own).map((g) => {
        const first = g[0];
        const axis = groupAxis(view, g);
        const name = itemName(first);
        const full = first.len >= first.max;
        const removable = first.len > first.min;
        const moveLabel = t("ui-list-move", { item: name });
        const removeLabel = t("ui-list-remove", { item: name });
        const [back, forth] = axis === "x" ? ["ArrowLeft", "ArrowRight"] : ["ArrowUp", "ArrowDown"];
        return (
          <Fragment key={`${first.owner}\u001f${first.list}`}>
            {!full && insertPoints(g, axis).map((p) =>
              plus(`+${p.index}`, p.x, p.y, t("ui-list-add", { item: name }), () => insert(first, p.index)))}
            {g.map((it) => (
              <div
                key={it.item}
                className={`item-handles${hover === handleKey(it) || (drag !== null && handleKey(drag.it) === handleKey(it)) ? " shown" : ""}`}
                style={{ left: px(it.x), top: px(it.y), width: px(it.w), height: px(it.h) }}
              >
                {removable && (
                  <button
                    type="button"
                    className="item-remove"
                    data-item-remove
                    aria-label={removeLabel}
                    title={removeLabel}
                    onPointerDown={stop}
                    onClick={() => onListOp(it, it.list, { op: "remove", item: it.item }, t("ui-undo-list-remove", { item: name }))}
                  >
                    ×
                  </button>
                )}
                {it.len > 1 && (
                  <button
                    type="button"
                    className="item-move"
                    data-item-move
                    aria-label={moveLabel}
                    title={moveLabel}
                    onPointerDown={(e) => {
                      e.stopPropagation();
                      e.currentTarget.setPointerCapture?.(e.pointerId);
                      setDrag({ group: g, axis, it, at: along(e, axis) });
                    }}
                    onPointerMove={(e) => drag && setDrag({ ...drag, at: along(e, axis) })}
                    onPointerUp={endDrag}
                    onPointerCancel={() => setDrag(null)}
                    // Au clavier : les flèches de l'axe déplacent l'item d'un rang.
                    onKeyDown={(e) => {
                      const dir = e.key === forth ? 1 : e.key === back ? -1 : 0;
                      if (dir === 0) return;
                      e.preventDefault();
                      move(it, Math.max(0, Math.min(it.len - 1, it.index + dir)));
                    }}
                  >
                    ⋮⋮
                  </button>
                )}
              </div>
            ))}
          </Fragment>
        );
      })}
      {empty.map((l) => {
        const box = l.owner_kind === "element" ? view.targets.find((x) => x.id === l.owner) : { x: 0, y: 0, w: view.width, h: view.height };
        if (!box) return null;
        return plus(`${l.owner}\u001f${l.list}`, box.x + box.w / 2, box.y + box.h / 2, t("ui-list-add", { item: itemName(l) }), () => insert(l, 0));
      })}
      {marker}
    </div>
  );
}
