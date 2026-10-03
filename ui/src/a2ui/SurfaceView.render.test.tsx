// Cycle de vie d'une surface : montage, démontage, remontage (StrictMode).

import { afterEach, expect, spyOn, test } from "bun:test";
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

// Le processeur écrit dans les valeurs des messages : chaque test prend les siens.
const fresh = () => [
  { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
  { version: "v0.9", updateComponents: { surfaceId: "s", components: [
    { id: "root", component: "Column", children: ["d", "n"] },
    { id: "d", component: "Disclosure", summary: "Pourquoi", child: "w" },
    { id: "w", component: "Text", text: "parce que" },
    { id: "n", component: "Text", text: { path: "/n" } },
  ] } },
  { version: "v0.9", updateDataModel: { surfaceId: "s", value: { n: "un" } } },
] as unknown as A2uiMessage[];

const view = (messages: A2uiMessage[], errors: string[] = []) =>
  createElement(SurfaceView, { surfaceId: "s", messages, onAction: () => {}, onError: (m: string) => errors.push(m) });

async function mountView(messages: A2uiMessage[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(view(messages)));
  return { host, root };
}

test("une liste prolongée met les données à jour sans reconstruire la surface", async () => {
  const base = fresh();
  const { host, root } = await mountView(base);
  await act(async () => (host.querySelector(".a2ui-disclosure > button") as HTMLElement).click());
  expect(host.textContent).toContain("parce que");
  const patch = { version: "v0.9", updateDataModel: { surfaceId: "s", path: "/n", value: "deux" } } as unknown as A2uiMessage;
  await act(async () => root.render(view([...base, patch])));
  expect(host.textContent).toContain("deux");
  expect(host.textContent).toContain("parce que"); // la Disclosure est restée ouverte : pas de reconstruction
});

test("une liste non prolongée reconstruit la surface", async () => {
  const base = fresh();
  const { host, root } = await mountView(base);
  await act(async () => (host.querySelector(".a2ui-disclosure > button") as HTMLElement).click());
  expect(host.textContent).toContain("parce que"); // ouverte avant la nouvelle liste
  await act(async () => root.render(view([...base])));
  expect(host.textContent).not.toContain("parce que"); // nouvelle liste (copie) : surface neuve, Disclosure fermée
});

test("un patch refusé est signalé une fois et la surface affichée reste", async () => {
  // @a2ui journalise l'erreur de validation : attendue ici.
  const logged = spyOn(console, "error").mockImplementation(() => {});
  try {
    const base = fresh();
    const errors: string[] = [];
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    await act(async () => root.render(view(base, errors)));
    // Un composant connu, strict, avec une propriété inconnue : refusé par la validation zod.
    const bad = { version: "v0.9", updateComponents: { surfaceId: "s", components: [{ id: "n", component: "Badge", text: "x", extra: 1 }] } } as unknown as A2uiMessage;
    await act(async () => root.render(view([...base, bad], errors)));
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain("extra");
    expect(host.textContent).toContain("un"); // la surface d'avant est toujours là
  } finally {
    logged.mockRestore();
  }
});
