import { expect, test } from "bun:test";
import { typedPath } from "./filePick";

test.each([
  ["  /tmp/alto.pptx \n", "/tmp/alto.pptx"],
  ["/Users/moi/Mon deck.oku-slides.pdf", "/Users/moi/Mon deck.oku-slides.pdf"],
  ["   ", null],
  ["", null],
  [null, null],
])("typedPath(%p) vaut %p", (typed, expected) => {
  expect(typedPath(typed)).toBe(expected);
});
