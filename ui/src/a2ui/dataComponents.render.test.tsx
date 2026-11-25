// Rendering of the data components (`ChartData`, `TableData`) of the `host-data` surface.

import { describe, expect, test } from "bun:test";
import "../test/dom";

// Dynamic imports after `../test/dom`: react-dom looks for the DOM when it loads.
const { act } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { t } = await import("../i18n");

const action = { event: { name: "data", context: { edit: { path: "/edit" } } } };
const data = { id: "d1", name: "ventes", users: 1 };
const choices = [{ id: "d1", name: "ventes" }, { id: "d2", name: "budget", label: "Budget 2026" }];
const chart = {
  id: "root", component: "ChartData", kind: "bar", source: [["M", "A"], ["x", 1]], row_ids: ["rx"], column_ids: ["m", "a"], checkbox_columns: [],
  stacked: false, series: [{ name: "A" }], palette: ["#0077b6"], data, choices, edit: { path: "/edit" }, action,
};
const table = {
  id: "root", component: "TableData", source: [["A", "B"], [1, 2]], row_ids: ["r1"], column_ids: ["a", "b"], header_row: true, header_column: false,
  striped: false, align: ["auto", "end"], data, choices, edit: { path: "/edit" }, action,
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

  test("TableData shows shared-by when users > 1", async () => {
    const { host } = await mount({ ...table, data: { ...data, users: 3 } });
    expect(host.querySelector(".data-shared-by")?.textContent).toBe(t("ui-data-shared-by", { count: 3 }));
  });

  test("TableData hides shared-by when users == 1", async () => {
    const { host } = await mount(table);
    expect(host.querySelector(".data-shared-by")).toBeNull();
  });

  test("TableData with a negative users count is refused", async () => {
    const { errors } = await mount({ ...table, data: { ...data, users: -1 } });
    expect(errors.length).toBeGreaterThan(0);
  });
});

describe("data picker", () => {
  test("picker shows the data's label, else its name", async () => {
    const { host } = await mount(chart);
    const options = [...host.querySelectorAll(".data-pick select option")].map((o) => o.textContent);
    expect(options).toEqual(["ventes", "Budget 2026"]);
  });

  test("picker emits bind edit", async () => {
    const { host, actions } = await mount(table);
    const select = host.querySelector(".data-pick select") as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(select, "d2");
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(actions.map((a) => a.context)).toEqual([{ edit: { edit: "bind", data: "d2" } }]);
  });

  test("data name emits open-data", async () => {
    const { host, actions } = await mount(chart);
    await click(host.querySelector(".data-pick .data-open"));
    expect(actions.map((a) => a.context)).toEqual([{ edit: { action: "open-data", data: "d1" } }]);
  });

  test("picker with duplicate choice ids is refused", async () => {
    const { errors } = await mount({ ...table, choices: [...choices, { id: "d2", name: "copie" }] });
    expect(errors.length).toBeGreaterThan(0);
  });
});
