// Barre d'outils d'un sous-onglet de widget : historique de l'éditeur et code.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { KeptUi } from "./useKeptUi";

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

type Options = Pick<KeptUi, "code" | "setCode">;

async function mount(code: boolean, history = { undo: 1, redo: 0 }) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  const options: Options = {
    code,
    setCode: (v) => calls.push(`code ${v}`),
  };
  await act(async () =>
    root.render(
      createElement(WidgetTools, {
        tab: 3,
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

for (const code of [false, true]) {
  test(`WidgetTools code ${code ? "déplié" : "replié"} : affiche l'historique et le code seulement`, async () => {
    const { labels } = await mount(code);
    expect(labels()).toEqual(["ui-toolbar-undo", "ui-toolbar-redo", "ui-widgets-source"]);
  });
}

test("WidgetTools rétablir désactivé sans pas à rétablir", async () => {
  const { host } = await mount(false);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="ui-toolbar-redo"]')!.disabled).toBe(true);
});

test("WidgetTools chaque bouton appelle son action", async () => {
  const { calls, press } = await mount(true);
  for (const l of ["ui-toolbar-undo", "ui-widgets-source"]) await press(l);
  expect(calls).toEqual(["undo", "code false"]);
});
