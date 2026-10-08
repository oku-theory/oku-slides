// Rendu des boutons slide précédente / suivante dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: SlideNav } = await import("./SlideNav");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const slides = [{ id: "a" }, { id: "b" }, { id: "c" }];

async function mount(current: string, shown: string[] = []) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(SlideNav, { slides, current, onShow: (id: string) => shown.push(id) })));
  return {
    previous: host.querySelector<HTMLButtonElement>(".slide-nav.previous")!,
    next: host.querySelector<HTMLButtonElement>(".slide-nav.next")!,
  };
}

describe("SlideNav", () => {
  test("SlideNav on the first slide disables previous", async () => {
    const { previous, next } = await mount("a");
    expect(previous.disabled).toBe(true);
    expect(next.disabled).toBe(false);
  });

  test("SlideNav on the last slide disables next", async () => {
    const { next } = await mount("c");
    expect(next.disabled).toBe(true);
  });

  test("SlideNav buttons show the neighbouring slides", async () => {
    const shown: string[] = [];
    const { previous, next } = await mount("b", shown);
    await act(async () => previous.click());
    await act(async () => next.click());
    expect(shown).toEqual(["a", "c"]);
  });
});
