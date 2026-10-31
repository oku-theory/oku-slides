// Barre d'onglets : le menu contextuel échappe à la vitre de la barre.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { TabDocument, TabInfo, WindowView, WorkspaceView } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const TabBar = (await import("./TabBar")).default;
const { setMessages, t } = await import("./i18n");
const { resetMessages } = await import("./test/messages");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(resetMessages);
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

/** Each tab's `data-kind`. */
const kinds = (host: HTMLElement) => [...host.querySelectorAll('[role="tab"]')].map((e) => e.getAttribute("data-kind"));

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
  expect(document.querySelector(".context-menu")?.parentElement).toBe(document.body);
});

test("TabBar tab in pack mode is of the pack kind", async () => {
  const host = await mount(true);
  expect(kinds(host)).toEqual(["pack", "deck"]);
});

test("TabBar pack tab names its kind", async () => {
  setMessages("en", ["ui-tab-kind-pack = { $title }, pack\nui-tab-kind-theme = { $title }, theme\n"]);
  const host = await mount(false, [tab("Mine", { kind: "pack", pack: PACK }), tab("Look", { kind: "theme", theme: PACK }), tab("Plain")]);
  const names = [...host.querySelectorAll('[role="tab"]')].map((e) => e.getAttribute("aria-label"));
  expect(names).toEqual([t("ui-tab-kind-pack", { title: "Mine" }), t("ui-tab-kind-theme", { title: "Look" }), null]);
  expect(names[0]).toBe("Mine, pack");
});

const ftl = (lang: string) => Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();
/** Each tab's accessible name, from the real `lang` messages over English. */
async function names(lang: string, tabs: TabInfo[]) {
  setMessages(lang, lang === "en" ? [await ftl("en")] : [await ftl("en"), await ftl(lang)]);
  const host = await mount(false, tabs);
  return [...host.querySelectorAll('[role="tab"]')].map((e) => e.getAttribute("aria-label"));
}
const dirty = (x: TabInfo): TabInfo => ({ ...x, dirty: true, pristine: false });

test.each([
  ["en", ["Mine, pack, Unsaved changes", "Look, theme, Unsaved changes"]],
  ["fr", ["Mine, pack, Modifications non enregistrées", "Look, thème, Modifications non enregistrées"]],
])("TabBar a dirty pack or theme tab names its unsaved state (%s)", async (lang, expected) => {
  const tabs = [dirty(tab("Mine", { kind: "pack", pack: PACK })), dirty(tab("Look", { kind: "theme", theme: PACK }))];
  expect(await names(lang, tabs)).toEqual(expected);
});

test("TabBar a clean pack tab's name says nothing of unsaved changes", async () => {
  expect(await names("en", [tab("Mine", { kind: "pack", pack: PACK })])).toEqual(["Mine, pack"]);
});

test("TabBar has no tab badge and its kind icon is aria-hidden", async () => {
  const host = await mount(false, [tab("p", { kind: "pack", pack: PACK }), tab("t", { kind: "theme", theme: PACK }), tab("d")]);
  expect(host.querySelector(".tab-badge")).toBeNull();
  const icons = [...host.querySelectorAll('[role="tab"]')].map((e) => e.querySelector(".tab-kind-icon"));
  expect(icons[0]?.getAttribute("aria-hidden")).toBe("true");
  expect(icons[1]?.getAttribute("aria-hidden")).toBe("true");
  expect(icons[2]).toBeNull();
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

test("TabBar kind follows the document kind, pack or deck_pack", async () => {
  const host = await mount(false, [
    tab("pack", { kind: "pack", pack: PACK }),
    tab("deck", { kind: "deck", path: null, deck_uid: null }),
    tab("deck-pack", { kind: "deck_pack", deck_tab: "deck", pack: PACK }),
    tab("help", { kind: "help", locale: "fr" }),
  ]);
  expect(kinds(host)).toEqual(["pack", "deck", "pack", "help"]);
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
  const items = [...document.querySelectorAll<HTMLButtonElement>('.context-menu [role="menuitem"]')];
  const others = items.find((b) => b.textContent === "ui-tab-close-others");
  expect(others?.disabled).toBe(true);
});

test("TabBar theme tab is of the theme kind, a pack tab still of the pack kind", async () => {
  const host = await mount(false, [
    tab("theme", { kind: "theme", theme: "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31" }),
    tab("pack", { kind: "pack", pack: PACK }),
  ]);
  expect(kinds(host)).toEqual(["theme", "pack"]);
});
