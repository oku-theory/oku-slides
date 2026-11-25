// The data grid rendered in a happy-dom DOM.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ChartDataGrid } = await import("./ChartDataGrid");
const { NumberLangContext } = await import("../a2ui/numberLang");
const { locale } = await import("../i18n");
const { formatNumber } = await import("../table/numbers");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const SOURCE = [["Mois", "Ventes"], ["Jan", 12345.5], ["Fév", 1234.5]];
const ROW_IDS = ["r-jan", "r-fev"];
const COLUMN_IDS = ["c-mois", "c-ventes"];

async function mount(numberLang: string | null | undefined, onEdit = (_edit: unknown) => {}, source: unknown[][] = SOURCE) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    const grid = createElement(ChartDataGrid, { source, rowIds: ROW_IDS, columnIds: COLUMN_IDS, onEdit });
    // `undefined`: no provider; `null`: the provider of a deck without a language.
    root.render(numberLang === undefined ? grid : createElement(NumberLangContext.Provider, { value: numberLang }, grid));
  });
  return [...host.querySelectorAll("input.cdg-number")] as HTMLInputElement[];
}

describe("ChartDataGrid", () => {
  test("ChartDataGrid numbers follow the deck language fr", async () => {
    const inputs = await mount("fr");
    expect(inputs.map((i) => i.value)).toEqual(["12 345,5", "1234,5"]);
  });

  test("ChartDataGrid numbers follow the deck language en", async () => {
    const inputs = await mount("en");
    expect(inputs.map((i) => i.value)).toEqual(["12,345.5", "1234.5"]);
  });

  test("ChartDataGrid without a provider writes numbers in the UI language", async () => {
    const inputs = await mount(undefined);
    expect(inputs.map((i) => i.value)).toEqual([formatNumber(12345.5, locale()), formatNumber(1234.5, locale())]);
  });

  test("ChartDataGrid under a deck without a language writes numbers in the UI language", async () => {
    const inputs = await mount(null);
    expect(inputs.map((i) => i.value)).toEqual([formatNumber(12345.5, locale()), formatNumber(1234.5, locale())]);
  });

  test("ChartDataGrid with an invalid deck language label does not throw", async () => {
    const inputs = await mount("zz-!!");
    expect(inputs).toHaveLength(2);
    expect(inputs.map((i) => i.value)).toEqual(["12345.5", "1234.5"]);
  });

  test("ChartDataGrid shows negatives with the minus sign and the shortest decimals, like the slide", async () => {
    const source = [["Mois", "Ventes"], ["Jan", -12345.5], ["Fév", 0.1 + 0.2]];
    expect((await mount("fr", undefined, source)).map((i) => i.value)).toEqual([
      "\u221212\u202f345,5",
      "0,30000000000000004",
    ]);
  });

  test("ChartDataGrid in an unknown deck language writes the minus sign without groups", async () => {
    const source = [["Mois", "Ventes"], ["Jan", -12345.5]];
    expect((await mount("xx", undefined, source)).map((i) => i.value)).toEqual(["\u221212345.5"]);
  });

  test("ChartDataGrid blur on an unchanged grouped number sends nothing", async () => {
    const ops: unknown[] = [];
    const inputs = await mount("en", (op) => ops.push(op));
    await act(async () => {
      inputs[0].focus();
      inputs[0].blur();
    });
    expect(ops).toEqual([]);
  });

  test("ChartDataGrid edit keeping en grouping stays a number", async () => {
    const ops: Record<string, unknown>[] = [];
    const inputs = await mount("en", (op) => ops.push(op as Record<string, unknown>));
    await act(async () => {
      inputs[0].focus();
      inputs[0].value = "12,346.5";
      inputs[0].blur();
    });
    expect(ops[0]).toEqual({
      edit: "cells",
      change: { op: "set_cell", row: "r-jan", column: "c-ventes", value: 12346.5 },
    });
  });

  test("ChartDataGrid emits edit_data with row and column ids", async () => {
    const ops: unknown[] = [];
    await mount("en", (op) => ops.push(op));
    const host = mounted?.host as HTMLElement;
    const header = host.querySelector("thead input") as HTMLInputElement;
    await act(async () => {
      header.focus();
      header.value = "Month";
      header.blur();
    });
    const [removeColumn] = host.querySelectorAll("thead .cdg-remove");
    const [removeRow] = host.querySelectorAll("tbody .cdg-remove");
    const [addRow, addColumn] = host.querySelectorAll(".cdg-actions button");
    for (const b of [removeColumn, removeRow, addRow, addColumn]) await act(async () => (b as HTMLButtonElement).click());
    const cells = (change: unknown) => ({ edit: "cells", change });
    expect(ops).toEqual([
      cells({ op: "rename_column", column: "c-mois", name: "Month" }),
      cells({ op: "remove_column", column: "c-mois" }),
      cells({ op: "remove_row", row: "r-jan" }),
      // The pane may show only the first rows: a new row goes after the data's last one.
      { edit: "add_row" },
      cells({ op: "insert_column", after: "c-ventes", name: expect.any(String) }),
    ]);
  });

  test("ChartDataGrid paste emits one paste edit anchored by row and column ids", async () => {
    const edits: unknown[] = [];
    const inputs = await mount("en", (edit) => edits.push(edit));
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", { value: { getData: () => "1\t2\n3\t4" } });
    await act(async () => {
      inputs[1].dispatchEvent(paste);
    });
    expect(edits).toEqual([
      { edit: "paste", row: "r-fev", column: "c-ventes", cells: [[1, 2], [3, 4]], names: [expect.any(String)] },
    ]);
  });

  test("column header menu emits set-type", async () => {
    const types: unknown[] = [];
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    await act(async () => {
      root.render(createElement(ChartDataGrid, {
        source: SOURCE, rowIds: ROW_IDS, columnIds: COLUMN_IDS, onEdit: () => {},
        columnTypes: ["text", "number"], onType: (column: string, type: string) => types.push([column, type]),
      }));
    });
    const menus = [...host.querySelectorAll("select.cdg-type")] as HTMLSelectElement[];
    expect(menus.map((m) => m.value)).toEqual(["text", "number"]);
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(menus[0], "date");
      menus[0]?.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(types).toEqual([["c-mois", "date"]]);
  });

  test("ChartDataGrid without onType shows no type menu", async () => {
    await mount("en");
    expect(document.querySelector("select.cdg-type")).toBeNull();
  });
});
