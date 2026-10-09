import { expect, test } from "bun:test";
import { randomId } from "./randomId";

test("randomId commence par son préfixe", () => {
  expect(randomId("v").startsWith("v-")).toBe(true);
});

test("randomId deux appels donnent deux identifiants", () => {
  expect(randomId("v")).not.toBe(randomId("v"));
});
