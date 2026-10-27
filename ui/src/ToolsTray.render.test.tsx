// Rendu de la barre d'outils sous la feuille dans un DOM happy-dom : le bouton des packs du deck,
// la bascule de la bande des miniatures.

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { SlideView, Snapshot, TabApi } from "./api";
import type { KeptUi } from "./useKeptUi";
import type { PackWidgetEditor } from "./usePackWidgetEditor";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ToolsTray } = await import("./ToolsTray");
const { expectIconOnly } = await import("./test/iconOnly");
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");

afterAll(resetMessages);
setMessages("fr", ["ui-toolbar-packs = Packs du deck\nui-toolbar-packs-update = Packs du deck — mises à jour disponibles\n"]);

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Un snapshot du deck : rien à annuler, aucune slide à éléments libres. */
const snapshot = (o: Partial<Snapshot> = {}): Snapshot =>
  ({
    editing: null, read_only: false, history: null, pack_updates: 0, free_slots: {},
    can_undo: false, can_redo: false, undo_label: null, redo_label: null, ...o,
  }) as unknown as Snapshot;

async function mount(snap: Snapshot, view: SlideView | null = null, themeTab = false) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  let opened = 0;
  await act(async () =>
    root.render(
      createElement(ToolsTray, {
        api: {} as TabApi, snap, view, widgetActive: false, widgets: {} as PackWidgetEditor,
        keptUi: {} as KeptUi, mainMenu: null, run: async () => null,
        onAdd: () => {}, onPacks: () => void opened++, thumbs: null, themeTab,
      }),
    ),
  );
  const packs = () => host.querySelector<HTMLButtonElement>(".packs-button");
  return { host, packs, opened: () => opened };
}

test("ToolsTray in deck mode shows the packs button, which opens the panel", async () => {
  const { packs, opened } = await mount(snapshot());
  await act(async () => packs()!.click());
  expect([packs()!.classList.contains("has-update"), packs()!.title, packs()!.getAttribute("aria-label"), opened()]).toEqual([
    false, "Packs du deck", "Packs du deck", 1,
  ]);
});

test("ToolsTray with pending updates shows the dot", async () => {
  const { packs } = await mount(snapshot({ pack_updates: 2 }));
  const full = "Packs du deck — mises à jour disponibles";
  expect([packs()!.classList.contains("has-update"), packs()!.title, packs()!.getAttribute("aria-label")]).toEqual([true, full, full]);
});

test("ToolsTray in help mode hides the packs button", async () => {
  const { packs } = await mount(snapshot({ read_only: true }));
  expect(packs()).toBeNull();
});

test("ToolsTray in a theme tab hides the packs button", async () => {
  const { packs } = await mount(snapshot({ theming: {} as Snapshot["theming"] }), null, true);
  expect(packs()).toBeNull();
});

test("ToolsTray in pack mode or history hides the packs button", async () => {
  const editing = await mount(snapshot({ editing: {} as Snapshot["editing"] }));
  const shown = [editing.packs()];
  await act(async () => mounted!.root.unmount());
  mounted!.host.remove();
  mounted = null;
  const history = await mount(snapshot({ history: {} as Snapshot["history"] }));
  expect([...shown, history.packs()]).toEqual([null, null]);
});

/** La slide montrée, de kind `kind`. */
const shown = (kind: string) => ({ slide: { id: "s", kind, params: {} } }) as unknown as SlideView;

test("ToolsTray on a slide without free slot disables the add button", async () => {
  const host = (await mount(snapshot({ free_slots: { "essentiel/content": null } }), shown("essentiel/title"))).host;
  expect(host.querySelector<HTMLButtonElement>(".add-button")!.disabled).toBe(true);
});

test("ToolsTray on a slide with a free slot enables the add button", async () => {
  const host = (await mount(snapshot({ free_slots: { "essentiel/content": null } }), shown("essentiel/content"))).host;
  expect(host.querySelector<HTMLButtonElement>(".add-button")!.disabled).toBe(false);
});

const thumbsSnap = snapshot();

async function mountThumbs(thumbs: { open: boolean; onToggle: () => void } | null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(ToolsTray, {
      api: {} as TabApi, snap: thumbsSnap, view: null, widgetActive: false, widgets: {} as never, keptUi: {} as never,
      mainMenu: null, run: async () => null, onAdd: () => {}, onPacks: () => {}, thumbs,
    }));
  });
  return host;
}

const toggle = (host: HTMLElement) => host.querySelector<HTMLButtonElement>("button.thumbs-toggle");

describe("ToolsTray", () => {
  test("ToolsTray open strip offers to hide the thumbnails, pressed", async () => {
    const host = await mountThumbs({ open: true, onToggle: () => {} });
    expect([toggle(host)?.getAttribute("aria-label"), toggle(host)?.getAttribute("aria-pressed")]).toEqual(["ui-toolbar-thumbs-hide", "true"]);
  });

  test("ToolsTray folded strip offers to show the thumbnails", async () => {
    const host = await mountThumbs({ open: false, onToggle: () => {} });
    expect(toggle(host)?.getAttribute("aria-label")).toBe("ui-toolbar-thumbs-show");
  });

  test("ToolsTray thumbnails toggle shows an icon, no text", async () => {
    const host = await mountThumbs({ open: true, onToggle: () => {} });
    expectIconOnly(toggle(host), "ui-toolbar-thumbs-hide");
  });

  test("ToolsTray thumbnails toggle calls onToggle", async () => {
    let toggled = 0;
    const host = await mountThumbs({ open: true, onToggle: () => toggled++ });
    await act(async () => toggle(host)?.click());
    expect(toggled).toBe(1);
  });

  test("ToolsTray without a strip shows no thumbnails toggle", async () => {
    const host = await mountThumbs(null);
    expect(toggle(host)).toBeNull();
  });
});
