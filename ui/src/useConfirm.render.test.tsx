// The shared question queue: one ConfirmDialog at a time, in order, answered no on unmount.

import { afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useConfirm } = await import("./useConfirm");
const { ARMING_MS } = await import("./modalFrame");

type Asker = ReturnType<typeof useConfirm>;

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

/** Mounts a host that renders the hook's dialog; `asker()` is its latest value. */
async function render() {
  let latest: Asker | null = null;
  function Host() {
    const asker = useConfirm();
    latest = asker;
    return asker.dialog;
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Host)));
  return { host, asker: () => latest as unknown as Asker };
}

const ask = (message: string) => ({ message, ok: "Close without saving", danger: true });

/** Clicks `selector` in the shown dialog once it is armed. */
async function click(host: HTMLElement, selector: string) {
  now += ARMING_MS + 1;
  await act(async () => (host.querySelector(selector) as HTMLButtonElement).click());
}

test("useConfirm confirm shows the question with its labels", async () => {
  const { host, asker } = await render();
  await act(async () => void asker().confirm({ message: "Update?", ok: "Update", danger: false, cancel: "Not now" }));
  expect(host.querySelector(".confirm-message")?.textContent).toBe("Update?");
  expect(host.querySelector(".confirm-ok")?.textContent).toBe("Update");
  expect(host.querySelector(".confirm-cancel")?.textContent).toBe("Not now");
});

test("useConfirm confirm resolves true on the confirming button", async () => {
  const { host, asker } = await render();
  let answer: Promise<boolean> | null = null;
  await act(async () => void (answer = asker().confirm(ask("Quit?"))));
  await click(host, ".confirm-ok");
  expect(await answer).toBe(true);
});

test("useConfirm asking is true only while a question is shown", async () => {
  const { host, asker } = await render();
  await act(async () => void asker().confirm(ask("Quit?")));
  expect(asker().asking).toBe(true);
  await click(host, ".confirm-cancel");
  expect(asker().asking).toBe(false);
});

test("useConfirm two questions show one after the other, in order", async () => {
  const { host, asker } = await render();
  let first: Promise<boolean> | null = null;
  let second: Promise<boolean> | null = null;
  await act(async () => {
    first = asker().confirm(ask("First?"));
    second = asker().confirm(ask("Second?"));
  });
  expect(host.querySelectorAll(".confirm-dialog")).toHaveLength(1);
  expect(host.querySelector(".confirm-message")?.textContent).toBe("First?");
  await click(host, ".confirm-cancel");
  expect(await first).toBe(false);
  expect(host.querySelector(".confirm-message")?.textContent).toBe("Second?");
  await click(host, ".confirm-ok");
  expect(await second).toBe(true);
});

test("useConfirm the next question is armed afresh", async () => {
  const { host, asker } = await render();
  await act(async () => {
    void asker().confirm(ask("First?"));
    void asker().confirm(ask("Second?"));
  });
  await click(host, ".confirm-ok");
  expect(host.querySelector(".confirm-dialog")?.hasAttribute("data-armed")).toBe(false);
});

test("useConfirm a question asked after unmount is answered no", async () => {
  const { asker } = await render();
  const confirm = asker().confirm;
  const m = mounted;
  mounted = null;
  await act(async () => m?.root.unmount());
  m?.host.remove();
  expect(await confirm(ask("Late?"))).toBe(false);
});

test("useConfirm unmount answers every pending question no", async () => {
  const { asker } = await render();
  let first: Promise<boolean> | null = null;
  let second: Promise<boolean> | null = null;
  await act(async () => {
    first = asker().confirm(ask("First?"));
    second = asker().confirm(ask("Second?"));
  });
  const m = mounted;
  mounted = null;
  await act(async () => m?.root.unmount());
  m?.host.remove();
  expect(await Promise.all([first, second])).toEqual([false, false]);
});
