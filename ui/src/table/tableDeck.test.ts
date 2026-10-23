import { describe, expect, test } from "bun:test";
import { asColumnAlign, starterTable } from "./tableDeck";

describe("tableDeck", () => {
  test("starterTable has three rows of the same width", () => {
    const s = starterTable().dataset.source;
    expect([s.length, new Set(s.map((r) => r.length)).size]).toEqual([3, 1]);
  });

  test("asColumnAlign of an unknown value is auto", () => {
    expect(asColumnAlign("diagonal")).toBe("auto");
  });
});
