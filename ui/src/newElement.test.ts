import { expect, test } from "bun:test";
import { newElement } from "./newElement";

test("newElement with a seed style copies the style", () => {
  const el = newElement("base/shape", "s-1", { params: {}, style: { fill: "surface", radius: "md" } });
  expect(el.style).toEqual({ fill: "surface", radius: "md" });
});

test("newElement without a seed style has no style key", () => {
  expect(Object.hasOwn(newElement("base/text", "t-1", { params: { body: [] } }), "style")).toBe(false);
});

test("newElement with a __proto__ param keeps it as data", () => {
  const params = JSON.parse('{"__proto__": {"polluted": true}}');
  const el = newElement("acme/x", "x-1", { params });
  expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  expect(Object.hasOwn(el.params ?? {}, "__proto__")).toBe(true);
});

test("newElement copies the seed params, leaving the seed untouched", () => {
  const seed = { params: { body: [{ runs: [{ t: "Nouveau texte" }] }] } };
  const el = newElement("base/text", "t-2", seed);
  (el.params!.body as { runs: { t: string }[] }[])[0].runs[0].t = "changé";
  expect(seed.params.body[0].runs[0].t).toBe("Nouveau texte");
});

test("newElement names the kind and the given id", () => {
  const el = newElement("acme/kpi", "w-1", { params: {} });
  expect([el.kind, el.id]).toEqual(["acme/kpi", "w-1"]);
});
