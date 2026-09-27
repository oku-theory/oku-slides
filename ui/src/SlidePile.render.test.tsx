// Rendu de la pile des miniatures dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { SlideSummary } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: SlidePile } = await import("./SlidePile");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const slide = (id: string) => ({ id, hash: id, title: id }) as unknown as SlideSummary;

async function mount({ canRemove = true } = {}) {
  const removed: string[] = [];
  const selected: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(SlidePile, {
        slides: [slide("a"), slide("b")],
        current: "a",
        svg: () => null,
        label: "Slides",
        addLabel: "+",
        removeLabel: "Supprimer la slide",
        canRemove,
        onSelect: (id: string) => selected.push(id),
        onMove: () => {},
        onAdd: () => {},
        onRemove: (id: string) => removed.push(id),
      }),
    );
  });
  const removes = () => [...host.querySelectorAll(".thumb-remove")] as HTMLButtonElement[];
  return { removes, removed, selected };
}

describe("pile des miniatures", () => {
  test("SlidePile chaque miniature a son bouton de suppression", async () => {
    const { removes } = await mount();
    expect(removes().length).toBe(2);
    expect(removes()[1].getAttribute("aria-label")).toBe("Supprimer la slide");
  });

  test("SlidePile supprimer une miniature supprime cette slide, sans la choisir", async () => {
    const { removes, removed, selected } = await mount();
    await act(async () => removes()[1].click());
    expect(removed).toEqual(["b"]);
    expect(selected).toEqual([]);
  });

  test("SlidePile dernière slide : suppression impossible", async () => {
    const { removes } = await mount({ canRemove: false });
    expect(removes().every((b) => b.disabled)).toBe(true);
  });
});
