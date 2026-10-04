// Barre d'outils d'un sous-onglet de widget : historique de l'éditeur, code, place du code, planche.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { ViewOptions } from "./useViewOptions";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: WidgetTools } = await import("./WidgetTools");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

type Options = Pick<ViewOptions, "code" | "setCode" | "codeLayout" | "setCodeLayout" | "board" | "setBoard">;

async function mount(widget: string, code: boolean, history = { undo: 1, redo: 0 }) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  const options: Options = {
    code,
    setCode: (v) => calls.push(`code ${v}`),
    codeLayout: { code_place: "below", code_height: null, code_width: null },
    setCodeLayout: (l) => calls.push(`place ${l.code_place}`),
    board: false,
    setBoard: (v) => calls.push(`board ${v}`),
  };
  await act(async () =>
    root.render(
      createElement(WidgetTools, {
        tab: 3,
        widget,
        history,
        onUndo: () => calls.push("undo"),
        onRedo: () => calls.push("redo"),
        options,
      }),
    ),
  );
  const labels = () => [...host.querySelectorAll("button")].map((b) => b.getAttribute("aria-label"));
  const press = (label: string) => act(async () => host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!.click());
  return { host, calls, labels, press };
}

test("WidgetTools code replié : historique, code et planche, sans place du code", async () => {
  const { labels } = await mount("kpi", false);
  expect(labels()).toEqual(["ui-toolbar-undo", "ui-toolbar-redo", "ui-widgets-source", "ui-widgets-board"]);
});

test("WidgetTools code déplié : la place du code apparaît après lui", async () => {
  const { labels } = await mount("kpi", true);
  expect(labels()).toEqual(["ui-toolbar-undo", "ui-toolbar-redo", "ui-widgets-source", "ui-widgets-code-right", "ui-widgets-board"]);
});

test("WidgetTools sur Commun : pas de planche", async () => {
  const { labels } = await mount("_commun", false);
  expect(labels()).not.toContain("ui-widgets-board");
});

test("WidgetTools rétablir désactivé sans pas à rétablir", async () => {
  const { host } = await mount("kpi", false);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="ui-toolbar-redo"]')!.disabled).toBe(true);
});

test("WidgetTools chaque bouton appelle son action", async () => {
  const { calls, press } = await mount("kpi", true);
  for (const l of ["ui-toolbar-undo", "ui-widgets-source", "ui-widgets-code-right", "ui-widgets-board"]) await press(l);
  expect(calls).toEqual(["undo", "code false", "place right", "board true"]);
});
