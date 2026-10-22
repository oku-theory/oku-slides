import { describe, expect, test } from "bun:test";
import type { AddWidget, SlideKind } from "./api";
import { ESSENTIEL, baseName } from "./builtinKinds";
import { ADD_NAMES, canAdd, canRemoveSlide, packOnlyWidgets } from "./slideTools";

describe("canAdd", () => {
  test("canAdd on essentiel content accepts any native widget", () => {
    expect(canAdd(ESSENTIEL.content, "text")).toBe(true);
    expect(ADD_NAMES.filter((k) => canAdd(ESSENTIEL.content, k))).toEqual(["text", "card", "shape", "chart", "table"]);
  });

  test("canAdd on essentiel blank accepts all but text", () => {
    expect(ADD_NAMES.filter((k) => canAdd(ESSENTIEL.blank, k))).toEqual(["card", "shape", "chart", "table"]);
  });

  test("canAdd on the other essentiel layouts refuses", () => {
    const others: SlideKind[] = [ESSENTIEL.title, ESSENTIEL.toc, ESSENTIEL.chapter, ESSENTIEL.end];
    expect(others.flatMap((s) => ADD_NAMES.filter((k) => canAdd(s, k)))).toEqual([]);
  });

  test.each(["content", "blank"])("canAdd on a bare %s kind refuses", (kind) => {
    expect(canAdd(kind, "card")).toBe(false);
  });

  test("canAdd refuses a copied base text on essentiel blank", () => {
    expect(canAdd(ESSENTIEL.blank, baseName("base/text") === "text" ? "text" : "widget")).toBe(false);
  });

  test("canAdd without a shown slide refuses", () => {
    expect(ADD_NAMES.filter((k) => canAdd(undefined, k))).toEqual([]);
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

describe("packOnlyWidgets", () => {
  test("packOnlyWidgets écarte tous les widgets de base", () => {
    const w = (name: string): AddWidget => ({ name, label: name, description: "" });
    expect(packOnlyWidgets([w("base/card"), w("base/image"), w("base/stack"), w("acme/card")]).map((x) => x.name)).toEqual(["acme/card"]);
  });
});

describe("canAdd widget", () => {
  test("canAdd un widget du pack suit la règle des éléments non textuels", () => {
    expect([canAdd(ESSENTIEL.content, "widget"), canAdd(ESSENTIEL.blank, "widget"), canAdd(ESSENTIEL.title, "widget")]).toEqual([true, true, false]);
  });
});
