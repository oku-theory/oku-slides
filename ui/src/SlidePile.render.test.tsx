// Rendu de la pile des miniatures dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { SlideSummary } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: SlidePile } = await import("./SlidePile");
const { expectIconOnly } = await import("./test/iconOnly");

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

async function mount({ canRemove = true, readOnly = false } = {}) {
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
        addLabel: "Nouvelle slide",
        removeLabel: "Supprimer la slide",
        canRemove,
        onSelect: (id: string) => selected.push(id),
        onMove: readOnly ? undefined : () => {},
        onAdd: readOnly ? undefined : () => {},
        onRemove: readOnly ? undefined : (id: string) => removed.push(id),
      } as never),
    );
  });
  const removes = () => [...host.querySelectorAll(".thumb-remove")] as HTMLButtonElement[];
  return { removes, removed, selected, host };
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

  test("SlidePile add button shows an icon only, named by its label", async () => {
    const { host } = await mount();
    expectIconOnly(host.querySelector(".add-slide"), "Nouvelle slide");
  });

  test("SlidePile add button is a round raised token", async () => {
    const { host } = await mount();
    expect(host.querySelector(".add-slide")?.classList.contains("token")).toBe(true);
  });

  test("SlidePile lecture seule (historique) : ni suppression, ni ajout", async () => {
    const { removes, host } = await mount({ readOnly: true });
    expect(removes().length).toBe(0);
    expect(host.querySelector(".add-slide")).toBeNull();
  });
});

describe("pile des miniatures : défilement", () => {
  test("SlidePile changer de slide courante fait défiler sa miniature en vue", async () => {
    const scrolled: { id: string | null; options: unknown }[] = [];
    const original = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = function (this: HTMLElement, options?: unknown) {
      scrolled.push({ id: this.closest(".thumb-slot")?.querySelector(".thumb-img")?.getAttribute("alt") ?? null, options });
    };
    try {
      const host = document.createElement("div");
      document.body.append(host);
      const root = createRoot(host);
      mounted = { root, host };
      const render = (current: string) =>
        createElement(SlidePile, {
          slides: [slide("a"), slide("b")], current, svg: () => "x.svg", label: "Slides", addLabel: "+",
          removeLabel: "Supprimer", canRemove: true, onSelect: () => {},
        } as never);
      await act(async () => root.render(render("a")));
      scrolled.length = 0;
      await act(async () => root.render(render("b")));
      expect(scrolled).toEqual([{ id: "b", options: { block: "nearest" } }]);
    } finally {
      HTMLElement.prototype.scrollIntoView = original;
    }
  });
});

describe("pile des miniatures : bande horizontale", () => {
  const strip = (current = "a") =>
    createElement(SlidePile, {
      slides: [slide("a"), slide("b")], current, svg: () => "x.svg", label: "Slides", addLabel: "+",
      removeLabel: "Supprimer", canRemove: true, onSelect: () => {}, axis: "x",
    } as never);

  async function mountStrip() {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    await act(async () => root.render(strip()));
    return { host, root };
  }

  test("SlidePile axis x marks the pile horizontal", async () => {
    const { host } = await mountStrip();
    expect(host.querySelector("nav.pile")?.classList.contains("horizontal")).toBe(true);
  });

  test("SlidePile axis x scrolls the current thumb into view along the strip", async () => {
    const scrolled: unknown[] = [];
    const original = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = (options?: unknown) => void scrolled.push(options);
    try {
      const { root } = await mountStrip();
      scrolled.length = 0;
      await act(async () => root.render(strip("b")));
      expect(scrolled).toEqual([{ block: "nearest", inline: "nearest" }]);
    } finally {
      HTMLElement.prototype.scrollIntoView = original;
    }
  });
});
