// A theme tab as App mounts it: the specimen pile, the stage, the theme panel and "Finish".

import { afterAll, afterEach, beforeAll, describe, expect, mock, test } from "bun:test";
import "./test/dom";
import type { Snapshot } from "./api";
import { resetMessages } from "./test/messages";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const App = (await import("./App")).default;
const { setMessages } = await import("./i18n");

const SLIDES = [
  { id: "layout-title", kind: "essentiel/title", title: "Titre", hash: "a" },
  { id: "layout-content", kind: "essentiel/content", title: null, hash: "b" },
];

/** A theme tab's live snapshot: the specimen deck, no pack, the draft in `theming`. */
const SNAP = {
  path: null, name: "Papier", dirty: false, theme: null, theme_pins: {}, theme_name: null, themes: [],
  title: "Papier", title_source: "Papier", lang: "fr", slides: SLIDES,
  can_undo: false, undo_label: null, can_redo: false, redo_label: null, undo_turn: null,
  warnings: [], notice: null, disk_replaced: false,
  appearance: { scheme: "light", slide: {}, app: {}, variants: {}, metrics: {}, font_display: [], font_body: [], chart_palette: [] },
  widgets: {}, layouts: {}, editing: null, deck_pack: false, pack_updates: 0, pack_widgets: [], add_packs: [],
  free_slots: { "essentiel/content": null }, widget_trials: 0, history: null, read_only: false, host_params: {},
  ui: { slide: null, ghosts: false, pile: null, notes: null, widget: null, code: false, code_width: null, folded: [] },
  theming: {
    theme: { base: { id: "t", version: "v1" }, namespace: "papier", name: "Papier", child: true, extends: [], dirty: false, revision: "r1" },
    session_version: null, read_only: false, leased: true, display_name: "Papier",
  },
} as unknown as Snapshot;

const VIEW = {
  slide: { id: "layout-title", kind: "essentiel/title", params: {} }, page: 0, hash: "a", width: 1280, height: 720,
  targets: [], fields: [], items: [], lists: [],
};

/** The host's answer to one request, by command; null for anything else. */
function answer(endpoint: string, body: { req?: { cmd?: string } }): unknown {
  if (endpoint !== "/api") return null;
  switch (body.req?.cmd) {
    case "snapshot":
    case "ui":
      return SNAP;
    case "slide":
      return VIEW;
    case "svg":
      return "<svg xmlns='http://www.w3.org/2000/svg'/>";
    default:
      return null;
  }
}

const realFetch = globalThis.fetch;
const realSource = (globalThis as { EventSource?: unknown }).EventSource;
beforeAll(() => {
  globalThis.fetch = (async (url: string, init?: { body?: string }) => {
    const body = JSON.parse(init?.body ?? "{}");
    return new Response(JSON.stringify(answer(String(url), body)), { status: 200 });
  }) as typeof fetch;
  // The dev server's event stream: never opened here.
  (globalThis as { EventSource?: unknown }).EventSource = class {
    addEventListener() {}
    close() {}
  };
  setMessages("en", ["ui-tab-theme = Theme\nui-toolbar-finish = Finish\nui-pile-add-slide = New slide\n"]);
});
afterAll(() => {
  globalThis.fetch = realFetch;
  (globalThis as { EventSource?: unknown }).EventSource = realSource;
});
afterAll(resetMessages);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mountThemeTab(onClose: () => void) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const noop = () => {};
  await act(async () =>
    root.render(
      createElement(App, {
        tab: "t-2", clip: { current: null }, clipGen: 0, onCopy: noop, nonce: 0, onOpen: noop, importJob: null,
        onImportPick: noop, onImport: noop, onImportClose: noop, onHelp: noop, onClose,
        onOpenPack: async () => false, onOpenTheme: noop,
      }),
    ),
  );
  await act(async () => new Promise((r) => setTimeout(r, 50)));
  return host;
}

describe("App in a theme tab", () => {
  test("App theme tab pile shows the specimens, fixed", async () => {
    const host = await mountThemeTab(() => {});
    const pile = host.querySelector(".pile-column");
    expect(pile).not.toBeNull();
    expect([...pile!.querySelectorAll(".thumb")].length).toBe(2);
    expect([...pile!.querySelectorAll("button")].some((b) => /New slide/.test(b.textContent ?? ""))).toBe(false);
    expect(pile!.querySelectorAll(".thumb-remove").length).toBe(0);
    expect(host.querySelector(".pack-pile")).toBeNull();
  });

  test("App theme tab mounts the theme panel in the notes column", async () => {
    const host = await mountThemeTab(() => {});
    expect(host.querySelector("aside.notes.theme-active #theme-panel")).not.toBeNull();
  });

  test("App theme tab has no packs button", async () => {
    const host = await mountThemeTab(() => {});
    expect(host.querySelector(".packs-button")).toBeNull();
  });

  test("App theme tab Finish calls the close", async () => {
    const onClose = mock(() => {});
    const host = await mountThemeTab(onClose);
    const finish = host.querySelector<HTMLButtonElement>('.tray button[aria-label="Finish"]');
    await act(async () => finish!.click());
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
