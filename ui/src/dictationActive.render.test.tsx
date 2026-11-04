// The active-dictation counter survives a StrictMode remount (t-998), in a happy-dom DOM.

import "./test/dom";
import { afterEach, expect, test } from "bun:test";

const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { dictationActive, resetDictationActive, useDictationActive } = await import("./dictationActive");

afterEach(resetDictationActive);

function Probe({ on }: { on: boolean }) {
  useDictationActive(on);
  return null;
}

test("dictationActive survives StrictMode remount", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(StrictMode, null, createElement(Probe, { on: true }))));
  expect(dictationActive()).toBe(1);
  await act(async () => root.render(createElement(StrictMode, null, createElement(Probe, { on: false }))));
  expect(dictationActive()).toBe(0);
  await act(async () => root.render(createElement(StrictMode, null, createElement(Probe, { on: true }))));
  await act(async () => root.unmount());
  expect(dictationActive()).toBe(0);
  host.remove();
});
