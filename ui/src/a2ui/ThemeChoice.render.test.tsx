// Rendu et survol d'une carte de thème du sélecteur de l'hôte, dans un DOM happy-dom.

import { describe, expect, test } from "bun:test";
import type { ReactNode } from "react";
import "../test/dom";

// Imports dynamiques après `../test/dom` : react-dom cherche le DOM à son chargement.
const { act, createElement, StrictMode, useState } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { StagePreviewContext, createStagePreviewStore } = await import("./stagePreview");
const { svgSrc } = await import("./svgSrc");

const card = {
  component: "ThemeChoice",
  svg: "<svg><script>alert(1)</script></svg>",
  name: "Papier",
  slide: "s-1",
  swatches: ["#ffffff", "#2f5fb3"],
  active: true,
  action: { event: { name: "theme", context: { theme: "papier" } } },
};
const other = { ...card, svg: "<svg>pro</svg>", name: "Pro", active: false, action: { event: { name: "theme", context: { theme: "pro" } } } };

type Store = ReturnType<typeof createStagePreviewStore>;

/** Monte des composants avec un magasin d'aperçu ; `around` enveloppe la surface (StrictMode…). */
async function mountWith(components: Record<string, unknown>[], around: (n: ReactNode) => ReactNode = (n) => n) {
  const store: Store = createStagePreviewStore();
  const mounted = await mountSurface(messagesOf(components, {}), (s) =>
    createElement(StagePreviewContext.Provider, { value: store }, around(s)),
  );
  const buttons = [...mounted.host.querySelectorAll("button.theme-choice")] as HTMLButtonElement[];
  return { ...mounted, store, buttons };
}
const mount = (root: Record<string, unknown> = card) => mountWith([{ id: "root", ...root }]);
const two = () =>
  mountWith([{ id: "root", component: "Row", children: ["a", "b"] }, { id: "a", ...card }, { id: "b", ...other }]);

/** Survol au sens de React 19 : `pointerover`/`pointerout` délégués, pas `pointerenter`. */
const hover = (el: Element) =>
  act(async () => el.dispatchEvent(new PointerEvent("pointerover", { bubbles: true, relatedTarget: document.body })));
const leave = (el: Element) =>
  act(async () => el.dispatchEvent(new PointerEvent("pointerout", { bubbles: true, relatedTarget: document.body })));

describe("ThemeChoice", () => {
  test("la miniature est une image, jamais un SVG en ligne", async () => {
    const { host, buttons } = await mount();
    expect(buttons[0].querySelector("img")?.getAttribute("src")).toBe(svgSrc(card.svg));
    expect(host.querySelector("script, svg, object, iframe")).toBeNull();
  });

  test("le thème actif est marqué aria-pressed", async () => {
    const { buttons } = await mount();
    expect(buttons[0].getAttribute("aria-pressed")).toBe("true");
  });

  test("les pastilles suivent l'ordre reçu", async () => {
    const { host } = await mount();
    const colours = [...host.querySelectorAll(".theme-choice-swatches .a2ui-swatch")].map((e) => (e as HTMLElement).style.background);
    expect(colours).toEqual(["#ffffff", "#2f5fb3"]);
  });

  test("le nom est isolé dans un bdi", async () => {
    const { buttons } = await mount();
    expect(buttons[0].querySelector("bdi.caption")?.textContent).toBe("Papier");
  });

  test("un clic envoie theme avec l'espace de noms du thème", async () => {
    const { buttons, actions } = await mount();
    await act(async () => buttons[0].click());
    expect(actions.map((a) => [a.name, a.context])).toEqual([["theme", { theme: "papier" }]]);
  });

  test("le survol pose l'aperçu de sa slide", async () => {
    const { buttons, store } = await mount();
    await hover(buttons[0]);
    expect(store.get()).toEqual({ slide: "s-1", url: svgSrc(card.svg) });
  });

  test("quitter la carte retire son aperçu", async () => {
    const { buttons, store } = await mount();
    await hover(buttons[0]);
    await leave(buttons[0]);
    expect(store.get()).toBeNull();
  });

  test("le focus pose l'aperçu", async () => {
    const { buttons, store } = await mount();
    await act(async () => buttons[0].focus());
    expect(store.get()?.url).toBe(svgSrc(card.svg));
  });

  test("la perte du focus retire l'aperçu", async () => {
    const { buttons, store } = await mount();
    await act(async () => buttons[0].focus());
    await act(async () => buttons[0].blur());
    expect(store.get()).toBeNull();
  });

  test("quitter une carte ne retire pas l'aperçu qu'une autre a posé depuis", async () => {
    const { buttons, store } = await two();
    await hover(buttons[1]);
    await act(async () => buttons[0].focus());
    await leave(buttons[1]);
    expect(store.get()?.url).toBe(svgSrc(card.svg));
  });

  test("démontée pendant le survol, la carte retire son aperçu", async () => {
    let hideSurface = () => {};
    function Shown({ child }: { child: ReactNode }) {
      const [on, setOn] = useState(true);
      hideSurface = () => setOn(false);
      return on ? child : null;
    }
    const { buttons, store } = await mountWith([{ id: "root", ...card }], (n) => createElement(Shown, { child: n }));
    await hover(buttons[0]);
    await act(async () => hideSurface());
    expect(store.get()).toBeNull();
  });

  test("sans miniature, aucun aperçu", async () => {
    const { svg: _svg, ...noSvg } = card;
    const { buttons, store } = await mount(noSvg);
    await hover(buttons[0]);
    expect(store.get()).toBeNull();
  });

  test("sous StrictMode, le survol pose encore l'aperçu", async () => {
    const { buttons, store } = await mountWith([{ id: "root", ...card }], (n) => createElement(StrictMode, null, n));
    await hover(buttons[0]);
    expect(store.get()?.slide).toBe("s-1");
  });

  test("une pastille qui n'est pas une couleur fait refuser la surface", async () => {
    const { errors, buttons } = await mount({ ...card, swatches: ["red"] });
    expect([errors.length > 0, buttons.length]).toEqual([true, 0]);
  });
});
