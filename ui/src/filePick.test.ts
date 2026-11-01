import { expect, test } from "bun:test";
import { saveName, typedPath } from "./filePick";

test.each([
  ["  /tmp/alto.pptx \n", "/tmp/alto.pptx"],
  ["/Users/moi/Mon deck.oku-slides.pdf", "/Users/moi/Mon deck.oku-slides.pdf"],
  ["   ", null],
  ["", null],
  [null, null],
])("typedPath(%p) vaut %p", (typed, expected) => {
  expect(typedPath(typed)).toBe(expected);
});

test.each([
  ["Roadmap", "Roadmap"],
  ["fdp.\u202Eexe", "fdp.exe"],
  ["a\nb", "a b"],
  ["a\n", "a"],
  ["../etc/x\\y", "..-etc-x-y"],
  ["\u202E\u200B", "ui-brand"],
  ["é".repeat(300), "é".repeat(200)],
])("saveName(%p) proposes %p (t-1383)", (name, expected) => {
  expect(saveName(name)).toBe(expected);
});
