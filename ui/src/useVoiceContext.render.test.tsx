// Context reports and listening at mount, in a happy-dom DOM, with a fake dictation engine.

import "./test/dom";
import { afterEach, describe, expect, test } from "bun:test";
import type { AgentUi } from "./api";
import { fakeStt } from "./test/sttSpy";
import { manualTimers } from "./test/timers";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useVoiceContext, CONTEXT_DEBOUNCE_MS } = await import("./useVoiceContext");
const { createVoiceContextStore } = await import("./voiceContextStore");

type Store = ReturnType<typeof createVoiceContextStore>;
interface Sent { tab: string; page: string; ui: AgentUi; seq: number; hasFocus: boolean; window: string }

const UI: AgentUi = { slide: "s-1", element: null, field: null, editing: false, widget: null };

let engine = fakeStt();
let roots: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const unmount of roots) await unmount();
  roots = [];
  engine.restore();
  engine = fakeStt();
});

/** Lets the fake engine's promises resolve and React render them. */
const settle = () => act(async () => new Promise((r) => setTimeout(r, 0)));

function Probe({ tab, window, deps }: { tab: string | null; window: string | null; deps: Parameters<typeof useVoiceContext>[2] }) {
  useVoiceContext(tab, window, deps);
  return null;
}

/** Mounts the hook for `tab` in window `w-1` over `store`; its reports go to `sent`. */
async function mount(store: Store, tab: string | null = "t-1") {
  const sent: Sent[] = [];
  const timers = manualTimers();
  const deps = {
    store,
    timers,
    report: (tab: string, page: string, ui: AgentUi, seq: number, hasFocus: boolean, window: string) => {
      sent.push({ tab, page, ui, seq, hasFocus, window });
      return Promise.resolve();
    },
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe, { tab, window: "w-1", deps })));
  await settle();
  const unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  roots.push(unmount);
  return { sent, timers, unmount };
}

describe("useVoiceContext", () => {
  test("useVoiceContext selection change reports context once", async () => {
    const store = createVoiceContextStore();
    store.write("t-1", { ui: UI, hasFocus: true });
    const { sent, timers } = await mount(store);
    expect(timers.delays).toContain(CONTEXT_DEBOUNCE_MS);
    timers.run();
    expect(sent).toEqual([{ tab: "t-1", page: store.page, ui: UI, seq: 1, hasFocus: true, window: "w-1" }]);
    act(() => {
      store.write("t-1", { ui: { ...UI, element: "e-1" }, hasFocus: true });
      store.write("t-1", { ui: { ...UI, element: "e-2" }, hasFocus: true });
    });
    timers.run();
    expect(sent.map((s) => [s.ui.element, s.seq])).toEqual([[null, 1], ["e-2", 2]]);
    // Another tab's change is not this window's active tab: nothing more goes.
    act(() => store.write("t-2", { ui: UI, hasFocus: true }));
    timers.run();
    expect(sent).toHaveLength(2);
  });

  test("useVoiceContext remount sends no Stop", async () => {
    engine.listening();
    const store = createVoiceContextStore();
    store.setResumed(true);
    const first = await mount(store);
    await first.unmount();
    await mount(store);
    await settle();
    expect(engine.calls).not.toContain("stop");
    expect(engine.calls).not.toContain("cancel");
  });

  test("useVoiceContext hands-free off asks no listen", async () => {
    const store = createVoiceContextStore();
    store.setResumed(true);
    await mount(store);
    await settle();
    expect(engine.calls).toEqual([]);
  });

  test("useVoiceContext hands-free on but paused by the user asks no listen", async () => {
    engine.restore();
    engine = fakeStt("bonjour", { config: { ...fakeStt().current().config, hands_free: true } });
    await mount(createVoiceContextStore());
    await settle();
    expect(engine.calls).toEqual([]);
  });

  test("useVoiceContext hands-free on and resumed asks listen at mount", async () => {
    engine.restore();
    engine = fakeStt("bonjour", { config: { ...fakeStt().current().config, hands_free: true } });
    const store = createVoiceContextStore();
    store.setResumed(true);
    await mount(store);
    await settle();
    expect(engine.calls).toEqual(["listen"]);
  });

  test("useVoiceContext new page sends a new page id", async () => {
    const [before, after] = [createVoiceContextStore(), createVoiceContextStore()];
    for (const store of [before, after]) store.write("t-1", { ui: UI, hasFocus: false });
    const old = await mount(before);
    old.timers.run();
    await old.unmount();
    const fresh = await mount(after);
    fresh.timers.run();
    expect(old.sent.map((s) => [s.page, s.seq])).toEqual([[before.page, 1]]);
    expect(fresh.sent.map((s) => [s.page, s.seq])).toEqual([[after.page, 1]]);
    expect(after.page).not.toBe(before.page);
  });
});
