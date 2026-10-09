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

  test("formatNumber negatives keep the ascii sign", () => {
    expect(formatNumber(-12345.5, "fr")).toBe("-12 345,5");
  });

  test("formatNumber unknown language is plain", () => {
    expect(formatNumber(12345.5, "xx")).toBe("12345.5");
  });
});

describe("parseNumber", () => {
  test("parseNumber reads back what formatNumber writes", () => {
    for (const lang of ["fr", "en", "de", "ru", "pl", "ar", "xx"])
      for (const n of [12345.5, -1234567.25, 0.5, 42])
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

  test("parseNumber leaves text and empty cells to parseCell", () => {
    expect(parseNumber("Jan", "en")).toBe("Jan");
    expect(parseNumber("  ", "en")).toBeNull();
  });
});
