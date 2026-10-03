import { describe, expect, test } from "bun:test";
import type { WidgetStatus } from "./api";
import { COMMON, diagnosticPlace, fileDiagnostics, isWidgetName, listedDiagnostics, merged, nameAvailable, ordered, skeleton, stateOf, withTrial } from "./widgetList";

const status = (name: string, error = false): WidgetStatus => ({
  name,
  schema: null,
  diagnostics: error ? [{ file: name, line: 1, column: 1, error: true, message: "x" }] : [],
});

describe("widgetList", () => {
  test("_commun en tête, puis par nom", () => {
    expect(ordered([status("kpi"), status(COMMON), status("a")]).map((s) => s.name)).toEqual([COMMON, "a", "kpi"]);
  });

  test("statuts fusionnés : ceux rendus remplacent ou s'ajoutent, dans l'ordre", () => {
    const out = merged([status(COMMON), status("kpi")], [status("kpi", true), status("a")]);
    expect(out.map((s) => [s.name, stateOf(s)])).toEqual([[COMMON, "valid"], ["a", "valid"], ["kpi", "errors"]]);
    expect(merged(null, [status("kpi")]).map((s) => s.name)).toEqual(["kpi"]);
  });

  test("nom saisi : admis et libre, `_commun` seulement créé ; refusé tant que la liste est inconnue", () => {
    const list = [status(COMMON), status("kpi")];
    expect(nameAvailable(list, "quote", null)).toBe(true);
    expect(nameAvailable(list, "kpi", null)).toBe(false);
    expect(nameAvailable(list, "kpi", "kpi")).toBe(true);
    // `_commun` : créé s'il manque, jamais en double ni par un renommage.
    expect(nameAvailable(list, COMMON, null)).toBe(false);
    expect(nameAvailable([status("kpi")], COMMON, null)).toBe(true);
    expect(nameAvailable([status("kpi")], COMMON, "kpi")).toBe(false);
    expect(nameAvailable(list, "Kpi", null)).toBe(false);
    expect(nameAvailable(list, "", null)).toBe(false);
    expect(nameAvailable(null, "quote", null)).toBe(false);
  });

  test("relecture et essai : les diagnostics de l'essai manquants s'ajoutent, sans doublon ni fichier disparu", () => {
    const read = [status("kpi", true), status("quote")];
    const trial = [status("kpi", true), { ...status("quote"), diagnostics: [{ file: "quote", line: 2, column: 1, error: true, message: "rendu" }] }, status("gone", true)];
    const out = withTrial(read, trial);
    expect(out.map((s) => [s.name, s.diagnostics.length])).toEqual([["kpi", 1], ["quote", 1]]);
    expect(out[1]?.diagnostics[0]?.message).toBe("rendu");
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
