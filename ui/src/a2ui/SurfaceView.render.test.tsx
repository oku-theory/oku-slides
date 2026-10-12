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
  const { host, root } = await mountView(fresh());
  expect(host.textContent).toContain("un");
  const next = fresh();
  (next[2] as unknown as { updateDataModel: { value: { n: string } } }).updateDataModel.value.n = "autre";
  await act(async () => root.render(view(next)));
  expect(host.textContent).toContain("autre"); // nouvelle liste : surface neuve, lue en entier
});

/** La même surface avec une suite (bouton `suggest`), comme un reçu avant l'expiration de ses actions. */
const withSuggest = () => {
  const messages = fresh();
  const update = (messages[1] as unknown as { updateComponents: { components: Record<string, unknown>[] } }).updateComponents;
  update.components[0] = { id: "root", component: "Column", children: ["d", "n", "go"] };
  update.components.push({ id: "go", component: "Button", label: "Encore", action: { event: { name: "suggest", context: { prompt: "Encore" } } } });
  return messages;
};

test("le détail ouvert le reste quand la surface est remplacée par sa version sans actions", async () => {
  const { host, root } = await mountView(withSuggest());
  await act(async () => (host.querySelector(".a2ui-disclosure > button") as HTMLElement).click());
  expect(host.textContent).toContain("parce que");
  // L'expiration des suites : mêmes composants sans le bouton, liste neuve (surface reconstruite).
  await act(async () => root.render(view(fresh())));
  expect(host.textContent).not.toContain("Encore");
  expect(host.textContent).toContain("parce que");
  expect(host.querySelector(".a2ui-disclosure > button")?.getAttribute("aria-expanded")).toBe("true");
  // Refermé, il reste fermé à la reconstruction suivante.
  await act(async () => (host.querySelector(".a2ui-disclosure > button") as HTMLElement).click());
  await act(async () => root.render(view(fresh())));
  expect(host.textContent).not.toContain("parce que");
});

test("une autre surface ne reprend pas l'ouverture du détail de la précédente", async () => {
  const { host, root } = await mountView(fresh());
  await act(async () => (host.querySelector(".a2ui-disclosure > button") as HTMLElement).click());
  const other = JSON.parse(JSON.stringify(fresh()).replaceAll('"s"', '"t"')) as A2uiMessage[];
  await act(async () => root.render(createElement(SurfaceView, { surfaceId: "t", messages: other, onAction: () => {}, onError: () => {} })));
  expect(host.textContent).not.toContain("parce que");
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

test("une suite avec un Slider aux bornes inversées est signalée et ne rend aucun curseur", async () => {
  const errors: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const base = fresh();
  await act(async () => root.render(view(base, errors)));
  const tail = {
    version: "v0.9",
    updateComponents: { surfaceId: "s", components: [{ id: "g", component: "Slider", label: "n", value: 1, min: 10, max: 2 }] },
  } as unknown as A2uiMessage;
  await act(async () => root.render(view([...base, tail], errors)));
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("min must be < max");
  expect(host.querySelector('input[type="range"]')).toBeNull();
});

/** Une timeline de tour : une étape d'outils dépliable (un appel), comme l'envoie l'hôte. */
const turn = () => [
  { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
  { version: "v0.9", updateComponents: { surfaceId: "s", components: [
    { id: "root", component: "Timeline", steps: { path: "/steps" }, collapsed: false, summary: "" },
  ] } },
  { version: "v0.9", updateDataModel: { surfaceId: "s", value: { steps: [
    { label: "widgets", status: "completed", count: 1, calls: [{ label: "widgets", status: "completed", input: "{}" }] },
  ] } } },
] as unknown as A2uiMessage[];

const write = (path: string, value: unknown) =>
  ({ version: "v0.9", updateDataModel: { surfaceId: "s", path, value } }) as unknown as A2uiMessage;

test("une étape écrite par son index (/steps/1) s'ajoute ; l'étape dépliée le reste", async () => {
  const base = turn();
  const { host, root } = await mountView(base);
  await act(async () => (host.querySelector(".a2ui-step-toggle") as HTMLElement).click());
  const next = [...base, write("/steps/1", { label: "Fini.", status: "completed" }), write("/steps/0/calls/0/status", "failed")];
  await act(async () => root.render(view(next)));
  expect([...host.querySelectorAll(".a2ui-step-label")].map((x) => x.textContent)).toEqual(["widgets", "Fini."]);
  expect(host.querySelector(".a2ui-call")?.classList.contains("failed")).toBe(true);
});

test("une entrée de plan ajoutée devant les étapes ne déplie pas une autre étape", async () => {
  const base = turn();
  const { host, root } = await mountView(base);
  await act(async () => (host.querySelector(".a2ui-step-toggle") as HTMLElement).click());
  const steps = [
    { label: "Lire la slide", status: "completed" },
    { label: "widgets", status: "completed", count: 1, calls: [{ label: "widgets", status: "completed", input: "{}" }] },
  ];
  await act(async () => root.render(view([...base, write("/steps", steps)])));
  expect(host.querySelectorAll(".a2ui-call").length).toBe(1);
  expect(host.querySelector(".a2ui-step-toggle")?.getAttribute("aria-expanded")).toBe("true");
});

test("une étape d'outils vidée par le plafond ne fait pas passer l'état déplié à sa voisine", async () => {
  const base = [
    ...turn().slice(0, 2),
    { version: "v0.9", updateDataModel: { surfaceId: "s", value: { steps: [
      { label: "slide", status: "completed", count: 1, calls: [{ label: "slide", status: "completed", input: "{}" }] },
      { label: "apply", status: "completed", count: 1, calls: [{ label: "apply", status: "completed", input: "{}" }] },
    ] } } },
  ] as unknown as A2uiMessage[];
  const { host, root } = await mountView(base);
  await act(async () => (host.querySelectorAll(".a2ui-step-toggle")[1] as HTMLElement).click());
  // Le plus ancien appel du tour sort de la liste : la première étape n'a plus d'appels listés.
  await act(async () => root.render(view([...base, write("/steps/0", { label: "slide", status: "completed", count: 1 })])));
  expect([...host.querySelectorAll(".a2ui-call-label")].map((x) => x.textContent)).toEqual(["apply"]);
});
