// Rendu de la vue des données d'un tableau dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import type { DataEdit } from "../chart/dataEdit";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { TableDataView } = await import("./TablePanel");
const { NumberLangContext } = await import("../a2ui/numberLang");
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

type Props = { source: (string | number)[][]; headerRow: boolean; headerColumn: boolean; striped: boolean; align: string[] };
const DEFAULTS: Props = { source: [["A", "B"], ["x", 1]], headerRow: true, headerColumn: false, striped: false, align: [] };

async function mount(props: Partial<Props> = {}, numberLang?: string) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const edits: DataEdit[] = [];
  await act(async () => {
    const view = createElement(TableDataView, { ...DEFAULTS, ...props, onEdit: (e: DataEdit) => edits.push(e) });
    root.render(numberLang === undefined ? view : createElement(NumberLangContext.Provider, { value: numberLang }, view));
  });
  return { host, edits };
}

const options = (host: HTMLElement) => [...host.querySelectorAll(".table-options button")] as HTMLButtonElement[];

describe("TableDataView", () => {
  test("TableDataView grid writes numbers in the deck language", async () => {
    const { host } = await mount({ source: [["A"], [12345.5]] }, "fr");
    const input = host.querySelector("input.cdg-number") as HTMLInputElement;
    expect(input.value).toBe("12\u202f345,5");
  });

  test("TableDataView option buttons show icons only", async () => {
    const { host } = await mount();
    expect(options(host)).toHaveLength(3);
    for (const b of options(host)) expectIconOnly(b);
  });

  test("TableDataView presses the flags it is given", async () => {
    const { host } = await mount({ headerRow: false, striped: true });
    expect(options(host).map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "false", "true"]);
  });

  test("TableDataView striped toggle sends a flag edit", async () => {
    const { host, edits } = await mount();
    await act(async () => options(host)[2].click());
    expect(edits).toEqual([{ edit: "flag", flag: "striped", on: true }]);
  });

  test("TableDataView alignment select sends an align edit", async () => {
    const { host, edits } = await mount();
    const select = host.querySelectorAll(".table-align select")[1] as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(select, "center");
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(edits).toEqual([{ edit: "align", column: 1, align: "center" }]);
  });

  test("TableDataView alignment select shows the given alignment", async () => {
    const { host } = await mount({ align: ["auto", "end"] });
    expect((host.querySelectorAll(".table-align select")[1] as HTMLSelectElement).value).toBe("end");
  });

  test("TableDataView add column sends a cells edit with a column name", async () => {
    const { host, edits } = await mount();
    const [, addColumn] = host.querySelectorAll(".cdg-actions button");
    await act(async () => (addColumn as HTMLButtonElement).click());
    expect(edits[0]).toMatchObject({ edit: "cells", change: { op: "insert_column", at: 2 } });
    const cells = (edits[0] as { change: { cells: string[] } }).change.cells;
    expect(cells[0]).not.toContain("Series");
  });

  test("TableDataView without data shows the no-data hint", async () => {
    const { host } = await mount({ source: [] });
    expect(host.querySelector(".a2ui-hint")).not.toBeNull();
  });

  test("TableDataView show-as-chart button sends a show_as edit", async () => {
    const { host, edits } = await mount();
    const button = host.querySelector(".show-as button") as HTMLButtonElement;
    expectIconOnly(button);
    await act(async () => button.click());
    expect(edits).toEqual([{ edit: "show_as", view: "chart" }]);
  });

  test("TableDataView show-as button closes the options toolbar row", async () => {
    const { host } = await mount();
    const toolbar = host.querySelector(".panel-toolbar") as HTMLElement;
    expect(toolbar.querySelector(".table-options")).not.toBeNull();
    expect(toolbar.lastElementChild?.classList.contains("show-as")).toBe(true);
  });
});
