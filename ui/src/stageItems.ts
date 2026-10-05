/**
 * Items de liste sur la scène : regroupés par liste, l'axe sur lequel ils se
 * suivent, les points où insérer et le rang visé par un glisser (pt).
 */
import type { ItemView, ListOp, ListView, Op, SlideView } from "./api";
import { t } from "./i18n";
import type { FieldKey } from "./stageFields";

export type Axis = "x" | "y";

/** Écart entre un bord de la liste et le « + » qui insère avant ou après (pt). */
const EDGE = 10;

/** Items regroupés par propriétaire et liste (ordre de première apparition), chaque groupe trié par rang. */
export function groups(items: ItemView[]): ItemView[][] {
  const by = new Map<string, ItemView[]>();
  for (const it of items) {
    const key = `${it.owner}\u001f${it.list}`;
    const g = by.get(key);
    if (g) g.push(it);
    else by.set(key, [it]);
  }
  return [...by.values()].map((g) => g.sort((a, b) => a.index - b.index));
}

const centre = (it: ItemView, axis: Axis) => (axis === "x" ? it.x + it.w / 2 : it.y + it.h / 2);

/**
 * Axe des items : celui où les centres du premier et du dernier s'écartent le
 * plus. Un seul item : celui où le cadre `frame` lui laisse le plus de place.
 */
export function axisOf(group: ItemView[], frame?: { w: number; h: number }): Axis {
  const first = group[0];
  const last = group[group.length - 1];
  if (!first || !last) return "x";
  if (group.length === 1) return !frame || frame.w - first.w >= frame.h - first.h ? "x" : "y";
  const dx = Math.abs(centre(last, "x") - centre(first, "x"));
  const dy = Math.abs(centre(last, "y") - centre(first, "y"));
  return dx >= dy ? "x" : "y";
}

/**
 * Points d'insertion : avant le premier item (à `EDGE` de son bord), au milieu
 * de chaque intervalle, après le dernier ; centrés sur l'autre axe.
 */
export function insertPoints(group: ItemView[], axis: Axis): { index: number; x: number; y: number }[] {
  const at = (index: number, along: number, it: ItemView) =>
    axis === "x" ? { index, x: along, y: it.y + it.h / 2 } : { index, x: it.x + it.w / 2, y: along };
  const start = (it: ItemView) => (axis === "x" ? it.x : it.y);
  const end = (it: ItemView) => (axis === "x" ? it.x + it.w : it.y + it.h);
  const first = group[0];
  const last = group[group.length - 1];
  if (!first || !last) return [];
  const points = [at(first.index, start(first) - EDGE, first)];
  for (let i = 1; i < group.length; i++) {
    const a = group[i - 1];
    const b = group[i];
    points.push(at(b.index, (end(a) + start(b)) / 2, b));
  }
  points.push(at(last.index + 1, end(last) + EDGE, last));
  return points;
}

/** Rang visé par un glisser à `at` le long de l'axe : nombre de centres d'items avant `at`, borné à `len - 1`. */
export function dropIndex(group: ItemView[], axis: Axis, at: number): number {
  const before = group.filter((it) => centre(it, axis) < at).length;
  const len = group[0]?.len ?? group.length;
  return Math.max(0, Math.min(before, len - 1));
}

/**
 * Cible d'un glisser de `it` à `at` : le rang final, compté parmi les autres items du groupe
 * (l'item glissé n'y est plus), et le point d'insertion qui le montre, sur le même ensemble.
 */
export function dropTarget(group: ItemView[], it: ItemView, axis: Axis, at: number): { index: number; x: number; y: number } | null {
  const others = group.filter((x) => x.item !== it.item);
  const index = dropIndex(others, axis, at);
  // Le point de rang `index` précède le `index`-ième des autres (le dernier suit le dernier).
  const point = insertPoints(others, axis)[index];
  return point ? { ...point, index } : null;
}

/** Centre `at` d'un bouton de `size` ramené dans `[0, extent]` : le bouton entier reste dedans. */
export const within = (at: number, extent: number, size: number) => Math.max(size / 2, Math.min(at, extent - size / 2));

/** Nom d'un item de la liste (« étape »), sinon le mot générique. */
export const itemName = (l: Pick<ListView, "item_label">) => l.item_label ?? t("ui-list-item");

/** Propriétaire d'une liste : la slide ou l'un de ses éléments. */
export type ListOwner = Pick<ListView, "owner" | "owner_kind">;

/**
 * Un changement de la liste `list` de `owner`, nommé `label` (étape
 * d'annulation) ; rend éventuellement une promesse : `false` si l'hôte l'a refusé.
 */
export type OnListOp = (owner: ListOwner, list: string, change: ListOp, label: string) => void | Promise<boolean>;

/** Déplace `it` au rang `to` de sa liste par `onListOp` (rien s'il y est déjà) : la poignée ⋮⋮ et Cmd/Ctrl+Alt+flèche. */
export function moveItemTo(onListOp: OnListOp, it: ItemView, to: number) {
  if (to !== it.index) void onListOp(it, it.list, { op: "move", item: it.item, to }, t("ui-undo-list-move", { item: itemName(it) }));
}

/** Ops d'un changement de la liste `l` de la slide de `view`. */
export function listOps(view: SlideView, l: Pick<ListView, "owner" | "owner_kind" | "list">, change: ListOp): Op[] {
  const target = l.owner_kind === "slide" ? { slide: view.slide.id } : { slide: view.slide.id, element: l.owner };
  return [{ op: "edit_list", target, param: l.list, change }];
}

/** Les changements de liste de la scène du deck : des ops `edit_list` sur la slide de `view`, nommées par leur geste. */
export const deckListOp = (view: SlideView, onApply: (label: string, ops: Op[]) => void | Promise<boolean>): OnListOp =>
  (owner, list, change, label) => onApply(label, listOps(view, { ...owner, list }, change));

/** Ce que reçoit la couche des poignées de liste (`ItemLayer`), outre la vue et l'échelle. */
export interface ItemHandles {
  /** Propriétaire dont les poignées sont montrées. */
  active: string | null;
  onListOp: OnListOp;
  /** Champ du nouvel item à ouvrir, dès qu'une vue le montre. */
  onInserted: (key: FieldKey) => void;
}

/** D'où viennent les poignées : la scène du deck, ou l'aperçu d'un widget. */
export type HandlesSource =
  | {
      mode: "deck";
      readOnly: boolean;
      selected: string | null;
      /** Propriétaire du champ en édition. */
      editing: string | null;
      onApply: (label: string, ops: Op[]) => void | Promise<boolean>;
      onEditWhenShown?: (key: FieldKey) => void;
    }
  | { mode: "preview"; onListOp?: OnListOp; onEditWhenShown?: (key: FieldKey) => void };

/**
 * Poignées de liste de la scène : sur le deck, en ops sur la slide (aucune en lecture
 * seule) ; en aperçu, dans le cas d'exemple (les listes de son seul widget), aucune sans `onListOp`.
 */
export function itemHandles(view: SlideView, src: HandlesSource): ItemHandles | null {
  const onInserted = (key: FieldKey) => src.onEditWhenShown?.(key);
  if (src.mode === "deck") {
    return src.readOnly ? null : { active: activeOwner(view, src.selected, src.editing), onListOp: deckListOp(view, src.onApply), onInserted };
  }
  return src.onListOp ? { active: view.lists[0]?.owner ?? null, onListOp: src.onListOp, onInserted } : null;
}

/** Axe d'un groupe d'items de la vue : un item seul suit la place que lui laisse son objet (la page pour une slide). */
export function groupAxis(view: SlideView, group: ItemView[]): Axis {
  const first = group[0];
  const frame = first?.owner_kind === "element" ? view.targets.find((x) => x.id === first.owner) : { w: view.width, h: view.height };
  return axisOf(group, frame);
}

/** L'item `item` de la liste `list` de `owner` dans la vue, et son groupe. */
export function findItem(view: SlideView, owner: string, list: string, item: string): { it: ItemView; group: ItemView[] } | null {
  const group = groups(view.items.filter((x) => x.owner === owner && x.list === list))[0];
  const it = group?.find((x) => x.item === item);
  return group && it ? { it, group } : null;
}

/**
 * Propriétaire dont les poignées de liste sont montrées : celui du champ en
 * édition, sinon l'élément sélectionné, sinon la slide (ses listes propres).
 */
export function activeOwner(view: Pick<SlideView, "slide">, selected: string | null, editing: string | null): string {
  return editing ?? selected ?? view.slide.id;
}
