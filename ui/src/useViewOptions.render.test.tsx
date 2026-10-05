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

type Tabs = Pick<UiState, "widgets" | "widget">;

// Slide gardée ≠ slide courante : la sauvegarde doit prendre la courante.
const kept: UiState = { slide: "s0", grid: false, ghosts: true, pile: 200, notes: null, widgets: [], widget: null, code: true,
  code_width: 640, folded: ["widgets"], case: { kpi: "Long" } };

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
  function Probe({ ui, current, tabs }: { ui: UiState | null; current: string; tabs: Tabs }) {
    out.hook = useViewOptions(ui, current, save, tabs);
    return null;
  }
  const render = (ui: UiState | null, current = "s1", tabs: Tabs = { widgets: [], widget: null }) =>
    act(async () => root.render(createElement(Probe, { ui, current, tabs })));
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
  expect(out.kept.at(-1)).toEqual({ slide: "s1", grid: false, ghosts: true, pile: null, notes: 540, widgets: [], widget: null, code: true,
    code_width: 640, folded: ["widgets"], case: { kpi: "Long" } });
});

test("useViewOptions premier snapshot reprend le code source déplié gardé par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.code).toBe(true);
});

test("useViewOptions setCode sauve le code replié avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCode(false); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", code: false });
});

test("useViewOptions premier snapshot reprend la largeur du code gardée par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.codeWidth).toBe(640);
});

test("useViewOptions setCodeWidth sauve la largeur du code avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCodeWidth(320); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", code: true, code_width: 320 });
});

test("useViewOptions setCodeWidth(null) sauve la proportion par défaut", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCodeWidth(null); });
  expect([out.hook!.codeWidth, out.kept.at(-1)?.code_width]).toEqual([null, null]);
});

test("useViewOptions premier snapshot reprend les sections repliées gardées par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.folded).toEqual(["widgets"]);
});

test("useViewOptions setFolded replie une section et la sauve avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setFolded("templates", true); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", folded: ["widgets", "templates"] });
});

test("useViewOptions setFolded rouvre une section repliée", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setFolded("widgets", false); });
  expect(out.kept.at(-1)).toMatchObject({ folded: [] });
});

test("useViewOptions setFolded sur l'état déjà en place ne sauve rien de plus", async () => {
  const { out } = await mount();
  const before = out.kept.length;
  await act(async () => { out.hook!.setFolded("widgets", true); });
  expect(out.kept.length).toBe(before);
});

test("useViewOptions sauve les sous-onglets passés avec le reste", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: "kpi" });
  expect(out.kept.at(-1)).toMatchObject({ widgets: ["kpi"], widget: "kpi" });
});

test("useViewOptions avec une nouvelle liste de même contenu ne sauve pas de nouveau", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  const n = out.saved.length;
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  expect(out.saved.length).toBe(n);
});

test("useViewOptions premier snapshot reprend les cas courants gardés par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.cases).toEqual({ kpi: "Long" });
});

test("useViewOptions un état gardé sans cas courants (hôte plus ancien) : aucun", async () => {
  const { case: _, ...older } = kept;
  const { out } = await mount(older as UiState);
  expect(out.hook!.cases).toEqual({});
});

test("useViewOptions setCase choisit le cas d'un widget et le sauve avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCase("quote", "Court"); });
  expect(out.hook!.cases).toEqual({ kpi: "Long", quote: "Court" });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", case: { kpi: "Long", quote: "Court" } });
});

test("useViewOptions setCase sur le cas déjà choisi ne sauve rien de plus", async () => {
  const { out } = await mount();
  const before = out.kept.length;
  await act(async () => { out.hook!.setCase("kpi", "Long"); });
  expect(out.kept.length).toBe(before);
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
