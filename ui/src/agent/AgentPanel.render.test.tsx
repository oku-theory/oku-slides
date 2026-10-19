// Le panneau de l'agent : un morceau de réponse ne re-rend ni `App` ni la scène (t-208).

import { afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { AgentScope, InterfaceChange, McpEvent, Surface } from "../api";
import { CATALOG_ID } from "../a2ui/catalog";
import type { HostHandlers } from "./events";
import type { ScopeSource } from "./AgentPanel";
import type { QuickTarget } from "./useQuickKeys";

const { act, createElement, createRef, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { AgentPanel, IDLE_AGENT } = await import("./AgentPanel");
const { createSlot, SlotView } = await import("./slot");
const { routeSurfaces } = await import("./surfaces");
const { ARMING_MS } = await import("../modalFrame");
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

const DRIVE: ScopeSource = { kind: "drive", view: { templates: false, history: false, pack: null, widget: null } };
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
  scope = DRIVE,
  keyPlaces,
}: { promptFails?: boolean; scope?: ScopeSource; keyPlaces?: QuickTarget["kind"][] } = {}) {
  const renders = { app: 0, stage: 0, pile: 0 };
  let handlers: HostHandlers | null = null;
  const subscribe = (_tab: string, h: HostHandlers) => {
    handlers = h;
    return () => {};
  };
  const prompts: string[] = [];
  const promptScopes: AgentScope[] = [];
  const resyncs: AgentScope[] = [];
  const follows: number[] = [];
  const actions: string[] = [];
  const drafts: { text: string | null; origin: string | null }[] = [];
  const pickers: number[] = [];
  const selected: string[] = [];
  const pickerSurfaces: Surface[] = [];
  const agent = {
    prompt: async (s: AgentScope, text: string) => {
      prompts.push(text);
      promptScopes.push(s);
      if (promptFails) throw new Error("no agent");
    },
    cancel: async () => {},
    newConversation: async () => {},
    surfaceAction: async (id: string, name: string) => void actions.push(`${id}:${name}`),
    resync: async (s: AgentScope) => void resyncs.push(s),
    follow: async () => void follows.push(1),
    setDraft: async () => {},
    picker: async () => void pickers.push(1),
    selectAgent: async (id: string) => void selected.push(id),
    downloadModel: async () => {},
    cancelDownload: async () => {},
  };
  const slots = { sheet: createSlot(), drawer: createSlot(), modal: createSlot() };
  const handle = createRef<import("./AgentPanel").AgentHandle>();
  const focus: [string, string | null][] = [];
  const presented: string[] = [];
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
    return createElement(
      "div",
      { "data-busy": String(status.busy), "data-modal": String(status.modal), "data-ready": String(status.ready) },
      createElement(Pile),
      createElement(Stage, { sheet: status.sheet }),
      createElement("main", { className: "drawer-probe" }, createElement(SlotView, { slot: slots.drawer })),
      createElement(AgentPanel, {
        tab: "t1",
        agent,
        scope,
        keyPlaces,
        route,
        ui: UI,
        onDraft: (e: { text: string | null; origin: string | null }) => void drafts.push(e),
        onPicker: (surface: Surface) => void pickerSurfaces.push(surface),
        keysOff: false,
        thread: true,
        thumbs: { cached: () => null, svg: () => null, current: () => undefined },
        slots,
        handle,
        onHostAction: () => false,
        onStatus: setStatus,
        onChanged: () => void changed++,
        onFocus: (slide: string, element: string | null) => void focus.push([slide, element]),
        onWidgetTouched: () => {},
        onPresent: (mode: string) => void presented.push(mode),
        onInterface: (change: InterfaceChange) => void interfaces.push(change),
        onOpenHistory: () => void histories++,
        say: (m: string) => void said.push(m),
        subscribe,
      }),
      createElement("div", { className: "modal-probe" }, createElement(SlotView, { slot: slots.modal })),
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
  return { host, renders, mcp, until, handlers: () => handlers!, handle, prompts, focus, presented, interfaces, histories: () => histories, changed: () => changed, said, slots, drafts, promptScopes, resyncs, follows, actions, pickers, selected, pickerSurfaces };
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
  await act(async () => p.handlers().onAgent({ kind: "status", state: "working", scope: { kind: "slides" } }));
  expect(p.host.firstElementChild?.getAttribute("data-busy")).toBe("true");
});

test("AgentPanel transmet focus et changements, et relit le deck à l'abonnement prêt", async () => {
  const p = await mount();
  await p.mcp({ kind: "focus", slide: "s2", element: "e9" });
  await p.mcp({ kind: "changed" });
  expect(p.focus).toEqual([["s2", "e9"]]);
  expect(p.changed()).toBe(1);
  await act(async () => p.handlers().onReady?.());
  expect(p.changed()).toBe(2);
});

test("AgentPanel transmet la présentation demandée par l'agent", async () => {
  const p = await mount();
  await p.mcp({ kind: "present", mode: "window" });
  expect(p.presented).toEqual(["window"]);
});

test("AgentPanel transmet les changements d'interface demandés par l'agent", async () => {
  const p = await mount();
  const change: InterfaceChange = { scheme: "dark", locale: null, grid: null, guides: true };
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
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.follows).toHaveLength(1);
});

test("AgentPanel suiveur à l'abonnement prêt ne demande jamais resync", async () => {
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  expect(m.resyncs).toEqual([]);
});

test("AgentPanel suiveur une consigne part dans la portée annoncée par l'hôte", async () => {
  const m = await mount({ scope: { kind: "follow" } });
  await act(async () => m.handlers().onReady?.());
  await m.mcp({ kind: "viewing", scope: { kind: "atelier", pack: "p-1" } });
  await act(async () => void (await m.handle.current!.prompt("bonjour")));
  expect(m.promptScopes).toEqual([{ kind: "atelier", pack: "p-1" }]);
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

/** Le panneau des agents ne vit plus dans le fil : il passe par `onPicker` (menu des réglages de la dictée). */
test("AgentPanel picker event calls onPicker with the surface", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "picker", surface: PICKER }));
  expect(m.pickerSurfaces).toEqual([PICKER]);
});

test("AgentPanel thread has no agents button nor picker sheet", async () => {
  const m = await mount();
  await act(async () => m.handlers().onAgent({ kind: "picker", surface: PICKER }));
  await act(async () => new Promise((r) => setTimeout(r, 0)));
  expect(m.host.querySelector(".agent-picker-toggle, .agent-picker")).toBeNull();
  expect(m.pickers).toEqual([]);
});

/** Une touche frappée depuis `target` (elle remonte jusqu'à la fenêtre). */
const keyFrom = async (target: Element, key: string, code: string) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key, code, bubbles: true, cancelable: true })));

test("AgentPanel digit key inside the agents picker sends no answer to the sheet question", async () => {
  const m = await mount();
  await m.mcp({ kind: "surface", surface: sheet });
  await m.until(() => (m.host.querySelector(".stage-probe")?.textContent ?? "").includes("Choisir"));
  await act(async () => new Promise((r) => setTimeout(r, ARMING_MS + 20)));
  // Le panneau des agents, posé hors du fil (menu des réglages de la dictée).
  const picker = document.createElement("div");
  picker.className = "agent-picker";
  const chip = document.createElement("button");
  picker.append(chip);
  document.body.append(picker);
  try {
    await keyFrom(chip, "1", "Digit1");
    expect(m.actions).toEqual([]);
    // Témoin : la même touche hors du panneau répond bien à la question.
    await keyFrom(document.body, "1", "Digit1");
    expect(m.actions).toHaveLength(1);
  } finally {
    picker.remove();
  }
});
