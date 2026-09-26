import { describe, expect, test } from "bun:test";
import type { Element } from "../api";
import { CHART_KINDS, chartKind, chartSource, isStacked, stackPatch, stackable, starterChart } from "./chartDeck";

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

test("les six types sont proposés", () => {
  expect([...CHART_KINDS]).toEqual(["bar", "line", "pie", "scatter", "radar", "waterfall"]);
});

describe("empilement", () => {
  test("des barres ou des lignes à plusieurs séries s'empilent", () => {
    expect(stackable(canonical([{ type: "bar" }, { type: "bar" }]))).toBe(true);
    expect(stackable(canonical([{ type: "line" }, { type: "line" }]))).toBe(true);
  });

  test("une seule série, un camembert ou un nuage ne s'empilent pas", () => {
    expect(stackable(canonical([{ type: "bar" }]))).toBe(false);
    expect(stackable(canonical([{ type: "pie" }]))).toBe(false);
    expect(stackable(canonical([{ type: "scatter" }, { type: "scatter" }]))).toBe(false);
  });

  test("empilé quand toutes les séries partagent un groupe", () => {
    expect(isStacked(canonical([{ type: "bar", stack: "total" }, { type: "bar", stack: "total" }]))).toBe(true);
    expect(isStacked(canonical([{ type: "bar", stack: "total" }, { type: "bar" }]))).toBe(false);
  });

  test("stackPatch met toutes les séries dans un groupe, en gardant leurs réglages", () => {
    const el = canonical([{ type: "bar", label: { show: true } }, { type: "line" }]);
    expect(stackPatch(el, true)).toEqual({
      series: [{ type: "bar", label: { show: true }, stack: "total" }, { type: "line", stack: "total" }],
    });
  });

  test("stackPatch sans empilement retire le groupe", () => {
    const el = canonical([{ type: "bar", stack: "total" }, { type: "bar", stack: "total" }]);
    expect(stackPatch(el, false)).toEqual({ series: [{ type: "bar" }, { type: "bar" }] });
  });
});
