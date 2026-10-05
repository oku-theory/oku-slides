// Tableau d'une liste au dock : chaque geste émet un seul changement de liste.

import { describe, expect, test } from "bun:test";
import "../test/dom";

// Voir catalog.render.test.tsx : React et la surface se chargent après le DOM.
const { act } = await import("react");
const { EditorView } = await import("@codemirror/view");
const { createElement } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { ActionOutcomeContext } = await import("./actionOutcome");
const { deferred } = await import("../test/widgetApi");
const { ListTable, popoverPlace } = await import("./ListTable");
const { createRoot } = await import("react-dom/client");
const { expectIconOnly } = await import("../test/iconOnly");

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

/**
 * Monte le tableau ; `outcome` rend l'issue de chaque action, comme les réglages de l'hôte
 * (sans lui, aucun fournisseur : toute action est acceptée).
 */
async function mount(messages: ReturnType<typeof table>, outcome?: () => Promise<boolean>) {
  const box = { current: null as Promise<boolean> | null };
  const m = outcome
    ? await mountSurface(
        messages,
        (s) => createElement(ActionOutcomeContext.Provider, { value: box }, s),
        () => (box.current = outcome()),
      )
    : await mountSurface(messages);
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

/** Pointeur sur `el` ; `over` est la cellule que `elementFromPoint` rend (`null` : hors du tableau). */
async function pointer(el: Element, type: string, over: Element | null) {
  const original = document.elementFromPoint;
  document.elementFromPoint = () => over;
  try {
    await act(async () => void el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1 })));
  } finally {
    document.elementFromPoint = original;
  }
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

  test("ListTable : « Ajouter » est un pictogramme nommé par son libellé", async () => {
    const { host } = await mount(table({ rows: [{ id: "d0" }], max: 3 }));
    expectIconOnly(host.querySelector("[data-list-add]"), "ui-list-add");
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

  test("ListTable : saisie refusée par la session → la cellule revient à la valeur des lignes", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }] }), async () => false);
    const el = input(host, 0, "date");
    await typeInto(el, "2020");
    await blur(el);
    expect(changes()).toEqual([{ op: "set_field", item: "d0", field: "date", value: "2020" }]);
    expect(input(host, 0, "date").value).toBe("2019");
  });

  test("ListTable : Entrée sur la dernière ligne attend l'enregistrement de la cellule avant d'insérer", async () => {
    const saved = deferred<boolean>();
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }] }), () => saved.promise);
    const el = input(host, 0, "date");
    await typeInto(el, "2020");
    await key(el, "Enter");
    expect(changes().map((c) => c.op)).toEqual(["set_field"]);
    await act(async () => saved.resolve(true));
    expect(changes().map((c) => c.op)).toEqual(["set_field", "insert"]);
  });

  test("ListTable : Entrée sur la dernière ligne n'insère rien si l'enregistrement est refusé", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0", date: "2019" }] }), async () => false);
    const el = input(host, 0, "date");
    await typeInto(el, "2020");
    await key(el, "Enter");
    expect(changes().map((c) => c.op)).toEqual(["set_field"]);
  });

  test("ListTable : Entrée sur la dernière ligne n'insère rien si la saisie est invalide", async () => {
    const columns = [{ key: "n", label: "n", kind: "number", options: [] }];
    const { host, changes } = await mount(table({ rows: [{ id: "d0", n: 1 }], columns }));
    const el = input(host, 0, "n");
    await typeInto(el, "abc");
    await key(el, "Enter");
    expect(changes()).toEqual([]);
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

  test("ListTable : glisser une poignée → fantôme de la ligne, source estompée, repère sur la cible, puis move", async () => {
    const rows = [{ id: "d0", date: "2018", texte: "Fondation" }, { id: "d1" }, { id: "d2" }];
    const { host, changes } = await mount(table({ rows }));
    const handle = host.querySelectorAll("[data-list-handle]")[0];
    const target = cell(host, 2, "date");
    const original = document.elementFromPoint;
    document.elementFromPoint = () => target;
    const pointer = (el: Element, type: string) =>
      act(async () => void el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: 10, clientY: 90 })));
    try {
      await pointer(handle, "pointerdown");
      await pointer(handle, "pointermove");
      const trs = host.querySelectorAll("tbody tr");
      expect(host.querySelector(".lt-ghost")?.textContent).toContain("Fondation");
      expect(trs[0].hasAttribute("data-lifted")).toBe(true);
      expect(trs[2].getAttribute("data-drop")).toBe("after");
      await pointer(handle, "pointerup");
    } finally {
      document.elementFromPoint = original;
    }
    expect(changes()).toEqual([{ op: "move", item: "d0", to: 2 }]);
    expect(host.querySelector(".lt-ghost")).toBeNull();
    expect(host.querySelector("[data-lifted], [data-drop]")).toBeNull();
  });

  test("ListTable : glisser sans rendu entre les événements → move quand même", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }, { id: "d2" }] }));
    const handle = host.querySelectorAll("[data-list-handle]")[2];
    const target = cell(host, 0, "date");
    const original = document.elementFromPoint;
    document.elementFromPoint = () => target;
    const fire = (type: string) => handle.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1 }));
    try {
      // Un geste vif : React n'a pas rendu entre l'appui, le déplacement et le relâcher.
      await act(async () => ["pointerdown", "pointermove", "pointerup"].forEach(fire));
    } finally {
      document.elementFromPoint = original;
    }
    expect(changes()).toEqual([{ op: "move", item: "d2", to: 0 }]);
  });

  test("ListTable : glisser vers le haut → repère « avant » sur la cible", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }, { id: "d2" }] }));
    const handle = host.querySelectorAll("[data-list-handle]")[2];
    await pointer(handle, "pointerdown", cell(host, 2, "date"));
    await pointer(handle, "pointermove", cell(host, 0, "date"));
    expect(host.querySelectorAll("tbody tr")[0].getAttribute("data-drop")).toBe("before");
    await pointer(handle, "pointerup", cell(host, 0, "date"));
    expect(changes()).toEqual([{ op: "move", item: "d2", to: 0 }]);
  });

  test("ListTable : pointercancel → plus de fantôme, le relâcher ne déplace rien", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }] }));
    const handle = host.querySelectorAll("[data-list-handle]")[0];
    await pointer(handle, "pointerdown", cell(host, 0, "date"));
    await pointer(handle, "pointermove", cell(host, 1, "date"));
    await pointer(handle, "pointercancel", null);
    expect(host.querySelector(".lt-ghost, [data-lifted], [data-drop]")).toBeNull();
    await pointer(handle, "pointerup", cell(host, 1, "date"));
    expect(changes()).toEqual([]);
  });

  test("ListTable : relâcher hors du tableau → aucun move, plus de fantôme", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }] }));
    const handle = host.querySelectorAll("[data-list-handle]")[0];
    await pointer(handle, "pointerdown", cell(host, 0, "date"));
    await pointer(handle, "pointermove", null);
    await pointer(handle, "pointerup", null);
    expect(changes()).toEqual([]);
    expect(host.querySelector(".lt-ghost, [data-lifted]")).toBeNull();
  });

  test("ListTable : clic sur la poignée sans bouger → ni fantôme ni move", async () => {
    const { host, changes } = await mount(table({ rows: [{ id: "d0" }, { id: "d1" }] }));
    const handle = host.querySelectorAll("[data-list-handle]")[0];
    await pointer(handle, "pointerdown", cell(host, 0, "date"));
    expect(host.querySelector(".lt-ghost, [data-lifted]")).toBeNull();
    await pointer(handle, "pointerup", cell(host, 0, "date"));
    expect(changes()).toEqual([]);
  });

  test("ListTable : la ligne tirée disparaît pendant le geste → aucun move", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const sent: unknown[] = [];
    const props = (rows: { id: string }[]) => ({
      label: "Étapes", rows, columns: [TEXT("date")] as never, min: 0, max: null, itemLabel: null, palette: [], markdown: {},
      onChange: async (c: unknown) => (sent.push(c), true),
    });
    await act(async () => root.render(createElement(ListTable, props([{ id: "d0" }, { id: "d1" }, { id: "d2" }]))));
    const at = (r: number) => host.querySelectorAll("tbody tr")[r].querySelector('[data-col="date"]') as Element;
    const handle = host.querySelectorAll("[data-list-handle]")[0];
    await pointer(handle, "pointerdown", at(0));
    await pointer(handle, "pointermove", at(2));
    await act(async () => root.render(createElement(ListTable, props([{ id: "d1" }, { id: "d2" }]))));
    expect(host.querySelector(".lt-ghost")).toBeNull();
    // La poignée tirée est partie avec sa ligne : le relâcher arrive à une poignée restante.
    await pointer(host.querySelectorAll("[data-list-handle]")[0], "pointerup", at(1));
    expect(sent).toEqual([]);
    await act(async () => root.unmount());
    host.remove();
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
    // Ancré à sa cellule : il porte son adresse et se place sous elle.
    expect(host.querySelector(".lt-popover")?.getAttribute("data-anchor")).toBe("d0/texte");
    const content = host.querySelector(".lt-popover .rich-field .cm-content") as HTMLElement;
    const view = EditorView.findFromDOM(content)!;
    expect(view.state.doc.toString()).toBe("**Un**\nDeux");
    await act(async () => view.dispatch({ changes: { from: view.state.doc.length - 4, to: view.state.doc.length, insert: "Trois" } }));
    await act(async () => content.dispatchEvent(new FocusEvent("blur")));
    expect(changes()).toEqual([
      { op: "set_field", item: "d0", field: "texte", value: [rows[0].texte[0], { runs: [{ t: "Trois" }] }] },
    ]);
  });

  test("popoverPlace : sous la cellule, ramené à gauche plutôt que de déborder du tableau", () => {
    const box = { left: 100, top: 50, width: 300 } as DOMRect;
    expect(popoverPlace(box, { left: 150, bottom: 90 } as DOMRect, 200)).toEqual({ left: 50, top: 40 });
    expect(popoverPlace(box, { left: 350, bottom: 120 } as DOMRect, 200)).toEqual({ left: 100, top: 70 });
    expect(popoverPlace(box, { left: 100, bottom: 90 } as DOMRect, 400)).toEqual({ left: 0, top: 40 });
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
