// A tab's window keys: in a pack tab ⌘Z undoes the session outside the item editor, no stage
// keys; ⌘↩ presents only when the snapshot says the tab can; in a deck tab, A opens the add grid.

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

/** A deck tab: one slide, nothing being edited. */
const DECK = {
  editing: null, history: null, read_only: false, theming: null,
  slides: [{ id: "s-1", kind: "essentiel/content", title: null, hash: "a" }],
} as unknown as Snapshot;

/** Mounts the keys of a tab (a pack tab by default); returns the undo, slide moves, presents and add openings made. */
async function mount({ snap = SNAP, packTab = true, canAdd = false }: { snap?: Snapshot; packTab?: boolean; canAdd?: boolean } = {}) {
  const openAdd = mock(() => {});
  const undo = mock(async () => null);
  const present = mock((_mode: string) => {});
  const goToSlide = mock((_id: string) => {});
  const api = { undo, redo: undo } as unknown as TabApi;
  /** The `deck` flag of each key handed to the kept UI: true lets ⌘⇧E toggle the inspector. */
  const deckKeys: boolean[] = [];
  const keptUi = { handleKey: (_e: unknown, deck = false) => (deckKeys.push(deck), false) } as unknown as KeptUi;
  function Probe() {
    useAppKeys({
      api, snap, view: null, current: snap.slides[0].id, selected: null, editing: null, presenting: null,
      agentStatus: IDLE_AGENT, dialogOpen: false, packTab, themeTab: false, cutMark: null, keptUi,
      cursor: {} as HistoryCursor, autosave: { flush: async () => {} } as never, run: (p) => p,
      apply: async () => true, present, goToSlide, moveSlide: noop, dropCut: noop, open: noop,
      saveCopy: noop, cancelAgent: noop, setEditing: noop, setSelected: noop, openAdd: canAdd ? openAdd : null,
    });
    return createElement("div", { className: "typst-editor" }, createElement("button", { type: "button" }, "tool"));
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Probe)));
  return { host, undo, goToSlide, present, openAdd, deckKeys };
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

describe("useAppKeys add shortcut", () => {
  const deck = () => mount({ snap: DECK, packTab: false, canAdd: true });

  test("useAppKeys A in a deck tab opens the add grid", async () => {
    const { openAdd } = await deck();
    await press(document.body, { key: "a", code: "KeyA" });
    expect(openAdd).toHaveBeenCalledTimes(1);
  });

  test("useAppKeys A is read by its letter, so it follows the layout", async () => {
    const { openAdd } = await deck();
    await press(document.body, { key: "a", code: "KeyQ" });
    expect(openAdd).toHaveBeenCalledTimes(1);
  });

  test("useAppKeys A with a modifier does not open the add grid", async () => {
    const { openAdd } = await deck();
    await press(document.body, { key: "a", code: "KeyA", metaKey: true });
    await press(document.body, { key: "a", code: "KeyA", altKey: true });
    await press(document.body, { key: "A", code: "KeyA", shiftKey: true });
    expect(openAdd).not.toHaveBeenCalled();
  });

  test("useAppKeys A held down opens the add grid once", async () => {
    const { openAdd } = await deck();
    await press(document.body, { key: "a", code: "KeyA" });
    await press(document.body, { key: "a", code: "KeyA", repeat: true });
    expect(openAdd).toHaveBeenCalledTimes(1);
  });

  test("useAppKeys A without an add target (openAdd null) does nothing", async () => {
    const { openAdd } = await mount({ snap: DECK, packTab: false, canAdd: false });
    await press(document.body, { key: "a", code: "KeyA" });
    expect(openAdd).not.toHaveBeenCalled();
  });

  test("useAppKeys A in a text field is the field's", async () => {
    const { host, openAdd } = await deck();
    const input = host.appendChild(document.createElement("input"));
    await press(input, { key: "a", code: "KeyA" });
    expect(openAdd).not.toHaveBeenCalled();
  });
});

describe("useAppKeys present", () => {
  const deck = (canPresent: boolean) => ({ ...SNAP, editing: null, can_present: canPresent }) as Snapshot;

  test("useAppKeys present when the snapshot can present presents", async () => {
    const { present } = await mount({ snap: deck(true), packTab: false });
    await press(document.body, { key: "Enter", metaKey: true });
    expect(present).toHaveBeenCalledWith("screen");
  });

  test("useAppKeys present when the snapshot cannot present does nothing", async () => {
    const { present } = await mount({ snap: deck(false), packTab: false });
    await press(document.body, { key: "Enter", metaKey: true });
    expect(present).not.toHaveBeenCalled();
  });
});

describe("useAppKeys inspector toggle", () => {
  const inspectorKey = { key: "e", code: "KeyE", metaKey: true, shiftKey: true };

  test("useAppKeys cmd shift E in a deck tab may toggle the inspector", async () => {
    const { deckKeys } = await mount({ snap: DECK, packTab: false });
    await press(document.body, inspectorKey);
    expect(deckKeys).toEqual([true]);
  });

  test.each([
    ["a read-only deck", { ...DECK, read_only: true } as Snapshot],
    ["history", { ...DECK, history: {} } as unknown as Snapshot],
    ["a pack tab", SNAP],
  ])("useAppKeys cmd shift E in %s never toggles the inspector", async (_, snap) => {
    const { deckKeys } = await mount({ snap, packTab: snap === SNAP });
    await press(document.body, inspectorKey);
    expect(deckKeys).not.toContain(true);
  });
});
