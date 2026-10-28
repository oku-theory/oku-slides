// Close guards: the work a tab finishes before it closes, across renders, unmounts and remounts.

import { afterEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi, WidgetChange } from "./api";

const { act, createElement, StrictMode, useCallback, useRef, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { runCloseGuard, useCloseGuard } = await import("./closeGuards");
const { usePackWidgetEditor } = await import("./usePackWidgetEditor");
const { useFileActions } = await import("./useFileActions");
const WidgetPane = (await import("./WidgetPane")).default;
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

const roots: { root: ReturnType<typeof createRoot>; host: HTMLElement }[] = [];
/** Guard work still pending when a test ends, awaited by `afterEach`. */
const leftovers: Promise<unknown>[] = [];
afterEach(async () => {
  for (const m of roots.splice(0)) {
    await act(async () => m.root.unmount());
    m.host.remove();
  }
  await Promise.all(leftovers.splice(0));
});

let ids = 0;
/** A tab id no other test uses: the guards live in a module-level map. */
const freshTab = () => `guard-tab-${++ids}`;

/** Mounts a component calling `useCloseGuard(tab, guard)`; `rerender` swaps the guard. */
async function mountGuard(tab: string, guard: () => Promise<void>, strict = false) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const entry = { root, host };
  roots.push(entry);
  function Probe({ g }: { g: () => Promise<void> }) {
    useCloseGuard(tab, g);
    return null;
  }
  const draw = (g: () => Promise<void>) => {
    const probe = createElement(Probe, { g });
    return strict ? createElement(StrictMode, null, probe) : probe;
  };
  await act(async () => root.render(draw(guard)));
  return {
    rerender: (g: () => Promise<void>) => act(async () => root.render(draw(g))),
    unmount: async () => {
      roots.splice(roots.indexOf(entry), 1);
      await act(async () => root.unmount());
      host.remove();
    },
  };
}

/** True once `p` has settled, checked after a short wait. */
async function settled(p: Promise<unknown>) {
  let done = false;
  void p.then(
    () => (done = true),
    () => (done = true),
  );
  await wait(10);
  return done;
}

test("runCloseGuard runs the registered guard and answers done", async () => {
  const tab = freshTab();
  let calls = 0;
  await mountGuard(tab, async () => void calls++);
  expect(await runCloseGuard(tab, 500)).toBe("done");
  expect(calls).toBe(1);
});

test("runCloseGuard without a guard answers done", async () => {
  expect(await runCloseGuard(freshTab(), 500)).toBe("done");
});

test("runCloseGuard answers timeout when the guard hangs", async () => {
  const tab = freshTab();
  const hang = deferred<void>();
  leftovers.push(hang.promise);
  await mountGuard(tab, () => hang.promise);
  expect(await runCloseGuard(tab, 20)).toBe("timeout");
  hang.resolve();
});

test("a re-render does not run the guard", async () => {
  const tab = freshTab();
  const calls: string[] = [];
  const m = await mountGuard(tab, async () => void calls.push("first"));
  await m.rerender(async () => void calls.push("second"));
  expect(calls).toEqual([]);
  await runCloseGuard(tab, 500);
  expect(calls).toEqual(["second"]);
});

test("unmount keeps the guard until its pending work settles", async () => {
  const tab = freshTab();
  const work = deferred<void>();
  const m = await mountGuard(tab, () => work.promise);
  await m.unmount();
  const closing = runCloseGuard(tab, 2000);
  expect(await settled(closing)).toBe(false);
  work.resolve();
  expect(await closing).toBe("done");
});

test("unmount starts no new write", async () => {
  const tab = freshTab();
  let calls = 0;
  const m = await mountGuard(tab, async () => void calls++);
  await m.unmount();
  expect(calls).toBe(1);
  expect(await runCloseGuard(tab, 500)).toBe("done");
  expect(calls).toBe(1);
});

test("a quick remount waits for the previous mount's pending work", async () => {
  const tab = freshTab();
  const work = deferred<void>();
  const first = await mountGuard(tab, () => work.promise);
  await first.unmount();
  await mountGuard(tab, async () => {});
  const closing = runCloseGuard(tab, 2000);
  expect(await settled(closing)).toBe(false);
  work.resolve();
  expect(await closing).toBe("done");
});

test("a StrictMode remount keeps the live guard and writes nothing extra", async () => {
  const tab = freshTab();
  let dirty = false;
  let writes = 0;
  await mountGuard(
    tab,
    async () => {
      if (dirty) writes++;
      dirty = false;
    },
    true,
  );
  expect(writes).toBe(0);
  dirty = true;
  expect(await runCloseGuard(tab, 500)).toBe("done");
  expect(writes).toBe(1);
});

test("a stale unregister keeps the newer guard", async () => {
  const tab = freshTab();
  const calls: string[] = [];
  const older = await mountGuard(tab, async () => void calls.push("older"));
  await mountGuard(tab, async () => void calls.push("newer"));
  await older.unmount();
  calls.length = 0;
  await runCloseGuard(tab, 500);
  expect(calls).toEqual(["newer"]);
});

test("the guard entry is gone once unmounted work settles", async () => {
  const tab = freshTab();
  const work = deferred<void>();
  const removed = spyOn(Map.prototype, "delete");
  try {
    const m = await mountGuard(tab, () => work.promise);
    await m.unmount();
    expect(removed.mock.calls.some(([key]) => key === tab)).toBe(false);
    work.resolve();
    await runCloseGuard(tab, 500);
    await wait(0);
    expect(removed.mock.calls.some(([key]) => key === tab)).toBe(true);
  } finally {
    removed.mockRestore();
  }
});

test("a remount older than ms whose previous write still hangs answers timeout", async () => {
  const tab = freshTab();
  const hang = deferred<void>();
  leftovers.push(hang.promise);
  const first = await mountGuard(tab, () => hang.promise);
  await first.unmount();
  await mountGuard(tab, async () => {});
  await wait(30);
  expect(await runCloseGuard(tab, 20)).toBe("timeout");
  hang.resolve();
});

test("remount then unmount waits for the first mount's work", async () => {
  const tab = freshTab();
  const work = deferred<void>();
  const first = await mountGuard(tab, () => work.promise);
  await first.unmount();
  const second = await mountGuard(tab, async () => {});
  await second.unmount();
  const closing = runCloseGuard(tab, 2000);
  expect(await settled(closing)).toBe(false);
  work.resolve();
  expect(await closing).toBe("done");
});

test("real editor hooks: the guard waits for the typed write and sends it once", async () => {
  const tab = freshTab();
  const { fake, log } = memory({ kpi: "k" });
  const written = deferred<void>();
  stub(fake, "setWidget", async (name: string, source: string): Promise<WidgetChange> => {
    log.push(`set ${name}:${source}`);
    await written.promise;
    return { status: { name, schema: null, diagnostics: [] }, broken: [], saved: true };
  });
  const first = {
    dirty: false, path: null, name: "p", history: null, read_only: false,
    editing: { pack: "p", base: "b", kind: "widgets", leased: true },
    widgets: { deck: { id: "p", version: "v" } }, layouts: {},
    ui: { slide: null, grid: true, ghosts: false, pile: null, notes: null, widgets: ["kpi"], widget: "kpi" },
  } as unknown as Snapshot;
  // App's declaration order: the widget editor, the file actions, then the close guard.
  function Tab({ api }: { api: TabApi }) {
    const [snap, setSnap] = useState<Snapshot | null>(first);
    const snapRef = useRef(snap);
    snapRef.current = snap;
    const run = useCallback(async (p: Promise<Snapshot | null>) => {
      const s = await p.catch(() => null);
      if (s) setSnap(s);
      return s;
    }, []);
    const widgets = usePackWidgetEditor(api, snap, { changed: () => {}, error: () => {}, notice: () => {}, blocked: () => false });
    const { autosave } = useFileActions({ api, snap, snapRef, run, say: () => {}, onOpen: () => {}, onImportPick: () => {} });
    useCloseGuard(tab, async () => {
      await widgets.files.settle();
      await autosave.flush();
    });
    return createElement(WidgetPane, { api, widgets, trials: 0, look: "", code: false, codeWidth: null, onCodeWidth: () => {}, run, palette: [] });
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Tab, { api: fake })));
  await wait(20);
  const content = host.querySelector(".cm-content") as HTMLElement;
  const view = EditorView.findFromDOM(content)!;
  await act(async () => {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "k2" }, userEvent: "input.type" });
  });
  await act(async () => root.unmount());
  host.remove();
  const closing = runCloseGuard(tab, 2000);
  expect(await settled(closing)).toBe(false);
  written.resolve();
  expect(await closing).toBe("done");
  expect(log).toEqual(["set kpi:k2"]);
});
