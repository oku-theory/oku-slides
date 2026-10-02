// La scène : le SVG rendu par Typst, et par-dessus la couche des objets
// (sélection, glisser) et celle des champs (édition en place).

import { useEffect, useImperativeHandle, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FieldView, Frame, ObjectTarget, Op, SlideView } from "./api";
import { editorBackground } from "./color";
import type { GridPlacement } from "./deck";
import { fieldChanged, fieldOps, findElement, gridPlacement, isBlock, placeOnGridOps } from "./deck";
import { editorGeometry, placeholderStyle } from "./editorGeometry";
import type { Cell, GridSpec, ZoneView } from "./grid";
import { cellRect, moveCell, resizeCell, sameCell, tracks } from "./grid";
import { t } from "./i18n";
import { sheetPosition } from "./agent/quick";
import type { TextEditorHandle } from "./TextEditor";
import TextEditor from "./TextEditor";
import type { FieldKey } from "./stageFields";
import { elementTag, fieldHitBox, fieldKey, fieldLabel, findField, kindLabel, nextField, primaryFieldOf } from "./stageFields";
import GhostLayer from "./GhostLayer";

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

/** Ce que l'application peut demander à la scène. */
export interface StageHandle {
  /** Enregistre la saisie ouverte (dans sa session) et ferme l'éditeur ; avant de quitter la slide. */
  commitEditing(): void;
}

interface Props {
  /** Poignée de l'application : valider la saisie avant de changer de slide. */
  ref?: React.Ref<StageHandle>;
  view: SlideView;
  svgUrl: string | null;
  selected: string | null;
  /** Champ en édition. */
  editing: FieldKey | null;
  onSelect: (id: string | null) => void;
  onEdit: (key: FieldKey | null) => void;
  /** `merge` : clé de la session de saisie ; ses applications font une seule étape d'annulation. */
  onApply: (label: string, ops: Op[], merge?: string) => void;
  /** Texte dicté à insérer au curseur de l'éditeur ouvert. */
  dictated?: { seq: number; text: string } | null;
  /** Grille affichée (sinon : seulement pendant un glisser). */
  showGridAlways?: boolean;
  /** Posé sous l'élément `target` (fiche d'une question de l'agent), s'il est sur la slide ; `@param` : sous ce champ de la slide. */
  overlay?: { target: string; node: ReactNode };
  /** Couleurs des pastilles dans l'éditeur (palette des graphiques du thème). */
  palette: string[];
  /** Lecture seule (historique) : pas de couche des champs. */
  readOnly?: boolean;
  /** Mode modèles : les placeholders de tous les champs vides se voient. */
  templates?: boolean;
  /** Fantômes : zone, objets et champs annotés ; placeholders de tous les champs vides. */
  showGhosts?: boolean;
  /** Message pour l'utilisateur (le champ en édition a disparu). */
  onNotice?: (msg: string) => void;
}

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

export default function Stage({
  ref, view, svgUrl, selected, editing, onSelect, onEdit, onApply, dictated, showGridAlways = false, overlay, palette,
  readOnly = false, templates = false, showGhosts = false, onNotice,
}: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const editor = useRef<TextEditorHandle>(null);
  const [scale, setScale] = useState(0.5);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [draft, setDraft] = useState("");
  // Une police qui finit de charger change les métriques de l'éditeur (mesurées
  // jusque-là sur la police de repli) : la scène se redessine.
  const [, setFontsLoaded] = useState(0);
  useEffect(() => {
    const fonts = typeof document === "undefined" ? undefined : document.fonts;
    if (!fonts?.addEventListener) return;
    const relayout = () => setFontsLoaded((n) => n + 1);
    fonts.addEventListener("loadingdone", relayout);
    return () => fonts.removeEventListener("loadingdone", relayout);
  }, []);
  // Clé de la session de saisie : de l'entrée en édition à la sortie, d'un champ à l'autre.
  const session = useRef<string | null>(null);

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

  // Relu à chaque vue : quand le rendu arrive, l'éditeur se recale sur le champ.
  const editField = readOnly ? null : findField(view, editing);
  // Brouillon à l'ouverture, avant l'affichage : il ne sert qu'à la largeur d'un champ.
  // Il ne suit pas le serveur pendant l'édition : CodeMirror est la source de vérité.
  useLayoutEffect(() => {
    setDraft(editField?.text ?? "");
    if (editing === null) session.current = null;
    else session.current ??= crypto.randomUUID();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing]);

  // Avant de quitter la slide (focus de l'agent, vignette…), l'éditeur valide
  // sa saisie : sinon la nouvelle vue n'a plus le champ, et la saisie se perd.
  useImperativeHandle(ref, () => ({ commitEditing: () => editor.current?.commit() }), []);

  // Le champ en édition n'est plus dans la vue de la même slide (objet ou
  // paramètre supprimé) : la saisie ne peut plus s'écrire nulle part.
  const gone = editing !== null && editField === null && !readOnly;
  useEffect(() => {
    if (!gone) return;
    onNotice?.(t("ui-toast-field-gone"));
    onEdit(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gone]);

  // La dictée s'insère au curseur de l'éditeur ouvert.
  useEffect(() => {
    if (dictated) editor.current?.insertAtCursor(dictated.text);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dictated?.seq]);

  const startDrag = (e: React.PointerEvent, t: ObjectTarget, mode: Mode) => {
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

  /** Écrit le brouillon d'un champ s'il a changé, dans la session de saisie. */
  const save = (f: FieldView, text: string) => {
    if (fieldChanged(f, text)) onApply(t("ui-undo-edit-slide"), fieldOps(view, f, text), session.current ?? undefined);
  };
  const commit = (text: string) => {
    if (editField) save(editField, text);
    onEdit(null);
  };
  /** Passe au champ `f` ; la sélection suit s'il est d'un autre élément. */
  const editNext = (f: FieldView | null) => {
    if (f?.owner_kind === "element" && f.owner !== selected) onSelect(f.owner);
    onEdit(f ? fieldKey(f) : null);
  };
  const next = (text: string, dir: 1 | -1) => {
    if (!editField || editing === null) return onEdit(null);
    save(editField, text);
    editNext(nextField(view, editing, dir));
  };
  /** En édition, un clic sur un autre champ valide le courant et y passe. */
  const switchTo = (f: FieldView) => {
    const text = editor.current?.text();
    if (editField && text !== undefined) save(editField, text);
    editNext(f);
  };

  const box = (t: ObjectTarget) => {
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

  // La grille se montre quand on travaille avec : élément accroché choisi, ou glisser en cours.
  const zone = view.zone;
  const grid = zone?.grid ?? null;
  const liveCell = drag ? draggedCell(drag) : null;
  // Affichée à la demande ; pendant un glisser sur la grille, toujours (on voit où l'on pose).
  const showGrid = !!grid && (showGridAlways || !!liveCell);
  const tagFor = (t: ObjectTarget) => {
    const cell = drag?.id === t.id && liveCell ? liveCell : findElement(view.slide, t.id)?.cell;
    return elementTag(t.kind, grid ? cell : null);
  };

  const size = { width: Math.floor(view.width * scale), height: Math.floor(view.height * scale) };
  return (
    // La slide garde ses coordonnées de gauche à droite, même dans une interface en arabe.
    <div className="stage-wrap" dir="ltr" ref={wrap} onPointerDown={() => editing || onSelect(null)}>
      {/* Cadre de la scène : même origine que la slide, sans rogner (la fiche de l'agent déborde). */}
      <div className="stage-frame" style={size}>
        <div
          className="stage"
          style={size}
          onPointerMove={moveDrag}
          onPointerUp={endDrag}
          onPointerCancel={() => setDrag(null)}
        >
          {svgUrl && <img src={svgUrl} alt={t("ui-slide-alt", { n: view.page })} draggable={false} />}
          {showGrid && zone && grid && <GridLayer zone={zone} grid={grid} scale={scale} target={liveCell} />}
          {showGhosts && <GhostLayer view={view} scale={scale} hidden={drag?.id ?? null} />}
          <div className="overlay" role="group" aria-label={t("ui-stage-elements")}>
            {view.targets.map((tg) => {
              const sel = tg.id === selected;
              const label = kindLabel(tg.kind);
              const movable = tg.free || (!!grid && isBlock(view, tg.id));
              return (
                <div
                  key={tg.id}
                  className={`target${movable ? " free" : ""}${sel ? " selected" : ""}${editField?.owner === tg.id ? " editing" : ""}`}
                  style={box(tg)}
                  aria-label={label}
                  aria-selected={sel}
                  onPointerDown={(e) => startDrag(e, tg, "move")}
                  onDoubleClick={() => {
                    const f = primaryFieldOf(view, tg.id);
                    if (f) onEdit(fieldKey(f));
                  }}
                >
                  {sel && !editing && <span className="tag">{tagFor(tg)}</span>}
                  {sel && movable && !editing &&
                    (["nw", "ne", "sw", "se"] as Mode[]).map((m) => (
                      <span key={m} className={`handle ${m}`} onPointerDown={(e) => startDrag(e, tg, m)} />
                    ))}
                </div>
              );
            })}
          </div>
          {!readOnly && (
            <div className="fields" role="group" aria-label={t("ui-stage-fields")}>
              {view.fields.map((f) => {
                const key = fieldKey(f);
                // On écrit dans les champs de l'objet sélectionné ou en édition ; partout en mode modèles ou fantômes.
                const hot = templates || showGhosts || f.owner === selected || editField?.owner === f.owner;
                const ghost = f.empty && key !== editing && hot;
                return (
                  <div
                    key={key}
                    className={`field${f.empty ? " empty" : ""}${hot ? " hot" : ""}`}
                    style={fieldHitBox(f, scale, view.fields)}
                    aria-label={fieldLabel(f)}
                    onPointerDown={(e) => {
                      if (editing) {
                        e.stopPropagation();
                        if (key !== editing) switchTo(f);
                        return;
                      }
                      // Hors édition, un clic sélectionne l'objet (un champ de slide : rien).
                      const owner = f.owner_kind === "element" ? view.targets.find((x) => x.id === f.owner) : undefined;
                      if (owner) startDrag(e, owner, "move");
                    }}
                    // En édition, le clic ne prend pas le focus : l'éditeur qui s'ouvre le garde.
                    onMouseDown={(e) => editing && e.preventDefault()}
                    onDoubleClick={() => onEdit(key)}
                  >
                    {/* Un champ vide qui garde un texte (surtitre sans titre) le montre grisé. */}
                    {ghost && <span className="field-placeholder" style={placeholderStyle(f, scale)}>{f.text || fieldLabel(f)}</span>}
                  </div>
                );
              })}
            </div>
          )}
          {editField && (() => {
            const hit = fieldHitBox(editField, scale, view.fields);
            return (
              <>
                {/* Cache le texte rendu dessous : seul le brouillon se voit. */}
                <span
                  className="editor-mask"
                  style={{ left: hit.left - 2, top: hit.top - 2, width: hit.width + 4, height: hit.height + 4, background: editorBackground(editField.style?.fill ?? null, editField.style?.bg ?? null) }}
                />
                <TextEditor
                  key={fieldKey(editField)}
                  ref={editor}
                  initial={editField.text}
                  plain={editField.kind === "plain"}
                  palette={palette}
                  label={t("ui-stage-edit-text")}
                  style={editorGeometry(editField, draft, scale, view.fields)}
                  onChange={setDraft}
                  onCommit={commit}
                  onNext={next}
                  onCancel={() => onEdit(null)}
                />
                <span className="editor-hint">
                  {t(editField.kind === "plain" ? "ui-stage-hint-plain" : "ui-stage-hint-rich")}
                </span>
              </>
            );
          })()}
        </div>
        {overlay && <SheetOverlay view={view} scale={scale} overlay={overlay} />}
      </div>
    </div>
  );
}

/** Boîte (pt) où poser la fiche : un élément, ou `@param` pour un champ de la slide. */
function sheetAnchor(view: SlideView, target: string): { x: number; y: number; w: number; h: number } | null {
  if (target.startsWith("@")) {
    const f = view.fields.find((x) => x.owner_kind === "slide" && x.field === target.slice(1));
    if (!f) return null;
    const b = fieldHitBox(f, 1, view.fields);
    return { x: b.left, y: b.top, w: b.width, h: b.height };
  }
  return view.targets.find((x) => x.id === target) ?? null;
}

/** La fiche de l'agent, posée sous son élément ; rien si l'élément n'est pas sur la slide. */
function SheetOverlay({ view, scale, overlay }: { view: SlideView; scale: number; overlay: { target: string; node: ReactNode } }) {
  const anchor = sheetAnchor(view, overlay.target);
  if (!anchor) return null;
  // L'interface peut être en arabe même si la slide reste de gauche à droite.
  const rtl = document.documentElement.dir === "rtl";
  const pos = sheetPosition(anchor, scale, rtl);
  return (
    <div className={`agent-sheet${rtl ? " rtl" : ""}`} dir={rtl ? "rtl" : "ltr"} style={{ left: pos.left, top: pos.top }}>
      {overlay.node}
    </div>
  );
}
