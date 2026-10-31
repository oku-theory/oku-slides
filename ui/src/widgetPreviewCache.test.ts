import { describe, expect, test } from "bun:test";
import { createPreviewCache } from "./widgetPreviewCache";

describe("createPreviewCache", () => {
  test("get after set returns the stored svg", () => {
    const c = createPreviewCache();
    c.set("k", "<svg/>");
    expect(c.get("k")).toBe("<svg/>");
  });

  test("get of an unknown key returns undefined", () => {
    expect(createPreviewCache().get("k")).toBeUndefined();
  });

  test("past its bound, the least recently used entry is dropped", () => {
    const c = createPreviewCache(2);
    c.set("a", "A");
    c.set("b", "B");
    c.get("a");
    c.set("c", "C");
    expect([c.get("a"), c.get("b"), c.get("c")]).toEqual(["A", undefined, "C"]);
  });
});
