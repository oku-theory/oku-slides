import { describe, expect, test } from "bun:test";
import { acceptsElements, canAdd, canRemoveSlide } from "./slideTools";

const slots = { "essentiel/content": null, "essentiel/blank": null, "acme/split": ["base/text"], "acme/none": [] };

describe("canAdd", () => {
  test.each([
    ["essentiel/content", "base/card", true],
    ["essentiel/blank", "base/text", true],
    ["essentiel/title", "base/card", false],
    ["acme/split", "base/text", true],
    ["acme/split", "base/card", false],
    ["acme/none", "base/text", false],
  ])("canAdd on %s with %s gives %s", (slide, kind, expected) => {
    expect(canAdd(slide, kind, slots)).toBe(expected);
  });

  test("canAdd without a shown slide refuses", () => {
    expect(canAdd(undefined, "base/text", slots)).toBe(false);
  });

  test("canAdd on a slide kind named like an Object property refuses", () => {
    expect(canAdd("constructor", "base/text", slots)).toBe(false);
  });
});

describe("acceptsElements", () => {
  test("acceptsElements on a layout without free slot gives false", () => {
    expect(acceptsElements("essentiel/title", slots)).toBe(false);
  });

  test("acceptsElements on a free slot accepting nothing gives true", () => {
    expect(acceptsElements("acme/none", slots)).toBe(true);
  });
});

describe("canRemoveSlide", () => {
  test("avec une seule slide, la suppression est refusée", () => {
    expect(canRemoveSlide(1)).toBe(false);
  });

  test("avec deux slides, la suppression est permise", () => {
    expect(canRemoveSlide(2)).toBe(true);
  });
});
