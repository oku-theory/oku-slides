import { describe, expect, test } from "bun:test";
import { pasteInto } from "./ChartDataGrid";

describe("pasteInto", () => {
  test("une colonne de cases reçoit des booléens", () => {
    const source = [["Poste", "Montant", "Total"], ["Début", 100, false]];
    const out = pasteInto(source, [["Fin", "", "x"]], 2, 0, true, [2]);
    expect(out[2]).toEqual(["Fin", null, true]);
  });
});

describe("pasteInto numbers", () => {
  test("pasteInto reads the deck language groups like typed input", () => {
    const source = [["Mois", "Ventes"], ["Jan", 1]];
    const out = pasteInto(source, [["12,345.5"]], 1, 1, true, [], undefined, "en");
    expect(out[1][1]).toBe(12345.5);
  });
});
