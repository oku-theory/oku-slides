// La coupe appartient à l'onglet : quitter l'onglet (démonter son App) la change en copie.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Element } from "./api";
import type { Clip, ClipRef } from "./useClipboard";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useClipboard } = await import("./useClipboard");

const element = { id: "e-image", kind: "base/image", params: {} } as unknown as Element;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Un onglet qui tient `clip`, comme App sous la fenêtre ; `n` change à chaque rendu voulu. */
const Tab = ({ clip }: { clip: ClipRef; n: number }) => {
  useClipboard(clip, null, null);
  return null;
};

/** Monte un onglet qui tient `clip`. */
async function mountTab(clip: ClipRef) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(Tab, { clip, n: 0 })));
  return root;
}

describe("useClipboard", () => {
  test("useClipboard unmounted with a cut keeps the element as a copy", async () => {
    const clip: ClipRef = { current: null };
    const root = await mountTab(clip);
    const cut: Clip = { element, text: "base/image", tab: "t-1", cut: { slide: "s-1" } };
    clip.current = cut;
    await act(async () => root.unmount());
    mounted = null;
    expect(clip.current).toEqual({ element, text: "base/image", tab: "t-1", cut: undefined });
  });

  test("useClipboard rerendered keeps a cut made in its tab", async () => {
    const clip: ClipRef = { current: null };
    const root = await mountTab(clip);
    clip.current = { element, text: "base/image", tab: "t-1", cut: { slide: "s-1" } };
    await act(async () => root.render(createElement(Tab, { clip, n: 1 })));
    expect(clip.current.cut).toEqual({ slide: "s-1" });
  });
});
