// The tab's autosave: paused by a refused guarded save, restarted when the deck moves.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { Autosave } from "./autosave";
import type { Snapshot, TabApi } from "./api";

const { act, createElement, useCallback, useRef } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useFileActions } = await import("./useFileActions");

const roots: { root: ReturnType<typeof createRoot>; host: HTMLElement }[] = [];
const confirm = window.confirm;
afterEach(async () => {
  for (const m of roots.splice(0)) {
    await act(async () => m.root.unmount());
    m.host.remove();
  }
  window.confirm = confirm;
});

/** A modified deck at `path`; `n` makes each snapshot a new object. */
const deck = (path: string, n: number) =>
  ({ dirty: true, path, name: `d${n}`, editing: null, theming: null, history: null, read_only: false }) as unknown as Snapshot;

/** Mounts `useFileActions` over `snap`; `show` swaps the snapshot, `autosave` is the hook's. */
async function mount(api: TabApi, first: Snapshot) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const handle: { autosave: Autosave | null } = { autosave: null };
  function Tab({ snap }: { snap: Snapshot }) {
    const snapRef = useRef<Snapshot | null>(snap);
    snapRef.current = snap;
    const run = useCallback(async (p: Promise<Snapshot | null>) => p.catch(() => null), []);
    const { autosave } = useFileActions({ api, snap, snapRef, run, say: () => {}, onOpen: () => {}, onImportPick: () => {} });
    handle.autosave = autosave;
    return null;
  }
  await act(async () => root.render(createElement(Tab, { snap: first })));
  return {
    show: (snap: Snapshot) => act(async () => root.render(createElement(Tab, { snap }))),
    /** Runs the pending save now; nothing when none is scheduled. */
    flush: () => act(async () => handle.autosave?.flush()),
  };
}

test("autosave_refusedOverwriteThenMove_pausesUntilThePathChanges", async () => {
  // Browser fallback of the "file replaced" question: refuse the overwrite.
  window.confirm = () => false;
  const saves: string[] = [];
  let replaced = true;
  const api = {
    save: async (path: string | null, force: boolean) => {
      saves.push(`${path}:${force}`);
      return { ...deck("/moved.oku", 0), dirty: false, disk_replaced: replaced };
    },
  } as unknown as TabApi;
  const tab = await mount(api, deck("/a.oku", 1));
  await tab.flush();
  expect(saves).toEqual(["null:false"]);

  // Paused: later changes schedule nothing.
  await tab.show(deck("/a.oku", 2));
  await tab.flush();
  await tab.show(deck("/a.oku", 3));
  await tab.flush();
  expect(saves).toEqual(["null:false"]);

  // The agent moved the deck: autosave saves again.
  replaced = false;
  await tab.show(deck("/moved.oku", 4));
  await tab.flush();
  expect(saves).toEqual(["null:false", "null:false"]);
});
