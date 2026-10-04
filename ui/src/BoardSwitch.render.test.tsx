// Bouton à bascule Scène / Planche d'un sous-onglet de widget, à côté de </>.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const BoardSwitch = (await import("./BoardSwitch")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(checked: boolean) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const asked: boolean[] = [];
  await act(async () => root.render(createElement(BoardSwitch, { checked, controls: "widget-panel-3", onChange: (v: boolean) => asked.push(v) })));
  return { button: host.querySelector<HTMLButtonElement>("button")!, asked };
}

test("BoardSwitch relâché (la scène) dit aria-pressed=false et désigne le panneau", async () => {
  const { button } = await mount(false);
  expect([button.getAttribute("aria-pressed"), button.getAttribute("aria-controls")]).toEqual(["false", "widget-panel-3"]);
});

test("BoardSwitch est un pictogramme nommé en aria-label et en infobulle", async () => {
  const { button } = await mount(false);
  expect([!!button.querySelector("svg"), button.getAttribute("aria-label"), button.title]).toEqual([true, "ui-widgets-board", "ui-widgets-board"]);
});

test("BoardSwitch enfoncé (la planche) dit aria-pressed=true et porte la classe on", async () => {
  const { button } = await mount(true);
  expect([button.getAttribute("aria-pressed"), button.classList.contains("on")]).toEqual(["true", true]);
});

test("BoardSwitch cliqué demande l'état inverse", async () => {
  const { button, asked } = await mount(true);
  await act(async () => button.click());
  expect(asked).toEqual([false]);
});
