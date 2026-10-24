// La fenêtre de l'agent : le fil, le brouillon et la dictée de l'onglet au premier plan.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { memoryTransport } from "./test/link";
import type { Snapshot, Surface, SttStatus } from "./api";
import { stt } from "./api";
import type { HostHandlers } from "./agent/events";
import type { LinkContext } from "./agent/link";
import type { AgentWindowHost } from "./AgentWindow";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentWindowFor, AgentWindowView } = await import("./AgentWindow");
const { expectIconOnly } = await import("./test/iconOnly");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const SNAP = { title: "Deck", slides: [], read_only: false } as unknown as Snapshot;
const PAPIER = {
  scheme: "light", slide: { bg: "#fffdf9", ink: "#2e2b26" }, app: { desk: "#ece8e0" },
  variants: {}, metrics: {}, font_display: [], font_body: [], chart_palette: [],
};
const READ_ONLY = { title: "Aide", slides: [], read_only: true } as unknown as Snapshot;
const UI = { slide: "s1", element: null, field: null, editing: false, widget: null };
const ctx = (over: Partial<LinkContext> = {}): LinkContext => ({ editing: false, ui: UI, staged: null, keysOff: false, look: "", ...over });
const sheetOn = (id: string): Surface => ({ id, place: "sheet", live: true, question: false, anchor: { slide: "s1", owner: "e1" }, messages: [] });

function fakeHost(snapshot: Snapshot = SNAP) {
  const calls: string[] = [];
  let handlers: HostHandlers | null = null;
  let closing: () => void = () => {};
  const host = {
    agent: {
      follow: async () => void calls.push("follow"),
      resync: async () => void calls.push("resync"),
      setDraft: async () => {},
      prompt: async () => {},
      cancel: async () => {},
      newConversation: async () => {},
      surfaceAction: async () => {},
      showSettings: async () => void calls.push("showSettings"),
    },
    api: { snapshot: async () => snapshot, svg: async () => "<svg/>" },
    subscribe: (_tab: string, h: HostHandlers) => ((handlers = h), () => {}),
    focusSelf: () => void calls.push("focus"),
    onCloseRequested: (cb: () => void) => ((closing = cb), () => {}),
    attach: async () => void calls.push("attach"),
  } satisfies Partial<Record<keyof AgentWindowHost, unknown>>;
  return { host: host as unknown as AgentWindowHost, calls, handlers: () => handlers!, close: () => closing() };
}

async function mountWindow(snapshot?: Snapshot) {
  const transport = memoryTransport();
  const h = fakeHost(snapshot);
  const el = document.createElement("div");
  const root = createRoot(el);
  await act(async () => root.render(createElement(AgentWindowFor, { tab: "t-1", transport, host: h.host })));
  cleanup = async () => act(async () => root.unmount());
  const ready = () => act(async () => h.handlers().onReady?.());
  const send = (m: Parameters<typeof transport.send>[0]) => act(async () => transport.send(m));
  return { ...h, transport, el, ready, send };
}

describe("AgentWindow", () => {
  test("AgentWindow l'instantané relu applique l'apparence du deck", async () => {
    document.documentElement.style.removeProperty("--app-desk");
    const m = await mountWindow({ ...SNAP, appearance: PAPIER } as Snapshot);
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx() });
    await act(async () => new Promise((r) => setTimeout(r, 600)));
    expect(document.documentElement.style.getPropertyValue("--app-desk")).toBe("#ece8e0");
  });

  test("AgentWindow abonnée : demande follow", async () => {
    const m = await mountWindow();
    await m.ready();
    expect(m.calls.filter((c) => c === "follow")).toHaveLength(1);
  });

  test("AgentWindow abonnée : ne demande jamais resync", async () => {
    const m = await mountWindow();
    await m.ready();
    expect(m.calls).not.toContain("resync");
  });

  test("AgentWindow dit hello une seule fois, écoute du canal en place", async () => {
    const m = await mountWindow();
    expect(m.transport.sent.filter((x) => x.kind === "hello")).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
    expect(m.transport.listeners()).toBeGreaterThan(0);
  });

  test("AgentWindow le contexte d'un autre onglet est ignoré", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx({ staged: "sheet-1" }) });
    await m.send({ to: "agent", kind: "context", tab: "t-2", context: ctx() });
    await act(async () => m.handlers().onMcp({ kind: "surfaces", list: [sheetOn("sheet-1")] }));
    expect(m.el.querySelector('[data-surface-id="sheet-1"]')).toBeNull();
  });

  test("AgentWindow une fiche non posée sur la scène est rendue dans le fil", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx() });
    await act(async () => m.handlers().onMcp({ kind: "surfaces", list: [sheetOn("sheet-1")] }));
    expect(m.el.querySelector('[data-surface-id="sheet-1"]')).not.toBeNull();
  });

  test("AgentWindow la fiche posée sur la scène n'est pas rendue dans le fil", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "context", tab: "t-1", context: ctx({ staged: "sheet-1" }) });
    await act(async () => m.handlers().onMcp({ kind: "surfaces", list: [sheetOn("sheet-1")] }));
    expect(m.el.querySelector('[data-surface-id="sheet-1"]')).toBeNull();
  });

  test("AgentWindow ⌘K relayé met la fenêtre devant", async () => {
    const m = await mountWindow();
    await m.send({ to: "agent", kind: "shortcut", tab: "t-1", key: "type" });
    expect(m.calls).toContain("focus");
  });

  test("AgentWindow bouton rattacher : rattache l'agent", async () => {
    const m = await mountWindow();
    await act(async () => (m.el.querySelector(".agent-attach") as HTMLButtonElement).click());
    expect(m.calls).toContain("attach");
  });

  test("AgentWindow onglet en lecture seule : état vide", async () => {
    const m = await mountWindow(READ_ONLY);
    await m.ready();
    expect(m.el.querySelector(".agent-window-empty")).not.toBeNull();
  });

  test("AgentWindow onglet en lecture seule : le bouton rattacher reste, icône seule", async () => {
    const m = await mountWindow(READ_ONLY);
    await m.ready();
    expectIconOnly(m.el.querySelector(".agent-attach"));
  });

  test("AgentWindow onglet en lecture seule : la dictée reste", async () => {
    const m = await mountWindow(READ_ONLY);
    await m.ready();
    expect(m.el.querySelector(".device")).not.toBeNull();
  });

  /** Un moteur de dictée au repos (le bouton des réglages n'existe qu'avec un état reçu). */
  const idleStt = () =>
    spyOn(stt, "status").mockImplementation(() =>
      Promise.resolve({
        state: "idle",
        config: { model: "turbo", language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY" } },
        models: [{ id: "turbo", label: "Turbo", size: 1, installed: true, available: true }],
        download: null, level: 0, speaking: false, committed: "", partial: "", detected: null,
        stop_in: null, text: null, seq: 0, seconds: 0, error: null,
      } as SttStatus),
    );
  /** Le ⚙ de la dictée, une fois la fenêtre prête et l'état de la dictée reçu ; `null` s'il manque. */
  const gearOf = async (m: Awaited<ReturnType<typeof mountWindow>>) => {
    await m.ready();
    await act(async () => new Promise((r) => setTimeout(r, 0)));
    return m.el.querySelector('[aria-label="ui-dictation-settings"]') as HTMLButtonElement | null;
  };
  /** Clique le ⚙ de la dictée, qui doit exister. */
  const openSettings = async (m: Awaited<ReturnType<typeof mountWindow>>) => {
    const gear = await gearOf(m);
    expect(gear).not.toBeNull();
    await act(async () => gear!.click());
  };

  test("AgentWindow gear asks the host for the settings box", async () => {
    const spy = idleStt();
    try {
      const m = await mountWindow();
      await openSettings(m);
      expect(m.calls.filter((c) => c === "showSettings")).toHaveLength(1);
    } finally {
      spy.mockRestore();
    }
  });

  test("AgentWindow read-only tab shows no gear", async () => {
    const spy = idleStt();
    try {
      const m = await mountWindow(READ_ONLY);
      expect(await gearOf(m)).toBeNull();
    } finally {
      spy.mockRestore();
    }
  });

  test("AgentWindowView sans vue reçue : chargement", async () => {
    const el = document.createElement("div");
    const root = createRoot(el);
    await act(async () => root.render(createElement(AgentWindowView, { view: null })));
    cleanup = async () => act(async () => root.unmount());
    expect(el.querySelector(".agent-window-loading")).not.toBeNull();
  });

  test("AgentWindowView sans onglet suivi : état vide", async () => {
    const el = document.createElement("div");
    const root = createRoot(el);
    const view = { generation: 1, focused: "w-9", agent_detached: true, import_pick: null, windows: [] };
    await act(async () => root.render(createElement(AgentWindowView, { view })));
    cleanup = async () => act(async () => root.unmount());
    expect(el.querySelector(".agent-window-empty")).not.toBeNull();
  });

  /** La page entière avec un hôte de fermeture à la main. */
  async function mountView(view: Parameters<typeof AgentWindowView>[0]["view"]) {
    const calls: string[] = [];
    let closing: () => void = () => {};
    const close = {
      onCloseRequested: (cb: () => void) => ((closing = cb), () => {}),
      attach: async () => void calls.push("attach"),
    };
    const el = document.createElement("div");
    const root = createRoot(el);
    await act(async () => root.render(createElement(AgentWindowView, { view, close })));
    cleanup = async () => act(async () => root.unmount());
    return { el, calls, closeWindow: () => act(async () => closing()) };
  }

  test("AgentWindowView chargement : la fermeture système rattache l'agent", async () => {
    const m = await mountView(null);
    await m.closeWindow();
    expect(m.calls).toEqual(["attach"]);
  });

  test("AgentWindowView sans onglet suivi : la fermeture système rattache l'agent", async () => {
    const m = await mountView({ generation: 1, focused: "w-9", agent_detached: true, import_pick: null, windows: [] });
    await m.closeWindow();
    expect(m.calls).toEqual(["attach"]);
  });

  test("AgentWindowView chargement : bouton rattacher, icône seule", async () => {
    const m = await mountView(null);
    expectIconOnly(m.el.querySelector(".agent-attach"));
    await act(async () => (m.el.querySelector(".agent-attach") as HTMLButtonElement).click());
    expect(m.calls).toEqual(["attach"]);
  });

  test("AgentWindowView sans onglet suivi : bouton rattacher présent", async () => {
    const m = await mountView({ generation: 1, focused: "w-9", agent_detached: true, import_pick: null, windows: [] });
    expectIconOnly(m.el.querySelector(".agent-attach"));
  });
});
