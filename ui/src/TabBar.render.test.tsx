// Barre d'onglets : le menu contextuel échappe à la vitre de la barre.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { TabDocument, TabInfo, WindowView, WorkspaceView } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const TabBar = (await import("./TabBar")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const PACK = "a7e11e40-0000-4000-8000-000000000001";
const tab = (id: string, document: TabDocument = { kind: "deck", path: null, deck_uid: null }): TabInfo => ({
  id,
  title: id,
  dirty: false,
  pristine: true,
  theme: null,
  document,
});

/** Each tab's badge text, or null. */
const badges = (host: HTMLElement) => [...host.querySelectorAll('[role="tab"]')].map((e) => e.querySelector(".tab-badge")?.textContent ?? null);

async function mount(packTab = false, tabs: TabInfo[] = [tab("a", packTab ? { kind: "pack", pack: PACK } : undefined), tab("b")]) {
  const win: WindowView = { label: "main", active: "a", tabs };
  const view: WorkspaceView = { generation: 1, focused: "main", agent_detached: false, import_pick: null, windows: [win] };
  const noop = () => {};
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(TabBar, {
        view,
        win,
        windows: false,
        onActivate: noop,
        onClose: noop,
        onCloseOthers: noop,
        onNew: noop,
        onReorder: noop,
        onMove: noop,
        onDragOver: async () => {},
        onDragLeave: async () => {},
        onDropOutside: noop,
      }),
    ),
  );
  return host;
}

// t-812 : la barre a un `backdrop-filter` (verre) ; un menu `position: fixed`
// rendu dedans se place par rapport à elle, sous la fenêtre. Rendu dans `body`.
test("TabBar context menu renders outside the glass tab bar", async () => {
  const host = await mount();
  const first = host.querySelector('[role="tab"]')!;
  await act(async () => {
    first.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 880 }));
  });
  expect(document.querySelector(".tab-menu")?.parentElement).toBe(document.body);
});

test("TabBar tab in pack mode shows the pack badge", async () => {
  const host = await mount(true);
  expect(badges(host)).toEqual(["ui-tab-pack", null]);
});

// One tab per document kind: the tooltip is the deck's file, else the title.
test("TabBar tooltip is the deck's file, else the title, whatever the kind", async () => {
  const host = await mount(false, [
    tab("deck", { kind: "deck", path: "/d/a.oku-slides.pdf", deck_uid: "u" }),
    tab("new", { kind: "deck", path: null, deck_uid: null }),
    tab("help", { kind: "help", locale: "fr" }),
    tab("pack", { kind: "pack", pack: PACK }),
  ]);
  const titles = [...host.querySelectorAll('[role="tab"]')].map((e) => e.getAttribute("title"));
  expect(titles).toEqual(["/d/a.oku-slides.pdf", "new", "help", "pack"]);
});

test("TabBar badge follows the document kind, pack or deck_pack", async () => {
  const host = await mount(false, [
    tab("pack", { kind: "pack", pack: PACK }),
    tab("deck", { kind: "deck", path: null, deck_uid: null }),
    tab("deck-pack", { kind: "deck_pack", deck_tab: "deck", pack: PACK }),
    tab("help", { kind: "help", locale: "fr" }),
  ]);
  expect(badges(host)).toEqual(["ui-tab-pack", null, "ui-tab-pack", null]);
});

test("TabBar of a deck tab whose linked tab is the only other tab keeps its close disabled", async () => {
  const host = await mount(false, [tab("a"), tab("p", { kind: "deck_pack", deck_tab: "a", pack: PACK })]);
  const closes = [...host.querySelectorAll<HTMLButtonElement>(".tab-close")].map((b) => b.disabled);
  expect(closes).toEqual([true, false]);
});

test("TabBar close others is disabled on a deck-pack tab whose only other tab is its owner", async () => {
  const host = await mount(false, [tab("a"), tab("p", { kind: "deck_pack", deck_tab: "a", pack: PACK })]);
  const linked = host.querySelectorAll('[role="tab"]')[1]!;
  await act(async () => {
    linked.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 40 }));
  });
  const items = [...document.querySelectorAll<HTMLButtonElement>('.tab-menu [role="menuitem"]')];
  const others = items.find((b) => b.textContent === "ui-tab-close-others");
  expect(others?.disabled).toBe(true);
});

test("TabBar theme tab shows the theme badge, a pack tab still the pack badge", async () => {
  const host = await mount(false, [
    tab("theme", { kind: "theme", theme: "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31" }),
    tab("pack", { kind: "pack", pack: PACK }),
  ]);
  expect(badges(host)).toEqual(["ui-tab-theme", "ui-tab-pack"]);
});
