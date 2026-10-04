// Poignées des items de liste sur la scène : « + » entre les items, « × » et
// ⋮⋮ sur chaque item, « + » au centre d'une liste vide.

import { afterAll, afterEach, beforeAll, expect, test } from "bun:test";
import "./test/dom";
import type { ItemView, ListView, Op, SlideView } from "./api";
import { setMessages } from "./i18n";
import { resetMessages } from "./test/messages";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ItemLayer } = await import("./ItemLayer");

const ftl = (lang: string) =>
  Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();

beforeAll(async () => {
  setMessages("fr", [await ftl("en"), await ftl("fr")]);
});
// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(resetMessages);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const item = (id: string, index: number, x: number, more: Partial<ItemView> = {}): ItemView => ({
  owner: "f", owner_kind: "element", list: "etapes", item: id, index, len: 2, min: 0, max: 8,
  item_label: "étape", first_text: "date", x, y: 100, w: 100, h: 50, ...more,
});

const viewOf = (items: ItemView[], lists: ListView[] = []) =>
  ({
    slide: { id: "s", kind: "content", params: {} },
    page: 1, hash: "h", width: 1000, height: 600, zone: null, fields: [],
    targets: [{ id: "f", kind: "frise", free: false, x: 0, y: 100, w: 400, h: 200 }],
    items, lists,
  }) as unknown as SlideView;

const two = (more: Partial<ItemView> = {}) => viewOf([item("a", 0, 0, more), item("b", 1, 120, more)]);

async function mount(view: SlideView, active: string | null = "f", result?: Promise<boolean>) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: { apply: [string, Op[]][]; inserted: string[] } = { apply: [], inserted: [] };
  await act(async () =>
    root.render(
      createElement(ItemLayer, {
        view, scale: 1, active,
        onApply: (label: string, ops: Op[]) => {
          calls.apply.push([label, ops]);
          return result;
        },
        onInserted: (key: string) => calls.inserted.push(key),
      }),
    ),
  );
  return { host, calls };
}

const click = (el: Element) =>
  act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });

test("ItemLayer two items of the active element offer three inserts, before, between and after", async () => {
  const { host } = await mount(two());
  const plus = [...host.querySelectorAll<HTMLElement>("[data-item-insert]")];
  expect(plus.length).toBe(3);
  expect(plus.map((p) => p.getAttribute("aria-label"))).toEqual(["Ajouter étape", "Ajouter étape", "Ajouter étape"]);
  // Entre les deux items (100 → 120) : au milieu, à mi-hauteur.
  expect(plus[1].style.left).toBe("110px");
  expect(plus[1].style.top).toBe("125px");
});

test("ItemLayer clicking an insert inserts an item at that index and asks to edit its first text", async () => {
  const { host, calls } = await mount(two());
  await click(host.querySelectorAll("[data-item-insert]")[1]);
  expect(calls.apply.length).toBe(1);
  const [label, ops] = calls.apply[0];
  expect(label).toBe("Ajouter étape");
  const change = (ops[0] as { change: { id: string } }).change;
  expect(ops).toEqual([{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "insert", index: 1, id: change.id } }]);
  expect(change.id).toMatch(/^i-/);
  expect(calls.inserted).toEqual([`f\u001fetapes/${change.id}/date`]);
});

test("ItemLayer a list without text column inserts without asking to edit", async () => {
  const { host, calls } = await mount(two({ first_text: null }));
  await click(host.querySelector("[data-item-insert]")!);
  expect(calls.apply.length).toBe(1);
  expect(calls.inserted).toEqual([]);
});

test("ItemLayer a full list offers no insert", async () => {
  const { host } = await mount(two({ max: 2 }));
  expect(host.querySelector("[data-item-insert]")).toBeNull();
  expect(host.querySelectorAll("[data-item-remove]").length).toBe(2);
});

test("ItemLayer a list at its minimum offers no remove", async () => {
  const { host } = await mount(two({ min: 2 }));
  expect(host.querySelector("[data-item-remove]")).toBeNull();
  expect(host.querySelectorAll("[data-item-insert]").length).toBe(3);
});

test("ItemLayer clicking a remove removes that item", async () => {
  const { host, calls } = await mount(two());
  const remove = host.querySelectorAll("[data-item-remove]")[1];
  expect(remove.getAttribute("aria-label")).toBe("Supprimer étape");
  await click(remove);
  expect(calls.apply).toEqual([
    ["Supprimer étape", [{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "remove", item: "b" } }]],
  ]);
});

test("ItemLayer shows nothing for an element neither selected nor edited", async () => {
  const { host } = await mount(two(), "other");
  expect(host.querySelector("button")).toBeNull();
  const none = await mount(two(), null);
  expect(none.host.querySelector("button")).toBeNull();
});

test("ItemLayer an item without label is named by the generic word", async () => {
  const { host } = await mount(two({ item_label: null }));
  expect(host.querySelector("[data-item-insert]")?.getAttribute("aria-label")).toBe("Ajouter élément");
});

test("ItemLayer a slide list targets the slide", async () => {
  const { host, calls } = await mount(two({ owner: "s", owner_kind: "slide" }), "s");
  await click(host.querySelector("[data-item-remove]")!);
  expect((calls.apply[0][1][0] as { target: unknown }).target).toEqual({ slide: "s" });
});

/** Glisse la poignée ⋮⋮ de `el` jusqu'à `x` (pt, à l'échelle 1). */
async function dragTo(el: HTMLElement, x: number) {
  el.setPointerCapture = () => {};
  const fire = (type: string, clientX: number) =>
    act(async () => {
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX, clientY: 120 }));
    });
  await fire("pointerdown", 50);
  await fire("pointermove", x);
  await fire("pointerup", x);
}

const three = () => viewOf([item("a", 0, 0, { len: 3 }), item("b", 1, 120, { len: 3 }), item("c", 2, 240, { len: 3 })]);

test("ItemLayer dragging an item past the next one moves it after it", async () => {
  const { host, calls } = await mount(three());
  await dragTo(host.querySelectorAll<HTMLElement>("[data-item-move]")[0], 200);
  expect(calls.apply).toEqual([
    ["Déplacer étape", [{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "move", item: "a", to: 1 } }]],
  ]);
});

test("ItemLayer dragging an item past the last one moves it to the end", async () => {
  const { host, calls } = await mount(three());
  await dragTo(host.querySelectorAll<HTMLElement>("[data-item-move]")[0], 400);
  expect((calls.apply[0][1][0] as { change: unknown }).change).toEqual({ op: "move", item: "a", to: 2 });
});

test("ItemLayer dropping an item where it was moves nothing", async () => {
  const { host, calls } = await mount(three());
  await dragTo(host.querySelectorAll<HTMLElement>("[data-item-move]")[1], 160);
  expect(calls.apply).toEqual([]);
});

const emptyList = (more: Partial<ListView> = {}): ListView => ({
  owner: "f", owner_kind: "element", list: "etapes", len: 0, min: 0, max: 8, item_label: "étape", first_text: "date", ...more,
});

test("ItemLayer an empty list of the active element offers one insert at the centre of the element", async () => {
  const { host, calls } = await mount(viewOf([], [emptyList()]));
  const plus = [...host.querySelectorAll<HTMLElement>("[data-item-insert]")];
  expect(plus.length).toBe(1);
  expect(plus[0].style.left).toBe("200px");
  expect(plus[0].style.top).toBe("200px");
  await click(plus[0]);
  const change = (calls.apply[0][1][0] as { change: { id: string } }).change;
  expect(calls.apply[0][1]).toEqual([{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes", change: { op: "insert", index: 0, id: change.id } }]);
  expect(calls.inserted).toEqual([`f\u001fetapes/${change.id}/date`]);
});

test("ItemLayer an empty list that allows no item offers no insert", async () => {
  const { host } = await mount(viewOf([], [emptyList({ max: 0 })]));
  expect(host.querySelector("[data-item-insert]")).toBeNull();
});

test("ItemLayer a list with items draws no centre insert", async () => {
  const { host } = await mount(viewOf([item("a", 0, 0, { len: 1 })], [emptyList({ len: 1 })]));
  expect(host.querySelectorAll("[data-item-insert]").length).toBe(2);
});

test("ItemLayer arrow keys along the list on a move handle move the item by one", async () => {
  const { host, calls } = await mount(three());
  const handle = host.querySelectorAll<HTMLElement>("[data-item-move]")[1];
  await act(async () => {
    handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true, cancelable: true }));
  });
  await act(async () => {
    handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
  });
  expect(calls.apply.map(([, ops]) => (ops[0] as { change: unknown }).change)).toEqual([{ op: "move", item: "b", to: 0 }]);
});

test("ItemLayer an insert refused by the host asks to edit nothing", async () => {
  const { host, calls } = await mount(two(), "f", Promise.resolve(false));
  await click(host.querySelector("[data-item-insert]")!);
  expect(calls.apply.length).toBe(1);
  expect(calls.inserted).toEqual([]);
});

test("ItemLayer an insert accepted by the host asks to edit the new field", async () => {
  const { host, calls } = await mount(two(), "f", Promise.resolve(true));
  await click(host.querySelector("[data-item-insert]")!);
  expect(calls.inserted.length).toBe(1);
});

test("ItemLayer a pointer inside an item shows its handles even within the slack of the previous one", async () => {
  const { host } = await mount(viewOf([item("a", 0, 0), item("b", 1, 108)]));
  await act(async () => {
    host.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 112, clientY: 120 }));
  });
  const shown = [...host.querySelectorAll(".item-handles")].map((h) => h.classList.contains("shown"));
  expect(shown).toEqual([false, true]);
});

test("ItemLayer two lists sharing an item id show handles only on the hovered one", async () => {
  const view = viewOf([
    item("d0", 0, 0, { list: "a" }),
    item("d0", 0, 300, { list: "b" }),
  ]);
  const { host } = await mount(view);
  await act(async () => {
    host.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 350, clientY: 120 }));
  });
  const shown = [...host.querySelectorAll<HTMLElement>(".item-handles")].map((e) => e.classList.contains("shown"));
  expect(shown).toEqual([false, true]);
});
