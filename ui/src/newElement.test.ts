import { describe, expect, test } from "bun:test";
import { newElement, newPackWidget } from "./newElement";

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

  test("table : un tableau de départ dans params.data", () => {
    const e = newElement("table", "t1");
    expect([e.kind, (e.params?.data as { dataset: { source: unknown[] } }).dataset.source.length]).toEqual(["table", 3]);
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

describe("newPackWidget", () => {
  test("newPackWidget nomme le widget en kind, sans params : ses défauts valent", () => {
    expect(newPackWidget("kpi", "e-5")).toEqual({ id: "e-5", kind: "kpi" });
  });
});
