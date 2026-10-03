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

  test("colonne en caractères : un emoji avant compte pour un, en deux unités UTF-16", () => {
    // « #let 😀 = » : 9 caractères, 10 unités ; `bad` est en colonne 10.
    expect(diagnosticRange("x\n#let 😀 = bad(", at(2, 10))).toEqual({ from: 12, to: 15, line: 2 });
  });

  test("un caractère hors du plan de base sous la colonne : souligné en entier", () => {
    expect(diagnosticRange("a 😀 b", at(1, 3))).toEqual({ from: 2, to: 4, line: 1 });
  });

  test("en fin de ligne : le dernier caractère est souligné, sans passer à la ligne suivante", () => {
    expect(diagnosticRange("x = (\ny", at(1, 6))).toEqual({ from: 4, to: 5, line: 1 });
    expect(diagnosticRange("a 😀\ny", at(1, 4))).toEqual({ from: 2, to: 4, line: 1 });
  });

  test("sur une ligne vide : une marque ponctuelle au début de la ligne", () => {
    expect(diagnosticRange("x\n\ny", at(2, 1))).toEqual({ from: 2, to: 2, line: 2 });
  });
});
