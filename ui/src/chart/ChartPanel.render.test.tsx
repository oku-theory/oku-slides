// Rendu du panneau de graphique dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ChartPanel } = await import("./ChartPanel");
const { CHART_KINDS } = await import("./chartDeck");
const { expectIconOnly } = await import("../test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

type Series = { type: string; stack?: string };
type Applied = { label: string; ops: Record<string, unknown>[] };

async function mount(
  series: Series[] = [{ type: "bar" }],
  onShowAs = (_view: string) => {},
  dataset: object | null = { source: [["Catégorie", "Série 1", "Série 2"], ["1", 3, 4]] },
  { kind, param }: { kind: string; param: string } = { kind: "base/chart", param: "option" },
) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const applied: Applied[] = [];
  const element = { id: "c1", kind, params: { [param]: { ...(dataset ? { dataset } : {}), series } } };
  const onApply = (label: string, ops: Record<string, unknown>[]) => applied.push({ label, ops });
  await act(async () => {
    root.render(createElement(ChartPanel, { slide: "s1", element, param, palette: ["#0077b6"], onApply, onKind: () => {}, onShowAs }));
  });
  return { host, applied };
}

describe("ChartPanel", () => {
  test("ChartPanel kind buttons show an icon without visible text", async () => {
    const { host } = await mount();
    const buttons = [...host.querySelectorAll(".chart-kinds button")];
    expect(buttons).toHaveLength(CHART_KINDS.length);
    for (const b of buttons) expectIconOnly(b);
  });

  test("ChartPanel stack option shows an icon only", async () => {
    const { host } = await mount([{ type: "bar" }, { type: "bar" }]);
    expectIconOnly(host.querySelector(".chart-stack button"));
  });

  test("ChartPanel add row and column buttons show icons only", async () => {
    const { host } = await mount();
    const [row, column] = host.querySelectorAll(".cdg-actions button");
    expectIconOnly(row);
    expectIconOnly(column);
  });

  test("ChartPanel show-as-table button shows an icon and asks for the table view", async () => {
    const views: string[] = [];
    const { host } = await mount([{ type: "bar" }], (view: string) => views.push(view));
    const button = host.querySelector(".show-as button") as HTMLButtonElement;
    expectIconOnly(button);
    await act(async () => button.click());
    expect(views).toEqual(["table"]);
  });

  test("ChartPanel show-as button is disabled without a dataset source", async () => {
    const { host } = await mount([{ type: "bar" }], () => {}, null);
    const button = host.querySelector(".show-as button") as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  test("ChartPanel show-as button closes the kinds toolbar row, after the stack toggle", async () => {
    const { host } = await mount([{ type: "bar" }, { type: "bar" }]);
    const toolbar = host.querySelector(".panel-toolbar") as HTMLElement;
    expect([...toolbar.children].map((c) => c.classList[0])).toEqual(["chart-kinds", "chart-stack", "show-as"]);
  });

  test("le panneau d'un graphique de pack écrit dans son paramètre", async () => {
    const { host, applied } = await mount([{ type: "bar" }, { type: "bar" }], undefined, undefined, { kind: "acme/plot", param: "series" });
    await act(async () => (host.querySelector(".chart-stack button") as HTMLButtonElement).click());
    expect(applied[0].ops[0]).toMatchObject({ op: "set_chart_option", param: "series" });
  });
});
