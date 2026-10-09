import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { memoryTransport } from "../test/link";
import type { LinkContext } from "./link";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useMainLink } = await import("./useMainLink");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const context: LinkContext = { editing: false, insertTo: "nouveau bloc", ui: { slide: "s1", element: null, field: null, editing: false, widget: null }, staged: null, keysOff: false };

async function mountLink({ detached = true, insert = (_t: string) => true } = {}) {
  const transport = memoryTransport();
  const written: string[] = [];
  const Probe = ({ d, c }: { d: boolean; c: LinkContext }) => {
    useMainLink({ tab: "t-1", detached: d, context: c, insert: (t) => (written.push(t), insert(t)), transport });
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe, { d: detached, c: context })));
  cleanup = async () => act(async () => root.unmount());
  const rerender = (d: boolean, c: LinkContext) => act(async () => root.render(createElement(Probe, { d, c })));
  return { transport, written, rerender };
}

const press = (init: KeyboardEventInit) => act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", init)));

describe("useMainLink", () => {
  test("useMainLink détaché : publie le contexte de l'onglet", async () => {
    const m = await mountLink();
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "context", tab: "t-1", context });
  });

  test("useMainLink détaché : un contexte changé est republié", async () => {
    const m = await mountLink();
    const next = { ...context, insertTo: "titre" };
    await m.rerender(true, next);
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "context", tab: "t-1", context: next });
  });

  test("useMainLink détaché : hello redonne le contexte", async () => {
    const m = await mountLink();
    m.transport.sent.length = 0;
    await act(async () => m.transport.send({ to: "main", kind: "hello", tab: "t-1" }));
    expect(m.transport.sent.filter((x) => x.kind === "context")).toHaveLength(1);
  });

  test("useMainLink détaché : insert écrit puis accuse ok", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-7", text: "dicté" }));
    expect(m.written).toEqual(["dicté"]);
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "insert-ack", tab: "t-1", id: "i-7", ok: true });
  });

  test("useMainLink détaché : insert sans zone où écrire accuse refus", async () => {
    const m = await mountLink({ insert: () => false });
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-8", text: "x" }));
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "insert-ack", tab: "t-1", id: "i-8", ok: false });
  });

  test("useMainLink détaché : insert annulé avant d'arriver n'est pas écrit", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert-cancel", tab: "t-1", id: "i-9" }));
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-9", text: "tard" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink détaché : insert d'un autre onglet ignoré", async () => {
    const m = await mountLink();
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-2", id: "i-1", text: "x" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink détaché : ⌘K est relayé brut", async () => {
    const m = await mountLink();
    await press({ key: "k", code: "KeyK", metaKey: true });
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "shortcut", tab: "t-1", key: "type" });
  });

  test("useMainLink détaché : ⌘⇧D est relayé brut, jamais en arrêt de l'agent", async () => {
    const m = await mountLink();
    await press({ key: "D", code: "KeyD", metaKey: true, shiftKey: true });
    expect(m.transport.sent).toContainEqual({ to: "agent", kind: "shortcut", tab: "t-1", key: "draft" });
  });

  test("useMainLink rattaché : hello ne publie rien", async () => {
    const m = await mountLink({ detached: false });
    await act(async () => m.transport.send({ to: "main", kind: "hello", tab: "t-1" }));
    expect(m.transport.sent.filter((x) => x.to === "agent")).toEqual([]);
  });

  test("useMainLink rattaché : insert n'écrit rien", async () => {
    const m = await mountLink({ detached: false });
    await act(async () => m.transport.send({ to: "main", kind: "insert", tab: "t-1", id: "i-1", text: "x" }));
    expect(m.written).toEqual([]);
  });

  test("useMainLink rattaché : ⌘K n'est pas relayé", async () => {
    const m = await mountLink({ detached: false });
    await press({ key: "k", code: "KeyK", metaKey: true });
    expect(m.transport.sent.filter((x) => x.kind === "shortcut")).toEqual([]);
  });

  test("useMainLink détaché puis rattaché : l'écoute est retirée", async () => {
    const m = await mountLink();
    await m.rerender(false, context);
    expect(m.transport.listeners()).toBe(0);
  });
});
