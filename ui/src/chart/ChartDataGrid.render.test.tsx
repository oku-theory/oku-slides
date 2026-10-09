// Rendu de la grille des données dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ChartDataGrid } = await import("./ChartDataGrid");

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

async function mount(numberLang: string | undefined, onOp = (_op: unknown) => {}, source: unknown[][] = SOURCE) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(ChartDataGrid, { source, onOp, numberLang }));
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
    expect(ops[0]).toEqual({ op: "set_cell", row: 1, col: 1, value: 12346.5 });
  });
});
