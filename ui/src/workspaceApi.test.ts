import { describe, expect, test } from "bun:test";
import { type TabDocument, type TabInfo, tabPath } from "./workspaceApi";

const tab = (document: TabDocument): TabInfo => ({
  id: "t-1",
  title: "essai",
  dirty: false,
  pristine: false,
  editing_pack: false,
  theme: null,
  document,
});

describe("tabPath", () => {
  test("a deck with a file: its path", () => {
    expect(tabPath(tab({ kind: "deck", path: "/d/a.oku-slides.pdf", deck_uid: "u" }))).toBe("/d/a.oku-slides.pdf");
  });

  test("a deck without a file: null", () => {
    expect(tabPath(tab({ kind: "deck", path: null, deck_uid: null }))).toBeNull();
  });

  test("a help tab: null", () => {
    expect(tabPath(tab({ kind: "help", locale: "fr" }))).toBeNull();
  });

  test("a pack tab: null", () => {
    expect(tabPath(tab({ kind: "pack", pack: "a7e11e40-0000-4000-8000-000000000001" }))).toBeNull();
  });

  test("an unknown kind (a newer host): null, not a throw", () => {
    const unknown = { kind: "slides", path: "/x" } as unknown as TabDocument;
    expect(tabPath(tab(unknown))).toBeNull();
  });
});
