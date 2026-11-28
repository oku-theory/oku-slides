// The knowledge view: history comes from the owner's revision, never from a snapshot read.

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import "../test/dom";
import type { KnowledgeRevision } from "./types";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { KnowledgeView } = await import("./KnowledgeView");
const { pruneSubTabs } = await import("./subTabs");
const { t } = await import("../i18n");

/** The commands the host was asked, in order. */
let asked: string[] = [];
const realFetch = globalThis.fetch;
beforeAll(() => {
  globalThis.fetch = (async (_url: string, init?: { body?: string }) => {
    const cmd = (JSON.parse(init?.body ?? "{}") as { req?: { cmd?: string } }).req?.cmd ?? "";
    asked.push(cmd);
    return new Response(JSON.stringify(cmd === "knowledge_tree" ? { files: [] } : null), { status: 200 });
  }) as typeof fetch;
});
afterAll(() => {
  globalThis.fetch = realFetch;
});

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  pruneSubTabs(new Set());
  asked = [];
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function render(revision: KnowledgeRevision): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(KnowledgeView, { tab: "k-1", owner: "d-1", revision })));
  return host;
}

describe("KnowledgeView", () => {
  test("knowledge view revision in history shows notice without snapshot", async () => {
    const el = await render({ generation: 1, approvals: 0, in_history: true });
    expect(el.querySelector(".knowledge-in-history")?.textContent).toBe(t("ui-knowledge-in-history"));
    expect(asked).not.toContain("snapshot");
  });

  test("knowledge view revision live reads tree without snapshot", async () => {
    const el = await render({ generation: 1, approvals: 0, in_history: false });
    expect(el.querySelector(".knowledge-in-history")).toBeNull();
    expect(asked).toContain("knowledge_tree");
    expect(asked).not.toContain("snapshot");
  });
});
