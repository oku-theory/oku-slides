import { expect, test } from "bun:test";
import { fileName, isPptx } from "./importFile";

test.each([
  ["/Users/moi/Deck.pptx", true],
  ["C:\\Decks\\Rapport.PPTX", true],
  ["/tmp/alto.pptx  ", false],
  ["/tmp/deck.oku-slides.pdf", false],
  ["/tmp/deck.json", false],
  ["/tmp/ancien.ppt", false],
  ["/tmp/pptx", false],
  ["/tmp/deck.pptx.pdf", false],
])("isPptx(%p) vaut %p", (path, expected) => {
  expect(isPptx(path)).toBe(expected);
});

test("fileName garde le nom du fichier, quel que soit le séparateur", () => {
  expect(fileName("/Users/moi/Alto Wealth.pptx")).toBe("Alto Wealth.pptx");
  expect(fileName("C:\\Decks\\Rapport.pptx")).toBe("Rapport.pptx");
  expect(fileName("seul.pptx")).toBe("seul.pptx");
});
