import { describe, expect, test } from "bun:test";
import { insertDictated, insertLink, minimalChange, toggleEmphasis } from "./textEdits";

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

describe("insertDictated", () => {
  test("après une espace et avant un mot : une espace après seulement", () => {
    expect(insertDictated("Bonjour monde", 8, 8, "cher")).toEqual({ value: "Bonjour cher monde", start: 12, end: 12 });
  });

  test("collé à la fin d'un mot : une espace avant", () => {
    expect(insertDictated("Fin", 3, 3, "suite")).toEqual({ value: "Fin suite", start: 9, end: 9 });
  });

  test("après une parenthèse et avant un point : aucune espace", () => {
    expect(insertDictated("(.", 1, 1, "note")).toEqual({ value: "(note.", start: 5, end: 5 });
  });

  test("avant un guillemet fermant : pas d'espace ajoutée (comportement actuel conservé)", () => {
    expect(insertDictated("« »", 2, 2, "mot")).toEqual({ value: "« mot»", start: 5, end: 5 });
  });

  test("document vide : le texte seul", () => {
    expect(insertDictated("", 0, 0, "Bonjour")).toEqual({ value: "Bonjour", start: 7, end: 7 });
  });

  test("sélection non vide : remplacée, curseur après le texte", () => {
    expect(insertDictated("un deux trois", 3, 7, "2")).toEqual({ value: "un 2 trois", start: 4, end: 4 });
  });
});

describe("minimalChange", () => {
  test("mot mis en gras : seul le milieu change", () => {
    expect(minimalChange("a mot b", "a **mot** b")).toEqual({ from: 2, to: 5, insert: "**mot**" });
  });

  test("gras retiré : seul le milieu change", () => {
    expect(minimalChange("a **mot** b", "a mot b")).toEqual({ from: 2, to: 9, insert: "mot" });
  });

  test("textes égaux : remplacement vide en fin de texte", () => {
    expect(minimalChange("abc", "abc")).toEqual({ from: 3, to: 3, insert: "" });
  });

  test("caractère répété ajouté ou retiré : à la fin du préfixe commun", () => {
    expect(minimalChange("aa", "aaa")).toEqual({ from: 2, to: 2, insert: "a" });
    expect(minimalChange("aaa", "aa")).toEqual({ from: 2, to: 3, insert: "" });
  });

  test("tout différent : tout remplacé", () => {
    expect(minimalChange("ab", "cd")).toEqual({ from: 0, to: 2, insert: "cd" });
  });
});
