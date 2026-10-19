import { describe, expect, test } from "bun:test";
import registry from "../../fixtures/widgets/base-registry.json";
import { BASE, BASE_NAMESPACE, baseName } from "./baseKinds";

describe("baseName", () => {
  test("d'un kind de base rend son nom", () => expect(baseName("base/card")).toBe("card"));
  test("d'un autre espace de noms rend null", () => expect(baseName("acme/card")).toBeNull());
  test("d'un kind nu rend null", () => expect(baseName("text")).toBeNull());
  test("d'un nom inconnu de base rend null", () => expect(baseName("base/")).toBeNull());
  test.each(["base/constructor", "base/__proto__", "base/toString"])("de %s rend null", (k) => expect(baseName(k)).toBeNull());
});

test("BASE a les widgets du pack base du registre figé", () => {
  const pack = registry.packs.find((p) => p.namespace === BASE_NAMESPACE);
  const names = (pack?.widgets ?? []).map((w) => `${BASE_NAMESPACE}/${w.name}`).sort();
  expect(names).toEqual(Object.values(BASE).sort());
});
