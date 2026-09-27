// Pile des miniatures, à gauche de la table. On y choisit une slide, et on
// la réordonne en la faisant glisser, et on la supprime depuis son coin.
//
// Le glisser est fait à la main (événements de pointeur) plutôt qu'avec le
// glisser-déposer HTML5 : dans l'app Tauri, la webview réserve ce dernier au
// dépôt de fichiers, et il ne se déclenche pas toujours.

import { useEffect, useRef, useState } from "react";
import type { SlideSummary } from "./api";
import { t } from "./i18n";

interface Props {
  slides: SlideSummary[];
  current: string | null;
  /** URL du SVG d'une slide (null tant qu'il n'est pas rendu). */
  svg: (id: string, hash: string) => string | null;
  /** Noms affichés sous les miniatures (mode modèles). */
  names?: Record<string, string>;
  label: string;
  addLabel: string;
  onSelect: (id: string) => void;
  /** Déplace la slide à la place `slot` (0 = avant la première). */
  onMove: (id: string, slot: number) => void;
  onAdd: () => void;
  /** Nom du bouton de suppression d'une miniature. */
  removeLabel: string;
  /** Faux s'il ne reste qu'une slide : on ne la supprime pas. */
  canRemove: boolean;
  onRemove: (id: string) => void;
}

interface Drag {
  id: string;
  pointer: number;
  startY: number;
  y: number;
  /** Décalage du pointeur dans la miniature, pour que le fantôme reste sous le doigt. */
  grabX: number;
  grabY: number;
  /** Position et largeur de l'image de la miniature soulevée. */
  left: number;
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
  slides, current, svg, names, label, addLabel, onSelect, onMove, onAdd, removeLabel, canRemove, onRemove,
}: Props) {
  const nav = useRef<HTMLElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  dragRef.current = drag;

  /** Place d'insertion sous le pointeur : avant la miniature dont on survole la moitié haute. */
  const slotAt = (clientY: number) => {
    const thumbs = [...(nav.current?.querySelectorAll<HTMLElement>(".thumb") ?? [])];
    for (let i = 0; i < thumbs.length; i++) {
      const r = thumbs[i].getBoundingClientRect();
      if (clientY < r.top + r.height / 2) return i;
    }
    return thumbs.length;
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
        const edge = 60;
        const speed = d.y < r.top + edge ? -(r.top + edge - d.y) / 4 : d.y > r.bottom - edge ? (d.y - (r.bottom - edge)) / 4 : 0;
        if (speed) {
          el.scrollTop += speed;
          setDrag((x) => (x ? { ...x, slot: slotAt(x.y) } : x));
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
    const r = e.currentTarget.getBoundingClientRect();
    const img = e.currentTarget.querySelector(".thumb-img")?.getBoundingClientRect() ?? r;
    setDrag({
      id,
      pointer: e.pointerId,
      startY: e.clientY,
      y: e.clientY,
      grabX: e.clientX - r.left,
      grabY: e.clientY - img.top,
      left: img.left,
      width: img.width,
      moved: false,
      slot: -1,
    });
  };

  const move = (e: React.PointerEvent<HTMLButtonElement>) => {
    const d = dragRef.current;
    if (!d || d.pointer !== e.pointerId) return;
    const moved = d.moved || Math.abs(e.clientY - d.startY) > THRESHOLD;
    setDrag({ ...d, y: e.clientY, moved, slot: moved ? slotAt(e.clientY) : -1 });
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
    if (d.slot !== from && d.slot !== from + 1) onMove(d.id, d.slot);
  };

  const dragged = drag?.moved ? slides.find((s) => s.id === drag.id) : undefined;
  const ghostUrl = dragged ? svg(dragged.id, dragged.hash) : null;

  return (
    <nav className={`pile${drag?.moved ? " dragging" : ""}`} aria-label={label} ref={nav}>
      {slides.map((s, i) => {
        const url = svg(s.id, s.hash);
        const name = names?.[s.id];
        const lifted = drag?.moved && drag.id === s.id;
        return (
          <div key={s.id} className="thumb-slot">
            {drag?.moved && drag.slot === i && <span className="drop-line" aria-hidden />}
            <button
              className={`thumb${lifted ? " lifted" : ""}`}
              aria-current={s.id === current}
              title={t("ui-pile-drag-hint")}
              onPointerDown={(e) => down(e, s.id)}
              onPointerMove={move}
              onPointerUp={up}
              onPointerCancel={() => setDrag(null)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelect(s.id);
                }
              }}
            >
              <span className="thumb-n">{i + 1}</span>
              <span className="thumb-body">
                {url ? <img className="thumb-img" src={url} alt={s.title ?? t("ui-slide-alt", { n: i + 1 })} draggable={false} /> : <span className="thumb-img" />}
                {name && <span className="thumb-name">{name}</span>}
              </span>
            </button>
            {/* À côté de la miniature, pas dedans : un bouton ne peut pas en contenir un autre. */}
            {!drag?.moved && (
              <button
                className="object token thumb-remove"
                aria-label={removeLabel}
                title={t("ui-toolbar-remove-hint", { n: i + 1 })}
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
      <button className="object btn add-slide" onClick={onAdd}>{addLabel}</button>

      {/* La miniature suit le pointeur, soulevée de la pile. */}
      {drag?.moved && (
        <div className="thumb-ghost" aria-hidden style={{ top: drag.y - drag.grabY, left: drag.left, width: drag.width }}>
          {ghostUrl ? <img src={ghostUrl} alt="" draggable={false} /> : <span />}
        </div>
      )}
    </nav>
  );
}
