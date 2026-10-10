// Options d'affichage (grille, fantômes, colonnes) : reprise, bascule, raccourcis, sauvegarde chez l'hôte.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UiOrder, UiState } from "./api";
import type { Timers } from "./debounce";
import { manualTimers } from "./test/timers";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { UI_RESEND_DELAY_MS, useKeptUi, viewToggle } = await import("./useKeptUi");

type Hook = ReturnType<typeof useKeptUi>;
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
  code_width: 640, band: 320, folded: ["widgets"] };

/** Monte le hook ; `render` le relance (état gardé, slide courante) ; `saved` reçoit chaque sauvegarde.
 *  `answer` : la réponse de l'hôte à une sauvegarde (immédiate par défaut). */
async function mount(first: UiState | null = kept, answer: () => Promise<unknown> = async () => {}, timers?: Timers) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: Hook | null; saved: [string | null, boolean, boolean][]; kept: UiState[]; orders: UiOrder[] } =
    { hook: null, saved: [], kept: [], orders: [] };
  const save = (ui: UiState, order: UiOrder) => {
    out.saved.push([ui.slide, ui.grid, ui.ghosts]);
    out.kept.push(ui);
    out.orders.push(order);
    return answer();
  };
  function Probe({ ui, current, tabs }: { ui: UiState | null; current: string; tabs: Tabs }) {
    out.hook = useKeptUi(ui, current, save, tabs, timers);
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

test("useKeptUi first snapshot restores the choices kept by the host", async () => {
  const { out } = await mount();
  expect([out.hook!.grid, out.hook!.ghosts]).toEqual([false, true]);
});

test("useKeptUi later snapshot keeps the user's choice", async () => {
  const { out, render } = await mount();
  await act(async () => { out.hook!.handleKey(key("'", { meta: true })); });
  await render({ ...kept, grid: false });
  expect(out.hook!.grid).toBe(true);
});

test("useKeptUi saves nothing before the first snapshot", async () => {
  const { out } = await mount(null);
  expect(out.saved).toEqual([]);
});

test("useKeptUi first snapshot after an empty mount restores the kept choices and saves them once", async () => {
  const { out, render } = await mount(null);
  await render(kept);
  expect([out.hook!.grid, out.hook!.ghosts, out.saved]).toEqual([false, true, [["s1", false, true]]]);
});

test("useKeptUi saves again when the current slide changes", async () => {
  const { out, render } = await mount();
  await render(kept, "s2");
  expect(out.saved.at(-1)).toEqual(["s2", false, true]);
});

test("useKeptUi saves the current slide and choices once restored and on each toggle", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.handleKey(key("g", { meta: true, shift: true })); });
  expect(out.saved).toEqual([["s1", false, true], ["s1", false, false]]);
});

test("useKeptUi menu entries name the toggle to come and switch it", async () => {
  const { out } = await mount();
  expect(labels(out.hook!)).toEqual(["ui-toolbar-grid-show ⌘'", "ui-toolbar-ghosts-hide ⌘⇧G"]);
  await act(async () => { (out.hook!.entries[0] as { onSelect: () => void }).onSelect(); });
  expect(labels(out.hook!)).toEqual(["ui-toolbar-grid-hide ⌘'", "ui-toolbar-ghosts-hide ⌘⇧G"]);
});

test("useKeptUi handleKey takes its shortcut and prevents the default", async () => {
  const { out } = await mount();
  const e = key("G", { meta: true, shift: true });
  let handled = false;
  await act(async () => { handled = out.hook!.handleKey(e); });
  expect([handled, e.prevented, out.hook!.ghosts]).toEqual([true, true, false]);
});

test("useKeptUi first snapshot restores the column widths kept by the host", async () => {
  const { out } = await mount();
  expect(out.hook!.columns).toEqual({ pile: 200, notes: null });
});

test("useKeptUi setColumns saves the new widths with the rest of the state", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setColumns({ pile: null, notes: 540 }); });
  expect(out.kept.at(-1)).toEqual({ slide: "s1", grid: false, ghosts: true, pile: null, notes: 540, widgets: [], widget: null, code: true,
    code_width: 640, band: 320, folded: ["widgets"] });
});

test("useKeptUi premier snapshot reprend le code source déplié gardé par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.code).toBe(true);
});

test("useKeptUi setCode sauve le code replié avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCode(false); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", code: false });
});

test("useKeptUi premier snapshot reprend la largeur du code gardée par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.codeWidth).toBe(640);
});

test("useKeptUi setCodeWidth sauve la largeur du code avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCodeWidth(320); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", code: true, code_width: 320 });
});

test("useKeptUi premier snapshot reprend la hauteur de la bande gardée par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.band).toBe(320);
});

test("useKeptUi setBand sauve la hauteur de la bande avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setBand(400); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", code_width: 640, band: 400 });
});

test("useKeptUi setBand(null) sauve la hauteur par défaut", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setBand(null); });
  expect([out.hook!.band, out.kept.at(-1)?.band]).toEqual([null, null]);
});

test("useKeptUi setCodeWidth(null) sauve la proportion par défaut", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setCodeWidth(null); });
  expect([out.hook!.codeWidth, out.kept.at(-1)?.code_width]).toEqual([null, null]);
});

test("useKeptUi premier snapshot reprend les sections repliées gardées par l'hôte", async () => {
  const { out } = await mount();
  expect(out.hook!.folded).toEqual(["widgets"]);
});

test("useKeptUi setFolded replie une section et la sauve avec le reste", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setFolded("templates", true); });
  expect(out.kept.at(-1)).toMatchObject({ slide: "s1", folded: ["widgets", "templates"] });
});

test("useKeptUi setFolded rouvre une section repliée", async () => {
  const { out } = await mount();
  await act(async () => { out.hook!.setFolded("widgets", false); });
  expect(out.kept.at(-1)).toMatchObject({ folded: [] });
});

test("useKeptUi setFolded sur l'état déjà en place ne sauve rien de plus", async () => {
  const { out } = await mount();
  const before = out.kept.length;
  await act(async () => { out.hook!.setFolded("widgets", true); });
  expect(out.kept.length).toBe(before);
});

test("useKeptUi sauve les sous-onglets passés avec le reste", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: "kpi" });
  expect(out.kept.at(-1)).toMatchObject({ widgets: ["kpi"], widget: "kpi" });
});

test("useKeptUi avec une nouvelle liste de même contenu ne sauve pas de nouveau", async () => {
  const { out, render } = await mount();
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  const n = out.saved.length;
  await render(kept, "s1", { widgets: ["kpi"], widget: null });
  expect(out.saved.length).toBe(n);
});

test("useKeptUi ne renvoie pas à l'hôte un cas courant d'un état plus ancien", async () => {
  const older = { ...kept, case: { kpi: "Long" } } as UiState;
  const { out, render } = await mount(older);
  await render(older, "s2");
  expect(out.kept.at(-1)).not.toHaveProperty("case");
});

/** Réponses de l'hôte tenues en attente : `settle(i, ok)` répond à la i-ème sauvegarde. */
function held() {
  const answers: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const answer = () => new Promise<void>((resolve, reject) => { answers.push({ resolve, reject }); });
  const settle = (i: number, ok = true) =>
    act(async () => { if (ok) answers[i].resolve(); else answers[i].reject(new Error("hôte parti")); });
  return { answer, settle };
}

// Rien n'attend côté page : une sauvegarde gardée en attente se perdait à la fermeture (⌘Q) (t-674).
test("useKeptUi envoie deux changements rapprochés sans attendre la réponse au premier, dans l'ordre", async () => {
  const { answer } = held();
  const { out } = await mount(kept, answer);
  await act(async () => { out.hook!.setCode(false); });
  await act(async () => { out.hook!.setCodeWidth(320); });
  expect(out.kept.length).toBe(3);
  expect(out.kept.at(-1)).toMatchObject({ code: false, code_width: 320 });
  const [a, b, c] = out.orders;
  expect([b.page, c.page]).toEqual([a.page, a.page]);
  expect(a.seq < b.seq && b.seq < c.seq).toBe(true);
});

// Des requêtes parallèles peuvent arriver à l'hôte dans le désordre : la dernière largeur porte le
// plus grand numéro, l'hôte ignore donc une plus ancienne traitée après (règle testée côté session, t-668).
test("useKeptUi donne à la dernière largeur le plus grand numéro", async () => {
  const { answer } = held();
  const { out } = await mount(kept, answer);
  await act(async () => { out.hook!.setCodeWidth(320); });
  await act(async () => { out.hook!.setCodeWidth(480); });
  const last = out.orders.reduce((a, b) => (b.seq > a.seq ? b : a));
  expect(out.kept[out.orders.indexOf(last)]).toMatchObject({ code_width: 480 });
});

// Le compteur est celui du module : il survit au remontage d'`App` d'un onglet à l'autre.
test("useKeptUi continue la numérotation après un remontage", async () => {
  const first = await mount();
  const before = first.out.orders.at(-1)!;
  const m = mounted!;
  mounted = null;
  await act(async () => m.root.unmount());
  m.host.remove();
  const second = await mount();
  const after = second.out.orders.at(-1)!;
  expect([after.page, after.seq > before.seq]).toEqual([before.page, true]);
});

test("useKeptUi après une sauvegarde refusée envoie encore la suivante", async () => {
  const { answer, settle } = held();
  const { out } = await mount(kept, answer);
  await act(async () => { out.hook!.setCodeWidth(320); });
  await settle(0, false);
  expect(out.kept.at(-1)).toMatchObject({ code_width: 320 });
});

// Une sauvegarde perdue (requête échouée) laissait à l'hôte l'ancien état jusqu'au changement suivant (t-749).
test("useKeptUi renvoie une sauvegarde refusée une fois, après un délai, avec le même ordre", async () => {
  const { answer, settle } = held();
  const timers = manualTimers();
  const { out } = await mount(kept, answer, timers);
  await act(async () => { out.hook!.setColumns({ pile: 200, notes: null }); });
  await settle(1, false);
  expect(out.kept.length).toBe(2);
  await act(async () => timers.run());
  expect(out.kept.length).toBe(3);
  expect([out.kept[2], out.orders[2]]).toEqual([out.kept[1], out.orders[1]]);
  expect(timers.delays).toEqual([UI_RESEND_DELAY_MS]);
});

test("useKeptUi ne renvoie pas une sauvegarde refusée si une plus récente est partie", async () => {
  const { answer, settle } = held();
  const timers = manualTimers();
  const { out } = await mount(kept, answer, timers);
  await act(async () => { out.hook!.setCodeWidth(320); });
  await settle(1, false);
  await act(async () => { out.hook!.setCodeWidth(480); });
  await act(async () => timers.run());
  expect(out.kept.length).toBe(3);
  expect(out.kept.at(-1)).toMatchObject({ code_width: 480 });
});

test("useKeptUi ne renvoie qu'une fois une sauvegarde dont le renvoi échoue aussi", async () => {
  const { answer, settle } = held();
  const timers = manualTimers();
  const { out } = await mount(kept, answer, timers);
  await act(async () => { out.hook!.setCodeWidth(320); });
  await settle(1, false);
  await act(async () => timers.run());
  expect(out.kept.length).toBe(3);
  await settle(2, false);
  await act(async () => timers.run());
  expect([out.kept.length, timers.delays.length]).toEqual([3, 1]);
});

test("useKeptUi handleKey leaves other keys alone", async () => {
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
