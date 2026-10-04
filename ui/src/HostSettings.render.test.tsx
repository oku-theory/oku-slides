// Réglages de l'hôte posés sous leur aperçu : la surface, son repère, ses actions.

import { afterEach, expect, test } from "bun:test";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import "./test/dom";
import { CATALOG_ID } from "./a2ui/catalog";
import type { Surface } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: HostSettings } = await import("./HostSettings");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const surface: Surface = {
  id: "host-widget",
  place: "thread",
  live: true,
  question: false,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "host-widget", catalogId: CATALOG_ID } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "host-widget",
        components: [{ id: "root", component: "Button", label: "Essai", action: { event: { name: "param", context: {} } } }],
      },
    },
  ] as unknown as A2uiMessage[],
};

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const actions: [string, string][] = [];
  await act(async () =>
    root.render(
      createElement(HostSettings, {
        surface,
        onAction: (s: Surface, a: A2uiClientAction) => actions.push([s.id, a.name]),
        onError: () => {},
      }),
    ),
  );
  // SurfaceView se charge à part.
  for (let i = 0; i < 50 && !host.querySelector("button"); i++) await act(() => new Promise((r) => setTimeout(r, 10)));
  return { host, actions };
}

test("HostSettings porte l'id de sa surface et ne s'annonce pas", async () => {
  const { host } = await mount();
  const box = host.querySelector<HTMLElement>(".host-settings")!;
  expect([box.dataset.surfaceId, box.getAttribute("aria-live")]).toEqual(["host-widget", "off"]);
});

test("HostSettings rend la surface et route ses actions avec elle", async () => {
  const { host, actions } = await mount();
  await act(async () => host.querySelector("button")!.click());
  expect(actions).toEqual([["host-widget", "param"]]);
});
