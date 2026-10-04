/**
 * Items de liste sur la scène : regroupés par liste, l'axe sur lequel ils se
 * suivent, les points où insérer et le rang visé par un glisser (pt).
 */
import type { ItemView, ListOp, ListView, Op, SlideView } from "./api";
import { t } from "./i18n";

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

/** Nom d'un item de la liste (« étape »), sinon le mot générique. */
export const itemName = (l: Pick<ListView, "item_label">) => l.item_label ?? t("ui-list-item");

/** Ops d'un changement de la liste `l` de la slide de `view`. */
export function listOps(view: SlideView, l: Pick<ListView, "owner" | "owner_kind" | "list">, change: ListOp): Op[] {
  const target = l.owner_kind === "slide" ? { slide: view.slide.id } : { slide: view.slide.id, element: l.owner };
  return [{ op: "edit_list", target, param: l.list, change }];
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
