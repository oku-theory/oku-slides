import { describe, expect, test } from "bun:test";
import type { HostParams } from "./api";
import { ownPanel } from "./hostParams";

const params: HostParams = {
  "base/chart": { chart: "option" },
  "acme/plot": { chart: "series", table: "grid" },
  "base/shape": { path: "path" },
  "acme/grid": { table: "grid" },
};

describe("ownPanel", () => {
  test("d'un graphique de pack ouvre le panneau graphique sur son paramètre", () =>
    expect(ownPanel(params, "acme/plot")).toEqual({ panel: "chart", param: "series" }));
  test("d'un tableau de pack ouvre le panneau tableau sur son paramètre", () =>
    expect(ownPanel(params, "acme/grid")).toEqual({ panel: "table", param: "grid" }));
  test("d'une forme n'ouvre aucun panneau", () => expect(ownPanel(params, "base/shape")).toBeNull());
  test("d'un kind absent n'ouvre aucun panneau", () => expect(ownPanel(params, "base/text")).toBeNull());
  test("sans élément n'ouvre aucun panneau", () => expect(ownPanel(params, undefined)).toBeNull());
  test.each(["constructor", "__proto__", "toString"])("de %s n'ouvre aucun panneau", (k) => expect(ownPanel(params, k)).toBeNull());
});
