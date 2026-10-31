// Pile des miniatures : en colonne (mode pack), ou en bande sous la scène. On y
// choisit une slide, on la réordonne en la faisant glisser, et on la supprime
// depuis son coin.
//
// Le glisser est fait à la main (événements de pointeur) plutôt qu'avec le
// glisser-déposer HTML5 : dans l'app Tauri, la webview réserve ce dernier au
// dépôt de fichiers, et il ne se déclenche pas toujours.

import { useEffect, useRef, useState } from "react";
import type { SlideSummary } from "./api";
import { t } from "./i18n";
import { dropSlot, type PileAxis } from "./pileSlot";
import { Icon } from "./icons";
import { IconButton } from "./primitives";

interface Props {
  slides: SlideSummary[];
  current: string | null;
  /** URL du SVG d'une slide (null tant qu'il n'est pas rendu). */
  svg: (id: string, hash: string) => string | null;
  /** Noms affichés sous les miniatures (mode pack). */
  names?: Record<string, string>;
  label: string;
  addLabel: string;
  /** Historique : slide supprimée ou modifiée depuis la version montrée. */
  marks?: Record<string, "removed" | "changed">;
  onSelect: (id: string) => void;
  /** Déplace la slide à la place `slot` (0 = avant la première) ; absent : pile en lecture seule. */
  onMove?: (id: string, slot: number) => void;
  /** Absent : pas de bouton d'ajout (lecture seule). */
  onAdd?: () => void;
  /** The name of a thumbnail's delete button, the same for all or each its own. */
  removeLabel: string | ((id: string) => string);
  /** Faux s'il ne reste qu'une slide : on ne la supprime pas. */
  canRemove: boolean;
  /** Absent : pas de bouton de suppression (lecture seule). */
  onRemove?: (id: string) => void;
  /** Colonne (`y`, défaut) ou bande horizontale (`x`) : le sens du glisser et du défilement. */
  axis?: PileAxis;
  /** The title of every thumbnail of a pile that does not move (a pack's rename hint). */
  hint?: string;
  /** A key on a thumbnail, before the pile's own (Enter, Space); it may prevent them. */
  onThumbKey?: (id: string, e: React.KeyboardEvent<HTMLButtonElement>) => void;
  /** A double-click on a thumbnail. */
  onThumbDoubleClick?: (id: string) => void;
  /** A context menu asked on a thumbnail (right-click, menu key); absent: the browser's own. */
  onThumbContextMenu?: (id: string, e: React.MouseEvent<HTMLButtonElement>) => void;
  /** Slides whose item has typing not yet saved: they carry a dot. */
  unsaved?: ReadonlySet<string>;
}

const MARK_TITLE = { removed: "ui-history-removed-since", changed: "ui-history-changed-since" } as const;

interface Drag {
  id: string;
  pointer: number;
  /** Position du pointeur à l'appui, puis maintenant. */
  startX: number;
  startY: number;
  x: number;
  y: number;
  /** Décalage du pointeur dans l'image de la miniature, pour que le fantôme reste sous le doigt. */
  grabX: number;
  grabY: number;
  /** Position et largeur de l'image de la miniature soulevée. */
  left: number;
  top: number;
  width: number;
  moved: boolean;
  slot: number;
}

/** La corbeille du bouton de suppression. */
const RemoveIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    <path d="M4 7h16" /><path d="M9 7V4h6v3" /><path d="M6 7l1 13h10l1-13" /><path d="M10 11v5M14 11v5" />
  </svg>
);

/** Distance (px) avant qu'un appui devienne un glisser. */
const THRESHOLD = 5;

export default function SlidePile({
  slides, current, svg, names, label, addLabel, marks, onSelect, onMove, onAdd, removeLabel, canRemove, onRemove, axis = "y",
  hint, onThumbKey, onThumbDoubleClick, onThumbContextMenu, unsaved,
}: Props) {
  const nav = useRef<HTMLElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  dragRef.current = drag;

  /** Coordonnée du pointeur le long de la pile. */
  const along = (d: { x: number; y: number }) => (axis === "x" ? d.x : d.y);

  /** Place d'insertion sous le pointeur, le long de la pile. */
  const slotAt = (d: { x: number; y: number }) => {
    const el = nav.current;
    const thumbs = [...(el?.querySelectorAll<HTMLElement>(".thumb") ?? [])].map((b) => b.getBoundingClientRect());
    return dropSlot(thumbs, along(d), axis, !!el && getComputedStyle(el).direction === "rtl");
  };

  // Défilement automatique près des bords pendant un glisser.
  useEffect(() => {
    if (!drag?.moved) return;
    let raf = 0;
    const tick = () => {
      const d = dragRef.current;
      const el = nav.current;
      if (d && el) {
        const r = el.getBoundingClientRect();
        const [start, end] = axis === "x" ? [r.left, r.right] : [r.top, r.bottom];
        const at = along(d);
        const edge = 60;
        const speed = at < start + edge ? -(start + edge - at) / 4 : at > end - edge ? (at - (end - edge)) / 4 : 0;
        if (speed) {
          if (axis === "x") el.scrollLeft += speed;
          else el.scrollTop += speed;
          setDrag((x) => (x ? { ...x, slot: slotAt(x) } : x));
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [drag?.moved]);

  const down = (e: React.PointerEvent<HTMLButtonElement>, id: string) => {
    if (e.button !== 0) return;
    // Capture dès l'appui : le glisser continue même si le pointeur sort de la miniature.
    e.currentTarget.setPointerCapture(e.pointerId);
    const img = e.currentTarget.querySelector(".thumb-img")?.getBoundingClientRect() ?? e.currentTarget.getBoundingClientRect();
    setDrag({
      id,
      pointer: e.pointerId,
      startX: e.clientX,
      startY: e.clientY,
      x: e.clientX,
      y: e.clientY,
      grabX: e.clientX - img.left,
      grabY: e.clientY - img.top,
      left: img.left,
      top: img.top,
      width: img.width,
      moved: false,
      slot: -1,
    });
  };

  const move = (e: React.PointerEvent<HTMLButtonElement>) => {
    const d = dragRef.current;
    if (!d || d.pointer !== e.pointerId || !onMove) return;
    const now = { x: e.clientX, y: e.clientY };
    const moved = d.moved || Math.abs(along(now) - along({ x: d.startX, y: d.startY })) > THRESHOLD;
    setDrag({ ...d, ...now, moved, slot: moved ? slotAt(now) : -1 });
  };

  const up = (e: React.PointerEvent<HTMLButtonElement>) => {
    const d = dragRef.current;
    if (!d || d.pointer !== e.pointerId) return;
    setDrag(null);
    if (!d.moved) {
      onSelect(d.id);
      return;
    }
    const from = slides.findIndex((s) => s.id === d.id);
    // Déposer juste avant ou juste après soi-même ne change rien.
    if (d.slot !== from && d.slot !== from + 1) onMove?.(d.id, d.slot);
  };

  // La miniature de la slide courante reste visible : la pile défile jusqu'à elle (clavier, chevrons, agent).
  useEffect(() => {
    nav.current?.querySelector<HTMLElement>('.thumb[aria-current="true"]')?.scrollIntoView(axis === "x" ? { block: "nearest", inline: "nearest" } : { block: "nearest" });
  }, [current]);

  const dragged = drag?.moved ? slides.find((s) => s.id === drag.id) : undefined;
  const ghostUrl = dragged ? svg(dragged.id, dragged.hash) : null;

  return (
    <nav className={`pile${axis === "x" ? " horizontal" : ""}${drag?.moved ? " dragging" : ""}`} aria-label={label} ref={nav}>
      {slides.map((s, i) => {
        const url = svg(s.id, s.hash);
        const name = names?.[s.id];
        const lifted = drag?.moved && drag.id === s.id;
        const mark = marks?.[s.id];
        return (
          <div key={s.id} className="thumb-slot">
            {drag?.moved && drag.slot === i && <span className="drop-line" aria-hidden />}
            <button
              className={`thumb${lifted ? " lifted" : ""}`}
              aria-current={s.id === current}
              title={onMove ? t("ui-pile-drag-hint") : hint}
              onPointerDown={(e) => down(e, s.id)}
              onPointerMove={move}
              onPointerUp={up}
              onPointerCancel={() => setDrag(null)}
              onDoubleClick={onThumbDoubleClick && (() => onThumbDoubleClick(s.id))}
              onContextMenu={onThumbContextMenu && ((e) => onThumbContextMenu(s.id, e))}
              onKeyDown={(e) => {
                onThumbKey?.(s.id, e);
                if (e.defaultPrevented) return;
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelect(s.id);
                }
              }}
            >
              <span className="thumb-n">{i + 1}</span>
              <span className="thumb-body">
                {url ? <img className="thumb-img" src={url} alt={name ?? s.title ?? t("ui-slide-alt", { n: i + 1 })} draggable={false} /> : <span className="thumb-img" />}
                {name && <span className="thumb-name">{name}</span>}
                {unsaved?.has(s.id) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} title={t("ui-widgets-unsaved")} />}
                {mark && <span className={`thumb-mark ${mark}`} role="img" title={t(MARK_TITLE[mark])} aria-label={t(MARK_TITLE[mark])} />}
              </span>
            </button>
            {/* À côté de la miniature, pas dedans : un bouton ne peut pas en contenir un autre. */}
            {onRemove && !drag?.moved && (
              <button
                className="object token thumb-remove"
                aria-label={typeof removeLabel === "string" ? removeLabel : removeLabel(s.id)}
                title={typeof removeLabel === "string" ? t("ui-toolbar-remove-hint", { n: i + 1 }) : removeLabel(s.id)}
                disabled={!canRemove}
                onClick={() => onRemove(s.id)}
              >
                {RemoveIcon}
              </button>
            )}
          </div>
        );
      })}
      {drag?.moved && drag.slot === slides.length && <span className="drop-line end" aria-hidden />}
      {/* Un jeton rond, pictogramme seul : le libellé est son nom et son infobulle. */}
      {onAdd && <IconButton className="object token add-slide" label={addLabel} icon={Icon.add} onClick={onAdd} />}

      {/* La miniature suit le pointeur le long de la pile, soulevée. */}
      {drag?.moved && (
        <div
          className="thumb-ghost"
          aria-hidden
          style={axis === "x" ? { top: drag.top, left: drag.x - drag.grabX, width: drag.width } : { top: drag.y - drag.grabY, left: drag.left, width: drag.width }}
        >
          {ghostUrl ? <img src={ghostUrl} alt="" draggable={false} /> : <span />}
        </div>
      )}
    </nav>
  );
}
