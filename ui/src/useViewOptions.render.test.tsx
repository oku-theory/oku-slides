// Options d'affichage (grille, fantômes, colonnes) : reprise, bascule, raccourcis, sauvegarde chez l'hôte.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UiState } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useViewOptions, viewToggle } = await import("./useViewOptions");

type Hook = ReturnType<typeof useViewOptions>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

// Slide gardée ≠ slide courante : la sauvegarde doit prendre la courante.
const kept: UiState = { slide: "s0", grid: false, ghosts: true, pile: 200, notes: null };

/** Monte le hook ; `render` le relance (état gardé, slide courante) ; `saved` reçoit chaque sauvegarde. */
async function mount(first: UiState | null = kept) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: Hook | null; saved: [string | null, boolean, boolean][]; kept: UiState[] } = { hook: null, saved: [], kept: [] };
  const save = async (ui: UiState) => {
    out.saved.push([ui.slide, ui.grid, ui.ghosts]);
    out.kept.push(ui);
  };
  function Probe({ ui, current }: { ui: UiState | null; current: string }) {
    out.hook = useViewOptions(ui, current, save);
    return null;
  }
  const render = (ui: UiState | null, current = "s1") => act(async () => root.render(createElement(Probe, { ui, current })));
  await render(first);
  return { out, render };
}

const key = (k: string, more: { meta?: boolean; ctrl?: boolean; shift?: boolean } = {}) => {
  const e = { key: k, metaKey: !!more.meta, ctrlKey: !!more.ctrl, shiftKey: !!more.shift, prevented: false, preventDefault: () => { e.prevented = true; } };
  return e;
};

/** Libellés et raccourcis des entrées du menu (identifiants bruts : pas de messages chargés). */
const labels = (h: Hook) => h.entries.map((e) => (e.kind === "item" ? `${e.label} ${e.shortcut}` : e.kind));

test("useViewOptions first snapshot restores the choices kept by the host", async () => {
  const { out } = await mount();
  expect([out.hook!.grid, out.hook!.ghosts]).toEqual([false, true]);
});

test("useViewOptions later snapshot keeps the user's choice", async () => {
  const { out, render } = await mount();
  await act(async () => { out.hook!.handleKey(key("'", { meta: true })); });
  await render({ ...kept, grid: false });
  expect(out.hook!.grid).toBe(true);
});

test("useViewOptions saves nothing before the first snapshot", async () => {
  const { out } = await mount(null);
  expect(out.saved).toEqual([]);
});

test("useViewOptions first snapshot after an empty mount restores the kept choices and saves them once", async () => {
  const { out, render } = await mount(null);
  await render(kept);
  expect([out.hook!.grid, out.hook!.ghosts, out.saved]).toEqual([false, true, [["s1", false, true]]]);
});

test("useViewOptions saves again when the current slide changes", async () => {
  const { out, render } = await mount();
  await render(kept, "s2");
  expect(out.saved.at(-1)).toEqual(["s2", false, true]);
});

test("useViewOptions saves the current slide and choices once restored and on each toggle", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.handleKey(key("g", { meta: true, shift: true })); });
  expect(out.saved).toEqual([["s1", false, true], ["s1", false, false]]);
});

test("useViewOptions menu entries name the toggle to come and switch it", async () => {
  const { out } = await mount();
  expect(labels(out.hook!)).toEqual(["ui-toolbar-grid-show ⌘'", "ui-toolbar-ghosts-hide ⌘⇧G"]);
  await act(async () => { (out.hook!.entries[0] as { onSelect: () => void }).onSelect(); });
  expect(labels(out.hook!)).toEqual(["ui-toolbar-grid-hide ⌘'", "ui-toolbar-ghosts-hide ⌘⇧G"]);
});

test("useViewOptions handleKey takes its shortcut and prevents the default", async () => {
  const { out } = await mount();
  const e = key("G", { meta: true, shift: true });
  let handled = false;
  await act(async () => { handled = out.hook!.handleKey(e); });
  expect([handled, e.prevented, out.hook!.ghosts]).toEqual([true, true, false]);
});

test("useViewOptions first snapshot restores the column widths kept by the host", async () => {
  const { out } = await mount();
  expect(out.hook!.columns).toEqual({ pile: 200, notes: null });
});

test("useViewOptions setColumns saves the new widths with the rest of the state", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setColumns({ pile: null, notes: 540 }); });
  expect(out.kept.at(-1)).toEqual({ slide: "s1", grid: false, ghosts: true, pile: null, notes: 540 });
});

test("useViewOptions handleKey leaves other keys alone", async () => {
  const { out } = await mount();
  const e = key("z", { meta: true });
  expect([out.hook!.handleKey(e), e.prevented]).toEqual([false, false]);
});

describe("viewToggle", () => {
  test("viewToggle cmd quote toggles the grid", () => {
    expect(viewToggle(key("'", { meta: true }))).toBe("grid");
  });

  test("viewToggle cmd shift G toggles the ghosts, whatever the case WebKit reports", () => {
    expect([viewToggle(key("G", { meta: true, shift: true })), viewToggle(key("g", { ctrl: true, shift: true }))]).toEqual(["ghosts", "ghosts"]);
  });

  test("viewToggle G without shift or without modifier toggles nothing", () => {
    expect([viewToggle(key("g", { meta: true })), viewToggle(key("G", { shift: true }))]).toEqual([null, null]);
  });
});
