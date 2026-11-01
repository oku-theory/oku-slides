// The left column of a theme tab (its foldable thumbnails), rendered in a happy-dom DOM.

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

async function mount() {
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
      slides: createElement("nav", { id: "slides" }),
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

  test("PileColumn folding the thumbnails hides them", async () => {
    const host = await mount();
    await fold(host);
    expect(host.querySelector("#slides")).toBeNull();
  });
});
