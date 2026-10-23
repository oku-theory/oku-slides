// Rendu de la colonne de gauche du mode pack (miniatures et widgets) dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PileColumn } = await import("./PileColumn");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(withSlides = true) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  const Probe = () => {
    const [open, setOpen] = useState(true);
    return createElement(PileColumn, {
      title: "Layouts",
      open,
      onToggle: () => setOpen((o) => !o),
      slides: withSlides ? createElement("nav", { id: "slides" }) : null,
      widgets: createElement("section", { id: "widgets" }),
    });
  };
  await act(async () => root.render(createElement(Probe)));
  return host;
}

const fold = (host: HTMLElement) => act(async () => host.querySelector<HTMLButtonElement>(".pile-thumbs .pile-section-toggle")?.click());

describe("PileColumn", () => {
  test("PileColumn les miniatures sont dans une section ouverte", async () => {
    const host = await mount();
    expect(host.querySelector(".pile-thumbs .pile-section-body #slides")).not.toBeNull();
  });

  test("PileColumn les widgets suivent la section des miniatures", async () => {
    const host = await mount();
    const thumbs = host.querySelector(".pile-thumbs") as Element;
    expect(thumbs.nextElementSibling?.id).toBe("widgets");
  });

  test("PileColumn replier les miniatures garde les widgets", async () => {
    const host = await mount();
    await fold(host);
    expect(host.querySelector("#slides")).toBeNull();
    expect(host.querySelector("#widgets")).not.toBeNull();
  });

  test("PileColumn of a widget pack shows the widgets without a thumbnail section", async () => {
    const host = await mount(false);
    expect([host.querySelector(".pile-thumbs"), !!host.querySelector(".pile-column > #widgets")]).toEqual([null, true]);
  });
});
