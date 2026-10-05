import { expect, test } from "bun:test";
import type { SlideView } from "./api";
import { activeOwner, axisOf, dropIndex, dropTarget, groups, insertPoints, itemHandles, within } from "./stageItems";

const item = (item: string, index: number, x: number, y = 0, owner = "f") =>
  ({ owner, owner_kind: "element", list: "etapes", item, index, len: 3, min: 0, max: 256,
     item_label: null, first_text: "date", x, y, w: 100, h: 50 }) as const;

test("axisOf : items côte à côte → x, empilés → y", () => {
  expect(axisOf([item("a", 0, 0), item("b", 1, 120)])).toBe("x");
  expect(axisOf([item("a", 0, 0, 0), item("b", 1, 0, 80)])).toBe("y");
});

test("axisOf : un seul item suit la place libre du cadre", () => {
  expect(axisOf([item("a", 0, 0)], { w: 500, h: 60 })).toBe("x");
  expect(axisOf([item("a", 0, 0)], { w: 110, h: 400 })).toBe("y");
});

test("insertPoints : avant, entre, après", () => {
  const pts = insertPoints([item("a", 0, 0), item("b", 1, 120)], "x");
  expect(pts.map((p) => p.index)).toEqual([0, 1, 2]);
  expect(pts[0]).toEqual({ index: 0, x: -10, y: 25 });
  expect(pts[1].x).toBe(110);
  expect(pts[2]).toEqual({ index: 2, x: 230, y: 25 });
});

test("insertPoints : le long de y, centrés en x", () => {
  const pts = insertPoints([item("a", 0, 0, 0), item("b", 1, 0, 80)], "y");
  expect(pts).toEqual([{ index: 0, x: 50, y: -10 }, { index: 1, x: 50, y: 65 }, { index: 2, x: 50, y: 140 }]);
});

test("dropIndex : position le long de l'axe → index cible", () => {
  const g = [item("a", 0, 0), item("b", 1, 120), item("c", 2, 240)];
  expect(dropIndex(g, "x", 10)).toBe(0);
  expect(dropIndex(g, "x", 200)).toBe(2);
  expect(dropIndex(g, "x", 300)).toBe(2);
});

test("groups : regroupe par propriétaire et liste, trié par index", () => {
  expect(groups([item("b", 1, 120), item("a", 0, 0)])[0].map((i) => i.item)).toEqual(["a", "b"]);
  expect(groups([item("a", 0, 0), item("z", 0, 0, 0, "g")]).map((g) => g[0].owner)).toEqual(["f", "g"]);
});

test("activeOwner : édition > sélection > slide", () => {
  const view = { slide: { id: "s" } } as Parameters<typeof activeOwner>[0];
  expect(activeOwner(view, "f", "g")).toBe("g");
  expect(activeOwner(view, "f", null)).toBe("f");
  expect(activeOwner(view, null, null)).toBe("s");
});

test("dropTarget : repère et rang comptés parmi les autres items, même avec des tailles inégales", () => {
  // a large, b étroit, c large : 181 est plus près du point entre b et c (125) que de celui après c (240),
  // mais après le centre de c (180).
  const g = [item("a", 0, 0), { ...item("b", 1, 110), w: 10 }, item("c", 2, 130)];
  const target = dropTarget(g, g[0], "x", 181);
  expect(target?.index).toBe(2);
  // Le repère est le point d'insertion du rang appliqué : après c.
  expect(target).toEqual({ index: 2, x: 240, y: 25 });
  expect(dropTarget(g, g[0], "x", 50)).toEqual({ index: 0, x: 100, y: 25 });
});

test("within : un bouton de `size` centré en `at` reste dans [0, extent]", () => {
  expect(within(-10, 1000, 24)).toBe(12);
  expect(within(5, 1000, 24)).toBe(12);
  expect(within(500, 1000, 24)).toBe(500);
  expect(within(995, 1000, 24)).toBe(988);
  expect(within(1010, 1000, 24)).toBe(988);
});

const handlesView = { slide: { id: "s" }, lists: [{ owner: "w" }] } as unknown as SlideView;

test("itemHandles : deck → propriétaire actif et ops sur la slide ; lecture seule → aucune", () => {
  const applied: unknown[] = [];
  const deck = { mode: "deck", readOnly: false, selected: "f", editing: null, onApply: (l: string, ops: unknown) => void applied.push([l, ops]) } as const;
  const h = itemHandles(handlesView, deck);
  expect(h?.active).toBe("f");
  h?.onListOp({ owner: "f", owner_kind: "element" }, "etapes", { op: "remove", item: "d0" }, "Supprimer");
  expect(applied).toEqual([["Supprimer", [{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "remove", item: "d0" } }]]]);
  expect(itemHandles(handlesView, { ...deck, readOnly: true })).toBeNull();
});

test("itemHandles : aperçu → propriétaire de sa liste et changement passé tel quel ; sans onListOp → aucune", () => {
  const seen: unknown[] = [];
  const edited: unknown[] = [];
  const h = itemHandles(handlesView, { mode: "preview", onListOp: (...a) => void seen.push(a), onEditWhenShown: (k) => void edited.push(k) });
  expect(h?.active).toBe("w");
  h?.onListOp({ owner: "w", owner_kind: "element" }, "etapes", { op: "remove", item: "d0" }, "Supprimer");
  h?.onInserted("k" as never);
  expect(seen).toEqual([[{ owner: "w", owner_kind: "element" }, "etapes", { op: "remove", item: "d0" }, "Supprimer"]]);
  expect(edited).toEqual(["k"]);
  expect(itemHandles(handlesView, { mode: "preview" })).toBeNull();
});
