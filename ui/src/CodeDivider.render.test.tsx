// Rendu du séparateur entre l'aperçu d'un widget et son code, dans un DOM happy-dom.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { CodeAxis } from "./codeSplit";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: CodeDivider } = await import("./CodeDivider");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(axis: CodeAxis) {
  const resized: [number, boolean][] = [];
  let resets = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(CodeDivider, {
        axis,
        label: "Taille du code",
        measure: () => ({ axis, total: 1000, size: 400 }),
        onResize: (size: number, done: boolean) => resized.push([size, done]),
        onReset: () => resets++,
      }),
    );
  });
  const el = host.querySelector<HTMLElement>("[role=separator]")!;
  el.setPointerCapture = () => {};
  const pointer = (type: string, at: number) =>
    act(async () => {
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: at, clientY: at }));
    });
  const press = (key: string) =>
    act(async () => {
      el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key }));
    });
  return { el, resized, resets: () => resets, pointer, press };
}

test("CodeDivider code below is a horizontal named separator", async () => {
  const { el } = await mount("below");
  expect([el.getAttribute("aria-orientation"), el.getAttribute("aria-label"), el.dataset.axis]).toEqual([
    "horizontal",
    "Taille du code",
    "below",
  ]);
});

test("CodeDivider code right is a vertical separator", async () => {
  const { el } = await mount("right");
  expect(el.getAttribute("aria-orientation")).toBe("vertical");
});

test("CodeDivider dragging up grows the code below, live then done on release", async () => {
  const { resized, pointer } = await mount("below");
  await pointer("pointerdown", 500);
  await pointer("pointermove", 460);
  await pointer("pointerup", 450);
  expect(resized).toEqual([
    [440, false],
    [450, true],
  ]);
});

test("CodeDivider dragging left grows the code on the right", async () => {
  const { resized, pointer } = await mount("right");
  await pointer("pointerdown", 500);
  await pointer("pointerup", 400);
  expect(resized).toEqual([[500, true]]);
});

test("CodeDivider arrow up grows the code below by one step", async () => {
  const { resized, press } = await mount("below");
  await press("ArrowUp");
  expect(resized).toEqual([[416, true]]);
});

test("CodeDivider arrow right shrinks the code on the right by one step", async () => {
  const { resized, press } = await mount("right");
  await press("ArrowRight");
  expect(resized).toEqual([[384, true]]);
});

test("CodeDivider arrows across the axis do nothing", async () => {
  const { resized, press } = await mount("below");
  await press("ArrowLeft");
  expect(resized).toEqual([]);
});

test("CodeDivider double click resets the size", async () => {
  const { el, resets } = await mount("right");
  await act(async () => {
    el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });
  expect(resets()).toBe(1);
});
