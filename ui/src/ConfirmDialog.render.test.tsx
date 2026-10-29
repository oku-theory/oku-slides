// Rendering of the shared confirmation dialog: armed clicks, no Enter, no digits, Escape.

import { afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ConfirmDialog = (await import("./ConfirmDialog")).default;
const { ARMING_MS } = await import("./modalFrame");

let now = 0;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
beforeEach(() => {
  now = 1_000;
  spyOn(Date, "now").mockImplementation(() => now);
});
afterEach(async () => {
  (Date.now as unknown as { mockRestore: () => void }).mockRestore();
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function render() {
  const calls = { confirm: 0, cancel: 0 };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(ConfirmDialog, {
        message: "Delete kpi?",
        confirmLabel: "Delete",
        onConfirm: () => calls.confirm++,
        onCancel: () => calls.cancel++,
      }),
    ),
  );
  const dialog = host.querySelector('[role="alertdialog"]') as HTMLElement;
  const confirm = dialog.querySelector(".confirm-ok") as HTMLButtonElement;
  return { calls, dialog, confirm };
}

const key = (target: HTMLElement, init: KeyboardEventInit) =>
  act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

test("ConfirmDialog names itself after its message", async () => {
  const { dialog } = await render();
  const described = document.getElementById(dialog.getAttribute("aria-describedby") ?? "");
  expect(described?.textContent).toBe("Delete kpi?");
});

test("ConfirmDialog Enter on the confirm button does nothing", async () => {
  const { calls, confirm } = await render();
  now += ARMING_MS + 1;
  confirm.focus();
  let defaulted = false;
  await act(async () => {
    const e = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    confirm.dispatchEvent(e);
    defaulted = e.defaultPrevented;
  });
  expect([calls.confirm, defaulted]).toEqual([0, true]);
});

test("ConfirmDialog digits do nothing and reach no one", async () => {
  const { calls, confirm } = await render();
  now += ARMING_MS + 1;
  let seen = 0;
  const spy = () => seen++;
  window.addEventListener("keydown", spy);
  await key(confirm, { key: "1", code: "Digit1" });
  window.removeEventListener("keydown", spy);
  expect([calls.confirm, calls.cancel, seen]).toEqual([0, 0, 0]);
});

test("ConfirmDialog a click on confirm within the arming delay does nothing", async () => {
  const { calls, confirm } = await render();
  now += ARMING_MS - 100;
  await act(async () => confirm.click());
  expect(calls.confirm).toBe(0);
});

test("ConfirmDialog a click on confirm after the arming delay confirms", async () => {
  const { calls, confirm } = await render();
  now += ARMING_MS + 1;
  await act(async () => confirm.click());
  expect(calls.confirm).toBe(1);
});

test("ConfirmDialog Escape cancels", async () => {
  const { calls } = await render();
  now += ARMING_MS + 1;
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(calls.cancel).toBe(1);
});

test("ConfirmDialog the cancel button cancels after the arming delay", async () => {
  const { calls, dialog } = await render();
  now += ARMING_MS + 1;
  await act(async () => (dialog.querySelector(".confirm-cancel") as HTMLButtonElement).click());
  expect(calls.cancel).toBe(1);
});
