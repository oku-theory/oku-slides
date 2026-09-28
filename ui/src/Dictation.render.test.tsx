// Dictée en deux modes, dans un DOM happy-dom, avec un faux moteur de dictée.

import "./test/dom";
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import type { SttState, SttStatus } from "./api";
import { stt } from "./api";
import type { DictationMode } from "./agent/keys";

// Voir a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Dictation } = await import("./Dictation");
const { expectIconOnly } = await import("./test/iconOnly");

/** Un faux moteur : `start` écoute, `stop` rend `heard` (seq + 1), `cancel` revient au repos. */
function fakeStt(heard = "bonjour") {
  let current: SttStatus = {
    state: "idle",
    config: { model: "turbo", language: null, auto_stop: false, silence: 2 },
    models: [{ id: "turbo", label: "Large v3 Turbo (rapide)", size: 1, installed: true }],
    download: null,
    level: 0,
    speaking: false,
    committed: "",
    partial: "",
    detected: null,
    stop_in: null,
    text: null,
    seq: 0,
    seconds: 0,
    error: null,
  };
  const set = (state: SttState, change: Partial<SttStatus> = {}) => {
    current = { ...current, state, ...change };
    return Promise.resolve(current);
  };
  const calls: string[] = [];
  const spies = [
    spyOn(stt, "status").mockImplementation(() => Promise.resolve(current)),
    spyOn(stt, "start").mockImplementation(() => (calls.push("start"), set("recording"))),
    spyOn(stt, "stop").mockImplementation(() => (calls.push("stop"), set("idle", { seq: current.seq + 1, text: heard }))),
    spyOn(stt, "cancel").mockImplementation(() => (calls.push("cancel"), set("idle"))),
  ];
  return { calls, restore: () => spies.forEach((s) => s.mockRestore()) };
}

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement; restore: () => void } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
  m.restore();
});

let engine: ReturnType<typeof fakeStt>;
beforeEach(() => {
  engine = fakeStt();
});

/** Laisse les promesses du faux moteur se résoudre et React les rendre. */
const settle = () => act(async () => new Promise((r) => setTimeout(r, 0)));

async function mount(agentBusy = false) {
  const texts: [string, DictationMode][] = [];
  let cancels = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host, restore: engine.restore };
  const render = (busy: boolean) =>
    root.render(
      createElement(Dictation, {
        destination: "le titre",
        prompt: "",
        onText: (text: string, mode: DictationMode) => texts.push([text, mode]),
        onError: () => {},
        agentBusy: busy,
        onAgentCancel: () => cancels++,
      }),
    );
  await act(async () => render(agentBusy));
  await settle();
  return {
    host,
    texts,
    cancels: () => cancels,
    rerender: async (busy: boolean) => {
      await act(async () => render(busy));
      await settle();
    },
  };
}

/** Appuie sur une touche comme l'utilisateur (l'écouteur de la dictée est sur window). */
async function press(key: string, mods: { meta?: boolean; shift?: boolean } = {}) {
  await act(async () => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key,
        code: key.length === 1 ? `Key${key.toUpperCase()}` : key,
        metaKey: !!mods.meta,
        shiftKey: !!mods.shift,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await settle();
}

const agentButton = (host: HTMLElement) => host.querySelector(".agent-mic") as HTMLButtonElement;

describe("dictée au repos", () => {
  test("Dictation idle shows only the mic, details in its tooltip", async () => {
    const { host } = await mount();
    expect(host.querySelector(".dictation-line")).toBeNull();
    expectIconOnly(host.querySelector(".mic"));
    expect(host.querySelector(".mic")?.getAttribute("title")).toContain("le titre");
  });

  test("Dictation cancel button while recording shows an icon only", async () => {
    const { host } = await mount();
    await press("d", { meta: true });
    expectIconOnly(host.querySelector(".dictation-cancel"));
  });
});

describe("dictée en deux modes", () => {
  test("⌘D dicte en mode auto : l'app choisit curseur ou brouillon", async () => {
    const { texts } = await mount();
    await press("d", { meta: true });
    await press("d", { meta: true });
    expect(texts).toEqual([["bonjour", "auto"]]);
  });

  test("⌘⇧D dicte dans le brouillon, affiché pendant l'écoute", async () => {
    const { host, texts } = await mount();
    await press("d", { meta: true, shift: true });
    expect(host.querySelector(".device")?.classList.contains("draft")).toBe(true);
    await press("d", { meta: true, shift: true });
    expect(texts).toEqual([["bonjour", "draft"]]);
  });

  test("la dictée garde le mode de son début, même arrêtée par ⌘D", async () => {
    const { texts } = await mount();
    await press("d", { meta: true, shift: true });
    await press("d", { meta: true });
    expect(texts).toEqual([["bonjour", "draft"]]);
  });

  test("au repos, le mode brouillon ne reste pas affiché", async () => {
    const { host } = await mount();
    await press("d", { meta: true, shift: true });
    await press("d", { meta: true, shift: true });
    expect(host.querySelector(".device")?.classList.contains("draft")).toBe(false);
    expect(host.querySelector(".mic")?.getAttribute("title")).toContain("le titre");
  });

  test("⌘⇧D pendant un tour arrête l'agent sans écouter", async () => {
    const { cancels } = await mount(true);
    await press("d", { meta: true, shift: true });
    expect(cancels()).toBe(1);
    expect(engine.calls).toEqual([]);
  });

  test("pendant une écoute, ⌘⇧D l'arrête même si un tour a commencé", async () => {
    const { texts, cancels, rerender } = await mount();
    await press("d", { meta: true, shift: true });
    await rerender(true);
    await press("d", { meta: true, shift: true });
    expect(cancels()).toBe(0);
    expect(texts).toEqual([["bonjour", "draft"]]);
  });

  test("Échap annule une écoute ; au repos, la dictée laisse Échap à l'app", async () => {
    const { cancels, rerender } = await mount();
    await press("d", { meta: true });
    await press("Escape");
    expect(engine.calls).toEqual(["start", "cancel"]);
    await rerender(true);
    await press("Escape");
    expect(cancels()).toBe(0);
    expect(engine.calls).toEqual(["start", "cancel"]);
  });

  test("un seul micro au repos : le bouton de l'agent n'apparaît que pour arrêter un tour", async () => {
    const { host, cancels, rerender } = await mount();
    expect(agentButton(host)).toBeNull();
    await rerender(true);
    expect(agentButton(host).getAttribute("aria-label")).toBe("ui-dictation-agent-stop");
    await act(async () => agentButton(host).click());
    expect(cancels()).toBe(1);
  });
});
