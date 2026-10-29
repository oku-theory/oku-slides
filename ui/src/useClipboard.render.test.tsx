// La coupe appartient à l'onglet : quitter l'onglet (démonter son App) la change en copie.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Element, SlideView, Snapshot, TabApi } from "./api";
import type { Clip, ClipRef } from "./useClipboard";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useClipboard, useClipboardEvents } = await import("./useClipboard");

const element = { id: "e-image", kind: "base/image", params: {} } as unknown as Element;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  shownCut = null;
  markCut = () => {};
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** La coupe marquée lue au dernier rendu de `Tab`. */
let shownCut: ReturnType<typeof useClipboard>["cutMark"] = null;
/** Le `setCutMark` du dernier rendu de `Tab`. */
let markCut: ReturnType<typeof useClipboard>["setCutMark"] = () => {};

/** Un onglet qui tient `clip`, comme App sous la fenêtre ; `gen` : copies venues d'ailleurs. */
const Tab = ({ clip, gen }: { clip: ClipRef; n: number; gen?: number }) => {
  const board = useClipboard(clip, gen ?? 0, null, null);
  shownCut = board.cutMark;
  markCut = board.setCutMark;
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

  test("useClipboard given a copy from another window drops its cut mark", async () => {
    const clip: ClipRef = { current: null };
    const root = await mountTab(clip);
    clip.current = { element, text: "base/image", tab: "t-1", cut: { slide: "s-1" } };
    await act(async () => markCut({ slide: "s-1", element: "e-image" }));
    // Copié dans une autre fenêtre : l'élément est remplacé, sans coupe.
    clip.current = { element, text: "base/image", tab: "t-9" };
    await act(async () => root.render(createElement(Tab, { clip, n: 1, gen: 1 })));
    expect(shownCut).toBeNull();
  });

  test("useClipboard rerendered without a new copy keeps its cut mark", async () => {
    const clip: ClipRef = { current: null };
    const root = await mountTab(clip);
    clip.current = { element, text: "base/image", tab: "t-1", cut: { slide: "s-1" } };
    await act(async () => markCut({ slide: "s-1", element: "e-image" }));
    await act(async () => root.render(createElement(Tab, { clip, n: 1, gen: 0 })));
    expect(shownCut).toEqual({ slide: "s-1", element: "e-image" });
  });

  test("useClipboard given a remote copy while its own clip is cut keeps its cut mark", async () => {
    const clip: ClipRef = { current: null };
    const root = await mountTab(clip);
    clip.current = { element, text: "base/image", tab: "t-1", cut: { slide: "s-1" } };
    await act(async () => markCut({ slide: "s-1", element: "e-image" }));
    // `gen` croît, mais une coupe faite ici depuis tient l'élément : la marque reste.
    await act(async () => root.render(createElement(Tab, { clip, n: 1, gen: 1 })));
    expect(shownCut).toEqual({ slide: "s-1", element: "e-image" });
  });
});

describe("useClipboardEvents", () => {
  const view = { slide: { id: "layout-title", kind: "essentiel/title", params: {} } } as unknown as SlideView;
  const snap = (theming: boolean) =>
    ({
      slides: [{ id: "layout-title" }], history: null, read_only: false, theming: theming ? {} : null,
      free_slots: { "essentiel/title": null }, add_packs: [{ widgets: [{ name: "base/text", seed: { params: {} } }] }],
    }) as unknown as Snapshot;

  /** Mounts a tab listening to the clipboard, pastes `text` on the window; the elements added. */
  async function pasteIn(themeTab: boolean, text: string) {
    const added: unknown[] = [];
    const s = snap(themeTab);
    function Probe() {
      const board = useClipboard({ current: null }, 0, s, view);
      useClipboardEvents({
        tab: "t", api: {} as TabApi, snap: s, snapRef: { current: s }, view, selected: null, editing: null,
        dialogOpen: false, packTab: false, themeTab, apply: async () => true, addElement: (w) => void added.push(w),
        setSelected: () => {}, say: () => {}, share: () => {}, clip: board.clip, setCutMark: board.setCutMark, dropCut: board.dropCut,
      });
      return null;
    }
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    await act(async () => root.render(createElement(Probe)));
    const e = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => text } });
    await act(async () => void document.body.dispatchEvent(e));
    return added;
  }

  test("useClipboardEvents in a deck pastes outside text as a text element", async () => {
    expect(await pasteIn(false, "Bonjour")).toHaveLength(1);
  });

  test("useClipboardEvents in a theme tab pastes no element", async () => {
    expect(await pasteIn(true, "Bonjour")).toHaveLength(0);
  });
});
