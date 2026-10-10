// Section des agents (menu des réglages de la dictée) dans un DOM happy-dom.

import "../test/dom";
import { afterEach, describe, expect, test } from "bun:test";
import type { Surface } from "../api";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentPicker } = await import("./AgentPicker");
const { PICKER } = await import("../a2ui/fixtures/agentPicker");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(surface: Surface | null) {
  const selected: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(AgentPicker, {
        surface,
        onSelect: (id: string) => void selected.push(id),
        onError: (m: string) => {
          throw new Error(m);
        },
      }),
    ),
  );
  // La surface est chargée à la demande (lazy) : laisser Suspense la résoudre.
  for (let i = 0; i < 50 && !host.querySelector(".chip"); i++) await act(async () => new Promise((r) => setTimeout(r, 5)));
  return { host, selected };
}

const chip = (host: HTMLElement, label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>(".chip")].find((b) => b.textContent?.trim() === label);

describe("AgentPicker", () => {
  test("AgentPicker with a surface shows every agent", async () => {
    const { host } = await mount(PICKER);
    expect(chip(host, "Claude")).toBeDefined();
    expect(chip(host, "Gemini")).toBeDefined();
  });

  test("AgentPicker choosing an agent calls onSelect with its id", async () => {
    const { host, selected } = await mount(PICKER);
    await act(async () => chip(host, "Claude")!.click());
    expect(selected).toEqual(["claude"]);
  });

  test("AgentPicker is an inline section, not a dialog, and takes no focus", async () => {
    const { host } = await mount(PICKER);
    const box = host.querySelector(".agent-picker")!;
    expect(box.getAttribute("role")).toBeNull();
    expect(box.hasAttribute("tabindex")).toBe(false);
    expect(document.activeElement).not.toBe(box);
  });

  test("AgentPicker without surface renders nothing", async () => {
    const { host } = await mount(null);
    expect(host.innerHTML).toBe("");
  });
});
