import { expect, test } from "bun:test";
import { axisOf, dropIndex, groups, insertPoints } from "./stageItems";

const item = (item: string, index: number, x: number, y = 0, owner = "f") =>
  ({ owner, owner_kind: "element", list: "etapes", item, index, len: 3, min: 0, max: null,
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
