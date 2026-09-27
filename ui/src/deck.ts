// Le deck vu de l'interface : retrouver un élément, convertir le texte, et
// fabriquer les ops que la session applique.

import type { Element, Op, Paragraph, Slide, SlideView, Target } from "./api";
import type { Cell, GridSpec, Rect, ZoneView } from "./grid";
import { cellRect, snapRect } from "./grid";
import { listPrefix, markdownToRuns } from "./markdown";

/** Identifiant court et unique pour un nouvel élément ou une nouvelle slide. */
export function newId(prefix: string): string {
  const r = crypto.getRandomValues(new Uint32Array(2));
  return `${prefix}-${r[0].toString(36)}${r[1].toString(36).slice(0, 3)}`;
}

/** Élément par id, dans les blocs, les éléments libres et les groupes. */
export function findElement(slide: Slide, id: string): Element | undefined {
  const walk = (list: Element[] | undefined): Element | undefined => {
    for (const e of list ?? []) {
      if (e.id === id) return e;
      const found = walk(e.children);
      if (found) return found;
    }
    return undefined;
  };
  return walk(slide.blocks) ?? walk(slide.elements);
}

/**
 * Texte éditable → paragraphes : l'inverse de `paragraphs_to_text` côté Rust
 * (une ligne par paragraphe, `- ` pour une puce, `1. ` pour une liste
 * numérotée, markdown inline pour le reste). Une ligne identique à une ligne
 * de `previous.text` (écrit par Rust pour `previous.paragraphs`) garde ce
 * paragraphe tel quel : couleur, taille et bords perdus à l'écriture compris.
 * Cette réutilisation suppose une ligne par paragraphe ; si un run contient
 * `\n`, les indices se décalent et on relit tout depuis le texte.
 */
export function textToParagraphs(text: string, previous?: { paragraphs: Paragraph[]; text: string }): Paragraph[] {
  const before = previous ? previous.text.split("\n") : [];
  const alignable = previous !== undefined && before.length === previous.paragraphs.length;
  return text.split("\n").map((line) => {
    const same = alignable ? previous?.paragraphs[before.indexOf(line)] : undefined;
    if (same) return same;
    const { list, length } = listPrefix(line);
    const runs = markdownToRuns(line.slice(length));
    return list ? { runs, list } : { runs };
  });
}

/** Donne de nouveaux identifiants aux éléments et à leurs enfants (sur place). */
function renewIds(list?: Element[]) {
  for (const e of list ?? []) {
    e.id = newId("e");
    renewIds(e.children);
  }
}

/** Copie d'une slide modèle avec de nouveaux identifiants (slide et éléments). */
export function instantiate(template: Slide): Slide {
  const copy: Slide = structuredClone(template);
  copy.id = newId("s");
  renewIds(copy.blocks);
  renewIds(copy.elements);
  return copy;
}

/** Le texte validé diffère-t-il de celui de la cible ? Un champ s'édite sur
 *  une ligne : ses retours à la ligne, aplatis par l'éditeur, ne comptent pas. */
export function textChanged(t: Pick<Target, "target" | "text">, text: string): boolean {
  const before = t.text ?? "";
  return text !== (t.target === "field" ? before.replaceAll("\n", " ") : before);
}

/** Ops pour remplacer le texte d'une cible. */
export function textOps(view: SlideView, t: Target, value: string): Op[] {
  const slide = view.slide;
  if (t.target === "element") {
    const e = findElement(slide, t.id);
    const previous = { paragraphs: e?.text ?? [], text: t.text ?? "" };
    return [{ op: "set_text", slide: slide.id, element: t.id, text: textToParagraphs(value, previous) }];
  }
  const field = t.kind;
  const v = value.trim() === "" ? null : value;
  // Sur la couverture, titre et sous-titre viennent des paramètres du deck.
  if (slide.kind === "title" && (field === "title" || field === "subtitle") && slide[field] === undefined) {
    return [{ op: "set_param", key: field, value: v }];
  }
  return [{ op: "set_slide_field", slide: slide.id, field, value: v }];
}

// --- Sur la grille -----------------------------------------------------------

/** Bloc mis en page par le layout (au premier niveau de la slide) ? */
export const isBlock = (view: SlideView, id: string) => (view.slide.blocks ?? []).some((b) => b.id === id);

/**
 * Fait passer une slide à blocs sur sa grille : chaque bloc devient un élément
 * accroché aux cases qu'il occupait à l'écran (`cells` impose certaines places).
 * Une seule transaction, annulable d'un coup.
 */
export function blocksToGridOps(view: SlideView, cells: Record<string, Cell> = {}): Op[] {
  const zone = view.zone;
  const g = zone?.grid;
  const blocks = view.slide.blocks ?? [];
  if (!zone || !g || blocks.length === 0) return [];
  const ops: Op[] = blocks.map((b) => ({ op: "remove_element", slide: view.slide.id, element: b.id }));
  for (const b of blocks) {
    const t = view.targets.find((x) => x.id === b.id);
    const cell =
      cells[b.id] ??
      (t ? snapRect(zone, g, { x: t.x - zone.x, y: t.y - zone.y, w: t.w, h: t.h }) : { col: 0, row: 0, cols: g.columns, rows: 1 });
    ops.push({ op: "add_element", slide: view.slide.id, zone: "elements", element: { ...b, frame: undefined, cell } });
  }
  return ops;
}

/** Place d'un élément sur la grille de sa slide. */
export interface GridPlacement {
  zone: ZoneView;
  grid: GridSpec;
  /** Case occupée (ou la plus proche de son cadre). */
  cell: Cell;
  /** Cadre actuel, relatif à la zone (pt). */
  frame: Rect;
  /** Bloc mis en page : le poser fait passer toute la slide sur la grille. */
  block: boolean;
  /** Déjà accroché à une case (sinon, il s'y accroche en bougeant). */
  onCell: boolean;
}

/**
 * Place sur la grille d'un bloc (là où il est à l'écran) ou d'un élément libre
 * (sa case, ou la case la plus proche de son cadre) ; `null` s'il ne s'y place pas.
 */
export function gridPlacement(view: SlideView, id: string): GridPlacement | null {
  const zone = view.zone;
  const grid = zone?.grid;
  if (!zone || !grid) return null;
  if (isBlock(view, id)) {
    const t = view.targets.find((x) => x.id === id);
    if (!t) return null;
    const frame = { x: t.x - zone.x, y: t.y - zone.y, w: t.w, h: t.h };
    return { zone, grid, cell: snapRect(zone, grid, frame), frame, block: true, onCell: false };
  }
  const el = (view.slide.elements ?? []).find((x) => x.id === id);
  if (el?.cell) return { zone, grid, cell: el.cell, frame: cellRect(zone, grid, el.cell), block: false, onCell: true };
  if (el?.frame) return { zone, grid, cell: snapRect(zone, grid, el.frame), frame: el.frame, block: false, onCell: false };
  return null;
}

/** Ops qui posent un élément sur une case : un bloc entraîne toute la slide, un élément libre perd son cadre. */
export function placeOnGridOps(view: SlideView, id: string, cell: Cell): Op[] {
  if (isBlock(view, id)) return blocksToGridOps(view, { [id]: cell });
  const ops: Op[] = [{ op: "set_cell", slide: view.slide.id, element: id, cell }];
  if (findElement(view.slide, id)?.frame) ops.push({ op: "set_frame", slide: view.slide.id, element: id, frame: null });
  return ops;
}

/**
 * Case d'un nouvel élément : à gauche de la première ligne libre sous les
 * autres (la première ligne si la grille est pleine), sur `cols` × `rows`.
 * Par défaut, la moitié de la largeur sur une ligne.
 */
export function freeCell(slide: Slide, grid: GridSpec, span: { cols?: number; rows?: number } = {}): Cell {
  const placed = (slide.elements ?? []).flatMap((e) => (e.cell ? [e.cell] : []));
  const below = placed.reduce((m, c) => Math.max(m, c.row + (c.rows ?? 1)), 0);
  const row = below < grid.rows ? below : 0;
  return { col: 0, row, cols: span.cols ?? Math.max(1, Math.round(grid.columns / 2)), rows: span.rows ?? 1 };
}

/** Décalage d'un élément collé sur son original, hors grille (pt). */
const PASTE_OFFSET = 12;

/**
 * Ops qui collent une copie de `source` sur la slide de `view`, avec de
 * nouveaux identifiants : une slide à blocs reçoit un bloc de plus ; sur la
 * grille, la copie garde son étendue sur la première ligne libre ; sinon,
 * elle se pose décalée de son cadre.
 */
export function pasteOps(view: SlideView, source: Element): { id: string; ops: Op[] } {
  const copy: Element = structuredClone(source);
  renewIds([copy]);
  const slide = view.slide;
  const grid = view.zone?.grid;
  const { frame, cell, ...body } = copy;
  let element: Element;
  let zone: "blocks" | "elements" = "elements";
  if ((slide.blocks?.length ?? 0) > 0 || (!grid && !frame)) {
    element = body;
    zone = "blocks";
  } else if (grid) {
    element = { ...body, cell: freeCell(slide, grid, cell ? { cols: cell.cols ?? 1, rows: cell.rows ?? 1 } : {}) };
  } else {
    element = { ...body, frame: frame && { ...frame, x: frame.x + PASTE_OFFSET, y: frame.y + PASTE_OFFSET } };
  }
  return { id: copy.id, ops: [{ op: "add_element", slide: slide.id, zone, element }] };
}

/** Slide à montrer après avoir retiré `removed` : la suivante, sinon la précédente. */
export function neighbourSlide(slides: { id: string }[], removed: string): string | null {
  const i = slides.findIndex((s) => s.id === removed);
  return slides[i + 1]?.id ?? slides[i - 1]?.id ?? null;
}

/** Slide dont charger la vue : `current` s'il est encore dans le deck, sinon aucune. */
export function slideToFetch(slides: { id: string }[], current: string | null): string | null {
  return current && slides.some((s) => s.id === current) ? current : null;
}

/**
 * Grille à montrer : au premier snapshot, celle que l'hôte a gardée pour la
 * reprise ; ensuite, le choix courant de l'utilisateur.
 */
export function restoredGrid(first: boolean, ui: { grid: boolean }, current: boolean): boolean {
  return first ? ui.grid : current;
}

/** Slide à montrer : la courante si elle existe encore, sinon celle retenue par l'hôte, sinon la première. */
export function pickSlide(slides: { id: string }[], current: string | null, remembered: string | null): string | null {
  const has = (id: string | null): id is string => id !== null && slides.some((s) => s.id === id);
  if (has(current)) return current;
  if (has(remembered)) return remembered;
  return slides[0]?.id ?? null;
}
