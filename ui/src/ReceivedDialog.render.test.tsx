// The dialog validating what a received deck brings, rendered in a happy-dom DOM.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { Capabilities, CapabilityTotals, Guides, ReceivedReview, Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ReceivedDialog = (await import("./ReceivedDialog")).default;
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
const capabilities = (extra: Partial<Capabilities> = {}): Capabilities => ({
  kind: "widgets", name: "Brought", namespace: "brought", description: "", items: ["card", "quote"],
  requires: [], tokens: [], fonts: [], profiles: 0, code_bytes: 2048, ...extra,
});
/** Totals equal to the lists' lengths: nothing was cut. */
const uncut = (c: Capabilities): CapabilityTotals => ({
  items: c.items.length, requires: c.requires.length, tokens: c.tokens.length, fonts: c.fonts.length,
});
const review = (version = "v1", extra: Partial<ReceivedReview> = {}, guides: Guides = {}): ReceivedReview => {
  const caps = extra.capabilities ?? capabilities();
  return {
    id: `id-${version}`, version, capabilities: caps, totals: uncut(caps), relation: "new_pack", install: true,
    blocker: null, guides, ...extra,
  };
};

function spyApi(calls: unknown[]): TabApi {
  const record = (name: string) => (versions: unknown) => (calls.push([name, versions]), Promise.resolve({} as Snapshot));
  return { acceptReceived: record("accept"), laterReceived: record("later") } as unknown as TabApi;
}

async function render(reviews: ReceivedReview[], calls: unknown[] = [], api: TabApi = spyApi(calls)) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(ReceivedDialog, { api, reviews, run })));
  return host;
}
const rerender = (reviews: ReceivedReview[], api: TabApi) =>
  act(async () => mounted?.root.render(createElement(ReceivedDialog, { api, reviews, run })));
const button = (host: HTMLElement, name: string) =>
  [...host.querySelectorAll("button")].find((b) => b.textContent === name) as HTMLButtonElement;
const click = (el: HTMLElement) => act(async () => void el.dispatchEvent(new MouseEvent("click", { bubbles: true })));
const texts = (host: HTMLElement, selector: string) => [...host.querySelectorAll(selector)].map((e) => e.textContent);

test("ReceivedDialog shows each guide as plain text", async () => {
  const host = await render([review("v1", {}, { use: "<b>bold</b>", edit: "[link](https://x.test)" })]);
  expect(texts(host, "pre")).toEqual(["[link](https://x.test)", "<b>bold</b>"]);
  expect(host.querySelector("pre b, pre a")).toBeNull();
});

test("ReceivedDialog describes what each version holds", async () => {
  const caps = capabilities({ requires: ["a", "b"], tokens: ["card.fill", "card.ink"], fonts: ["Inter", "Lora"], code_bytes: 3072 });
  const host = await render([review("v1", { capabilities: caps })]);
  expect(host.querySelector(".received-name")?.textContent).toBe("Brought");
  expect(host.querySelector(".received-namespace")?.textContent).toBe("Namespace: \u2068brought\u2069");
  expect(host.querySelector(".received-kind")?.textContent).toBe("\u2068Widgets\u2069 · \u2068New pack\u2069");
  expect(texts(host, ".received-field")).toEqual([
    "Contents: \u2068\u2068card\u2069, \u2068quote\u2069\u2069",
    "Requires: \u2068\u2068a\u2069, \u2068b\u2069\u2069",
    "Tokens: \u2068\u2068card.fill\u2069, \u2068card.ink\u2069\u2069",
    "Fonts: \u2068\u2068Inter\u2069, \u2068Lora\u2069\u2069",
    "Typst code: 3 KB",
  ]);
});

test("ReceivedDialog names a new version of a held pack", async () => {
  const host = await render([review("v1", { relation: "new_version", capabilities: capabilities({ kind: "theme", items: [], code_bytes: 0 }) })]);
  expect(host.querySelector(".received-kind")?.textContent).toBe("\u2068Theme\u2069 · \u2068New version\u2069");
  expect(texts(host, ".received-field")).toEqual([]);
});

test("ReceivedDialog counts the layout profiles of a layouts pack", async () => {
  const host = await render([review("v1", { capabilities: capabilities({ kind: "layouts", items: ["cover"], profiles: 2, code_bytes: 1536 }) })]);
  expect(texts(host, ".received-field")).toEqual(["Contents: \u2068cover\u2069", "2 layout profiles", "Typst code: 2 KB"]);
});

test("ReceivedDialog shows the description as plain text", async () => {
  const host = await render([review("v1", { capabilities: capabilities({ description: "<i>Cards</i>" }) })]);
  expect(host.querySelector(".received-description")?.textContent).toBe("<i>Cards</i>");
  expect(host.querySelector(".received-description i")).toBeNull();
});

test("ReceivedDialog marks the versions it installs", async () => {
  const host = await render([review("v1")]);
  expect(texts(host, ".received-install")).toEqual(["Will be added to your library"]);
});

test("ReceivedDialog shows a blocker without an install mark", async () => {
  const blocker = "Stays in this deck: a pack of this name is yours.";
  const host = await render([review("v1", { install: false, blocker })]);
  expect(texts(host, ".received-blocker")).toEqual([blocker]);
  expect(host.querySelector(".received-install")).toBeNull();
});

test("ReceivedDialog states that packs stay offline", async () => {
  const host = await render([review("v1")]);
  expect(host.querySelector(".received-offline")?.textContent).toBe("Packs and themes cannot read your files or the network.");
});

test("ReceivedDialog offers to add when a version installs", async () => {
  const host = await render([review("v1", { install: false, guides: { use: "u" } }), review("v2")]);
  expect(button(host, "Add to my library")).toBeDefined();
  expect(button(host, "Approve guides")).toBeUndefined();
});

test("ReceivedDialog offers to approve guides when nothing installs", async () => {
  const host = await render([review("v1", { install: false, relation: "held" }, { use: "u" })]);
  expect(button(host, "Approve guides")).toBeDefined();
  expect(button(host, "Add to my library")).toBeUndefined();
});

test("ReceivedDialog focuses Later by default", async () => {
  const host = await render([review("v1")]);
  expect(document.activeElement).toBe(button(host, "Later"));
});

test("ReceivedDialog accept sends every listed version", async () => {
  const calls: unknown[] = [];
  const host = await render([review("v1"), review("v2", { install: false, blocker: "Stays in this deck: no." })], calls);
  now += ARMING_MS + 1;
  await click(button(host, "Add to my library"));
  expect(calls).toEqual([["accept", [{ id: "id-v1", version: "v1" }, { id: "id-v2", version: "v2" }]]]);
});

test("ReceivedDialog Later sends every listed version", async () => {
  const calls: unknown[] = [];
  const host = await render([review("v1"), review("v2")], calls);
  now += ARMING_MS + 1;
  await click(button(host, "Later"));
  expect(calls).toEqual([["later", [{ id: "id-v1", version: "v1" }, { id: "id-v2", version: "v2" }]]]);
});

test("ReceivedDialog Escape means Later", async () => {
  const calls: unknown[] = [];
  await render([review("v1")], calls);
  now += ARMING_MS + 1;
  await act(async () => void window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })));
  expect(calls).toEqual([["later", [{ id: "id-v1", version: "v1" }]]]);
});

test("ReceivedDialog ignores clicks while arming", async () => {
  const calls: unknown[] = [];
  const host = await render([review("v1")], calls);
  await click(button(host, "Add to my library"));
  expect(calls).toEqual([]);
});

test("ReceivedDialog Enter does not accept", async () => {
  const calls: unknown[] = [];
  const host = await render([review("v1")], calls);
  now += ARMING_MS + 1;
  await act(async () => void button(host, "Later").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
  expect(calls).toEqual([]);
});

test("ReceivedDialog with a cut list says how many names are left out", async () => {
  const caps = capabilities();
  const host = await render([review("v1", { capabilities: caps, totals: { ...uncut(caps), items: 52 } })]);
  expect(texts(host, ".received-field")[0]).toBe("Contents: \u2068\u2068card\u2069, \u2068quote\u2069 and 50 more\u2069");
});

test("ReceivedDialog rounds the code size up to whole KB", async () => {
  const host = await render([review("v1", { capabilities: capabilities({ code_bytes: 1 }) })]);
  expect(texts(host, ".received-field")).toEqual(["Contents: \u2068\u2068card\u2069, \u2068quote\u2069\u2069", "Typst code: 1 KB"]);
});

test("ReceivedDialog tells apart two packs of the same name by namespace", async () => {
  const host = await render([
    review("v1", { capabilities: capabilities({ namespace: "acme" }) }),
    review("v2", { capabilities: capabilities({ namespace: "other" }) }),
  ]);
  expect(texts(host, ".received-namespace")).toEqual(["Namespace: \u2068acme\u2069", "Namespace: \u2068other\u2069"]);
});

test("ReceivedDialog describes the dialog by its intro", async () => {
  const host = await render([review("v1")]);
  const id = host.querySelector("[role=alertdialog]")?.getAttribute("aria-describedby");
  expect(id).toBeTruthy();
  expect(document.getElementById(id ?? "")?.textContent).toBe(host.querySelector(".confirm-message")?.textContent);
});

test("ReceivedDialog offers a plain OK when every version is blocked", async () => {
  const host = await render([review("v1", { install: false, blocker: "Stays in this deck: no." })]);
  expect(button(host, "OK")).toBeDefined();
  expect(button(host, "Approve guides")).toBeUndefined();
  expect(button(host, "Add to my library")).toBeUndefined();
});

test("ReceivedDialog OK on blocked versions sends every listed version", async () => {
  const calls: unknown[] = [];
  const blocked = { install: false, blocker: "Stays in this deck: no." };
  const host = await render([review("v1", blocked), review("v2", blocked)], calls);
  now += ARMING_MS + 1;
  await click(button(host, "OK"));
  expect(calls).toEqual([["accept", [{ id: "id-v1", version: "v1" }, { id: "id-v2", version: "v2" }]]]);
});

test("ReceivedDialog sends nothing on a second click while the first is pending", async () => {
  const calls: unknown[] = [];
  const pending = (name: string) => (versions: unknown) => (calls.push([name, versions]), new Promise<Snapshot>(() => {}));
  const api = { acceptReceived: pending("accept"), laterReceived: pending("later") } as unknown as TabApi;
  const host = await render([review("v1")], calls, api);
  now += ARMING_MS + 1;
  await click(button(host, "Add to my library"));
  await click(button(host, "Add to my library"));
  await click(button(host, "Later"));
  expect(calls).toEqual([["accept", [{ id: "id-v1", version: "v1" }]]]);
});

test("ReceivedDialog arms again when another version joins the list", async () => {
  const calls: unknown[] = [];
  const api = spyApi(calls);
  await render([review("v1")], calls, api);
  now += ARMING_MS + 1;
  await rerender([review("v1"), review("v2")], api);
  await click(button(mounted?.host as HTMLElement, "Add to my library"));
  expect(calls).toEqual([]);
});

test("ReceivedDialog focuses Later again when the list changes", async () => {
  const api = spyApi([]);
  await render([review("v1")], [], api);
  await rerender([review("v1"), review("v2")], api);
  expect(document.activeElement).toBe(button(mounted?.host as HTMLElement, "Later"));
});
