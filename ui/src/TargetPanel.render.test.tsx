// La carte de la cible, en tête de la colonne des notes : graphique, réglages, ou rien.

import { afterEach, describe, expect, test } from "bun:test";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import "./test/dom";
import { CATALOG_ID } from "./a2ui/catalog";
import type { Element, SlideView, Surface } from "./api";
import type { HostSettings } from "./useHostSettings";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { TargetPanel } = await import("./TargetPanel");
const { wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const surface: Surface = {
  id: "host-target",
  place: "dock",
  live: true,
  question: false,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "host-target", catalogId: CATALOG_ID } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "host-target",
        components: [{ id: "root", component: "Button", label: "Essai", action: { event: { name: "param", context: {} } } }],
      },
    },
  ] as unknown as A2uiMessage[],
};

const view = { slide: { id: "s-ch1", elements: [] } } as unknown as SlideView;
const chart = {
  id: "c1",
  kind: "chart",
  params: { option: { dataset: { source: [["Catégorie", "Série 1"], ["1", 3]] }, series: [{ type: "bar" }] } },
} as unknown as Element;
const widget = { id: "fr", kind: "frise", params: {} } as unknown as Element;

interface Opts { selected?: Element; shown?: Surface | null; readOnly?: boolean; withView?: boolean }

async function mount({ selected, shown = surface, readOnly = false, withView = true }: Opts) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const target: HostSettings = { surface: shown, onAction: () => true, outcome: { current: null } };
  await act(async () => {
    root.render(
      createElement(TargetPanel, {
        view: withView ? view : null, readOnly, selected, target, palette: ["#0077b6"],
        onApply: () => {}, onKind: () => {}, onSurfaceAction: () => {}, onError: () => {},
      }),
    );
  });
  await wait(20);
  return host;
}

describe("TargetPanel", () => {
  test("TargetPanel chart selected shows ChartPanel only", async () => {
    const host = await mount({ selected: chart });
    expect(host.querySelector(".chart-panel")).not.toBeNull();
    expect(host.querySelector(".host-settings")).toBeNull();
  });

  test("TargetPanel widget selected shows its settings", async () => {
    const host = await mount({ selected: widget });
    expect(host.querySelector('.host-settings[data-surface-id="host-target"]')).not.toBeNull();
    expect(host.querySelector(".chart-panel")).toBeNull();
  });

  test("TargetPanel read only shows nothing", async () => {
    expect((await mount({ selected: widget, readOnly: true })).innerHTML).toBe("");
  });

  test("TargetPanel without view shows nothing", async () => {
    expect((await mount({ selected: chart, withView: false })).innerHTML).toBe("");
  });

  test("TargetPanel no surface shows nothing", async () => {
    expect((await mount({ selected: widget, shown: null })).innerHTML).toBe("");
  });
});
