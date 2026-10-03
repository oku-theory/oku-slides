import { describe, expect, test } from "bun:test";
import { fitColumns } from "./columns";

describe("fitColumns", () => {
  test("fitColumns largeurs dans les bornes les garde", () => {
    expect(fitColumns(1440, 240, 500)).toEqual({ pile: 240, notes: 500 });
  });

  test("fitColumns pile trop étroite ou trop large la borne à [180, 360]", () => {
    expect(fitColumns(1440, 80, 500).pile).toBe(180);
    expect(fitColumns(1440, 600, 500).pile).toBe(360);
  });

  test("fitColumns fil trop étroit le remonte à 300", () => {
    expect(fitColumns(1440, 240, 120).notes).toBe(300);
  });

  test("fitColumns fil trop large laisse 320 px à la feuille", () => {
    expect(fitColumns(1440, 240, 1200).notes).toBe(1440 - 240 - 320);
  });

  test("fitColumns table étroite réduit la pile avant d'écraser la feuille", () => {
    expect(fitColumns(900, 360, 300)).toEqual({ pile: 280, notes: 300 });
  });

  test("fitColumns largeurs fractionnaires les arrondit", () => {
    expect(fitColumns(1440, 240.6, 500.4)).toEqual({ pile: 241, notes: 500 });
  });
});
