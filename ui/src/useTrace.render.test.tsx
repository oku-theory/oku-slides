// Trace clicks from receipts: show the slide, select the element, open its place.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { SlideView, Snapshot, Surface } from "./api";

const { act, createElement, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useTrace } = await import("./useTrace");
const { t } = await import("./i18n");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const snap = { slides: [{ id: "s1" }, { id: "s2" }] } as unknown as Snapshot;
/** The view of a slide holding `<id>-e1`. */
const viewOf = (id: string) =>
  ({ slide: { id, kind: "essentiel/content", params: { body: [{ id: `${id}-e1`, kind: "base/text" }] } } }) as unknown as SlideView;
const receipt = { id: "report-3" } as Surface;
const trace = (slide: string, element: string, pane = false) => ({ name: "trace", context: { slide, element, pane } });

/** A reduced tab on `s1`: its view follows the shown slide only when `deliver` is called. */
async function mount() {
  const calls: string[] = [];
  let out: {
    handle: ReturnType<typeof useTrace>;
    current: string;
    selected: string | null;
    deliver: () => void;
  } | null = null;
  const Probe = () => {
    const [current, setCurrent] = useState("s1");
    const [view, setView] = useState<SlideView>(viewOf("s1"));
    const [selected, setSelected] = useState<string | null>(null);
    const handle = useTrace({
      snap, view, current, editing: null, show: setCurrent,
      select: (e) => {
        calls.push(`select ${e}`);
        setSelected(e);
      },
      onPane: () => calls.push("pane"),
      onInspector: () => calls.push("inspector"),
      say: (text, error) => calls.push(`say ${text} ${error}`),
    });
    out = { handle, current, selected, deliver: () => setView(viewOf(current)) };
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return { state: () => out!, calls };
}

test("useTrace ignores a trace from a surface that is not a receipt", async () => {
  const h = await mount();
  let handled = true;
  await act(async () => void (handled = h.state().handle({ id: "render-2" } as Surface, trace("s1", "s1-e1"))));
  expect([handled, h.calls]).toEqual([false, []]);
});

test("useTrace selects the element of a trace to the shown slide", async () => {
  const h = await mount();
  await act(async () => void h.state().handle(receipt, trace("s1", "s1-e1")));
  expect(h.state().selected).toBe("s1-e1");
});

test("useTrace shows another slide, then selects once its view arrives", async () => {
  const h = await mount();
  await act(async () => void h.state().handle(receipt, trace("s2", "s2-e1")));
  expect([h.state().current, h.state().selected]).toEqual(["s2", null]);
  await act(async () => h.state().deliver());
  expect(h.state().selected).toBe("s2-e1");
});

test("useTrace to an unknown slide or element says it is gone and changes nothing", async () => {
  const h = await mount();
  await act(async () => void h.state().handle(receipt, trace("s9", "s9-e1")));
  await act(async () => void h.state().handle(receipt, trace("s1", "absent")));
  const gone = `say ${t("ui-trace-gone")} true`;
  expect([h.state().current, h.state().selected, h.calls]).toEqual(["s1", null, [gone, gone]]);
});

test("useTrace with pane opens the data pane after the selection", async () => {
  const h = await mount();
  await act(async () => void h.state().handle(receipt, trace("s1", "s1-e1", true)));
  expect(h.calls).toEqual(["select s1-e1", "pane"]);
});

test("useTrace without pane opens the inspector after the selection", async () => {
  const h = await mount();
  await act(async () => void h.state().handle(receipt, trace("s1", "s1-e1")));
  expect(h.calls).toEqual(["select s1-e1", "inspector"]);
});
