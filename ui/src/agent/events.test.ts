import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { subscribeHost, subscribeSse, subscribeTauri } from "./events";
import type { Listen } from "./events";
import type { AgentEvent, McpEvent } from "../api";

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
    FakeSource.last.emit("agent", envelope(TAB, { kind: "conversation" }));
    expect(FakeSource.last.url).toBe("/api/events");
    expect(r.mcp).toEqual([{ kind: "changed" }]);
    expect(r.agent).toEqual([{ kind: "conversation" }]);
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
    f.callbacks.get("oku-slides:agent")?.({ payload: JSON.stringify({ event: { kind: "conversation" } }) });
    f.callbacks.get("oku-slides:agent")?.({ payload: JSON.stringify({ tab: TAB }) });
    expect(r.agent).toEqual([]);
    expect(r.errors).toHaveLength(2);
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
    f.callbacks.get("oku-slides:agent")?.({ payload: envelope(TAB, { kind: "conversation" }) });
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
