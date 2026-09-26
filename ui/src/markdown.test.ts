import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import type { Paragraph } from "./api";
import { insertLink, markdownToRuns, sameFormatting, toggleEmphasis } from "./markdown";
import { textToParagraphs } from "./deck";

interface Case {
  name: string;
  markdown: string;
  paragraphs: Paragraph[];
}

const fixture = new URL("../../fixtures/markdown.json", import.meta.url);
const cases: Case[] = JSON.parse(readFileSync(fixture, "utf8")).cases;

describe("textToParagraphs sur la fixture partagée avec Rust", () => {
  for (const c of cases) {
    test(`${c.name} redonne ses paragraphes`, () => {
      expect(textToParagraphs(c.markdown)).toEqual(c.paragraphs);
    });
  }
});

describe("markdownToRuns", () => {
  test("underscore autour d'un mot donne de l'italique", () => {
    expect(markdownToRuns("un _mot_ ici")).toEqual([{ t: "un " }, { t: "mot", italic: true }, { t: " ici" }]);
  });

  test("délimiteur sans fermeture reste du texte", () => {
    expect(markdownToRuns("a **b et [c](")).toEqual([{ t: "a **b et [c](" }]);
  });

  test("italique dans du gras cumule les deux", () => {
    expect(markdownToRuns("**a *b* c**")).toEqual([
      { t: "a ", bold: true },
      { t: "b", bold: true, italic: true },
      { t: " c", bold: true },
    ]);
  });

  test("ligne vide donne un run vide", () => {
    expect(markdownToRuns("")).toEqual([{ t: "" }]);
  });
});

describe("sameFormatting", () => {
  test("ignore couleur et taille, et fusionne les runs voisins", () => {
    const before = [
      { t: "a", bold: true, color: "accent" },
      { t: "b", bold: true },
    ];
    expect(sameFormatting(before, [{ t: "ab", bold: true }])).toBe(true);
  });

  test("un lien différent change la mise en forme", () => {
    expect(sameFormatting([{ t: "a", link: "https://x.y" }], [{ t: "a" }])).toBe(false);
  });
});

test("textToParagraphs garde tel quel un lien non encodé d'un paragraphe inchangé", () => {
  const raw: Paragraph = { runs: [{ t: "w", link: "https://w.org/A (b)", color: "accent" }] };
  expect(textToParagraphs("[w](https://w.org/A%20%28b%29)\nNouveau", [raw])[0]).toBe(raw);
});

test("textToParagraphs garde un paragraphe coloré dont le markdown n'a pas changé", () => {
  const colored: Paragraph = { runs: [{ t: "Gras", bold: true, color: "accent" }] };
  expect(textToParagraphs("**Gras**\nNouveau", [colored])[0]).toBe(colored);
});

describe("toggleEmphasis", () => {
  test("gras entoure la sélection et la garde", () => {
    expect(toggleEmphasis("a mot b", 2, 5, "bold")).toEqual({ value: "a **mot** b", start: 4, end: 7 });
  });

  test("gras retire le gras autour de la sélection", () => {
    expect(toggleEmphasis("a **mot** b", 4, 7, "bold")).toEqual({ value: "a mot b", start: 2, end: 5 });
  });

  test("gras retire le gras inclus dans la sélection", () => {
    expect(toggleEmphasis("a **mot** b", 2, 9, "bold")).toEqual({ value: "a mot b", start: 2, end: 5 });
  });

  test("italique sur du gras donne les deux", () => {
    expect(toggleEmphasis("**mot**", 2, 5, "italic")).toEqual({ value: "***mot***", start: 3, end: 6 });
  });

  test("gras retiré de gras-italique laisse l'italique", () => {
    expect(toggleEmphasis("***mot***", 3, 6, "bold")).toEqual({ value: "*mot*", start: 1, end: 4 });
  });

  test("sans sélection place le curseur entre les marques", () => {
    expect(toggleEmphasis("ab", 1, 1, "italic")).toEqual({ value: "a**b", start: 2, end: 2 });
  });
});

describe("insertLink", () => {
  test("entoure la sélection et sélectionne l'adresse à remplir", () => {
    expect(insertLink("voir site", 5, 9)).toEqual({ value: "voir [site](url)", start: 12, end: 15 });
  });

  test("une adresse sélectionnée devient la cible et le texte reste à écrire", () => {
    expect(insertLink("https://oku.dev", 0, 15)).toEqual({ value: "[](https://oku.dev)", start: 1, end: 1 });
  });
});
