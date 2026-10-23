// Données du graphique ou du tableau sélectionné : surface `host-data` lue pour la sélection,
// modifications envoyées à l'élément de la surface affichée.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Element, Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { HOST_DATA, useDataSettings } = await import("./useDataSettings");
const { memory, stub, wait } = await import("./test/widgetApi");

const surface = (): Surface => ({ id: HOST_DATA, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { readOnly?: boolean; history?: boolean }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    host_params: { "base/chart": { chart: "option" }, "base/table": { table: "data" } },
    read_only: !!o.readOnly, history: o.history ? {} : null, undo_label: null, redo_label: null,
    slides: [{ id: "s1", hash: "h1" }],
  }) as unknown as Snapshot;
const element = (id: string, kind: string): Element => ({ id, kind }) as Element;
const action = (name: string, context: Record<string, unknown>): A2uiClientAction =>
  ({ name, surfaceId: HOST_DATA, sourceComponentId: "data", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; selected: Element | undefined; off?: boolean }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

function fake() {
  const { fake } = memory({});
  const reads: [string, string][] = [];
  const edits: [string, string, unknown][] = [];
  stub(fake, "dataPanel", async (slide: string, el: string) => (reads.push([slide, el]), surface()));
  stub(fake, "dataEdit", async (slide: string, el: string, edit: unknown) => (edits.push([slide, el, edit]), snapshot()));
  return { fake, reads, edits };
}

async function mountData(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useDataSettings> | null } = { h: null };
  const run = async (p: Promise<Snapshot>) => {
    try {
      return await p;
    } catch {
      return null;
    }
  };
  function Probe(p: Props) {
    out.h = useDataSettings(api, p.snap, "s1", p.selected, !!p.off, run, () => {});
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

describe("useDataSettings", () => {
  test("useDataSettings with a chart selected reads its host-data surface", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/chart") });
    expect([m.out.h!.surface?.id, f.reads]).toEqual([HOST_DATA, [["s1", "g"]]]);
  });

  test("useDataSettings with a text selected reads nothing", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("t", "base/text") });
    expect([m.out.h!.surface, f.reads.length]).toEqual([null, 0]);
  });

  test("useDataSettings without selection reads nothing", async () => {
    const f = fake();
    await mountData(f.fake, { snap: snapshot(), selected: undefined });
    expect(f.reads.length).toBe(0);
  });

  test("useDataSettings read only reads nothing", async () => {
    const f = fake();
    await mountData(f.fake, { snap: snapshot({ readOnly: true }), selected: element("g", "base/chart") });
    expect(f.reads.length).toBe(0);
  });

  test("useDataSettings in history reads nothing", async () => {
    const f = fake();
    await mountData(f.fake, { snap: snapshot({ history: true }), selected: element("g", "base/chart") });
    expect(f.reads.length).toBe(0);
  });

  test("useDataSettings switched off reads nothing", async () => {
    const f = fake();
    await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/table"), off: true });
    expect(f.reads.length).toBe(0);
  });

  test("useDataSettings data action sends the edit to the surface's element", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/chart") });
    const shown = m.out.h!.surface!;
    // La sélection change ensuite : l'action part quand même vers `g`, l'élément de la surface.
    expect(m.out.h!.onAction(shown, action("data", { edit: { edit: "stack", stacked: true } }))).toBe(true);
    await wait(10);
    expect(f.edits).toEqual([["s1", "g", { edit: "stack", stacked: true }]]);
  });

  test("useDataSettings action without an edit sends nothing", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/chart") });
    m.out.h!.onAction(m.out.h!.surface!, action("data", { edit: null }));
    await wait(10);
    expect(f.edits.length).toBe(0);
  });

  test("useDataSettings action from a replaced surface sends nothing", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/chart") });
    m.out.h!.onAction(surface(), action("data", { edit: { edit: "stack", stacked: true } }));
    await wait(10);
    expect(f.edits.length).toBe(0);
  });

  test("useDataSettings ignores other surfaces", async () => {
    const f = fake();
    const m = await mountData(f.fake, { snap: snapshot(), selected: element("g", "base/chart") });
    const other = { ...surface(), id: "host-widget" } as Surface;
    expect(m.out.h!.onAction(other, action("param", { key: "x" }))).toBe(false);
  });
});
