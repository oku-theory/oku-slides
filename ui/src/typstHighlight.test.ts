import { describe, expect, test } from "bun:test";
import { typstTokens } from "./typstHighlight";

describe("typstTokens", () => {
  test("dièse, mot-clé, chaîne et descripteur p.*", () => {
    expect(typstTokens('#let w = widget("kpi", params: (v: p.plain()))')).toEqual([
      { from: 0, to: 1, className: "cm-typ-hash" },
      { from: 1, to: 4, className: "cm-typ-keyword" },
      { from: 16, to: 21, className: "cm-typ-string" },
      { from: 35, to: 42, className: "cm-typ-param" },
    ]);
  });

  test("commentaire de ligne jusqu'au bout, chaîne échappée", () => {
    expect(typstTokens('"a\\"b" // fin')).toEqual([
      { from: 0, to: 6, className: "cm-typ-string" },
      { from: 7, to: 13, className: "cm-typ-comment" },
    ]);
  });

  test("p.* collé à un identifiant n'est pas un descripteur", () => {
    expect(typstTokens("top.plain")).toEqual([]);
  });
});
