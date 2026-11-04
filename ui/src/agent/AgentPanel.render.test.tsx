// Le panneau de l'agent : un morceau de réponse ne re-rend ni `App` ni la scène (t-208).

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import type { InterfaceChange, McpEvent, Surface } from "../api";
import { CATALOG_ID } from "../a2ui/catalog";
import type { HostHandlers } from "./events";
import type { ThreadMode } from "./AgentPanel";
import type { PresentCommand } from "../present";
import type { QuickTarget } from "./useQuickKeys";

const { act, createElement, createRef, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentPanel, IDLE_AGENT } = await import("./AgentPanel");
const { createSlot, SlotView } = await import("./slot");
const { routeSurfaces } = await import("./surfaces");
const { ARMING_MS } = await import("../modalFrame");
const { t } = await import("../i18n");
const { AGENT_WINDOW_PLACES, DETACHED_MAIN_PLACES } = await import("./useQuickKeys");
const { realSurface } = await import("../a2ui/fixtures/agentSurfaces");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Une surface avec un bouton `label`. */
const surface = (id: string, label: string, extra: Partial<Surface>): Surface => ({
  id,
  place: "thread",
  live: true,
  question: false,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: id, catalogId: CATALOG_ID } },
    { version: "v0.9", updateComponents: { surfaceId: id, components: [button(label)] } },
  ] as unknown as Surface["messages"],
  ...extra,
});
const button = (label: string) => ({ id: "root", component: "Button", label, action: { event: { name: "ok", context: {} } } });
/** Un morceau de réponse : le bouton de la surface `id` change de libellé. */
const patch = (id: string, label: string): McpEvent => ({
  kind: "surface_patch",
  id,
  messages: [{ version: "v0.9", updateComponents: { surfaceId: id, components: [button(label)] } }] as unknown as Surface["messages"],
});

const DRIVE: ThreadMode = { kind: "drive", loaded: true };
const UI = { slide: "s1", element: null, field: null, editing: false, widget: null };
const route = (list: Surface[]) => routeSurfaces(list, { slide: "s1", has: () => true });
/** Échap, une fois la fenêtre armée (un geste trop tôt est ignoré). */
const escape = async () => {
  await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
};

/** Monte le panneau dans un hôte qui compte ses rendus, avec une scène et une pile qui comptent les leurs. */
async function mount({
  promptFails = false,
  mode = DRIVE,
  keyPlaces,
  fallback,
  offersInline,
  inspector = true,
  anchors = [],
}: {
  promptFails?: boolean;
  mode?: ThreadMode;
  keyPlaces?: QuickTarget["kind"][];
  fallback?: string | null;
  offersInline?: boolean;
  /** The tab can show the inspector (a deck tab): it gets `onOpenInspector`. */
  inspector?: boolean;
  /** Extra `data-popover-anchor` buttons the window renders (the tools tray's). */
  anchors?: string[];
} = {}) {
  let voices = 0;
  const fallbacks: (string | null)[] = [];
  const renders = { app: 0, stage: 0, pile: 0 };
  let handlers: HostHandlers | null = null;
  const subscribe = (_tab: string, h: HostHandlers) => {
    handlers = h;
    return () => {};
  };
  const prompts: string[] = [];
  const promptArgs: unknown[][] = [];
  let resyncs = 0;
  let newConversations = 0;
  const follows: number[] = [];
  const actions: string[] = [];
  const hostActions: string[] = [];
  const drafts: { text: string | null; origin: string | null }[] = [];
  const agent = {
    prompt: async (...args: unknown[]) => {
      prompts.push(args[0] as string);
      promptArgs.push(args);
      if (promptFails) throw new Error("no agent");
    },
    cancel: async () => {},
    newConversation: async (...args: unknown[]) => void (args.length === 0 && newConversations++),
    surfaceAction: async (id: string, name: string) => void actions.push(`${id}:${name}`),
    resync: async () => void resyncs++,
    follow: async () => void follows.push(1),
    setDraft: async () => {},
    showSettings: async () => {},
  };
  const slots = { sheet: createSlot(), drawer: createSlot(), modal: createSlot(), popover: createSlot(), inspector: createSlot() };
  let inspectorOpens = 0;
  let select: (id: string | null) => void = () => {};
  const handle = createRef<import("./AgentPanel").AgentHandle>();
  const focus: (string | null)[] = [];
  const presented: PresentCommand[] = [];
  const interfaces: InterfaceChange[] = [];
  let histories = 0;
  let changed = 0;
  const said: string[] = [];
  const Stage = ({ sheet }: { sheet: { owner: string; field?: string } | null }) => {
    renders.stage++;
    const at = sheet ? [sheet.owner, sheet.field].filter(Boolean).join("/") : "";
    return createElement("div", { className: "stage-probe", "data-sheet": at }, sheet ? createElement(SlotView, { slot: slots.sheet }) : null);
  };
  const Pile = () => {
    renders.pile++;
    return createElement("div", { className: "pile-probe" });
  };
  const App = () => {
    renders.app++;
    const [status, setStatus] = useState(IDLE_AGENT);
    const [selected, setSelected] = useState<string | null>(null);
    select = setSelected;
    return createElement(
      "div",
      { "data-busy": String(status.busy), "data-modal": String(status.modal), "data-ready": String(status.ready) },
      createElement(Pile),
      createElement(Stage, { sheet: status.sheet }),
      createElement("main", { className: "drawer-probe" }, createElement(SlotView, { slot: slots.drawer })),
      createElement(AgentPanel, {
        tab: "t1",
        agent,
        mode,
        keyPlaces,
        route,
        ui: UI,
        onDraft: (e: { text: string | null; origin: string | null }) => void drafts.push(e),
        onVoice: () => void voices++,
        onFallback: (reason: string | null) => void fallbacks.push(reason),
        fallback,
        offersInline,
        selected,
        onOpenInspector: inspector ? () => void inspectorOpens++ : undefined,
        keysOff: false,
        thread: true,
        thumbs: { cached: () => null, svg: () => null, current: () => undefined },
        slots,
        handle,
        onHostAction: (s: Surface, a: { name: string }) => (hostActions.push(`${s.id}:${a.name}`), false),
        onStatus: setStatus,
        onChanged: () => void changed++,
        onFocus: (slide: string | null) => void focus.push(slide),
        onWidgetTouched: () => {},
        onPresent: (command: PresentCommand) => void presented.push(command),
        onInterface: (change: InterfaceChange) => void interfaces.push(change),
        onOpenHistory: () => void histories++,
        say: (m: string) => void said.push(m),
        subscribe,
      }),
      createElement("div", { className: "modal-probe" }, createElement(SlotView, { slot: slots.modal })),
      createElement("button", { className: "packs-probe", "data-popover-anchor": "host-packs" }),
      ...anchors.map((name) => createElement("button", { key: name, "data-popover-anchor": name })),
      createElement("div", { className: "popover-probe" }, createElement(SlotView, { slot: slots.popover })),
      createElement("aside", { className: "inspector-probe" }, createElement(SlotView, { slot: slots.inspector })),
    );
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(App)));
  const mcp = async (e: McpEvent) => {
    await act(async () => handlers!.onMcp(e));
    // Les surfaces sont chargées à la demande (lazy) : laisser Suspense les résoudre.
    await act(async () => new Promise((r) => setTimeout(r, 0)));
  };
  /** Attend que `cond` soit vraie : le premier chargement des surfaces (lazy) prend plusieurs tours. */
  const until = async (cond: () => boolean) => {
    for (let i = 0; i < 50 && !cond(); i++) await act(async () => new Promise((r) => setTimeout(r, 5)));
  };
  const reselect = async (id: string | null) => act(async () => select(id));
  return { inspectorOpens: () => inspectorOpens, reselect, host, renders, mcp, until, handlers: () => handlers!, handle, prompts, focus, presented, interfaces, histories: () => histories, changed: () => changed, said, slots, drafts, promptArgs, resyncs: () => resyncs, newConversations: () => newConversations, follows, actions, hostActions, voices: () => voices, fallbacks };
}

const sheet = surface("q-1", "Choisir", { place: "sheet", live: true, question: true, anchor: { slide: "s1", owner: "e1" } });
const dock = surface("set-1", "Régler", {});
const drawer = surface("v-1", "Variante", { place: "drawer" });
const modal = surface("m-1", "Confirmer", { place: "modal", question: true });

test("AgentPanel un morceau de réponse (surface_patch) ne re-rend ni App ni la scène ni la pile", async () => {
  const p = await mount();
  await p.mcp({ kind: "surfaces", list: [sheet, dock, drawer] });
  const text = (sel: string) => p.host.querySelector(sel)?.textContent ?? "";
  await p.until(() => text(".stage-probe").includes("Choisir") && text(".drawer-probe").includes("Variante"));
  // La fiche est posée sur la scène, le tiroir dessous, le réglage au fil.
  expect(p.host.querySelector(".stage-probe")?.textContent).toContain("Choisir");
  expect(p.host.querySelector(".drawer-probe")?.textContent).toContain("Variante");
  expect(p.host.querySelector(".agent-thread")?.textContent).toContain("Régler");
  const before = { ...p.renders };
  await p.mcp(patch("q-1", "Choisir encore"));
  await p.mcp(patch("set-1", "Régler encore"));
  await p.mcp(patch("v-1", "Variante encore"));
  // Les morceaux sont montrés…
  expect(p.host.querySelector(".stage-probe")?.textContent).toContain("Choisir encore");
  expect(p.host.querySelector(".agent-thread")?.textContent).toContain("Régler encore");
  expect(p.host.querySelector(".drawer-probe")?.textContent).toContain("Variante encore");
  // …sans re-rendre l'hôte, la scène ni la pile.
  expect(p.renders).toEqual(before);
});

test("AgentPanel une fiche qui arrive ou s'en va re-rend App (la scène reçoit sa cible)", async () => {
  const p = await mount();
  const before = p.renders.app;
  await p.mcp({ kind: "surface", surface: sheet });
  expect(p.renders.app).toBeGreaterThan(before);
  expect(p.host.querySelector(".stage-probe")?.getAttribute("data-sheet")).toBe("e1");
  await p.mcp({ kind: "close_surface", id: "q-1" });
  expect(p.host.querySelector(".stage-probe")?.getAttribute("data-sheet")).toBe("");
});

test("AgentPanel un tour en cours est transmis à App (occupé)", async () => {
  const p = await mount();
  await act(async () => p.handlers().onAgent({ kind: "status", state: "working" }));
  expect(p.host.firstElementChild?.getAttribute("data-busy")).toBe("true");
});

test("AgentPanel transmet focus et changements, et relit le deck à l'abonnement prêt", async () => {
  const p = await mount();
  await p.mcp({ kind: "focus", slide: "s2" });
  await p.mcp({ kind: "changed" });
  expect(p.focus).toEqual(["s2"]);
  expect(p.changed()).toBe(1);
  await act(async () => p.handlers().onReady?.());
  expect(p.changed()).toBe(2);
});

test("AgentPanel transmet la présentation demandée par l'agent", async () => {
  const p = await mount();
  await p.mcp({ kind: "present", command: { action: "start", mode: "window" } });
  await p.mcp({ kind: "present", command: { action: "next" } });
  expect(p.presented).toEqual([{ action: "start", mode: "window" }, { action: "next" }]);
});

test("AgentPanel transmet les changements d'interface demandés par l'agent", async () => {
  const p = await mount();
  const change: InterfaceChange = { scheme: "dark", locale: null, guides: true, folded: null, pile: null, notes: null, code: null };
  await p.mcp({ kind: "interface", change });
  expect(p.interfaces).toEqual([change]);
});

test("AgentPanel transmet l'ouverture de l'historique demandée par l'agent", async () => {
  const p = await mount();
  await p.mcp({ kind: "open_history" });
  expect(p.histories()).toBe(1);
});

test("AgentPanel la poignée envoie une consigne à l'agent et dit qu'elle est partie", async () => {
  const p = await mount();
  let ok: boolean | null = null;
  await act(async () => void (ok = await p.handle.current!.prompt("Ajoute une slide")));
  expect(p.prompts).toEqual(["Ajoute une slide"]);
  expect(ok).toBe(true);
});

test("AgentPanel une consigne qui échoue est signalée et rend faux (t-211)", async () => {
  const p = await mount({ promptFails: true });
  let ok: boolean | null = null;
  await act(async () => void (ok = await p.handle.current!.prompt("Ajoute une slide")));
  expect(ok).toBe(false);
  expect(p.said).toEqual(["Error: no agent"]);
});

test("AgentPanel transmet à App que l'agent est prêt une fois le fil rappelé (t-211)", async () => {
  const p = await mount();
  expect(p.host.firstElementChild?.getAttribute("data-ready")).toBe("false");
  await act(async () => p.handlers().onReady?.());
  expect(p.host.firstElementChild?.getAttribute("data-ready")).toBe("true");
});

test("AgentPanel un morceau de réponse d'une fenêtre ne re-rend pas App", async () => {
  const p = await mount();
  await p.mcp({ kind: "surfaces", list: [modal] });
  const text = () => p.host.querySelector(".modal-probe")?.textContent ?? "";
  await p.until(() => text().includes("Confirmer"));
  expect(p.host.firstElementChild?.getAttribute("data-modal")).toBe("true");
  const before = { ...p.renders };
  await p.mcp(patch("m-1", "Confirmer encore"));
  expect(text()).toContain("Confirmer encore");
  expect(p.renders).toEqual(before);
});

test("AgentPanel suiveur à l'abonnement prêt demande follow", async () => {
  const m = await mount({ mode: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.follows).toHaveLength(1);
});

test("AgentPanel suiveur à l'abonnement prêt ne demande jamais resync", async () => {
  const m = await mount({ mode: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.resyncs()).toBe(0);
});

test("AgentPanel sends a prompt without scope", async () => {
  const m = await mount();
  await act(async () => void (await m.handle.current!.prompt("bonjour")));
  expect(m.promptArgs).toEqual([["bonjour", UI, null, []]]);
});

test("AgentPanel new conversation calls newConversation without arguments", async () => {
  const m = await mount();
  await m.mcp({ kind: "surface", surface: dock });
  await m.until(() => m.host.querySelector(".agent-new") !== null);
  const button = m.host.querySelector<HTMLButtonElement>(".agent-new");
  expect(button).not.toBeNull();
  await act(async () => void button!.click());
  expect(m.newConversations()).toBe(1);
});

test("AgentPanel toutes places : Échap répond à la fenêtre de confirmation", async () => {
  const m = await mount();
  await m.mcp({ kind: "surface", surface: modal });
  await escape();
  expect(m.actions.some((a) => a.startsWith("m-1:"))).toBe(true);
});

test("AgentPanel touches limitées au dock : Échap laisse la fenêtre de confirmation", async () => {
  const m = await mount({ keyPlaces: ["dock"] });
  await m.mcp({ kind: "surface", surface: modal });
  await escape();
  expect(m.actions).toEqual([]);
});

test("AgentPanel un brouillon reçu de l'hôte est transmis", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "draft", text: "x", origin: "v-2" }));
  expect(m.drafts).toEqual([{ text: "x", origin: "v-2" }]);
});

test("AgentPanel voice event calls onVoice", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "voice" }));
  expect(m.voices()).toBe(1);
});

test("AgentPanel fallback event calls onFallback with its reason", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "fallback", reason: "Agent introuvable : npx." }));
  await act(async () => m.handlers().onAgent({ kind: "fallback", reason: null }));
  expect(m.fallbacks).toEqual(["Agent introuvable : npx.", null]);
});

test("AgentPanel shows the local agent badge with its reason while fallback is set", async () => {
  const m = await mount({ fallback: "Agent introuvable : npx." });
  await m.mcp({ kind: "surfaces", list: [dock] });
  const badge = m.host.querySelector(".agent-thread-head .agent-fallback");
  expect(badge?.textContent).toBe(t("ui-agent-fallback"));
  expect(badge?.getAttribute("title")).toBe("Agent introuvable : npx.");
});

test("AgentPanel shows no local agent badge without fallback", async () => {
  const m = await mount({ fallback: null });
  await m.mcp({ kind: "surfaces", list: [dock] });
  expect(m.host.querySelector(".agent-thread-head")).not.toBeNull();
  expect(m.host.querySelector(".agent-fallback")).toBeNull();
});

// Une reconnexion a pu manquer des évènements : la dictée relit son état.
test("AgentPanel reconnect calls onVoice", async () => {
  const m = await mount();
  await act(async () => m.handlers().onReady?.());
  expect(m.voices()).toBe(1);
});

/** Une touche frappée depuis `target` (elle remonte jusqu'à la fenêtre). */
const keyFrom = async (target: Element, key: string, code: string) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key, code, bubbles: true, cancelable: true })));

test("AgentPanel quick digit inside the voice box answers no question", async () => {
  const m = await mount();
  await m.mcp({ kind: "surface", surface: sheet });
  await m.until(() => (m.host.querySelector(".stage-probe")?.textContent ?? "").includes("Choisir"));
  await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
  // Une puce de la boîte des réglages, focalisée au clavier.
  const box = document.createElement("div");
  box.setAttribute("data-surface-id", "host-voice");
  const chip = document.createElement("button");
  chip.className = "chip";
  box.append(chip);
  document.body.append(box);
  try {
    await keyFrom(chip, "1", "Digit1");
    expect(m.actions).toEqual([]);
    // Témoin : la même touche hors de la boîte répond bien à la question.
    await keyFrom(document.body, "1", "Digit1");
    expect(m.actions).toHaveLength(1);
  } finally {
    box.remove();
  }
});


describe("AgentPanel popovers", () => {
  /** The packs box: turnless when the user opened it, in `turn` when an agent tool did. */
  const packs = (turn?: number): Surface => surface("host-packs", "Fermer", { place: "popover", turn });
  const turnOf = (n: number) => surface(`turn-${n}`, "Tour", { live: false, turn: n });

  test("AgentPanel with a turnless popover opens it next to its anchor", async () => {
    const { host, mcp, until } = await mount();
    await mcp({ kind: "surface", surface: packs() });
    await until(() => !!host.querySelector(".agent-popover"));
    expect(host.querySelector('.popover-probe .agent-popover [data-surface-id="host-packs"]')).not.toBeNull();
    expect(host.querySelector(".agent-thread .agent-offer")).toBeNull();
  });

  test("AgentPanel popover on Escape dismisses it on the hub", async () => {
    const { host, mcp, actions } = await mount();
    await mcp({ kind: "surface", surface: packs() });
    await act(async () => {
      host.querySelector(".agent-popover [data-surface-id]")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    });
    expect(actions).toEqual(["host-packs:dismiss"]);
  });

  test("AgentPanel popover on a second click of its anchor dismisses it", async () => {
    const { host, mcp, actions } = await mount();
    await mcp({ kind: "surface", surface: packs() });
    let reached = 0;
    const anchor = host.querySelector(".packs-probe") as HTMLButtonElement;
    anchor.addEventListener("click", () => reached++);
    await act(async () => anchor.click());
    expect([actions, reached]).toEqual([["host-packs:dismiss"], 0]);
  });

  test("AgentPanel popover click goes to the host first (it opens pack and theme tabs)", async () => {
    const { host, mcp, until, hostActions } = await mount();
    await mcp({ kind: "surface", surface: packs() });
    await until(() => !!host.querySelector(".agent-popover button"));
    await act(async () => (host.querySelector(".agent-popover button") as HTMLButtonElement).click());
    expect(hostActions).toEqual(["host-packs:ok"]);
  });

  test("AgentPanel with an agent-opened box shows a chip, not the popover", async () => {
    const { host, mcp } = await mount();
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: packs(3) });
    expect([!!host.querySelector(".agent-turn .agent-offer"), !!host.querySelector(".agent-popover")]).toEqual([true, false]);
  });

  test("AgentPanel turnless popover after a chip-opened one opens at its own button", async () => {
    const { host, mcp, until } = await mount();
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: packs(3) });
    await act(async () => (host.querySelector(".agent-offer") as HTMLButtonElement).click());
    await mcp({ kind: "close_surface", id: "host-packs" });
    await mcp({ kind: "surface", surface: packs() });
    await until(() => !!host.querySelector(".agent-popover"));
    expect(host.querySelector('.popover-probe .agent-popover [data-surface-id="host-packs"]')).not.toBeNull();
  });

  test("AgentPanel popover opened from a button gone since opens at its own button", async () => {
    const { host, mcp, until } = await mount();
    const inspector = document.createElement("button");
    inspector.setAttribute("data-popover-anchor", "host-packs@inspector");
    document.body.append(inspector);
    await act(async () => inspector.click());
    inspector.remove();
    await mcp({ kind: "surface", surface: packs() });
    await until(() => !!host.querySelector(".agent-popover"));
    expect(host.querySelector('.popover-probe .agent-popover [data-surface-id="host-packs"]')).not.toBeNull();
  });

  test("AgentPanel chip click opens the offer next to the chip", async () => {
    const { host, mcp } = await mount();
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: packs(3) });
    await act(async () => (host.querySelector(".agent-offer") as HTMLButtonElement).click());
    await act(async () => new Promise((r) => setTimeout(r, 0)));
    expect(host.querySelector('.popover-probe .agent-popover [data-surface-id="host-packs"]')).not.toBeNull();
  });
});

describe("AgentPanel inspector offers", () => {
  /** The agent's settings offered in the inspector, filed in turn 3. */
  const params = surface("show-4", "Régler", { place: "inspector", turn: 3 });
  const turnOf = (n: number) => surface(`turn-${n}`, "Tour", { live: false, turn: n });
  const chip = (host: HTMLElement) => host.querySelector(".agent-turn .agent-offer") as HTMLButtonElement;
  const inInspector = (host: HTMLElement) => !!host.querySelector('.inspector-probe [data-surface-id="show-4"]');

  test("AgentPanel offer chip click fills the inspector slot and opens the inspector", async () => {
    const { host, mcp, until, inspectorOpens } = await mount();
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: params });
    await act(async () => chip(host).click());
    await until(() => inInspector(host));
    expect([inInspector(host), inspectorOpens(), !!host.querySelector('.agent-thread [data-surface-id="show-4"]')]).toEqual([true, 1, false]);
  });

  test("AgentPanel offer in the inspector closes when the selection changes", async () => {
    const { host, mcp, until, reselect } = await mount();
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: params });
    await act(async () => chip(host).click());
    await until(() => inInspector(host));
    const before = inInspector(host);
    await reselect("e-2");
    expect([before, inInspector(host)]).toEqual([true, false]);
  });

  test("AgentPanel offer chip in the agent window opens under its chip, not in the inspector", async () => {
    const { host, mcp, until, inspectorOpens } = await mount({ offersInline: true });
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: params });
    await act(async () => chip(host).click());
    await until(() => !!host.querySelector('.agent-offer-item [data-surface-id="show-4"]'));
    expect([!!host.querySelector('.agent-offer-item [data-surface-id="show-4"]'), inInspector(host), inspectorOpens()]).toEqual([true, false, 0]);
  });
});

describe("AgentPanel theme picker keys with the agent detached", () => {
  /** The theme picker the user opened from the tools tray: a turnless popover. */
  const picker = (): Surface => ({ ...realSurface("host-theme-picker"), place: "popover", live: true, turn: undefined });

  test("AgentPanel in the main window, detached, gives the shown picker digits", async () => {
    const m = await mount({ keyPlaces: DETACHED_MAIN_PLACES, anchors: ["host-theme-picker"] });
    await m.mcp({ kind: "surface", surface: picker() });
    await m.until(() => !!m.host.querySelector('.popover-probe [data-surface-id="host-theme-picker"]'));
    await keyFrom(document.body, "1", "Digit1");
    expect(m.actions.map((a) => a.split(":")[0])).toEqual(["host-theme-picker"]);
  });

  test("AgentPanel in the agent window, without the picker's anchor, takes no digit for it", async () => {
    const m = await mount({ keyPlaces: AGENT_WINDOW_PLACES });
    await m.mcp({ kind: "surface", surface: picker() });
    await act(async () => new Promise((r) => setTimeout(r, 20)));
    await keyFrom(document.body, "1", "Digit1");
    expect([!!m.host.querySelector(".agent-popover"), m.actions]).toEqual([false, []]);
  });

  test("AgentPanel with the picker's anchor gone takes no digit for it", async () => {
    const m = await mount();
    await m.mcp({ kind: "surface", surface: picker() });
    await act(async () => new Promise((r) => setTimeout(r, 20)));
    await keyFrom(document.body, "1", "Digit1");
    expect(m.actions).toEqual([]);
  });
});

describe("AgentPanel inspector offer where the tab cannot show the inspector", () => {
  const params = surface("show-4", "Régler", { place: "inspector", turn: 3 });
  const turnOf = (n: number) => surface(`turn-${n}`, "Tour", { live: false, turn: n });

  test("AgentPanel offer chip without an inspector opens under its chip", async () => {
    const { host, mcp, until } = await mount({ inspector: false });
    await mcp({ kind: "surface", surface: turnOf(3) });
    await mcp({ kind: "surface", surface: params });
    await act(async () => (host.querySelector(".agent-turn .agent-offer") as HTMLButtonElement).click());
    await until(() => !!host.querySelector('.agent-offer-item [data-surface-id="show-4"]'));
    expect([
      !!host.querySelector('.agent-offer-item [data-surface-id="show-4"]'),
      !!host.querySelector('.inspector-probe [data-surface-id="show-4"]'),
    ]).toEqual([true, false]);
  });
});
