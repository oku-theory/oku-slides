// Rappel du fil de l'agent quand la portée change.

import { afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { AgentScope, McpEvent } from "../api";
import type { ScopeView } from "./scope";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentScope } = await import("./useAgentScope");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

/** Monte le hook ; rend ses appels à `resync`, ses vidages et de quoi changer la vue. */
async function mountHook(view: ScopeView | null, fail = false) {
  const resyncs: AgentScope[] = [];
  const cleared: McpEvent[] = [];
  const agent = {
    resync: async (s: AgentScope) => {
      resyncs.push(s);
      if (fail) throw new Error("no tools");
    },
  };
  const clear = (e: McpEvent) => void cleared.push(e);
  let state: ReturnType<typeof useAgentScope> | null = null;
  const Probe = ({ v }: { v: ScopeView | null }) => {
    state = useAgentScope(agent, v, clear);
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe, { v: view })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const rerender = (v: ScopeView | null) => act(async () => root.render(createElement(Probe, { v })));
  return { resyncs, cleared, rerender, state: () => state! };
}

const deck: ScopeView = { templates: false, history: false, pack: null, widget: null };
const atelier: ScopeView = { templates: true, history: false, pack: "p-1", widget: null };

test("useAgentScope before the subscription is ready does not resync", async () => {
  const h = await mountHook(deck);
  expect(h.resyncs).toEqual([]);
});

test("useAgentScope at the first ready resyncs once the view is known", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  expect(h.resyncs).toEqual([{ kind: "slides" }]);
});

test("useAgentScope on a scope change clears then resyncs", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  await h.rerender(atelier);
  expect(h.cleared.at(-1)).toEqual({ kind: "surfaces", list: [] });
  expect(h.resyncs.at(-1)).toEqual({ kind: "atelier", pack: "p-1" });
});

test("useAgentScope at a reconnection resyncs without clearing", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onReady());
  const cleared = h.cleared.length;
  await act(async () => h.state().onReady());
  expect(h.resyncs).toHaveLength(2);
  expect(h.cleared).toHaveLength(cleared);
});

test("useAgentScope n'est pas prêt avant le premier rappel du fil (t-211)", async () => {
  const h = await mountHook(deck);
  expect(h.state().ready).toBe(false);
  await act(async () => h.state().onReady());
  expect(h.state().ready).toBe(true);
});

test("useAgentScope sans agent (le rappel du fil échoue) n'est pas prêt (t-211)", async () => {
  const h = await mountHook(deck, true);
  await act(async () => h.state().onReady());
  expect(h.resyncs).toHaveLength(1);
  expect(h.state().ready).toBe(false);
});

test("useAgentScope un évènement draft ne change pas l'état de l'agent", async () => {
  const h = await mountHook(deck);
  await act(async () => h.state().onAgentEvent({ kind: "status", state: "working", scope: { kind: "slides" } }));
  await act(async () => h.state().onAgentEvent({ kind: "draft", text: "x", origin: null }));
  expect(h.state().status.state).toBe("working");
});
