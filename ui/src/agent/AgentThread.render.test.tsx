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
const { realSurface } = await import("../a2ui/fixtures/agentSurfaces");
const { expectIconOnly } = await import("../test/iconOnly");

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

  test("AgentThread new conversation button shows an icon only", async () => {
    const { host } = await mount({ state: "idle", entries: [{ kind: "user", text: "plus court" }] });
    expectIconOnly(host.querySelector(".agent-new"));
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
    expect(kinds).toEqual(["user", "tools", "plan", "agent", "error", "stop"]);
    expect(host.querySelector(".you")?.textContent).toBe("plus court");
    expect(host.querySelector(".tool")?.textContent).toBe("☑ Lire la slide");
    expect(host.querySelector(".plan li")?.className).toBe("in_progress");
    expect(host.querySelector(".msg")?.textContent).toBe("C'est fait.");
    expect(host.querySelector('[role="alert"]')?.textContent).toBe("agent introuvable");
    expect(host.querySelector(".agent-entry.stop")?.textContent).toBe("ui-agent-stopped");
  });

  test("une réponse de l'agent est rendue en Markdown", async () => {
    const { host } = await mount({ state: "idle", entries: [{ kind: "agent", text: "- **Toto** et la Seine" }] });
    expect(host.querySelector(".msg li strong")?.textContent).toBe("Toto");
  });

  test("une consigne envoyée reste sur sa carte", async () => {
    const { host } = await mount({ state: "working", entries: [{ kind: "user", text: "plus court" }] });
    expect(host.querySelector(".agent-entry.user .you")?.classList.contains("postit")).toBe(true);
  });

  test("les consignes successives s'empilent dans l'ordre", async () => {
    const { host } = await mount({
      state: "idle",
      entries: [
        { kind: "user", text: "un" },
        { kind: "agent", text: "ok" },
        { kind: "user", text: "deux" },
      ],
    });
    expect([...host.querySelectorAll(".you")].map((e) => e.textContent)).toEqual(["un", "deux"]);
  });

  test("les appels d'outils consécutifs tiennent dans un accordéon fermé", async () => {
    const { host } = await mount({
      state: "working",
      entries: [
        { kind: "user", text: "a" },
        { kind: "tool", id: "t1", title: "Lire", status: "completed" },
        { kind: "tool", id: "t2", title: "Écrire", status: "in_progress" },
        { kind: "agent", text: "fait" },
        { kind: "tool", id: "t3", title: "Rendre", status: "completed" },
      ],
    });
    const groups = [...host.querySelectorAll("details.tools")] as HTMLDetailsElement[];
    expect(groups.map((d) => d.querySelectorAll(".tool").length)).toEqual([2, 1]);
    expect(groups.every((d) => !d.open)).toBe(true);
  });

  test("le résumé d'un accordéon d'outils porte la marque du dernier outil", async () => {
    const { host } = await mount({
      state: "working",
      entries: [
        { kind: "tool", id: "t1", title: "Lire", status: "completed" },
        { kind: "tool", id: "t2", title: "Écrire", status: "in_progress" },
      ],
    });
    expect(host.querySelector("details.tools summary")?.textContent).toBe("◌ ui-agent-tools");
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
    const button = host.querySelector(".agent-surface.place-thread button") as HTMLElement;
    expect(button).not.toBeNull();
    expect(button.textContent).toBe("Valider");
    await act(async () => button.click());
    expect(actions).toEqual([["ask-1", "ok"]]);
  });

  test("un post-it est rendu dans le fil, à sa place", async () => {
    const { host } = await mount({ state: "idle", entries: [] }, [surface("show-1", "postit")]);
    expect(host.querySelector(".agent-surface.place-postit button")?.textContent).toBe("Valider");
  });

  test("le cadre d'un post-it du fil ne prend pas la classe postit des notes", async () => {
    // Régression : `.postit` (132px, min-height 96px) rétrécissait le cadre, le contenu débordait.
    const { host } = await mount({ state: "idle", entries: [] }, [surface("show-1", "postit")]);
    expect(host.querySelector(".agent-surface")?.classList.contains("postit")).toBe(false);
  });

  test("les suites de la réponse sont des boutons du fil ; un clic envoie suggest", async () => {
    const { host, actions } = await mount({ state: "idle", entries: [{ kind: "agent", text: "Fait." }] }, [realSurface("show-7")]);
    const buttons = [...host.querySelectorAll(".agent-surface.place-suggestions button")] as HTMLElement[];
    expect(buttons.map((b) => b.textContent)).toEqual(["Mettre en barres", "Ajouter une légende"]);
    await act(async () => buttons[0].click());
    expect(actions).toEqual([["show-7", "suggest"]]);
  });
});
