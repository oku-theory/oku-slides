// Rendu du séparateur entre la scène et la bande de réglages dessous, dans un DOM happy-dom.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: BandDivider } = await import("./BandDivider");
const { BAND_MIN, STAGE_MIN } = await import("./bandSplit");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Hauteurs mesurées au début d'un geste ; défaut : 800 px dont 280 à la bande. */
async function mount(measured = { total: 800, size: 280 }) {
  const resized: [number, boolean][] = [];
  let resets = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(BandDivider, {
        label: "Hauteur des réglages",
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
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: 0, clientY: at }));
    });
  const press = (key: string) =>
    act(async () => {
      el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key }));
    });
  const dblclick = () =>
    act(async () => {
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
  return { el, resized, resets: () => resets, pointer, press, dblclick };
}

test("BandDivider is a horizontal named separator", async () => {
  const { el } = await mount();
  expect([el.getAttribute("aria-orientation"), el.getAttribute("aria-label")]).toEqual(["horizontal", "Hauteur des réglages"]);
});

test("BandDivider dragging up grows the band, live then done on release", async () => {
  const { pointer, resized } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointermove", 460);
  await pointer("pointerup", 440);
  expect(resized).toEqual([[320, false], [340, true]]);
});

test("BandDivider dragging down is bounded by the band minimum", async () => {
  const { pointer, resized } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointerup", 900);
  expect(resized).toEqual([[BAND_MIN, true]]);
});

test("BandDivider dragging up is bounded by the stage minimum", async () => {
  const { pointer, resized } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointerup", 0);
  expect(resized).toEqual([[800 - STAGE_MIN, true]]);
});

test("BandDivider a click without movement resizes nothing", async () => {
  const { pointer, resized } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointerup", 500);
  expect(resized).toEqual([]);
});

test("BandDivider interrupted drag keeps the last height, saved once", async () => {
  const { pointer, resized } = await mount();
  await pointer("pointerdown", 500);
  await pointer("pointermove", 450);
  await pointer("pointercancel", 0);
  expect(resized).toEqual([[330, false], [330, true]]);
});

test("BandDivider arrow up grows the band by one step, arrow down shrinks it", async () => {
  const { press, resized } = await mount();
  await press("ArrowUp");
  await press("ArrowDown");
  await press("ArrowLeft");
  expect(resized).toEqual([[296, true], [264, true]]);
});

test("BandDivider double click resets the height", async () => {
  const { dblclick, resets } = await mount();
  await dblclick();
  expect(resets()).toBe(1);
});

test("BandDivider without anything measured ignores drags and keys", async () => {
  const { pointer, press, resized } = await mount({ total: 0, size: 0 });
  await pointer("pointerdown", 500);
  await pointer("pointerup", 400);
  await press("ArrowUp");
  expect(resized).toEqual([]);
});
