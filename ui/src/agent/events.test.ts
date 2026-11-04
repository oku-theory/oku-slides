import { afterEach, beforeEach, describe, expect, mock, spyOn, test } from "bun:test";
import { PRESENT_SLIDE_MAX, subscribeHost, subscribeSse, subscribeTauri } from "./events";
import type { Listen } from "./events";
import type { AgentEvent, McpEvent } from "../api";
import { NOTES_MIN, PILE_MAX, PILE_MIN } from "../columns";

const TAB = "t-1";

/** Un EventSource de test : on déclenche ses évènements à la main. */
class FakeSource {
  static last: FakeSource;
  readonly handlers = new Map<string, (e: { data?: string }) => void>();
  closed = false;
  constructor(readonly url: string) {
    FakeSource.last = this;
  }
  addEventListener(name: string, f: (e: { data?: string }) => void) {
    this.handlers.set(name, f);
  }
  close() {
    this.closed = true;
  }
  emit(name: string, data?: string) {
    this.handlers.get(name)?.({ data });
  }
}

/** L'enveloppe `{tab, event}` telle qu'envoyée par l'hôte. */
const envelope = (tab: string, event: unknown) => JSON.stringify({ tab, event });

const recorder = () => {
  const mcp: McpEvent[] = [];
  const agent: AgentEvent[] = [];
  const errors: string[] = [];
  const ready = { count: 0 };
  return {
    mcp,
    agent,
    errors,
    ready,
    handlers: {
      onMcp: (e: McpEvent) => mcp.push(e),
      onAgent: (e: AgentEvent) => agent.push(e),
      onError: (m: string) => errors.push(m),
      onReady: () => ready.count++,
    },
  };
};

describe("évènements en SSE (développement)", () => {
  const original = globalThis.EventSource;
  beforeEach(() => {
    globalThis.EventSource = FakeSource as unknown as typeof EventSource;
  });
  afterEach(() => {
    globalThis.EventSource = original;
  });

  test("les évènements mcp et agent de l'onglet vont chacun à leur rappel", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "changed" }));
    FakeSource.last.emit("agent", envelope(TAB, { kind: "status", state: "idle" }));
    expect(FakeSource.last.url).toBe("/api/events");
    expect(r.mcp).toEqual([{ kind: "changed" }]);
    expect(r.agent).toEqual([{ kind: "status", state: "idle" }]);
  });

  test("un évènement d'un autre onglet n'est pas livré (et n'est pas une erreur)", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", envelope("t-2", { kind: "changed" }));
    expect(r.mcp).toEqual([]);
    expect(r.errors).toEqual([]);
  });

  test("un JSON illisible va à onError sans lever", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    expect(() => FakeSource.last.emit("agent", "")).not.toThrow();
    expect(() => FakeSource.last.emit("mcp", '{"pas":"de kind"}')).not.toThrow();
    expect(r.agent).toEqual([]);
    expect(r.mcp).toEqual([]);
    expect(r.errors).toHaveLength(2);
  });

  test("une charge sans tab ou sans event va à onError, rien n'est livré", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", JSON.stringify({ event: { kind: "changed" } }));
    FakeSource.last.emit("mcp", JSON.stringify({ tab: TAB }));
    expect(r.mcp).toEqual([]);
    expect(r.errors).toHaveLength(2);
  });

  test("une erreur du flux va à onError", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("error");
    expect(r.errors).toHaveLength(1);
  });

  test("onReady à chaque ouverture du flux, reconnexions comprises", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("open");
    FakeSource.last.emit("error");
    FakeSource.last.emit("open");
    expect(r.ready.count).toBe(2);
  });

  test("subscribeHost hors de Tauri s'abonne au flux SSE avec les mêmes rappels", () => {
    const r = recorder();
    const stop = subscribeHost(TAB, r.handlers);
    FakeSource.last.emit("open");
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "changed" }));
    expect(FakeSource.last.url).toBe("/api/events");
    expect(r.ready.count).toBe(1);
    expect(r.mcp).toEqual([{ kind: "changed" }]);
    stop();
    expect(FakeSource.last.closed).toBe(true);
  });

  test("focus with an empty slide is dropped, a deselect delivered", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "focus", slide: "" }));
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "focus", slide: null }));
    expect(r.mcp).toEqual([{ kind: "focus", slide: null }]);
    expect(r.errors).toEqual([]);
  });

  test("interface event outside the static bounds is dropped with a warning naming only its kind", () => {
    const warn = spyOn(console, "warn").mockImplementation(() => {});
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    const change = (over: Record<string, unknown>) => ({
      scheme: null,
      locale: null,
      guides: null,
      folded: null,
      pile: null,
      notes: null,
      code: null,
      ...over,
    });
    const send = (over: Record<string, unknown>) =>
      FakeSource.last.emit("mcp", envelope(TAB, { kind: "interface", change: change(over) }));
    send({ pile: PILE_MAX + 1 });
    send({ pile: PILE_MIN - 1 });
    send({ notes: NOTES_MIN - 1 });
    send({ locale: "x".repeat(40) });
    send({ scheme: "blue" });
    send({ pile: PILE_MIN, notes: NOTES_MIN, locale: "fr" });
    expect(r.mcp).toEqual([{ kind: "interface", change: change({ pile: PILE_MIN, notes: NOTES_MIN, locale: "fr" }) }]);
    expect(warn.mock.calls.length).toBe(5);
    expect(warn.mock.calls.flat().join(" ")).not.toContain("xxxx");
    warn.mockRestore();
  });

  test("focus with an oversized slide id is dropped", () => {
    const warn = spyOn(console, "warn").mockImplementation(() => {});
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "focus", slide: "s".repeat(PRESENT_SLIDE_MAX + 1) }));
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "focus", slide: "s-2" }));
    expect(r.mcp).toEqual([{ kind: "focus", slide: "s-2" }]);
    warn.mockRestore();
  });

  test("present show with oversized slide id is dropped", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    const show = (slide: string) => envelope(TAB, { kind: "present", command: { action: "show", slide } });
    FakeSource.last.emit("mcp", show("s".repeat(PRESENT_SLIDE_MAX + 1)));
    FakeSource.last.emit("mcp", show("s-2"));
    expect(r.mcp).toEqual([{ kind: "present", command: { action: "show", slide: "s-2" } }]);
    expect(r.errors).toEqual([]);
  });

  test("close event with invalid ids is dropped", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    const id = "0b0c4b52-2b7c-4a43-9df2-6c2b8f0e6b11";
    const close = (target: unknown, decision: string) => envelope(TAB, { kind: "close", target, decision });
    FakeSource.last.emit("mcp", close({ kind: "tab", tab: "t-x" }, id));
    FakeSource.last.emit("mcp", close({ kind: "window", window: "t-1" }, id));
    FakeSource.last.emit("mcp", close({ kind: "tab", tab: "t-2" }, "not-a-uuid"));
    FakeSource.last.emit("mcp", close({ kind: "tab", tab: "t-2", window: "w-1" }, id));
    FakeSource.last.emit("mcp", close({ kind: "tab", tab: "t-2" }, id));
    expect(r.mcp).toEqual([{ kind: "close", target: { kind: "tab", tab: "t-2" }, decision: id }]);
    expect(r.errors).toEqual([]);
  });

  test("close event ids follow the host's 18-digit bound", () => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    const id = "0b0c4b52-2b7c-4a43-9df2-6c2b8f0e6b11";
    const close = (tab: string) => envelope(TAB, { kind: "close", target: { kind: "tab", tab }, decision: id });
    FakeSource.last.emit("mcp", close(`t-${"9".repeat(19)}`));
    FakeSource.last.emit("mcp", close(`t-${"9".repeat(18)}`));
    expect(r.mcp).toEqual([{ kind: "close", target: { kind: "tab", tab: `t-${"9".repeat(18)}` }, decision: id }]);
  });

  test.each([
    ["an unknown action", { action: "jump" }],
    ["a start without mode", { action: "start" }],
    ["no command", undefined],
  ])("present with %s is dropped", (_name, command) => {
    const r = recorder();
    subscribeSse("/api/events", TAB, r.handlers);
    FakeSource.last.emit("mcp", envelope(TAB, { kind: "present", command }));
    expect(r.mcp).toEqual([]);
    expect(r.errors).toEqual([]);
  });

  test("se désabonner ferme le flux", () => {
    const stop = subscribeSse("/api/events", TAB, recorder().handlers);
    stop();
    expect(FakeSource.last.closed).toBe(true);
  });
});

describe("évènements Tauri", () => {
  /** Un `listen` de test : garde les rappels, compte les désabonnements. */
  const fakeListen = (fail?: string) => {
    const callbacks = new Map<string, (e: { payload: string }) => void>();
    const unlistened: string[] = [];
    const listen: Listen = async (name, cb) => {
      if (name === fail) throw new Error(`listen ${name}`);
      callbacks.set(name, cb);
      return () => unlistened.push(name);
    };
    return { listen, callbacks, unlistened };
  };
  const settle = () => new Promise((r) => setTimeout(r, 0));

  test("les charges JSON de l'onglet vont à leur rappel", async () => {
    const f = fakeListen();
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    f.callbacks.get("oku-slides:mcp")?.({ payload: envelope(TAB, { kind: "close_surface", id: "a" }) });
    f.callbacks.get("oku-slides:agent")?.({ payload: envelope(TAB, { kind: "status", state: "idle" }) });
    expect(r.mcp).toEqual([{ kind: "close_surface", id: "a" }]);
    expect(r.agent).toEqual([{ kind: "status", state: "idle" }]);
  });

  test("onMcp n'est appelé qu'une fois pour l'abonnement de t-1, un évènement de t-2 est ignoré", async () => {
    const f = fakeListen();
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    f.callbacks.get("oku-slides:mcp")?.({ payload: envelope("t-2", { kind: "changed" }) });
    f.callbacks.get("oku-slides:mcp")?.({ payload: envelope(TAB, { kind: "changed" }) });
    expect(r.mcp).toEqual([{ kind: "changed" }]);
    expect(r.errors).toEqual([]);
  });

  test("une charge sans tab ou sans event va à onError, rien n'est livré", async () => {
    const f = fakeListen();
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    f.callbacks.get("oku-slides:agent")?.({ payload: JSON.stringify({ event: { kind: "status", state: "idle" } }) });
    f.callbacks.get("oku-slides:agent")?.({ payload: JSON.stringify({ tab: TAB }) });
    expect(r.agent).toEqual([]);
    expect(r.errors).toHaveLength(2);
  });

  test("present with an invalid command is dropped, a valid one delivered", async () => {
    const f = fakeListen();
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    const present = (command: unknown) => ({ payload: envelope(TAB, { kind: "present", command }) });
    f.callbacks.get("oku-slides:mcp")?.(present({ action: "start", mode: "fullscreen" }));
    f.callbacks.get("oku-slides:mcp")?.(present({ action: "next" }));
    expect(r.mcp).toEqual([{ kind: "present", command: { action: "next" } }]);
    expect(r.errors).toEqual([]);
  });

  test("onReady une fois les deux écoutes en place", async () => {
    const f = fakeListen();
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    expect(r.ready.count).toBe(0);
    await settle();
    expect(r.ready.count).toBe(1);
  });

  test.each(["oku-slides:mcp", "oku-slides:agent"])("pas d'onReady si l'écoute %s échoue", async (event) => {
    const f = fakeListen(event);
    const r = recorder();
    subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    expect(r.ready.count).toBe(0);
  });

  test("pas d'onReady après un désabonnement", async () => {
    const f = fakeListen();
    const r = recorder();
    const stop = subscribeTauri(async () => f.listen, TAB, r.handlers);
    stop();
    await settle();
    expect(r.ready.count).toBe(0);
  });

  test("se désabonner avant la fin de listen retire les deux écoutes", async () => {
    const f = fakeListen();
    const stop = subscribeTauri(async () => f.listen, TAB, recorder().handlers);
    stop();
    await settle();
    expect(f.unlistened.sort()).toEqual(["oku-slides:agent", "oku-slides:mcp"]);
  });

  test("après désabonnement, plus aucun rappel", async () => {
    const f = fakeListen();
    const r = recorder();
    const stop = subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    stop();
    f.callbacks.get("oku-slides:agent")?.({ payload: envelope(TAB, { kind: "status", state: "idle" }) });
    expect(r.agent).toEqual([]);
  });

  test("si la seconde écoute échoue, la première est retirée au désabonnement", async () => {
    const f = fakeListen("oku-slides:agent");
    const r = recorder();
    const stop = subscribeTauri(async () => f.listen, TAB, r.handlers);
    await settle();
    stop();
    expect(f.unlistened).toEqual(["oku-slides:mcp"]);
    expect(r.errors).toHaveLength(1);
  });

  test("un module Tauri introuvable va à onError", async () => {
    const r = recorder();
    const load = mock(async (): Promise<Listen> => {
      throw new Error("module");
    });
    subscribeTauri(load, TAB, r.handlers);
    await settle();
    expect(r.errors).toHaveLength(1);
  });
});
