// Rendu du mode présentation dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import { manualTimers } from "./test/timers";
import type { Screen } from "./fullscreen";
import type { PresentMode } from "./present";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Presenter } = await import("./Presenter");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const slides = [{ id: "a", hash: "ha" }, { id: "b", hash: "hb" }, { id: "c", hash: "hc" }];

/** Un écran à la main : `enter` dit s'il était déjà plein, `left()` simule une sortie par le système. */
function fakeScreen(wasFull = false) {
  const calls: string[] = [];
  let leftCb: (() => void) | null = null;
  const screen: Screen = {
    enter: async () => {
      calls.push("enter");
      return !wasFull;
    },
    leave: async () => void calls.push("leave"),
    onLeft: (cb) => {
      leftCb = cb;
      return () => (leftCb = null);
    },
  };
  return { screen, calls, left: () => leftCb?.() };
}

async function mount({ current = "a", mode = "window" as PresentMode, screen = fakeScreen().screen } = {}) {
  const shown: string[] = [];
  const asked: string[] = [];
  const exits: number[] = [];
  const timers = manualTimers();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(Presenter, {
        mode,
        slides,
        current,
        svg: (id: string, hash: string) => {
          asked.push(id);
          return `blob:${hash}`;
        },
        onShow: (id: string) => shown.push(id),
        onExit: () => exits.push(1),
        screen,
        timers,
      }),
    ),
  );
  const root$ = host.querySelector<HTMLElement>(".presenter")!;
  return { host, root: root$, shown, asked, exits, timers };
}

/** Laisse finir les entrées et sorties du plein écran, chaînées l'une après l'autre. */
const settle = () => act(async () => void (await new Promise((r) => setTimeout(r, 0))));

const press = (key: string, init: KeyboardEventInit = {}) =>
  act(async () => void document.body.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init })));

describe("Presenter", () => {
  test("Presenter shows the current slide image", async () => {
    const { root } = await mount({ current: "b" });
    expect(root.querySelector("img")?.getAttribute("src")).toBe("blob:hb");
  });

  test("Presenter asks the next slide image ahead", async () => {
    const { asked } = await mount({ current: "a" });
    expect(asked).toContain("b");
  });

  test("Presenter space shows the next slide", async () => {
    const { shown } = await mount({ current: "a" });
    await press(" ");
    expect(shown).toEqual(["b"]);
  });

  test("Presenter arrow left shows the previous slide", async () => {
    const { shown } = await mount({ current: "b" });
    await press("ArrowLeft");
    expect(shown).toEqual(["a"]);
  });

  test("Presenter click shows the next slide", async () => {
    const { root, shown } = await mount({ current: "b" });
    await act(async () => root.click());
    expect(shown).toEqual(["c"]);
  });

  test("Presenter escape exits", async () => {
    const { exits } = await mount();
    await press("Escape");
    expect(exits).toEqual([1]);
  });

  test("Presenter keeps its keys from later window listeners", async () => {
    const seen: string[] = [];
    const later = (e: KeyboardEvent) => seen.push(e.key);
    await mount();
    window.addEventListener("keydown", later);
    try {
      await press("Escape");
      await press(" ");
      await press("ArrowRight", { metaKey: true });
    } finally {
      window.removeEventListener("keydown", later);
    }
    expect(seen).toEqual(["ArrowRight"]);
  });

  test("Presenter hides the cursor after idle time", async () => {
    const { root, timers } = await mount();
    await act(async () => timers.run());
    expect(root.classList.contains("idle")).toBe(true);
  });

  test("Presenter shows the cursor again on mouse move", async () => {
    const { root, timers } = await mount();
    await act(async () => timers.run());
    await act(async () => void root.dispatchEvent(new MouseEvent("mousemove", { bubbles: true })));
    expect(root.classList.contains("idle")).toBe(false);
  });

  test("Presenter in window mode leaves the screen alone", async () => {
    const fake = fakeScreen();
    await mount({ mode: "window", screen: fake.screen });
    expect(fake.calls).toEqual([]);
  });

  test("Presenter in screen mode enters then leaves full screen", async () => {
    const fake = fakeScreen();
    await mount({ mode: "screen", screen: fake.screen });
    const m = mounted!;
    await act(async () => m.root.unmount());
    mounted = null;
    m.host.remove();
    await settle();
    expect(fake.calls).toEqual(["enter", "leave"]);
  });

  test("Presenter in screen mode keeps a full screen it did not make", async () => {
    const fake = fakeScreen(true);
    await mount({ mode: "screen", screen: fake.screen });
    const m = mounted!;
    await act(async () => m.root.unmount());
    mounted = null;
    m.host.remove();
    await settle();
    expect(fake.calls).toEqual(["enter"]);
  });

  test("Presenter exits when the system leaves full screen", async () => {
    const fake = fakeScreen();
    const { exits } = await mount({ mode: "screen", screen: fake.screen });
    await act(async () => fake.left());
    expect(exits).toEqual([1]);
  });

  test("Presenter after a full screen request that never answers enters again", async () => {
    const calls: string[] = [];
    const hanging: Screen = {
      enter: () => (calls.push("enter"), new Promise<boolean>(() => {})),
      leave: async () => void calls.push("leave"),
      onLeft: () => () => {},
    };
    await mount({ mode: "screen", screen: hanging });
    const m = mounted!;
    await act(async () => m.root.unmount());
    mounted = null;
    m.host.remove();
    await mount({ mode: "screen", screen: hanging });
    await act(async () => void (await new Promise((r) => setTimeout(r, 3200))));
    expect(calls).toEqual(["enter", "enter"]);
  }, 10000);
});
