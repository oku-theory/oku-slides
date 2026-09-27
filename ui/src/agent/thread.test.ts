import { describe, expect, test } from "bun:test";
import { EMPTY_THREAD, threadItems, threadReducer } from "./thread";
import type { AgentEvent } from "../api";

const run = (events: AgentEvent[]) => events.reduce(threadReducer, EMPTY_THREAD);

describe("fil de l'agent", () => {
  test("les morceaux de réponse s'ajoutent au même message", () => {
    const t = run([
      { kind: "user_text", text: "plus court" },
      { kind: "chunk", text: "Je " },
      { kind: "chunk", text: "regarde." },
    ]);
    expect(t.entries).toEqual([
      { kind: "user", text: "plus court" },
      { kind: "agent", text: "Je regarde." },
    ]);
  });

  test("une mise à jour d'outil change la ligne de l'outil", () => {
    const t = run([
      { kind: "tool_call", id: "t1", title: "Lire", status: "in_progress" },
      { kind: "tool_update", id: "t1", title: null, status: "completed" },
    ]);
    expect(t.entries).toEqual([{ kind: "tool", id: "t1", title: "Lire", status: "completed" }]);
  });

  test("le plan remplace le plan précédent", () => {
    const t = run([
      { kind: "plan", entries: [{ content: "Lire", status: "in_progress" }] },
      { kind: "plan", entries: [{ content: "Lire", status: "completed" }] },
    ]);
    expect(t.entries).toEqual([{ kind: "plan", entries: [{ content: "Lire", status: "completed" }] }]);
  });

  test("un tour annulé le dit ; une nouvelle conversation vide le fil", () => {
    const t = run([{ kind: "user_text", text: "a" }, { kind: "turn_ended", stop: "cancelled" }]);
    expect(t.entries.at(-1)).toEqual({ kind: "stop", stop: "cancelled" });
    expect(threadReducer(t, { kind: "conversation" }).entries).toEqual([]);
  });

  test("l'état suit les évènements de statut", () => {
    expect(run([{ kind: "status", state: "working" }]).state).toBe("working");
  });

  test("un morceau après un outil ouvre un nouveau message", () => {
    const t = run([
      { kind: "chunk", text: "Je lis." },
      { kind: "tool_call", id: "t1", title: "Lire", status: "pending" },
      { kind: "chunk", text: "Fait." },
    ]);
    expect(t.entries.map((x) => x.kind)).toEqual(["agent", "tool", "agent"]);
    expect(t.entries.at(-1)).toEqual({ kind: "agent", text: "Fait." });
  });

  test("un tour fini normalement n'ajoute rien", () => {
    const t = run([{ kind: "user_text", text: "a" }]);
    expect(threadReducer(t, { kind: "turn_ended", stop: "end_turn" })).toBe(t);
  });

  test("une erreur s'ajoute au fil", () => {
    expect(run([{ kind: "error", message: "agent introuvable" }]).entries).toEqual([
      { kind: "error", text: "agent introuvable" },
    ]);
  });

  test("une mise à jour d'outil remplace le titre donné et garde le statut absent", () => {
    const t = run([
      { kind: "tool_call", id: "t1", title: "Lire", status: "in_progress" },
      { kind: "tool_update", id: "t1", title: "Lire la slide 2", status: null },
    ]);
    expect(t.entries).toEqual([{ kind: "tool", id: "t1", title: "Lire la slide 2", status: "in_progress" }]);
  });

  test("une mise à jour d'un outil inconnu ne change rien", () => {
    const t = run([{ kind: "tool_call", id: "t1", title: "Lire", status: "pending" }]);
    const next = threadReducer(t, { kind: "tool_update", id: "t9", title: "x", status: "completed" });
    expect(next.entries).toEqual(t.entries);
  });

  test("le plan est remplacé à sa place, pas déplacé", () => {
    const t = run([
      { kind: "plan", entries: [{ content: "Lire", status: "pending" }] },
      { kind: "chunk", text: "ok" },
      { kind: "plan", entries: [{ content: "Lire", status: "completed" }] },
    ]);
    expect(t.entries.map((x) => x.kind)).toEqual(["plan", "agent"]);
  });

  test("une nouvelle conversation garde l'état de l'agent", () => {
    const t = run([{ kind: "status", state: "working" }, { kind: "conversation" }]);
    expect(t.state).toBe("working");
  });

  test("un évènement inconnu d'un hôte plus récent laisse le fil intact", () => {
    const t = run([{ kind: "user_text", text: "a" }]);
    expect(threadReducer(t, { kind: "nouveau" } as unknown as AgentEvent)).toBe(t);
  });

  test("le fil gardé (resync) remplace l'état courant par le replay de ses entrées", () => {
    const withAChunk = run([{ kind: "chunk", text: "b" }]);
    const entries: AgentEvent[] = [{ kind: "user_text", text: "a" }, { kind: "chunk", text: "b" }];
    const replayed = run(entries);
    expect(threadReducer(withAChunk, { kind: "thread", entries })).toEqual(replayed);
  });
});

describe("lignes affichées du fil", () => {
  const tool = (id: string) => ({ kind: "tool" as const, id, title: id, status: "completed" });

  test("threadItems regroupe les outils consécutifs", () => {
    const items = threadItems([{ kind: "user", text: "a" }, tool("1"), tool("2"), { kind: "agent", text: "b" }, tool("3")]);
    expect(items.map((x) => (x.kind === "tools" ? x.tools.map((t) => t.id) : x.kind))).toEqual([
      "user",
      ["1", "2"],
      "agent",
      ["3"],
    ]);
  });

  test("threadItems garde l'indice de la première ligne de chaque élément", () => {
    const items = threadItems([{ kind: "user", text: "a" }, tool("1"), tool("2"), { kind: "agent", text: "b" }]);
    expect(items.map((x) => x.at)).toEqual([0, 1, 3]);
  });
});
