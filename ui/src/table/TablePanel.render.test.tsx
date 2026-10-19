// Rendu du panneau de tableau dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { TablePanel } = await import("./TablePanel");
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

type Applied = { label: string; ops: Record<string, unknown>[] };
const target = { slide: "s1", element: "t1" };

async function mount(
  data: Record<string, unknown> = { dataset: { source: [["A", "B"], ["x", 1]] } },
  onShowAs = (_view: string) => {},
  numberLang?: string,
  { kind, param }: { kind: string; param: string } = { kind: "base/table", param: "data" },
) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const applied: Applied[] = [];
  const element = { id: "t1", kind, params: { [param]: data } };
  const onApply = (label: string, ops: Record<string, unknown>[]) => applied.push({ label, ops });
  await act(async () => {
    const panel = createElement(TablePanel, { slide: "s1", element, param, onApply, onShowAs });
    root.render(numberLang === undefined ? panel : createElement(NumberLangContext.Provider, { value: numberLang }, panel));
  });
  return { host, applied };
}

const options = (host: HTMLElement) => [...host.querySelectorAll(".table-options button")] as HTMLButtonElement[];

describe("TablePanel", () => {
  test("TablePanel grid writes numbers in the deck language", async () => {
    const { host } = await mount({ dataset: { source: [["A"], [12345.5]] } }, undefined, "fr");
    const input = host.querySelector("input.cdg-number") as HTMLInputElement;
    expect(input.value).toBe("12\u202f345,5");
  });

  test("TablePanel option buttons show icons only", async () => {
    const { host } = await mount();
    expect(options(host)).toHaveLength(3);
    for (const b of options(host)) expectIconOnly(b);
  });

  test("TablePanel defaults press header row only", async () => {
    const { host } = await mount();
    expect(options(host).map((b) => b.getAttribute("aria-pressed"))).toEqual(["true", "false", "false"]);
  });

  test("TablePanel striped toggle sends a set_table_option patch to the table", async () => {
    const { host, applied } = await mount();
    await act(async () => options(host)[2].click());
    expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "data", patch: { striped: true } });
  });

  test("TablePanel header row off toggle sends header-row true", async () => {
    const { host, applied } = await mount({ dataset: { source: [["A"], [1]] }, "header-row": false });
    await act(async () => options(host)[0].click());
    expect(applied[0].ops[0]).toMatchObject({ patch: { "header-row": true } });
  });

  test("TablePanel alignment select sends the align patch", async () => {
    const { host, applied } = await mount();
    const select = host.querySelectorAll(".table-align select")[1] as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(select, "center");
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "data", patch: { align: ["auto", "center"] } });
  });

  test("TablePanel add column sends set_data on the data param with a column name", async () => {
    const { host, applied } = await mount();
    const [, addColumn] = host.querySelectorAll(".cdg-actions button");
    await act(async () => (addColumn as HTMLButtonElement).click());
    expect(applied[0].ops[0]).toMatchObject({ op: "set_data", target, param: "data", change: { op: "insert_column", at: 2 } });
    const cells = (applied[0].ops[0].change as { cells: string[] }).cells;
    expect(cells[0]).not.toContain("Series");
  });

  test("TablePanel without data shows the no-data hint", async () => {
    const { host } = await mount({});
    expect(host.querySelector(".a2ui-hint")).not.toBeNull();
  });

  test("TablePanel show-as-chart button asks for the chart view", async () => {
    const views: string[] = [];
    const { host } = await mount(undefined, (view: string) => views.push(view));
    const button = host.querySelector(".show-as button") as HTMLButtonElement;
    expectIconOnly(button);
    await act(async () => button.click());
    expect(views).toEqual(["chart"]);
  });

  test("TablePanel show-as button closes the options toolbar row", async () => {
    const { host } = await mount();
    const toolbar = host.querySelector(".panel-toolbar") as HTMLElement;
    expect(toolbar.querySelector(".table-options")).not.toBeNull();
    expect(toolbar.lastElementChild?.classList.contains("show-as")).toBe(true);
  });

  test("le panneau d'un tableau de pack écrit dans son paramètre", async () => {
    const { host, applied } = await mount(undefined, undefined, undefined, { kind: "acme/grid", param: "grid" });
    await act(async () => options(host)[2].click());
    expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "grid", patch: { striped: true } });
  });
});
