// Dictée en deux modes, dans un DOM happy-dom, avec un faux moteur de dictée.

import "./test/dom";
import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { fakeStt } from "./test/sttSpy";
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
const { voiceContext } = await import("./voiceContextStore");
const { stt } = await import("./voiceApi");

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
      config: { model: "apple", language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", api_version: "", headers: [] }, hands_free: false, read_aloud: null, speech: { voice: null, rate: 1, in_presentation: false, barge_in: true } },
      models: [{ ...apple(true), installed: false }],
    });
    const { host } = await mount();
    expect(host.querySelector(".dictation-line .caption")?.textContent).toBe("ui-dictation-to-download-language");
  });

  test("Dictation Apple chosen but unavailable says why", async () => {
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "apple", language: "xx", auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", api_version: "", headers: [] }, hands_free: false, read_aloud: null, speech: { voice: null, rate: 1, in_presentation: false, barge_in: true } },
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
const apiConfig = { model: "api" as const, language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", api_version: "", headers: [] }, hands_free: false };

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

describe("hands-free", () => {
  afterEach(() => voiceContext.setResumed(false));

  /** The engine listening hands-free, `change` on top. */
  function listeningEngine(change: Parameters<typeof fakeStt>[1] = {}) {
    engine.restore();
    engine = fakeStt("bonjour", change);
    engine.listening();
  }
  const indicator = (host: HTMLElement) => host.querySelector(".mic.hands-free") as HTMLButtonElement;
  const status = (host: HTMLElement) => host.querySelector('[role="status"]')?.textContent;

  test("Dictation listening shows steady indicator with label", async () => {
    listeningEngine({ level: 0.8, speaking: true });
    const { host } = await mount();
    expect(indicator(host).getAttribute("aria-label")).toBe(t("ui-hands-free-pause"));
    expect(indicator(host).getAttribute("aria-pressed")).toBe("true");
    // Steady: no ring following the microphone level, no wave.
    expect(host.querySelector(".mic-ring")).toBeNull();
    expect(host.querySelector(".wave")).toBeNull();
    expect(status(host)).toBe(t("ui-hands-free-on"));
  });

  test("Dictation live region never holds utterance text", async () => {
    listeningEngine({ committed: "ajoute une slide", partial: "titre" });
    const { host } = await mount();
    const live = [...host.querySelectorAll("[aria-live]")].map((e) => e.textContent);
    expect(live).toEqual([t("ui-hands-free-on")]);
    expect(host.textContent).not.toContain("ajoute une slide");
  });

  test("Dictation mic while listening pauses", async () => {
    listeningEngine();
    voiceContext.setResumed(true);
    const { host } = await mount();
    await act(async () => indicator(host).click());
    await settle();
    expect(engine.calls).toEqual(["stop"]);
    expect(indicator(host).getAttribute("aria-pressed")).toBe("false");
    expect(status(host)).toBe(t("ui-hands-free-paused"));
    expect(voiceContext.resumed()).toBe(false);
    await act(async () => indicator(host).click());
    await settle();
    expect(engine.calls).toEqual(["stop", "listen"]);
    expect(voiceContext.resumed()).toBe(true);
  });

  test("Dictation resume the host refuses is not recorded as resumed", async () => {
    listeningEngine();
    const { host } = await mount();
    await act(async () => indicator(host).click());
    await settle();
    spyOn(stt, "listen").mockImplementation(() => Promise.reject(new Error("no microphone")));
    await act(async () => indicator(host).click());
    await settle();
    expect(voiceContext.resumed()).toBe(false);
  });

  test("Dictation ⌘D while listening pauses, then resumes", async () => {
    listeningEngine();
    await mount();
    await press("d", { meta: true });
    await press("d", { meta: true });
    expect(engine.calls).toEqual(["stop", "listen"]);
  });

  test("Dictation ⌘D with hands-free off still dictates", async () => {
    await mount();
    await press("d", { meta: true });
    expect(engine.calls).toEqual(["start"]);
  });
});
