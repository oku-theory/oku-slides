// A knowledge page's images as `blob:` URLs: fetched per asset, four at most at once, landing
// together, revoked on cleanup.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import "../test/dom";
import type { TabApi } from "../api";
import type { KnowledgeAsset, KnowledgePage } from "./types";

const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useKnowledgeAssets } = await import("./useKnowledgeAssets");
const { deferred } = await import("../test/widgetApi");

const ASSET = `sha256:${"a".repeat(64)}`;
/** A one-pixel PNG's signature, base64. */
const PNG = btoa("\x89PNG\r\n\x1a\n");

const page = (extra: Partial<KnowledgePage> = {}): KnowledgePage => ({
  id: "f1", path: "/a.md", awaiting: false, header: null, body: "![x](img.png)", truncated_bytes: 0,
  header_invalid: false, images: [{ path: "/img.png", asset: ASSET }], ...extra,
});

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** An api whose `knowledgeAsset` waits for `release(i, …)`. */
function heldApi() {
  const calls: { id: string; asset: string; done: ReturnType<typeof deferred<KnowledgeAsset>> }[] = [];
  const api = {
    knowledgeAsset: (id: string, asset: string) => {
      const done = deferred<KnowledgeAsset>();
      calls.push({ id, asset, done });
      return done.promise;
    },
  } as unknown as TabApi;
  const release = (i: number, reply: KnowledgeAsset) => act(async () => calls[i].done.resolve(reply));
  const refuse = (i: number) => act(async () => calls[i].done.reject(new Error("refused")));
  return { api, calls, release, refuse };
}

type Props = { page: KnowledgePage | null };
/** Mounts the hook; `out.image` is the last render's lookup. */
async function mount(api: TabApi, props: Props, strict = false) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { image: ((path: string) => string | null) | null } = { image: null };
  function Probe(p: Props) {
    out.image = useKnowledgeAssets(api, p.page);
    return null;
  }
  const render = (p: Props) => act(async () => {
    const probe = createElement(Probe, p);
    root.render(strict ? createElement(StrictMode, null, probe) : probe);
  });
  await render(props);
  return { out, render, root };
}

describe("useKnowledgeAssets", () => {
  test("useKnowledgeAssets asset image survives strict remount", async () => {
    const { api, calls, release } = heldApi();
    const { out } = await mount(api, { page: page() }, true);
    expect(out.image!("/img.png")).toBeNull();
    // StrictMode ran the effect twice: the first run's reply is dropped, the second one shows.
    for (let i = 0; i < calls.length; i++) await release(i, { mime: "image/png", data: PNG });
    expect(calls.map((c) => [c.id, c.asset]).at(-1)).toEqual(["f1", ASSET]);
    expect(out.image!("/img.png")).toStartWith("blob:");
    expect(out.image!("/other.png")).toBeNull();
  });

  test("useKnowledgeAssets revokes its urls on cleanup", async () => {
    const { api, release } = heldApi();
    const revoke = spyOn(URL, "revokeObjectURL");
    try {
      const { out, root } = await mount(api, { page: page() });
      await release(0, { mime: "image/png", data: PNG });
      const url = out.image!("/img.png");
      expect(url).toStartWith("blob:");
      await act(async () => root.unmount());
      mounted = null;
      expect(revoke).toHaveBeenCalledWith(url);
    } finally {
      revoke.mockRestore();
    }
  });

  test("useKnowledgeAssets late reply after unmount dropped", async () => {
    const { api, release } = heldApi();
    const create = spyOn(URL, "createObjectURL");
    try {
      const { root } = await mount(api, { page: page() });
      await act(async () => root.unmount());
      mounted = null;
      await release(0, { mime: "image/png", data: PNG });
      expect(create).not.toHaveBeenCalled();
    } finally {
      create.mockRestore();
    }
  });

  test("useKnowledgeAssets decode failure marked failed", async () => {
    const { api, calls, release, refuse } = heldApi();
    const two = page({ images: [{ path: "/img.png", asset: ASSET }, { path: "/b.png", asset: `sha256:${"b".repeat(64)}` }] });
    const { out } = await mount(api, { page: two });
    expect(calls).toHaveLength(2);
    await release(0, { mime: "image/png", data: "%%% not base64" });
    await refuse(1);
    expect(out.image!("/img.png")).toBeNull();
    expect(out.image!("/b.png")).toBeNull();
    // Failed, not pending: nothing is asked again on the next render.
    expect(calls).toHaveLength(2);
  });

  test("useKnowledgeAssets mime outside the four images marked failed", async () => {
    const { api, release } = heldApi();
    const { out } = await mount(api, { page: page() });
    await release(0, { mime: "image/svg+xml", data: PNG });
    expect(out.image!("/img.png")).toBeNull();
  });

  test("useKnowledgeAssets null page clears then reloads", async () => {
    const { api, calls, release } = heldApi();
    const revoke = spyOn(URL, "revokeObjectURL");
    try {
      const { out, render } = await mount(api, { page: page() });
      await release(0, { mime: "image/png", data: PNG });
      const first = out.image!("/img.png");
      expect(first).toStartWith("blob:");
      // A page re-read with the same images keeps them.
      await render({ page: page() });
      expect(calls).toHaveLength(1);
      expect(out.image!("/img.png")).toBe(first);
      // No page (the deck browses its history): cleared; the page back asks again.
      await render({ page: null });
      expect(revoke).toHaveBeenCalledWith(first);
      expect(out.image!("/img.png")).toBeNull();
      await render({ page: page() });
      expect(calls).toHaveLength(2);
      await release(1, { mime: "image/png", data: PNG });
      expect(out.image!("/img.png")).toStartWith("blob:");
    } finally {
      revoke.mockRestore();
    }
  });

  test("useKnowledgeAssets many images four in flight land together", async () => {
    const { api, calls, release } = heldApi();
    const images = Array.from({ length: 6 }, (_, i) => ({ path: `/i${i}.png`, asset: `sha256:${String(i).repeat(64)}` }));
    const { out } = await mount(api, { page: page({ images }) });
    expect(calls).toHaveLength(4);
    const seen = new Set<(path: string) => string | null>([out.image!]);
    for (let i = 0; i < 5; i++) {
      await release(i, { mime: "image/png", data: PNG });
      seen.add(out.image!);
    }
    expect(calls).toHaveLength(6);
    // Nothing lands before every image settled: the body is not rendered again per image.
    expect(seen.size).toBe(1);
    expect(out.image!("/i0.png")).toBeNull();
    await release(5, { mime: "image/png", data: PNG });
    for (const i of images) expect(out.image!(i.path)).toStartWith("blob:");
  });

  test("useKnowledgeAssets awaiting page loads nothing", async () => {
    const { api, calls } = heldApi();
    const { out } = await mount(api, { page: page({ awaiting: true }) });
    expect(calls).toHaveLength(0);
    expect(out.image!("/img.png")).toBeNull();
  });
});
