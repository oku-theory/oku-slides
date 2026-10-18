import { describe, expect, spyOn, test } from "bun:test";
import { memoryTransport } from "../test/link";
import { manualTimers } from "../test/timers";
import { MAX_LINK_TEXT, channelTransport, insertWithAck, onlyFor, parseLink, type LinkMessage } from "./link";

/** Une fenêtre principale qui accuse chaque `insert` de `tab` avec `ok`. */
function answering(t: ReturnType<typeof memoryTransport>, tab: string, ok: boolean) {
  t.listen(onlyFor("main", tab, (m) => {
    if (m.kind === "insert") t.send({ to: "agent", kind: "insert-ack", tab, id: m.id, ok });
  }));
}

describe("agent-link", () => {
  test("insertWithAck accusé ok : true", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    expect(await insertWithAck(t, "t-1", "texte", 3000, manualTimers())).toBe(true);
  });

  test("insertWithAck refusé : false", async () => {
    const t = memoryTransport();
    answering(t, "t-1", false);
    expect(await insertWithAck(t, "t-1", "texte", 3000, manualTimers())).toBe(false);
  });

  test("insertWithAck sans réponse : false à l'échéance", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck sans réponse : insert-cancel envoyé pour cet identifiant", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    await done;
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    expect(t.sent).toContainEqual({ to: "main", kind: "insert-cancel", tab: "t-1", id: insert.id });
  });

  test("insertWithAck ignore l'accusé d'un autre envoi", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    t.send({ to: "agent", kind: "insert-ack", tab: "t-1", id: "autre", ok: true });
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck ignore l'accusé d'un autre onglet au même identifiant", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    t.send({ to: "agent", kind: "insert-ack", tab: "t-2", id: insert.id, ok: true });
    timers.run();
    expect(await done).toBe(false);
  });

  test("insertWithAck un accusé après l'échéance ne change rien", async () => {
    const t = memoryTransport();
    const timers = manualTimers();
    const done = insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    const insert = t.sent.find((m) => m.kind === "insert") as { id: string };
    t.send({ to: "agent", kind: "insert-ack", tab: "t-1", id: insert.id, ok: true });
    expect(await done).toBe(false);
  });

  test("insertWithAck retire son écoute une fois fini", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    const before = t.listeners();
    await insertWithAck(t, "t-1", "texte", 3000, manualTimers());
    expect(t.listeners()).toBe(before);
  });

  test("insertWithAck accusé reçu : la minuterie est oubliée", async () => {
    const t = memoryTransport();
    answering(t, "t-1", true);
    const timers = manualTimers();
    await insertWithAck(t, "t-1", "texte", 3000, timers);
    timers.run();
    expect(t.sent.filter((m) => m.kind === "insert-cancel")).toEqual([]);
  });

  test("onlyFor ne garde que le sens et l'onglet visés", () => {
    const got: LinkMessage[] = [];
    const cb = onlyFor("agent", "t-1", (m) => void got.push(m));
    const mine: LinkMessage = { to: "agent", kind: "shortcut", tab: "t-1", key: "auto" };
    cb({ to: "agent", kind: "shortcut", tab: "t-2", key: "auto" });
    cb({ to: "main", kind: "hello", tab: "t-1" });
    cb(mine);
    expect(got).toEqual([mine]);
  });

  test("parseLink écarte un texte plus long que la limite", () => {
    expect(parseLink({ to: "main", kind: "insert", tab: "t-1", id: "i-1", text: "a".repeat(MAX_LINK_TEXT + 1) })).toBeNull();
  });

  test("parseLink écarte un message mal formé", () => {
    expect(parseLink({ to: "main", kind: "insert", tab: "t-1", text: 3 })).toBeNull();
  });

  test("parseLink garde un message bien formé", () => {
    const m = { to: "main", kind: "hello", tab: "t-1" };
    expect(parseLink(m)).toEqual(m);
  });

  test("channelTransport livre à la page elle-même", () => {
    const t = channelTransport();
    const got: LinkMessage[] = [];
    t.listen((m) => void got.push(m));
    t.send({ to: "main", kind: "hello", tab: "t-1" });
    t.close();
    expect(got).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
  });

  test("channelTransport livre à une autre page du même navigateur", async () => {
    const a = channelTransport();
    const b = channelTransport();
    const got: LinkMessage[] = [];
    b.listen((m) => void got.push(m));
    a.send({ to: "main", kind: "hello", tab: "t-1" });
    for (let i = 0; i < 50 && got.length === 0; i++) await new Promise((r) => setTimeout(r, 5));
    a.close();
    b.close();
    expect(got).toEqual([{ to: "main", kind: "hello", tab: "t-1" }]);
  });

  test("parseLink écarte un message mal formé sans journaliser son texte", () => {
    const spy = spyOn(console, "debug").mockImplementation(() => {});
    try {
      const secret = "texte dicté confidentiel";
      expect(parseLink({ to: "main", kind: "insert", tab: "t-1", id: 7, text: secret })).toBeNull();
      expect(spy).toHaveBeenCalled();
      expect(JSON.stringify(spy.mock.calls)).not.toContain(secret);
    } finally {
      spy.mockRestore();
    }
  });

  const context = (look: string) => ({
    to: "agent",
    kind: "context",
    tab: "t-1",
    context: { editing: false, ui: { slide: null, element: null, field: null, editing: false, widget: null }, staged: null, keysOff: false, look },
  });

  test("parseLink contexte avec l'empreinte de l'apparence : accepté", () => {
    expect(parseLink(context("a1b2c3d4"))).not.toBeNull();
  });

  test("parseLink contexte à l'empreinte trop longue : écarté", () => {
    expect(parseLink(context("x".repeat(65)))).toBeNull();
  });
});
