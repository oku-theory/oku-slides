// Rendu de la vue des données d'un graphique dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import type { DataEdit } from "./dataEdit";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ChartDataView } = await import("./ChartPanel");
const { t } = await import("../i18n");
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

type Swatch = { name: string; color?: string };

async function mount({
  kind = "bar" as string | undefined,
  source = [["Catégorie", "Série 1", "Série 2"], ["1", 3, 4]] as (string | number)[][],
  stacked = undefined as boolean | undefined,
  series = [{ name: "Série 1" }] as Swatch[],
} = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const edits: DataEdit[] = [];
  await act(async () => {
    root.render(createElement(ChartDataView, {
      kind, source, checkboxColumns: [], stacked, series, palette: ["#0077b6", "#e76f51"], onEdit: (e: DataEdit) => edits.push(e),
    }));
  });
  return { host, edits };
}

const click = (el: Element | null) => act(async () => (el as HTMLButtonElement).click());

describe("ChartDataView", () => {
  test("ChartDataView kind buttons show an icon without visible text", async () => {
    const { host } = await mount();
    const buttons = [...host.querySelectorAll(".chart-kinds button")];
    expect(buttons).toHaveLength(CHART_KINDS.length);
    for (const b of buttons) expectIconOnly(b);
  });

  test("ChartDataView presses the button of its kind", async () => {
    const { host } = await mount({ kind: "line" });
    const pressed = [...host.querySelectorAll(".chart-kinds button[aria-pressed=true]")];
    expect(pressed.map((b) => b.getAttribute("aria-label"))).toEqual([t("ui-chart-kind", { kind: "line" })]);
  });

  test("ChartDataView choosing another kind sends a kind edit", async () => {
    const { host, edits } = await mount();
    await click(host.querySelectorAll(".chart-kinds button")[1]);
    expect(edits).toEqual([{ edit: "kind", kind: "line" }]);
  });

  test("ChartDataView choosing its own kind sends nothing", async () => {
    const { host, edits } = await mount();
    await click(host.querySelectorAll(".chart-kinds button")[0]);
    expect(edits).toEqual([]);
  });

  test("ChartDataView without stacked shows no stack option", async () => {
    const { host } = await mount();
    expect(host.querySelector(".chart-stack")).toBeNull();
  });

  test("ChartDataView stack option shows an icon and sends the flipped stack", async () => {
    const { host, edits } = await mount({ stacked: false });
    expectIconOnly(host.querySelector(".chart-stack button"));
    await click(host.querySelector(".chart-stack button"));
    expect(edits).toEqual([{ edit: "stack", stacked: true }]);
  });

  test("ChartDataView add row and column buttons show icons only", async () => {
    const { host } = await mount();
    const [row, column] = host.querySelectorAll(".cdg-actions button");
    expectIconOnly(row);
    expectIconOnly(column);
  });

  test("ChartDataView adding a row sends a cells edit", async () => {
    const { host, edits } = await mount();
    await click(host.querySelectorAll(".cdg-actions button")[0]);
    expect(edits[0]).toMatchObject({ edit: "cells", change: { op: "insert_row", at: 2 } });
  });

  test("ChartDataView picking a palette colour sends a color edit", async () => {
    const { host, edits } = await mount();
    await click(host.querySelector(".series-chip"));
    await click(host.querySelectorAll(".swatches .swatch")[2]);
    expect(edits).toEqual([{ edit: "color", index: 0, color: "#e76f51" }]);
  });

  test("ChartDataView show-as-table button shows an icon and sends a show_as edit", async () => {
    const { host, edits } = await mount();
    const button = host.querySelector(".show-as button") as HTMLButtonElement;
    expectIconOnly(button);
    await click(button);
    expect(edits).toEqual([{ edit: "show_as", view: "table" }]);
  });

  test("ChartDataView show-as button is disabled without data", async () => {
    const { host } = await mount({ kind: undefined, source: [] });
    expect((host.querySelector(".show-as button") as HTMLButtonElement).disabled).toBe(true);
  });

  test("ChartDataView show-as button closes the kinds toolbar row, after the stack toggle", async () => {
    const { host } = await mount({ stacked: false });
    const toolbar = host.querySelector(".panel-toolbar") as HTMLElement;
    expect([...toolbar.children].map((c) => c.classList[0])).toEqual(["chart-kinds", "chart-stack", "show-as"]);
  });
});
