// Rendu d'une section repliable de la pile dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PileSection } = await import("./PileSection");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(initial = true) {
  const Probe = () => {
    const [open, setOpen] = useState(initial);
    return createElement(
      PileSection,
      { title: "Widgets", open, onToggle: () => setOpen((o) => !o), actions: createElement("button", { id: "act" }) },
      createElement("p", { id: "body" }),
    );
  };
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  const toggle = host.querySelector<HTMLButtonElement>(".pile-section-toggle") as HTMLButtonElement;
  return { host, toggle };
}

describe("PileSection", () => {
  test("PileSection ouverte montre son contenu et le dit au bouton", async () => {
    const { host, toggle } = await mount();
    expect(host.querySelector("#body")).not.toBeNull();
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
  });

  test("PileSection le bouton du titre replie le contenu", async () => {
    const { host, toggle } = await mount();
    await act(async () => toggle.click());
    expect(host.querySelector("#body")).toBeNull();
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
  });

  test("PileSection repliée porte data-open à faux", async () => {
    const { host } = await mount(false);
    expect(host.querySelector("section")?.getAttribute("data-open")).toBe("false");
  });

  test("PileSection le bouton désigne le corps qu'il replie", async () => {
    const { host, toggle } = await mount();
    const id = toggle.getAttribute("aria-controls") as string;
    expect(host.querySelector(".pile-section-body")?.id).toBe(id);
  });

  test("PileSection les actions restent visibles une fois repliée", async () => {
    const { host } = await mount(false);
    expect(host.querySelector(".pile-section-head #act")).not.toBeNull();
  });

  test("PileSection le titre est porté par le bouton", async () => {
    const { toggle } = await mount();
    expect(toggle.textContent).toBe("Widgets");
  });
});
