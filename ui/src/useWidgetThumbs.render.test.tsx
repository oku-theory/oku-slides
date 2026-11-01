// A widget pack's thumbnails: each widget's preview, cropped, asked one at a time and kept by key.

import { afterEach, expect, spyOn, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetPreview } from "./api";
import type { ThumbItem } from "./useWidgetThumbs";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useWidgetThumbs } = await import("./useWidgetThumbs");
const { deferred, wait } = await import("./test/widgetApi");

type Hook = ReturnType<typeof useWidgetThumbs>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
let spies: (() => void) | null = null;
afterEach(async () => {
  spies?.();
  spies = null;
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const SVG = '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt"></svg>';
const preview = (bbox = { x: 10, y: 20, w: 300, h: 100 }): WidgetPreview => ({ svg: SVG, bbox, diagnostics: [] });

/** Mounts the hook over `items`; `asked` lists each preview asked; `blobs` each image made. */
async function mount(
  items: ThumbItem[] | null,
  answer: (name: string) => Promise<WidgetPreview> = async () => preview(),
  strict = false,
) {
  const asked: string[] = [];
  const api = { previewWidget: (name: string) => (asked.push(name), answer(name)) } as unknown as TabApi;
  const blobs: Blob[] = [];
  const revoked: string[] = [];
  const made = spyOn(URL, "createObjectURL").mockImplementation((b) => (blobs.push(b as Blob), `blob:${blobs.length}`));
  const freed = spyOn(URL, "revokeObjectURL").mockImplementation((u) => void revoked.push(u));
  const out: { hook: Hook | null } = { hook: null };
  function Probe({ list }: { list: ThumbItem[] | null }) {
    out.hook = useWidgetThumbs(api, list);
    return null;
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const probe = (list: ThumbItem[] | null) => createElement(Probe, { list });
  const render = (list: ThumbItem[] | null) =>
    act(async () => root.render(strict ? createElement(StrictMode, null, probe(list)) : probe(list)));
  await render(items);
  await wait(5);
  spies = () => (made.mockRestore(), freed.mockRestore());
  return { asked, blobs, out, render, revoked, root };
}

const A = { name: "a", key: "h1" };
const B = { name: "b", key: "h2" };

test("useWidgetThumbs without items asks no preview", async () => {
  const { asked } = await mount(null);
  expect(asked).toEqual([]);
});

test("useWidgetThumbs crops each widget's thumbnail to what it draws", async () => {
  const { blobs } = await mount([A]);
  expect(await blobs[0].text()).toContain('viewBox="10 20 300 100"');
});

test("useWidgetThumbs gives each item its image URL", async () => {
  const { out } = await mount([A, B]);
  expect([out.hook!.url("a"), out.hook!.url("b")]).toEqual(["blob:1", "blob:2"]);
});

test("useWidgetThumbs asks one preview at a time", async () => {
  const held = deferred<WidgetPreview>();
  const { asked } = await mount([A, B], () => held.promise);
  expect(asked).toEqual(["a"]);
});

test("useWidgetThumbs with unchanged keys asks nothing again", async () => {
  const { asked, render } = await mount([A]);
  await render([{ ...A }]);
  await wait(5);
  expect(asked).toEqual(["a"]);
});

test("useWidgetThumbs with a new key asks that widget again", async () => {
  const { asked, render } = await mount([A, B]);
  await render([A, { ...B, key: "h3" }]);
  await wait(5);
  expect(asked).toEqual(["a", "b", "b"]);
});

test("useWidgetThumbs after a refused preview does not ask again for the same key", async () => {
  const { asked, render } = await mount([A], async () => Promise.reject(new Error("refusé")));
  await render([A]);
  await wait(5);
  expect(asked).toEqual(["a"]);
});

test("useWidgetThumbs put takes a preview rendered elsewhere without asking", async () => {
  const held = deferred<WidgetPreview>();
  const { asked, out, render } = await mount([A, B], (n) => (n === "a" ? held.promise : Promise.resolve(preview())));
  await act(async () => out.hook!.put("b", preview()));
  held.resolve(preview());
  await wait(5);
  await render([A, B]);
  await wait(5);
  expect(asked).toEqual(["a"]);
});

test("useWidgetThumbs under StrictMode asks each widget once", async () => {
  const { asked } = await mount([A, B], undefined, true);
  expect(asked).toEqual(["a", "b"]);
});

test("useWidgetThumbs keeps a handed-over preview over an older answer", async () => {
  const held = deferred<WidgetPreview>();
  const { asked, out, render } = await mount([A], () => held.promise);
  await render([{ ...A, key: "h9" }]);
  await act(async () => out.hook!.put("a", preview()));
  held.resolve(preview());
  await wait(5);
  expect([asked, out.hook!.url("a")]).toEqual([["a"], "blob:1"]);
});

test("useWidgetThumbs frees the URL of a removed widget", async () => {
  const { render, revoked } = await mount([A, B]);
  await render([A]);
  expect(revoked).toEqual(["blob:2"]);
});

test("useWidgetThumbs frees every URL on unmount", async () => {
  const { revoked, root } = await mount([A, B]);
  await act(async () => root.unmount());
  mounted = null;
  expect(revoked.sort()).toEqual(["blob:1", "blob:2"]);
});
