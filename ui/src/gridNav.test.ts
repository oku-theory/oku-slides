// Déplacement dans la grille d'ajout, sans DOM.

import { describe, expect, test } from "bun:test";
import { gridStep, rowLength, sectionSlots } from "./gridNav";

/** 10 cartes en 4 colonnes, toutes ajoutables sauf celles de `refused`. */
const step = (at: number, key: Parameters<typeof gridStep>[1], refused: number[] = []) =>
  gridStep(at, key, 4, 10, (i) => !refused.includes(i));

describe("gridStep", () => {
  test("gridStep right moves to the next card", () => {
    expect(step(1, "right")).toBe(2);
  });

  test("gridStep right at the end of a row wraps to the next row", () => {
    expect(step(3, "right")).toBe(4);
  });

  test("gridStep left on the first card stays", () => {
    expect(step(0, "left")).toBe(0);
  });

  test("gridStep down moves one row below, same column", () => {
    expect(step(1, "down")).toBe(5);
  });

  test("gridStep up moves one row above, same column", () => {
    expect(step(6, "up")).toBe(2);
  });

  test("gridStep down on the last row stays", () => {
    expect(step(9, "down")).toBe(9);
  });

  test("gridStep down onto a refused card goes on down the column", () => {
    expect(step(1, "down", [5])).toBe(9);
  });

  test("gridStep right skips refused cards", () => {
    expect(step(1, "right", [2, 3])).toBe(4);
  });

  test("gridStep with no focused card stays without focus", () => {
    expect(step(-1, "down")).toBe(-1);
  });

  test("gridStep with zero columns stays", () => {
    expect(gridStep(1, "down", 0, 10, (i) => i !== 1)).toBe(1);
  });

  test("gridStep down from a column empty in the last row stays", () => {
    expect(step(6, "down")).toBe(6);
  });

  test("gridStep down with every card below refused stays", () => {
    expect(step(1, "down", [5, 9])).toBe(1);
  });

  test("gridStep left at the start of a row goes to the end of the row above", () => {
    expect(step(4, "left")).toBe(3);
  });

  test("gridStep right on the last card stays", () => {
    expect(step(9, "right")).toBe(9);
  });

  test("gridStep up on the first row stays", () => {
    expect(step(2, "up")).toBe(2);
  });
});

describe("rowLength", () => {
  test("rowLength counts the cards on the first row", () => {
    expect(rowLength([0, 0, 0, 100, 100])).toBe(3);
  });

  test("rowLength of a single row is its length", () => {
    expect(rowLength([0, 0])).toBe(2);
  });

  test("rowLength without cards is one", () => {
    expect(rowLength([])).toBe(1);
  });
});

describe("sectionSlots", () => {
  test("sectionSlots pads each section to full rows", () => {
    expect(sectionSlots([5, 2], 3)).toEqual([0, 1, 2, 3, 4, null, 5, 6, null]);
  });

  test("sectionSlots skips an empty section", () => {
    expect(sectionSlots([0, 2], 3)).toEqual([0, 1, null]);
  });
});
