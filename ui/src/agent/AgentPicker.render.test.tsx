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

/** Le panneau avec l'agent local : un bouton par évènement de `buttons` (`download_model`, `cancel_download`). */
function withButtons(buttons: { id: string; label: string; event: string }[]): Surface {
  const components = [
    { id: "root", component: "Column", children: ["title", ...buttons.map((b) => b.id)] },
    { id: "title", component: "Text", text: "Agent", variant: "h4" },
    ...buttons.map((b) => ({ id: b.id, component: "Button", label: b.label, action: { event: { name: b.event, context: {} } } })),
  ];
  return {
    ...PICKER,
    messages: [
      PICKER.messages[0],
      { version: "v0.9", updateComponents: { surfaceId: "agent-picker", components } },
    ] as unknown as Surface["messages"],
  };
}

async function mount(surface: Surface | null) {
  const selected: string[] = [];
  const calls: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(AgentPicker, {
        surface,
        onSelect: (id: string) => void selected.push(id),
        onDownload: () => void calls.push("download"),
        onCancelDownload: () => void calls.push("cancel"),
        onError: (m: string) => {
          throw new Error(m);
        },
      }),
    ),
  );
  // La surface est chargée à la demande (lazy) : laisser Suspense la résoudre.
  for (let i = 0; i < 50 && !host.querySelector(".chip, button"); i++) await act(async () => new Promise((r) => setTimeout(r, 5)));
  return { host, selected, calls };
}

const chip = (host: HTMLElement, label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>(".chip")].find((b) => b.textContent?.trim() === label);

const button = (host: HTMLElement, label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === label);

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

  test("AgentPicker download button calls onDownload once", async () => {
    const { host, calls } = await mount(withButtons([{ id: "download", label: "Télécharger le modèle", event: "download_model" }]));
    await act(async () => button(host, "Télécharger le modèle")!.click());
    expect(calls).toEqual(["download"]);
  });

  test("AgentPicker cancel button calls onCancelDownload once", async () => {
    const { host, calls } = await mount(withButtons([{ id: "cancel-download", label: "Annuler", event: "cancel_download" }]));
    await act(async () => button(host, "Annuler")!.click());
    expect(calls).toEqual(["cancel"]);
  });

  test("AgentPicker without surface renders nothing", async () => {
    const { host } = await mount(null);
    expect(host.innerHTML).toBe("");
  });
});
