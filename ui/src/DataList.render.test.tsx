// Rendering of the deck's data list (`DataList`, the `deck-data` surface) and its routing.

import { describe, expect, test } from "bun:test";
import "./test/dom";

// Dynamic imports after `./test/dom`: react-dom looks for the DOM when it loads.
const { act } = await import("react");
const { mountSurface, messagesOf } = await import("./test/surface");
const { t } = await import("./i18n");
const { routeDataAction } = await import("./dataActions");

const action = { event: { name: "data", context: { edit: { path: "/edit" } } } };
const rows = [
  { id: "d5", name: "spare", kind: "manual", columns: 1, rows: 3, users: [] },
  {
    id: "d9", name: "ventes", label: "Ventes", kind: "manual", columns: 2, rows: 2,
    users: [{ slide: "s-libre", element: "ch1", label: "ch1 on slide 7" }],
  },
];
const list = { id: "root", component: "DataList", rows, edit: { path: "/edit" }, action };

const mount = (root: Record<string, unknown>) => mountSurface(messagesOf([root], { edit: null }));
const click = (el: Element | null | undefined) => act(async () => (el as HTMLButtonElement).click());
const edits = (actions: { context: Record<string, unknown> }[]) => actions.map((a) => a.context.edit);

/** Types `name` into the rename field of `d9` and presses Enter, which also blurs it. */
async function type(host: HTMLElement, name: string) {
  const field = host.querySelectorAll<HTMLInputElement>(".data-row-rename")[1] as HTMLInputElement;
  await act(async () => {
    field.focus();
    field.value = name;
    field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  });
}

describe("DataList", () => {
  test("DataList renders rows and opens DataPane on select", async () => {
    const { host, actions } = await mount(list);
    const names = [...host.querySelectorAll(".data-row-name")].map((n) => n.textContent);
    expect(names).toEqual(["spare", "Ventes"]);
    expect(host.querySelector(".data-row-unused")?.textContent).toBe(t("ui-data-unused"));
    expect(host.querySelectorAll(".data-row")[1]?.querySelector(".data-row-views")?.textContent).toBe(t("ui-data-views", { count: 1 }));
    await click(host.querySelectorAll(".data-row-name")[1]);
    expect(edits(actions)).toEqual([{ action: "select", data: "d9" }]);
    expect(routeDataAction({ action: "select", data: "d9" })).toEqual({ to: "sheet", data: "d9", offset: 0 });
  });

  test("rename field submits rename", async () => {
    const { host, actions } = await mount(list);
    const field = host.querySelectorAll<HTMLInputElement>(".data-row-rename")[1] as HTMLInputElement;
    await act(async () => {
      field.focus();
      field.value = "sales";
      field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    });
    expect(edits(actions)).toEqual([{ action: "rename", data: "d9", name: "sales" }]);
  });

  test("DataList rename back after an undo is sent again", async () => {
    const { host, actions, extend } = await mount(list);
    const named = (name: string) => rows.map((r) => (r.id === "d9" ? { ...r, name } : r));
    const update = (name: string) =>
      extend([{ version: "v0.9", updateComponents: { surfaceId: "s", components: [{ ...list, rows: named(name) }] } }] as never);
    await type(host, "sales");
    await update("sales");
    await update("ventes");
    await type(host, "sales");
    const rename = { action: "rename", data: "d9", name: "sales" };
    expect(edits(actions)).toEqual([rename, rename]);
  });

  test("DataList refused name is sent again on retry", async () => {
    const { host, actions } = await mount(list);
    await type(host, "sales");
    await type(host, "sales");
    const rename = { action: "rename", data: "d9", name: "sales" };
    expect(edits(actions)).toEqual([rename, rename]);
  });

  test("DataList add and remove send their actions", async () => {
    const { host, actions } = await mount(list);
    await click(host.querySelector(".data-list-add"));
    await click(host.querySelectorAll(".data-row-remove")[0]);
    expect(edits(actions)).toEqual([{ action: "add" }, { action: "remove", data: "d5" }]);
  });

  test("DataList user link emits open-element", async () => {
    const { host, actions } = await mount(list);
    await click(host.querySelector(".data-row-user"));
    expect(edits(actions)).toEqual([{ action: "open-element", slide: "s-libre", element: "ch1" }]);
  });

  test("DataList shows a row's refusal inline", async () => {
    const refused = rows.map((r) => (r.id === "d9" ? { ...r, error: "in use" } : r));
    const { host } = await mount({ ...list, rows: refused });
    expect(host.querySelector(".data-row-error")?.textContent).toBe("in use");
  });

  test("DataList with duplicate row ids is refused", async () => {
    const { errors } = await mount({ ...list, rows: [...rows, rows[0]] });
    expect(errors.length).toBeGreaterThan(0);
  });
});

describe("routeDataAction", () => {
  test("routeDataAction open-data opens the sheet at its first page", () => {
    expect(routeDataAction({ action: "open-data", data: "d1" })).toEqual({ to: "sheet", data: "d1", offset: 0 });
  });

  test("routeDataAction page keeps the data and moves the offset", () => {
    expect(routeDataAction({ action: "page", data: "d1", offset: 100 })).toEqual({ to: "sheet", data: "d1", offset: 100 });
  });

  test("routeDataAction open-element selects the element", () => {
    expect(routeDataAction({ action: "open-element", slide: "s", element: "e" })).toEqual({ to: "element", slide: "s", element: "e" });
  });

  test("routeDataAction session actions go to the session", () => {
    const set = { action: "set-type", data: "d1", column: "c1", type: "number" } as const;
    expect(routeDataAction(set)).toEqual({ to: "session", action: set });
  });
});
