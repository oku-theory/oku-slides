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
    root.render(createElement(AppMenu, { kind: "deck", present: noop, open: noop, onHelp: noop, onAbout: noop, onNewPack: noop, onNewTheme: noop, ...extra })),
  );
  await act(async () => (host.querySelector(".action-menu-trigger") as HTMLButtonElement).click());
  return [...host.querySelectorAll('[role="menuitem"]')] as HTMLButtonElement[];
}

/** The entries' key ids, without shortcut hints. */
const names = (items: HTMLButtonElement[]) => items.map((b) => b.textContent?.replace(/(Ctrl|Cmd|⌘).*$/, ""));
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
    const items = await openMenu({ kind: "theme" });
    expect([labelled(items, "ui-present-screen"), labelled(items, "ui-toolbar-open"), !!labelled(items, "ui-help")]).toEqual([undefined, undefined, true]);
  });

  test("AppMenu about entry opens the about dialog", async () => {
    let opened = 0;
    const items = await openMenu({ onAbout: () => opened++ });
    await act(async () => labelled(items, "ui-about")?.click());
    expect(opened).toBe(1);
  });

  test("AppMenu theme tab keeps the about entry", async () => {
    const items = await openMenu({ kind: "theme" });
    expect(labelled(items, "ui-about")).toBeDefined();
  });

  test("AppMenu pack mode keeps the update entry", async () => {
    const items = await openMenu({ kind: "pack", onCheckUpdates: () => {} });
    expect(labelled(items, "ui-update-check")).toBeDefined();
  });

  test("AppMenu in a deck tab lists present, open, the three new entries, then the app entries", async () => {
    const items = await openMenu({ onCheckUpdates: () => {} });
    expect(names(items)).toEqual([
      "ui-present-screen", "ui-toolbar-open", "ui-menu-new-widget-pack", "ui-menu-new-layout-pack", "ui-menu-new-theme", "ui-update-check", "ui-help", "ui-about",
    ]);
  });

  test("AppMenu in a pack tab lists the three new entries and the app entries", async () => {
    const items = await openMenu({ kind: "pack" });
    expect(names(items)).toEqual(["ui-menu-new-widget-pack", "ui-menu-new-layout-pack", "ui-menu-new-theme", "ui-help", "ui-about"]);
  });

  test("AppMenu in a theme tab lists only the new theme entry and the app entries", async () => {
    const items = await openMenu({ kind: "theme" });
    expect(names(items)).toEqual(["ui-menu-new-theme", "ui-help", "ui-about"]);
  });

  test.each(["help", "history"])("AppMenu in a %s tab lists the app entries only", async (kind) => {
    const items = await openMenu({ kind });
    expect(names(items)).toEqual(["ui-help", "ui-about"]);
  });

  test("AppMenu new entries call their handlers with their kind", async () => {
    const calls: string[] = [];
    const handlers = { onNewPack: (k: string) => calls.push(k), onNewTheme: () => calls.push("theme") };
    for (const key of ["ui-menu-new-widget-pack", "ui-menu-new-layout-pack", "ui-menu-new-theme"]) {
      const items = await openMenu(handlers);
      await act(async () => labelled(items, key)?.click());
      await act(async () => mounted?.root.unmount());
      mounted?.host.remove();
      mounted = null;
    }
    expect(calls).toEqual(["widgets", "layouts", "theme"]);
  });
});
