// A tab's window keys in a pack tab: ⌘Z undoes the session outside the item editor, no stage keys.

import { afterEach, describe, expect, mock, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";
import type { HistoryCursor } from "./HistoryBar";
import type { KeptUi } from "./useKeptUi";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAppKeys } = await import("./useAppKeys");
const { IDLE_AGENT } = await import("./agent/AgentPanel");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** A layout pack's tab: two item slides, the first shown. */
const SNAP = {
  editing: { pack: "p", name: "Acme", kind: "layouts" }, history: null, read_only: false, theming: null,
  slides: [
    { id: "layout-a", kind: "acme/a", title: null, hash: "a" },
    { id: "layout-b", kind: "acme/b", title: null, hash: "b" },
  ],
} as unknown as Snapshot;

const noop = () => {};

/** Mounts the keys of a pack tab; returns the session undo and the slide moves they made. */
async function mount() {
  const undo = mock(async () => null);
  const goToSlide = mock((_id: string) => {});
  const api = { undo, redo: undo } as unknown as TabApi;
  const keptUi = { handleKey: () => false } as unknown as KeptUi;
  function Probe() {
    useAppKeys({
      api, snap: SNAP, view: null, current: "layout-a", selected: null, editing: null, presenting: null,
      agentStatus: IDLE_AGENT, dialogOpen: false, packTab: true, themeTab: false, cutMark: null, keptUi,
      cursor: {} as HistoryCursor, autosave: { flush: async () => {} } as never, run: (p) => p,
      apply: async () => true, present: noop, goToSlide, moveSlide: noop, dropCut: noop, open: noop,
      saveCopy: noop, cancelAgent: noop, setEditing: noop, setSelected: noop,
    });
    return createElement("div", { className: "typst-editor" }, createElement("button", { type: "button" }, "tool"));
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  return { host, undo, goToSlide };
}

const press = (target: EventTarget, init: KeyboardEventInit) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

describe("useAppKeys in a pack tab", () => {
  test("useAppKeys in a pack tab cmd Z outside the editor undoes the session", async () => {
    const { undo } = await mount();
    await press(document.body, { key: "z", metaKey: true });
    expect(undo).toHaveBeenCalledTimes(1);
  });

  test("useAppKeys in a pack tab cmd Z inside the typst editor leaves it to the editor", async () => {
    const { host, undo } = await mount();
    await press(host.querySelector(".typst-editor button")!, { key: "z", metaKey: true });
    expect(undo).not.toHaveBeenCalled();
  });

  test("useAppKeys in a pack tab stage arrows do nothing", async () => {
    const { goToSlide } = await mount();
    await press(document.body, { key: "ArrowRight" });
    await press(document.body, { key: "PageDown" });
    expect(goToSlide).not.toHaveBeenCalled();
  });
});
