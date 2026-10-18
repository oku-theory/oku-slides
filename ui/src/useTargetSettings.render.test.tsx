// Réglages de la cible : la surface de l'élément sélectionné (ou du widget de la slide),
// relue avec la sélection, et ses actions adressées à la cible affichée.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { HOST_TARGET, useTargetSettings } = await import("./useTargetSettings");
const { HOST_TEMPLATE, HOST_WIDGET } = await import("./useHostSettings");
const { memory, stub, wait } = await import("./test/widgetApi");

const surface = (id: string): Surface => ({ id, place: "dock", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { history?: boolean; readOnly?: boolean; hash?: string }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    editing: null, widgets: {}, history: o.history ? {} : null, read_only: !!o.readOnly,
    undo_label: null, redo_label: null,
    slides: [{ id: "s-ch1", hash: o.hash ?? "h" }],
  }) as unknown as Snapshot;
const action = (context: Record<string, unknown>): A2uiClientAction =>
  ({ name: "param", surfaceId: HOST_TARGET, sourceComponentId: "f0", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

/** `element` : l'id de l'élément sélectionné, de nature `kind` (une frise par défaut). */
interface Props { snap: Snapshot; current: string; element: string | null; kind?: string; widgetActive?: boolean }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

async function mountTarget(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useTargetSettings> | null; toasts: string[]; runs: number } = { h: null, toasts: [], runs: 0 };
  // Comme `run` de l'App : attend, et dit l'erreur en toast.
  const run = async (p: Promise<Snapshot>) => {
    out.runs += 1;
    try {
      return await p;
    } catch (e) {
      out.toasts.push(String(e));
      return null;
    }
  };
  function Probe(p: Props) {
    const element = p.element === null ? null : { id: p.element, kind: p.kind ?? "frise" };
    out.h = useTargetSettings(api, p.snap, p.current, element, !!p.widgetActive, run, (m) => out.toasts.push(m));
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const render = async (p: Props) => {
    await act(async () => root.render(createElement(Probe, p)));
    await wait(10);
  };
  await render(first);
  return { out, render };
}

/** Une api dont la lecture note ses arguments ; une surface par appel. */
function asking() {
  const { fake } = memory({});
  const asked: unknown[][] = [];
  stub(fake, "targetSettings", async (...a: unknown[]) => (asked.push(a), surface(HOST_TARGET)));
  return { fake, asked };
}

/** Une api dont les envois sont notés, par méthode. */
function sending() {
  const { fake, asked } = asking();
  const sent: unknown[][] = [];
  stub(fake, "targetParam", async (...a: unknown[]) => (sent.push(["param", ...a]), snapshot()));
  stub(fake, "targetList", async (...a: unknown[]) => (sent.push(["list", ...a]), snapshot()));
  return { fake, asked, sent };
}

describe("useTargetSettings", () => {
  test("useTargetSettings element selected reads its surface", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(asked).toEqual([["s-ch1", "fr"]]);
    expect(m.out.h!.surface?.id).toBe(HOST_TARGET);
    await m.render({ snap: snapshot(), current: "s-ch1", element: "c1" });
    expect(asked).toEqual([["s-ch1", "fr"], ["s-ch1", "c1"]]);
  });

  test("useTargetSettings nothing selected reads the slide widget", async () => {
    const { fake, asked } = asking();
    await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: null });
    expect(asked).toEqual([["s-ch1", null]]);
  });

  test("useTargetSettings no surface shows nothing", async () => {
    const { fake } = memory({});
    stub(fake, "targetSettings", async () => null);
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: null });
    expect(m.out.h!.surface).toBeNull();
  });

  test("useTargetSettings read only or history sends no request", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot({ readOnly: true }), current: "s-ch1", element: "fr" });
    expect(m.out.h!.surface).toBeNull();
    await m.render({ snap: snapshot({ history: true }), current: "s-ch1", element: "fr" });
    expect(m.out.h!.surface).toBeNull();
    expect(asked).toEqual([]);
  });

  test("useTargetSettings widget sub-tab active sends no request", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr", widgetActive: true });
    expect(m.out.h!.surface).toBeNull();
    expect(asked).toEqual([]);
  });

  test("useTargetSettings chart selected sends no request", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "c1", kind: "chart" });
    expect(m.out.h!.surface).toBeNull();
    expect(asked).toEqual([]);
    await m.render({ snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(asked).toEqual([["s-ch1", "fr"]]);
  });

  test("useTargetSettings table selected sends no request", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "t1", kind: "table" });
    expect(m.out.h!.surface).toBeNull();
    expect(asked).toEqual([]);
  });

  test("useTargetSettings same snapshot does not reread", async () => {
    const { fake, asked } = asking();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    await m.render({ snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(asked).toHaveLength(1);
    await m.render({ snap: snapshot({ hash: "h2" }), current: "s-ch1", element: "fr" });
    expect(asked).toHaveLength(2);
  });

  test("useTargetSettings value action sends targetParam to the shown target", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(m.out.h!.onAction(m.out.h!.surface!, action({ key: "titre", value: "Étapes" }))).toBe(true);
    await wait(10);
    expect(sent).toEqual([["param", "s-ch1", "fr", "titre", "Étapes"]]);
  });

  test("useTargetSettings list change sends targetList", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change: { op: "remove", item: "d0" } }));
    await wait(10);
    expect(sent).toEqual([["list", "s-ch1", "fr", "etapes", { op: "remove", item: "d0" }]]);
  });

  test("useTargetSettings nested list change sends targetList on the nested list path", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    const change = { op: "nested", path: "d0/taches", change: { op: "remove", item: "t1" } };
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change }));
    await wait(10);
    expect(sent).toEqual([["list", "s-ch1", "fr", "etapes/d0/taches", { op: "remove", item: "t1" }]]);
  });

  test("useTargetSettings replace sends targetParam with the items", async () => {
    const { fake, sent } = sending();
    const items = [{ id: "d0", date: "2020" }];
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change: { op: "replace", items } }));
    await wait(10);
    expect(sent).toEqual([["param", "s-ch1", "fr", "etapes", items]]);
  });

  test("useTargetSettings selection changed before the action keeps the shown target", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    const shown = m.out.h!.surface!;
    // La sélection passe au graphique : la surface de la frise est remplacée.
    await m.render({ snap: snapshot(), current: "s-ch1", element: "c1" });
    expect(m.out.h!.onAction(shown, action({ key: "titre", value: "x" }))).toBe(true);
    // L'action de la surface courante part vers la cible qu'elle montre.
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "titre", value: "y" }));
    await wait(10);
    expect(sent).toEqual([["param", "s-ch1", "c1", "titre", "y"]]);
    expect(sent.some((a) => a[2] === "c1" && a[4] === "x")).toBe(false);
  });

  test("useTargetSettings refusal rereads an identical surface", async () => {
    const { fake, asked } = asking();
    stub(fake, "targetParam", async () => {
      throw new Error("refusé");
    });
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    const before = m.out.h!.surface!;
    m.out.h!.onAction(before, action({ key: "titre", value: 3 }));
    expect(await m.out.h!.outcome.current).toBe(false);
    await wait(10);
    expect(asked).toHaveLength(2);
    expect(m.out.h!.surface).not.toBe(before);
    expect(m.out.toasts).toEqual(["Error: refusé"]);
  });

  test("useTargetSettings claims a stale host-target action without sending", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(m.out.h!.onAction(surface(HOST_TARGET), action({ key: "titre", value: "x" }))).toBe(true);
    expect(await m.out.h!.outcome.current).toBe(false);
    await wait(10);
    expect(sent).toEqual([]);
    expect(m.out.runs).toBe(0);
  });

  test("useTargetSettings ignores host-template and host-widget", async () => {
    const { fake, sent } = sending();
    const m = await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "fr" });
    expect(m.out.h!.onAction(surface(HOST_TEMPLATE), action({ key: "titre", value: "x" }))).toBe(false);
    expect(m.out.h!.onAction(surface(HOST_WIDGET), action({ key: "titre", value: "x" }))).toBe(false);
    await wait(10);
    expect(sent).toEqual([]);
  });
});
