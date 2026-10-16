import { expect, test } from "bun:test";
import type { SlideView } from "./api";
import { axisOf, groups, moveItemTo, stageListOp } from "./stageItems";

const item = (item: string, index: number, x: number, y = 0, owner = "f") =>
  ({ owner, owner_kind: "element", list: "etapes", item, index, len: 3, min: 0, max: 256,
     item_label: null, x, y, w: 100, h: 50 }) as const;

test("axisOf : items côte à côte → x, empilés → y", () => {
  expect(axisOf([item("a", 0, 0), item("b", 1, 120)])).toBe("x");
  expect(axisOf([item("a", 0, 0, 0), item("b", 1, 0, 80)])).toBe("y");
});

test("axisOf : un seul item suit la place libre du cadre", () => {
  expect(axisOf([item("a", 0, 0)], { w: 500, h: 60 })).toBe("x");
  expect(axisOf([item("a", 0, 0)], { w: 110, h: 400 })).toBe("y");
});

test("groups : regroupe par propriétaire et liste, trié par index", () => {
  expect(groups([item("b", 1, 120), item("a", 0, 0)])[0].map((i) => i.item)).toEqual(["a", "b"]);
  expect(groups([item("a", 0, 0), item("z", 0, 0, 0, "g")]).map((g) => g[0].owner)).toEqual(["f", "g"]);
});

const listView = { slide: { id: "s" } } as unknown as SlideView;

test("stageListOp : deck → ops sur la slide ; lecture seule → aucun", () => {
  const applied: unknown[] = [];
  const deck = { mode: "deck", readOnly: false, onApply: (l: string, ops: unknown) => void applied.push([l, ops]) } as const;
  stageListOp(listView, deck)?.({ owner: "f", owner_kind: "element" }, "etapes", { op: "remove", item: "d0" }, "Supprimer");
  expect(applied).toEqual([["Supprimer", [{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "remove", item: "d0" } }]]]);
  expect(stageListOp(listView, { ...deck, readOnly: true })).toBeNull();
});

test("stageListOp : aperçu → changement passé tel quel ; sans onListOp → aucun", () => {
  const onListOp = () => {};
  expect(stageListOp(listView, { mode: "preview", onListOp })).toBe(onListOp);
  expect(stageListOp(listView, { mode: "preview" })).toBeNull();
});

test("groups : les listes imbriquées de deux items forment deux groupes, à part de la liste parente", () => {
  const nested = (list: string, it: string) => ({ ...item(it, 0, 0), list });
  const g = groups([item("a", 0, 0), nested("etapes/a/taches", "t1"), nested("etapes/b/taches", "t1")]);
  expect(g.map((x) => x[0].list)).toEqual(["etapes", "etapes/a/taches", "etapes/b/taches"]);
});

test("moveItemTo : un item imbriqué se déplace dans sa liste imbriquée, par son chemin", () => {
  const applied: unknown[] = [];
  const deck = { mode: "deck", readOnly: false, onApply: (_l: string, ops: unknown) => void applied.push(ops) } as const;
  const op = stageListOp(listView, deck);
  if (!op) throw new Error("stageListOp attendu");
  moveItemTo(op, { ...item("t1", 0, 0), list: "etapes/a/taches" }, 1);
  expect(applied).toEqual([[{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes/a/taches",
    change: { op: "move", item: "t1", to: 1 } }]]);
});
