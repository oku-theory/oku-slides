// Rendu du menu principal : la recherche de mises à jour n'y figure qu'avec un hôte.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AppMenu } = await import("./AppMenu");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function openMenu(extra: Record<string, unknown> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const noop = () => {};
  await act(async () =>
    root.render(createElement(AppMenu, { packMode: false, themeTab: false, present: noop, open: noop, onHelp: noop, onAbout: noop, ...extra })),
  );
  await act(async () => (host.querySelector(".action-menu-trigger") as HTMLButtonElement).click());
  return [...host.querySelectorAll('[role="menuitem"]')] as HTMLButtonElement[];
}

const labelled = (items: HTMLButtonElement[], key: string) => items.find((b) => b.textContent?.includes(key));

describe("AppMenu", () => {
  test("AppMenu without host has no update entry", async () => {
    const items = await openMenu();
    expect(labelled(items, "ui-update-check")).toBeUndefined();
  });

  test("AppMenu update entry runs the check", async () => {
    let checks = 0;
    const items = await openMenu({ onCheckUpdates: () => checks++ });
    await act(async () => labelled(items, "ui-update-check")?.click());
    expect(checks).toBe(1);
  });

  test("AppMenu in a theme tab offers neither present nor open", async () => {
    const items = await openMenu({ themeTab: true });
    expect([labelled(items, "ui-present-screen"), labelled(items, "ui-toolbar-open"), !!labelled(items, "ui-help")]).toEqual([undefined, undefined, true]);
  });

  test("AppMenu about entry opens the about dialog", async () => {
    let opened = 0;
    const items = await openMenu({ onAbout: () => opened++ });
    await act(async () => labelled(items, "ui-about")?.click());
    expect(opened).toBe(1);
  });

  test("AppMenu theme tab keeps the about entry", async () => {
    const items = await openMenu({ themeTab: true });
    expect(labelled(items, "ui-about")).toBeDefined();
  });

  test("AppMenu pack mode keeps the update entry", async () => {
    const items = await openMenu({ packMode: true, onCheckUpdates: () => {} });
    expect(labelled(items, "ui-update-check")).toBeDefined();
  });
});
