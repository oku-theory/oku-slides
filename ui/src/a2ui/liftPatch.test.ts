import { describe, expect, test } from "bun:test";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import { liftPatch } from "./liftPatch";

const write = (path: string | undefined, value: unknown) =>
  ({ version: "v0.9", updateDataModel: { surfaceId: "s", ...(path === undefined ? {} : { path }), value } }) as unknown as A2uiMessage;
const body = (m: A2uiMessage) => (m as unknown as { updateDataModel: { path?: string; value: unknown } }).updateDataModel;

describe("liftPatch", () => {
  test("liftPatch nested write becomes a fresh root-key write", () => {
    const steps = [{ label: "a", calls: [{ status: "in_progress" }] }];
    const lifted = liftPatch(write("/steps/0/calls/0/status", "failed"), (pointer) => (pointer === "/steps" ? steps : undefined));
    expect(body(lifted).path).toBe("/steps");
    expect(body(lifted).value).toEqual([{ label: "a", calls: [{ status: "failed" }] }]);
  });

  test("liftPatch leaves the current model untouched", () => {
    const steps = [{ label: "a" }];
    liftPatch(write("/steps/0/label", "b"), () => steps);
    expect(steps).toEqual([{ label: "a" }]);
  });

  test("liftPatch at the list length appends", () => {
    const lifted = liftPatch(write("/steps/1", { label: "b" }), () => [{ label: "a" }]);
    expect(body(lifted).value).toEqual([{ label: "a" }, { label: "b" }]);
  });

  test("liftPatch root-key or root write is kept as is", () => {
    const root = write("/narration", "x");
    expect(liftPatch(root, () => "y")).toBe(root);
    const whole = write(undefined, { a: 1 });
    expect(liftPatch(whole, () => undefined)).toBe(whole);
  });

  test("liftPatch path not leading anywhere is kept as is", () => {
    const m = write("/steps/4/label", "x");
    expect(liftPatch(m, () => [])).toBe(m);
  });

  test("liftPatch decodes escaped keys and reads the root key by its pointer", () => {
    const read: string[] = [];
    const lifted = liftPatch(write("/a~1b/c~0d", 1), (pointer) => (read.push(pointer), {}));
    expect(read).toEqual(["/a~1b"]);
    expect(body(lifted).path).toBe("/a~1b");
    expect(body(lifted).value).toEqual({ "c~d": 1 });
  });

  test("liftPatch refuses prototype segments", () => {
    // Des clés propres à ces noms : sans le refus, l'écriture les atteindrait.
    const model = JSON.parse('{"__proto__": {"a": 1}, "constructor": {"prototype": {"a": 1}}}');
    for (const path of ["/m/__proto__/x", "/m/constructor/prototype/x", "/m/__proto__"]) {
      const m = write(path, 1);
      expect(liftPatch(m, () => model)).toBe(m);
    }
  });

  test("liftPatch refuses indexes JSON Pointer does not allow", () => {
    for (const path of ["/steps/", "/steps/01", "/steps/+1", "/steps/1e0"]) {
      const m = write(path, { label: "x" });
      expect(liftPatch(m, () => [{ label: "a" }, { label: "b" }])).toBe(m);
    }
  });
});
