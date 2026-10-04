// Tableau d'une liste au dock : chaque geste émet un seul changement de liste.

import { describe, expect, test } from "bun:test";
import "../test/dom";

// Voir catalog.render.test.tsx : React et la surface se chargent après le DOM.
const { act } = await import("react");
const { EditorView } = await import("@codemirror/view");
const { mountSurface, messagesOf } = await import("../test/surface");

type Change = { op: string } & Record<string, unknown>;

const TEXT = (key: string) => ({ key, label: key, kind: "text", options: [] });

/** Un `ListTable` lié à `/f0`, comme le construit la session ; son action envoie `change`. */
function table(o: { rows: unknown[]; min?: number; max?: number; columns?: unknown[]; markdown?: unknown }) {
  const root = {
    id: "root",
    component: "ListTable",
    label: "Étapes",
    rows: { path: "/f0" },
    change: { path: "/f0-change" },
    markdown: { path: "/f0-markdown" },
    columns: o.columns ?? [TEXT("date"), TEXT("texte")],
    min: o.min ?? 0,
    ...(o.max === undefined ? {} : { max: o.max }),
    item_label: "étape",
    palette: [],
    action: { event: { name: "param", context: { field: 0, change: { path: "/f0-change" }, key: "etapes" } } },
  };
  return messagesOf([root], { f0: o.rows, "f0-change": null, "f0-markdown": o.markdown ?? {} });
}

async function mount(messages: ReturnType<typeof table>) {
  const m = await mountSurface(messages);
  /** Les changements reçus par l'action, dans l'ordre. */
  const changes = () => m.actions.map((a) => a.context?.change as Change);
  return { ...m, changes };
}

const click = (el: Element) => act(async () => (el as HTMLElement).click());
const cell = (host: HTMLElement, row: number, col: string) =>
  host.querySelectorAll("tbody tr")[row].querySelector(`[data-col="${col}"]`) as HTMLElement;
const input = (host: HTMLElement, row: number, col: string) => cell(host, row, col).querySelector("input") as HTMLInputElement;

/** Tape `value` dans un champ comme le navigateur (setter du prototype, voir catalog.render.test.tsx). */
async function typeInto(el: HTMLInputElement, value: string) {
  await act(async () => {
    el.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
const blur = (el: HTMLElement) => act(async () => el.blur());
const key = (el: Element, k: string, init: KeyboardEventInit = {}) =>
  act(async () => void el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...init })));

async function paste(el: Element, text: string) {
  await act(async () => {
    const e = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => text } });
    el.dispatchEvent(e);
  });
}

describe("ListTable", () => {
  test("une ligne par item, une colonne par colonne du schéma", async () => {
    const { host } = await mount(table({ rows: [{ id: "d0", date: "2019", texte: "A" }, { id: "d1", date: "2021" }] }));
    expect(host.querySelectorAll("tbody tr").length).toBe(2);
    expect([...host.querySelectorAll("thead th")].map((th) => th.textContent)).toEqual(["", "date", "texte", ""]);
    expect([input(host, 0, "date").value, input(host, 1, "texte").value]).toEqual(["2019", ""]);
  });

  test("ListTable : « Ajouter » envoie un insert à la fin avec un id neuf", async () => {
    const { host, actions, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }], max: 3 }));
    await click(host.querySelector("[data-list-add]") as Element);
    const change = changes()[0] as { op: string; id: string; index?: number };
    expect(change.op).toBe("insert");
    expect(change.id.startsWith("i-")).toBe(true);
    expect(change.index).toBeUndefined();
    expect(actions.map((a) => a.name)).toEqual(["param"]);
    expect(actions[0].context?.key).toBe("etapes");
  });

  test("ListTable : « Ajouter » désactivé à max, « × » désactivé à min", async () => {
    const full = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }], min: 2, max: 2 }));
    const add = full.host.querySelector("[data-list-add]") as HTMLButtonElement;
    expect(add.disabled).toBe(true);
    expect(add.title).toBe("ui-list-full");
    const removes = [...full.host.querySelectorAll("[data-list-remove]")] as HTMLButtonElement[];
    expect(removes.map((b) => b.disabled)).toEqual([true, true]);
    await click(add);
    expect(full.actions).toHaveLength(0);
  });

  test("« × » au-dessus de min envoie le retrait de sa ligne", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }], min: 1 }));
    const removes = [...host.querySelectorAll("[data-list-remove]")] as HTMLButtonElement[];
    expect(removes.map((b) => b.disabled)).toEqual([false, false]);
    await click(removes[1]);
    expect(changes()).toEqual([{ op: "remove", item: "d1" }]);
  });

  test("ListTable : frappe dans une cellule texte → set_field au blur", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }] }));
    const el = input(host, 0, "date");
    await typeInto(el, " 2020 ");
    expect(changes()).toEqual([]);
    await blur(el);
    expect(changes()).toEqual([{ op: "set_field", item: "d0", field: "date", value: "2020" }]);
  });

  test("une cellule inchangée n'envoie rien ; Échap rétablit sans envoyer", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }] }));
    const el = input(host, 0, "date");
    await act(async () => el.focus());
    await blur(el);
    await typeInto(el, "autre");
    await key(el, "Escape");
    expect(el.value).toBe("2019");
    expect(changes()).toEqual([]);
  });

  test("nombre à virgule converti ; saisie qui n'est pas un nombre refusée et signalée", async () => {
    const columns = [{ key: "n", label: "n", kind: "number", options: [] }];
    const { host, changes } = await mount(table({ rows: [{ id: "d0", n: 1 }], columns }));
    const el = input(host, 0, "n");
    expect(el.inputMode).toBe("decimal");
    await typeInto(el, "abc");
    await blur(el);
    expect(changes()).toEqual([]);
    expect(cell(host, 0, "n").hasAttribute("data-invalid")).toBe(true);
    await typeInto(el, "3,5");
    await blur(el);
    expect(changes()).toEqual([{ op: "set_field", item: "d0", field: "n", value: 3.5 }]);
  });

  test("case et liste déroulante envoient leur valeur", async () => {
    const columns = [
      { key: "on", label: "on", kind: "flag", options: [] },
      { key: "ton", label: "ton", kind: "select", options: [{ id: "a", label: "A" }, { id: "b", label: "B" }] },
    ];
    const { host, changes } = await mount(table({ rows: [{ id: "d0", on: false, ton: "a" }], columns }));
    await click(input(host, 0, "on"));
    const select = cell(host, 0, "ton").querySelector("select") as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(select, "b");
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(changes()).toEqual([
      { op: "set_field", item: "d0", field: "on", value: true },
      { op: "set_field", item: "d0", field: "ton", value: "b" },
    ]);
  });

  test("Entrée dans la dernière ligne ajoute un item", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }, { id: "d1", date: "2021" }] }));
    await key(input(host, 0, "date"), "Enter");
    expect(changes()).toEqual([]);
    await key(input(host, 1, "date"), "Enter");
    expect(changes().map((c) => c.op)).toEqual(["insert"]);
  });

  test("ListTable : Alt+↓ sur une ligne → move à l'index suivant", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }, { id: "d2" }] }));
    const handles = host.querySelectorAll("[data-list-handle]");
    expect(handles[0].getAttribute("aria-label")).toBe("ui-list-move");
    await key(handles[0], "ArrowDown", { altKey: true });
    await key(handles[2], "ArrowUp", { altKey: true });
    await key(handles[0], "ArrowUp", { altKey: true });
    await key(handles[1], "ArrowDown");
    expect(changes()).toEqual([
      { op: "move", item: "d0", to: 1 },
      { op: "move", item: "d2", to: 1 },
    ]);
  });

  test("ListTable : collage TSV de deux lignes → un seul changement de liste entière", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019", texte: "A" }], max: 5 }));
    await paste(input(host, 0, "date"), "2020\tB\n2021\tC\n");
    const all = changes();
    expect(all).toHaveLength(1);
    const change = all[0] as { op: string; items: { id: string }[] };
    expect(change.op).toBe("replace");
    expect(change.items[0]).toEqual({ id: "d0", date: "2020", texte: "B" });
    expect(change.items[1].id.startsWith("i-")).toBe(true);
    expect(change.items[1]).toEqual({ id: change.items[1].id, date: "2021", texte: "C" });
  });

  test("le collage s'arrête à max et laisse une cellule invalide, signalée", async () => {
    const columns = [TEXT("date"), { key: "n", label: "n", kind: "number", options: [] }];
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019", n: 1 }], max: 2, columns }));
    await paste(input(host, 0, "date"), "2020\tabc\n2021\t2\n2022\t3");
    const change = changes()[0] as { op: string; items: Record<string, unknown>[] };
    expect(change.items.map((i) => [i.date, i.n])).toEqual([["2020", 1], ["2021", 2]]);
    expect(cell(host, 0, "n").hasAttribute("data-invalid")).toBe(true);
  });

  test("ListTable : cellule rich → popover avec RichTextField", async () => {
    const columns = [{ key: "texte", label: "Texte", kind: "rich", options: [] }];
    const rows = [{ id: "d0", texte: [{ runs: [{ t: "Un", bold: true }] }, { runs: [{ t: "Deux" }] }] }];
    const { host, changes } = await mount(table({ rows, columns, markdown: { d0: { texte: "**Un**\nDeux" } } }));
    const open = cell(host, 0, "texte").querySelector("[data-list-rich]") as HTMLButtonElement;
    expect(open.textContent).toBe("Un");
    expect(host.querySelector(".lt-popover")).toBeNull();
    await click(open);
    const content = host.querySelector(".lt-popover .rich-field .cm-content") as HTMLElement;
    const view = EditorView.findFromDOM(content)!;
    expect(view.state.doc.toString()).toBe("**Un**\nDeux");
    await act(async () => view.dispatch({ changes: { from: view.state.doc.length - 4, to: view.state.doc.length, insert: "Trois" } }));
    await act(async () => content.dispatchEvent(new FocusEvent("blur")));
    expect(changes()).toEqual([
      { op: "set_field", item: "d0", field: "texte", value: [rows[0].texte[0], { runs: [{ t: "Trois" }] }] },
    ]);
  });

  test("ListTable : colonne asset en lecture seule", async () => {
    const columns = [{ key: "logo", label: "Logo", kind: "asset", options: [] }];
    const id = "sha256:0123456789abcdef0123456789abcdef";
    const { host } = await mount(table({ rows: [{ id: "d0", logo: id }], columns }));
    const c = cell(host, 0, "logo");
    expect(c.querySelector("input, select, button, textarea")).toBeNull();
    expect(c.textContent?.startsWith("sha256:0123")).toBe(true);
    expect(c.querySelector("[title]")?.getAttribute("title")).toBe(id);
  });
});
