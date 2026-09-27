// La scène : le SVG rendu par Typst, et par-dessus la couche de sélection.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Frame, Op, SlideView, Target } from "./api";
import { fontStack } from "./appearance";
import { editorBackground } from "./color";
import type { GridPlacement } from "./deck";
import { findElement, gridPlacement, isBlock, placeOnGridOps, textChanged, textOps } from "./deck";
import type { Cell, GridSpec, ZoneView } from "./grid";
import { cellLabel, cellRect, moveCell, resizeCell, sameCell, tracks } from "./grid";
import { t } from "./i18n";
import type { TextEditorHandle } from "./TextEditor";
import TextEditor from "./TextEditor";

type Mode = "move" | "nw" | "ne" | "sw" | "se";

interface Drag {
  id: string;
  mode: Mode;
  startX: number;
  startY: number;
  frame: Frame;
  dx: number;
  dy: number;
  /** Sur la grille : la place au départ ; l'élément saute de case en case. */
  place?: GridPlacement;
}

/** Cellule visée par un glisser sur la grille. */
function draggedCell(d: Drag): Cell | null {
  if (!d.place) return null;
  const { zone, grid, cell } = d.place;
  return d.mode === "move" ? moveCell(zone, grid, cell, d.dx, d.dy) : resizeCell(zone, grid, cell, d.mode, d.dx, d.dy);
}

interface Props {
  view: SlideView;
  svgUrl: string | null;
  selected: string | null;
  editing: string | null;
  onSelect: (id: string | null) => void;
  onEdit: (id: string | null) => void;
  onApply: (label: string, ops: Op[]) => void;
  /** Texte dicté à insérer au curseur de l'éditeur ouvert. */
  dictated?: { seq: number; text: string } | null;
  /** Grille affichée (sinon : seulement pendant un glisser). */
  showGridAlways?: boolean;
  /** Couleurs des pastilles dans l'éditeur (palette des graphiques du thème). */
  palette: string[];
}

/** Nom d'un champ de slide (titre, sous-titre…) ; un champ inconnu garde son identifiant. */
export const fieldLabel = (kind: string) => t("ui-field", { kind });

/** Nom d'une sorte d'élément (texte, carte…) ; une sorte inconnue garde son identifiant. */
const kindLabel = (kind: string) => t("ui-element-kind", { kind });

/** Cadre de l'élément après un glisser (en pt). Sur la grille, il saute de nœud en nœud. */
function dragged(d: Drag): Frame {
  const cell = draggedCell(d);
  if (cell && d.place) return cellRect(d.place.zone, d.place.grid, cell);
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

const WEIGHTS: Record<string, number> = {
  thin: 100, extralight: 200, light: 300, regular: 400, medium: 500, semibold: 600, bold: 700, extrabold: 800, black: 900,
};

let measureCtx: CanvasRenderingContext2D | null = null;

/** Largeur (px) de la plus longue ligne, dans la police donnée. */
function textWidth(text: string, font: string, tracking: number): number {
  measureCtx ??= document.createElement("canvas").getContext("2d");
  if (!measureCtx) return 0;
  measureCtx.font = font;
  return Math.max(0, ...text.split("\n").map((l) => measureCtx!.measureText(l).width + tracking * l.length));
}

/**
 * Éditeur posé exactement sur le texte rendu : même police, taille, graisse,
 * couleur, interlignage et alignement, première ligne sur la même ligne de base.
 * Un champ (titre…) épouse son texte et s'élargit en tapant, jusqu'à sa zone ;
 * un bloc garde la largeur de sa boîte, où le texte revient à la ligne.
 */
function editorGeometry(t: Target, draft: string, scale: number) {
  const st = t.style;
  const pad = 5; // marge intérieure (px écran) : le cadre ne colle pas aux lettres
  if (!st) {
    const left = t.x * scale - pad;
    return {
      left,
      style: { left, top: t.y * scale - pad, width: Math.max(t.w * scale, 240) + 2 * pad, minHeight: t.h * scale + 2 * pad, padding: pad } as React.CSSProperties,
    };
  }
  const size = st.size * scale;
  // Typst : interligne = hauteur de capitale + espacement ; en CSS, hauteur de ligne.
  const line = Math.max(st.size * 0.7 + st.leading, st.size * 1.05) * scale;
  // Ligne de base de la première ligne : Typst la place à y + hauteur de capitale ;
  // CSS à mi-interligne + (ascendante − descendante) / 2.
  const top = t.y * scale + size * 0.7 - line / 2 - size * 0.325;
  const weight = typeof st.weight === "number" ? st.weight : WEIGHTS[String(st.weight).replace(/[^a-z]/g, "")] ?? 400;
  const family = fontStack(st.font, "serif");
  const tracking = st.tracking * scale;
  const align = st.align ?? "left";

  let width: number;
  if (t.target === "element") {
    width = t.w * scale;
  } else {
    const shown = st.case === "upper" ? draft.toUpperCase() : st.case === "lower" ? draft.toLowerCase() : draft;
    const natural = textWidth(shown, `${weight} ${size}px ${family}`, tracking) + size * 0.35; // place du curseur
    width = Math.min(Math.max(natural, size * 3), t.aw * scale);
  }
  const zoneLeft = t.ax * scale;
  const zoneWidth = t.aw * scale;
  const x = t.target === "element"
    ? t.x * scale
    : align === "center"
      ? zoneLeft + (zoneWidth - width) / 2
      : align === "right"
        ? zoneLeft + zoneWidth - width
        : t.x * scale;
  const left = x - pad;
  return {
    left,
    style: {
      left,
      top: top - pad,
      width: width + 2 * pad,
      minHeight: line + 2 * pad,
      padding: pad,
      fontFamily: family,
      fontSize: size,
      fontWeight: weight,
      lineHeight: `${line}px`,
      letterSpacing: tracking || undefined,
      color: st.fill ?? undefined,
      background: editorBackground(st.fill, st.bg),
      textAlign: align,
      textTransform: st.case === "upper" ? "uppercase" : st.case === "lower" ? "lowercase" : undefined,
    } as React.CSSProperties,
  };
}

/** La grille : un petit point sur chaque nœud (coins des cases), et la place visée. */
function GridLayer({ zone, grid, scale, target }: { zone: ZoneView; grid: GridSpec; scale: number; target: Cell | null }) {
  const t = tracks(zone, grid);
  const px = (v: number) => v * scale;
  const hot = target ? cellRect(zone, grid, target) : null;
  // Nœuds : là où les bords des colonnes croisent ceux des lignes ; c'est là que tout s'accroche.
  const xs = t.cols.flatMap((c) => [c.start, c.end]).map(px);
  const ys = t.rows.flatMap((r) => [r.start, r.end]).map(px);
  return (
    <svg className="grid-layer" aria-hidden width={px(zone.w)} height={px(zone.h)} style={{ left: px(zone.x), top: px(zone.y) }}>
      {hot && <rect className="grid-hot" x={px(hot.x)} y={px(hot.y)} width={px(hot.w)} height={px(hot.h)} rx={4} />}
      {ys.map((y, i) => xs.map((x, j) => <circle key={`${i}-${j}`} className="grid-node" cx={x} cy={y} r={1.6} />))}
    </svg>
  );
}

export default function Stage({ view, svgUrl, selected, editing, onSelect, onEdit, onApply, dictated, showGridAlways = false, palette }: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const editor = useRef<TextEditorHandle>(null);
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
  // Brouillon à l'ouverture, avant l'affichage : il ne sert qu'à la largeur d'un champ.
  // Il ne suit pas le serveur pendant l'édition : CodeMirror est la source de vérité.
  useLayoutEffect(() => {
    setDraft(editTarget?.text ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing]);

  // La dictée s'insère au curseur de l'éditeur ouvert.
  useEffect(() => {
    if (dictated) editor.current?.insertAtCursor(dictated.text);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dictated?.seq]);

  const startDrag = (e: React.PointerEvent, t: Target, mode: Mode) => {
    e.stopPropagation();
    onSelect(t.id);
    if (editing) return;
    // Sur une slide à grille, un bloc (depuis sa place à l'écran) ou un
    // élément libre se déplace de case en case : l'accrochage est automatique.
    const place = gridPlacement(view, t.id);
    const frame = place?.frame ?? (t.free ? findElement(view.slide, t.id)?.frame : undefined);
    if (!frame) return;
    (e.target as Element).setPointerCapture(e.pointerId);
    setDrag({ id: t.id, mode, startX: e.clientX, startY: e.clientY, frame, dx: 0, dy: 0, place: place ?? undefined });
  };

  const moveDrag = (e: React.PointerEvent) => {
    if (!drag) return;
    setDrag({ ...drag, dx: (e.clientX - drag.startX) / scale, dy: (e.clientY - drag.startY) / scale });
  };

  const endDrag = () => {
    if (!drag) return;
    const cell = draggedCell(drag);
    const place = drag.place;
    if (cell && place) {
      setDrag(null);
      const moved = Math.abs(drag.dx) > 2 || Math.abs(drag.dy) > 2;
      // Un élément qui n'est pas encore sur une case s'y accroche dès qu'il bouge.
      if (!sameCell(cell, place.cell) || (moved && !place.onCell)) {
        const label = t(place.block
          ? drag.mode === "move" ? "ui-undo-place-on-grid" : "ui-undo-resize-on-grid"
          : drag.mode === "move" ? "ui-undo-move-on-grid" : "ui-undo-change-anchor");
        onApply(label, placeOnGridOps(view, drag.id, cell));
      }
      return;
    }
    const f = dragged(drag);
    const o = drag.frame;
    setDrag(null);
    if (f.x !== o.x || f.y !== o.y || f.w !== o.w || f.h !== o.h) {
      const label = t(drag.mode === "move" ? "ui-undo-move" : "ui-undo-resize");
      onApply(label, [{ op: "set_frame", slide: view.slide.id, element: drag.id, frame: f }]);
    }
  };

  const commit = (text: string) => {
    if (!editTarget) return;
    if (textChanged(editTarget, text)) {
      onApply(t("ui-undo-edit-text"), textOps(view, editTarget, text));
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

  // La grille se montre quand on travaille avec : élément accroché choisi, ou glisser en cours.
  const zone = view.zone;
  const grid = zone?.grid ?? null;
  const selectedEl = selected ? findElement(view.slide, selected) : undefined;
  const liveCell = drag ? draggedCell(drag) : null;
  // Affichée à la demande ; pendant un glisser sur la grille, toujours (on voit où l'on pose).
  const showGrid = !!grid && (showGridAlways || !!liveCell);
  const tagFor = (t: Target, label: string) => {
    const el = t.target === "element" ? findElement(view.slide, t.id) : undefined;
    const cell = drag?.id === t.id && liveCell ? liveCell : el?.cell;
    return cell && grid ? `${label} · ${cellLabel(cell)}` : label;
  };

  return (
    // La slide garde ses coordonnées de gauche à droite, même dans une interface en arabe.
    <div className="stage-wrap" dir="ltr" ref={wrap} onPointerDown={() => editing || onSelect(null)}>
      <div
        className="stage"
        style={{ width: Math.floor(view.width * scale), height: Math.floor(view.height * scale) }}
        onPointerMove={moveDrag}
        onPointerUp={endDrag}
        onPointerCancel={() => setDrag(null)}
      >
        {svgUrl && <img src={svgUrl} alt={t("ui-slide-alt", { n: view.page })} draggable={false} />}
        {showGrid && zone && grid && <GridLayer zone={zone} grid={grid} scale={scale} target={liveCell ?? selectedEl?.cell ?? null} />}
        <div className="overlay" role="group" aria-label={t("ui-stage-elements")}>
          {ordered.map((tg) => {
            const sel = tg.id === selected;
            const label = tg.target === "field" ? fieldLabel(tg.kind) : kindLabel(tg.kind);
            return (
              <div
                key={tg.id}
                className={`target${tg.free || (!!grid && isBlock(view, tg.id)) ? " free" : ""}${sel ? " selected" : ""}`}
                style={box(tg)}
                aria-label={label}
                aria-selected={sel}
                onPointerDown={(e) => startDrag(e, tg, "move")}
                onDoubleClick={() => tg.text !== null && onEdit(tg.id)}
              >
                {sel && !editing && <span className="tag">{tagFor(tg, label)}</span>}
                {sel && (tg.free || (!!grid && isBlock(view, tg.id))) &&
                  (["nw", "ne", "sw", "se"] as Mode[]).map((m) => (
                    <span key={m} className={`handle ${m}`} onPointerDown={(e) => startDrag(e, tg, m)} />
                  ))}
              </div>
            );
          })}
        </div>
        {editTarget && (() => {
          const g = editorGeometry(editTarget, draft, scale);
          return (
            <>
              {/* Cache le texte rendu dessous : seul le brouillon se voit. */}
              <span
                className="editor-mask"
                style={{ left: editTarget.x * scale - 2, top: editTarget.y * scale - 2, width: editTarget.w * scale + 4, height: editTarget.h * scale + 4, background: editorBackground(editTarget.style?.fill ?? null, editTarget.style?.bg ?? null) }}
              />
              <TextEditor
                key={editTarget.id}
                ref={editor}
                initial={editTarget.text ?? ""}
                field={editTarget.target === "field"}
                palette={palette}
                label={t("ui-stage-edit-text")}
                style={g.style}
                onChange={setDraft}
                onCommit={commit}
                onCancel={() => onEdit(null)}
              />
              <span className="editor-hint">
                {t(editTarget.target === "field" ? "ui-stage-hint-field" : "ui-stage-hint-block")}
              </span>
            </>
          );
        })()}
      </div>
    </div>
  );
}
