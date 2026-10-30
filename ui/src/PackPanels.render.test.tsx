// A pack tab's panels: header, tools, pile and the one selected item's view.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Op, Snapshot, TabApi } from "./api";
import type { PackKind } from "./packTypes";
import type { KeptUi } from "./useKeptUi";

const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PackPanels } = await import("./PackPanels");
const { usePackItem } = await import("./usePackItem");
const { itemSlide } = await import("./packItems");
const { memory, stub, wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const THEMES = [
  { id: "p", version: "v", name: "Papier", namespace: "papier", extends: [], builtin: true },
  { id: "n", version: "v", name: "Nuit", namespace: "nuit", extends: [], builtin: true },
];

/** A pack tab's snapshot with the fields the panels read. */
function snapOf(kind: PackKind, items: string[], widget: string | null): Snapshot {
  return {
    name: "acme", theme: null, themes: [], title: "Acme", title_source: "Acme", path: null, dirty: false, history: null, read_only: false, theming: null,
    editing: { pack: "p", name: "Acme", kind, base: "b", deck_only: false, read_only: false, leased: true },
    widgets: {}, layouts: {}, widget_trials: 0, free_slots: {}, pack_updates: 0, can_undo: false, can_redo: false,
    undo_label: null, redo_label: null, appearance: { chart_palette: [] },
    slides: items.map((n, i) => ({ id: itemSlide(kind, n), kind: "essentiel/content", title: null, page: i + 1, hash: `h${i}` })),
    ui: { slide: null, ghosts: false, pile: null, notes: null, widget, code: false, code_width: null, folded: [] },
  } as unknown as Snapshot;
}

function keptUi(): KeptUi & { foldedNow: string[] } {
  const k = {
    ghosts: false, columns: { pile: null, notes: null }, setColumns: () => {}, code: false, setCode: () => {},
    codeWidth: null, setCodeWidth: () => {}, folded: [] as string[], foldedNow: [] as string[],
    setFolded: (s: string, fold: boolean) => { k.foldedNow = fold ? [...k.foldedNow, s] : k.foldedNow.filter((x) => x !== s); },
    setGhosts: () => {}, handleKey: () => false,
  };
  return k as unknown as KeptUi & { foldedNow: string[] };
}

async function mount(fake: TabApi, snap: Snapshot, over: { onClose?: () => void; run?: (p: Promise<Snapshot | null>) => Promise<Snapshot | null> } = {}) {
  const applied: Op[][] = [];
  const dialogs: boolean[] = [];
  stub(fake, "apply", async (_label: string, ops: Op[]) => (applied.push(ops), snap));
  const kept = keptUi();
  function Probe() {
    const [s] = useState(snap);
    const item = usePackItem(fake, s, { changed: () => {}, error: () => {}, notice: () => {}, blocked: () => false });
    const k = { ...kept, folded: kept.foldedNow } as KeptUi;
    return createElement(PackPanels, {
      api: fake, snap: s, item, keptUi: k, look: "", run: over.run ?? ((p: Promise<Snapshot | null>) => p),
      settings: null, svg: (id: string) => `blob:${id}`, mainMenu: null, drawer: null, onClose: over.onClose,
      onDialog: (open: boolean) => dialogs.push(open),
    });
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  await wait(10);
  const rerender = () => act(async () => root.render(createElement(Probe)));
  return { host, applied, dialogs, kept, rerender };
}

describe("PackPanels", () => {
  test("PackPanels of an empty pack shows the empty text once, in the pile, and no item view", async () => {
    const { fake } = memory({});
    const { host } = await mount(fake, snapOf("widgets", [], null));
    expect(host.textContent?.split("ui-widgets-empty").length).toBe(2);
    expect(host.querySelector(".pile-column")?.textContent).toContain("ui-widgets-empty");
    expect(host.querySelector(".widget-tab")).toBeNull();
  });

  test("PackPanels with a selected item shows exactly one item view and no tab", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const { host } = await mount(fake, snapOf("widgets", ["chart", "kpi"], "kpi"));
    const sheet = host.querySelector(".sheet-area")!;
    expect([sheet.querySelectorAll(".widget-tab").length, sheet.querySelectorAll('[role="tab"]').length]).toEqual([1, 0]);
  });

  test("PackPanels shows the finish button when the tab can close", async () => {
    let closed = 0;
    const { fake } = memory({});
    const { host } = await mount(fake, snapOf("widgets", [], null), { onClose: () => closed++ });
    const finish = host.querySelector<HTMLButtonElement>('[aria-label="ui-toolbar-finish"]');
    await act(async () => finish?.click());
    expect(closed).toBe(1);
  });

  test("PackPanels without a way to close shows no finish button", async () => {
    const { fake } = memory({});
    const { host } = await mount(fake, snapOf("widgets", [], null));
    expect(host.querySelector('[aria-label="ui-toolbar-finish"]')).toBeNull();
  });

  test("PackPanels the title commits set_deck_param", async () => {
    const { fake } = memory({});
    const { host, applied } = await mount(fake, snapOf("widgets", [], null));
    await act(async () => host.querySelector<HTMLButtonElement>(".doc-name-button")?.click());
    const input = host.querySelector<HTMLInputElement>(".doc-name-input")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, "Nouveau");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => void input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(applied).toEqual([[{ op: "set_deck_param", key: "title", value: "Nouveau" }]]);
  });

  test("PackPanels of a layout pack folds its layouts section", async () => {
    const { fake } = memory({ cover: "c" });
    const m = await mount(fake, snapOf("layouts", ["cover"], "cover"));
    expect(m.host.querySelectorAll(".pile-column .thumb").length).toBe(1);
    await act(async () => m.host.querySelector<HTMLButtonElement>(".pile-section-toggle")?.click());
    await m.rerender();
    expect([m.kept.foldedNow, m.host.querySelectorAll(".pile-column .thumb").length]).toEqual([["layouts"], 0]);
  });

  test("PackPanels a delete asks in a ConfirmDialog and reports it open", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, dialogs } = await mount(fake, snapOf("widgets", ["kpi"], "kpi"));
    await act(async () => host.querySelector<HTMLButtonElement>(".thumb-remove")?.click());
    expect([host.querySelector('[role="alertdialog"]') !== null, dialogs.at(-1)]).toEqual([true, true]);
    await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  });
  test("PackPanels unmounted with the dialog open reports it closed", async () => {
    const { fake } = memory({ kpi: "k" });
    const { host, dialogs } = await mount(fake, snapOf("widgets", ["kpi"], "kpi"));
    await act(async () => host.querySelector<HTMLButtonElement>(".thumb-remove")?.click());
    const m = mounted!;
    mounted = null;
    await act(async () => m.root.unmount());
    m.host.remove();
    expect(dialogs.at(-1)).toBe(false);
  });

  test("PackPanels the preview theme menu shows the snapshot's theme and picks through run", async () => {
    const { fake } = memory({});
    const picked: string[] = [];
    stub(fake, "setPreviewTheme", async (ns: string) => (picked.push(ns), null));
    const ran: Promise<Snapshot | null>[] = [];
    const snap = { ...snapOf("widgets", [], null), theme: "papier", themes: THEMES } as unknown as Snapshot;
    const { host } = await mount(fake, snap, { run: (p) => (ran.push(p), p) });
    const select = host.querySelector<HTMLSelectElement>("header select")!;
    expect(select.value).toBe("papier");
    await act(async () => {
      select.value = "nuit";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect([picked, ran.length]).toEqual([["nuit"], 1]);
    // A refusal (run answers null) leaves the menu on the snapshot's theme.
    expect(host.querySelector<HTMLSelectElement>("header select")!.value).toBe("papier");
  });
});
