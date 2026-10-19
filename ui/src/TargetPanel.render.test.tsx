// La carte de la cible, sous la scène : graphique, réglages, ou rien.

import { afterEach, describe, expect, test } from "bun:test";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import "./test/dom";
import { CATALOG_ID } from "./a2ui/catalog";
import type { Element, HostParams, SlideView, Surface } from "./api";
import type { HostSettings } from "./useHostSettings";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { TargetPanel } = await import("./TargetPanel");
const { bandHeight } = await import("./bandSplit");
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

const templateSurface: Surface = {
  ...surface,
  id: "host-template",
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "host-template", catalogId: CATALOG_ID } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "host-template",
        components: [{ id: "root", component: "Button", label: "Modèle", action: { event: { name: "param", context: {} } } }],
      },
    },
  ] as unknown as A2uiMessage[],
};

const view = { slide: { id: "s-ch1", elements: [] } } as unknown as SlideView;
const chart = {
  id: "c1",
  kind: "base/chart",
  params: { option: { dataset: { source: [["Catégorie", "Série 1"], ["1", 3]] }, series: [{ type: "bar" }] } },
} as unknown as Element;
const table = {
  id: "t1",
  kind: "base/table",
  params: { data: { dataset: { source: [["A", "B"], ["x", 1]] } } },
} as unknown as Element;
const widget = { id: "fr", kind: "deck/frise", params: {} } as unknown as Element;
const plot = { id: "p1", kind: "acme/plot", params: { series: { dataset: { source: [["C", "S"], ["1", 3]] }, series: [{ type: "bar" }] } } } as unknown as Element;
const text = { id: "x1", kind: "base/text", params: {} } as unknown as Element;
const params: HostParams = { "base/chart": { chart: "option" }, "base/table": { table: "data" }, "acme/plot": { chart: "series" } };

interface Opts { selected?: Element; hostParams?: HostParams; shown?: Surface | null; readOnly?: boolean; withView?: boolean; folded?: boolean; lead?: Surface;
  band?: number | null; onBand?: (band: number | null) => void }

async function mount({ selected, hostParams = params, shown = surface, readOnly = false, withView = true, folded = false, lead, band = null, onBand = () => {} }: Opts) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const target: HostSettings = { surface: shown, onAction: () => true, outcome: { current: null } };
  await act(async () => {
    root.render(
      createElement(TargetPanel, {
        view: withView ? view : null, readOnly, selected, hostParams, target, palette: ["#0077b6"],
        onApply: () => {}, onKind: () => {}, onSurfaceAction: () => {}, onError: () => {},
        folded, band, onBand,
        lead: lead && { surface: lead, outcome: { current: null } },
      }),
    );
  });
  await wait(20);
  return host;
}

describe("TargetPanel", () => {
  test("TargetPanel folded shows nothing", async () => {
    const host = await mount({ selected: widget, folded: true });
    expect(host.innerHTML).toBe("");
  });

  test("TargetPanel folded hides the chart panel too", async () => {
    const host = await mount({ selected: chart, folded: true });
    expect(host.querySelector(".chart-panel")).toBeNull();
  });

  test("TargetPanel chart selected shows ChartPanel only", async () => {
    const host = await mount({ selected: chart });
    expect(host.querySelector(".chart-panel")).not.toBeNull();
    expect(host.querySelector(".host-settings")).toBeNull();
  });

  test("TargetPanel table selected shows TablePanel only", async () => {
    const host = await mount({ selected: table });
    expect(host.querySelector(".table-panel")).not.toBeNull();
    expect(host.querySelector(".host-settings")).toBeNull();
  });

  test("TargetPanel élément de pack à hôte graphique montre ChartPanel", async () => {
    const host = await mount({ selected: plot });
    expect(host.querySelector(".chart-panel")).not.toBeNull();
  });

  test("TargetPanel texte de base montre les réglages, sans panneau propre", async () => {
    const host = await mount({ selected: text });
    expect(host.querySelector(".chart-panel") ?? host.querySelector(".table-panel")).toBeNull();
    expect(host.querySelector(".host-settings")).not.toBeNull();
  });

  test("TargetPanel widget selected shows its settings", async () => {
    const host = await mount({ selected: widget });
    expect(host.querySelector('.host-settings [data-surface-id="host-target"]')).not.toBeNull();
    expect(host.querySelector(".chart-panel")).toBeNull();
  });

  // Sans surface (le temps de la lire, ou élément sans paramètres) : la bande reste, la slide ne bouge pas.
  test("TargetPanel no surface keeps its empty band", async () => {
    const host = await mount({ selected: widget, shown: null });
    const band = host.querySelector(".settings-band") as HTMLElement;
    expect([...band.children].map((c) => c.getAttribute("role"))).toEqual(["separator"]);
  });

  test("TargetPanel band opens on a horizontal separator above its card", async () => {
    const host = await mount({ selected: table });
    const band = host.querySelector(".settings-band") as HTMLElement;
    expect(band.firstElementChild?.getAttribute("aria-orientation")).toBe("horizontal");
  });

  test("TargetPanel chosen band height is set on the band", async () => {
    const host = await mount({ selected: table, band: 320 });
    const band = host.querySelector(".settings-band") as HTMLElement;
    expect(band.style.getPropertyValue("--band-h")).toBe(bandHeight(320));
  });

  test("TargetPanel default band height leaves the style sheet in charge", async () => {
    const host = await mount({ selected: table });
    const band = host.querySelector(".settings-band") as HTMLElement;
    expect(band.style.getPropertyValue("--band-h")).toBe("");
  });

  test("TargetPanel double click on the separator resets the band height", async () => {
    const bands: (number | null)[] = [];
    const host = await mount({ selected: table, band: 320, onBand: (b) => bands.push(b) });
    await act(async () => {
      host.querySelector("[role=separator]")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(bands).toEqual([null]);
  });

  /** Glisse le séparateur de `from` à `to` (clientY) dans une colonne de 800 px dont 320 à la bande. */
  async function drag(host: HTMLElement, moves: number[]) {
    const band = host.querySelector(".settings-band") as HTMLElement;
    Object.defineProperty(host, "clientHeight", { configurable: true, value: 800 });
    Object.defineProperty(band, "offsetHeight", { configurable: true, value: 320 });
    const sep = band.querySelector("[role=separator]") as HTMLElement;
    sep.setPointerCapture = () => {};
    const [first, ...rest] = moves;
    const fire = (type: string, y: number) =>
      act(async () => {
        sep.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: 0, clientY: y }));
      });
    await fire("pointerdown", first);
    for (const y of rest.slice(0, -1)) await fire("pointermove", y);
    return { band, release: () => fire("pointerup", rest[rest.length - 1]) };
  }

  test("TargetPanel dragging the separator shows the height live, kept only on release", async () => {
    const bands: (number | null)[] = [];
    const host = await mount({ selected: table, band: 320, onBand: (b) => bands.push(b) });
    const { band, release } = await drag(host, [500, 450, 440]);
    expect([band.style.getPropertyValue("--band-h"), bands]).toEqual([bandHeight(370), []]);
    await release();
    expect([band.style.getPropertyValue("--band-h"), bands]).toEqual([bandHeight(380), [380]]);
  });

  test("TargetPanel drag released back on the kept height shows the kept height", async () => {
    const bands: (number | null)[] = [];
    const host = await mount({ selected: table, band: 320, onBand: (b) => bands.push(b) });
    const { band, release } = await drag(host, [500, 440, 500]);
    await release();
    expect([band.style.getPropertyValue("--band-h"), bands]).toEqual([bandHeight(320), [320]]);
  });

  test("TargetPanel read only shows nothing", async () => {
    expect((await mount({ selected: widget, readOnly: true })).innerHTML).toBe("");
  });

  test("TargetPanel without view shows nothing", async () => {
    expect((await mount({ selected: chart, withView: false })).innerHTML).toBe("");
  });

  test("TargetPanel with a lead merges it with the target settings in one card", async () => {
    const host = await mount({ selected: widget, lead: templateSurface });
    const boxes = host.querySelectorAll(".host-settings");
    const ids = [...host.querySelectorAll<HTMLElement>("[data-surface-id]")].map((e) => e.dataset.surfaceId);
    expect([boxes.length, ids]).toEqual([1, ["host-template", "host-target"]]);
  });

  test("TargetPanel with a lead and no target settings shows the lead alone", async () => {
    const host = await mount({ selected: widget, shown: null, lead: templateSurface });
    const ids = [...host.querySelectorAll<HTMLElement>("[data-surface-id]")].map((e) => e.dataset.surfaceId);
    expect(ids).toEqual(["host-template"]);
  });

  test("TargetPanel with a lead and a chart selected shows the lead beside the chart panel", async () => {
    const host = await mount({ selected: chart, lead: templateSurface });
    expect([!!host.querySelector('[data-surface-id="host-template"]'), !!host.querySelector(".chart-panel")]).toEqual([true, true]);
  });

  test("TargetPanel folded still shows the lead", async () => {
    const host = await mount({ selected: widget, folded: true, lead: templateSurface });
    const ids = [...host.querySelectorAll<HTMLElement>("[data-surface-id]")].map((e) => e.dataset.surfaceId);
    expect(ids).toEqual(["host-template"]);
  });
});
