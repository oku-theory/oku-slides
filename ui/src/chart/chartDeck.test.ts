import { describe, expect, test } from "bun:test";
import type { Element } from "../api";
import { chartKind, chartSource, kindPatch, starterChart } from "./chartDeck";

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

describe("kindPatch", () => {
  test("vers des lignes : une série par colonne de valeurs", () => {
    expect(kindPatch(canonical([{ type: "bar" }, { type: "bar" }]), "line")).toEqual({
      xAxis: { type: "category" },
      yAxis: { type: "value" },
      series: [{ type: "line" }, { type: "line" }],
    });
  });

  test("vers un camembert : une seule série, sans axes", () => {
    expect(kindPatch(canonical([{ type: "bar" }, { type: "bar" }]), "pie")).toEqual({
      xAxis: null,
      yAxis: null,
      series: [{ type: "pie" }],
    });
  });

  test("sans dataset, rien : le patch effacerait les données en ligne des séries", () => {
    const inline: Element = { id: "g", kind: "chart", option: { series: [{ type: "bar", data: [1, 2] }] } };
    expect(kindPatch(inline, "pie")).toBeNull();
  });

  test("le même type ne change rien", () => {
    expect(kindPatch(canonical([{ type: "bar" }, { type: "bar" }]), "bar")).toBeNull();
  });
});
