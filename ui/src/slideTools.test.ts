import { describe, expect, test } from "bun:test";
import type { SlideKind } from "./api";
import { ADD_KINDS, canAdd, canRemoveSlide } from "./slideTools";

describe("canAdd", () => {
  test("sur une slide de contenu, tout s'ajoute", () => {
    expect(ADD_KINDS.filter((k) => canAdd(k, "content"))).toEqual(["text", "card", "shape", "chart"]);
  });

  test("sur une slide vide, tout sauf le texte", () => {
    expect(ADD_KINDS.filter((k) => canAdd(k, "blank"))).toEqual(["card", "shape", "chart"]);
  });

  test("sur les autres slides, rien", () => {
    const others: SlideKind[] = ["title", "toc", "chapter", "end"];
    expect(others.flatMap((s) => ADD_KINDS.filter((k) => canAdd(k, s)))).toEqual([]);
  });

  test("un élément collé suit la même règle, image et groupe compris", () => {
    expect([canAdd("image", "blank"), canAdd("group", "content"), canAdd("group", "title")]).toEqual([true, true, false]);
  });

  test("sans slide affichée, rien", () => {
    expect(ADD_KINDS.filter((k) => canAdd(k, undefined))).toEqual([]);
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
