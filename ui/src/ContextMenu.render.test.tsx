// The shared context menu: in body, its first entry focused, closed by Escape or a click away.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const ContextMenu = (await import("./ContextMenu")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** A menu of two entries opened from a focused `opener`; onClose counts and unmounts it, as callers do. */
async function render(disabled = false) {
  const calls = { closed: 0, picked: 0 };
  const host = document.createElement("div");
  document.body.append(host);
  const opener = document.createElement("button");
  host.append(opener);
  opener.focus();
  const inner = document.createElement("div");
  host.append(inner);
  const root = createRoot(inner);
  mounted = { root, host };
  function Host() {
    const [open, setOpen] = useState(true);
    const onClose = () => {
      calls.closed++;
      setOpen(false);
    };
    return open
      ? createElement(
          ContextMenu,
          { x: 40, y: 50, above: false, label: "Actions", onClose },
          createElement("button", { role: "menuitem", disabled, onClick: () => calls.picked++ }, "One"),
          createElement("button", { role: "menuitem", disabled }, "Two"),
        )
      : null;
  }
  await act(async () => root.render(createElement(Host)));
  const menu = document.querySelector('[role="menu"]') as HTMLElement;
  return { calls, menu, opener, entries: [...menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')] };
}

const key = (target: EventTarget, k: string) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })));

test("ContextMenu renders in body, named by its label", async () => {
  const { menu } = await render();
  expect([menu.parentElement === document.body, menu.getAttribute("aria-label")]).toEqual([true, "Actions"]);
});

test("ContextMenu focuses its first entry", async () => {
  const { entries } = await render();
  expect(document.activeElement).toBe(entries[0]);
});

test("ContextMenu ArrowDown moves to the next entry, wrapping", async () => {
  const { entries } = await render();
  await key(entries[0], "ArrowDown");
  await key(entries[1], "ArrowDown");
  expect(document.activeElement).toBe(entries[0]);
});

test("ContextMenu Escape closes it and gives the focus back", async () => {
  const { calls, entries, opener } = await render();
  await key(entries[0], "Escape");
  expect([calls.closed, document.activeElement === opener]).toEqual([1, true]);
});

test("ContextMenu keys inside it never reach the window", async () => {
  const { entries } = await render();
  const seen: string[] = [];
  const spy = (e: KeyboardEvent) => seen.push(e.key);
  window.addEventListener("keydown", spy);
  await key(entries[0], "Delete");
  await key(entries[0], "Escape");
  window.removeEventListener("keydown", spy);
  expect(seen).toEqual([]);
});

test("ContextMenu with every entry disabled takes the focus itself", async () => {
  const { menu } = await render(true);
  expect(document.activeElement).toBe(menu);
});

test("ContextMenu with every entry disabled still closes on Escape", async () => {
  const { calls, menu } = await render(true);
  await key(menu, "Escape");
  expect(calls.closed).toBe(1);
});

test("ContextMenu Tab closes it", async () => {
  const { calls, entries } = await render();
  await key(entries[0], "Tab");
  expect(calls.closed).toBe(1);
});

test("ContextMenu a pointer down outside closes it", async () => {
  const { calls } = await render();
  await act(async () => void document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
  expect(calls.closed).toBe(1);
});

test("ContextMenu a pointer down inside keeps it", async () => {
  const { calls, entries } = await render();
  await act(async () => void entries[1].dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
  expect(calls.closed).toBe(0);
});
