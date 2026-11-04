// Escape in a presentation with dictation and quick keys mounted (t-998), in a happy-dom DOM.

import "./test/dom";
import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { fakeStt } from "./test/sttSpy";
import { realSurface } from "./a2ui/fixtures/agentSurfaces";
import { manualTimers } from "./test/timers";
import { resetMessages } from "./test/messages";

// See a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement, Fragment } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Presenter } = await import("./Presenter");
const { default: Dictation } = await import("./Dictation");
const { useQuickKeys } = await import("./agent/useQuickKeys");
const { useModalFrame } = await import("./modalFrame");
const { useRef } = await import("react");

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

const slides = [{ id: "a", hash: "ha" }];
const settle = () => act(async () => new Promise((r) => setTimeout(r, 0)));

/** An armed modal question whose quick keys are live, mounted as App does (before the Presenter). */
function Quick({ actions }: { actions: string[] }) {
  useQuickKeys({ surface: realSurface("ask-2"), kind: "modal" }, (_s, a) => actions.push(a.name));
  return null;
}

/** A modal frame, armed, closing on Escape. */
function Frame({ closed }: { closed: number[] }) {
  const ref = useRef<HTMLDivElement>(null);
  useModalFrame(ref, { onEscape: () => closed.push(1) });
  return createElement("div", { ref, tabIndex: -1 });
}

async function mount(opts: { order?: "dictation-first" | "presenter-first"; dictation?: boolean; quick?: boolean; frame?: number[]; hands?: boolean } = {}) {
  const { order = "dictation-first", dictation = true, quick = false, frame, hands = false } = opts;
  const engine = fakeStt("bonjour", hands ? { config: { ...fakeStt().current().config, hands_free: true } } : {});
  if (hands) engine.listening();
  const exits: number[] = [];
  const actions: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host, restore: engine.restore };
  const presenter = createElement(Presenter, {
    key: "p",
    mode: "window",
    slides,
    current: "a",
    svg: () => "blob:x",
    onShow: () => {},
    onExit: () => exits.push(1),
    screen: { enter: async () => true, leave: async () => {}, onLeft: () => () => {} },
    timers: manualTimers(),
    titleBar: false,
  });
  const dict = dictation
    ? createElement(Dictation, {
        key: "d",
        destination: "x",
        vocabulary: [],
        onText: () => {},
        onError: () => {},
        agentBusy: false,
        onAgentCancel: () => {},
        onType: () => {},
      })
    : null;
  const quickKeys = quick ? createElement(Quick, { key: "q", actions }) : null;
  const modal = frame ? createElement(Frame, { key: "m", closed: frame }) : null;
  const parts = order === "dictation-first" ? [modal, quickKeys, dict, presenter] : [modal, quickKeys, presenter, dict];
  await act(async () => root.render(createElement(Fragment, null, ...parts)));
  await settle();
  return { engine, exits, actions };
}

const press = async (key: string, init: KeyboardEventInit = {}) => {
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key, code: key, bubbles: true, cancelable: true, ...init })));
  await settle();
};

describe("Escape in a presentation", () => {
  for (const order of ["dictation-first", "presenter-first"] as const) {
    test(`t-998 Escape while push-to-talk in a presentation cancels only the dictation (${order})`, async () => {
      const { engine, exits } = await mount({ order });
      await press("d", { metaKey: true });
      expect(engine.current().state).toBe("recording");
      await press("Escape");
      expect(engine.calls).toContain("cancel");
      expect(exits).toEqual([]);
      await press("Escape");
      expect(exits).toEqual([1]);
    });
  }

  test("t-998 Escape with no dictation exits the presentation", async () => {
    const { exits } = await mount({ dictation: false });
    await press("Escape");
    expect(exits).toEqual([1]);
  });

  test("t-998 Escape while hands-free listening exits the presentation", async () => {
    const { engine, exits } = await mount({ hands: true });
    expect(engine.current().state).toBe("listening");
    await press("Escape");
    expect(exits).toEqual([1]);
    expect(engine.calls).not.toContain("cancel");
  });

  test("t-998 Escape in a presentation does not dismiss an armed modal", async () => {
    const { exits, actions } = await mount({ dictation: false, quick: true });
    const now = Date.now();
    const clock = spyOn(Date, "now").mockReturnValue(now + 5000);
    try {
      await press("Escape");
    } finally {
      clock.mockRestore();
    }
    expect(actions).toEqual([]);
    expect(exits).toEqual([1]);
  });

  test("t-998 Escape in a presentation does not close an armed modal frame", async () => {
    const closed: number[] = [];
    const { exits } = await mount({ dictation: false, frame: closed });
    const now = Date.now();
    const clock = spyOn(Date, "now").mockReturnValue(now + 5000);
    try {
      await press("Escape");
    } finally {
      clock.mockRestore();
    }
    expect(closed).toEqual([]);
    expect(exits).toEqual([1]);
  });
});
