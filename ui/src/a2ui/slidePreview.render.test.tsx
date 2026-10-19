// Survol d'une miniature de variante : la scène montre son rendu (t-1070).

import { describe, expect, test } from "bun:test";
import "../test/dom";

// react-dom cherche le DOM à son chargement : React et la surface après `../test/dom`.
const { act, createElement, useState } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { SlidePreviewContext, nextPreview } = await import("./slidePreview");

/** Monte une miniature sous un aperçu tenu comme dans l'app ; `close` démonte la miniature (tiroir fermé). */
async function mountThumb(svg = "<svg/>") {
  const state = { url: null as string | null, close: () => {}, setSvg: (_: string) => {} };
  const thumb = (s: string) => messagesOf([{ id: "root", component: "SlideThumb", svg: s, label: "2 · Papier" }]);
  function Host({ children }: { children: React.ReactElement }) {
    const [open, setOpen] = useState(true);
    state.close = () => setOpen(false);
    const preview = (url: string, on: boolean) => (state.url = nextPreview(state.url, url, on));
    return createElement(SlidePreviewContext.Provider, { value: preview }, open ? children : null);
  }
  const { host } = await mountSurface(thumb(svg), (s) => createElement(Host, null, s));
  const button = host.querySelector<HTMLButtonElement>(".slide-thumb")!;
  return { button, src: () => button.querySelector("img")!.getAttribute("src"), state };
}

const fire = (el: Element, type: string, init: PointerEventInit = {}) =>
  act(async () => void el.dispatchEvent(new PointerEvent(type, { bubbles: true, ...init })));
const leave = (el: Element) => fire(el, "pointerout", { relatedTarget: document.body });

describe("aperçu d'une variante sur la scène", () => {
  test("SlideThumb survolée montre son rendu sur la scène", async () => {
    const { button, src, state } = await mountThumb();
    await fire(button, "pointerover");
    expect(state.url).toBe(src());
  });

  test("SlideThumb quittée rend la slide à la scène", async () => {
    const { button, state } = await mountThumb();
    await fire(button, "pointerover");
    await leave(button);
    expect(state.url).toBeNull();
  });

  test("SlideThumb prenant le focus clavier montre son rendu sur la scène", async () => {
    const { button, src, state } = await mountThumb();
    await act(async () => button.focus());
    expect(state.url).toBe(src());
  });

  test("SlideThumb quittée par la souris mais focalisée garde son rendu", async () => {
    const { button, src, state } = await mountThumb();
    await act(async () => button.focus());
    await fire(button, "pointerover");
    await leave(button);
    expect(state.url).toBe(src());
  });

  test("SlideThumb démontée pendant l'aperçu rend la slide à la scène", async () => {
    const { button, state } = await mountThumb();
    await fire(button, "pointerover");
    await act(async () => state.close());
    expect(state.url).toBeNull();
  });
});
