// Rendu de la vue des données d'un graphique dans un DOM happy-dom.

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { resetMessages } from "../test/messages";
import type { QueryView } from "../DataSheet";
import type { DataEdit } from "./dataEdit";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ChartDataView } = await import("./ChartPanel");
const { t } = await import("../i18n");
const { CHART_KINDS } = await import("./chartDeck");
const { expectIconOnly } = await import("../test/iconOnly");
const { setMessages } = await import("../i18n");
const { readFileSync } = await import("node:fs");

afterAll(resetMessages);

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
  query = undefined as QueryView | undefined,
} = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const edits: DataEdit[] = [];
  const opened: string[] = [];
  await act(async () => {
    root.render(createElement(ChartDataView, {
      kind, source, rowIds: ["r1"], columnIds: ["c0", "c1", "c2"], checkboxColumns: [], stacked, series, palette: ["#0077b6", "#e76f51"],
      data: { id: "d1", name: "ventes", users: 1 }, choices: [{ id: "d1", name: "ventes" }], query,
      onEdit: (e: DataEdit) => edits.push(e), onOpen: (d: string) => opened.push(d),
    }));
  });
  return { host, edits, opened };
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

  test("ChartDataView adding a row sends an add_row edit", async () => {
    const { host, edits } = await mount();
    await click(host.querySelectorAll(".cdg-actions button")[0]);
    expect(edits[0]).toEqual({ edit: "add_row" });
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

  test("element data panel shows a query result read-only", async () => {
    const query: QueryView = { sql: "SELECT a FROM m", state: "ready", inputs: [{ id: "m", name: "ventes" }] };
    const { host, edits, opened } = await mount({ stacked: false, query });
    // The view's settings stay: data pick, kinds, stacking, colors.
    expect([".data-pick", ".chart-kinds", ".chart-stack", ".series-colors"].map((c) => !!host.querySelector(c))).toEqual([true, true, true, true]);
    await click(host.querySelector(".chart-stack button"));
    await click(host.querySelector(".series-chip"));
    await click(host.querySelectorAll(".swatches .swatch")[2]);
    expect(edits).toEqual([{ edit: "stack", stacked: true }, { edit: "color", index: 0, color: "#e76f51" }]);
    // Converting it reads its rows, which a query's are not: refused, so disabled.
    const kinds = [...host.querySelectorAll(".chart-kinds button, .show-as button")] as HTMLButtonElement[];
    expect(kinds.every((b) => b.disabled)).toBe(true);
    // Only the grid is read-only: the result under its SQL, no cell input, no add button.
    const cells = [...host.querySelectorAll(".data-query-result tbody td")].map((c) => c.textContent);
    expect(cells).toEqual(["1", "3", "4"]);
    expect([host.querySelectorAll(".cdg input").length, host.querySelector(".cdg-actions"), host.querySelector(".data-query-apply")]).toEqual([0, null, null]);
    const sql = host.querySelector(".data-query-sql") as HTMLTextAreaElement;
    expect([sql.value, sql.readOnly]).toEqual(["SELECT a FROM m", true]);
    expect(host.querySelector(".data-row-state")?.textContent).toBe(t("ui-data-state-ready"));
    await click(host.querySelector(".data-query-edit"));
    await click(host.querySelector(".data-query-input"));
    expect(opened).toEqual(["d1", "m"]);
    const english = t("ui-data-query-edit");
    setMessages("fr", [readFileSync(new URL("../../../crates/oku-slides-i18n/src/locales/fr/ui.ftl", import.meta.url), "utf8")]);
    const { host: fr } = await mount({ query });
    const edit = fr.querySelector(".data-query-edit")?.textContent;
    expect([edit, edit === english]).toEqual([t("ui-data-query-edit"), false]);
  });

  test("ChartDataView over a failed query shows its error and no grid", async () => {
    const query: QueryView = { sql: "SELECT z FROM m", state: "failed", error: "no column z", inputs: [] };
    const { host } = await mount({ kind: undefined, source: [], query });
    expect([host.querySelector(".data-query-error")?.textContent, host.querySelector(".cdg")]).toEqual(["no column z", null]);
  });
});
