// Rendu du fil de l'agent dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import "../test/dom";
import type { Surface } from "../api";
import { CATALOG_ID } from "../a2ui/catalog";
import type { Thread } from "./thread";

// Voir ../a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentThread } = await import("./AgentThread");

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
const surface = (id: string, place: Surface["place"] = "thread"): Surface => ({
  id,
  place,
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
});

async function mount(thread: Thread, surfaces: Surface[] = []) {
  const actions: [string, string][] = [];
  let fresh = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(AgentThread, {
        thread,
        surfaces,
        onAction: (s: Surface, a: A2uiClientAction) => actions.push([s.id, a.name]),
        onNewConversation: () => fresh++,
        onError: () => {},
      }),
    );
  });
  // La surface est chargée à la demande (lazy) : laisser Suspense la résoudre.
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  return { host, actions, fresh: () => fresh };
}

describe("fil de l'agent", () => {
  test("un fil vide sans surface ne montre rien", async () => {
    const { host } = await mount({ state: "idle", entries: [] });
    expect(host.querySelector(".agent-thread")).toBeNull();
  });

  test("chaque ligne du fil a sa forme", async () => {
    const { host } = await mount({
      state: "working",
      entries: [
        { kind: "user", text: "plus court" },
        { kind: "tool", id: "t1", title: "Lire la slide", status: "completed" },
        { kind: "plan", entries: [{ content: "Raccourcir", status: "in_progress" }] },
        { kind: "agent", text: "C'est fait." },
        { kind: "error", text: "agent introuvable" },
        { kind: "stop", stop: "cancelled" },
      ],
    });
    const kinds = [...host.querySelectorAll(".agent-entry")].map((e) => [...e.classList].filter((c) => c !== "agent-entry")[0]);
    expect(kinds).toEqual(["user", "tool", "plan", "agent", "error", "stop"]);
    expect(host.querySelector(".you")?.textContent).toBe("« plus court »");
    expect(host.querySelector(".tool")?.textContent).toBe("☑ Lire la slide");
    expect(host.querySelector(".plan li")?.className).toBe("in_progress");
    expect(host.querySelector(".msg")?.textContent).toBe("C'est fait.");
    expect(host.querySelector('[role="alert"]')?.textContent).toBe("agent introuvable");
    expect(host.querySelector(".agent-entry.stop")?.textContent).toBe("ui-agent-stopped");
  });

  test("chaque fin de tour connue a sa phrase ; une raison inconnue est citée", async () => {
    const { host } = await mount({
      state: "idle",
      entries: [
        { kind: "stop", stop: "max_tokens" },
        { kind: "stop", stop: "max_turn_requests" },
        { kind: "stop", stop: "refusal" },
        { kind: "stop", stop: "surprise" },
      ],
    });
    expect([...host.querySelectorAll(".agent-entry.stop")].map((e) => e.textContent)).toEqual([
      "ui-agent-stop-max-tokens",
      "ui-agent-stop-max-turns",
      "ui-agent-stop-refusal",
      "ui-agent-ended",
    ]);
  });

  test("un statut inconnu ne devient pas une classe", async () => {
    const { host } = await mount({ state: "idle", entries: [{ kind: "plan", entries: [{ content: "x", status: "hidden" }] }] });
    expect(host.querySelector(".plan li")?.className).toBe("other");
  });

  test("l'état de l'agent est dit en tête", async () => {
    const { host } = await mount({ state: "starting", entries: [{ kind: "user", text: "a" }] });
    expect(host.querySelector(".agent-thread-head .caption")?.textContent).toBe("ui-agent-state-starting");
  });

  test("« Nouvelle conversation » le demande", async () => {
    const { host, fresh } = await mount({ state: "idle", entries: [{ kind: "user", text: "a" }] });
    await act(async () => (host.querySelector(".agent-thread-head button") as HTMLElement).click());
    expect(fresh()).toBe(1);
  });

  test("une surface du fil est rendue, même sans ligne, et ses actions nomment la surface", async () => {
    const { host, actions } = await mount({ state: "working", entries: [] }, [surface("ask-1")]);
    const button = host.querySelector(".agent-surface.thread button") as HTMLElement;
    expect(button).not.toBeNull();
    expect(button.textContent).toBe("Valider");
    await act(async () => button.click());
    expect(actions).toEqual([["ask-1", "ok"]]);
  });

  test("un post-it est rendu dans le fil, à sa place", async () => {
    const { host } = await mount({ state: "idle", entries: [] }, [surface("show-1", "postit")]);
    expect(host.querySelector(".agent-surface.postit button")?.textContent).toBe("Valider");
  });
});
