// The knowledge tab: its tree, file sub-tabs (kept across remounts), header, awaiting files and
// live re-reads.

import { afterAll, afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { resetMessages } from "../test/messages";
import type { Snapshot, TabApi } from "../api";
import type { Run } from "../GuidesSection";
import type { KnowledgeHeaderView, KnowledgeNode, KnowledgePage, KnowledgeRevision } from "./types";

const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { KnowledgeTab } = await import("./KnowledgeTab");
const { pruneSubTabs } = await import("./subTabs");
const { setMessages, t } = await import("../i18n");
const { readFileSync } = await import("node:fs");

afterAll(resetMessages);
const EN = new URL("../../../crates/oku-slides-i18n/src/locales/en/ui.ftl", import.meta.url);
setMessages("en", [readFileSync(EN, "utf8")]);

const node = (id: string, path: string, title: string | null, awaiting = false): KnowledgeNode => ({
  id, path, title: awaiting ? null : title, kind: awaiting ? null : "note", awaiting,
});
const HOME = node("home", "/index.md", "Home");
const A = node("a", "/a.md", "Alpha");
const B = node("b", "/b.md", "Beta");

const header = (extra: Partial<KnowledgeHeaderView> = {}): KnowledgeHeaderView => ({
  kind: null, title: null, description: null, status: null, stale: false, tags: [],
  generated_by: null, generated_at: null, verified: null, sources: [], ...extra,
});

/** A held reply: `go` answers it. */
interface Held { go: () => void }

/** The fake deck session: its tree and pages, what it was asked, and replies it may hold. */
interface Host {
  files: KnowledgeNode[];
  pages: Record<string, Partial<KnowledgePage>>;
  history: boolean;
  holdTree: boolean;
  holdPages: boolean;
  held: Held[];
  calls: string[];
  /** Pages refused with an error other than history. */
  refuse: Set<string>;
  /** The assets asked, as `id:asset`. */
  assets: string[];
}
function host(files: KnowledgeNode[] = [HOME, A, B], extra: Partial<Host> = {}): Host {
  return {
    files, pages: {}, history: false, holdTree: false, holdPages: false, held: [], calls: [], refuse: new Set(), assets: [],
    ...extra,
  };
}
function pageOf(h: Host, id: string): KnowledgePage {
  const n = h.files.find((f) => f.id === id);
  if (!n) throw new Error("unknown knowledge file");
  if (h.refuse.has(id)) throw new Error(`refused ${id}`);
  return {
    id, path: n.path, awaiting: n.awaiting, header: n.awaiting ? null : header({ title: n.title }),
    header_invalid: false, body: `body of ${n.path}`, truncated_bytes: 0, images: [], ...h.pages[id],
  };
}
/** `answer` now, or once released when `hold`. */
function reply<T>(h: Host, hold: boolean, answer: () => T): Promise<T> {
  if (h.history) return Promise.reject(new Error("in history"));
  if (!hold) {
    try {
      return Promise.resolve(answer());
    } catch (e) {
      return Promise.reject(e);
    }
  }
  return new Promise<T>((ok, ko) => h.held.push({ go: () => {
    try {
      ok(answer());
    } catch (e) {
      ko(e);
    }
  } }));
}
function apiOf(h: Host): TabApi {
  return {
    knowledgeTree: () => (h.calls.push("tree"), reply(h, h.holdTree, () => ({ files: [...h.files] }))),
    knowledgePage: (id: string) => (h.calls.push(`page:${id}`), reply(h, h.holdPages, () => pageOf(h, id))),
    knowledgeAsset: (id: string, asset: string) => (h.assets.push(`${id}:${asset}`), reply(h, false, () => {
      throw new Error("no asset");
    })),
    knowledgeReview: () => Promise.resolve({ files: [{ id: "n", path: "/n.md", title: null, bytes: 10 }], more: 0, over_limit: [] }),
    snapshot: () => Promise.resolve({ history: h.history ? {} : null } as unknown as Snapshot),
  } as unknown as TabApi;
}
const run: Run = (p) => p.catch(() => null);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  // No test's sub-tabs reach the next one.
  pruneSubTabs(new Set());
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const REV: KnowledgeRevision = { generation: 1, approvals: 0, in_history: false };
const rev = (generation: number, approvals = 0): KnowledgeRevision => ({ generation, approvals, in_history: false });
/** The knowledge tab's id. */
const TAB = "t-9";

async function render(h: Host, said: string[] = [], revision: KnowledgeRevision | null = REV, strict = false) {
  const el = document.createElement("div");
  document.body.append(el);
  const root = createRoot(el);
  mounted = { root, host: el };
  const api = apiOf(h);
  // The deck's history flag follows the host unless a test makes it lag behind.
  const props = (rev: KnowledgeRevision | null, deckInHistory: boolean) =>
    ({ tab: TAB, api, revision: rev, deckInHistory, say: (m: string) => void said.push(m), run });
  const tab = (rev: KnowledgeRevision | null, inHistory: boolean) => {
    const own = createElement(KnowledgeTab, props(rev, inHistory));
    return strict ? createElement(StrictMode, null, own) : own;
  };
  await act(async () => root.render(tab(revision, h.history)));
  const rerender = (rev: KnowledgeRevision | null, inHistory = h.history) =>
    act(async () => root.render(tab(rev, inHistory)));
  /** Unmounts the tab, as a tab switch does, and mounts it again. */
  const remount = async () => {
    await act(async () => root.render(createElement("div")));
    await act(async () => root.render(tab(revision, h.history)));
  };
  return { el, rerender, remount };
}
const click = (el: Element | undefined | null) =>
  act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
const treeButton = (el: HTMLElement, label: string) =>
  [...el.querySelectorAll<HTMLButtonElement>(".knowledge-tree button")].find((b) => b.textContent?.includes(label));
const subtabs = (el: HTMLElement) =>
  [...el.querySelectorAll<HTMLButtonElement>('[role="tab"]')].map((b) => [b.textContent, b.getAttribute("aria-selected")]);
const texts = (el: HTMLElement, selector: string) => [...el.querySelectorAll(selector)].map((e) => e.textContent);
/** Releases every held reply, in order, until none is left. */
async function releaseAll(h: Host) {
  while (h.held.length > 0) {
    const next = h.held.shift() as Held;
    await act(async () => next.go());
  }
}

describe("KnowledgeTab", () => {
  test("knowledge tab opens root index first", async () => {
    const h = host();
    const { el } = await render(h);
    expect(h.calls).toEqual(["tree", "page:home"]);
    expect(subtabs(el)).toEqual([["Home", "true"]]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
    expect(texts(el, ".knowledge-md p")).toEqual(["body of /index.md"]);
  });

  test("knowledge tab survives strict remount", async () => {
    const h = host();
    const { el, rerender } = await render(h, [], REV, true);
    expect(h.calls).toEqual(["tree", "page:home"]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
    await rerender(rev(2));
    expect(h.calls).toEqual(["tree", "page:home", "tree", "page:home"]);
  });

  test("knowledge tab without an approved root index opens nothing", async () => {
    const h = host([node("home", "/index.md", null, true), A]);
    const { el } = await render(h);
    expect(h.calls).toEqual(["tree"]);
    expect(subtabs(el)).toEqual([]);
  });

  test("knowledge tab empty tree says so", async () => {
    const { el } = await render(host([]));
    expect(texts(el, ".knowledge-empty")).toEqual([t("ui-knowledge-empty")]);
  });

  test("knowledge tab tree click opens subtab", async () => {
    const h = host();
    const { el } = await render(h);
    await click(treeButton(el, "Alpha"));
    expect(subtabs(el)).toEqual([["Home", "false"], ["Alpha", "true"]]);
    expect(h.calls).toEqual(["tree", "page:home", "page:a"]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Alpha"]);
    expect(treeButton(el, "Alpha")?.getAttribute("aria-current")).toBe("page");
  });

  test("knowledge tab second click focuses subtab", async () => {
    const h = host();
    const { el } = await render(h);
    await click(treeButton(el, "Alpha"));
    await click(treeButton(el, "Home"));
    expect(subtabs(el)).toEqual([["Home", "true"], ["Alpha", "false"]]);
    await click(treeButton(el, "Alpha"));
    expect(subtabs(el)).toEqual([["Home", "false"], ["Alpha", "true"]]);
    // Each page was read once: an open, current sub-tab is not read again.
    expect(h.calls).toEqual(["tree", "page:home", "page:a"]);
  });

  test("knowledge tab close button closes the subtab", async () => {
    const h = host();
    const { el } = await render(h);
    await click(treeButton(el, "Alpha"));
    const close = el.querySelectorAll<HTMLButtonElement>(".knowledge-subtab-close");
    expect(close[1]?.getAttribute("aria-label")).toBe(t("ui-knowledge-close-file"));
    await click(close[1]);
    expect(subtabs(el)).toEqual([["Home", "true"]]);
  });

  test("knowledge tab header shows fields", async () => {
    const h = host();
    h.pages.home = {
      header: header({
        title: "Home", description: "Start here", kind: "guide", status: "draft", stale: true, tags: ["x", "y"],
        generated_by: "agent", generated_at: "2026-11-27", verified: "human",
        sources: [{ resource: "res", title: "Report", url: "https://example.com" }],
      }),
    };
    const { el } = await render(h);
    expect(texts(el, ".knowledge-description")).toEqual(["Start here"]);
    expect(texts(el, ".knowledge-header dt")).toEqual([
      t("ui-knowledge-header-type"), t("ui-knowledge-header-status"), t("ui-knowledge-header-tags"),
      t("ui-knowledge-header-generated"), t("ui-knowledge-header-verified"), t("ui-knowledge-header-sources"),
    ]);
    expect(texts(el, ".knowledge-stale")).toEqual([t("ui-knowledge-header-stale")]);
    expect(texts(el, ".knowledge-tags li")).toEqual(["x", "y"]);
    expect(el.querySelector(".knowledge-header")?.textContent).toContain("agent");
    expect(el.querySelector(".knowledge-header")?.textContent).toContain("2026-11-27");
    expect(texts(el, ".knowledge-sources li")).toEqual(["Reportreshttps://example.com"]);
  });

  test("knowledge tab header url is text", async () => {
    const h = host();
    h.pages.home = { header: header({ title: "Home", sources: [{ resource: null, title: null, url: "javascript:alert(1)" }] }) };
    const { el } = await render(h);
    expect(el.querySelectorAll(".knowledge-header a")).toHaveLength(0);
    expect(texts(el, ".knowledge-sources li")).toEqual(["javascript:alert(1)"]);
  });

  test("knowledge tab header proto key harmless", async () => {
    const h = host();
    h.pages.home = { header: header({ title: "Home", tags: ["__proto__", "constructor", "__proto__"] }) };
    const { el } = await render(h);
    expect(texts(el, ".knowledge-tags li")).toEqual(["__proto__", "constructor", "__proto__"]);
  });

  test("knowledge tab untitled page shows its file name", async () => {
    const h = host([HOME, node("u", "/notes/u.md", null)]);
    h.pages.home = { header: null };
    const { el } = await render(h);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["index.md"]);
    await click(treeButton(el, "u.md"));
    expect(texts(el, ".knowledge-pane h1")).toEqual(["u.md"]);
  });

  test("knowledge tab awaiting badge and pre text", async () => {
    const h = host([HOME, node("n", "/n.md", null, true)]);
    h.pages.n = { body: "# Hi​<b>x</b>" };
    const { el } = await render(h);
    await click(treeButton(el, "n.md"));
    expect(texts(el, ".knowledge-pane .knowledge-awaiting-badge")).toEqual([t("ui-knowledge-awaiting-badge")]);
    expect(texts(el, ".knowledge-pane pre")).toEqual(["# Hi⟨U+200B⟩<b>x</b>"]);
    expect(el.querySelectorAll(".knowledge-pane .knowledge-md")).toHaveLength(0);
    expect(el.querySelectorAll(".knowledge-pane b")).toHaveLength(0);
  });

  test("knowledge tab awaiting path bidi stripped", async () => {
    const h = host([HOME, node("n", "/‮evil⁦.md", null, true)]);
    const { el } = await render(h);
    const row = el.querySelector(".knowledge-tree [data-awaiting] .knowledge-tree-name");
    expect(row?.textContent).toBe("evil.md");
    await click(row?.closest("button"));
    expect(texts(el, ".knowledge-pane h1")).toEqual(["/evil.md"]);
  });

  test("knowledge tab review opens dialog", async () => {
    const h = host([HOME, node("n", "/n.md", null, true)]);
    const { el } = await render(h);
    await click(treeButton(el, "n.md"));
    const review = [...el.querySelectorAll("button")].find((b) => b.textContent === t("ui-knowledge-review"));
    await click(review);
    expect(el.querySelectorAll('[role="dialog"]')).toHaveLength(1);
  });

  test("knowledge tab new revision rereads tree and active page only", async () => {
    const h = host();
    const { el, rerender } = await render(h);
    await click(treeButton(el, "Alpha"));
    h.calls.length = 0;
    await rerender(rev(2));
    expect(h.calls).toEqual(["tree", "page:a"]);
    // The other sub-tab went stale: it is read again once shown.
    await click(el.querySelectorAll<HTMLButtonElement>('[role="tab"]')[0]);
    expect(h.calls).toEqual(["tree", "page:a", "page:home"]);
    await rerender(rev(2, 1));
    expect(h.calls).toEqual(["tree", "page:a", "page:home", "tree", "page:home"]);
  });

  test("knowledge tab null revision rereads nothing", async () => {
    const h = host();
    const { el, rerender } = await render(h);
    h.calls.length = 0;
    await rerender(null);
    expect(h.calls).toEqual([]);
    await rerender(REV);
    expect(h.calls).toEqual([]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
  });

  test("knowledge tab rereads coalesce", async () => {
    const h = host();
    const { rerender } = await render(h);
    h.calls.length = 0;
    h.holdTree = true;
    for (let g = 2; g <= 6; g++) await rerender({ generation: g, approvals: 0 });
    expect(h.calls).toEqual(["tree"]);
    await releaseAll(h);
    expect(h.calls.filter((c) => c === "tree")).toHaveLength(2);
  });

  test("knowledge tab deleted file closes subtab one toast", async () => {
    const h = host();
    const said: string[] = [];
    const { el, rerender } = await render(h, said);
    h.holdPages = true;
    await click(treeButton(el, "Alpha"));
    h.files = [HOME, B];
    h.holdPages = false;
    await rerender(rev(2));
    expect(subtabs(el)).toEqual([["Home", "true"]]);
    expect(said).toEqual([t("ui-knowledge-file-gone", { path: "/a.md" })]);
    // The page asked before the delete answers late: ignored, no sub-tab comes back.
    h.files = [HOME, A, B];
    await releaseAll(h);
    h.files = [HOME, B];
    await rerender(rev(3));
    expect(subtabs(el)).toEqual([["Home", "true"]]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
    expect(said).toHaveLength(1);
  });

  test("knowledge tab moved file keeps subtab", async () => {
    const h = host();
    const { el, rerender } = await render(h);
    await click(treeButton(el, "Alpha"));
    h.files = [HOME, node("a", "/moved/a.md", "Alpha"), B];
    await rerender(rev(2));
    expect(subtabs(el)).toEqual([["Home", "false"], ["Alpha", "true"]]);
    expect(texts(el, ".knowledge-md p")).toEqual(["body of /moved/a.md"]);
  });

  test("knowledge tab in history shows notice no toast", async () => {
    const h = host();
    const said: string[] = [];
    const { el, rerender } = await render(h, said);
    h.history = true;
    await rerender(rev(2));
    expect(texts(el, ".knowledge-in-history")).toEqual([t("ui-knowledge-in-history")]);
    expect(el.querySelectorAll(".knowledge-pane")).toHaveLength(0);
    expect(said).toEqual([]);
    h.history = false;
    await rerender(rev(3));
    expect(el.querySelectorAll(".knowledge-in-history")).toHaveLength(0);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
  });

  test("knowledge tab opened in history recovers on leaving", async () => {
    const h = host([HOME, A, B], { history: true });
    const said: string[] = [];
    const { el, rerender } = await render(h, said);
    expect(texts(el, ".knowledge-in-history")).toEqual([t("ui-knowledge-in-history")]);
    expect(h.calls).toEqual([]);
    h.history = false;
    // Leaving history does not change the knowledge revision.
    await rerender(REV);
    expect(el.querySelectorAll(".knowledge-in-history")).toHaveLength(0);
    expect(el.querySelectorAll(".knowledge-tree")).toHaveLength(1);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Home"]);
    expect(said).toEqual([]);
  });

  test("knowledge tab refused tree read recovers on leaving history", async () => {
    const h = host();
    const said: string[] = [];
    const { el, rerender } = await render(h, said);
    await click(treeButton(el, "Alpha"));
    h.calls = [];
    // The session already browses its history; the tab's flag lags one render behind.
    h.history = true;
    await rerender(rev(2), false);
    expect(h.calls).toEqual(["tree"]);
    // No snapshot is read (it would take the deck's one-shot notice): the refusal that lands
    // before the view says history is told once.
    expect(said).toEqual(["in history"]);
    await rerender(rev(2));
    expect(texts(el, ".knowledge-in-history")).toEqual([t("ui-knowledge-in-history")]);
    h.history = false;
    h.files = [HOME, node("a", "/a.md", "Alpha 2"), B];
    await rerender(rev(2));
    expect(h.calls).toEqual(["tree", "tree", "page:a"]);
    expect(el.querySelectorAll(".knowledge-in-history")).toHaveLength(0);
    expect(subtabs(el)).toEqual([["Home", "false"], ["Alpha 2", "true"]]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Alpha 2"]);
    // The other open file went stale: shown again, it is read again.
    await click(el.querySelectorAll<HTMLButtonElement>('[role="tab"]')[0]);
    expect(h.calls).toEqual(["tree", "tree", "page:a", "page:home"]);
  });

  test("knowledge tab recreated file deleted again toasts again", async () => {
    const h = host();
    const said: string[] = [];
    const { el, rerender } = await render(h, said);
    await click(treeButton(el, "Alpha"));
    h.files = [HOME, B];
    await rerender(rev(2));
    h.files = [HOME, A, B];
    await rerender(rev(3));
    await click(treeButton(el, "Alpha"));
    h.files = [HOME, B];
    await rerender(rev(4));
    const gone = t("ui-knowledge-file-gone", { path: "/a.md" });
    expect(said).toEqual([gone, gone]);
  });

  test("knowledge tab subtab strip is labelled", async () => {
    const { el } = await render(host());
    const strip = el.querySelector('[role="tablist"]');
    expect(strip?.getAttribute("aria-label")).toBe(t("ui-knowledge-open-files"));
  });

  test("knowledge tab other read error is said", async () => {
    const h = host();
    const said: string[] = [];
    const { el } = await render(h, said);
    h.refuse.add("a");
    await click(treeButton(el, "Alpha"));
    expect(said).toEqual(["refused a"]);
    expect(el.querySelectorAll(".knowledge-in-history")).toHaveLength(0);
  });

  test("knowledge tab truncated notice", async () => {
    const h = host();
    h.pages.home = { truncated_bytes: 2048 };
    const { el } = await render(h);
    expect(texts(el, ".knowledge-truncated")).toEqual([t("ui-knowledge-truncated", { kb: 2 })]);
  });

  test("knowledge tab bundle link opens subtab", async () => {
    const h = host();
    h.pages.home = { body: "[go](b.md)" };
    const { el } = await render(h);
    await click(el.querySelector(".knowledge-md .knowledge-link"));
    expect(subtabs(el)).toEqual([["Home", "false"], ["Beta", "true"]]);
  });

  test("knowledge tab strip arrow keys rtl", async () => {
    const h = host();
    const { el } = await render(h);
    await click(treeButton(el, "Alpha"));
    await click(treeButton(el, "Beta"));
    await click(el.querySelectorAll<HTMLButtonElement>('[role="tab"]')[0]);
    const strip = el.querySelector<HTMLElement>('[role="tablist"]');
    const key = (k: string) =>
      act(async () => void strip?.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
    await key("ArrowLeft");
    expect(subtabs(el).map(([, s]) => s)).toEqual(["false", "false", "true"]);
    el.style.direction = "rtl";
    await key("ArrowLeft");
    expect(subtabs(el).map(([, s]) => s)).toEqual(["true", "false", "false"]);
    await key("ArrowLeft");
    expect(subtabs(el).map(([, s]) => s)).toEqual(["false", "true", "false"]);
    expect(document.activeElement?.textContent).toBe("Alpha");
  });

  test("knowledge tab remount keeps subtabs", async () => {
    const h = host();
    const { el, remount } = await render(h);
    await click(treeButton(el, "Alpha"));
    await click(treeButton(el, "Beta"));
    await click(el.querySelectorAll<HTMLButtonElement>('[role="tab"]')[1]);
    h.calls = [];
    await remount();
    expect(subtabs(el)).toEqual([["Home", "false"], ["Alpha", "true"], ["Beta", "false"]]);
    // The root page is not opened again; the shown file is read.
    expect(h.calls).toEqual(["tree", "page:a"]);
    expect(texts(el, ".knowledge-pane h1")).toEqual(["Alpha"]);
  });

  test("knowledge tab reply before close and reopen dropped", async () => {
    const h = host();
    const { el } = await render(h);
    h.holdPages = true;
    h.pages.a = { body: "old" };
    await click(treeButton(el, "Alpha"));
    const first = h.held.shift() as Held;
    await click(el.querySelectorAll<HTMLButtonElement>(".knowledge-subtab-close")[1]);
    h.pages.a = { body: "new" };
    await click(treeButton(el, "Alpha"));
    await releaseAll(h);
    // The reply asked before the close lands last: the reopened sub-tab keeps the new page.
    await act(async () => first.go());
    expect(texts(el, ".knowledge-md p")).toEqual(["new"]);
  });

  test("knowledge tab images not asked in history asked on leaving", async () => {
    const asset = `sha256:${"a".repeat(64)}`;
    const h = host();
    h.pages.home = { body: "![x](/img.png)", images: [{ path: "/img.png", asset }] };
    const { rerender } = await render(h);
    expect(h.assets).toEqual([`home:${asset}`]);
    h.history = true;
    await rerender(REV);
    h.history = false;
    await rerender(REV);
    expect(h.assets).toEqual([`home:${asset}`, `home:${asset}`]);
  });

  test("knowledge tab delete key closes subtab and focuses next", async () => {
    const h = host();
    const { el } = await render(h);
    await click(treeButton(el, "Alpha"));
    await click(treeButton(el, "Beta"));
    await click(el.querySelectorAll<HTMLButtonElement>('[role="tab"]')[1]);
    const strip = el.querySelector<HTMLElement>('[role="tablist"]');
    const key = (k: string) =>
      act(async () => void strip?.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
    await key("Delete");
    expect(subtabs(el)).toEqual([["Home", "false"], ["Beta", "true"]]);
    expect(document.activeElement?.textContent).toBe("Beta");
    await key("Backspace");
    expect(subtabs(el)).toEqual([["Home", "true"]]);
    expect(document.activeElement?.textContent).toBe("Home");
  });

  test("knowledge tab unreadable frontmatter raw text with notice", async () => {
    const h = host();
    h.pages.home = { header: null, header_invalid: true, body: "---\ntype: [\n---\n# Hi" };
    const { el } = await render(h);
    expect(texts(el, ".knowledge-header-invalid")).toEqual([t("ui-knowledge-header-invalid")]);
    expect(texts(el, ".knowledge-pane pre")).toEqual(["---\ntype: [\n---\n# Hi"]);
    expect(el.querySelectorAll(".knowledge-md")).toHaveLength(0);
  });

  test("knowledge tab bundle link to a long path opens it", async () => {
    const long = node("l", `/${"d".repeat(40)}/${"f".repeat(50)}.md`, "Long");
    const h = host([HOME, long]);
    h.pages.home = { body: `[go](${long.path})` };
    const { el } = await render(h);
    await click(el.querySelector(".knowledge-md .knowledge-link"));
    expect(subtabs(el)).toEqual([["Home", "false"], ["Long", "true"]]);
    expect(h.calls).toEqual(["tree", "page:home", "page:l"]);
  });
});
