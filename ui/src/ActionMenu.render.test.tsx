// Rendu du menu d'actions dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { ActionMenu } = await import("./ActionMenu");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount() {
  const picked: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(ActionMenu, {
        label: "Menu",
        entries: [
          { kind: "item", label: "Ouvrir…", shortcut: "⌘O", onSelect: () => picked.push("open") },
          { kind: "separator" },
          { kind: "item", label: "Déplacer…", disabled: true, onSelect: () => picked.push("move") },
          { kind: "custom", node: createElement("span", { className: "extra" }, "Thème") },
        ],
      }),
    );
  });
  const trigger = () => host.querySelector(".action-menu-trigger") as HTMLButtonElement;
  const menu = () => host.querySelector('[role="menu"]');
  const items = () => [...host.querySelectorAll('[role="menuitem"]')] as HTMLButtonElement[];
  return { host, trigger, menu, items, picked };
}

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));

describe("menu d'actions", () => {
  test("ActionMenu fermé par défaut, le bouton porte son nom", async () => {
    const { trigger, menu } = await mount();
    expect(menu()).toBeNull();
    expect(trigger().getAttribute("aria-label")).toBe("Menu");
    expect(trigger().getAttribute("aria-expanded")).toBe("false");
  });

  test("ActionMenu clic sur le bouton l'ouvre, avec ses entrées", async () => {
    const { trigger, items, host } = await mount();
    await act(async () => trigger().click());
    expect(items().map((b) => b.textContent)).toEqual(["Ouvrir…⌘O", "Déplacer…"]);
    expect(host.querySelector(".extra")?.textContent).toBe("Thème");
    expect(trigger().getAttribute("aria-expanded")).toBe("true");
  });

  test("ActionMenu choisir une entrée l'exécute et ferme le menu", async () => {
    const { trigger, items, menu, picked } = await mount();
    await act(async () => trigger().click());
    await act(async () => items()[0].click());
    expect(picked).toEqual(["open"]);
    expect(menu()).toBeNull();
  });

  test("ActionMenu entrée désactivée : rien", async () => {
    const { trigger, items } = await mount();
    await act(async () => trigger().click());
    expect(items()[1].disabled).toBe(true);
  });

  test("ActionMenu Échap ferme et rend le focus au bouton", async () => {
    const { trigger, menu } = await mount();
    await act(async () => trigger().click());
    await act(async () => key(menu() as Element, "Escape"));
    expect(menu()).toBeNull();
    expect(document.activeElement).toBe(trigger());
  });

  test("ActionMenu clic ailleurs ferme", async () => {
    const { trigger, menu } = await mount();
    await act(async () => trigger().click());
    await act(async () => document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
    expect(menu()).toBeNull();
  });

  test("ActionMenu ouvert : focus sur la première entrée, flèches pour descendre", async () => {
    const { trigger, items, menu } = await mount();
    await act(async () => trigger().click());
    expect(document.activeElement).toBe(items()[0]);
    await act(async () => key(menu() as Element, "ArrowDown"));
    // La seconde est désactivée : on reste sur la seule active.
    expect(document.activeElement).toBe(items()[0]);
  });
});
