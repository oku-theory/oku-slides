// A page of one data's rows in the data pane (`DataSheetView`): paging and the panel actions
// its grid sends; a query's SQL editor, read-only result, error and inputs.

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

/** A query's view: its SQL, state, error and inputs. */
interface Query {
  sql: string;
  state: "pending" | "ready" | "failed";
  error?: string;
  inputs: { id: string; name: string }[];
}

/** Mounts a page of `d9` at `offset`, out of `total` rows; returns its host and the actions sent. */
async function mount(offset = 0, total = 2, more: { query?: Query; error?: string; source?: unknown[][] } = {}) {
  const actions: unknown[] = [];
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const sheet = createElement(DataSheetView, {
    data: { id: "d9", name: "ventes" }, source: (more.source ?? SOURCE) as never, rowIds: ["r1", "r2"],
    columnIds: ["c1", "c2"], types: ["text", "number"], offset, total, query: more.query, error: more.error,
    onAction: (a: unknown) => actions.push(a),
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

  test("DataSheet shows SQL, result and error for a query", async () => {
    const ready: Query = { sql: "SELECT a FROM ventes", state: "ready", inputs: [{ id: "m", name: "ventes" }] };
    const { host } = await mount(0, 2, { query: ready, error: "refused" });
    expect((host.querySelector(".data-query-sql") as HTMLTextAreaElement).value).toBe("SELECT a FROM ventes");
    const cells = [...host.querySelectorAll(".data-query-result tbody td")].map((c) => c.textContent);
    expect(cells).toEqual(["Jan", "7", "Fév", "41"]);
    expect(host.querySelectorAll(".data-query-result input, .data-query-result select").length).toBe(0);
    expect(host.querySelector(".data-sheet-refusal")?.textContent).toBe("refused");
    expect(host.querySelector(".data-query-error")).toBeNull();
    await act(async () => mounted?.root.unmount());
    mounted?.host.remove();
    mounted = null;
    const failed: Query = { sql: "SELECT zzz FROM ventes", state: "failed", error: "no zzz", inputs: [] };
    const { host: other } = await mount(0, 0, { query: failed, source: [[]] });
    expect(other.querySelector(".data-query-error")?.textContent).toBe("no zzz");
    expect(other.querySelectorAll(".data-query-result tbody tr").length).toBe(0);
  });

  test("SQL submit emits set-query", async () => {
    const query: Query = { sql: "SELECT 1 AS a", state: "ready", inputs: [{ id: "m", name: "ventes" }] };
    const { host, actions } = await mount(0, 2, { query });
    const sql = host.querySelector(".data-query-sql") as HTMLTextAreaElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(sql, "SELECT a FROM ventes");
      sql.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await click(button(host, "data-query-apply"));
    await click(host.querySelector(".data-query-input") as HTMLButtonElement);
    expect(actions).toEqual([
      { action: "set-query", data: "d9", sql: "SELECT a FROM ventes" },
      { action: "open-data", data: "m" },
    ]);
  });
});
