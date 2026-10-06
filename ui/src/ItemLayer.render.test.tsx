// Poignées des items de liste sur la scène : « × » et ⋮⋮ sur chaque item,
// repère du glisser ; pas de « + » (l'ajout passe par le tableau du panneau).

import { afterAll, afterEach, beforeAll, expect, test } from "bun:test";
import "./test/dom";
import type { ItemView, ListOp, ListView, Op, SlideView } from "./api";
import { setMessages } from "./i18n";
import { resetMessages } from "./test/messages";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ItemLayer } = await import("./ItemLayer");
const { deckListOp } = await import("./stageItems");

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
  item_label: "étape", x, y: 100, w: 100, h: 50, ...more,
});

const viewOf = (items: ItemView[], lists: ListView[] = []) =>
  ({
    slide: { id: "s", kind: "content", params: {} },
    page: 1, hash: "h", width: 1000, height: 600, zone: null, fields: [],
    targets: [{ id: "f", kind: "frise", free: false, x: 0, y: 100, w: 400, h: 200 }],
    items, lists,
  }) as unknown as SlideView;

const two = (more: Partial<ItemView> = {}) => viewOf([item("a", 0, 0, more), item("b", 1, 120, more)]);

async function mount(view: SlideView, active: string | null = "f") {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  // `listOp` : les changements tels que rendus ; `apply` : les ops qu'en fait la scène du deck.
  const calls: { listOp: [string, string, string, ListOp, string][]; apply: [string, Op[]][] } = { listOp: [], apply: [] };
  // Comme la scène du deck : l'adaptateur réel, qui fait des ops `edit_list`.
  const apply = deckListOp(view, (label, ops) => void calls.apply.push([label, ops]));
  await act(async () =>
    root.render(
      createElement(ItemLayer, {
        view, scale: 1, active,
        onListOp: (owner: Pick<ListView, "owner" | "owner_kind">, list: string, change: ListOp, label: string) => {
          calls.listOp.push([owner.owner, owner.owner_kind, list, change, label]);
          apply(owner, list, change, label);
        },
      }),
    ),
  );
  return { host, calls };
}

const click = (el: Element) =>
  act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });

test("ItemLayer a list at its minimum offers no remove", async () => {
  const { host } = await mount(two({ min: 2 }));
  expect(host.querySelector("[data-item-remove]")).toBeNull();
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
  expect(host.querySelector("[data-item-remove]")?.getAttribute("aria-label")).toBe("Supprimer élément");
});

test("ItemLayer a slide list targets the slide", async () => {
  const { host, calls } = await mount(two({ owner: "s", owner_kind: "slide" }), "s");
  await click(host.querySelector("[data-item-remove]")!);
  expect((calls.apply[0][1][0] as { target: unknown }).target).toEqual({ slide: "s" });
});

/** Glisse la poignée ⋮⋮ de `el` jusqu'à `x` (pt, à l'échelle 1) ; `during` voit la scène avant le lâcher. */
async function dragTo(el: HTMLElement, x: number, during: () => void = () => {}) {
  el.setPointerCapture = () => {};
  const fire = (type: string, clientX: number) =>
    act(async () => {
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX, clientY: 120 }));
    });
  await fire("pointerdown", 50);
  await fire("pointermove", x);
  during();
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

test("ItemLayer the drop marker shows the index the drop applies, with items of unequal sizes", async () => {
  // a large, b étroit, c large : à 181, le point entre b et c (125) est plus proche que celui après c (240),
  // mais le lâcher, compté sur les centres des autres items, met a après c.
  const view = viewOf([item("a", 0, 0, { len: 3 }), item("b", 1, 110, { len: 3, w: 10 }), item("c", 2, 130, { len: 3 })]);
  const { host, calls } = await mount(view);
  let marker = "";
  await dragTo(host.querySelectorAll<HTMLElement>("[data-item-move]")[0], 181, () => {
    marker = host.querySelector<HTMLElement>(".item-drop")?.style.left ?? "";
  });
  expect(marker).toBe("240px");
  expect((calls.apply[0][1][0] as { change: unknown }).change).toEqual({ op: "move", item: "a", to: 2 });
});

test("ItemLayer dropping an item where it was moves nothing", async () => {
  const { host, calls } = await mount(three());
  await dragTo(host.querySelectorAll<HTMLElement>("[data-item-move]")[1], 160);
  expect(calls.apply).toEqual([]);
});

const emptyList = (more: Partial<ListView> = {}): ListView => ({
  owner: "f", owner_kind: "element", list: "etapes", len: 0, min: 0, max: 8, item_label: "étape", ...more,
});

test("ItemLayer a list shows remove and move handles but no plus", async () => {
  const { host } = await mount(viewOf(three().items, [emptyList({ list: "vide" })]));
  expect(host.querySelector("[data-item-insert]")).toBeNull();
  expect(host.querySelectorAll(".item-remove").length).toBe(3);
  expect(host.querySelectorAll(".item-move").length).toBe(3);
});

test("ItemLayer a full list still offers remove and move", async () => {
  const { host } = await mount(two({ max: 2 }));
  expect(host.querySelectorAll("[data-item-remove]").length).toBe(2);
  expect(host.querySelectorAll("[data-item-move]").length).toBe(2);
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

test("ItemLayer a remove gives the owner, the list and the change, without building deck ops", async () => {
  const { host, calls } = await mount(two());
  await click(host.querySelectorAll("[data-item-remove]")[0]);
  expect(calls.listOp).toEqual([["f", "element", "etapes", { op: "remove", item: "a" }, "Supprimer étape"]]);
});
