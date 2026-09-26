import { describe, expect, test } from "bun:test";
import type { Element } from "../api";
import { CHART_KINDS, chartKind, chartSource, colorPatch, hexColor, isStacked, seriesColor, seriesNames, stackPatch, stackable, starterChart } from "./chartDeck";

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

describe("couleurs des séries", () => {
  test("barres et lignes : une série par colonne de valeurs, nommée par son en-tête", () => {
    expect(seriesNames(canonical([{ type: "bar" }, { type: "line" }]))).toEqual(["Ventes", "Objectif"]);
  });

  test("nuage : une série par paire de colonnes, nommée par la colonne des y", () => {
    const el = canonical([{ type: "scatter" }], [["x de A", "A", "x de B", "B"], [1, 2, 3, 4]]);
    expect(seriesNames({ ...el, option: { ...(el.option as object), series: [{ type: "scatter" }, { type: "scatter" }] } })).toEqual(["A", "B"]);
  });

  test("secteurs, radar et cascade suivent la palette : aucune série à colorer", () => {
    expect(seriesNames(canonical([{ type: "pie" }]))).toEqual([]);
    expect(seriesNames(canonical([{ type: "radar" }]))).toEqual([]);
    expect(seriesNames(canonical([{ type: "waterfall" }]))).toEqual([]);
  });

  test("seriesColor lit itemStyle.color", () => {
    const el = canonical([{ type: "bar" }, { type: "bar", itemStyle: { color: "#C4622D" } }]);
    expect([seriesColor(el, 0), seriesColor(el, 1)]).toEqual([null, "#c4622d"]);
  });

  test("colorPatch colore une série et garde les autres réglages", () => {
    const el = canonical([{ type: "bar", stack: "total" }, { type: "bar" }]);
    expect(colorPatch(el, 0, "#5B8A3C")).toEqual({
      series: [{ type: "bar", stack: "total", itemStyle: { color: "#5B8A3C" } }, { type: "bar" }],
    });
  });

  test("colorPatch sans couleur rend la série à la palette", () => {
    const el = canonical([{ type: "bar", itemStyle: { color: "#5B8A3C" } }]);
    expect(colorPatch(el, 0, null)).toEqual({ series: [{ type: "bar" }] });
  });
});

describe("hexColor", () => {
  test("ramène #rgb et #RRGGBB à #rrggbb", () => {
    expect([hexColor("#ABC"), hexColor("#C4622D"), hexColor("#c4622d")]).toEqual(["#aabbcc", "#c4622d", "#c4622d"]);
  });

  test("refuse tout ce qui n'est pas une couleur hexadécimale", () => {
    expect([hexColor("red"), hexColor("#12345"), hexColor("url(x)"), hexColor("#fff;background:url(x)"), hexColor(3)]).toEqual([
      null, null, null, null, null,
    ]);
  });

  test("seriesColor ignore une couleur invalide dans le deck", () => {
    const el = canonical([{ type: "bar", itemStyle: { color: "red;background:url(x)" } }]);
    expect(seriesColor(el, 0)).toBeNull();
  });

  test("seriesColor rend la couleur normalisée", () => {
    expect(seriesColor(canonical([{ type: "bar", itemStyle: { color: "#ABC" } }]), 0)).toBe("#aabbcc");
  });
});
