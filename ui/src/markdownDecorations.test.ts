import { describe, expect, test } from "bun:test";
import { lineDecorations, linePreview, listOrdinal } from "./markdownDecorations";

describe("lineDecorations", () => {
  test("préfixe de puce et marques atténués, gras décoré", () => {
    expect(lineDecorations("- **a**", [])).toEqual([
      { from: 0, to: 2, className: "cm-md-mark" },
      { from: 2, to: 4, className: "cm-md-mark" },
      { from: 4, to: 5, className: "cm-md-bold" },
      { from: 5, to: 7, className: "cm-md-mark" },
    ]);
  });

  test("préfixe numéroté : plages décalées, adresse du lien à part", () => {
    expect(lineDecorations("12. [a](u)", [])).toEqual([
      { from: 0, to: 4, className: "cm-md-mark" },
      { from: 4, to: 5, className: "cm-md-mark" },
      { from: 5, to: 6, className: "cm-md-link" },
      { from: 6, to: 8, className: "cm-md-mark" },
      { from: 8, to: 9, className: "cm-md-url" },
      { from: 9, to: 10, className: "cm-md-mark" },
    ]);
  });

  test("pastille sur fond sombre : texte couleur du fond de page", () => {
    expect(lineDecorations("==x=={2}", ["#ffffff", "#2f5d8a"])[1]).toEqual({
      from: 2,
      to: 3,
      className: "cm-md-highlight",
      style: "background-color: #2f5d8a; color: var(--t-bg)",
    });
  });

  test("pastille au-delà de la palette : on reboucle", () => {
    expect(lineDecorations("==x=={3}", ["#f5e6a0", "#2f5d8a"])[1].style).toBe(
      "background-color: #f5e6a0; color: var(--t-ink)",
    );
  });

  test("pastille {0} : la dernière couleur, comme Typst", () => {
    expect(lineDecorations("==x=={0}", ["#f5e6a0", "#2f5d8a"])[1].style).toBe(
      "background-color: #2f5d8a; color: var(--t-bg)",
    );
  });

  test("seuil de ink-on (Typst) : gris 60 % foncé, #00a7cc et #9a9a9a clairs", () => {
    const ink = (c: string) => lineDecorations("==x==", [c])[1].style;
    expect(ink("#999999")).toBe("background-color: #999999; color: var(--t-bg)");
    expect(ink("#9a9a9a")).toBe("background-color: #9a9a9a; color: var(--t-ink)");
    expect(ink("#00a7cc")).toBe("background-color: #00a7cc; color: var(--t-ink)");
  });

  test("couleur courte claire : texte foncé", () => {
    expect(lineDecorations("==x==", ["#fff"])[1].style).toBe("background-color: #fff; color: var(--t-ink)");
  });

  test("palette vide ou valeur qui n'est pas hexadécimale : pastille sans style", () => {
    expect(lineDecorations("==x==", [])[1]).toEqual({ from: 2, to: 3, className: "cm-md-highlight" });
    expect(lineDecorations("==x==", ["red;background:url(//x)"])[1]).toEqual({
      from: 2,
      to: 3,
      className: "cm-md-highlight",
    });
  });

  test("marqueurs de liste échappés : pas de préfixe, la barre oblique est une marque", () => {
    expect(lineDecorations("1\\. pas", [])).toEqual([{ from: 1, to: 2, className: "cm-md-mark" }]);
    expect(lineDecorations("\\- pas", [])).toEqual([{ from: 0, to: 1, className: "cm-md-mark" }]);
  });

  test("numéro sans espace : pas de préfixe", () => {
    expect(lineDecorations("1.x", [])).toEqual([]);
  });
});

/** Plages cachées d'une ligne pour une sélection (positions dans la ligne). */
const hidden = (line: string, selection: { from: number; to: number }[], ordinal = 1) =>
  linePreview(line, [], selection, ordinal)
    .filter((d) => "hide" in d)
    .map((d) => [d.from, d.to]);
const at = (pos: number) => [{ from: pos, to: pos }];

describe("linePreview", () => {
  test("curseur hors du gras : ses marques sont cachées, le gras reste mis en forme", () => {
    const preview = linePreview("a **b** c", [], at(0), 1);
    expect(preview).toContainEqual({ from: 4, to: 5, className: "cm-md-bold" });
    expect(hidden("a **b** c", at(0))).toEqual([[2, 4], [5, 7]]);
  });

  test("curseur dans le gras : ses marques se montrent, atténuées", () => {
    expect(hidden("a **b** c", at(4))).toEqual([]);
    expect(linePreview("a **b** c", [], at(4), 1)).toContainEqual({ from: 2, to: 4, className: "cm-md-mark" });
  });

  test("curseur contre la marque fermante : la construction est touchée, ses marques se montrent", () => {
    expect(hidden("a **b** c", at(7))).toEqual([]);
    expect(hidden("a **b** c", at(2))).toEqual([]);
  });

  test("sélection qui couvre la construction : ses marques se montrent", () => {
    expect(hidden("a **b** c", [{ from: 0, to: 9 }])).toEqual([]);
  });

  test("ligne sans curseur : toutes les marques sont cachées", () => {
    expect(hidden("*a* et _b_", [])).toEqual([[0, 1], [2, 3], [7, 8], [9, 10]]);
  });

  test("italique dans le gras, curseur dans le gras seul : seules les marques de l'italique sont cachées", () => {
    expect(hidden("**a *b* c**", at(3))).toEqual([[4, 5], [6, 7]]);
  });

  test("lien hors du curseur : crochets, adresse et parenthèses cachés d'un seul tenant", () => {
    expect(hidden("[a](u) x", at(8))).toEqual([[0, 1], [2, 6]]);
    expect(linePreview("[a](u) x", [], at(8), 1)).toContainEqual({ from: 1, to: 2, className: "cm-md-link" });
  });

  test("pastille colorée hors du curseur : == et =={n} cachés", () => {
    expect(hidden("==x=={2} y", at(10))).toEqual([[0, 2], [3, 8]]);
  });

  test("échappement hors du curseur : la barre oblique est cachée, la ponctuation reste", () => {
    expect(hidden("\\* y", at(4))).toEqual([[0, 1]]);
    expect(hidden("\\* y", at(1))).toEqual([]);
  });

  test("puce hors du curseur : le tiret devient une puce", () => {
    expect(linePreview("- a", [], at(3), 1)).toContainEqual({ from: 0, to: 2, marker: "• " });
  });

  test("puce sous le curseur : le tiret se montre, atténué", () => {
    expect(linePreview("- a", [], at(0), 1)).toContainEqual({ from: 0, to: 2, className: "cm-md-mark" });
  });

  test("numéro hors du curseur : le rang de l'item dans sa liste, comme au rendu", () => {
    expect(linePreview("1. a", [], at(4), 3)).toContainEqual({ from: 0, to: 3, marker: "3. " });
  });
});

describe("listOrdinal", () => {
  const lines = ["1. a", "1. b", "x", "7. c", "- d", "2. e", "3. f"];
  const line = (n: number) => lines[n - 1];

  test("items numérotés consécutifs : 1, 2", () => {
    expect([listOrdinal(line, 1), listOrdinal(line, 2)]).toEqual([1, 2]);
  });

  test("un paragraphe ou une puce clôt la liste : la suivante repart à 1, quel que soit le numéro écrit", () => {
    expect([listOrdinal(line, 4), listOrdinal(line, 6), listOrdinal(line, 7)]).toEqual([1, 1, 2]);
  });
});

describe("headings", () => {
  test("heading line: level class on the whole line, prefix dimmed", () => {
    expect(lineDecorations("## Plan", [])).toEqual([
      { from: 0, to: 7, className: "cm-md-h2" },
      { from: 0, to: 3, className: "cm-md-mark" },
    ]);
  });

  test("heading prefix stays visible outside the selection", () => {
    expect(linePreview("# Plan", [], [], 0)).toEqual([
      { from: 0, to: 6, className: "cm-md-h1" },
      { from: 0, to: 2, className: "cm-md-mark" },
    ]);
  });

  test("a heading ends the numbered list", () => {
    const lines = ["1. a", "# T", "1. b"];
    expect(listOrdinal((k) => lines[k - 1], 3)).toBe(1);
  });
});
