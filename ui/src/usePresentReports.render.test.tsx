// Presenting reports and the host's present commands, in a happy-dom DOM.

import "./test/dom";
import { afterEach, describe, expect, test } from "bun:test";
import type { PresentCommand, PresentMode, SlideCommand } from "./present";
import type { PresentWiring } from "./usePresentReports";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePresentReports } = await import("./usePresentReports");
const { createVoiceContextStore } = await import("./voiceContextStore");

interface Sent { tab: string; page: string; mode: PresentMode | null; seq: number; window: string }

let roots: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const unmount of roots) await unmount();
  roots = [];
});

interface ProbeProps {
  tab: string;
  presenting: PresentMode | null;
  out: { wiring: PresentWiring | null; presented: PresentMode[] };
  deps: Parameters<typeof usePresentReports>[4];
}

function Probe({ tab, presenting, out, deps }: ProbeProps) {
  out.wiring = usePresentReports(tab, "w-1", presenting, (mode) => void out.presented.push(mode), deps);
  return null;
}

/** Mounts the hook for `tab` in window `w-1`; its reports go to `sent`. */
async function mount(presenting: PresentMode | null, tab = "t-1") {
  const store = createVoiceContextStore();
  const sent: Sent[] = [];
  const deps = {
    store,
    report: (tab: string, page: string, mode: PresentMode | null, seq: number, window: string) => {
      sent.push({ tab, page, mode, seq, window });
      return Promise.resolve();
    },
  };
  const out: ProbeProps["out"] = { wiring: null, presented: [] };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const render = (presenting: PresentMode | null, t = tab) =>
    act(async () => root.render(createElement(Probe, { tab: t, presenting, out, deps })));
  await render(presenting);
  const unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  roots.push(unmount);
  return { sent, store, out, render, unmount };
}

describe("usePresentReports", () => {
  test("presenting start is reported with the window and the page id", async () => {
    const p = await mount("window");
    expect(p.sent).toEqual([{ tab: "t-1", page: p.store.page, mode: "window", seq: 1, window: "w-1" }]);
  });

  test("not presenting reports nothing", async () => {
    const p = await mount(null);
    expect(p.sent).toEqual([]);
  });

  test("presenting end is reported on exit", async () => {
    const p = await mount("screen");
    await p.render(null);
    expect(p.sent.map((s) => [s.mode, s.seq])).toEqual([["screen", 1], [null, 2]]);
  });

  test("presenting end is reported on unmount", async () => {
    const p = await mount("window");
    await p.unmount();
    roots = [];
    expect(p.sent.map((s) => [s.tab, s.mode, s.seq])).toEqual([["t-1", "window", 1], ["t-1", null, 2]]);
  });

  test("presenting end of the old tab is reported on a tab switch", async () => {
    const p = await mount("window");
    await p.render("window", "t-2");
    expect(p.sent.map((s) => [s.tab, s.mode])).toEqual([["t-1", "window"], ["t-1", null], ["t-2", "window"]]);
  });

  test("present start command presents in its mode", async () => {
    const p = await mount(null);
    act(() => p.out.wiring!.onCommand({ action: "start", mode: "window" }));
    expect(p.out.presented).toEqual(["window"]);
  });

  test("present steering commands reach the subscribers only", async () => {
    const p = await mount("window");
    const got: SlideCommand[] = [];
    const stop = p.out.wiring!.commands((c) => void got.push(c));
    const next: PresentCommand = { action: "next" };
    act(() => p.out.wiring!.onCommand(next));
    stop();
    act(() => p.out.wiring!.onCommand({ action: "exit" }));
    expect(got).toEqual([{ action: "next" }]);
    expect(p.out.presented).toEqual([]);
  });
});
