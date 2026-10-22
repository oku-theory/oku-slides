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

/** Un tableau de liste d'une ligne, lié à `/f0`, dont l'action part avec le changement. */
const listSurface: Surface = {
  ...surface,
  messages: [
    surface.messages[0],
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "host-widget",
        components: [{
          id: "root", component: "ListTable", label: "Étapes", rows: { path: "/f0" }, change: { path: "/f0-change" },
          markdown: { path: "/f0-markdown" }, columns: [{ key: "date", label: "date", kind: "text", options: [] }], min: 0,
          palette: [], action: { event: { name: "param", context: { change: { path: "/f0-change" }, key: "etapes" } } },
        }],
      },
    },
    { version: "v0.9", updateDataModel: { surfaceId: "host-widget", value: { f0: [{ id: "d0", date: "2019" }], "f0-change": null, "f0-markdown": {} } } },
  ] as unknown as A2uiMessage[],
};

async function mount(shown: Surface = surface, answer: Promise<boolean> | null = null) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const actions: [string, string][] = [];
  const outcome = { current: null as Promise<boolean> | null };
  await act(async () =>
    root.render(
      createElement(HostSettings, {
        surface: shown,
        onAction: (s: Surface, a: A2uiClientAction) => {
          actions.push([s.id, a.name]);
          outcome.current = answer;
        },
        outcome,
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
  const ids = [...box.querySelectorAll<HTMLElement>("[data-surface-id]")].map((e) => e.dataset.surfaceId);
  expect([ids, box.getAttribute("aria-live")]).toEqual([["host-widget"], "off"]);
});

test("HostSettings rend la surface et route ses actions avec elle", async () => {
  const { host, actions } = await mount();
  await act(async () => host.querySelector("button")!.click());
  expect(actions).toEqual([["host-widget", "param"]]);
});

test("HostSettings rend aux champs l'issue de leurs actions : une cellule refusée revient à sa valeur", async () => {
  const { host, actions } = await mount(listSurface, Promise.resolve(false));
  const cell = host.querySelector<HTMLInputElement>("tbody input")!;
  await act(async () => {
    cell.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(cell, "2020");
    cell.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => cell.blur());
  expect(actions).toEqual([["host-widget", "param"]]);
  expect(host.querySelector<HTMLInputElement>("tbody input")!.value).toBe("2019");
});
