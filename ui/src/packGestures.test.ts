// Pack gestures of a tab: opening a pack, finishing it, a widget click and the finish button.
import { afterAll, beforeAll, describe, expect, mock, test } from "bun:test";
import type { Snapshot, TabApi } from "./api";
import { setMessages } from "./i18n";
import { packGestures, showFinish, widgetGesture } from "./packGestures";
import type { PackMode, PackPin } from "./packTypes";
import { resetMessages } from "./test/messages";

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
beforeAll(async () => {
  setMessages("en", [await ftl("en")]);
});
afterAll(resetMessages);

const editing = (o: Partial<PackMode> = {}) =>
  ({ pack: "e", name: "Essentiel", kind: "layouts", builtin: true, host: "deck", read_only: false, leased: true, display_name: "Essentiel", ...o }) as PackMode;
const snap = (s: Partial<Snapshot> = {}) =>
  ({ dirty: false, path: null, name: "essai", editing: null, history: null, widgets: {}, layouts: {}, ...s }) as unknown as Snapshot;

/** A context whose calls are noted in `calls`; `over` replaces any part. */
function context(over: Record<string, unknown> = {}) {
  const calls: string[] = [];
  const snapRef = { current: (over.snap as Snapshot | null | undefined) ?? snap() };
  // The workshop stays open until closePack: the reread snapshot follows it.
  let open = true;
  const api = {
    snapshot: async () => (calls.push("snapshot"), snap({ editing: open ? editing() : null })),
    savePack: async () => (calls.push("savePack"), snap({ editing: editing() })),
    closePack: async () => (calls.push("closePack"), (open = false), snap()),
    save: async () => (calls.push("save"), snap()),
    ...((over.api as object | undefined) ?? {}),
  } as unknown as TabApi;
  const ctx = {
    api,
    snap: snapRef.current,
    snapRef,
    run: async (p: Promise<Snapshot | null>) => p.catch(() => null),
    autosave: { flush: async () => void calls.push("flush"), schedule: () => {}, cancel: () => {} },
    widgets: { finish: async (close: () => Promise<unknown>) => (calls.push("settle"), close()) },
    setCurrent: (id: string | null) => calls.push(`current ${id}`),
    setSelected: (id: string | null) => calls.push(`selected ${id}`),
    onOpenPack: async (pack: string, widget: string) => (calls.push(`open ${pack} ${widget}`), true),
    onClose: () => calls.push("onClose"),
    confirm: mock(() => true),
    ...over,
  };
  return { ctx, calls, snapRef };
}
type Ctx = Parameters<typeof packGestures>[0];

describe("packGestures, editPack", () => {
  test("editPack opens the pack through onOpenPack", async () => {
    const { ctx, calls } = context();
    await packGestures(ctx as unknown as Ctx).editPack("p", "kpi");
    expect(calls[0]).toBe("open p kpi");
  });

  test("editPack resets the selection when the deck-only workshop opens here", async () => {
    const { ctx, calls } = context();
    await packGestures(ctx as unknown as Ctx).editPack("e", "title");
    expect(calls).toEqual(["open e title", "snapshot", "current null", "selected null"]);
  });

  test("editPack leaves the selection when a pack tab opened elsewhere", async () => {
    const { ctx, calls } = context({ onOpenPack: async () => false });
    await packGestures(ctx as unknown as Ctx).editPack("p", "kpi");
    expect(calls).toEqual([]);
  });

  test("editPack keeps the selection when the snapshot fails", async () => {
    const { ctx, calls } = context({ api: { snapshot: async () => Promise.reject(new Error("rendu")) } });
    await packGestures(ctx as unknown as Ctx).editPack("e", "title");
    expect(calls).toEqual(["open e title"]);
  });

  test("onOpenPack resolving false on a null view leaves the selection", async () => {
    // As Workspace answers: a null view (a refused open) is false.
    const view: { windows: unknown[] } | null = null;
    const { ctx, calls } = context({ onOpenPack: async () => view !== null });
    await packGestures(ctx as unknown as Ctx).editPack("e", "title");
    expect(calls.filter((c) => c.startsWith("current") || c.startsWith("selected") || c === "snapshot")).toEqual([]);
  });
});

describe("packGestures, finishPack", () => {
  test("finishPack in a pack tab calls onClose only", async () => {
    const { ctx, calls } = context({ snap: snap({ dirty: true, editing: editing({ host: "tab" }) }) });
    await packGestures(ctx as unknown as Ctx).finishPack();
    expect(calls).toEqual(["onClose"]);
  });

  test("finishPack in the deck workshop closes the workshop", async () => {
    const { ctx, calls } = context({ snap: snap({ dirty: true, editing: editing() }) });
    await packGestures(ctx as unknown as Ctx).finishPack();
    expect(calls).toEqual(["flush", "settle", "savePack", "closePack", "snapshot", "current null", "selected null"]);
  });

  test("finishPack in the deck workshop reads snapRef after the flush", async () => {
    const { ctx, calls, snapRef } = context({ snap: snap({ dirty: true, editing: editing() }) });
    ctx.autosave.flush = async () => {
      calls.push("flush");
      snapRef.current = snap({ dirty: false, editing: editing() });
    };
    await packGestures(ctx as unknown as Ctx).finishPack();
    expect(calls).toEqual(["flush", "settle", "closePack", "snapshot", "current null", "selected null"]);
  });

  test("finishPack in the deck workshop with a cancelled deck save resets the selection", async () => {
    const dirtyDeck = snap({ dirty: true, path: "/d/a.oku-slides.pdf" });
    const api = {
      closePack: async () => (calls.push("closePack"), dirtyDeck),
      save: async () => (calls.push("save"), snap({ dirty: true, disk_replaced: true } as Partial<Snapshot>)),
      snapshot: async () => (calls.push("snapshot"), dirtyDeck),
    };
    const { ctx, calls } = context({ snap: snap({ editing: editing() }), api });
    // Outside Tauri the replaced-file question is window.confirm: false cancels the save.
    const g = globalThis as { window?: unknown };
    const before = g.window;
    g.window = { confirm: () => false };
    try {
      await packGestures(ctx as unknown as Ctx).finishPack();
    } finally {
      g.window = before;
    }
    expect(calls).toEqual(["flush", "settle", "closePack", "save", "snapshot", "current null", "selected null"]);
  });

  test("finishPack in a read-only deck workshop asks the read-only confirm", async () => {
    const confirm = mock(() => true);
    const ro = snap({ dirty: true, editing: editing({ read_only: true, leased: false, display_name: "Acme" }) });
    const { ctx, calls } = context({ snap: ro, confirm });
    await packGestures(ctx as unknown as Ctx).finishPack();
    expect(confirm).toHaveBeenCalledWith(
      "⁨Acme⁩ cannot be saved here. Close this tab and lose its changes? To keep them, ask the agent to save it as a new pack.",
    );
    expect(calls).toEqual(["flush", "settle", "closePack", "snapshot", "current null", "selected null"]);
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
  test("showFinish with host deck is true", () => expect(showFinish(snap({ editing: editing() }), false)).toBe(true));
  test("showFinish with host tab and onClose is true", () => expect(showFinish(snap({ editing: editing({ host: "tab" }) }), true)).toBe(true));
  test("showFinish with host tab without onClose is false", () => expect(showFinish(snap({ editing: editing({ host: "tab" }) }), false)).toBe(false));
  test("showFinish without editing is false", () => expect(showFinish(snap(), true)).toBe(false));
});
