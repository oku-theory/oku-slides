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

async function mount(series: Series[] = [{ type: "bar" }]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const element = {
    id: "c1",
    kind: "chart" as const,
    option: { dataset: { source: [["Catégorie", "Série 1", "Série 2"], ["1", 3, 4]] }, series },
  };
  await act(async () => {
    root.render(createElement(ChartPanel, { slide: "s1", element, palette: ["#0077b6"], onApply: () => {}, onKind: () => {} }));
  });
  return host;
}

describe("ChartPanel", () => {
  test("ChartPanel kind buttons show an icon without visible text", async () => {
    const host = await mount();
    const buttons = [...host.querySelectorAll(".chart-kinds button")];
    expect(buttons).toHaveLength(CHART_KINDS.length);
    for (const b of buttons) expectIconOnly(b);
  });

  test("ChartPanel stack option shows an icon only", async () => {
    const host = await mount([{ type: "bar" }, { type: "bar" }]);
    expectIconOnly(host.querySelector(".chart-options button"));
  });

  test("ChartPanel add row and column buttons show icons only", async () => {
    const host = await mount();
    const [row, column] = host.querySelectorAll(".cdg-actions button");
    expectIconOnly(row);
    expectIconOnly(column);
  });
});
