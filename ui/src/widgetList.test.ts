import { describe, expect, test } from "bun:test";
import type { WidgetStatus } from "./api";
import { COMMON, diagnosticPlace, fileDiagnostics, isWidgetName, listedDiagnostics, ordered, skeleton, stateOf } from "./widgetList";

const status = (name: string, error = false): WidgetStatus => ({
  name,
  schema: null,
  diagnostics: error ? [{ file: name, line: 1, column: 1, error: true, message: "x" }] : [],
});

describe("widgetList", () => {
  test("_commun en tête, puis par nom", () => {
    expect(ordered([status("kpi"), status(COMMON), status("a")]).map((s) => s.name)).toEqual([COMMON, "a", "kpi"]);
  });

  test("état : valide, ou erreurs", () => {
    expect(stateOf(status("kpi"))).toBe("valid");
    expect(stateOf(status("kpi", true))).toBe("errors");
  });

  test("noms admis : ceux de la session", () => {
    expect(isWidgetName("kpi-2")).toBe(true);
    expect(isWidgetName(COMMON)).toBe(true);
    expect(isWidgetName("Kpi")).toBe(false);
    expect(isWidgetName("_autre")).toBe(false);
  });

  test("diagnostics du seul fichier montré", () => {
    const s: WidgetStatus = {
      name: "kpi",
      schema: null,
      diagnostics: [
        { file: "kpi", line: 1, column: 1, error: true, message: "ici" },
        { file: COMMON, line: 2, column: 1, error: true, message: "là" },
      ],
    };
    expect(fileDiagnostics(s, "kpi").map((d) => d.message)).toEqual(["ici"]);
  });

  test("squelette : déclare le nom de son fichier", () => {
    expect(skeleton("kpi")).toContain('widget("kpi"');
    expect(skeleton(COMMON)).not.toContain("#let widget");
  });

  test("liste des diagnostics : ceux de _commun compris, sans doublon avec l'aperçu", () => {
    const common = { file: COMMON, line: 2, column: 1, error: true, message: "là" };
    const own = { file: "kpi", line: 1, column: 1, error: true, message: "ici" };
    const late = { file: "kpi", line: null, column: null, error: false, message: "rendu" };
    const s: WidgetStatus = { name: "kpi", schema: null, diagnostics: [common, own] };
    expect(listedDiagnostics(s, [{ ...own }, late])).toEqual([common, own, late]);
    expect(listedDiagnostics(undefined, [late])).toEqual([late]);
  });

  test("lieu d'un diagnostic : fichier, ligne et colonne", () => {
    expect(diagnosticPlace({ file: COMMON, line: 3, column: 2, error: true, message: "m" })).toBe("_commun:3:2");
    expect(diagnosticPlace({ file: "kpi", line: 3, column: null, error: true, message: "m" })).toBe("kpi:3:1");
    expect(diagnosticPlace({ file: COMMON, line: null, column: null, error: true, message: "m" })).toBe("_commun");
    expect(diagnosticPlace({ file: null, line: null, column: null, error: true, message: "m" })).toBe("");
  });
});
