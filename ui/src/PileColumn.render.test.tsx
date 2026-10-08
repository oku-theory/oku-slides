// Rendu de la colonne de gauche (miniatures, et widgets en mode modèles) dans un DOM happy-dom.

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

async function mount(withWidgets = true) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  const Probe = () => {
    const [open, setOpen] = useState(true);
    return createElement(PileColumn, {
      title: "Modèles",
      open,
      onToggle: () => setOpen((o) => !o),
      slides: createElement("nav", { id: "slides" }),
      widgets: withWidgets ? createElement("section", { id: "widgets" }) : undefined,
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

  test("PileColumn sans widgets replier vers la gauche cache les miniatures et marque la colonne", async () => {
    const host = await mount(false);
    await act(async () => host.querySelector<HTMLButtonElement>(".pile-fold")?.click());
    expect(host.querySelector("#slides")).toBeNull();
    expect(host.querySelector(".pile-column")?.getAttribute("data-folded")).toBe("true");
  });

  test("PileColumn sans widgets le bouton de repli dit son état", async () => {
    const host = await mount(false);
    const fold = host.querySelector<HTMLButtonElement>(".pile-fold");
    expect(fold?.getAttribute("aria-expanded")).toBe("true");
    await act(async () => fold?.click());
    expect(fold?.getAttribute("aria-expanded")).toBe("false");
  });
});
