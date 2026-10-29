// Hors contexte sûr (UI servie en HTTP sur une adresse réseau), `crypto.randomUUID` lève : l'ordre des
// sauvegardes doit quand même partir (t-695). Le module tire son `page` au chargement : il est réévalué ici (requête `?insecure`, hors cache),
// quel que soit l'ordre des fichiers de test.

import { expect, test } from "bun:test";
import "./test/dom";
import type { UiOrder, UiState } from "./api";

const original = Object.getOwnPropertyDescriptor(globalThis.crypto, "randomUUID");
Object.defineProperty(globalThis.crypto, "randomUUID", {
  value: () => { throw new TypeError("crypto.randomUUID is not a function"); },
  configurable: true,
});

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
let useKeptUi: typeof import("./useKeptUi").useKeptUi;
let fresh = false;
try {
  const cached = await import("./useKeptUi");
  const loaded = await import("./useKeptUi?insecure");
  fresh = loaded !== cached;
  useKeptUi = loaded.useKeptUi;
} finally {
  // Rétabli aussitôt : les autres fichiers de test partagent le processus.
  if (original) Object.defineProperty(globalThis.crypto, "randomUUID", original);
  else delete (globalThis.crypto as { randomUUID?: unknown }).randomUUID;
}

test("useKeptUi sans randomUUID envoie la sauvegarde avec un ordre", async () => {
  expect(fresh).toBe(true);
  const kept: UiState = { slide: "s0", grid: false, ghosts: true, pile: 200, notes: null, widget: null, code: true,
    code_width: 640, folded: [] };
  const orders: UiOrder[] = [];
  let hook: ReturnType<typeof useKeptUi> | null = null;
  function Probe() {
    hook = useKeptUi(kept, "s1", async (_ui, order) => { orders.push(order); }, { widget: null });
    return null;
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  await act(async () => { hook!.setCodeWidth(320); });
  expect(orders.length > 0 && typeof orders[0]!.page === "string" && orders[0]!.page.length > 0).toBe(true);
  await act(async () => root.unmount());
  host.remove();
});
