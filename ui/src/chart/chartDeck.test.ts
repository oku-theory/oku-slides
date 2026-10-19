import { describe, expect, test } from "bun:test";
import type { Element } from "../api";
import { CHART_KINDS, chartKind, checkboxColumns, chartSource, colorPatch, hexColor, isStacked, paletteOf, seriesColor, seriesNames, stackPatch, stackable, starterChart } from "./chartDeck";

const canonical = (series: object[], source = [["Mois", "Ventes", "Objectif"], ["Jan", 3, 4]]): Element => ({
  id: "g",
  kind: "base/chart",
  params: { option: { dataset: { source }, xAxis: { type: "category" }, series } },
});

describe("starterChart", () => {
  test("des barres aux catégories neutres, à mettre en forme par la session", () => {
    const option = starterChart();
    expect(option.series).toEqual([{ type: "bar", data: [3, 5, 2] }]);
    expect(option.xAxis).toEqual({ type: "category", data: ["1", "2", "3"] });
  });
});

describe("chartSource et chartKind", () => {
  test("lisent le dataset et le type de la première série", () => {
    const el = canonical([{ type: "line" }, { type: "bar" }]);
    expect(chartSource(el, "option")).toEqual([["Mois", "Ventes", "Objectif"], ["Jan", 3, 4]]);
    expect(chartKind(el, "option")).toBe("line");
  });

  test("un paramètre absent ou qui n'est pas un objet n'a ni données ni type", () => {
    const missing: Element = { id: "t", kind: "base/text" };
    const scalar: Element = { id: "t", kind: "base/chart", params: { option: "x" } };
    expect([chartSource(missing, "option"), chartKind(missing, "option")]).toEqual([null, null]);
    expect([chartSource(scalar, "option"), chartKind(scalar, "option")]).toEqual([null, null]);
  });

  test("chartSource d'un graphique de pack lit son paramètre", () => {
    const el = { id: "c", kind: "acme/plot", params: { series: { dataset: { source: [["x"], [1]] } } } };
    expect(chartSource(el, "series")).toEqual([["x"], [1]]);
  });

  test.each(["__proto__", "constructor"])("chartSource d'un paramètre %s absent rend null", (param) => {
    expect(chartSource({ id: "c", kind: "acme/plot", params: {} }, param)).toBeNull();
  });
});

describe("checkboxColumns", () => {
  test("la colonne Total d'une cascade se coche", () => {
    const el = canonical([{ type: "waterfall" }], [["Poste", "Montant", "Total"], ["Fin", null, true]]);
    expect(checkboxColumns(el, "option")).toEqual([2]);
  });

  test("une cascade sans colonne Total n'a pas de case", () => {
    expect(checkboxColumns(canonical([{ type: "waterfall" }], [["Poste", "Montant"]]), "option")).toEqual([]);
  });

  test("des barres n'ont pas de case", () => {
    expect(checkboxColumns(canonical([{ type: "bar" }, { type: "bar" }]), "option")).toEqual([]);
  });
});

test("les six types sont proposés", () => {
  expect([...CHART_KINDS]).toEqual(["bar", "line", "pie", "scatter", "radar", "waterfall"]);
});

describe("empilement", () => {
  test("des barres ou des lignes à plusieurs séries s'empilent", () => {
    expect(stackable(canonical([{ type: "bar" }, { type: "bar" }]), "option")).toBe(true);
    expect(stackable(canonical([{ type: "line" }, { type: "line" }]), "option")).toBe(true);
  });

  test("une seule série, un camembert ou un nuage ne s'empilent pas", () => {
    expect(stackable(canonical([{ type: "bar" }]), "option")).toBe(false);
    expect(stackable(canonical([{ type: "pie" }]), "option")).toBe(false);
    expect(stackable(canonical([{ type: "scatter" }, { type: "scatter" }]), "option")).toBe(false);
  });

  test("empilé quand toutes les séries partagent un groupe", () => {
    expect(isStacked(canonical([{ type: "bar", stack: "total" }, { type: "bar", stack: "total" }]), "option")).toBe(true);
    expect(isStacked(canonical([{ type: "bar", stack: "total" }, { type: "bar" }]), "option")).toBe(false);
  });

  test("stackPatch met toutes les séries dans un groupe, en gardant leurs réglages", () => {
    const el = canonical([{ type: "bar", label: { show: true } }, { type: "line" }]);
    expect(stackPatch(el, "option", true)).toEqual({
      series: [{ type: "bar", label: { show: true }, stack: "total" }, { type: "line", stack: "total" }],
    });
  });

  test("stackPatch sans empilement retire le groupe", () => {
    const el = canonical([{ type: "bar", stack: "total" }, { type: "bar", stack: "total" }]);
    expect(stackPatch(el, "option", false)).toEqual({ series: [{ type: "bar" }, { type: "bar" }] });
  });
});

describe("couleurs des séries", () => {
  test("barres et lignes : une série par colonne de valeurs, nommée par son en-tête", () => {
    expect(seriesNames(canonical([{ type: "bar" }, { type: "line" }]), "option")).toEqual(["Ventes", "Objectif"]);
  });

  test("nuage : une série par paire de colonnes, nommée par la colonne des y", () => {
    const el = canonical([{ type: "scatter" }], [["x de A", "A", "x de B", "B"], [1, 2, 3, 4]]);
    expect(seriesNames({ ...el, params: { option: { ...(el.params?.option as object), series: [{ type: "scatter" }, { type: "scatter" }] } } }, "option")).toEqual(["A", "B"]);
  });

  test("secteurs, radar et cascade suivent la palette : aucune série à colorer", () => {
    expect(seriesNames(canonical([{ type: "pie" }]), "option")).toEqual([]);
    expect(seriesNames(canonical([{ type: "radar" }]), "option")).toEqual([]);
    expect(seriesNames(canonical([{ type: "waterfall" }]), "option")).toEqual([]);
  });

  test("seriesColor lit itemStyle.color", () => {
    const el = canonical([{ type: "bar" }, { type: "bar", itemStyle: { color: "#C4622D" } }]);
    expect([seriesColor(el, "option", 0), seriesColor(el, "option", 1)]).toEqual([null, "#c4622d"]);
  });

  test("colorPatch colore une série et garde les autres réglages", () => {
    const el = canonical([{ type: "bar", stack: "total" }, { type: "bar" }]);
    expect(colorPatch(el, "option", 0, "#5B8A3C")).toEqual({
      series: [{ type: "bar", stack: "total", itemStyle: { color: "#5B8A3C" } }, { type: "bar" }],
    });
  });

  test("colorPatch sans couleur rend la série à la palette", () => {
    const el = canonical([{ type: "bar", itemStyle: { color: "#5B8A3C" } }]);
    expect(colorPatch(el, "option", 0, null)).toEqual({ series: [{ type: "bar" }] });
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
    expect(seriesColor(el, "option", 0)).toBeNull();
  });

  test("seriesColor rend la couleur normalisée", () => {
    expect(seriesColor(canonical([{ type: "bar", itemStyle: { color: "#ABC" } }]), "option", 0)).toBe("#aabbcc");
  });
});

describe("paletteOf", () => {
  test("garde les couleurs valides de la palette du thème, normalisées", () => {
    expect(paletteOf(["#3F6B5E", "rouge", "#abc", 4])).toEqual(["#3f6b5e", "#aabbcc"]);
  });

  test("sans palette, rien", () => {
    expect(paletteOf(undefined)).toEqual([]);
  });
});
