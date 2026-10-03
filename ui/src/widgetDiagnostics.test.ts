import { describe, expect, test } from "bun:test";
import type { WidgetDiagnostic } from "./api";
import { diagnosticRange } from "./widgetDiagnostics";

const at = (line: number | null, column: number | null): WidgetDiagnostic => ({
  file: "kpi",
  line,
  column,
  error: true,
  message: "m",
});

describe("diagnosticRange", () => {
  test("ligne 2, colonne 6 : le mot souligné à la bonne place", () => {
    expect(diagnosticRange("#let a = 1\n#let bad = (", at(2, 6))).toEqual({ from: 16, to: 19, line: 2 });
  });

  test("sans colonne : le premier caractère de la ligne", () => {
    expect(diagnosticRange("x\ny = (", at(2, null))).toEqual({ from: 2, to: 3, line: 2 });
  });

  test("hors du texte ou sans ligne : rien", () => {
    expect(diagnosticRange("x", at(5, 1))).toBeNull();
    expect(diagnosticRange("x", at(null, null))).toBeNull();
  });
});
