// Volet des sous-onglets de widgets : tous montés, seul l'actif visible.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useTemplateWidgets } = await import("./useTemplateWidgets");
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

async function mount(fake: TabApi, widgets: string[], widget: string | null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const snap = {
    editing: { set: "p", base: "b" },
    templates: { version: "v" },
    history: null,
    ui: { slide: null, grid: true, ghosts: false, pile: null, notes: null, widgets, widget },
  } as unknown as Snapshot;
  function Probe() {
    const w = useTemplateWidgets(fake, snap, { changed: () => {}, error: () => {}, notice: () => {}, blocked: () => false });
    return createElement(WidgetPane, { api: fake, widgets: w, trials: 0, look: "", code: false, codeWidth: null, onCodeWidth: () => {},
      run: async () => null, palette: [] });
  }
  await act(async () => root.render(createElement(Probe)));
  await wait(10);
  return host;
}

describe("WidgetPane", () => {
  test("tous les sous-onglets sont montés ; seul l'actif est visible", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const host = await mount(fake, ["kpi", "chart"], "chart");
    const tabs = [...host.querySelectorAll<HTMLElement>(".widget-tab")];
    expect(tabs.length).toBe(2);
    expect(tabs.map((t) => t.hidden)).toEqual([true, false]);
    expect(host.querySelector<HTMLElement>(".widget-pane")!.hidden).toBe(false);
    expect([...host.querySelectorAll(".widget-tab .cm-content")].map((c) => c.textContent)).toEqual(["k", "c"]);
  });

  test("sur Modèles, le volet entier est caché", async () => {
    const { fake } = memory({ kpi: "k" });
    const host = await mount(fake, ["kpi"], null);
    expect(host.querySelectorAll(".widget-tab").length).toBe(1);
    expect(host.querySelector<HTMLElement>(".widget-pane")!.hidden).toBe(true);
  });
});
