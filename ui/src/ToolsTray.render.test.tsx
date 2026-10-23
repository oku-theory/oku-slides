// La barre d'outils sous la feuille : la bascule de la bande des miniatures.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ToolsTray } = await import("./ToolsTray");
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

const snap = { can_undo: false, can_redo: false, undo_label: null, redo_label: null } as unknown as Snapshot;

async function mount(thumbs: { open: boolean; onToggle: () => void } | null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(ToolsTray, {
      api: {} as TabApi, snap, view: null, widgetActive: false, widgets: {} as never, keptUi: {} as never,
      mainMenu: null, run: async () => null, onAdd: () => {}, thumbs,
    }));
  });
  return host;
}

const toggle = (host: HTMLElement) => host.querySelector<HTMLButtonElement>("button.thumbs-toggle");

describe("ToolsTray", () => {
  test("ToolsTray open strip offers to hide the thumbnails, pressed", async () => {
    const host = await mount({ open: true, onToggle: () => {} });
    expect([toggle(host)?.getAttribute("aria-label"), toggle(host)?.getAttribute("aria-pressed")]).toEqual(["ui-toolbar-thumbs-hide", "true"]);
  });

  test("ToolsTray folded strip offers to show the thumbnails", async () => {
    const host = await mount({ open: false, onToggle: () => {} });
    expect(toggle(host)?.getAttribute("aria-label")).toBe("ui-toolbar-thumbs-show");
  });

  test("ToolsTray thumbnails toggle shows an icon, no text", async () => {
    const host = await mount({ open: true, onToggle: () => {} });
    expectIconOnly(toggle(host), "ui-toolbar-thumbs-hide");
  });

  test("ToolsTray thumbnails toggle calls onToggle", async () => {
    let toggled = 0;
    const host = await mount({ open: true, onToggle: () => toggled++ });
    await act(async () => toggle(host)?.click());
    expect(toggled).toBe(1);
  });

  test("ToolsTray without a strip shows no thumbnails toggle", async () => {
    const host = await mount(null);
    expect(toggle(host)).toBeNull();
  });
});
