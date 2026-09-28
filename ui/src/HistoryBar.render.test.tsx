// Rendu du bandeau de l'historique dans un DOM happy-dom.

import { afterEach, describe, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const HistoryBar = (await import("./HistoryBar")).default;
const { expectIconOnly } = await import("./test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const version = { id: "v1", timestamp: 0 };
  const history = { count: 1, index: 0, version, changes: { added_since: [] } };
  const cursor = { versions: [version], value: 0, go: () => {}, arrow: () => {} };
  await act(async () => {
    root.render(createElement(HistoryBar, { history, cursor, onRestore: () => {}, onBack: () => {} } as never));
  });
  return host;
}

describe("HistoryBar", () => {
  test("HistoryBar restore and back buttons show icons only", async () => {
    const host = await mount();
    const [restore, back] = host.querySelectorAll(".history-actions button");
    expectIconOnly(restore);
    expectIconOnly(back);
  });
});
