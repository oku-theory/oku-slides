// Reformulation d'un élément : le texte gardé est reparsé en paragraphes ici
// (l'interface est seule à lire le markdown), avant d'être envoyé à l'agent.

import { describe, expect, test } from "bun:test";
import { withRewriteParagraphs } from "./rewrite";
import type { Surface } from "../api";

const surface = (anchor?: { slide: string; element: string }): Surface => ({
  id: "ask-1",
  place: "sheet",
  live: false,
  anchor,
  messages: [],
});

describe("withRewriteParagraphs", () => {
  test("ajoute des paragraphes markdown pour un élément gardé", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", element: "e-texte" }), a);
    const paragraphs = out.context.paragraphs as { runs: { t: string; bold?: boolean }[] }[];
    expect(paragraphs[0].runs[0]).toEqual({ t: "a", bold: true });
    expect(paragraphs[0].runs[1].t).toBe(" b");
  });

  test("kept original : contexte inchangé", () => {
    const a = { name: "rewrite", context: { kept: "original", text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", element: "e-texte" }), a);
    expect(out).toEqual(a);
  });

  test("ancre un champ (@title) : contexte inchangé", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", element: "@title" }), a);
    expect(out).toEqual(a);
  });

  test("autre action : inchangée", () => {
    const a = { name: "choose", context: { choice: "a" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", element: "e-texte" }), a);
    expect(out).toEqual(a);
  });

  test("sans ancre : inchangée", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface(undefined), a);
    expect(out).toEqual(a);
  });
});
