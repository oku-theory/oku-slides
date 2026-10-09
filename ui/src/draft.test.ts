import { describe, expect, test } from "bun:test";
import type { Snapshot } from "./api";
import { appendDraft, deckVocabulary, dictationRoute } from "./draft";

describe("brouillon de consigne", () => {
  test("appendDraft sans brouillon ouvre le brouillon avec la dictée", () => {
    expect(appendDraft(null, "rends ce titre plus court")).toBe("rends ce titre plus court");
  });

  test("appendDraft sur un brouillon ajoute la dictée après une espace", () => {
    expect(appendDraft("rends ce titre", "plus court")).toBe("rends ce titre plus court");
  });

  test("appendDraft sur un brouillon qui finit par un blanc ne double pas l'espace", () => {
    expect(appendDraft("rends ce titre\n", "plus court")).toBe("rends ce titre plus court");
  });

  test("appendDraft sur un brouillon vide rend la dictée seule", () => {
    expect(appendDraft("  ", "plus court")).toBe("plus court");
  });
});

describe("où va une dictée", () => {
  test("dictationRoute auto pendant l'édition d'un texte va au curseur", () => {
    expect(dictationRoute("auto", true)).toBe("cursor");
  });

  test("dictationRoute auto hors édition va au brouillon", () => {
    expect(dictationRoute("auto", false)).toBe("draft");
  });

  test("dictationRoute brouillon pendant l'édition va quand même au brouillon", () => {
    expect(dictationRoute("draft", true)).toBe("draft");
  });
});

test("deckVocabulary prend le titre du deck et ceux des slides, sans vides", () => {
  const snap = { title: "Feuille", slides: [{ title: "Constat" }, { title: null }] } as unknown as Snapshot;
  expect(deckVocabulary(snap)).toEqual(["Feuille", "Constat"]);
});

test("deckVocabulary sans instantané : aucun mot", () => {
  expect(deckVocabulary(null)).toEqual([]);
});
