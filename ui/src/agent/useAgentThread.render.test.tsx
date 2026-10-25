// Rappel du fil de l'agent : une fois à l'abonnement, à chaque reconnexion.

import { afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { McpEvent } from "../api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentThread } = await import("./useAgentThread");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

/** Monte le hook ; rend le nombre d'appels à `resync`, ses vidages et de quoi changer `loaded`. */
async function mountHook(loaded: boolean, fail = false) {
  let resyncs = 0;
  const cleared: McpEvent[] = [];
  const agent = {
    resync: async () => {
      resyncs++;
      if (fail) throw new Error("no tools");
    },
  };
  const clear = (e: McpEvent) => void cleared.push(e);
  let state: ReturnType<typeof useAgentThread> | null = null;
  const Probe = ({ l }: { l: boolean }) => {
    state = useAgentThread(agent, l, clear);
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe, { l: loaded })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const rerender = (l: boolean) => act(async () => root.render(createElement(Probe, { l })));
  return { resyncs: () => resyncs, cleared, rerender, state: () => state! };
}

test("useAgentThread resyncs once when ready and loaded", async () => {
  const h = await mountHook(true);
  expect(h.resyncs()).toBe(0);
  await act(async () => h.state().onReady());
  expect(h.resyncs()).toBe(1);
  expect(h.cleared.at(-1)).toEqual({ kind: "surfaces", list: [] });
});

test("useAgentThread does not resync before the snapshot is loaded", async () => {
  const h = await mountHook(false);
  await act(async () => h.state().onReady());
  expect(h.resyncs()).toBe(0);
  await h.rerender(true);
  expect(h.resyncs()).toBe(1);
});

test("useAgentThread resyncs again on reconnection", async () => {
  const h = await mountHook(true);
  await act(async () => h.state().onReady());
  const cleared = h.cleared.length;
  await act(async () => h.state().onReady());
  expect(h.resyncs()).toBe(2);
  expect(h.cleared).toHaveLength(cleared);
});

test("useAgentThread ready is false when resync fails", async () => {
  const h = await mountHook(true, true);
  await act(async () => h.state().onReady());
  expect(h.resyncs()).toBe(1);
  expect(h.state().ready).toBe(false);
});

test("useAgentThread ready is true after a successful resync (t-211)", async () => {
  const h = await mountHook(true);
  expect(h.state().ready).toBe(false);
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(true);
});

test("useAgentThread status follows status events", async () => {
  const h = await mountHook(true);
  await act(async () => h.state().onAgentEvent({ kind: "status", state: "working" }));
  expect(h.state().state).toBe("working");
  await act(async () => h.state().onAgentEvent({ kind: "draft", text: "x", origin: null, images: [] }));
  expect(h.state().state).toBe("working");
});
