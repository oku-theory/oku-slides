// A page of one data's rows in the data pane (`DataSheetView`): paging and the panel actions
// its grid sends.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DataSheetView } = await import("./DataSheet");
const { NumberLangContext } = await import("./a2ui/numberLang");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const SOURCE = [["Mois", "ventes"], ["Jan", 7], ["Fév", 41]];

/** Mounts a page of `d9` at `offset`, out of `total` rows; returns its host and the actions sent. */
async function mount(offset = 0, total = 2) {
  const actions: unknown[] = [];
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const sheet = createElement(DataSheetView, {
    data: { id: "d9", name: "ventes" }, source: SOURCE, rowIds: ["r1", "r2"], columnIds: ["c1", "c2"],
    types: ["text", "number"], offset, total, onAction: (a: unknown) => actions.push(a),
  });
  await act(async () => root.render(createElement(NumberLangContext.Provider, { value: "en" }, sheet)));
  return { host, actions };
}

const button = (host: HTMLElement, cls: string) => host.querySelector(`.${cls}`) as HTMLButtonElement;
const click = (b: HTMLButtonElement) => act(async () => b.click());

describe("DataSheetView", () => {
  test("DataSheetView single page disables previous and next", async () => {
    const { host } = await mount(0, 2);
    expect([button(host, "data-sheet-previous").disabled, button(host, "data-sheet-next").disabled]).toEqual([true, true]);
  });

  test("DataSheetView middle page sends both neighbours", async () => {
    const { host, actions } = await mount(100, 250);
    await click(button(host, "data-sheet-previous"));
    await click(button(host, "data-sheet-next"));
    expect(actions).toEqual([
      { action: "page", data: "d9", offset: 0 },
      { action: "page", data: "d9", offset: 200 },
    ]);
  });

  test("DataSheetView last page disables next", async () => {
    const { host } = await mount(200, 202);
    expect([button(host, "data-sheet-previous").disabled, button(host, "data-sheet-next").disabled]).toEqual([false, true]);
  });

  test("DataSheetView grid edits become cells add-row and set-type actions", async () => {
    const { host, actions } = await mount();
    const cell = host.querySelectorAll<HTMLInputElement>("tbody input")[1] as HTMLInputElement;
    await act(async () => {
      cell.focus();
      cell.value = "8";
      cell.blur();
    });
    const [addRow] = host.querySelectorAll(".cdg-actions button");
    await click(addRow as HTMLButtonElement);
    const menu = host.querySelector("select.cdg-type") as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(menu, "date");
      menu.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(actions).toEqual([
      { action: "cells", data: "d9", change: { op: "set_cell", row: "r1", column: "c2", value: 8 } },
      { action: "add-row", data: "d9" },
      { action: "set-type", data: "d9", column: "c1", type: "date" },
    ]);
  });

  test("DataSheetView paste spreads a block across cells typed by column", async () => {
    const { host, actions } = await mount();
    const first = host.querySelector<HTMLInputElement>("tbody input") as HTMLInputElement;
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", { value: { getData: () => "007\t5\nAvr\t6\t9" } });
    await act(async () => {
      first.dispatchEvent(paste);
    });
    expect(actions).toEqual([{
      action: "paste", data: "d9", row: "r1", column: "c1",
      cells: [["007", 5], ["Avr", 6, 9]], names: [expect.any(String)],
    }]);
  });
});
