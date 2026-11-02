// The Guides section of a pack or theme tab, rendered in a happy-dom DOM.

import { afterAll, afterEach, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { GuidesView, Guides, Snapshot, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const GuidesSection = (await import("./GuidesSection")).default;
const { setMessages } = await import("./i18n");
const { readFileSync } = await import("node:fs");

afterAll(resetMessages);
setMessages("en", [readFileSync(new URL("../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url), "utf8")]);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const run = (p: Promise<Snapshot>) => p.then(() => null);
const view = (guides: Guides, origin: GuidesView["origin"] = "local", approved = true, approvable?: boolean, kept_awaiting = false): GuidesView =>
  ({ guides, max_chars: 100, id: "id-1", version: "v1", origin, approved, approvable: approvable ?? (origin === "received" && !approved), kept_awaiting });

/** The "Approve this guide" button, if shown. */
const approveButton = (host: HTMLElement) =>
  [...host.querySelectorAll("button")].find((b) => b.textContent === "Approve this guide") as HTMLButtonElement | undefined;

/** An API that records `setGuide` and `approveGuides` calls. */
function spyApi(calls: unknown[]): TabApi {
  return {
    setGuide: (which: string, text: string) => (calls.push([which, text]), Promise.resolve({} as Snapshot)),
    approveGuides: (versions: unknown) => (calls.push(["approve", versions]), Promise.resolve({} as Snapshot)),
  } as unknown as TabApi;
}

async function render(api: TabApi, v: GuidesView) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(GuidesSection, { api, view: v, run })));
  return host;
}

/** The textarea whose label starts with `label`. */
const field = (host: HTMLElement, label: string) =>
  [...host.querySelectorAll("label")].find((l) => l.textContent?.startsWith(label))?.querySelector("textarea") as HTMLTextAreaElement;

/** Types into a controlled field the way a user does. */
async function type(el: HTMLTextAreaElement, value: string) {
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
    set?.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
const blur = (el: HTMLElement) => act(async () => void el.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));

test("GuidesSection shows both guides and the origin badge", async () => {
  const host = await render(spyApi([]), view({ edit: "e", use: "u" }));
  expect([field(host, "How to edit").value, field(host, "How to use").value]).toEqual(["e", "u"]);
  expect(host.querySelector(".guides-badge")?.textContent).toBe("Yours");
});

test("GuidesSection counts the characters against the cap", async () => {
  const host = await render(spyApi([]), view({ use: "abc" }));
  expect(field(host, "How to use").closest("label")?.textContent).toContain("3 / 100");
});

test("GuidesSection saves a changed guide on blur", async () => {
  const calls: unknown[] = [];
  const host = await render(spyApi(calls), view({ use: "u" }));
  const use = field(host, "How to use");
  await type(use, "u2");
  await blur(use);
  expect(calls).toEqual([["use", "u2"]]);
});

test("GuidesSection does not save an unchanged guide", async () => {
  const calls: unknown[] = [];
  const host = await render(spyApi(calls), view({ use: "u" }));
  await blur(field(host, "How to use"));
  expect(calls).toEqual([]);
});

test("GuidesSection on a received unapproved version offers approval", async () => {
  const calls: unknown[] = [];
  const host = await render(spyApi(calls), view({ use: "u" }, "received", false));
  const button = [...host.querySelectorAll("button")].find((b) => b.textContent === "Approve this guide");
  expect(button).toBeTruthy();
  await act(async () => void button?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  expect(calls).toEqual([["approve", [{ id: "id-1", version: "v1" }]]]);
});

test("GuidesSection offers no approval for a local or approved version", async () => {
  const local = await render(spyApi([]), view({ use: "u" }));
  expect([...local.querySelectorAll("button")]).toEqual([]);
  const approved = await render(spyApi([]), view({ use: "u" }, "received", true));
  expect([...approved.querySelectorAll("button")]).toEqual([]);
});

test("GuidesSection keeps its keystrokes from the app's shortcuts", async () => {
  const host = await render(spyApi([]), view({ use: "u" }));
  let reached = false;
  document.addEventListener("keydown", () => (reached = true), { once: true });
  await act(async () => void field(host, "How to use").dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true })));
  expect(reached).toBe(false);
});

test("GuidesSection turns Approve off while a field holds unsaved text", async () => {
  const host = await render(spyApi([]), view({ use: "u" }, "received", false));
  expect(approveButton(host)?.disabled).toBe(false);
  await type(field(host, "How to use"), "u2");
  expect(approveButton(host)?.disabled).toBe(true);
  await type(field(host, "How to use"), "u");
  expect(approveButton(host)?.disabled).toBe(false);
});

test("GuidesSection shows no Approve when the session says the shown text is not the approved one", async () => {
  const host = await render(spyApi([]), view({ use: "u" }, "received", false, false));
  expect(host.textContent).toContain("Not sent to the agent until you approve it.");
  expect(approveButton(host)).toBeUndefined();
});

test("GuidesSection awaiting with a kept guide says how to approve", async () => {
  const host = await render(spyApi([]), view({ edit: "mine", use: "kept" }, "received", false, false, true));
  expect(host.querySelector(".guides-approval")?.textContent).toContain("undo your edits");
});

test("GuidesSection awaiting without a kept guide shows no reason", async () => {
  const host = await render(spyApi([]), view({ use: "kept" }, "received", false, false));
  expect(host.querySelector(".guides-approval")?.textContent).not.toContain("undo your edits");
});

test("GuidesSection awaiting and approvable shows no reason", async () => {
  const host = await render(spyApi([]), view({ use: "kept" }, "received", false));
  expect(host.querySelector(".guides-approval")?.textContent).not.toContain("undo your edits");
});
