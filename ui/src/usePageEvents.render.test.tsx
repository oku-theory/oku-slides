// Page events shared by the menu and the agent: presenting the deck, opening the history.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";

const { act, createElement, useRef } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePageEvents } = await import("./usePageEvents");
const { deferred, wait } = await import("./test/widgetApi");

const roots: { root: ReturnType<typeof createRoot>; host: HTMLElement }[] = [];
afterEach(async () => {
  for (const m of roots.splice(0)) {
    await act(async () => m.root.unmount());
    m.host.remove();
  }
});

/** Mounts `usePageEvents` with recording fakes; `log` lists every call in order. */
async function mount(snap: Partial<Snapshot> | null, historyOpen: () => Promise<Snapshot | null> = async () => snap as Snapshot) {
  const log: string[] = [];
  const api = {
    historyOpen: () => {
      log.push("historyOpen");
      return historyOpen();
    },
  } as unknown as TabApi;
  const out: { events: ReturnType<typeof usePageEvents> | null } = { events: null };
  function Probe() {
    const snapRef = useRef(snap as Snapshot | null);
    const stage = useRef({ commitEditing: () => void log.push("commit") });
    out.events = usePageEvents({
      api,
      snapRef,
      stage,
      autosave: {
        flush: async () => {
          log.push("flush");
          await new Promise((r) => setTimeout(r, 5));
          log.push("flushed");
        },
      },
      run: async (p) => p,
      setSelected: (id) => void log.push(`selected ${id}`),
      setEditing: (key) => void log.push(`editing ${key}`),
    });
    return null;
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  await act(async () => root.render(createElement(Probe)));
  return { log, events: () => out.events! };
}

test("present_cannotPresent_doesNothing", async () => {
  const { log, events } = await mount({ can_present: false });
  await act(async () => events().present("screen"));
  expect(log).toEqual([]);
  expect(events().presenting).toBeNull();
});

test("present_canPresent_commitsClearsAndPresentsInTheMode", async () => {
  const { log, events } = await mount({ can_present: true });
  await act(async () => events().present("window"));
  expect(log).toEqual(["commit", "selected null", "editing null"]);
  expect(events().presenting).toBe("window");
});

test("openHistory_opened_commitsFlushesBeforeOpeningThenClearsSelection", async () => {
  const { log, events } = await mount({ can_present: true });
  await act(async () => events().openHistory());
  expect(log).toEqual(["commit", "flush", "flushed", "historyOpen", "selected null", "editing null"]);
});

test("openHistory_refused_keepsSelection", async () => {
  const { log, events } = await mount({}, async () => null);
  await act(async () => events().openHistory());
  expect(log).toEqual(["commit", "flush", "flushed", "historyOpen"]);
});

test("openHistory_calledWhilePending_doesNothing", async () => {
  const opened = deferred<Snapshot | null>();
  const { log, events } = await mount({}, () => opened.promise);
  let first!: Promise<void>;
  await act(async () => {
    first = events().openHistory();
  });
  await wait(10);
  await act(async () => events().openHistory());
  expect(log).toEqual(["commit", "flush", "flushed", "historyOpen"]);
  opened.resolve({} as Snapshot);
  await act(async () => first);
  expect(log).toEqual(["commit", "flush", "flushed", "historyOpen", "selected null", "editing null"]);
  // Settled: a new opening goes through.
  await act(async () => events().openHistory());
  expect(log.filter((l) => l === "historyOpen")).toHaveLength(2);
});
