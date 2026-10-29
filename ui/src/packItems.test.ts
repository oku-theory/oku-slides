import { describe, expect, test } from "bun:test";
import type { SlideSummary, Snapshot } from "./api";
import type { PackKind } from "./packTypes";
import { itemSlide, neighbour, pileItems, slideItem } from "./packItems";

const slide = (id: string): SlideSummary => ({ id, kind: "essentiel/content", title: null, page: 1, hash: `h-${id}` });
/** A pack tab's snapshot reduced to the fields `pileItems` reads. */
const snap = (kind: PackKind | null, ids: string[]) =>
  ({ editing: kind ? { kind } : null, slides: ids.map(slide) }) as unknown as Snapshot;

describe("itemSlide", () => {
  test.each([
    ["widgets", "kpi", "widget-kpi"],
    ["layouts", "cover", "layout-cover"],
    ["widgets", "_commun", null],
    ["layouts", "_commun", null],
  ] as [PackKind, string, string | null][])("itemSlide of a %s item %s returns %p", (kind, name, id) =>
    expect(itemSlide(kind, name)).toBe(id),
  );
});

describe("slideItem", () => {
  test.each([["widgets", "kpi"], ["layouts", "cover-2"]] as [PackKind, string][])(
    "slideItem of the %s slide of %s returns the item",
    (kind, name) => expect(slideItem(itemSlide(kind, name)!)).toBe(name),
  );
  test.each(["s-1", "widget-", "layout-", "other-kpi"])("slideItem of %p returns null", (id) =>
    expect(slideItem(id)).toBeNull(),
  );
});

describe("pileItems", () => {
  test("pileItems of a widget pack lists its widget slides in slide order", () => {
    const items = pileItems(snap("widgets", ["widget-b", "widget-a"]));
    expect(items.map((i) => [i.name, i.slide.id])).toEqual([["b", "widget-b"], ["a", "widget-a"]]);
  });
  test("pileItems of a layout pack keeps only its layout slides", () => {
    const items = pileItems(snap("layouts", ["s-1", "layout-cover", "widget-kpi", "layout-end"]));
    expect(items.map((i) => i.name)).toEqual(["cover", "end"]);
  });
  test("pileItems outside a pack tab is empty", () => {
    expect(pileItems(snap(null, ["layout-cover"]))).toEqual([]);
  });
});

describe("neighbour", () => {
  const all = () => true;
  test.each([
    [["a", "b", "c"], "b", "c"],
    [["a", "b", "c"], "c", "b"],
    [["_commun", "a"], "a", null],
    [["a"], "a", null],
    [["a", "b"], "x", null],
  ] as [string[], string, string | null][])("neighbour in %p of %s returns %p", (order, gone, next) =>
    expect(neighbour(order, gone, all)).toBe(next),
  );
  test("neighbour skips the items that are gone too", () => {
    expect(neighbour(["a", "b", "c", "d"], "b", (n) => n !== "c")).toBe("d");
  });
});
