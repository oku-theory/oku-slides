// Cycle de vie d'une surface : montage, démontage, remontage (StrictMode).

import { afterEach, expect, test } from "bun:test";
import type { A2uiMessage } from "@a2ui/web_core/v0_9";
import "../test/dom";
import { CATALOG_ID } from "./catalog";

// Imports dynamiques après `../test/dom` : voir catalog.render.test.tsx.
const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { SurfaceView } = await import("./SurfaceView");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const messages = [
  { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
  { version: "v0.9", updateComponents: { surfaceId: "s", components: [{ id: "root", component: "Text", text: "Bonjour" }] } },
] as unknown as A2uiMessage[];

test("SurfaceView remontée par StrictMode garde ses composants", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const view = createElement(SurfaceView, { messages, surfaceId: "s", onAction: () => {}, onError: () => {} });
  await act(async () => root.render(createElement(StrictMode, null, view)));
  expect(host.textContent).toContain("Bonjour");
});
