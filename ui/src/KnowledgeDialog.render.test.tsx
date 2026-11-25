// The dialog approving a deck's received knowledge, rendered in a happy-dom DOM.

import { afterAll, afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { KnowledgeApproval, KnowledgeReview, KnowledgeText, Snapshot, TabApi } from "./api";
import type { Run } from "./GuidesSection";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const KnowledgeDialog = (await import("./KnowledgeDialog")).default;
const { ARMING_MS } = await import("./modalFrame");
const { setMessages } = await import("./i18n");
const { readFileSync } = await import("node:fs");

afterAll(resetMessages);
const EN = new URL("../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url);
setMessages("en", [readFileSync(EN, "utf8")]);

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

const file = (id: string, extra: Partial<KnowledgeReview["files"][number]> = {}) => ({
  id, path: `notes/${id}.md`, title: null, bytes: 1500, ...extra,
});
const review = (files = [file("a")], extra: Partial<KnowledgeReview> = {}): KnowledgeReview => ({
  files, more: 0, over_limit: [], ...extra,
});
const text = (id: string, extra: Partial<KnowledgeText> = {}): KnowledgeText => ({
  id, path: `notes/${id}.md`, hash: `sha256:${id}`, text: `text of ${id}`, truncated: 0, ...extra,
});

/** What the fake host answers, and what it was asked. */
interface Host {
  reviews: KnowledgeReview[];
  texts: (id: string) => Promise<KnowledgeText>;
  approve: (files: KnowledgeApproval[]) => Promise<Snapshot>;
  calls: unknown[];
}
function host(extra: Partial<Host> = {}): Host {
  return {
    reviews: [review(), review([])],
    texts: (id) => Promise.resolve(text(id)),
    approve: () => Promise.resolve({} as Snapshot),
    calls: [],
    ...extra,
  };
}
function apiOf(h: Host): TabApi {
  return {
    knowledgeReview: () => {
      h.calls.push(["review"]);
      // Each review in turn; the last one again.
      const next = h.reviews.length > 1 ? (h.reviews.shift() as KnowledgeReview) : h.reviews[0];
      return Promise.resolve(next);
    },
    knowledgeText: (id: string) => (h.calls.push(["text", id]), h.texts(id)),
    approveKnowledge: (files: KnowledgeApproval[]) => {
      h.calls.push(["approve", files]);
      return h.approve(files);
    },
  } as unknown as TabApi;
}

/** `run` as the tab has it: a refusal is said, then answered `null`. */
function runSaying(said: string[]): Run {
  return (p) => p.catch((e: unknown) => {
    said.push(e instanceof Error ? e.message : String(e));
    return null;
  });
}

async function render(
  h: Host, closes: string[] = [], said: string[] = [], api = apiOf(h), run = runSaying(said),
) {
  const el = document.createElement("div");
  document.body.append(el);
  const root = createRoot(el);
  mounted = { root, host: el };
  const props = {
    api, run, onError: (m: string) => void said.push(m),
    onClose: () => void closes.push("close"),
  };
  await act(async () => root.render(createElement(KnowledgeDialog, props)));
  now += ARMING_MS + 1;
  return el;
}
const button = (el: HTMLElement, name: string) =>
  [...el.querySelectorAll("button")].find((b) => b.textContent === name);
const click = (el: HTMLElement | undefined) =>
  act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
const texts = (el: HTMLElement, selector: string) =>
  [...el.querySelectorAll(selector)].map((e) => e.textContent);
const calls = (h: Host, kind: string) => h.calls.filter((c) => (c as string[])[0] === kind);

test("KnowledgeDialog lists each file with its path, title and size", async () => {
  const pricing = file("a", { title: "Pricing", bytes: 1025 });
  const el = await render(host({ reviews: [review([pricing])] }));
  expect(texts(el, ".knowledge-path")).toEqual(["notes/a.md"]);
  expect(texts(el, ".knowledge-title")).toEqual(["Pricing"]);
  expect(texts(el, ".knowledge-size")).toEqual(["2 KB"]);
});

test("KnowledgeDialog reveals hidden characters in a title", async () => {
  const el = await render(host({ reviews: [review([file("a", { title: "Pri\u200Bcing" })])] }));
  expect(texts(el, ".knowledge-title")).toEqual(["Pri⟨U+200B⟩cing"]);
});

test("KnowledgeDialog while a text loads points aria-controls at a present panel", async () => {
  const el = await render(host({ texts: () => new Promise<KnowledgeText>(() => {}) }));
  await click(button(el, "Show"));
  const id = button(el, "Hide")?.getAttribute("aria-controls");
  expect(id).toBeTruthy();
  expect(el.querySelector(`[id="${id}"]`)?.textContent).toBe("Loading…");
});

test("KnowledgeDialog before Show offers no Approve", async () => {
  const el = await render(host());
  expect(button(el, "Show")).toBeDefined();
  expect(button(el, "Approve")).toBeUndefined();
});

test("KnowledgeDialog Show renders markup literally", async () => {
  const body = "<script>alert(1)</script>\n**bold** <b>b</b>";
  const el = await render(host({ texts: (id) => Promise.resolve(text(id, { text: body })) }));
  await click(button(el, "Show"));
  expect(texts(el, "pre")).toEqual([body]);
  expect(el.querySelector("pre script, pre b, pre strong")).toBeNull();
  expect(el.querySelector("pre")?.getAttribute("dir")).toBe("auto");
  expect(button(el, "Hide")?.getAttribute("aria-expanded")).toBe("true");
});

test("KnowledgeDialog Show reveals hidden characters and counts them", async () => {
  const hidden = "a\u200Bb\u202E";
  const el = await render(host({ texts: (id) => Promise.resolve(text(id, { text: hidden })) }));
  await click(button(el, "Show"));
  expect(texts(el, "pre")).toEqual(["a⟨U+200B⟩b⟨U+202E⟩"]);
  expect(texts(el, ".knowledge-hidden")).toEqual(["2 hidden characters are shown as ⟨U+…⟩."]);
});

test("KnowledgeDialog Approve sends the shown hash once", async () => {
  const h = host({ texts: (id) => Promise.resolve(text(id, { hash: "sha256:shown" })) });
  const el = await render(h);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  expect(calls(h, "approve")).toEqual([["approve", [{ id: "a", hash: "sha256:shown" }]]]);
});

test("KnowledgeDialog accepted approval fetches the list once more", async () => {
  const h = host();
  const el = await render(h);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  expect(calls(h, "review")).toHaveLength(2);
});

test("KnowledgeDialog Escape during an approval keeps it open", async () => {
  const closes: string[] = [];
  const el = await render(host({ approve: () => new Promise<Snapshot>(() => {}) }), closes);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  const escape = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
  await act(async () => void window.dispatchEvent(escape));
  expect(closes).toEqual([]);
});

test("KnowledgeDialog a list reply after unmount is ignored", async () => {
  let answer: (r: KnowledgeReview) => void = () => {};
  const closes: string[] = [];
  const api = apiOf(host());
  api.knowledgeReview = () => new Promise((r) => (answer = r));
  await render(host(), closes, [], api);
  const m = mounted;
  mounted = null;
  await act(async () => m?.root.unmount());
  m?.host.remove();
  await act(async () => answer(review([])));
  expect(closes).toEqual([]);
});

test("KnowledgeDialog second click while busy sends nothing", async () => {
  const h = host({ approve: () => new Promise<Snapshot>(() => {}) });
  const el = await render(h);
  await click(button(el, "Show"));
  const approve = button(el, "Approve");
  await click(approve);
  await click(approve);
  expect(calls(h, "approve")).toHaveLength(1);
  expect(button(el, "Close")?.disabled).toBe(true);
});

test("KnowledgeDialog truncated row has no Approve", async () => {
  const el = await render(host({ texts: (id) => Promise.resolve(text(id, { truncated: 10 })) }));
  await click(button(el, "Show"));
  expect(button(el, "Approve")).toBeUndefined();
  const tooLarge = "This file is too large to approve: delete it.";
  expect(texts(el, ".knowledge-too-large")).toEqual([tooLarge]);
});

test("KnowledgeDialog refusal shows the message and fetches the list once more", async () => {
  const refusal = "This knowledge file changed since it was shown: review it again.";
  const said: string[] = [];
  const h = host({ reviews: [review()], approve: () => Promise.reject(new Error(refusal)) });
  const el = await render(h, [], said);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  expect(said).toEqual([refusal]);
  expect(calls(h, "review")).toHaveLength(2);
});

test("KnowledgeDialog a stale text reply is ignored", async () => {
  let answerA: (t: KnowledgeText) => void = () => {};
  const h = host({
    reviews: [review([file("a"), file("b")])],
    texts: (id) => (id === "a" ? new Promise((r) => (answerA = r)) : Promise.resolve(text(id))),
  });
  const el = await render(h);
  const [showA, showB] = [...el.querySelectorAll("button")].filter((b) => b.textContent === "Show");
  await click(showA);
  await click(showB);
  await act(async () => answerA(text("a")));
  expect(texts(el, "pre")).toEqual(["text of b"]);
  expect(texts(el, "[aria-expanded=true]")).toEqual(["Hide"]);
});

test("KnowledgeDialog empty list closes", async () => {
  const closes: string[] = [];
  await render(host({ reviews: [review([])] }), closes);
  expect(closes).toEqual(["close"]);
});

test("KnowledgeDialog empty list over limit stays open with the limits", async () => {
  const closes: string[] = [];
  const overLimit = review([], { over_limit: ["file-too-large"] });
  const el = await render(host({ reviews: [overLimit] }), closes);
  expect(closes).toEqual([]);
  expect(texts(el, ".knowledge-over-limit")).toHaveLength(1);
});

test("KnowledgeDialog says how many files are not listed", async () => {
  const el = await render(host({ reviews: [review([file("a")], { more: 4 })] }));
  expect(texts(el, ".knowledge-more")).toEqual(["4 more files are not listed."]);
});

test("KnowledgeDialog focuses Close first", async () => {
  const el = await render(host());
  expect(document.activeElement).toBe(button(el, "Close") as HTMLButtonElement);
});

test("KnowledgeDialog Escape closes", async () => {
  const closes: string[] = [];
  await render(host(), closes);
  const escape = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
  await act(async () => void window.dispatchEvent(escape));
  expect(closes).toEqual(["close"]);
});

test("KnowledgeDialog a run that throws says why and frees the buttons", async () => {
  const said: string[] = [];
  const h = host({ approve: () => Promise.reject(new Error("lost")) });
  const throwing: Run = (p) => p;
  const el = await render(h, [], said, apiOf(h), throwing);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  expect(said).toEqual(["lost"]);
  expect(button(el, "Close")?.disabled).toBe(false);
});

test("KnowledgeDialog an approval that empties the list closes and frees nothing", async () => {
  const closes: string[] = [];
  const el = await render(host({ reviews: [review(), review([])] }), closes);
  await click(button(el, "Show"));
  await click(button(el, "Approve"));
  expect(closes).toEqual(["close"]);
  expect(button(el, "Close")?.disabled).toBe(true);
});
