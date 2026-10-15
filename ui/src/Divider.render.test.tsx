// Rendu du séparateur générique (valeurs ARIA, Début et Fin), dans un DOM happy-dom.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { Measured } from "./Divider";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Divider } = await import("./Divider");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Mesure par défaut : 300 px, bornés à [120, 600]. */
async function mount(measured: Measured | null = { size: 300.4, bounds: { min: 120, max: 600 } }) {
  const resized: [number, boolean][] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(Divider, {
        className: "test-divider",
        label: "Taille",
        axis: "y",
        grow: 1,
        measure: () => measured,
        onResize: (size: number, done: boolean) => resized.push([size, done]),
        onReset: () => {},
      }),
    );
  });
  const el = host.querySelector<HTMLElement>("[role=separator]")!;
  const press = (key: string) =>
    act(async () => {
      el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key }));
    });
  const values = () => ["aria-valuenow", "aria-valuemin", "aria-valuemax"].map((a) => el.getAttribute(a));
  return { el, resized, press, values };
}

test("Divider exposes the measured size and bounds in px", async () => {
  const { values } = await mount();
  expect(values()).toEqual(["300", "120", "600"]);
});

test("Divider aria-valuenow follows a keyboard step", async () => {
  const { press, values } = await mount();
  await press("ArrowDown");
  expect(values()).toEqual(["316", "120", "600"]);
});

test("Divider Home goes to the minimum size, done", async () => {
  const { press, resized } = await mount();
  await press("Home");
  expect(resized).toEqual([[120, true]]);
});

test("Divider End goes to the maximum size, done", async () => {
  const { press, resized } = await mount();
  await press("End");
  expect(resized).toEqual([[600, true]]);
});

test("Divider without anything measured exposes no value and ignores Home and End", async () => {
  const { press, resized, values } = await mount(null);
  await press("Home");
  await press("End");
  expect([values(), resized]).toEqual([[null, null, null], []]);
});
