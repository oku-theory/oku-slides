// A pack tab's selected item: restore, fallback, echoes, agent writes, flushes.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";
import type { PackKind } from "./packTypes";

const { act, createElement, StrictMode, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { usePackItem } = await import("./usePackItem");
const { useKeptUi } = await import("./useKeptUi");
const { itemSlide } = await import("./packItems");
const PackPile = (await import("./PackPile")).default;
const { runCloseGuard, useCloseGuard } = await import("./closeGuards");
const { t } = await import("./i18n");
const WidgetPane = (await import("./WidgetPane")).default;
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

type Item = ReturnType<typeof usePackItem>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** A snapshot reduced to what the hook reads; `items` are the pile's items, in slide order. */
function snapOf(o: {
  set?: string | null; version?: string; base?: string; widget?: string | null; history?: boolean; kind?: PackKind; items?: string[];
  slide?: string | null;
}): Snapshot {
  const { set = "p", version = "v", base = "b", widget = null, history = false, kind = "widgets", items = [], slide = null } = o;
  return {
    editing: set === null ? null : { pack: set, base, kind },
    widgets: { deck: { id: "p", version } },
    history: history ? {} : null,
    slides: items.map((n, i) => ({ id: itemSlide(kind, n), kind: "essentiel/content", title: null, page: i + 1, hash: `h${i}` })),
    ui: { slide, grid: true, ghosts: false, pile: null, notes: null, widget, code: false, code_width: null, folded: [] },
  } as unknown as Snapshot;
}

interface Out {
  item: Item | null;
  setSnap: ((s: Snapshot) => void) | null;
  blocked: boolean;
  notices: string[];
}

/** Each mount's close guard tab, unique to it (App registers `item.files.settle` the same way). */
let guardTabs = 0;

async function mount(fake: TabApi, first: Snapshot, strict = false) {
  const guardTab = `item-probe-${++guardTabs}`;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: Out = { item: null, setSnap: null, blocked: false, notices: [] };
  function Probe() {
    const [snap, setSnap] = useState(first);
    out.setSnap = setSnap;
    const item = usePackItem(fake, snap, {
      changed: () => {},
      error: () => {},
      notice: (m: string) => out.notices.push(m),
      blocked: () => out.blocked,
    });
    out.item = item;
    useCloseGuard(guardTab, () => item.files.settle());
    return createElement(WidgetPane, {
      api: fake, item, trials: snap.widget_trials ?? 0, look: "", code: false, codeWidth: null,
      onCodeWidth: () => {}, run: async () => null, palette: [],
    });
  }
  const probe = createElement(Probe);
  await act(async () => root.render(strict ? createElement(StrictMode, null, probe) : probe));
  await wait(10);
  const render = async (s: Snapshot) => {
    await act(async () => out.setSnap!(s));
    await wait(10);
  };
  const select = async (name: string) => {
    await act(async () => out.item!.select(name));
    await wait(10);
  };
  const content = () => host.querySelector(".widget-tab .cm-content") as HTMLElement;
  const typeIn = (insert: string) =>
    act(async () => {
      const v = EditorView.findFromDOM(content())!;
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert }, userEvent: "input.type" });
    });
  const text = () => content()?.textContent;
  return { host, out, render, select, typeIn, text, guardTab };
}

const later = () => wait(PREVIEW_DELAY_MS + 50);

/** `widgetStatus` held until `release`. */
function heldStatus(fake: TabApi) {
  const gate = deferred<void>();
  const real = fake.widgetStatus;
  stub(fake, "widgetStatus", async () => {
    await gate.promise;
    return real();
  });
  return { release: async () => { gate.resolve(); await wait(10); } };
}

describe("usePackItem, restore", () => {
  test("usePackItem restores the host selection once the statuses arrive", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const held = heldStatus(fake);
    const m = await mount(fake, snapOf({ widget: "chart" }));
    expect(m.out.item!.selected).toBeNull();
    await held.release();
    expect(m.out.item!.selected).toBe("chart");
  });

  test("usePackItem keeps the host selection when a snapshot without it arrives before the statuses", async () => {
    const { fake } = memory({ kpi: "k" });
    const held = heldStatus(fake);
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.render(snapOf({ widget: null }));
    await held.release();
    expect(m.out.item!.selected).toBe("kpi");
  });

  test("usePackItem with a host selection gone falls back to the shared file", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const m = await mount(fake, snapOf({ widget: "gone" }));
    expect(m.out.item!.selected).toBe("_commun");
  });

  test("usePackItem with a host selection gone and no shared file selects nothing", async () => {
    const { fake } = memory({ kpi: "k" });
    const asked: string[] = [];
    stub(fake, "widgetSource", async (name: string) => {
      asked.push(name);
      return "";
    });
    const m = await mount(fake, snapOf({ widget: "gone" }));
    expect([m.out.item!.selected, asked]).toEqual([null, []]);
  });

  test("usePackItem opened with items and none named selects the first pile item", async () => {
    const { fake } = memory({ _commun: "s", kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widget: null, items: ["chart", "kpi"] }));
    expect([m.out.item!.selected, m.out.item!.kept.widget]).toEqual(["chart", "chart"]);
  });

  test("usePackItem opened with no items and none named selects the shared file", async () => {
    const { fake } = memory({ _commun: "s" });
    const m = await mount(fake, snapOf({ widget: null }));
    expect(m.out.item!.selected).toBe("_commun");
  });

  test("usePackItem opened with no items, no shared file and none named selects nothing", async () => {
    const { fake } = memory({});
    const m = await mount(fake, snapOf({ widget: null }));
    expect(m.out.item!.selected).toBeNull();
  });

  test("usePackItem keeps a just-created item before the statuses list it", async () => {
    const files: Record<string, string> = { kpi: "k" };
    const { fake } = memory(files);
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.select("neuf");
    await m.render(snapOf({ widget: "kpi" }));
    expect(m.out.item!.selected).toBe("neuf");
    files.neuf = "n";
    await m.render(snapOf({ version: "v2", widget: "neuf" }));
    expect(m.out.item!.selected).toBe("neuf");
  });

  test("usePackItem keeps an agent-written item until it is listed", async () => {
    const files: Record<string, string> = { kpi: "k" };
    const { fake } = memory(files);
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await act(async () => m.out.item!.touched("carte", false));
    await m.render(snapOf({ widget: "kpi" }));
    expect(m.out.item!.selected).toBe("carte");
    files.carte = "c";
    await m.render(snapOf({ version: "v2", widget: "kpi" }));
    expect(m.out.item!.selected).toBe("carte");
  });

  test("usePackItem ignores statuses of another revision", async () => {
    const files: Record<string, string> = { kpi: "k" };
    const { fake } = memory(files);
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    // The agent creates `carte` and the host opens it: a new revision, its statuses late.
    files.carte = "c";
    const held = heldStatus(fake);
    await m.render(snapOf({ version: "v2", widget: "carte" }));
    expect(m.out.item!.selected).toBe("carte");
    await held.release();
    expect(m.out.item!.selected).toBe("carte");
  });
});

describe("usePackItem, fallback and rename", () => {
  test("usePackItem with a selection gone falls back to the neighbour", async () => {
    const files: Record<string, string> = { a: "a", b: "b", c: "c" };
    const { fake } = memory(files);
    const m = await mount(fake, snapOf({ widget: "b" }));
    delete files.b;
    await m.render(snapOf({ version: "v2", widget: "b" }));
    expect(m.out.item!.selected).toBe("c");
  });

  test("usePackItem with a rename keeps the renamed item selected", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widget: "kpi" }), true);
    await act(async () => {
      await m.out.item!.files.rename("kpi", "kpi2");
    });
    await wait(10);
    expect(m.out.item!.selected).toBe("kpi2");
  });

  test("usePackItem removed selects the neighbour of the removed item", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const m = await mount(fake, snapOf({ widget: "b" }));
    await act(async () => m.out.item!.removed("b"));
    expect(m.out.item!.selected).toBe("a");
  });
});

describe("usePackItem, echoes", () => {
  test("usePackItem with a stale echo keeps the newer selection", async () => {
    const { fake } = memory({ a: "a", b: "b" });
    const m = await mount(fake, snapOf({ widget: null }));
    await m.select("a");
    await m.select("b");
    await m.render(snapOf({ widget: "a" }));
    expect(m.out.item!.selected).toBe("b");
  });

  test("usePackItem with a host-initiated open selects it", async () => {
    const { fake } = memory({ a: "a", c: "c" });
    const m = await mount(fake, snapOf({ widget: "a" }));
    await m.render(snapOf({ widget: "c" }));
    expect(m.out.item!.selected).toBe("c");
  });

  test("usePackItem with a second host-initiated open of the same item selects it", async () => {
    const { fake } = memory({ kpi: "k", card: "c" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.render(snapOf({ widget: "card" }));
    await m.select("kpi");
    await m.render(snapOf({ widget: "kpi" }));
    await m.render(snapOf({ widget: "card" }));
    expect(m.out.item!.selected).toBe("card");
  });
});

describe("usePackItem, typing", () => {
  test("usePackItem switching items flushes the previous item's typing", async () => {
    const { fake, log } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.typeIn("k2");
    expect(log).toEqual([]);
    await m.select("chart");
    expect(log).toEqual(["set kpi:k2"]);
  });

  test("usePackItem pending typing is written before the close guard answers", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.typeIn("k2");
    await act(async () => {
      expect(await runCloseGuard(m.guardTab)).toBe("done");
      log.push("close");
    });
    expect(log).toEqual(["set kpi:k2", "close"]);
  });

  test("usePackItem undo and redo drive the selected item's editor", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.typeIn("k2");
    expect(m.out.item!.history).toEqual({ undo: 1, redo: 0 });
    await act(async () => m.out.item!.undo());
    expect([m.text(), m.out.item!.history]).toEqual(["k", { undo: 0, redo: 1 }]);
    await act(async () => m.out.item!.redo());
    expect(m.text()).toBe("k2");
    await later();
  });
});

describe("usePackItem, the agent writes", () => {
  test("usePackItem agent removal of the selected item selects the neighbour, drops its typing and notifies", async () => {
    const mem = memory({ a: "a", b: "b", c: "c" });
    const m = await mount(mem.fake, snapOf({ widget: "b" }));
    await m.typeIn("tapé");
    delete mem.files.b;
    await act(async () => m.out.item!.touched("b", true));
    await later();
    expect(m.out.item!.selected).toBe("c");
    expect(mem.log.some((l) => l.startsWith("set b:"))).toBe(false);
    expect(m.out.notices).toEqual([t("ui-widgets-removed-by-agent", { name: "b" })]);
  });

  test("usePackItem agent removal in a layout pack notifies with the layout wording", async () => {
    const mem = memory({ a: "a", b: "b" });
    const m = await mount(mem.fake, snapOf({ widget: "b", kind: "layouts" }));
    delete mem.files.b;
    await act(async () => m.out.item!.touched("b", true));
    expect(m.out.notices).toEqual([t("ui-layouts-removed-by-agent", { name: "b" })]);
  });

  test("usePackItem agent write to the selected item keeps pending typing", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({ widget: "kpi" }));
    await m.typeIn("tapé");
    mem.files.kpi = "agent";
    await act(async () => m.out.item!.touched("kpi", false));
    await later();
    expect(mem.files.kpi).toBe("tapé");
  });

  test("usePackItem agent write elsewhere selects it unless typing is pending", async () => {
    const mem = memory({ kpi: "k", quote: "q", note: "n" });
    const m = await mount(mem.fake, snapOf({ widget: "kpi" }));
    await act(async () => m.out.item!.touched("quote", false));
    await wait(10);
    expect(m.out.item!.selected).toBe("quote");
    await m.typeIn("tapé");
    await act(async () => m.out.item!.touched("note", false));
    expect(m.out.item!.selected).toBe("quote");
    await later();
  });

  test("usePackItem agent write elsewhere while blocked keeps the selection", async () => {
    const mem = memory({ kpi: "k", quote: "q" });
    const m = await mount(mem.fake, snapOf({ widget: "kpi" }));
    m.out.blocked = true;
    await act(async () => m.out.item!.touched("quote", false));
    expect(m.out.item!.selected).toBe("kpi");
  });

  test("usePackItem outside a pack tab touched selects nothing later", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({ set: null }));
    await act(async () => m.out.item!.touched("kpi", false));
    await m.render(snapOf({}));
    expect(m.out.item!.selected).toBeNull();
  });
});

describe("usePackItem, lifecycle", () => {
  test("usePackItem leaving the pack resets the selection", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.render(snapOf({ set: null }));
    expect([m.out.item!.selected, m.out.item!.kept]).toEqual([null, { widget: null, slide: null }]);
  });

  test("usePackItem another pack restores its own host selection", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widget: "kpi" }));
    await m.render(snapOf({ set: "q", base: "b2", widget: "chart" }));
    expect(m.out.item!.selected).toBe("chart");
  });

  test("usePackItem survives a StrictMode remount", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widget: "chart" }), true);
    expect(m.out.item!.selected).toBe("chart");
    await m.select("kpi");
    expect(m.out.item!.selected).toBe("kpi");
    expect(m.host.querySelectorAll(".widget-tab").length).toBe(1);
  });

  test("usePackItem kept reports the selected item", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const held = heldStatus(fake);
    const m = await mount(fake, snapOf({ widget: "chart", slide: "widget-chart" }));
    // Before the statuses, the host's own selection and slide: never overwritten with nothing.
    expect(m.out.item!.kept).toEqual({ widget: "chart", slide: "widget-chart" });
    await held.release();
    await m.select("kpi");
    expect(m.out.item!.kept).toEqual({ widget: "kpi", slide: "widget-kpi" });
  });
});

describe("usePackItem, the current slide", () => {
  /** A pack tab's pile and kept UI, wired as App wires them; `saved` gets each `ui.slide` sent. */
  async function mountPile(fake: TabApi, first: Snapshot) {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const out: { item: Item | null; saved: (string | null)[] } = { item: null, saved: [] };
    const save = async (ui: { slide: string | null }) => void out.saved.push(ui.slide);
    function Probe() {
      const item = usePackItem(fake, first, { changed: () => {}, error: () => {}, notice: () => {}, blocked: () => false });
      out.item = item;
      useKeptUi(first.ui, item.kept.slide, save, item.kept);
      return createElement(PackPile, {
        snap: first, kind: first.editing!.kind, files: item.files, selected: item.selected, svg: (id: string) => `blob:${id}`,
        onSelect: item.select, onRemoved: item.removed, confirm: async () => true,
      });
    }
    await act(async () => root.render(createElement(Probe)));
    await wait(10);
    return { host, out };
  }

  test.each([["widgets", "widget-b"], ["layouts", "layout-b"]] as [PackKind, string][])(
    "usePackItem selecting b in a %s pile sends ui.slide %s",
    async (kind, slide) => {
      const { fake } = memory({ a: "a", b: "b" });
      const { host, out } = await mountPile(fake, snapOf({ widget: "a", kind, items: ["a", "b"] }));
      const b = host.querySelectorAll<HTMLButtonElement>(".thumb")[1]!;
      await act(async () => void b.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
      await wait(10);
      expect([out.item!.slide, out.saved.at(-1)]).toEqual([slide, slide]);
    },
  );

  test("usePackItem with the shared file selected has no current slide", async () => {
    const { fake } = memory({ _commun: "s", a: "a" });
    const { out } = await mountPile(fake, snapOf({ widget: "_commun", items: ["a"] }));
    expect([out.item!.slide, out.saved.at(-1)]).toEqual([null, null]);
  });
});
