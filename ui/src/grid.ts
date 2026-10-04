// Grille de mise en page : géométrie, accroche et déplacement sur les nœuds.
//
// Une grille découpe la zone de corps d'une slide en colonnes et lignes égales,
// séparées par une gouttière. Un élément accroché occupe un rectangle de
// cellules : il prend une colonne, une ligne, ou un bloc de cases.
//
// Même calcul que `grid-frame` dans typst/oku-slides/src/data.typ : le rendu et
// l'édition doivent rester alignés au point près.

import { t } from "./i18n";

/** Grille résolue par le rendu : gouttière en pt. */
export interface GridSpec {
  columns: number;
  rows: number;
  gap: number;
}

/** Zone des éléments libres d'une slide, en pt sur la page. */
export interface ZoneView {
  page: number;
  x: number;
  y: number;
  w: number;
  h: number;
  grid: GridSpec | null;
}

/** Place sur la grille : première colonne et ligne (0…), étendue. */
export interface Cell {
  col: number;
  row: number;
  cols?: number;
  rows?: number;
}

/** Cadre relatif à la zone (pt). */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Handle = "nw" | "ne" | "sw" | "se";

const span = (c: Cell) => ({ cols: c.cols ?? 1, rows: c.rows ?? 1 });

/** Largeur d'une colonne et hauteur d'une ligne. */
export function trackSize(zone: ZoneView, g: GridSpec) {
  return {
    cw: (zone.w - (g.columns - 1) * g.gap) / g.columns,
    rh: (zone.h - (g.rows - 1) * g.gap) / g.rows,
  };
}

/** Début et fin de chaque colonne (x) et de chaque ligne (y), relatifs à la zone. */
export function tracks(zone: ZoneView, g: GridSpec) {
  const { cw, rh } = trackSize(zone, g);
  const cols = Array.from({ length: g.columns }, (_, i) => ({ start: i * (cw + g.gap), end: i * (cw + g.gap) + cw }));
  const rows = Array.from({ length: g.rows }, (_, i) => ({ start: i * (rh + g.gap), end: i * (rh + g.gap) + rh }));
  return { cols, rows };
}

/** Cadre d'une cellule, relatif à la zone. */
export function cellRect(zone: ZoneView, g: GridSpec, cell: Cell): Rect {
  const { cw, rh } = trackSize(zone, g);
  const { cols, rows } = span(cell);
  return {
    x: cell.col * (cw + g.gap),
    y: cell.row * (rh + g.gap),
    w: cols * cw + (cols - 1) * g.gap,
    h: rows * rh + (rows - 1) * g.gap,
  };
}

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

/** Indice de la piste dont le bord (début ou fin) est le plus proche de `v`. */
function nearest(edges: number[], v: number): number {
  let best = 0;
  for (let i = 1; i < edges.length; i++) if (Math.abs(edges[i] - v) < Math.abs(edges[best] - v)) best = i;
  return best;
}

/** Accroche un cadre libre aux nœuds les plus proches. */
export function snapRect(zone: ZoneView, g: GridSpec, r: Rect): Cell {
  const t = tracks(zone, g);
  const col = nearest(t.cols.map((c) => c.start), r.x);
  const row = nearest(t.rows.map((c) => c.start), r.y);
  const endCol = Math.max(col, nearest(t.cols.map((c) => c.end), r.x + r.w));
  const endRow = Math.max(row, nearest(t.rows.map((c) => c.end), r.y + r.h));
  return { col, row, cols: endCol - col + 1, rows: endRow - row + 1 };
}

/** Déplace une cellule d'un glissé (pt), par pas entiers, sans sortir de la grille. */
export function moveCell(zone: ZoneView, g: GridSpec, cell: Cell, dx: number, dy: number): Cell {
  const { cw, rh } = trackSize(zone, g);
  const { cols, rows } = span(cell);
  return {
    col: clamp(cell.col + Math.round(dx / (cw + g.gap)), 0, g.columns - cols),
    row: clamp(cell.row + Math.round(dy / (rh + g.gap)), 0, g.rows - rows),
    cols,
    rows,
  };
}

/**
 * Redimensionne par un coin : le bord tiré s'accroche au nœud le plus proche,
 * le bord opposé reste en place ; une case au moins.
 */
export function resizeCell(zone: ZoneView, g: GridSpec, cell: Cell, handle: Handle, dx: number, dy: number): Cell {
  const t = tracks(zone, g);
  const r = cellRect(zone, g, cell);
  let c0 = cell.col;
  let r0 = cell.row;
  let c1 = cell.col + span(cell).cols - 1;
  let r1 = cell.row + span(cell).rows - 1;
  if (handle === "nw" || handle === "sw") c0 = Math.min(c1, nearest(t.cols.map((c) => c.start), r.x + dx));
  else c1 = Math.max(c0, nearest(t.cols.map((c) => c.end), r.x + r.w + dx));
  if (handle === "nw" || handle === "ne") r0 = Math.min(r1, nearest(t.rows.map((c) => c.start), r.y + dy));
  else r1 = Math.max(r0, nearest(t.rows.map((c) => c.end), r.y + r.h + dy));
  return { col: c0, row: r0, cols: c1 - c0 + 1, rows: r1 - r0 + 1 };
}

/** Cellules égales ? */
export const sameCell = (a: Cell, b: Cell) =>
  a.col === b.col && a.row === b.row && span(a).cols === span(b).cols && span(a).rows === span(b).rows;

/** Libellé court : « colonnes 1–8, lignes 1–6 ». */
export function cellLabel(cell: Cell): string {
  const { cols, rows } = span(cell);
  const range = (a: number, n: number) => (n === 1 ? `${a + 1}` : `${a + 1}–${a + n}`);
  return t("ui-grid-cell", { cols: range(cell.col, cols), rows: range(cell.row, rows) });
}
