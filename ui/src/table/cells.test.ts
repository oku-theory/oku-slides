import { describe, expect, test } from "bun:test";
import { coerce, parseCell, parseCheck, parseTsv } from "./cells";

describe("parseCheck", () => {
  test("une case collée est cochée sauf si elle est vide ou fausse", () => {
    expect(["x", "TRUE", "vrai", "1"].map(parseCheck)).toEqual([true, true, true, true]);
    expect(["", " ", "false", "FAUX", "0", "non", "no"].map(parseCheck)).toEqual([false, false, false, false, false, false, false]);
  });
});

describe("parseCell", () => {
  test("nombre à espaces et virgule, vide, texte", () => {
    expect(parseCell("1 234,5")).toBe(1234.5);
    expect(parseCell("  ")).toBeNull();
    expect(parseCell(" abc ")).toBe("abc");
  });
});

describe("parseTsv", () => {
  test("lignes et tabulations, fin de ligne Windows et saut final ignorés", () => {
    expect(parseTsv("a\tb\r\nc\td\n")).toEqual([["a", "b"], ["c", "d"]]);
  });
});

describe("coerce", () => {
  test("coerce : texte rogné, nombre à virgule, case, choix hors liste refusé", () => {
    expect(coerce("text", " a ")).toEqual({ ok: true, value: "a" });
    expect(coerce("number", "3,5")).toEqual({ ok: true, value: 3.5 });
    expect(coerce("flag", "oui")).toEqual({ ok: true, value: true });
    expect(coerce("select", "z", ["a", "b"])).toEqual({ ok: false });
    expect(coerce("number", "abc")).toEqual({ ok: false });
  });

  test("coerce : choix connu accepté, asset refusé, rich gardé en texte", () => {
    expect(coerce("select", " b ", ["a", "b"])).toEqual({ ok: true, value: "b" });
    expect(coerce("asset", "sha256:abc")).toEqual({ ok: false });
    expect(coerce("rich", " Un ")).toEqual({ ok: true, value: "Un" });
  });
});
