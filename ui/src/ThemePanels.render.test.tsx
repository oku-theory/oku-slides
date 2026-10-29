// A theme tab's panels: header, tools, the fixed specimens pile and the theme panel in the notes.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";
import type { KeptUi } from "./useKeptUi";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: ThemePanels } = await import("./ThemePanels");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** A theme tab's snapshot: two specimens, the draft in `theming`. */
const SNAP = {
  name: "Papier", title: "Papier", dirty: false, history: null, read_only: false, editing: null,
  slides: [
    { id: "layout-title", kind: "essentiel/title", title: "Titre", hash: "a" },
    { id: "layout-content", kind: "essentiel/content", title: null, hash: "b" },
  ],
  can_undo: false, can_redo: false, undo_label: null, redo_label: null, appearance: { chart_palette: [] },
  theming: {
    theme: { base: { id: "t", version: "v1" }, namespace: "papier", name: "Papier", child: true, extends: [], dirty: false, revision: "r1" },
    session_version: null, read_only: false, leased: true, display_name: "Papier",
  },
  ui: { slide: null, grid: true, ghosts: false, pile: null, notes: null, widget: null, code: false, code_width: null, folded: [] },
} as unknown as Snapshot;

/** Any request answers nothing. */
const api = new Proxy({}, { get: () => async () => null }) as TabApi;

const keptUi = {
  grid: true, ghosts: false, columns: { pile: null, notes: null }, setColumns: () => {}, code: false, setCode: () => {},
  codeWidth: null, setCodeWidth: () => {}, folded: [], setFolded: () => {}, setGrid: () => {}, setGhosts: () => {},
  handleKey: () => false,
} as unknown as KeptUi;

async function mount() {
  const host = document.body.appendChild(document.createElement("div"));
  const notes = document.body.appendChild(document.createElement("aside"));
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(ThemePanels, {
        api, snap: SNAP, keptUi, current: "layout-title", run: async () => null, say: () => {},
        svg: (id: string) => `blob:${id}`, mainMenu: null, notes, onSelect: () => {},
      }),
    ),
  );
  return { host, notes };
}

describe("ThemePanels", () => {
  test("ThemePanels shows the specimens pile without add or delete", async () => {
    const { host } = await mount();
    const pile = host.querySelector(".pile-column");
    expect([pile?.querySelectorAll(".thumb").length, pile?.querySelectorAll(".thumb-remove").length]).toEqual([2, 0]);
    expect(pile?.querySelector(".add-slide")).toBeNull();
  });

  test("ThemePanels mounts the theme panel in the notes column", async () => {
    const { host, notes } = await mount();
    expect([!!notes.querySelector("#theme-panel"), !!host.querySelector("#theme-panel")]).toEqual([true, false]);
  });
});
