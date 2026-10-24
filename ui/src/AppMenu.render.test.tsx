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
    root.render(createElement(AppMenu, { packMode: false, present: noop, open: noop, onHelp: noop, ...extra })),
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

  test("AppMenu pack mode keeps the update entry", async () => {
    const items = await openMenu({ packMode: true, onCheckUpdates: () => {} });
    expect(labelled(items, "ui-update-check")).toBeDefined();
  });
});
