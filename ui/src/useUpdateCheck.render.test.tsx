// Recherche de mise à jour demandée par le menu de l'app : résultat dit par l'avis de l'onglet.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateStatus } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useUpdateCheck } = await import("./useUpdateCheck");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const said: Array<[string, boolean]> = [];
const say = (text: string, error = false) => void said.push([text, error]);

async function mount(check: () => Promise<UpdateStatus | null>, requested = 0) {
  said.length = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  function Probe({ n }: { n: number }) {
    useUpdateCheck({ check, requested: n }, say);
    return null;
  }
  await act(async () => root.render(createElement(Probe, { n: requested })));
  return { rerender: (n: number) => act(async () => root.render(createElement(Probe, { n }))) };
}

describe("useUpdateCheck", () => {
  test("useUpdateCheck menu request when up to date says so", async () => {
    const m = await mount(async () => ({ offer: null, installing: false }));
    await m.rerender(1);
    expect(said).toEqual([["ui-update-up-to-date", false]]);
  });

  test("useUpdateCheck offer checks once and says nothing", async () => {
    let checks = 0;
    const m = await mount(async () => (checks++, { offer: { version: "2.2.0", mode: "install" }, installing: false }));
    await m.rerender(1);
    expect([checks, said]).toEqual([1, []]);
  });

  test("useUpdateCheck failure says the error", async () => {
    const m = await mount(async () => { throw new Error("réseau"); });
    await m.rerender(1);
    expect(said).toEqual([["réseau", true]]);
  });

  test("useUpdateCheck mount does not check", async () => {
    await mount(async () => ({ offer: null, installing: false }), 3);
    expect(said).toEqual([]);
  });
});
