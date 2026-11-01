// The pack tab header's preview-slot select.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: PreviewSpanMenu } = await import("./PreviewSpanMenu");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount(span: "full" | "half" | "third", onPick: (s: string) => void) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(PreviewSpanMenu, { span, onPick })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return host.querySelector("select") as HTMLSelectElement;
}

test("PreviewSpanMenu offers full, half and third, current selected", async () => {
  const select = await mount("half", () => {});
  expect([...select.options].map((o) => o.value)).toEqual(["full", "half", "third"]);
  expect(select.value).toBe("half");
});

test("PreviewSpanMenu change picks the chosen span", async () => {
  const picked: string[] = [];
  const select = await mount("full", (s) => picked.push(s));
  await act(async () => {
    select.value = "third";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(picked).toEqual(["third"]);
});
