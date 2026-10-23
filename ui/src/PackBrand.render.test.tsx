// Rendu de la marque du mode pack dans un DOM happy-dom.

import { afterAll, afterEach, beforeAll, expect, test } from "bun:test";
import "./test/dom";
import type { PackKind } from "./packTypes";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { PackBrand } = await import("./PackBrand");
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});
beforeAll(async () => {
  const en = await Bun.file(new URL("../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url)).text();
  setMessages("en", [en]);
});
afterAll(resetMessages);

async function mount(kind: PackKind, deckOnly: boolean) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(PackBrand, { name: "Essentiel", kind, deckOnly })));
  return host;
}

test("PackBrand of a layout pack shows its name and kind", async () => {
  const host = await mount("layouts", false);
  expect(host.querySelector(".brand")?.textContent).toBe("Essentiel · layout pack");
});

test("PackBrand of a widget pack shows its name and kind", async () => {
  const host = await mount("widgets", false);
  expect(host.querySelector(".brand")?.textContent).toBe("Essentiel · widget pack");
});

test("PackBrand of a deck-only pack shows the deck-only note", async () => {
  const host = await mount("layouts", true);
  expect(host.querySelector(".brand-note")?.textContent).toBe("Saved in this presentation only");
});

test("PackBrand of a library pack shows no note", async () => {
  const host = await mount("widgets", false);
  expect(host.querySelector(".brand-note")).toBeNull();
});
