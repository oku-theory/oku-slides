// Rendu des composants de données (`ChartData`, `TableData`) de la surface `host-data`.

import { describe, expect, test } from "bun:test";
import "../test/dom";

// Imports dynamiques après `../test/dom` : react-dom cherche le DOM à son chargement.
const { act } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");

const action = { event: { name: "data", context: { edit: { path: "/edit" } } } };
const chart = {
  id: "root", component: "ChartData", kind: "bar", source: [["M", "A"], ["x", 1]], checkbox_columns: [],
  stacked: false, series: [{ name: "A" }], palette: ["#0077b6"], edit: { path: "/edit" }, action,
};
const table = {
  id: "root", component: "TableData", source: [["A", "B"], [1, 2]], header_row: true, header_column: false,
  striped: false, align: ["auto", "end"], edit: { path: "/edit" }, action,
};

const mount = (root: Record<string, unknown>) => mountSurface(messagesOf([root], { edit: null }));
const click = (el: Element | null | undefined) => act(async () => (el as HTMLButtonElement).click());

describe("ChartData", () => {
  test("ChartData renders the chart's data grid", async () => {
    const { host } = await mount(chart);
    expect(host.querySelector(".chart-panel .chart-kinds")).not.toBeNull();
  });

  test("ChartData choosing a kind sends data with the kind edit", async () => {
    const { host, actions } = await mount(chart);
    await click(host.querySelectorAll(".chart-kinds button")[1]);
    expect(actions.map((a) => [a.name, a.context])).toEqual([["data", { edit: { edit: "kind", kind: "line" } }]]);
  });

  test("ChartData stacking sends data with the stack edit", async () => {
    const { host, actions } = await mount(chart);
    await click(host.querySelector(".chart-stack button"));
    expect(actions[0]?.context).toEqual({ edit: { edit: "stack", stacked: true } });
  });

  test("ChartData with an unknown prop is refused", async () => {
    const { errors } = await mount({ ...chart, explode: true });
    expect(errors.length).toBeGreaterThan(0);
  });
});

describe("TableData", () => {
  test("TableData shows its column alignment", async () => {
    const { host } = await mount(table);
    expect((host.querySelectorAll(".table-align select")[1] as HTMLSelectElement).value).toBe("end");
  });

  test("TableData toggling striped sends data with the flag edit", async () => {
    const { host, actions } = await mount(table);
    await click(host.querySelectorAll(".table-options button")[2]);
    expect(actions[0]?.context).toEqual({ edit: { edit: "flag", flag: "striped", on: true } });
  });
});
