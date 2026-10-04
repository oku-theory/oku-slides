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
const { t } = await import("../i18n");

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
  question: false,
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

type State = "idle" | "starting" | "working";

async function mount(surfaces: Surface[], state: State = "idle") {
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
  const rerender = async (next: Surface[], keysTo?: string | null, nextState: State = state) => {
    const routed = routeSurfaces(next, null);
    await act(async () => {
      root.render(createElement(AgentThread, { ...props(), ...routed, state: nextState, keysTo }));
    });
    await act(async () => new Promise((r) => setTimeout(r, 0)));
  };
  return { host, actions, fresh: () => fresh, rerender };
}

describe("fil de l'agent par tours", () => {
  test("sans tour ni surface, rien de visible : seules les régions annoncées restent (0 px)", async () => {
    const { host } = await mount([]);
    const thread = host.querySelector(".agent-thread");
    expect(thread?.classList.contains("vacant")).toBe(true);
    expect(thread?.textContent).toBe("");
    expect(host.querySelector(".agent-thread-head")).toBeNull();
  });

  test("une région polie persistante annonce les ajouts du fil, pas le dernier tour réannoncé", async () => {
    const { host, rerender } = await mount([]);
    const region = host.querySelector(".agent-turns");
    expect(region?.getAttribute("aria-live")).toBe("polite");
    expect(region?.getAttribute("aria-relevant")).toBe("additions");
    await rerender([realSurface("turn-3")]);
    // La même région, présente avant l'ajout : le premier tour est annoncé.
    expect(host.querySelector(".agent-turns")).toBe(region);
    expect(host.querySelector(".agent-thread")?.classList.contains("vacant")).toBe(false);
    expect(host.querySelectorAll(".agent-turn[aria-live]")).toHaveLength(0);
  });

  test("le dock est une région persistante qui annonce ses ajouts", async () => {
    const { host, rerender } = await mount([realSurface("turn-3")]);
    const dock = host.querySelector(".agent-dock");
    expect(dock?.classList.contains("vacant")).toBe(true);
    expect(dock?.getAttribute("aria-live")).toBe("polite");
    expect(dock?.getAttribute("aria-relevant")).toBe("additions");
    await rerender([realSurface("turn-3"), realSurface("ask-8")]);
    expect(host.querySelector(".agent-dock")).toBe(dock);
    expect(dock?.classList.contains("vacant")).toBe(false);
  });

  test("plusieurs questions au dock : seule la dernière (celle des touches) montre les chiffres 1–4", async () => {
    const second = JSON.parse(JSON.stringify(realSurface("ask-8")).replaceAll('"ask-8"', '"ask-9"')) as Surface;
    const { host } = await mount([realSurface("turn-3"), realSurface("ask-8"), second]);
    const keys = (id: string) => host.querySelectorAll(`.agent-dock [data-surface-id="${id}"] .chip-key`).length;
    expect(keys("ask-8")).toBe(0);
    expect(keys("ask-9")).toBe(2);
  });

  test("une fiche ou une fenêtre prend les touches : la question du dock cache ses chiffres", async () => {
    const { host, rerender } = await mount([realSurface("turn-3"), realSurface("ask-8")]);
    expect(host.querySelectorAll(".agent-dock .chip-key")).toHaveLength(2);
    await rerender([realSurface("turn-3"), realSurface("ask-8")], "ask-1");
    expect(host.querySelectorAll(".agent-dock .chip-key")).toHaveLength(0);
  });

  test("plusieurs questions au dock : toutes visibles, chacune répond pour elle-même", async () => {
    // Une autre question, mêmes messages sous un autre identifiant de surface.
    const second = JSON.parse(JSON.stringify(realSurface("ask-8")).replaceAll('"ask-8"', '"ask-9"')) as Surface;
    const { host, actions } = await mount([realSurface("turn-3"), realSurface("ask-8"), second]);
    const asks = [...host.querySelectorAll<HTMLElement>(".agent-dock .agent-dock-item.ask")];
    expect(asks.map((a) => a.dataset.surfaceId)).toEqual(["ask-8", "ask-9"]);
    await act(async () => asks[1].querySelectorAll<HTMLElement>(".chip")[0].click());
    await act(async () => asks[0].querySelectorAll<HTMLElement>(".chip")[1].click());
    expect(actions).toEqual([["ask-9", "choose"], ["ask-8", "choose"]]);
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

  test("le dock est vide quand rien n'est vivant", async () => {
    const { host } = await mount([realSurface("turn-3")]);
    expect(host.querySelector(".agent-dock")?.childElementCount).toBe(0);
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
    expect(host.querySelector(".agent-dock")?.childElementCount).toBe(0);
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
    scroller.scrollTop = 900;
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    scroller.scrollTop = 0;
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    const jump = host.querySelector(".agent-jump") as HTMLButtonElement;
    expect(jump).not.toBeNull();
    await act(async () => jump.click());
    expect(scroller.scrollTop).toBe(1000);
    expect(host.querySelector(".agent-jump")).toBeNull();
  });

  test("un nouveau tour se suit à nouveau, même après être remonté", async () => {
    const { host, rerender } = await mount([realSurface("turn-3")], "working");
    const scroller = host.querySelector(".agent-scroller") as HTMLElement;
    Object.defineProperty(scroller, "scrollHeight", { value: 1000, configurable: true, writable: true });
    Object.defineProperty(scroller, "clientHeight", { value: 100, configurable: true });
    scroller.scrollTop = 900;
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    scroller.scrollTop = 0;
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    expect(host.querySelector(".agent-jump")).not.toBeNull();
    // Le tour suivant arrive : le fil y retourne en bas et le bouton de retour se cache.
    const next = JSON.parse(JSON.stringify(realSurface("turn-3")).replaceAll('"turn-3"', '"turn-4"')) as Surface;
    Object.defineProperty(scroller, "scrollHeight", { value: 1400, configurable: true });
    await rerender([realSurface("turn-3"), { ...next, turn: 4 }]);
    expect(host.querySelectorAll(".agent-turn")).toHaveLength(2);
    expect(scroller.scrollTop).toBe(1400);
    expect(host.querySelector(".agent-jump")).toBeNull();
  });

  test("la légende d'état suit l'état de l'agent", async () => {
    const { host, rerender } = await mount([realSurface("turn-3")], "starting");
    const caption = () => host.querySelector(".agent-thread-head .caption")?.textContent;
    expect(caption()).toBe(t("ui-agent-state-starting"));
    await rerender([realSurface("turn-3")], undefined, "working");
    expect(caption()).toBe(t("ui-agent-state-working"));
    await rerender([realSurface("turn-3")], undefined, "idle");
    expect(caption()).toBe(t("ui-agent-state-idle"));
  });

  test("le fil suivi reste en bas quand le dock grandit (la zone qui défile est observée)", async () => {
    const observed: Element[] = [];
    let fire = () => {};
    const original = globalThis.ResizeObserver;
    globalThis.ResizeObserver = class {
      constructor(cb: () => void) {
        fire = cb;
      }
      observe(el: Element) {
        observed.push(el);
      }
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    try {
      const { host } = await mount([realSurface("turn-3")], "working");
      const scroller = host.querySelector(".agent-scroller") as HTMLElement;
      expect(observed).toContain(scroller);
      // Le dock prend de la place : la zone rétrécit, le fil suivi revient en bas.
      Object.defineProperty(scroller, "scrollHeight", { value: 800, configurable: true });
      scroller.scrollTop = 100;
      await act(async () => fire());
      expect(scroller.scrollTop).toBe(800);
    } finally {
      globalThis.ResizeObserver = original;
    }
  });

  test("le dock qui grandit après un retour en bas ne détache pas le fil (seul un geste vers le haut le fait)", async () => {
    const { host } = await mount([realSurface("turn-3")], "working");
    const scroller = host.querySelector(".agent-scroller") as HTMLElement;
    Object.defineProperty(scroller, "scrollHeight", { value: 421, configurable: true });
    Object.defineProperty(scroller, "clientHeight", { value: 308, configurable: true, writable: true });
    scroller.scrollTop = 113; // en bas
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
    // Le dock grandit avant que l'évènement du défilement ne soit lu : l'écart au bas dépasse le seuil.
    Object.defineProperty(scroller, "clientHeight", { value: 274, configurable: true });
    await act(async () => scroller.dispatchEvent(new Event("scroll")));
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
