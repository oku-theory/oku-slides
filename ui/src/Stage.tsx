// La scène : le SVG rendu par Typst, et par-dessus la couche de sélection.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Frame, Op, SlideView, Target } from "./api";
import { findElement, textToParagraphs } from "./api";

type Mode = "move" | "nw" | "ne" | "sw" | "se";

interface Drag {
  id: string;
  mode: Mode;
  startX: number;
  startY: number;
  frame: Frame;
  dx: number;
  dy: number;
}

interface Props {
  view: SlideView;
  svgUrl: string | null;
  selected: string | null;
  editing: string | null;
  onSelect: (id: string | null) => void;
  onEdit: (id: string | null) => void;
  onApply: (label: string, ops: Op[]) => void;
}

const FIELD_LABELS: Record<string, string> = {
  title: "Titre",
  subtitle: "Sous-titre",
  kicker: "Surtitre",
  text: "Texte",
};

const KIND_LABELS: Record<string, string> = {
  text: "Texte",
  card: "Carte",
  image: "Image",
  shape: "Forme",
  group: "Groupe",
};

/** Cadre de l'élément après un glisser (en pt). */
function dragged(d: Drag): Frame {
  const f = { ...d.frame };
  const min = 20;
  switch (d.mode) {
    case "move":
      f.x += d.dx;
      f.y += d.dy;
      break;
    case "se":
      f.w = Math.max(min, f.w + d.dx);
      f.h = Math.max(min, f.h + d.dy);
      break;
    case "sw":
      f.x += Math.min(d.dx, f.w - min);
      f.w = Math.max(min, f.w - d.dx);
      f.h = Math.max(min, f.h + d.dy);
      break;
    case "ne":
      f.y += Math.min(d.dy, f.h - min);
      f.w = Math.max(min, f.w + d.dx);
      f.h = Math.max(min, f.h - d.dy);
      break;
    case "nw":
      f.x += Math.min(d.dx, f.w - min);
      f.y += Math.min(d.dy, f.h - min);
      f.w = Math.max(min, f.w - d.dx);
      f.h = Math.max(min, f.h - d.dy);
      break;
  }
  return { x: Math.round(f.x), y: Math.round(f.y), w: Math.round(f.w), h: Math.round(f.h) };
}

/** Ops pour remplacer le texte d'une cible. */
export function textOps(view: SlideView, t: Target, value: string): Op[] {
  const slide = view.slide;
  if (t.target === "element") {
    const e = findElement(slide, t.id);
    return [{ op: "set_text", slide: slide.id, element: t.id, text: textToParagraphs(value, e?.text ?? []) }];
  }
  const field = t.kind;
  const v = value.trim() === "" ? null : value;
  // Sur la couverture, titre et sous-titre viennent des paramètres du deck.
  if (slide.kind === "title" && (field === "title" || field === "subtitle") && slide[field] === undefined) {
    return [{ op: "set_param", key: field, value: v }];
  }
  return [{ op: "set_slide_field", slide: slide.id, field, value: v }];
}

export default function Stage({ view, svgUrl, selected, editing, onSelect, onEdit, onApply }: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(0.5);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [draft, setDraft] = useState("");

  useLayoutEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const fit = () => {
      const s = Math.min(el.clientWidth / view.width, el.clientHeight / view.height);
      setScale(Math.max(0.1, s));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [view.width, view.height]);

  const editTarget = view.targets.find((t) => t.id === editing) ?? null;
  useEffect(() => {
    setDraft(editTarget?.text ?? "");
  }, [editing, editTarget?.text]);

  const startDrag = (e: React.PointerEvent, t: Target, mode: Mode) => {
    e.stopPropagation();
    onSelect(t.id);
    if (!t.free || editing) return;
    const el = findElement(view.slide, t.id);
    if (!el?.frame) return;
    (e.target as Element).setPointerCapture(e.pointerId);
    setDrag({ id: t.id, mode, startX: e.clientX, startY: e.clientY, frame: el.frame, dx: 0, dy: 0 });
  };

  const moveDrag = (e: React.PointerEvent) => {
    if (!drag) return;
    setDrag({ ...drag, dx: (e.clientX - drag.startX) / scale, dy: (e.clientY - drag.startY) / scale });
  };

  const endDrag = () => {
    if (!drag) return;
    const f = dragged(drag);
    const o = drag.frame;
    setDrag(null);
    if (f.x !== o.x || f.y !== o.y || f.w !== o.w || f.h !== o.h) {
      const label = drag.mode === "move" ? "Déplacer" : "Redimensionner";
      onApply(label, [{ op: "set_frame", slide: view.slide.id, element: drag.id, frame: f }]);
    }
  };

  const commit = () => {
    if (!editTarget) return;
    if (draft !== (editTarget.text ?? "")) {
      onApply("Modifier le texte", textOps(view, editTarget, draft));
    }
    onEdit(null);
  };

  const box = (t: Target) => {
    let { x, y, w, h } = t;
    if (drag && drag.id === t.id) {
      const f = dragged(drag);
      x += f.x - drag.frame.x;
      y += f.y - drag.frame.y;
      w += f.w - drag.frame.w;
      h += f.h - drag.frame.h;
    }
    return { left: x * scale, top: y * scale, width: Math.max(w * scale, 8), height: Math.max(h * scale, 8) };
  };

  // Les petites boîtes au-dessus des grandes, pour pouvoir les atteindre.
  const ordered = [...view.targets].sort((a, b) => b.w * b.h - a.w * a.h);

  return (
    <div className="stage-wrap" ref={wrap} onPointerDown={() => editing || onSelect(null)}>
      <div
        className="stage"
        style={{ width: Math.floor(view.width * scale), height: Math.floor(view.height * scale) }}
        onPointerMove={moveDrag}
        onPointerUp={endDrag}
        onPointerCancel={() => setDrag(null)}
      >
        {svgUrl && <img src={svgUrl} alt={`Slide ${view.page}`} draggable={false} />}
        <div className="overlay" role="group" aria-label="Éléments de la slide">
          {ordered.map((t) => {
            const sel = t.id === selected;
            const label = t.target === "field" ? FIELD_LABELS[t.kind] ?? t.kind : KIND_LABELS[t.kind] ?? t.kind;
            return (
              <div
                key={t.id}
                className={`target${t.free ? " free" : ""}${sel ? " selected" : ""}`}
                style={box(t)}
                aria-label={label}
                aria-selected={sel}
                onPointerDown={(e) => startDrag(e, t, "move")}
                onDoubleClick={() => t.text !== null && onEdit(t.id)}
              >
                {sel && <span className="tag">{label}</span>}
                {sel && t.free &&
                  (["nw", "ne", "sw", "se"] as Mode[]).map((m) => (
                    <span key={m} className={`handle ${m}`} onPointerDown={(e) => startDrag(e, t, m)} />
                  ))}
              </div>
            );
          })}
        </div>
        {editTarget && (
          <>
            <textarea
              className="editor"
              autoFocus
              aria-label="Modifier le texte"
              style={{
                left: editTarget.x * scale,
                top: editTarget.y * scale,
                width: Math.max(editTarget.w * scale + 24, 320),
                height: Math.max(editTarget.h * scale + 24, 64),
              }}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onPointerDown={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  onEdit(null);
                } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey || editTarget.target === "field")) {
                  e.preventDefault();
                  commit();
                }
                e.stopPropagation();
              }}
              onBlur={commit}
            />
            <span
              className="editor-hint"
              style={{ left: editTarget.x * scale, top: editTarget.y * scale - 26 }}
            >
              {editTarget.target === "field" ? "Entrée pour valider · Échap pour annuler" : "⌘ Entrée pour valider · une ligne par paragraphe, « - » pour une puce"}
            </span>
          </>
        )}
      </div>
    </div>
  );
}
