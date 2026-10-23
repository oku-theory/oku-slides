import { describe, expect, test } from "bun:test";
import { asChartKind, starterChart } from "./chartDeck";

describe("starterChart", () => {
  test("des barres aux catégories neutres, à mettre en forme par la session", () => {
    const option = starterChart();
    expect(option.series).toEqual([{ type: "bar", data: [3, 5, 2] }]);
    expect(option.xAxis).toEqual({ type: "category", data: ["1", "2", "3"] });
  });
});

describe("asChartKind", () => {
  test("asChartKind of an unknown type is null", () => {
    expect(asChartKind("donut")).toBeNull();
  });
});
