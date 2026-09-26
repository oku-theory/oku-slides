import { describe, expect, test } from "bun:test";
import type { Element } from "../api";
import { CHART_KINDS, chartKind, chartSource, starterChart } from "./chartDeck";

const canonical = (series: object[], source = [["Mois", "Ventes", "Objectif"], ["Jan", 3, 4]]): Element => ({
  id: "g",
  kind: "chart",
  option: { dataset: { source }, xAxis: { type: "category" }, series },
});

describe("starterChart", () => {
  test("des barres aux catégories neutres, à mettre en forme par la session", () => {
    const body = starterChart();
    expect(body.kind).toBe("chart");
    expect(body.option.series).toEqual([{ type: "bar", data: [3, 5, 2] }]);
    expect(body.option.xAxis).toEqual({ type: "category", data: ["1", "2", "3"] });
  });
});

describe("chartSource et chartKind", () => {
  test("lisent le dataset et le type de la première série", () => {
    const el = canonical([{ type: "line" }, { type: "bar" }]);
    expect(chartSource(el)).toEqual([["Mois", "Ventes", "Objectif"], ["Jan", 3, 4]]);
    expect(chartKind(el)).toBe("line");
  });

  test("un autre élément n'a ni données ni type", () => {
    const text: Element = { id: "t", kind: "text" };
    expect(chartSource(text)).toBeNull();
    expect(chartKind(text)).toBeNull();
  });
});

test("les cinq types sont proposés", () => {
  expect([...CHART_KINDS]).toEqual(["bar", "line", "pie", "scatter", "radar"]);
});
