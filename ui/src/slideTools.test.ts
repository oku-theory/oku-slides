import { describe, expect, test } from "bun:test";
import type { AddWidget, SlideKind } from "./api";
import { baseName } from "./baseKinds";
import { ADD_NAMES, canAdd, canRemoveSlide, packOnlyWidgets } from "./slideTools";

describe("canAdd", () => {
  test("sur une slide de contenu, tout s'ajoute", () => {
    expect(ADD_NAMES.filter((k) => canAdd(k, "content"))).toEqual(["text", "card", "shape", "chart", "table"]);
  });

  test("sur une slide vide, tout sauf le texte", () => {
    expect(ADD_NAMES.filter((k) => canAdd(k, "blank"))).toEqual(["card", "shape", "chart", "table"]);
  });

  test("sur les autres slides, rien", () => {
    const others: SlideKind[] = ["title", "toc", "chapter", "end"];
    expect(others.flatMap((s) => ADD_NAMES.filter((k) => canAdd(k, s)))).toEqual([]);
  });

  test("un texte de base copié ne se colle pas sur une slide vide", () => {
    expect(canAdd(baseName("base/text") === "text" ? "text" : "widget", "blank")).toBe(false);
  });

  test("sans slide affichée, rien", () => {
    expect(ADD_NAMES.filter((k) => canAdd(k, undefined))).toEqual([]);
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
    expect([canAdd("widget", "content"), canAdd("widget", "blank"), canAdd("widget", "title")]).toEqual([true, true, false]);
  });
});
