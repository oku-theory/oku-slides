// Rendu de la marque du mode modèles dans un DOM happy-dom.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { TemplatesBrand } = await import("./TemplatesBrand");
const { t } = await import("./i18n");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(deckOnly: boolean) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(TemplatesBrand, { deckOnly })));
  return host;
}

test("TemplatesBrand of a deck-only pack shows the deck-only note", async () => {
  const host = await mount(true);
  expect(host.querySelector(".brand")?.textContent).toBe(t("ui-brand-templates"));
  expect(host.querySelector(".brand-note")?.textContent).toBe(t("ui-templates-deck-only"));
});

test("TemplatesBrand of a library pack shows no note", async () => {
  const host = await mount(false);
  expect(host.querySelector(".brand")).not.toBeNull();
  expect(host.querySelector(".brand-note")).toBeNull();
});
