// The install dialog for received guides, rendered in a happy-dom DOM.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { Guides, GuidesReview, Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const GuidesInstallDialog = (await import("./GuidesInstallDialog")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");
const { readFileSync } = await import("node:fs");

afterAll(resetMessages);
setMessages("en", [readFileSync(new URL("../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url), "utf8")]);

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

const run = (p: Promise<Snapshot>) => p.then(() => null);
const review = (guides: Guides, version = "v1", extra: Partial<GuidesReview> = {}): GuidesReview => ({
  id: `id-${version}`, version, kind: "widgets", name: "Brought", items: 3, requires: [], guides, ...extra,
});

function spyApi(calls: unknown[]): TabApi {
  const record = (name: string) => (versions: unknown) => (calls.push([name, versions]), Promise.resolve({} as Snapshot));
  return { approveGuides: record("approve"), laterGuides: record("later") } as unknown as TabApi;
}

async function render(reviews: GuidesReview[], calls: unknown[] = []) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(GuidesInstallDialog, { api: spyApi(calls), reviews, run })));
  return host;
}
const button = (host: HTMLElement, name: string) =>
  [...host.querySelectorAll("button")].find((b) => b.textContent === name) as HTMLButtonElement;
const click = (el: HTMLElement) => act(async () => void el.dispatchEvent(new MouseEvent("click", { bubbles: true })));

test("GuidesInstallDialog shows each guide as plain text", async () => {
  const host = await render([review({ use: "<b>bold</b>", edit: "[link](https://x.test)" })]);
  const pres = [...host.querySelectorAll("pre")];
  expect(pres.map((p) => p.textContent)).toEqual(["[link](https://x.test)", "<b>bold</b>"]);
  expect(host.querySelector("pre b, pre a")).toBeNull();
});

test("GuidesInstallDialog describes what each version holds", async () => {
  const host = await render([review({ use: "u" }, "v1", { requires: ["a", "b"] })]);
  expect(host.querySelector(".guides-review-name")?.textContent).toBe("Brought");
  expect(host.querySelector(".guides-review .caption")?.textContent).toBe("Widgets · 3 items · Requires \u2068\u2068a\u2069, \u2068b\u2069\u2069");
});

test("GuidesInstallDialog focuses Later by default", async () => {
  const host = await render([review({ use: "u" })]);
  expect(document.activeElement).toBe(button(host, "Later"));
});

test("GuidesInstallDialog approve sends every listed version", async () => {
  const calls: unknown[] = [];
  const host = await render([review({ use: "a" }, "v1"), review({ edit: "b" }, "v2")], calls);
  now += ARMING_MS + 1;
  await click(button(host, "Approve guides"));
  expect(calls).toEqual([["approve", [{ id: "id-v1", version: "v1" }, { id: "id-v2", version: "v2" }]]]);
});

test("GuidesInstallDialog Later sends every listed version", async () => {
  const calls: unknown[] = [];
  const host = await render([review({ use: "a" }, "v1"), review({ edit: "b" }, "v2")], calls);
  now += ARMING_MS + 1;
  await click(button(host, "Later"));
  expect(calls).toEqual([["later", [{ id: "id-v1", version: "v1" }, { id: "id-v2", version: "v2" }]]]);
});

test("GuidesInstallDialog Escape means Later", async () => {
  const calls: unknown[] = [];
  await render([review({ use: "a" })], calls);
  now += ARMING_MS + 1;
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })));
  expect(calls).toEqual([["later", [{ id: "id-v1", version: "v1" }]]]);
});

test("GuidesInstallDialog ignores clicks while arming", async () => {
  const calls: unknown[] = [];
  const host = await render([review({ use: "a" })], calls);
  await click(button(host, "Approve guides"));
  expect(calls).toEqual([]);
});

test("GuidesInstallDialog Enter does not approve", async () => {
  const calls: unknown[] = [];
  const host = await render([review({ use: "a" })], calls);
  now += ARMING_MS + 1;
  await act(async () => void button(host, "Later").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
  expect(calls).toEqual([]);
});
