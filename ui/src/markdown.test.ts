import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import type { Paragraph } from "./api";
import { listPrefix, markdownSpans, markdownToRuns, sameFormatting } from "./markdown";
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

  test("pastille sans fermeture ou triple égal reste du texte", () => {
    expect(markdownToRuns("a ==b et c === d")).toEqual([{ t: "a ==b et c === d" }]);
  });

  test("ligne vide donne un run vide", () => {
    expect(markdownToRuns("")).toEqual([{ t: "" }]);
  });

  test("couleur de pastille de 0 à 255 (un u8) ; au-delà ou avec zéro en tête, texte littéral", () => {
    expect(markdownToRuns("==x=={255}")).toEqual([{ t: "x", highlight: 255 }]);
    expect(markdownToRuns("==x=={0}")).toEqual([{ t: "x", highlight: 0 }]);
    expect(markdownToRuns("==x=={256}")).toEqual([{ t: "x", highlight: 1 }, { t: "{256}" }]);
    expect(markdownToRuns("==x=={007}")).toEqual([{ t: "x", highlight: 1 }, { t: "{007}" }]);
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

describe("markdownSpans", () => {
  test("gras : marques autour, gras dedans", () => {
    expect(markdownSpans("a **b** c")).toEqual([
      { from: 2, to: 4, kind: "mark" },
      { from: 4, to: 5, kind: "bold" },
      { from: 5, to: 7, kind: "mark" },
    ]);
  });

  test("triple étoile : gras puis italique sur la même plage", () => {
    expect(markdownSpans("***x***")).toEqual([
      { from: 0, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "bold" },
      { from: 3, to: 4, kind: "italic" },
      { from: 4, to: 7, kind: "mark" },
    ]);
  });

  test("italique dans du gras : l'englobante avant l'englobée", () => {
    expect(markdownSpans("**a *b* c**")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 9, kind: "bold" },
      { from: 4, to: 5, kind: "mark" },
      { from: 5, to: 6, kind: "italic" },
      { from: 6, to: 7, kind: "mark" },
      { from: 9, to: 11, kind: "mark" },
    ]);
  });

  test("underscore autour d'un mot : italique", () => {
    expect(markdownSpans("_mot_")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 4, kind: "italic" },
      { from: 4, to: 5, kind: "mark" },
    ]);
  });

  test("lien : texte, adresse et marques", () => {
    expect(markdownSpans("[le site](https://a.b)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 8, kind: "link" },
      { from: 8, to: 10, kind: "mark" },
      { from: 10, to: 21, kind: "url" },
      { from: 21, to: 22, kind: "mark" },
    ]);
  });

  test("lien en gras : le gras est dans le lien", () => {
    expect(markdownSpans("[**a**](u)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 6, kind: "link" },
      { from: 1, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "bold" },
      { from: 4, to: 6, kind: "mark" },
      { from: 6, to: 8, kind: "mark" },
      { from: 8, to: 9, kind: "url" },
      { from: 9, to: 10, kind: "mark" },
    ]);
  });

  test("lien au texte vide : pas de plage vide", () => {
    expect(markdownSpans("[](u)")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "url" },
      { from: 4, to: 5, kind: "mark" },
    ]);
  });

  test("pastille avec couleur : la couleur fait partie de la marque fermante", () => {
    expect(markdownSpans("==上手=={2}")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 4, kind: "highlight", highlight: 2 },
      { from: 4, to: 9, kind: "mark" },
    ]);
  });

  test("couleur de pastille hors 0..255 : pas de marque sur le {n}", () => {
    expect(markdownSpans("==x=={255}")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 3, kind: "highlight", highlight: 255 },
      { from: 3, to: 10, kind: "mark" },
    ]);
    for (const n of ["{256}", "{007}"]) {
      expect(markdownSpans(`==x==${n}`)).toEqual([
        { from: 0, to: 2, kind: "mark" },
        { from: 2, to: 3, kind: "highlight", highlight: 1 },
        { from: 3, to: 5, kind: "mark" },
      ]);
    }
  });

  test("pastille sans couleur : la première", () => {
    expect(markdownSpans("==x==")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 3, kind: "highlight", highlight: 1 },
      { from: 3, to: 5, kind: "mark" },
    ]);
  });

  test("emoji avant du gras : décalages en unités UTF-16", () => {
    expect(markdownSpans("😀 **a**")).toEqual([
      { from: 3, to: 5, kind: "mark" },
      { from: 5, to: 6, kind: "bold" },
      { from: 6, to: 8, kind: "mark" },
    ]);
  });

  test("échappement dans du gras : la barre oblique est une marque", () => {
    expect(markdownSpans("**\\*x**")).toEqual([
      { from: 0, to: 2, kind: "mark" },
      { from: 2, to: 5, kind: "bold" },
      { from: 2, to: 3, kind: "mark" },
      { from: 5, to: 7, kind: "mark" },
    ]);
  });

  test("délimiteurs sans fermeture, quadruple étoile, snake_case : aucune plage", () => {
    expect(markdownSpans("a **b et [c](")).toEqual([]);
    expect(markdownSpans("****")).toEqual([]);
    expect(markdownSpans("snake_case_mot")).toEqual([]);
    expect(markdownSpans("")).toEqual([]);
  });
});

describe("listPrefix", () => {
  test("puce : deux caractères", () => {
    expect(listPrefix("- a")).toEqual({ list: "bullet", length: 2 });
  });

  test("numéro à deux chiffres : quatre caractères", () => {
    expect(listPrefix("12. a")).toEqual({ list: "number", length: 4 });
  });

  test("sans espace, tiret collé ou marqueur échappé : pas de liste", () => {
    for (const line of ["1.x", "-x", "\\- x", "1\\. x", ""]) {
      expect(listPrefix(line)).toEqual({ length: 0 });
    }
  });
});
