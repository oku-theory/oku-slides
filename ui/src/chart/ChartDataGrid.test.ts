import { describe, expect, test } from "bun:test";
import { pasteInto } from "./ChartDataGrid";

describe("pasteInto", () => {
  test("une colonne de cases reçoit des booléens", () => {
    const source = [["Poste", "Montant", "Total"], ["Début", 100, false]];
    const out = pasteInto(source, [["Fin", "", "x"]], 2, 0, true, [2]);
    expect(out[2]).toEqual(["Fin", null, true]);
  });
});
