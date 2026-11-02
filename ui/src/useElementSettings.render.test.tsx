// The selected element's settings: a `host-element` surface read for the selection, each
// value or list change sent to the element of the surface shown.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Element, Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { HOST_ELEMENT, useElementSettings } = await import("./useElementSettings");
const { memory, stub, wait } = await import("./test/widgetApi");

const surface = (): Surface => ({ id: HOST_ELEMENT, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { history?: boolean; theming?: boolean; undo?: string }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    host_params: {},
    guides: null,
    read_only: false, history: o.history ? {} : null, theming: o.theming ? {} : null, undo_label: o.undo ?? null, redo_label: null,
    slides: [{ id: "s1", hash: "h1" }],
  }) as unknown as Snapshot;
const element = (id: string, kind: string): Element => ({ id, kind }) as Element;
const action = (context: Record<string, unknown>): A2uiClientAction =>
  ({ name: "param", surfaceId: HOST_ELEMENT, sourceComponentId: "f0", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; selected: Element | undefined }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

function fake() {
  const { fake } = memory({});
  const reads: [string, string][] = [];
  const params: [string, string, string, unknown][] = [];
  const lists: [string, string, string, unknown][] = [];
  stub(fake, "elementPanel", async (slide: string, el: string) => (reads.push([slide, el]), surface()));
  stub(fake, "elementParam", async (slide: string, el: string, key: string, value: unknown) => (params.push([slide, el, key, value]), snapshot()));
  stub(fake, "elementList", async (slide: string, el: string, key: string, change: unknown) => (lists.push([slide, el, key, change]), snapshot()));
  return { fake, reads, params, lists };
}

async function mountElement(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useElementSettings> | null } = { h: null };
  const run = async (p: Promise<Snapshot>) => {
    try {
      return await p;
    } catch {
      return null;
    }
  };
  function Probe(p: Props) {
    out.h = useElementSettings(api, p.snap, "s1", p.selected, false, run, () => {});
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

describe("useElementSettings", () => {
  test("useElementSettings with any element selected reads its host-element surface", async () => {
    const f = fake();
    const m = await mountElement(f.fake, { snap: snapshot(), selected: element("t", "base/text") });
    expect([m.out.h!.surface?.id, f.reads]).toEqual([HOST_ELEMENT, [["s1", "t"]]]);
  });

  test("useElementSettings in history reads nothing", async () => {
    const f = fake();
    await mountElement(f.fake, { snap: snapshot({ history: true }), selected: element("t", "base/text") });
    expect(f.reads.length).toBe(0);
  });

  test("useElementSettings in a theme tab reads nothing", async () => {
    const f = fake();
    await mountElement(f.fake, { snap: snapshot({ theming: true }), selected: element("t", "base/text") });
    expect(f.reads.length).toBe(0);
  });

  test("useElementSettings param value sends elementParam to the surface's element", async () => {
    const f = fake();
    const m = await mountElement(f.fake, { snap: snapshot(), selected: element("t", "base/text") });
    expect(m.out.h!.onAction(m.out.h!.surface!, action({ key: "shape", value: "b" }))).toBe(true);
    await wait(10);
    expect(f.params).toEqual([["s1", "t", "shape", "b"]]);
  });

  test("useElementSettings list change sends elementList", async () => {
    const f = fake();
    const m = await mountElement(f.fake, { snap: snapshot(), selected: element("l", "base/list") });
    const change = { op: "remove", item: "i1" };
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "items", change }));
    await wait(10);
    expect([f.lists, f.params.length]).toEqual([[["s1", "l", "items", change]], 0]);
  });

  test("useElementSettings rereads after undo", async () => {
    const f = fake();
    const m = await mountElement(f.fake, { snap: snapshot(), selected: element("t", "base/text") });
    await m.render({ snap: snapshot({ undo: "Forme" }), selected: element("t", "base/text") });
    expect(f.reads.length).toBe(2);
  });
});
