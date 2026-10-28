// Pack gestures of a tab: opening a pack, finishing it, a widget click and the finish button.
import { describe, expect, test } from "bun:test";
import type { Snapshot } from "./api";
import { packGestures, showFinish, widgetGesture } from "./packGestures";
import type { PackMode, PackPin } from "./packTypes";

const editing = (o: Partial<PackMode> = {}) =>
  ({ pack: "e", name: "Essentiel", kind: "layouts", builtin: true, read_only: false, leased: true, display_name: "Essentiel", ...o }) as PackMode;
const snap = (s: Partial<Snapshot> = {}) =>
  ({ dirty: false, path: null, name: "essai", editing: null, history: null, widgets: {}, layouts: {}, ...s }) as unknown as Snapshot;

/** A context whose calls are noted in `calls`; `over` replaces any part. */
function context(over: Record<string, unknown> = {}) {
  const calls: string[] = [];
  const ctx = {
    onOpenPack: async (pack: string, widget: string) => (calls.push(`open ${pack} ${widget}`), true),
    onClose: () => calls.push("onClose"),
    ...over,
  };
  return { ctx, calls };
}

describe("packGestures, editPack", () => {
  test("editPack opens the pack through onOpenPack", async () => {
    const { ctx, calls } = context();
    await packGestures(ctx).editPack("p", "kpi");
    expect(calls).toEqual(["open p kpi"]);
  });

  test("editPack leaves the selection when a pack tab opened elsewhere", async () => {
    const { ctx, calls } = context({ onOpenPack: async () => false });
    await packGestures(ctx).editPack("p", "kpi");
    expect(calls).toEqual([]);
  });
});

describe("packGestures, finishPack", () => {
  test("finishPack of a pack tab calls onClose", async () => {
    const { ctx, calls } = context();
    await packGestures(ctx).finishPack();
    expect(calls).toEqual(["onClose"]);
  });
});

describe("widgetGesture", () => {
  const pin = (id: string): PackPin => ({ id, version: "1" });

  test("widgetGesture of an unknown kind is none", () => {
    expect(widgetGesture(snap({ widgets: { acme: pin("a") } }), "card")).toEqual({ kind: "none" });
  });

  test("widgetGesture in pack mode of the edited pack's widget opens it here", () => {
    const s = snap({ editing: editing({ pack: "a" }), widgets: { acme: pin("a") } });
    expect(widgetGesture(s, "acme/card")).toEqual({ kind: "local", name: "card" });
  });

  test("widgetGesture in pack mode of another pack's widget is none", () => {
    const s = snap({ editing: editing({ pack: "a" }), widgets: { acme: pin("a"), base: pin("b") } });
    expect(widgetGesture(s, "base/card")).toEqual({ kind: "none" });
  });

  test("widgetGesture in deck mode with a library pack opens it", () => {
    expect(widgetGesture(snap({ widgets: { acme: pin("a") } }), "acme/card")).toEqual({ kind: "open", pack: "a", name: "card" });
  });

  test("widgetGesture in deck mode with a deck-only pack opens it", () => {
    expect(widgetGesture(snap({ layouts: { "deck-layouts": pin("d") } }), "deck-layouts/cover")).toEqual({ kind: "open", pack: "d", name: "cover" });
  });

  test("widgetGesture in deck mode with no pack is none", () => {
    expect(widgetGesture(snap({ widgets: {} }), "acme/card")).toEqual({ kind: "none" });
  });
});

describe("showFinish", () => {
  test("showFinish of a pack tab with onClose is true", () => expect(showFinish(snap({ editing: editing() }), true)).toBe(true));
  test("showFinish of a pack tab without onClose is false", () => expect(showFinish(snap({ editing: editing() }), false)).toBe(false));
  test("showFinish without editing is false", () => expect(showFinish(snap(), true)).toBe(false));
});
