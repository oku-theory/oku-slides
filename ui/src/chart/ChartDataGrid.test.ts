import { describe, expect, test } from "bun:test";
import { type CellKind, pasteCells } from "./ChartDataGrid";

/** A view's kinds: column 0 a label, `checks` checkboxes, the rest numbers. */
const viewKinds = (checks: readonly number[] = []) => (c: number): CellKind =>
  c === 0 ? "label" : checks.includes(c) ? "check" : "number";

describe("pasteCells", () => {
  test("pasteCells checkbox column reads booleans", () => {
    const out = pasteCells([["Fin", "", "x"]], 0, 3, viewKinds([2]));
    expect(out.cells).toEqual([["Fin", null, true]]);
  });

  test("pasteCells reads the deck language groups like typed input", () => {
    const out = pasteCells([["12,345.5"]], 1, 2, viewKinds(), undefined, "en");
    expect(out.cells).toEqual([[12345.5]]);
  });

  test("pasteCells past the last column names only the added columns", () => {
    const out = pasteCells([["1", "2", "3"]], 1, 2, viewKinds(), (n) => `S${n}`);
    expect(out).toEqual({ cells: [[1, 2, 3]], names: ["S2", "S3"] });
  });

  test("pasteCells inside the columns adds no name", () => {
    expect(pasteCells([["1"], ["2"]], 1, 3, viewKinds()).names).toEqual([]);
  });

  test("pasteCells label column keeps digits as text", () => {
    const out = pasteCells([["007", "8"]], 0, 2, (c) => (c === 0 ? "label" : "number"));
    expect(out.cells).toEqual([["007", 8]]);
  });
});
