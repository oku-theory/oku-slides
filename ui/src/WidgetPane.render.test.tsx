// The sheet area of a pack tab: the selected item's view alone, or nothing.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePackItem } = await import("./usePackItem");
const WidgetPane = (await import("./WidgetPane")).default;
const { memory, wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(fake: TabApi, widget: string | null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const snap = {
    editing: { pack: "p", base: "b", kind: "widgets" },
    widgets: { deck: { id: "p", version: "v" } },
    history: null,
    slides: [],
    ui: { slide: null, ghosts: false, pile: null, notes: null, widget },
  } as unknown as Snapshot;
  function Probe() {
    const item = usePackItem(fake, snap, { changed: () => {}, error: () => {}, notice: () => {}, blocked: () => false });
    return createElement(WidgetPane, { api: fake, item, trials: 0, look: "", code: false, codeWidth: null, onCodeWidth: () => {},
      run: async () => null, palette: [] });
  }
  await act(async () => root.render(createElement(Probe)));
  await wait(10);
  return host;
}

describe("WidgetPane", () => {
  test("WidgetPane with a selected item mounts its view alone", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const host = await mount(fake, "chart");
    expect([...host.querySelectorAll(".widget-tab .cm-content")].map((c) => c.textContent)).toEqual(["c"]);
  });

  test("WidgetPane with no item selected shows nothing", async () => {
    const { fake } = memory({});
    const host = await mount(fake, null);
    expect([host.querySelectorAll(".widget-tab").length, host.querySelector(".widget-pane")?.textContent]).toEqual([0, ""]);
  });
});
