// Rendering of the About dialog: version, releases, closing.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { Release } from "./changelog";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const AboutDialog = (await import("./AboutDialog")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");

afterAll(resetMessages);
setMessages("fr", [
  [
    "ui-about = À propos d’Oku Slides",
    "ui-about-version = Version { $version }",
    "ui-about-changelog = Nouveautés",
    "ui-about-breaking = incompatible",
    "ui-about-close = Fermer",
  ].join("\n") + "\n",
]);

const RELEASES: Release[] = [
  {
    version: "7.0.0",
    date: "2026-10-27",
    groups: [{ title: "Added", items: [{ scope: "ui", breaking: true, text: "themes in tabs (t-1)" }] }],
  },
  { version: "6.0.0", date: "2026-10-26", groups: [{ title: "Fixed", items: [{ scope: null, breaking: false, text: "a fix" }] }] },
];

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

async function render(onClose: () => void = () => {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(AboutDialog, { version: "7.0.0", releases: RELEASES, onClose })));
  return host.querySelector('[role="dialog"]') as HTMLElement;
}

test("AboutDialog shows the version", async () => {
  const dialog = await render();
  expect(dialog.querySelector(".about-version")?.textContent).toBe("Version 7.0.0");
});

test("AboutDialog lists every release newest first", async () => {
  const dialog = await render();
  expect([...dialog.querySelectorAll(".about-release h3")].map((h) => h.textContent)).toEqual([
    "7.0.0 · 2026-10-27",
    "6.0.0 · 2026-10-26",
  ]);
});

test("AboutDialog breaking item shows scope and flag", async () => {
  const dialog = await render();
  const item = dialog.querySelector(".about-release li");
  expect([item?.querySelector(".about-scope")?.textContent, item?.querySelector(".about-breaking")?.textContent]).toEqual([
    "ui",
    "incompatible",
  ]);
});

test("AboutDialog close button closes after arming", async () => {
  let closed = 0;
  const dialog = await render(() => closed++);
  now += ARMING_MS + 1;
  await act(async () => (dialog.querySelector(".about-actions button") as HTMLButtonElement).click());
  expect(closed).toBe(1);
});

test("AboutDialog escape closes after arming", async () => {
  let closed = 0;
  await render(() => closed++);
  now += ARMING_MS + 1;
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(closed).toBe(1);
});
