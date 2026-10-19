import { describe, expect, test } from "bun:test";

import { alignPatch, asColumnAlign, columnAlign, flag, starterTable, tableOption, type TableOption } from "./tableDeck";

const option: TableOption = { dataset: { source: [["A", "B", "C"], [1, 2, 3]] }, align: ["end"] };

describe("tableDeck", () => {
  test("tableOption of a table element returns its data", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: option } }, "data")?.dataset.source[0]).toEqual(["A", "B", "C"]);
  });

  test("tableOption with a missing param returns null", () => {
    expect(tableOption({ id: "g", kind: "base/chart", params: { option: {} } }, "data")).toBeNull();
  });

  test("tableOption with a non-object param returns null", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: "x" } }, "data")).toBeNull();
  });

  test("tableOption of a pack table reads its own param", () => {
    expect(tableOption({ id: "t", kind: "acme/grid", params: { grid: option } }, "grid")?.dataset.source[0]).toEqual(["A", "B", "C"]);
  });

  test.each(["__proto__", "constructor"])("tableOption with an absent %s param returns null", (param) => {
    expect(tableOption({ id: "t", kind: "acme/grid", params: {} }, param)).toBeNull();
  });

  test("tableOption with a non-array source returns null", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: { dataset: { source: "x" } } } }, "data")).toBeNull();
  });

  test("tableOption with a non-array row returns null", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: { dataset: { source: ["x"] } } } }, "data")).toBeNull();
  });

  test("tableOption with an object cell returns null", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: { dataset: { source: [["A"], [{ a: 1 }]] } } } }, "data")).toBeNull();
  });

  test("tableOption with an empty source returns null", () => {
    expect(tableOption({ id: "t", kind: "base/table", params: { data: { dataset: { source: [] } } } }, "data")).toBeNull();
  });

  test("flag without value returns the default", () => {
    expect([flag(option, "header-row"), flag(option, "header-column"), flag(option, "striped")]).toEqual([true, false, false]);
  });

  test("flag with a value returns it", () => {
    expect(flag({ ...option, "header-row": false }, "header-row")).toBe(false);
  });

  test("columnAlign without entry is auto", () => {
    expect([columnAlign(option, 0), columnAlign(option, 2)]).toEqual(["end", "auto"]);
  });

  test("alignPatch on one column keeps the others", () => {
    expect(alignPatch(option, 2, "center")).toEqual({ align: ["end", "auto", "center"] });
  });

  test("alignPatch back to auto trims trailing autos to null", () => {
    expect(alignPatch(option, 0, "auto")).toEqual({ align: null });
  });

  test("asColumnAlign of an unknown value returns auto", () => {
    expect([asColumnAlign("center"), asColumnAlign("left")]).toEqual(["center", "auto"]);
  });

  test("starterTable has three rows of the same width", () => {
    const s = starterTable().dataset.source;
    expect([s.length, new Set(s.map((r) => r.length)).size]).toEqual([3, 1]);
  });
});
