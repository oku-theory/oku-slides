import { describe, expect, test } from "bun:test";
import { parseCheck, pasteInto } from "./ChartDataGrid";

describe("parseCheck", () => {
  test("une case collée est cochée sauf si elle est vide ou fausse", () => {
    expect(["x", "TRUE", "vrai", "1"].map(parseCheck)).toEqual([true, true, true, true]);
    expect(["", " ", "false", "FAUX", "0", "non", "no"].map(parseCheck)).toEqual([false, false, false, false, false, false, false]);
  });
});

describe("pasteInto", () => {
  test("une colonne de cases reçoit des booléens", () => {
    const source = [["Poste", "Montant", "Total"], ["Début", 100, false]];
    const out = pasteInto(source, [["Fin", "", "x"]], 2, 0, true, [2]);
    expect(out[2]).toEqual(["Fin", null, true]);
  });
});
