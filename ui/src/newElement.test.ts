import { describe, expect, test } from "bun:test";
import { newElement } from "./newElement";

describe("newElement", () => {
  test("text : un paragraphe de départ dans params.body", () => {
    const e = newElement("text", "e-1");
    expect(e.kind).toBe("text");
    expect(e.params?.body).toEqual([{ runs: [{ t: expect.any(String) }] }]);
  });

  test("card : titre et corps de départ dans params", () => {
    const e = newElement("card", "e-2");
    expect(e.kind).toBe("card");
    const params = e.params as { title: string; body: unknown };
    expect(typeof params.title).toBe("string");
    expect(params.body).toEqual([{ runs: [{ t: expect.any(String) }] }]);
  });

  test("shape : un style, sans params (la forme vaut rect par défaut)", () => {
    const e = newElement("shape", "e-3");
    expect(e.kind).toBe("shape");
    expect(e.style).toEqual({ fill: "surface", radius: "md" });
    expect(e.params).toBeUndefined();
  });

  test("chart : une option de départ dans params.option", () => {
    const e = newElement("chart", "e-4");
    expect(e.kind).toBe("chart");
    const params = e.params as { option: { series: unknown[] } };
    expect(params.option.series).toEqual([{ type: "bar", data: [3, 5, 2] }]);
  });

  test("chaque élément garde l'id donné", () => {
    expect(newElement("text", "abc").id).toBe("abc");
  });
});
