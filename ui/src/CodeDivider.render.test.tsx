// Rendu du séparateur entre l'aperçu d'un widget et son code à droite, dans un DOM happy-dom.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: CodeDivider } = await import("./CodeDivider");
const { CODE_MIN, SIDE_MIN, SPLIT } = await import("./codeSplit");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Options {
  /** Largeurs mesurées au début d'un geste ; défaut : 1000 px dont 400 au code. */
  measured?: { total: number; size: number };
  /** De droite à gauche : le code est à gauche de l'aperçu. */
  rtl?: boolean;
}

async function mount({ measured = { total: 1000, size: 400 }, rtl = false }: Options = {}) {
  const resized: [number, boolean][] = [];
  let resets = 0;
  const host = document.createElement("div");
  if (rtl) {
    // happy-dom ne déduit pas `direction` de l'attribut `dir` : le style la pose.
    host.dir = "rtl";
    host.style.direction = "rtl";
  }
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(CodeDivider, {
        label: "Taille du code",
        measure: () => measured,
        onResize: (size: number, done: boolean) => resized.push([size, done]),
        onReset: () => resets++,
      }),
    );
  });
  const el = host.querySelector<HTMLElement>("[role=separator]")!;
  el.setPointerCapture = () => {};
  const pointer = (type: string, at: number) =>
    act(async () => {
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: at, clientY: 0 }));
    });
  const dblclick = () =>
    act(async () => {
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
  const press = (key: string) =>
    act(async () => {
      el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key }));
    });
  return { el, resized, resets: () => resets, pointer, press, dblclick };
}

test("CodeDivider is a vertical named separator", async () => {
  const { el } = await mount();
  expect([el.getAttribute("aria-orientation"), el.getAttribute("aria-label")]).toEqual(["vertical", "Taille du code"]);
});

test("CodeDivider dragging left grows the code, live then done on release", async () => {
  const { resized, pointer } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointermove", 460);
  await pointer("pointerup", 400);
  expect(resized).toEqual([
    [440, false],
    [500, true],
  ]);
});

test("CodeDivider arrow right shrinks the code by one step", async () => {
  const { resized, press } = await mount();
  await press("ArrowRight");
  expect(resized).toEqual([[384, true]]);
});

test("CodeDivider arrow left grows the code by one step", async () => {
  const { resized, press } = await mount();
  await press("ArrowLeft");
  expect(resized).toEqual([[416, true]]);
});

test("CodeDivider ArrowUp and ArrowDown leave the size alone", async () => {
  const { resized, press } = await mount();
  await press("ArrowUp");
  await press("ArrowDown");
  expect(resized).toEqual([]);
});

test("CodeDivider double click resets the size", async () => {
  const { resets, dblclick } = await mount();
  await dblclick();
  expect(resets()).toBe(1);
});

test("CodeDivider a click without movement resizes nothing", async () => {
  const { resized, pointer } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointerup", 500);
  expect(resized).toEqual([]);
});

test("CodeDivider double click sends only the reset, not the clicks before it", async () => {
  const { resized, resets, pointer, dblclick } = await mount();
  for (let i = 0; i < 2; i++) {
    await pointer("pointerdown", 500);
    await pointer("pointerup", 500);
  }
  await dblclick();
  expect([resized, resets()]).toEqual([[], 1]);
});

test("CodeDivider interrupted drag keeps the last width, saved once", async () => {
  const { resized, pointer } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointermove", 460);
  await pointer("pointercancel", 460);
  await pointer("pointerup", 300);
  expect(resized).toEqual([
    [440, false],
    [440, true],
  ]);
});

test("CodeDivider interrupted click without movement resizes nothing", async () => {
  const { resized, pointer } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointercancel", 500);
  expect(resized).toEqual([]);
});

test("CodeDivider arrow right at the code minimum stays there", async () => {
  const { resized, press } = await mount({ measured: { total: 1000, size: CODE_MIN } });
  await press("ArrowRight");
  expect(resized).toEqual([[CODE_MIN, true]]);
});

test("CodeDivider arrow left at the preview minimum stays there", async () => {
  const widest = 1000 - SPLIT - SIDE_MIN;
  const { resized, press } = await mount({ measured: { total: 1000, size: widest } });
  await press("ArrowLeft");
  expect(resized).toEqual([[widest, true]]);
});

test("CodeDivider without a code to measure ignores drags and keys", async () => {
  const { resized, pointer, press } = await mount({ measured: { total: 0, size: 0 } });
  await pointer("pointerdown", 500);
  await pointer("pointermove", 460);
  await pointer("pointerup", 400);
  await press("ArrowLeft");
  expect(resized).toEqual([]);
});

test("CodeDivider right to left: dragging right grows the code on the left", async () => {
  const { resized, pointer } = await mount({ rtl: true });
  await pointer("pointerdown", 500);
  await pointer("pointermove", 540);
  await pointer("pointerup", 600);
  expect(resized).toEqual([
    [440, false],
    [500, true],
  ]);
});

test("CodeDivider right to left: arrow right grows the code, arrow left shrinks it", async () => {
  const { resized, press } = await mount({ rtl: true });
  await press("ArrowRight");
  await press("ArrowLeft");
  expect(resized).toEqual([
    [416, true],
    [384, true],
  ]);
});
