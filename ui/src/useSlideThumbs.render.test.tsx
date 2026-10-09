import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi } from "./api";
import type { SlideThumbs } from "./a2ui/slideThumbs";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useSlideThumbs } = await import("./useSlideThumbs");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const api = { svg: async () => "<svg/>" } as unknown as TabApi;

async function mountThumbs(slides: { id: string; hash: string }[]) {
  let value: SlideThumbs | null = null;
  const Probe = () => {
    value = useSlideThumbs(api, slides as never);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => act(async () => root.unmount());
  return () => value!;
}

test("useSlideThumbs donne l'empreinte actuelle d'une slide", async () => {
  const thumbs = await mountThumbs([{ id: "s1", hash: "h1" }]);
  expect(thumbs().current("s1")).toBe("h1");
});

test("useSlideThumbs slide disparue : pas d'empreinte", async () => {
  const thumbs = await mountThumbs([]);
  expect(thumbs().current("s1")).toBeUndefined();
});
