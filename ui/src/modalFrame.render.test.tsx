// The modal frame re-arming its click and Escape guard when the window regains focus or visibility.

import { afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";

const { act, createElement, useRef } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ARMING_MS, useModalFrame } = await import("./modalFrame");

let now = 0;
let clicks = 0;
let escapes = 0;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;

function Probe() {
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture } = useModalFrame(ref, { onEscape: () => void escapes++ });
  return createElement(
    "div",
    { ref, tabIndex: -1, onClickCapture },
    createElement("button", { id: "probe", onClick: () => void clicks++ }, "go"),
  );
}

const setVisibility = (state: "visible" | "hidden") =>
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });

beforeEach(async () => {
  now = 1_000;
  clicks = 0;
  escapes = 0;
  spyOn(Date, "now").mockImplementation(() => now);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  now += ARMING_MS + 1;
});
afterEach(async () => {
  (Date.now as unknown as { mockRestore: () => void }).mockRestore();
  setVisibility("visible");
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const click = () => act(async () => void document.getElementById("probe")?.click());
const escape = () =>
  act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));

test("useModalFrame window focus after arming ignores an immediate click", async () => {
  await act(async () => void window.dispatchEvent(new Event("focus")));
  await click();
  expect(clicks).toBe(0);
  now += ARMING_MS + 1;
  await click();
  expect(clicks).toBe(1);
});

test("useModalFrame visibility change to visible ignores an immediate click", async () => {
  setVisibility("visible");
  await act(async () => void document.dispatchEvent(new Event("visibilitychange")));
  await click();
  expect(clicks).toBe(0);
  now += ARMING_MS + 1;
  await click();
  expect(clicks).toBe(1);
});

test("useModalFrame window focus re-arms escape", async () => {
  await act(async () => void window.dispatchEvent(new Event("focus")));
  await escape();
  expect(escapes).toBe(0);
  now += ARMING_MS + 1;
  await escape();
  expect(escapes).toBe(1);
});

test("useModalFrame hidden visibility change keeps the frame armed", async () => {
  setVisibility("hidden");
  await act(async () => void document.dispatchEvent(new Event("visibilitychange")));
  await click();
  expect(clicks).toBe(1);
});
