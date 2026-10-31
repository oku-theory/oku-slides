// The toolbar of a pack item: the editor's history while its code shows, else the session's; the code switch.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { KeptUi } from "./useKeptUi";

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

type Options = Pick<KeptUi, "code" | "setCode">;

async function mount(code: boolean, history = { undo: 1, redo: 0 }, selected = true) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  const options: Options = {
    code,
    setCode: (v) => calls.push(`code ${v}`),
  };
  await act(async () =>
    root.render(
      createElement(WidgetTools, {
        selected,
        history,
        onUndo: () => calls.push("undo"),
        onRedo: () => calls.push("redo"),
        session: createElement("button", { "aria-label": "session-undo", onClick: () => calls.push("session undo") }),
        options,
      }),
    ),
  );
  const labels = () => [...host.querySelectorAll("button")].map((b) => b.getAttribute("aria-label"));
  const press = (label: string) => act(async () => host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!.click());
  return { host, calls, labels, press };
}

test("WidgetTools with the code shown drives the editor's history", async () => {
  const { labels } = await mount(true);
  expect(labels()).toEqual(["ui-toolbar-undo", "ui-toolbar-redo", "ui-widgets-source"]);
});

test("WidgetTools with the code folded shows the session's history", async () => {
  const { labels } = await mount(false);
  expect(labels()).toEqual(["session-undo", "ui-widgets-source"]);
});

test("WidgetTools without a selected item shows the session's history, code shown or not", async () => {
  const { labels } = await mount(true, { undo: 1, redo: 0 }, false);
  expect(labels()).toEqual(["session-undo", "ui-widgets-source"]);
});

test("WidgetTools the session's undo is pressable with the code folded", async () => {
  const { calls, press } = await mount(false);
  await press("session-undo");
  expect(calls).toEqual(["session undo"]);
});

test("WidgetTools rétablir désactivé sans pas à rétablir", async () => {
  const { host } = await mount(true);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="ui-toolbar-redo"]')!.disabled).toBe(true);
});

test("WidgetTools chaque bouton appelle son action", async () => {
  const { calls, press } = await mount(true);
  for (const l of ["ui-toolbar-undo", "ui-widgets-source"]) await press(l);
  expect(calls).toEqual(["undo", "code false"]);
});

test("WidgetTools with an item selected controls its editor", async () => {
  const { host } = await mount(false);
  expect(host.querySelector(".code-switch")?.getAttribute("aria-controls")).toBe("widget-code");
});

test("WidgetTools without a selected item controls nothing", async () => {
  const { host } = await mount(false, { undo: 0, redo: 0 }, false);
  expect(host.querySelector(".code-switch")?.hasAttribute("aria-controls")).toBe(false);
});
