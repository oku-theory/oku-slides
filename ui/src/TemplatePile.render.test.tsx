// Rendu de la colonne des modèles (sections Modèles et Widgets) dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: TemplatePile } = await import("./TemplatePile");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(createElement(TemplatePile, {
      title: "Modèles",
      slides: createElement("nav", { id: "slides" }),
      widgets: createElement("section", { id: "widgets" }),
    })),
  );
  return host;
}

describe("TemplatePile", () => {
  test("TemplatePile les miniatures sont dans une section Modèles ouverte", async () => {
    const host = await mount();
    expect(host.querySelector(".pile-templates .pile-section-body #slides")).not.toBeNull();
  });

  test("TemplatePile les widgets suivent la section Modèles", async () => {
    const host = await mount();
    const templates = host.querySelector(".pile-templates") as Element;
    expect(templates.nextElementSibling?.id).toBe("widgets");
  });

  test("TemplatePile replier Modèles garde les widgets", async () => {
    const host = await mount();
    await act(async () => host.querySelector<HTMLButtonElement>(".pile-templates .pile-section-toggle")?.click());
    expect(host.querySelector("#slides")).toBeNull();
    expect(host.querySelector("#widgets")).not.toBeNull();
  });
});
