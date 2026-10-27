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

const tab = (
  id: string,
  editing_pack = false,
  document: TabDocument = { kind: "deck", path: null, deck_uid: null },
): TabInfo => ({
  id,
  title: id,
  dirty: false,
  pristine: true,
  editing_pack,
  theme: null,
  document,
});

async function mount(packTab = false, tabs: TabInfo[] = [tab("a", packTab), tab("b")]) {
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
  const badges = [...host.querySelectorAll('[role="tab"]')].map((e) => e.querySelector(".tab-badge")?.textContent ?? null);
  expect(badges).toEqual(["ui-tab-pack", null]);
});

// One tab per document kind: the tooltip is the deck's file, else the title; the badge follows `editing_pack` alone.
test("TabBar tooltip is the deck's file, else the title, whatever the kind", async () => {
  const host = await mount(false, [
    tab("deck", false, { kind: "deck", path: "/d/a.oku-slides.pdf", deck_uid: "u" }),
    tab("new", false, { kind: "deck", path: null, deck_uid: null }),
    tab("help", false, { kind: "help", locale: "fr" }),
    tab("pack", false, { kind: "pack", pack: "a7e11e40-0000-4000-8000-000000000001" }),
  ]);
  const titles = [...host.querySelectorAll('[role="tab"]')].map((e) => e.getAttribute("title"));
  expect(titles).toEqual(["/d/a.oku-slides.pdf", "new", "help", "pack"]);
});

test("TabBar badge follows editing_pack, not the document kind", async () => {
  const host = await mount(false, [
    tab("pack", false, { kind: "pack", pack: "a7e11e40-0000-4000-8000-000000000001" }),
    tab("workshop", true, { kind: "deck", path: null, deck_uid: null }),
    tab("help", false, { kind: "help", locale: "fr" }),
  ]);
  const badges = [...host.querySelectorAll('[role="tab"]')].map((e) => e.querySelector(".tab-badge")?.textContent ?? null);
  expect(badges).toEqual([null, "ui-tab-pack", null]);
});

test("TabBar theme tab shows the theme badge, a pack tab still the pack badge", async () => {
  const host = await mount(false, [
    tab("theme", false, { kind: "theme", theme: "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31" }),
    tab("pack", true, { kind: "pack", pack: "a7e11e40-0000-4000-8000-000000000001" }),
  ]);
  const badges = [...host.querySelectorAll('[role="tab"]')].map((e) => e.querySelector(".tab-badge")?.textContent ?? null);
  expect(badges).toEqual(["ui-tab-theme", "ui-tab-pack"]);
});
