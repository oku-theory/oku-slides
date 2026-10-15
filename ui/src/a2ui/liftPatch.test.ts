import { afterEach, describe, expect, spyOn, test } from "bun:test";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import { groupByRoot, liftGroup, liftPatch } from "./liftPatch";

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

describe("liftGroup", () => {
  const model = () => ({ steps: [{ label: "a", calls: [{ status: "in_progress" }] }, { label: "b" }], narration: "n", other: {} }) as Record<string, unknown>;
  const reader = (m: Record<string, unknown>) => (pointer: string) => m[pointer.slice(1)];
  /** Applique les messages un à un avec `liftPatch`, comme avant le regroupement. */
  const oneByOne = (messages: A2uiMessage[]) => {
    const m = model();
    for (const msg of messages) {
      const u = body(liftPatch(msg, reader(m)));
      if (u.path && u.path.split("/").length === 2) m[u.path.slice(1)] = u.value;
    }
    return m;
  };
  /** Applique les messages par groupes de même clé racine. */
  const grouped = (messages: A2uiMessage[]) => {
    const m = model();
    const out: A2uiMessage[] = [];
    for (const group of groupByRoot(messages)) {
      const lifted = liftGroup(group, reader(m));
      if (!lifted) continue;
      out.push(lifted);
      const u = body(lifted);
      if (u.path && u.path.split("/").length === 2) m[u.path.slice(1)] = u.value;
    }
    return { model: m, out };
  };
  const batch = [
    write("/steps/0/calls/0/status", "completed"),
    write("/steps/1/label", "B"),
    write("/steps/2", { label: "c" }),
    write("/steps/9/label", "nowhere"),
    write("/steps/2/label", "C"),
  ];
  let spy: ReturnType<typeof spyOn> | undefined;
  afterEach(() => spy?.mockRestore());

  test("liftGroup nested writes on one root key clone once", () => {
    spy = spyOn(globalThis, "structuredClone");
    const { out } = grouped(batch);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(out).toHaveLength(1);
    expect(body(out[0]).path).toBe("/steps");
  });

  test("liftGroup batch equals applying writes one by one", () => {
    expect(grouped(batch).model).toEqual(oneByOne(batch));
  });

  test("liftGroup mixed root keys keep their order", () => {
    const mixed = [
      write("/steps/0/label", "x"),
      write("/narration", "m"),
      write("/steps/0/label", "y"),
      write("/steps/1/label", "z"),
      write("/other/k", 1),
    ];
    const { model: m, out } = grouped(mixed);
    expect(out.map((o) => body(o).path)).toEqual(["/steps", "/narration", "/steps", "/other"]);
    expect(m).toEqual(oneByOne(mixed));
  });

  test("liftGroup group leading nowhere gives nothing", () => {
    const [group] = groupByRoot([write("/steps/9/label", "x"), write("/steps/0/__proto__", 1)]);
    expect(liftGroup(group, reader(model()))).toBeNull();
  });

  test("groupByRoot keeps a nested deletion and other messages alone", () => {
    const del = { version: "v0.9", updateDataModel: { surfaceId: "s", path: "/steps/0" } } as unknown as A2uiMessage;
    const groups = groupByRoot([write("/steps/0/label", "x"), del, write("/steps/1/label", "y")]);
    expect(groups.map((g) => g.length)).toEqual([1, 1, 1]);
    expect(liftGroup(groups[1], reader(model()))).toBe(del);
  });
});
