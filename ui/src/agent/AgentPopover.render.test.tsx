// An agent box as a popover next to its opener, in a happy-dom DOM.

import "../test/dom";
import { afterEach, describe, expect, test } from "bun:test";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";

// See ../a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement, useRef } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentPopover } = await import("./AgentPopover");
const { ARMING_MS, useModalFrame } = await import("../modalFrame");

const picker = (): Surface => ({ ...real("host-theme-picker"), place: "popover" });

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement; anchor: HTMLElement | null } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
  m.anchor?.remove();
});

/** Mounts a popover on `anchor`, with an opener carrying `withAnchor` (none: `null`). */
async function mount(anchor: string, withAnchor: string | null = anchor, fallback?: string) {
  let closed = 0;
  let opener: HTMLElement | null = null;
  if (withAnchor !== null) {
    opener = document.createElement("button");
    opener.setAttribute("data-popover-anchor", withAnchor);
    document.body.append(opener);
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host, anchor: opener };
  await act(async () =>
    root.render(createElement(AgentPopover, { surface: picker(), anchor, fallback, onAction: () => {}, onError: () => {}, onClose: () => closed++ })),
  );
  // The A2UI view loads lazily: let Suspense resolve it.
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  return { closed: () => closed, opener, box: () => document.querySelector(".agent-popover") };
}

async function dispatch(target: EventTarget, e: Event) {
  await act(async () => {
    target.dispatchEvent(e);
  });
  return e;
}

describe("AgentPopover", () => {
  test("AgentPopover on Escape closes and the key does not reach the app", async () => {
    const { closed, box } = await mount("host-theme-picker");
    let reached = false;
    const app = () => (reached = true);
    window.addEventListener("keydown", app);
    try {
      const e = await dispatch(box()!.querySelector("[data-surface-id]")!, new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
      expect([closed(), e.defaultPrevented, reached]).toEqual([1, true, false]);
    } finally {
      window.removeEventListener("keydown", app);
    }
  });

  test("AgentPopover on an outside pointerdown closes", async () => {
    const { closed } = await mount("host-theme-picker");
    await dispatch(document.body, new PointerEvent("pointerdown", { bubbles: true }));
    expect(closed()).toBe(1);
  });

  test("AgentPopover on a pointerdown inside stays open", async () => {
    const { closed, box } = await mount("host-theme-picker");
    await dispatch(box()!.querySelector("[data-surface-id]")!, new PointerEvent("pointerdown", { bubbles: true }));
    expect(closed()).toBe(0);
  });

  test("AgentPopover on a pointerdown on its anchor stays open", async () => {
    const { closed, opener } = await mount("host-theme-picker");
    await dispatch(opener!, new PointerEvent("pointerdown", { bubbles: true }));
    expect(closed()).toBe(0);
  });

  test("AgentPopover with a quote in its anchor id finds the anchor", async () => {
    const { box } = await mount('host-"x');
    expect(box()).not.toBeNull();
  });

  test("AgentPopover without its anchor renders nothing", async () => {
    const { box } = await mount("host-theme-picker", null);
    expect(box()).toBeNull();
  });

  test("AgentPopover on open focuses inside the box", async () => {
    const { box } = await mount("host-theme-picker");
    expect(box()!.contains(document.activeElement)).toBe(true);
  });

  test("AgentPopover on close gives the focus back to its anchor", async () => {
    const { opener } = await mount("host-theme-picker");
    await act(async () => mounted!.root.render(null));
    expect(document.activeElement).toBe(opener);
  });

  test("AgentPopover without its anchor opens at its fallback", async () => {
    const { box } = await mount("host-theme-picker@chip", "host-theme-picker", "host-theme-picker");
    expect(box()).not.toBeNull();
  });

  test("AgentPopover whose anchor leaves while open moves to its fallback", async () => {
    const other = document.createElement("button");
    other.setAttribute("data-popover-anchor", "host-theme-picker");
    document.body.append(other);
    try {
      const { closed, opener, box } = await mount("host-theme-picker@inspector", "host-theme-picker@inspector", "host-theme-picker");
      await act(async () => opener!.remove());
      await act(async () => new Promise((r) => setTimeout(r, 0)));
      expect([closed(), !!box()]).toEqual([0, true]);
    } finally {
      other.remove();
    }
  });

  test("AgentPopover whose only anchor leaves while open closes", async () => {
    const { closed, opener } = await mount("host-theme-picker");
    await act(async () => opener!.remove());
    await act(async () => new Promise((r) => setTimeout(r, 0)));
    expect(closed()).toBe(1);
  });

  test("AgentPopover on Escape while editing elsewhere stays open", async () => {
    const { closed } = await mount("host-theme-picker");
    const input = document.createElement("input");
    document.body.append(input);
    try {
      input.focus();
      await dispatch(input, new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
      expect(closed()).toBe(0);
    } finally {
      input.remove();
    }
  });

  test("AgentPopover on an IME Escape stays open", async () => {
    const { closed, box } = await mount("host-theme-picker");
    await dispatch(box()!.querySelector("[data-surface-id]")!, new KeyboardEvent("keydown", { key: "Escape", isComposing: true, bubbles: true, cancelable: true }));
    expect(closed()).toBe(0);
  });

  test("AgentPopover under a modal leaves Escape to the modal", async () => {
    const { closed } = await mount("host-theme-picker");
    let escaped = 0;
    function Modal() {
      const ref = useRef<HTMLDivElement>(null);
      useModalFrame(ref, { onEscape: () => escaped++ });
      return createElement("div", { ref, tabIndex: -1, role: "dialog" });
    }
    const host = document.createElement("div");
    document.body.append(host);
    const modal = createRoot(host);
    try {
      await act(async () => modal.render(createElement(Modal)));
      await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
      await dispatch(document.activeElement ?? window, new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
      expect([closed(), escaped]).toEqual([0, 1]);
    } finally {
      await act(async () => modal.unmount());
      host.remove();
    }
  });

  test("AgentPopover is a non-modal dialog holding the surface", async () => {
    const { box } = await mount("host-theme-picker");
    const dialog = box()!.querySelector('[role="dialog"]');
    expect([dialog?.getAttribute("aria-modal"), dialog?.getAttribute("data-surface-id")]).toEqual(["false", "host-theme-picker"]);
  });
});
