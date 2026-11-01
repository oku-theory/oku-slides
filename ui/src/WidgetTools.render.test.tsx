// The toolbar of a pack item: the editor's history while its code shows, else the session's.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: WidgetTools } = await import("./WidgetTools");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(editor: boolean, history = { undo: 1, redo: 0 }) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  await act(async () =>
    root.render(
      createElement(WidgetTools, {
        editor,
        history,
        onUndo: () => calls.push("undo"),
        onRedo: () => calls.push("redo"),
        session: createElement("button", { "aria-label": "session-undo", onClick: () => calls.push("session undo") }),
      }),
    ),
  );
  const labels = () => [...host.querySelectorAll("button")].map((b) => b.getAttribute("aria-label"));
  const press = (label: string) => act(async () => host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!.click());
  return { host, calls, labels, press };
}

test("WidgetTools with the code shown drives the editor's history", async () => {
  const { labels } = await mount(true);
  expect(labels()).toEqual(["ui-toolbar-undo", "ui-toolbar-redo"]);
});

test("WidgetTools without the code shows the session's history", async () => {
  const { labels } = await mount(false);
  expect(labels()).toEqual(["session-undo"]);
});

test("WidgetTools the session's undo is pressable without the code", async () => {
  const { calls, press } = await mount(false);
  await press("session-undo");
  expect(calls).toEqual(["session undo"]);
});

test("WidgetTools rétablir désactivé sans pas à rétablir", async () => {
  const { host } = await mount(true);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="ui-toolbar-redo"]')!.disabled).toBe(true);
});

test("WidgetTools chaque bouton appelle son action", async () => {
  const { calls, press } = await mount(true, { undo: 1, redo: 1 });
  for (const l of ["ui-toolbar-undo", "ui-toolbar-redo"]) await press(l);
  expect(calls).toEqual(["undo", "redo"]);
});
