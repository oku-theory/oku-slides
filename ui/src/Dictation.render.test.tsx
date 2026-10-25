// Dictée en deux modes, dans un DOM happy-dom, avec un faux moteur de dictée.

import "./test/dom";
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import type { SttState, SttStatus } from "./api";
import { stt } from "./api";
import type { DictationMode } from "./agent/keys";
import type { LinkShortcut } from "./agent/link";
import { t } from "./i18n";
import { resetMessages } from "./test/messages";

// Voir a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Dictation } = await import("./Dictation");
const { expectIconOnly } = await import("./test/iconOnly");
const { bumpVoiceSeq } = await import("./voiceSeq");

/** Un faux moteur : `start` écoute, `stop` rend `heard` (seq + 1), `cancel` revient au repos. */
function fakeStt(heard = "bonjour", change: Partial<SttStatus> = {}) {
  let current: SttStatus = {
    state: "idle",
    config: { model: "turbo", language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", query: [], headers: [] } },
    models: [{ id: "turbo", label: "Large v3 Turbo (rapide)", size: 1, installed: true, available: true }],
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
    api_missing: null,
    ...change,
  };
  const set = (state: SttState, change: Partial<SttStatus> = {}) => {
    current = { ...current, state, ...change };
    return Promise.resolve(current);
  };
  const calls: string[] = [];
  const starts: string[][] = [];
  // Une réponse retenue : le prochain `stt.status()` attend que le test la rende.
  let held: Promise<SttStatus> | null = null;
  const statusSpy = spyOn(stt, "status").mockImplementation(() => {
    const p = held ?? Promise.resolve(current);
    held = null;
    return p;
  });
  const spies = [
    statusSpy,
    spyOn(stt, "start").mockImplementation((vocabulary: string[]) => (calls.push("start"), starts.push(vocabulary), set("recording"))),
    spyOn(stt, "stop").mockImplementation(() => (calls.push("stop"), set("idle", { seq: current.seq + 1, text: heard }))),
    spyOn(stt, "cancel").mockImplementation(() => (calls.push("cancel"), set("idle"))),
  ];
  return {
    calls,
    starts,
    statusCalls: () => statusSpy.mock.calls.length,
    current: (): SttStatus => current,
    /** Retient la réponse du prochain `stt.status()` ; `resolve` la rend, fusionnée sur l'état courant. */
    defer(): { resolve(s: Partial<SttStatus>): void } {
      let resolve: (s: SttStatus) => void = () => {};
      held = new Promise((r) => (resolve = r));
      return { resolve: (change) => resolve({ ...current, ...change }) };
    },
    restore: () => spies.forEach((s) => s.mockRestore()),
  };
}

// Remise à zéro même si une assertion échoue en cours de test.
afterEach(resetMessages);

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

type Remote = { seq: number; key: LinkShortcut };

async function mount(agentBusy = false, vocabulary: string[] = [], remote?: Remote, opts: { onSettings?: boolean } = {}) {
  const texts: [string, DictationMode][] = [];
  let cancels = 0;
  let typed = 0;
  let settings = 0;
  const host = document.createElement("div");
  document.body.append(host);
  let root = createRoot(host);
  mounted = { root, host, restore: engine.restore };
  const onSettings = (opts.onSettings ?? true) ? () => void settings++ : undefined;
  const render = (busy: boolean, shortcut?: Remote) =>
    root.render(
      createElement(Dictation, {
        destination: "le titre",
        vocabulary,
        onText: (text: string, mode: DictationMode) => texts.push([text, mode]),
        onError: () => {},
        agentBusy: busy,
        onAgentCancel: () => cancels++,
        onType: () => typed++,
        remote: shortcut,
        onSettings,
      }),
    );
  await act(async () => render(agentBusy, remote));
  await settle();
  return {
    host,
    texts,
    cancels: () => cancels,
    typed: () => typed,
    settingsClicks: () => settings,
    rerender: async (busy: boolean) => {
      await act(async () => render(busy, remote));
      await settle();
    },
    /** Même composant, rendu à nouveau avec ce raccourci relayé. */
    rerenderRemote: async (shortcut: Remote) => {
      await act(async () => render(agentBusy, shortcut));
      await settle();
    },
    /** Un composant neuf, monté avec ce raccourci relayé. */
    remount: async (shortcut: Remote) => {
      await act(async () => root.unmount());
      root = createRoot(host);
      mounted = { root, host, restore: engine.restore };
      await act(async () => render(agentBusy, shortcut));
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

describe("écrire à l'agent", () => {
  test("Dictation keyboard button opens the draft, icon only", async () => {
    const { host, typed } = await mount();
    const button = host.querySelector(".dictation-type") as HTMLButtonElement;
    expectIconOnly(button);
    await act(async () => button.click());
    expect(typed()).toBe(1);
  });

  test("⌘K ouvre le brouillon, même pendant un tour de l'agent", async () => {
    const { typed } = await mount(true);
    await press("k", { meta: true });
    expect(typed()).toBe(1);
    expect(engine.calls).toEqual([]);
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

  test("Dictation agent busy shows the state and the stop hint on two lines", async () => {
    const { host, rerender } = await mount();
    await rerender(true);
    expect(host.querySelector(".dictation-state")?.textContent).toBe("ui-dictation-agent-working");
    expect(host.querySelector(".dictation-hint")?.textContent).toBe("ui-dictation-agent-working-hint");
  });
});

describe("vocabulaire", () => {
  test("Dictation start sends the deck vocabulary", async () => {
    await mount(false, ["Bilan", "Q3"]);
    await press("d", { meta: true });
    expect(engine.starts).toEqual([["Bilan", "Q3"]]);
  });
});

const apple = (available: boolean) => ({ id: "apple" as const, label: "Apple (système)", size: 0, installed: available, available });

describe("modèle Apple", () => {
  test("Dictation Apple language to download shows no size", async () => {
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "apple", language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", query: [], headers: [] } },
      models: [{ ...apple(true), installed: false }],
    });
    const { host } = await mount();
    expect(host.querySelector(".dictation-line .caption")?.textContent).toBe("ui-dictation-to-download-language");
  });

  test("Dictation Apple chosen but unavailable says why", async () => {
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "apple", language: "xx", auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", query: [], headers: [] } },
      models: [apple(false)],
    });
    const { host } = await mount();
    expect(host.querySelector(".dictation-line .caption")?.textContent).toBe("ui-dictation-model-unavailable");
  });
});

describe("raccourci relayé", () => {
  test("Dictation remote type ouvre le brouillon une fois", async () => {
    const m = await mount();
    await m.rerenderRemote({ seq: 1, key: "type" });
    expect(m.typed()).toBe(1);
  });

  test("Dictation remote même numéro au rendu suivant : rien de plus", async () => {
    const m = await mount();
    await m.rerenderRemote({ seq: 1, key: "type" });
    await m.rerenderRemote({ seq: 1, key: "type" });
    expect(m.typed()).toBe(1);
  });

  test("Dictation remote présent au montage : rien", async () => {
    const m = await mount(false, [], { seq: 3, key: "type" });
    expect(m.typed()).toBe(0);
  });

  test("Dictation remote présent à un remontage : rien", async () => {
    const m = await mount();
    await m.remount({ seq: 4, key: "type" });
    expect(m.typed()).toBe(0);
  });

  test("Dictation remote draft pendant un tour, sans écoute : arrête l'agent", async () => {
    const m = await mount(true);
    await m.rerenderRemote({ seq: 1, key: "draft" });
    expect(m.cancels()).toBe(1);
  });

  test("Dictation remote auto au repos : lance l'écoute", async () => {
    const m = await mount();
    await m.rerenderRemote({ seq: 1, key: "auto" });
    expect(engine.calls).toContain("start");
  });
});

const apiModel = (installed: boolean) => ({ id: "api" as const, label: "API (gpt-4o-transcribe)", size: 0, installed, available: true });
const apiConfig = { model: "api" as const, language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", query: [], headers: [] } };

describe("modèle API", () => {
  test("Dictation API without key says so in the idle caption", async () => {
    engine.restore();
    engine = fakeStt("bonjour", { config: apiConfig, models: [apiModel(false)] });
    const { host } = await mount();
    expect(host.querySelector(".dictation-line .caption")?.textContent).toBe("ui-dictation-api-key-missing");
  });
});

const gear = (host: HTMLElement) => host.querySelector(`[aria-label="${t("ui-dictation-settings")}"]`) as HTMLButtonElement | null;
const cancel = (host: HTMLElement) => host.querySelector(`[aria-label="${t("ui-dictation-cancel")}"]`);

describe("réglages de l'hôte", () => {
  test("Dictation gear click asks the host for the settings box", async () => {
    const m = await mount(false, [], undefined, { onSettings: true });
    await act(async () => gear(m.host)!.click());
    expect(m.settingsClicks()).toBe(1);
  });

  test("Dictation without onSettings shows no gear", async () => {
    const m = await mount(false, [], undefined, { onSettings: false });
    expect(gear(m.host)).toBeNull();
  });

  test("Dictation while recording shows cancel instead of gear", async () => {
    const m = await mount();
    await act(async () => (m.host.querySelector(`[aria-label="${t("ui-dictation-start")}"]`) as HTMLButtonElement).click());
    await settle();
    expect([gear(m.host) === null, cancel(m.host) !== null]).toEqual([true, true]);
  });

  test("Dictation refetches its status on a voice change", async () => {
    await mount();
    const before = engine.statusCalls();
    await act(async () => bumpVoiceSeq());
    await settle();
    expect(engine.statusCalls()).toBe(before + 1);
  });

  test("Dictation keeps its status without a voice change", async () => {
    const m = await mount();
    const before = engine.statusCalls();
    await m.rerender(false);
    await settle();
    expect(engine.statusCalls()).toBe(before);
  });

  test("Dictation late older reply does not overwrite a newer one", async () => {
    const m = await mount();
    const first = engine.defer();
    await act(async () => bumpVoiceSeq());
    const second = engine.defer();
    await act(async () => bumpVoiceSeq());
    second.resolve({ config: { ...engine.current().config, language: "fr" } });
    first.resolve({ config: { ...engine.current().config, language: "en" } });
    await settle();
    expect(m.host.querySelector(".mic")?.getAttribute("title") ?? "").toContain("Français");
  });
});
