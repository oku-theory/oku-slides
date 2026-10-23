// Le deck vu de l'interface : retrouver un élément, convertir le texte, et
// fabriquer les ops que la session applique.

import type { Element, FieldView, Op, Params, Paragraph, Slide, SlideView, Snapshot } from "./api";
import type { Cell, GridSpec, Rect, ZoneView } from "./grid";
import { cellRect, snapRect } from "./grid";
import { escapeInline, listPrefix, markdownToRuns } from "./markdown";

/** Identifiant court et unique pour un nouvel élément ou une nouvelle slide. */
export function newId(prefix: string): string {
  const r = crypto.getRandomValues(new Uint32Array(2));
  return `${prefix}-${r[0].toString(36)}${r[1].toString(36).slice(0, 3)}`;
}

/** `v` a la forme d'un slot : un tableau (vide compris) d'objets avec `id` et `kind`. */
function isElementArray(v: unknown): v is Element[] {
  return Array.isArray(v) && v.every((x) => x !== null && typeof x === "object" && typeof (x as Element).id === "string" && typeof (x as Element).kind === "string");
}

/** Éléments du paramètre `name` de `owner` (slot), ou un tableau vide. */
export function slot(owner: { params?: Params }, name: string): Element[] {
  const v = owner.params?.[name];
  return isElementArray(v) ? v : [];
}

/** Paragraphes du paramètre riche `name` d'une slide ou d'un élément (un tableau qui n'a pas la forme d'un slot), ou `undefined`. */
export function rich(e: { params?: Params }, name: string): Paragraph[] | undefined {
  const v = e.params?.[name];
  return Array.isArray(v) && !isElementArray(v) ? (v as Paragraph[]) : undefined;
}

/** Élément par id, dans tous les slots de la slide (au premier niveau et dans ceux des éléments). */
export function findElement(slide: Slide, id: string): Element | undefined {
  const walk = (list: Element[]): Element | undefined => {
    for (const e of list) {
      if (e.id === id) return e;
      for (const v of Object.values(e.params ?? {})) {
        if (isElementArray(v)) {
          const found = walk(v);
          if (found) return found;
        }
      }
    }
    return undefined;
  };
  for (const v of Object.values(slide.params ?? {})) {
    if (isElementArray(v)) {
      const found = walk(v);
      if (found) return found;
    }
  }
  return undefined;
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

/**
 * Texte brut venu d'ailleurs (collage externe, dictée) → paragraphes : son markdown inline reste
 * littéral (`2 * 3 * 4` n'est pas de l'italique), seuls les préfixes `- ` et `1. ` font des listes.
 */
export function literalToParagraphs(text: string): Paragraph[] {
  return textToParagraphs(literalText(text));
}

/** Texte brut échappé ligne à ligne, préfixe de liste gardé, pour que `textToParagraphs` le relise tel quel. */
function literalText(text: string): string {
  return text
    .split("\n")
    .map((line) => {
      const { length } = listPrefix(line);
      return line.slice(0, length) + escapeInline(line.slice(length));
    })
    .join("\n");
}

/** Donne de nouveaux identifiants aux éléments et à leurs enfants, dans tous leurs slots (sur place). */
function renewIds(list: Element[]) {
  for (const e of list) {
    e.id = newId("e");
    for (const v of Object.values(e.params ?? {})) {
      if (isElementArray(v)) renewIds(v);
    }
  }
}

/** Copie d'une slide (celle d'un layout) avec de nouveaux identifiants, éléments compris dans tous leurs slots. */
export function instantiate(slide: Slide): Slide {
  const copy: Slide = structuredClone(slide);
  copy.id = newId("s");
  for (const v of Object.values(copy.params ?? {})) {
    if (isElementArray(v)) renewIds(v);
  }
  return copy;
}

/** Le brouillon change-t-il le champ ? Un champ `plain` compare ses retours à la ligne en espaces. */
export function fieldChanged(f: Pick<FieldView, "kind" | "text">, text: string): boolean {
  return text !== fieldDraft(f);
}

/** Texte d'un champ tel que l'éditeur le montre : un champ `plain` tient sur une ligne. */
export function fieldDraft(f: Pick<FieldView, "kind" | "text">): string {
  return f.kind === "plain" ? f.text.replaceAll("\n", " ") : f.text;
}

/** L'item `id` d'une liste (valeur JSON), ou `undefined`. */
function itemOf(items: unknown, id: string | null | undefined): Record<string, unknown> | undefined {
  if (!Array.isArray(items)) return undefined;
  return items.find((x): x is Record<string, unknown> => x !== null && typeof x === "object" && (x as { id?: unknown }).id === id);
}

/**
 * Paragraphes de la colonne `f.column` de l'item `f.item` dans la liste `f.list` de `owner`, ou
 * `undefined`. `f.list` est le paramètre, ou `liste/id/colonne` pour une liste imbriquée.
 */
function itemRich(owner: { params?: Params } | undefined, f: FieldView): Paragraph[] | undefined {
  const [param, outer, column] = (f.list ?? "").split("/");
  const top = owner?.params?.[param];
  const items = outer === undefined ? top : itemOf(top, outer)?.[column];
  const v = itemOf(items, f.item)?.[f.column];
  return Array.isArray(v) ? (v as Paragraph[]) : undefined;
}

/**
 * Ops pour écrire le brouillon d'un champ : `set_param`, en paragraphes pour
 * un champ `rich` (les lignes inchangées gardent leurs runs, lus dans les
 * paramètres de la slide ou de l'élément) ; `edit_list` `set_field` pour un
 * champ d'item de liste.
 */
export function fieldOps(view: SlideView, f: FieldView, value: string): Op[] {
  const slide = view.slide;
  const target = f.owner_kind === "slide" ? { slide: slide.id } : { slide: slide.id, element: f.owner };
  const v = fieldValue(view, f, value);
  if (f.list && f.item) {
    return [{ op: "edit_list", target, param: f.list, change: { op: "set_field", item: f.item, field: f.column, value: v } }];
  }
  return [{ op: "set_param", target, param: f.field, value: v }];
}

/**
 * Valeur du paramètre d'un champ pour son brouillon : le texte d'un champ
 * `plain`, les paragraphes d'un `rich` (les lignes inchangées gardent leurs
 * runs, lus dans les paramètres de la slide ou de l'élément ; pour un item de
 * liste, dans sa colonne).
 */
export function fieldValue(view: SlideView, f: FieldView, text: string): string | Paragraph[] {
  if (f.kind === "plain") return text;
  const owner = f.owner_kind === "slide" ? view.slide : findElement(view.slide, f.owner);
  const previous = f.list && f.item ? itemRich(owner, f) : owner && rich(owner, f.field);
  return textToParagraphs(text, { paragraphs: previous ?? [], text: f.text });
}

// --- Sur la grille -----------------------------------------------------------

/** Bloc mis en page par le layout (au premier niveau de la slide) ? */
export const isBlock = (view: SlideView, id: string) => slot(view.slide, "blocks").some((b) => b.id === id);

/**
 * Fait passer une slide à blocs sur sa grille : chaque bloc devient un élément
 * accroché aux cases qu'il occupait à l'écran (`cells` impose certaines places).
 * Une seule transaction, annulable d'un coup.
 */
export function blocksToGridOps(view: SlideView, cells: Record<string, Cell> = {}): Op[] {
  const zone = view.zone;
  const g = zone?.grid;
  const blocks = slot(view.slide, "blocks");
  if (!zone || !g || blocks.length === 0) return [];
  const ops: Op[] = blocks.map((b) => ({ op: "remove_element", slide: view.slide.id, element: b.id }));
  for (const b of blocks) {
    const t = view.targets.find((x) => x.id === b.id);
    const cell =
      cells[b.id] ??
      (t ? snapRect(zone, g, { x: t.x - zone.x, y: t.y - zone.y, w: t.w, h: t.h }) : { col: 0, row: 0, cols: g.columns, rows: 1 });
    ops.push({ op: "add_element", slot: { owner: { slide: view.slide.id }, param: "elements" }, element: { ...b, frame: undefined, cell } });
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
  const el = slot(view.slide, "elements").find((x) => x.id === id);
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
  const placed = slot(slide, "elements").flatMap((e) => (e.cell ? [e.cell] : []));
  const below = placed.reduce((m, c) => Math.max(m, c.row + (c.rows ?? 1)), 0);
  const row = below < grid.rows ? below : 0;
  return { col: 0, row, cols: span.cols ?? Math.max(1, Math.round(grid.columns / 2)), rows: span.rows ?? 1 };
}

/**
 * Ops qui posent un élément neuf de la bibliothèque sur la slide de `view` :
 * sur la grille, la moitié gauche de la première ligne libre sous les autres ;
 * une slide à blocs (ou sans grille) reçoit un bloc de plus. `defaults` : le
 * cœur amorce ses listes avec le défaut du schéma (pas un collage).
 */
export function newElementOps(view: SlideView, element: Element): Op[] {
  const slide = view.slide;
  const g = view.zone?.grid;
  if (g && slot(slide, "blocks").length === 0) {
    return [{ op: "add_element", slot: { owner: { slide: slide.id }, param: "elements" }, defaults: true, element: { ...element, cell: freeCell(slide, g) } }];
  }
  return [{ op: "add_element", slot: { owner: { slide: slide.id }, param: "blocks" }, defaults: true, element }];
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
  let param: "blocks" | "elements" = "elements";
  if (slot(slide, "blocks").length > 0 || (!grid && !frame)) {
    element = body;
    param = "blocks";
  } else if (grid) {
    element = { ...body, cell: freeCell(slide, grid, cell ? { cols: cell.cols ?? 1, rows: cell.rows ?? 1 } : {}) };
  } else {
    element = { ...body, frame: frame && { ...frame, x: frame.x + PASTE_OFFSET, y: frame.y + PASTE_OFFSET } };
  }
  return { id: copy.id, ops: [{ op: "add_element", slot: { owner: { slide: slide.id }, param }, element }] };
}

/** Ops qui déplacent `source` (de la slide `from`) sur la slide de `view`. Même placement que `pasteOps`, ids gardés. */
export function moveOps(view: SlideView, source: Element, from: string): { id: string; ops: Op[] } {
  const slide = view.slide;
  const grid = view.zone?.grid;
  const id = source.id;
  // Le slot `blocks` retire case et cadre côté cœur ; hors grille, le cadre est gardé tel quel.
  const param = slot(slide, "blocks").length > 0 || (!grid && !source.frame) ? "blocks" : "elements";
  const ops: Op[] = [{ op: "move_element", slide: from, element: id, to: { owner: { slide: slide.id }, param } }];
  if (param === "elements" && grid) {
    const span = source.cell ? { cols: source.cell.cols ?? 1, rows: source.cell.rows ?? 1 } : {};
    ops.push({ op: "set_cell", slide: slide.id, element: id, cell: freeCell(slide, grid, span) });
    if (source.frame) ops.push({ op: "set_frame", slide: slide.id, element: id, frame: null });
  }
  return { id, ops };
}

/**
 * Après un `move_element` refusé : coller une copie à la place ?
 *
 * Seulement si l'élément n'est plus sur la slide `from` (retirée, ou
 * l'élément en est sorti). Sinon le refus tient (slot refusé, profondeur…) :
 * une copie doublerait l'élément. Vue illisible : pas de copie.
 */
export async function copyAfterRefusedMove(
  slides: { id: string }[],
  from: string,
  element: string,
  fetchSlide: (id: string) => Promise<SlideView>,
): Promise<boolean> {
  if (!slides.some((s) => s.id === from)) return true;
  try {
    return !findElement((await fetchSlide(from)).slide, element);
  } catch {
    return false;
  }
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

/** Slide à montrer : la courante si elle existe encore, sinon celle retenue par l'hôte, sinon la première. */
export function pickSlide(slides: { id: string }[], current: string | null, remembered: string | null): string | null {
  const has = (id: string | null): id is string => id !== null && slides.some((s) => s.id === id);
  if (has(current)) return current;
  if (has(remembered)) return remembered;
  return slides[0]?.id ?? null;
}

type LookSource = Pick<Snapshot, "theme_pins" | "appearance" | "editing" | "undo_label" | "redo_label">;

/**
 * Style du deck vu par les aperçus de widgets : un changement le relance.
 *
 * En mode pack, le deck de travail n'a pas de thème : le brouillon (base,
 * nom, modifié, révision de ses jetons) et les étapes d'annulation (une
 * modification de l'agent, une annulation) en tiennent lieu. La révision
 * distingue deux réglages successifs du même jeton, que rien d'autre ne sépare.
 */
export function previewLook(snap: LookSource): string {
  return JSON.stringify([
    snap.theme_pins,
    snap.appearance.slide,
    snap.editing?.theme ?? null,
    snap.undo_label,
    snap.redo_label,
  ]);
}
