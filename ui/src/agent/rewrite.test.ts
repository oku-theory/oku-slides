// Reformulation : le texte gardé est reparsé en paragraphes ici (l'interface est seule à lire
// le markdown), avant d'être envoyé à l'agent ; la session prend les paragraphes pour un champ
// `rich`, le texte pour un champ `plain` (t-326 : l'interface n'a plus à deviner lequel).

import { describe, expect, test } from "bun:test";
import { withRewriteParagraphs } from "./rewrite";
import type { Anchor, Surface } from "../api";

const surface = (anchor?: Anchor): Surface => ({
  id: "ask-1",
  place: "sheet",
  live: false,
  question: true,
  anchor,
  messages: [],
});

describe("withRewriteParagraphs", () => {
  test("ajoute des paragraphes markdown pour un élément gardé", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", owner: "e-texte" }), a);
    const paragraphs = out.context.paragraphs as { runs: { t: string; bold?: boolean }[] }[];
    expect(paragraphs[0].runs[0]).toEqual({ t: "a", bold: true });
    expect(paragraphs[0].runs[1].t).toBe(" b");
  });

  test("kept original : contexte inchangé", () => {
    const a = { name: "rewrite", context: { kept: "original", text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", owner: "e-texte" }), a);
    expect(out).toEqual(a);
  });

  test("ancre un champ de la slide : paragraphes ajoutés aussi, la session choisit", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", owner: "s-libre", field: "title" }), a);
    expect(out.context.text).toBe("**a** b");
    expect((out.context.paragraphs as unknown[]).length).toBe(1);
  });

  test("autre action : inchangée", () => {
    const a = { name: "choose", context: { choice: "a" } };
    const out = withRewriteParagraphs(surface({ slide: "s-libre", owner: "e-texte" }), a);
    expect(out).toEqual(a);
  });

  test("sans ancre : inchangée", () => {
    const a = { name: "rewrite", context: { kept: 0, text: "**a** b" } };
    const out = withRewriteParagraphs(surface(undefined), a);
    expect(out).toEqual(a);
  });
});
