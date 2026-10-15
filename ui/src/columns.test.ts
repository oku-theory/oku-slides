import { describe, expect, test } from "bun:test";
import { fitIn } from "./bounds";
import { type Columns, columnBounds } from "./columns";

/** Largeurs bornées de la pile et du fil, comme au séparateur de chacune. */
const fitColumns = (table: number, pile: number, notes: number): Columns => ({
  pile: fitIn(columnBounds("pile", table, pile), pile),
  notes: fitIn(columnBounds("notes", table, pile), notes),
});

describe("columnBounds", () => {
  test("columnBounds largeurs dans les bornes les garde", () => {
    expect(fitColumns(1440, 240, 500)).toEqual({ pile: 240, notes: 500 });
  });

  test("columnBounds pile trop étroite ou trop large la borne à [180, 360]", () => {
    expect(fitColumns(1440, 80, 500).pile).toBe(180);
    expect(fitColumns(1440, 600, 500).pile).toBe(360);
  });

  test("columnBounds fil trop étroit le remonte à 300", () => {
    expect(fitColumns(1440, 240, 120).notes).toBe(300);
  });

  test("columnBounds fil trop large laisse 320 px à la feuille", () => {
    expect(fitColumns(1440, 240, 1200).notes).toBe(1440 - 240 - 320);
  });

  test("columnBounds table étroite réduit la pile avant d'écraser la feuille", () => {
    expect(fitColumns(900, 360, 300)).toEqual({ pile: 280, notes: 300 });
  });

  test("columnBounds largeurs fractionnaires les arrondit", () => {
    expect(fitColumns(1440, 240.6, 500.4)).toEqual({ pile: 241, notes: 500 });
  });
});
