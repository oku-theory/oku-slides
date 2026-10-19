// Règle de l'aperçu d'une variante sur la scène (t-1070).

import { expect, test } from "bun:test";
import { nextPreview } from "./slidePreview";

test("nextPreview posing an image replaces the current one", () => {
  expect(nextPreview("a", "b", true)).toBe("b");
});

test("nextPreview removing the shown image restores the slide", () => {
  expect(nextPreview("a", "a", false)).toBeNull();
});

test("nextPreview removing another image keeps the shown one", () => {
  expect(nextPreview("b", "a", false)).toBe("b");
});
