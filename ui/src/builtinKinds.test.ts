import { describe, expect, test } from "bun:test";
import registry from "../../fixtures/widgets/builtin-registry.json";
import { BASE, BASE_NAMESPACE, ESSENTIEL, ESSENTIEL_NAMESPACE, baseName } from "./builtinKinds";

describe("baseName", () => {
  test("d'un kind de base rend son nom", () => expect(baseName("base/card")).toBe("card"));
  test("d'un autre espace de noms rend null", () => expect(baseName("acme/card")).toBeNull());
  test("d'un kind nu rend null", () => expect(baseName("text")).toBeNull());
  test("d'un nom inconnu de base rend null", () => expect(baseName("base/")).toBeNull());
  test.each(["base/constructor", "base/__proto__", "base/toString"])("de %s rend null", (k) => expect(baseName(k)).toBeNull());
});

/** Les kinds qualifiés des widgets du pack `namespace` du registre figé, triés. */
const frozen = (namespace: string) =>
  (registry.packs.find((p) => p.namespace === namespace)?.widgets ?? []).map((w) => `${namespace}/${w.name}`).sort();

test("BASE a les widgets du pack base du registre figé", () => {
  expect(frozen(BASE_NAMESPACE)).toEqual(Object.values(BASE).sort());
});

test("ESSENTIEL kinds are qualified by essentiel", () => {
  expect(Object.values(ESSENTIEL).every((k) => k.startsWith(`${ESSENTIEL_NAMESPACE}/`))).toBe(true);
});

test("ESSENTIEL a les layouts du pack essentiel du registre figé", () => {
  expect(frozen(ESSENTIEL_NAMESPACE)).toEqual(Object.values(ESSENTIEL).sort());
});
