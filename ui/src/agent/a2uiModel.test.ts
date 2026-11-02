import { expect, spyOn, test } from "bun:test";
import type { Surface } from "../api";
import { dataModel, firstText } from "./a2uiModel";

const surface = (messages: unknown[]) => ({ id: "turn-1", messages }) as unknown as Surface;
const write = (path: string | undefined, value: unknown) => ({ version: "v0.9", updateDataModel: { surfaceId: "turn-1", path, value } });

test("dataModel nested write lands inside its root key", () => {
  const s = surface([write(undefined, { steps: [{ label: "a" }] }), write("/steps/1", { label: "b" }), write("/steps/0/label", "A")]);
  expect(dataModel(s)).toEqual({ steps: [{ label: "A" }, { label: "b" }] });
});

test("dataModel nested write leading nowhere is ignored", () => {
  const s = surface([write(undefined, { steps: [] }), write("/steps/4/label", "x")]);
  expect(dataModel(s)).toEqual({ steps: [] });
});

test("dataModel escaped root key is decoded", () => {
  const s = surface([write(undefined, {}), write("/a~1b", 1)]);
  expect(dataModel(s)).toEqual({ "a/b": 1 });
});

test("dataModel nested write under an inherited name is ignored", () => {
  const s = surface([write(undefined, {}), write("/toString/x", 1)]);
  expect(dataModel(s)).toEqual({});
});

test("dataModel consecutive nested writes on one root key clone it once", () => {
  const spy = spyOn(globalThis, "structuredClone");
  try {
    const s = surface([write(undefined, { steps: [{ label: "a" }] }), write("/steps/1", { label: "b" }), write("/steps/0/label", "A"), write("/steps/1/label", "B")]);
    expect(dataModel(s)).toEqual({ steps: [{ label: "A" }, { label: "B" }] });
    expect(spy).toHaveBeenCalledTimes(1);
  } finally {
    spy.mockRestore();
  }
});

const parts = (components: unknown[]) => ({ version: "v0.9", updateComponents: { surfaceId: "turn-1", components } });

test("firstText with a literal title gives it", () => {
  const s = surface([parts([{ id: "root", component: "Column", children: ["t"] }, { id: "t", component: "Text", text: " Packs " }])]);
  expect(firstText(s)).toBe("Packs");
});

test("firstText with a bound title reads the data model", () => {
  const s = surface([parts([{ id: "t", component: "Text", text: { path: "/title" } }]), write(undefined, { title: "Thèmes" })]);
  expect(firstText(s)).toBe("Thèmes");
});

test("firstText without a Text gives an empty string", () => {
  expect(firstText(surface([parts([{ id: "b", component: "Button" }])]))).toBe("");
});
