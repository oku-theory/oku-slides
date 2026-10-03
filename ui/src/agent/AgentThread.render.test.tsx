// Rendu du fil de l'agent dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import "../test/dom";
import type { Surface } from "../api";
import { CATALOG_ID } from "../a2ui/catalog";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentThread } = await import("./AgentThread");
const { realSurface } = await import("../a2ui/fixtures/agentSurfaces");
const { expectIconOnly } = await import("../test/iconOnly");
const { routeSurfaces } = await import("./surfaces");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Une surface « dans le fil » : un bouton qui envoie `ok`. */
const surface = (id: string, place: Surface["place"] = "thread", extra: Partial<Surface> = {}): Surface => ({
  id,
  place,
  live: false,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: id, catalogId: CATALOG_ID } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: id,
        components: [{ id: "root", component: "Button", label: "Valider", action: { event: { name: "ok", context: {} } } }],
      },
    },
  ] as unknown as Surface["messages"],
  ...extra,
});

async function mount(surfaces: Surface[], state: "idle" | "working" = "idle") {
  const actions: [string, string][] = [];
  let fresh = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const props = () => ({
    onAction: (s: Surface, a: A2uiClientAction) => actions.push([s.id, a.name]),
    onNewConversation: () => fresh++,
    onError: (e: string) => {
      throw new Error(e);
    },
  });
  const { turns, loose, dock } = routeSurfaces(surfaces, null);
  await act(async () => {
    root.render(createElement(AgentThread, { ...props(), state, turns, loose, dock }));
  });
  // Les surfaces sont chargées à la demande (lazy) : laisser Suspense les résoudre.
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  /** Remonte avec une autre liste de surfaces (même racine React : l'état local est gardé). */
  const rerender = async (next: Surface[]) => {
    const routed = routeSurfaces(next, null);
    await act(async () => {
      root.render(createElement(AgentThread, { ...props(), ...routed, state }));
    });
    await act(async () => new Promise((r) => setTimeout(r, 0)));
  };
  return { host, actions, fresh: () => fresh, rerender };
}

describe("fil de l'agent par tours", () => {
  test("rien n'est rendu sans tour ni surface", async () => {
    const { host } = await mount([]);
    expect(host.querySelector(".agent-thread")).toBeNull();
  });

  test("un tour montre sa consigne puis son reçu", async () => {
    const { host } = await mount([realSurface("report-8"), realSurface("turn-3")]);
    const turn = host.querySelector(".agent-turn");
    const prompt = turn?.querySelector(".a2ui-prompt");
    const receipt = turn?.querySelector(".a2ui-receipt");
    expect(prompt?.textContent).toBe("Change le fond de la slide quatre.");
    expect(receipt?.querySelector(".a2ui-title")?.textContent).toBe("Slide 4 : fond bleu nuit");
    expect(prompt!.compareDocumentPosition(receipt!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  test("le dock n'est pas rendu quand rien n'est vivant", async () => {
    const { host } = await mount([realSurface("turn-3")]);
    expect(host.querySelector(".agent-dock")).toBeNull();
  });

  test("une question vivante est au dock, sous les tours", async () => {
    const { host } = await mount([realSurface("turn-3"), realSurface("ask-8")]);
    const dock = host.querySelector(".agent-dock");
    expect(dock?.querySelector(".chips")).not.toBeNull();
    expect(host.querySelector(".agent-turns")!.compareDocumentPosition(dock!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(host.querySelector(".agent-turn .chips")).toBeNull();
  });

  test("une question figée reste dans son tour, pas au dock", async () => {
    const { host } = await mount([realSurface("turn-3"), realSurface("ask-7")]);
    expect(host.querySelector(".agent-dock")).toBeNull();
    expect(host.querySelector('.agent-turn [data-surface-id="ask-7"]')?.textContent).toContain("✓ Deux colonnes");
  });

  test("le fil ne rend ni entrée ni post-it", async () => {
    const { host } = await mount([realSurface("turn-3")]);
    expect(host.querySelector(".agent-entry, .postit")).toBeNull();
  });

  test("une action d'une surface du dock part avec son identifiant", async () => {
    const { host, actions } = await mount([surface("show-9", "thread", { live: true, turn: 1 })]);
    await act(async () => (host.querySelector('.agent-dock [data-surface-id="show-9"] button.btn') as HTMLElement).click());
    expect(actions).toEqual([["show-9", "ok"]]);
  });

  test("une question replie les réglages du dock, qu'un clic rouvre", async () => {
    const { host } = await mount([surface("show-9", "thread", { live: true, turn: 1 }), realSurface("ask-8")]);
    const toggle = host.querySelector('.agent-dock [data-surface-id="show-9"] button[aria-expanded]') as HTMLButtonElement;
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(host.querySelector('[data-surface-id="show-9"] button.btn')).toBeNull();
    await act(async () => toggle.click());
    expect(host.querySelector('[data-surface-id="show-9"] button.btn')).not.toBeNull();
  });

  test("à une nouvelle question, les réglages ouverts se replient à nouveau", async () => {
    const settings = surface("show-9", "thread", { live: true, turn: 3 });
    const { host, rerender } = await mount([settings, realSurface("ask-8")]);
    await act(async () => (host.querySelector('[data-surface-id="show-9"] button[aria-expanded]') as HTMLElement).click());
    expect(host.querySelector('[data-surface-id="show-9"] button.btn')).not.toBeNull();
    const next = { ...realSurface("ask-8"), id: "ask-10" };
    await rerender([settings, next]);
    expect(host.querySelector('[data-surface-id="show-9"] button.btn')).toBeNull();
  });

  test("« Tour en cours » ramène en bas et se cache", async () => {
    const { host } = await mount([realSurface("turn-3")], "working");
    const scroller = host.querySelector(".agent-scroller") as HTMLElement;
    // Remonté : l'écart au bas dépasse le seuil.
    Object.defineProperty(scroller, "scrollHeight", { value: 1000, configurable: true });
    Object.defineProperty(scroller, "clientHeight", { value: 100, configurable: true });
    scroller.scrollTop = 0;
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    const jump = host.querySelector(".agent-jump") as HTMLButtonElement;
    expect(jump).not.toBeNull();
    await act(async () => jump.click());
    expect(scroller.scrollTop).toBe(1000);
    expect(host.querySelector(".agent-jump")).toBeNull();
  });

  test("une surface figée sans tour reste visible", async () => {
    const { host } = await mount([surface("render-4", "thread")]);
    expect(host.querySelector('.agent-turn[data-surface-id="render-4"] button.btn')).not.toBeNull();
  });

  test("nouvelle conversation reste un bouton icône", async () => {
    const { host, fresh } = await mount([realSurface("turn-3")]);
    const button = host.querySelector(".agent-new") as HTMLButtonElement;
    expectIconOnly(button);
    await act(async () => button.click());
    expect(fresh()).toBe(1);
  });
});
