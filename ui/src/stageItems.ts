/**
 * Items de liste sur la scène : regroupés par liste, l'axe sur lequel ils se
 * suivent, et leur déplacement au clavier depuis un champ édité.
 */
import type { ItemView, ListOp, ListView, Op, SlideView } from "./api";
import { t } from "./i18n";

export type Axis = "x" | "y";

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

/** Nom d'un item de la liste (« étape »), sinon le mot générique. */
export const itemName = (l: Pick<ListView, "item_label">) => l.item_label ?? t("ui-list-item");

/** Propriétaire d'une liste : la slide ou l'un de ses éléments. */
export type ListOwner = Pick<ListView, "owner" | "owner_kind">;

/** Un changement de la liste `list` de `owner`, nommé `label` (étape d'annulation). */
export type OnListOp = (owner: ListOwner, list: string, change: ListOp, label: string) => void;

/** Déplace `it` au rang `to` de sa liste par `onListOp` (rien s'il y est déjà) : Cmd/Ctrl+Alt+flèche depuis un champ édité. */
export function moveItemTo(onListOp: OnListOp, it: ItemView, to: number) {
  if (to !== it.index) onListOp(it, it.list, { op: "move", item: it.item, to }, t("ui-undo-list-move", { item: itemName(it) }));
}

/** Ops d'un changement de la liste `l` de la slide de `view`. */
export function listOps(view: SlideView, l: Pick<ListView, "owner" | "owner_kind" | "list">, change: ListOp): Op[] {
  const target = l.owner_kind === "slide" ? { slide: view.slide.id } : { slide: view.slide.id, element: l.owner };
  return [{ op: "edit_list", target, param: l.list, change }];
}

/** Les changements de liste de la scène du deck : des ops `edit_list` sur la slide de `view`, nommées par leur geste. */
export const deckListOp = (view: SlideView, onApply: (label: string, ops: Op[]) => void | Promise<boolean>): OnListOp =>
  (owner, list, change, label) => void onApply(label, listOps(view, { ...owner, list }, change));

/** D'où viennent les changements de liste de la scène : le deck, ou l'aperçu d'un widget. */
export type ListOpSource =
  | { mode: "deck"; readOnly: boolean; onApply: (label: string, ops: Op[]) => void | Promise<boolean> }
  | { mode: "preview"; onListOp?: OnListOp };

/**
 * Changements de liste de la scène : sur le deck, en ops sur la slide (aucun en lecture
 * seule) ; en aperçu, dans les valeurs d'exemple du widget, aucun sans `onListOp`.
 */
export function stageListOp(view: SlideView, src: ListOpSource): OnListOp | null {
  if (src.mode === "deck") return src.readOnly ? null : deckListOp(view, src.onApply);
  return src.onListOp ?? null;
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
