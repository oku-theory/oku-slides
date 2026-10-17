// Raccourcis des onglets de la fenêtre : retenus derrière un dialogue, et pendant une saisie.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { TabAction } from "./tabs";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useTabShortcuts } = await import("./useTabShortcuts");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount(held: () => boolean, windows = true) {
  const actions: TabAction[] = [];
  const Probe = () => {
    useTabShortcuts(true, { mac: true, windows, held, onAction: (a) => actions.push(a) });
    return createElement("input", { "aria-label": "champ" });
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return { actions, input: host.querySelector("input")! };
}

const press = (target: EventTarget, init: KeyboardEventInit) => {
  const e = new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true });
  target.dispatchEvent(e);
  return e;
};

test("useTabShortcuts cmd T opens a tab and takes the key", async () => {
  const { actions } = await mount(() => false);
  const e = press(document.body, { key: "t", code: "KeyT", metaKey: true });
  expect(actions).toEqual([{ kind: "new-tab" }]);
  expect(e.defaultPrevented).toBe(true);
});

test.each([
  ["cmd T", { key: "t", code: "KeyT", metaKey: true }],
  ["cmd W", { key: "w", code: "KeyW", metaKey: true }],
  ["cmd 1", { key: "&", code: "Digit1", metaKey: true }],
  ["ctrl Tab", { key: "Tab", code: "Tab", ctrlKey: true }],
])("useTabShortcuts held by a dialog ignores %s", async (_name, init) => {
  const { actions } = await mount(() => true);
  const e = press(document.body, init);
  expect(actions).toEqual([]);
  expect(e.defaultPrevented).toBe(false);
});

test("useTabShortcuts while typing keeps only ctrl Tab", async () => {
  const { actions, input } = await mount(() => false);
  press(input, { key: "t", code: "KeyT", metaKey: true });
  press(input, { key: "Tab", code: "Tab", ctrlKey: true });
  expect(actions).toEqual([{ kind: "cycle", delta: 1 }]);
});

test("useTabShortcuts without windows ignores new window", async () => {
  const { actions } = await mount(() => false, false);
  press(document.body, { key: "n", code: "KeyN", metaKey: true, shiftKey: true });
  expect(actions).toEqual([]);
});
