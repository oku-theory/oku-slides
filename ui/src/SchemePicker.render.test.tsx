// Rendu du choix du schéma (système, clair, sombre) dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { SchemeChoice } from "./scheme";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: SchemePicker } = await import("./SchemePicker");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(value: SchemeChoice) {
  const chosen: SchemeChoice[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(SchemePicker, { value, onChange: (c: SchemeChoice) => chosen.push(c) }));
  });
  return { select: host.querySelector("select")!, chosen };
}

describe("SchemePicker", () => {
  test("SchemePicker propose système, clair et sombre", async () => {
    const { select } = await mount(null);
    expect([...select.options].map((o) => [o.value, o.textContent])).toEqual([
      ["", "ui-scheme-system"],
      ["light", "ui-scheme-light"],
      ["dark", "ui-scheme-dark"],
    ]);
  });

  test("SchemePicker montre le choix courant", async () => {
    const { select } = await mount("dark");
    expect(select.value).toBe("dark");
  });

  test("SchemePicker choisir sombre puis système rend dark puis null", async () => {
    const { select, chosen } = await mount("light");
    for (const value of ["dark", ""]) {
      await act(async () => {
        select.value = value;
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
    }
    expect(chosen).toEqual(["dark", null]);
  });
});
