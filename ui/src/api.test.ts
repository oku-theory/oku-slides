import { afterEach, expect, test } from "bun:test";
import { tabApi } from "./api";

const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});

/** The request body `send` posts to the host, answering `reply`. */
async function sent(send: () => Promise<unknown>, reply: unknown = {}): Promise<unknown> {
  let body = "";
  globalThis.fetch = (async (_url: string, init?: RequestInit) => {
    body = String(init?.body);
    return new Response(JSON.stringify(reply), { status: 200 });
  }) as unknown as typeof fetch;
  await send();
  return JSON.parse(body);
}

test("knowledgeTree asks the tab for the knowledge tree", async () => {
  expect(await sent(() => tabApi("t-1").knowledgeTree())).toEqual({ tab: "t-1", req: { cmd: "knowledge_tree" } });
});

test("knowledgePage asks the tab for one page by file id", async () => {
  expect(await sent(() => tabApi("t-2").knowledgePage("ab"))).toEqual({
    tab: "t-2",
    req: { cmd: "knowledge_page", id: "ab" },
  });
});

test("knowledgeAsset asks the tab for one image of a file", async () => {
  expect(await sent(() => tabApi("t-1").knowledgeAsset("ab", "sha256:cd"))).toEqual({
    tab: "t-1",
    req: { cmd: "knowledge_asset", id: "ab", asset: "sha256:cd" },
  });
});

test("knowledgeAsset answers the host's mime and data unchanged", async () => {
  const reply = { mime: "image/png", data: "AAAA" };
  globalThis.fetch = (async () => new Response(JSON.stringify(reply), { status: 200 })) as unknown as typeof fetch;
  expect(await tabApi("t-1").knowledgeAsset("ab", "sha256:cd")).toEqual(reply);
});
