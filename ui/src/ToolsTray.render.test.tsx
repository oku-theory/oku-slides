// Renders the toolbar under the sheet in a happy-dom DOM: the history button, the thumbnails strip
// toggle, and no packs or inspector button (they are in the side panel).

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";
import type { KeptUi } from "./useKeptUi";
import type { PackItem } from "./usePackItem";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ToolsTray } = await import("./ToolsTray");
const { expectIconOnly } = await import("./test/iconOnly");
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");

afterAll(resetMessages);
setMessages("fr", [
  "ui-toolbar-history = Afficher l'historique\n",
]);

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

async function mount(
  snap: Snapshot, themeTab = false, item: PackItem | null = null, keptUi = {} as KeptUi,
  onAdd: (() => void) | null = () => {},
) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  let histories = 0;
  await act(async () =>
    root.render(
      createElement(ToolsTray, {
        api: {} as TabApi, snap, item,
        keptUi, mainMenu: null, run: async () => null,
        onAdd, onHistory: () => void histories++, thumbs: null, themeTab,
      }),
    ),
  );
  const history = () => host.querySelector<HTMLButtonElement>(".history-button");
  return { host, history, histories: () => histories };
}

test("ToolsTray in deck mode shows no packs or inspector button", async () => {
  const { host } = await mount(snapshot({ pack_updates: 2 }));
  expect([host.querySelector(".packs-toggle, .packs-button"), host.querySelector(".inspector-toggle")]).toEqual([null, null]);
});

test("ToolsTray in deck mode shows the history button, which opens the history", async () => {
  const { history, histories } = await mount(snapshot());
  await act(async () => history()!.click());
  expect([history()!.title, histories()]).toEqual(["Afficher l'historique", 1]);
});

test("ToolsTray history button shows an icon, no text", async () => {
  const { history } = await mount(snapshot());
  expectIconOnly(history(), "Afficher l'historique");
});

test.each([
  ["help", { read_only: true }, false],
  ["a theme tab", { theming: {} as Snapshot["theming"] }, true],
  ["pack mode", { editing: {} as Snapshot["editing"] }, false],
  ["history", { history: {} as Snapshot["history"] }, false],
] as const)("ToolsTray in %s hides the history button", async (_, o, themeTab) => {
  const { history } = await mount(snapshot(o), themeTab);
  expect(history()).toBeNull();
});

/** The add button: the opener App passes decides it; `null` disables it (t-1390). */
test("ToolsTray without an add opener disables the add button", async () => {
  const host = (await mount(snapshot(), false, null, {} as KeptUi, null)).host;
  expect(host.querySelector<HTMLButtonElement>(".add-button")!.disabled).toBe(true);
});

test("ToolsTray with an add opener enables the add button", async () => {
  const host = (await mount(snapshot())).host;
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
      api: {} as TabApi, snap: thumbsSnap, item: null, keptUi: {} as never,
      mainMenu: null, run: async () => null, onAdd: () => {}, onHistory: () => {}, thumbs,
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

test("ToolsTray in a pack tab with the code folded shows the session's labelled undo", async () => {
  const item = { selected: "kpi", history: { undo: 0, redo: 0 }, undo: () => {}, redo: () => {} } as unknown as PackItem;
  const keptUi = { code: false, setCode: () => {} } as unknown as KeptUi;
  const snap = snapshot({ editing: {} as Snapshot["editing"], can_undo: true, undo_label: "Delete kpi" });
  const { host } = await mount(snap, false, item, keptUi);
  const undo = host.querySelector<HTMLButtonElement>('button[aria-label="ui-toolbar-undo-label"]');
  expect([undo?.title, undo?.disabled]).toEqual(["Delete kpi", false]);
});
