// Les widgets des packs du snapshot : la grille d'ajout gardée tant que son contenu ne change pas.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot } from "./api";
import type { AddPackView } from "./packTypes";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePackWidgets } = await import("./usePackWidgets");

let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

const packs = (label: string): AddPackView[] => [
  { namespace: "base", name: "Base", widgets: [{ name: "base/text", label, description: null, seed: { params: { body: [] } } }] },
];
const snapshot = (label: string) => ({ pack_widgets: [], add_packs: packs(label) }) as unknown as Snapshot;

async function mount(first: Snapshot) {
  const seen: ReturnType<typeof usePackWidgets>[] = [];
  function Probe({ snap }: { snap: Snapshot }) {
    seen.push(usePackWidgets(snap));
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const render = (snap: Snapshot) => act(async () => root.render(createElement(Probe, { snap })));
  await render(first);
  return { seen, render };
}

test("usePackWidgets with the same add grid content gives the same array identity", async () => {
  const m = await mount(snapshot("Texte"));
  await m.render(snapshot("Texte"));
  expect(m.seen.at(-1)!.addSections).toBe(m.seen[0].addSections);
});

test("usePackWidgets with a relabelled widget gives a new add grid", async () => {
  const m = await mount(snapshot("Texte"));
  await m.render(snapshot("Text"));
  expect(m.seen.at(-1)!.addSections[0].choices[0].label).toBe("Text");
});
