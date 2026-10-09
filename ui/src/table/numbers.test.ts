import { describe, expect, test } from "bun:test";
import { formatNumber, parseNumber } from "./numbers";

describe("formatNumber", () => {
  test("formatNumber fr groups with a narrow no-break space from 10000", () => {
    expect(formatNumber(12345.5, "fr")).toBe("12 345,5");
    expect(formatNumber(1234.5, "fr")).toBe("1234,5");
  });

  test("formatNumber en groups with commas", () => {
    expect(formatNumber(12345.5, "en")).toBe("12,345.5");
    expect(formatNumber(1234567, "en-GB")).toBe("1,234,567");
  });

  test("formatNumber de, ru and ar follow the render separators", () => {
    expect(formatNumber(12345.5, "de-CH")).toBe("12.345,5");
    expect(formatNumber(100000, "ru")).toBe("100 000");
    expect(formatNumber(12345.5, "ar")).toBe("12,345.5");
    expect(formatNumber(123456, "hi")).toBe("123,456");
  });

  test("formatNumber negatives use the minus sign", () => {
    expect(formatNumber(-12345.5, "fr")).toBe("\u221212\u202f345,5");
    expect(formatNumber(-0.25, "en")).toBe("\u22120.25");
    expect(formatNumber(-12345, "de")).toBe("\u221212.345");
  });

  test("formatNumber negatives without known language use the minus sign without groups", () => {
    expect(formatNumber(-12345.5, "xx")).toBe("\u221212345.5");
    expect(formatNumber(-0.25, "")).toBe("\u22120.25");
  });

  test("formatNumber writes the shortest round-trip decimals, like the slide", () => {
    expect(formatNumber(0.1 + 0.2, "fr")).toBe("0,30000000000000004");
    expect(formatNumber(0.1 + 0.2, "xx")).toBe("0.30000000000000004");
    expect(formatNumber(1 / 3, "en")).toBe("0.3333333333333333");
    expect(formatNumber(42, "fr")).toBe("42");
  });

  test("formatNumber never writes scientific notation", () => {
    expect(formatNumber(1e-7, "xx")).toBe("0.0000001");
    expect(formatNumber(1.5e-10, "fr")).toBe("0,00000000015");
    expect(formatNumber(1e15, "xx")).toBe("1000000000000000");
    expect(formatNumber(1e21, "xx")).toBe("1000000000000000000000");
    expect(formatNumber(1.5e21, "en")).toBe("1,500,000,000,000,000,000,000");
    expect(formatNumber(-1e21, "xx")).toBe("\u22121000000000000000000000");
  });

  test("formatNumber unknown language is plain", () => {
    expect(formatNumber(12345.5, "xx")).toBe("12345.5");
  });
});

describe("parseNumber", () => {
  test("parseNumber reads back what formatNumber writes", () => {
    for (const lang of ["fr", "en", "de", "ru", "pl", "ar", "xx"])
      for (const n of [12345.5, -1234567.25, 0.5, 42, 0.1 + 0.2, -0.25])
        expect(parseNumber(formatNumber(n, lang), lang)).toBe(n);
  });

  test("parseNumber en reads comma groups, fr keeps comma as decimal", () => {
    expect(parseNumber("12,345", "en")).toBe(12345);
    expect(parseNumber("12,345", "fr")).toBe(12.345);
    expect(parseNumber("12.345", "de")).toBe(12345);
  });

  test("parseNumber reads dot or comma groups only from 10000, like the display", () => {
    expect(parseNumber("3.141", "de")).toBe(3.141);
    expect(parseNumber("12.345", "de")).toBe(12345);
    expect(parseNumber("1.234.567,5", "es")).toBe(1234567.5);
    expect(parseNumber("12,345", "en")).toBe(12345);
    expect(parseNumber("1,234", "en")).toBe(1.234);
    expect(parseNumber("1234.5", "en")).toBe(1234.5);
  });

  test("parseNumber reads both the ascii and the typographic minus", () => {
    expect(parseNumber("-12 345,5", "fr")).toBe(-12345.5);
    expect(parseNumber("\u221212\u202f345,5", "fr")).toBe(-12345.5);
    expect(parseNumber("\u221212,345.5", "en")).toBe(-12345.5);
    expect(parseNumber("\u22120.25", "xx")).toBe(-0.25);
  });

  test("parseNumber leaves text and empty cells to parseCell", () => {
    expect(parseNumber("Jan", "en")).toBe("Jan");
    expect(parseNumber("  ", "en")).toBeNull();
  });
});
