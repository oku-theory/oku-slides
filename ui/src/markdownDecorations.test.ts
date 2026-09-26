import { describe, expect, test } from "bun:test";
import { lineDecorations } from "./markdownDecorations";

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
