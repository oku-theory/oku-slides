// Tableau d'une liste au dock, liste imbriquée (t-495) : une colonne `list` ouvre le tableau de
// détail des sous-items de sa ligne ; chacun de ses gestes part comme un changement `nested`.

import { describe, expect, test } from "bun:test";
import "../test/dom";

const { act } = await import("react");
const { createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { mountSurface, messagesOf } = await import("../test/surface");
const { ListTable } = await import("./ListTable");

type Change = { op: string } & Record<string, unknown>;

const TEXT = (key: string) => ({ key, label: key, kind: "text", options: [] });
const TACHES = {
  key: "taches",
  label: "Tâches",
  kind: "list",
  options: [],
  list: { columns: [TEXT("texte"), { key: "note", label: "Note", kind: "rich", options: [] }], min: 0, max: 3, item_label: "tâche" },
};

const ROWS = [
  { id: "a", titre: "Cadrer", taches: [{ id: "t1", texte: "Objectifs" }, { id: "t2", texte: "Équipe" }] },
  { id: "b", titre: "Livrer" },
];

async function mount(rows: unknown[] = ROWS, markdown: unknown = {}) {
  const root = {
    id: "root",
    component: "ListTable",
    label: "Étapes",
    rows: { path: "/f0" },
    change: { path: "/f0-change" },
    markdown: { path: "/f0-markdown" },
    columns: [TEXT("titre"), TACHES],
    min: 0,
    item_label: "étape",
    palette: [],
    action: { event: { name: "param", context: { field: 0, change: { path: "/f0-change" }, key: "etapes" } } },
  };
  const m = await mountSurface(messagesOf([root], { f0: rows, "f0-change": null, "f0-markdown": markdown }));
  const changes = () => m.actions.map((a) => a.context?.change as Change);
  return { ...m, changes };
}

const click = (el: Element | null) => act(async () => (el as HTMLElement).click());
const toggle = (host: HTMLElement, row: number) =>
  host.querySelectorAll(".lt-table")[0].querySelectorAll("tbody > tr")[row].querySelector("[data-list-sub]") as HTMLButtonElement;
const detail = (host: HTMLElement) => host.querySelector("[data-list-detail]") as HTMLElement | null;
const detailRows = (host: HTMLElement) => [...(detail(host)?.querySelectorAll("tbody tr") ?? [])];
const key = (el: Element, k: string, init: KeyboardEventInit = {}) =>
  act(async () => void el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...init })));

async function typeInto(el: HTMLInputElement, value: string) {
  await act(async () => {
    el.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("ListTable, liste imbriquée", () => {
  test("une colonne list montre le nombre de sous-items de sa ligne, sans tableau ouvert", async () => {
    const { host } = await mount();
    expect([toggle(host, 0).textContent, toggle(host, 1).textContent]).toEqual(["2", "0"]);
    expect(toggle(host, 0).getAttribute("aria-label")).toBe("Tâches");
    expect(toggle(host, 0).getAttribute("aria-expanded")).toBe("false");
    expect(detail(host)).toBeNull();
  });

  test("clic sur la colonne : tableau de détail des sous-items, refermé par un second clic", async () => {
    const { host } = await mount();
    await click(toggle(host, 0));
    expect(toggle(host, 0).getAttribute("aria-expanded")).toBe("true");
    const inputs = detailRows(host).map((r) => (r.querySelector('[data-col="texte"] input') as HTMLInputElement).value);
    expect(inputs).toEqual(["Objectifs", "Équipe"]);
    await click(toggle(host, 0));
    expect(detail(host)).toBeNull();
  });

  test("« Ajouter » du détail envoie un insert imbriqué, adressé ligne/colonne", async () => {
    const { host, changes } = await mount();
    await click(toggle(host, 1));
    await click(detail(host)?.querySelector("[data-list-add]") ?? null);
    const c = changes()[0] as { op: string; path: string; change: { op: string; id: string } };
    expect([c.op, c.path, c.change.op]).toEqual(["nested", "b/taches", "insert"]);
    expect(c.change.id.startsWith("i-")).toBe(true);
  });

  test("« Ajouter » du détail désactivé au max de la liste imbriquée", async () => {
    const full = [{ id: "a", taches: [{ id: "x" }, { id: "y" }, { id: "z" }] }];
    const { host } = await mount(full);
    await click(toggle(host, 0));
    expect((detail(host)?.querySelector("[data-list-add]") as HTMLButtonElement).disabled).toBe(true);
  });

  test("frappe, × et Alt+↓ dans le détail : set_field, remove et move imbriqués", async () => {
    const { host, changes } = await mount();
    await click(toggle(host, 0));
    const el = detailRows(host)[0].querySelector('[data-col="texte"] input') as HTMLInputElement;
    await typeInto(el, "Buts");
    await act(async () => el.blur());
    await click(detailRows(host)[1].querySelector("[data-list-remove]"));
    await key(detailRows(host)[0].querySelector("[data-list-handle]") as Element, "ArrowDown", { altKey: true });
    expect(changes()).toEqual([
      { op: "nested", path: "a/taches", change: { op: "set_field", item: "t1", field: "texte", value: "Buts" } },
      { op: "nested", path: "a/taches", change: { op: "remove", item: "t2" } },
      { op: "nested", path: "a/taches", change: { op: "move", item: "t1", to: 1 } },
    ]);
  });

  test("collage dans le détail : la colonne entière de la ligne, en un set_field", async () => {
    const { host, changes } = await mount();
    await click(toggle(host, 0));
    const el = detailRows(host)[0].querySelector('[data-col="texte"] input') as HTMLInputElement;
    await act(async () => {
      const e = new Event("paste", { bubbles: true, cancelable: true });
      Object.defineProperty(e, "clipboardData", { value: { getData: () => "Un\nDeux\n" } });
      el.dispatchEvent(e);
    });
    const c = changes()[0] as { op: string; item: string; field: string; value: { id: string; texte: string }[] };
    expect([c.op, c.item, c.field]).toEqual(["set_field", "a", "taches"]);
    expect(c.value.map((t) => [t.id, t.texte])).toEqual([["t1", "Un"], ["t2", "Deux"]]);
  });

  test("le détail lit le markdown de ses cellules riches sous ligne/colonne/sous-item", async () => {
    const rows = [{ id: "a", taches: [{ id: "t1", note: [{ runs: [{ t: "Gras", bold: true }] }] }] }];
    const { host } = await mount(rows, { "a/taches/t1": { note: "**Gras**" } });
    await click(toggle(host, 0));
    await click(detail(host)?.querySelector("[data-list-rich]") ?? null);
    expect(detail(host)?.querySelector(".lt-popover")?.textContent).toContain("**Gras**");
  });

  test("la ligne ouverte disparaît : le détail se ferme", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const props = (rows: typeof ROWS) => ({
      label: "Étapes", rows, columns: [TEXT("titre"), TACHES] as never, min: 0, max: null, itemLabel: null,
      palette: [], markdown: {}, onChange: async () => true,
    });
    await act(async () => root.render(createElement(ListTable, props(ROWS))));
    await click(toggle(host, 0));
    expect(detail(host)).not.toBeNull();
    await act(async () => root.render(createElement(ListTable, props([ROWS[1]]))));
    expect(detail(host)).toBeNull();
    await act(async () => root.unmount());
    host.remove();
  });
});
