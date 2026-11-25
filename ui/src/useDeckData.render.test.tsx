// The deck's data in a deck tab: the `deck-data` list, the `data-sheet` page opened in the data
// pane, and the routing of their actions.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DATA_SHEET, DECK_DATA, useDeckData } = await import("./useDeckData");
const { memory, stub, wait } = await import("./test/widgetApi");

const surface = (id: string): Surface => ({ id, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
const snapshot = (): Snapshot =>
  ({ read_only: false, history: null, theming: null, editing: null, slides: [] }) as unknown as Snapshot;
const action = (id: string, edit: unknown): A2uiClientAction =>
  ({ name: "data", surfaceId: id, sourceComponentId: "root", timestamp: new Date(0).toISOString(), context: { edit } }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; paneOpen: boolean; selected: string | null }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

/** An api whose sheets exist while `alive` holds their data. */
function fake(alive = new Set(["d9"])) {
  const { fake } = memory({});
  const sheets: [string, number][] = [];
  const sent: unknown[] = [];
  stub(fake, "dataList", async () => surface(DECK_DATA));
  stub(fake, "dataSheet", async (data: string, offset: number) =>
    (sheets.push([data, offset]), alive.has(data) ? surface(DATA_SHEET) : null));
  stub(fake, "dataAction", async (a: unknown) => (sent.push(a), snapshot()));
  return { fake, sheets, sent, alive };
}

async function mountDeckData(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useDeckData> | null } = { h: null };
  const panes: number[] = [];
  const selects: [string, string][] = [];
  const run = async (p: Promise<Snapshot>) => {
    try {
      return await p;
    } catch {
      return null;
    }
  };
  function Probe(p: Props) {
    out.h = useDeckData({
      api, snap: p.snap, listOpen: true, paneOpen: p.paneOpen, selected: p.selected,
      onPane: () => panes.push(1), select: (s, e) => selects.push([s, e]), run, say: () => {},
    });
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
  return { out, render, panes, selects };
}

/** Sends `edit` from the surface `h` shows for `id`. */
async function send(h: ReturnType<typeof useDeckData>, id: string, edit: unknown) {
  const shown = id === DECK_DATA ? h.list.surface : h.sheet.surface;
  if (!shown) throw new Error(`${id} not shown`);
  await act(async () => void h.onAction(shown, action(id, edit)));
  await wait(10);
}

describe("useDeckData", () => {
  test("useDeckData select opens the pane and its sheet", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: false, selected: null });
    await send(m.out.h!, DECK_DATA, { action: "select", data: "d9" });
    expect([m.panes.length, m.out.h!.sheetOpen]).toEqual([1, true]);
    await m.render({ snap: snapshot(), paneOpen: true, selected: null });
    expect([m.out.h!.sheet.surface?.id, f.sheets]).toEqual([DATA_SHEET, [["d9", 0]]]);
  });

  test("useDeckData a new selection gives the pane back to the view", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: true, selected: null });
    await act(async () => m.out.h!.openSheet("d9"));
    await wait(10);
    expect(m.out.h!.sheetOpen).toBe(true);
    await m.render({ snap: snapshot(), paneOpen: true, selected: "ch1" });
    expect([m.out.h!.sheetOpen, m.out.h!.sheet.surface]).toEqual([false, null]);
  });

  test("useDeckData closing the pane closes the sheet", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: true, selected: null });
    await act(async () => m.out.h!.openSheet("d9"));
    await wait(10);
    await m.render({ snap: snapshot(), paneOpen: false, selected: null });
    expect(m.out.h!.sheetOpen).toBe(false);
  });

  test("useDeckData page rereads the sheet at the offset", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: true, selected: null });
    await act(async () => m.out.h!.openSheet("d9"));
    await wait(10);
    await send(m.out.h!, DATA_SHEET, { action: "page", data: "d9", offset: 100 });
    expect(f.sheets).toEqual([["d9", 0], ["d9", 100]]);
  });

  test("useDeckData session actions go to the session and reread", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: true, selected: null });
    await act(async () => m.out.h!.openSheet("d9"));
    await wait(10);
    const paste = { action: "paste", data: "d9", row: "r1", column: "c1", cells: [[1, 2]], names: [] };
    await send(m.out.h!, DATA_SHEET, paste);
    await m.render({ snap: snapshot(), paneOpen: true, selected: null });
    expect([f.sent, f.sheets.length]).toEqual([[paste], 2]);
  });

  test("useDeckData open-element selects the view", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: false, selected: null });
    await send(m.out.h!, DECK_DATA, { action: "open-element", slide: "s1", element: "ch1" });
    expect([m.selects, f.sent]).toEqual([[["s1", "ch1"]], []]);
  });

  test("useDeckData sheet of a removed data closes it", async () => {
    const f = fake();
    const m = await mountDeckData(f.fake, { snap: snapshot(), paneOpen: true, selected: null });
    await act(async () => m.out.h!.openSheet("d9"));
    await wait(10);
    f.alive.delete("d9");
    await m.render({ snap: snapshot(), paneOpen: true, selected: null });
    expect(m.out.h!.sheetOpen).toBe(false);
  });
});
