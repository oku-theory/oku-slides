// Bouton à bascule du code source d'un widget, à droite d'annuler / rétablir.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const CodeSwitch = (await import("./CodeSwitch")).default;

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
  await act(async () => root.render(createElement(CodeSwitch, { checked, controls: "widget-code-3", onChange: (v: boolean) => asked.push(v) })));
  return { button: host.querySelector<HTMLButtonElement>("button")!, asked };
}

test("CodeSwitch relâché dit aria-pressed=false et désigne le code", async () => {
  const { button } = await mount(false);
  expect([button.getAttribute("aria-pressed"), button.getAttribute("aria-controls")]).toEqual(["false", "widget-code-3"]);
});

test("CodeSwitch est un pictogramme nommé en aria-label et en infobulle", async () => {
  const { button } = await mount(false);
  expect([!!button.querySelector("svg"), button.getAttribute("aria-label"), button.title]).toEqual([true, "ui-widgets-source", "ui-widgets-source"]);
});

test("CodeSwitch enfoncé dit aria-pressed=true et porte la classe on", async () => {
  const { button } = await mount(true);
  expect([button.getAttribute("aria-pressed"), button.classList.contains("on")]).toEqual(["true", true]);
});

test("CodeSwitch cliqué demande l'état inverse", async () => {
  const { button, asked } = await mount(true);
  await act(async () => button.click());
  expect(asked).toEqual([false]);
});
