// Rendu du sélecteur de layouts dans un DOM happy-dom.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { LayoutView, LayoutsView, Slide } from "./api";
import { ESSENTIEL } from "./builtinKinds";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const LayoutPicker = (await import("./LayoutPicker")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");

afterAll(resetMessages);
setMessages("fr", ["ui-picker-new-slide = Nouvelle slide\nui-picker-close = Fermer\n"]);

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
let now = 0;
beforeEach(() => {
  now = 1_000_000;
  spyOn(Date, "now").mockImplementation(() => now);
});
afterEach(async () => {
  (Date.now as unknown as { mockRestore: () => void }).mockRestore();
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const slideOf = (kind: string): Slide => ({ id: `layout-${kind.split("/")[1]}`, kind, params: {} });
const layout = (kind: string, label: string): LayoutView => ({ kind, label, slide: slideOf(kind), svg: "<svg/>" });
const oneLayout = (kind: string, label: string): LayoutsView => ({
  packs: [{ namespace: "essentiel", name: "Essentiel", layouts: [layout(kind, label)] }],
});

/** Monte le sélecteur sur la réponse `layouts` ; rend les layouts choisis et les fermetures. */
async function mount(layouts: () => Promise<LayoutsView>) {
  const picked: string[] = [];
  let closed = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(LayoutPicker, {
        api: { layouts },
        themeKey: "t",
        onPick: (l: LayoutView) => void picked.push(l.kind),
        onClose: () => void closed++,
      }),
    ),
  );
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  const cards = () => [...host.querySelectorAll<HTMLButtonElement>("button.layout-card")];
  const card = (label: string) => cards().find((b) => b.textContent === label)!;
  return { host, picked, closed: () => closed, cards, card };
}

/** Clique après la garde des 500 ms. */
async function click(el: HTMLElement) {
  now += ARMING_MS + 1;
  await act(async () => el.click());
}

test("LayoutPicker lists each layout pack with its layouts", async () => {
  const { host, cards } = await mount(() =>
    Promise.resolve({
      packs: [
        { namespace: "essentiel", name: "Essentiel", layouts: [layout(ESSENTIEL.title, "Couverture"), layout(ESSENTIEL.content, "Contenu")] },
        { namespace: "acme", name: "Acme", layouts: [layout("acme/kpi", "Indicateurs")] },
      ],
    }),
  );
  expect([...host.querySelectorAll(".layout-pack h3")].map((h) => h.textContent)).toEqual(["Essentiel", "Acme"]);
  expect(cards().map((b) => b.textContent)).toEqual(["Couverture", "Contenu", "Indicateurs"]);
  expect(cards().every((b) => b.querySelector("img") !== null)).toBe(true);
});

test("LayoutPicker picking a card returns its layout", async () => {
  const { picked, card } = await mount(() => Promise.resolve(oneLayout(ESSENTIEL.end, "Fin")));
  await click(card("Fin"));
  expect(picked).toEqual([ESSENTIEL.end]);
});

test("LayoutPicker shows the host refusal", async () => {
  const { host, cards } = await mount(() => Promise.reject(new Error("rendu refusé")));
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("rendu refusé");
  expect(cards()).toHaveLength(0);
});

test("LayoutPicker close button closes", async () => {
  const { host, closed } = await mount(() => Promise.resolve(oneLayout(ESSENTIEL.end, "Fin")));
  await click(host.querySelector<HTMLButtonElement>('button[aria-label="Fermer"]')!);
  expect(closed()).toBe(1);
});

test("LayoutPicker arrow keys move the focus between cards across packs", async () => {
  const { cards } = await mount(() =>
    Promise.resolve({
      packs: [
        { namespace: "essentiel", name: "Essentiel", layouts: [layout(ESSENTIEL.title, "Couverture")] },
        { namespace: "acme", name: "Acme", layouts: [layout("acme/kpi", "Indicateurs")] },
      ],
    }),
  );
  expect(document.activeElement).toBe(cards()[0]!);
  const press = (key: string) =>
    act(async () => void document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
  await press("ArrowRight");
  expect(document.activeElement).toBe(cards()[1]!);
  await press("ArrowRight");
  expect(document.activeElement).toBe(cards()[1]!);
  await press("ArrowLeft");
  expect(document.activeElement).toBe(cards()[0]!);
});
