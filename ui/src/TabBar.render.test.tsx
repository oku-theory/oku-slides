// Barre d'onglets : le menu contextuel échappe à la vitre de la barre.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { TabInfo, WindowView, WorkspaceView } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const TabBar = (await import("./TabBar")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const tab = (id: string): TabInfo => ({
  id,
  title: id,
  dirty: false,
  editing_templates: false,
  path: null,
  pristine: true,
  deck_uid: null,
});

async function mount() {
  const win: WindowView = { label: "main", active: "a", tabs: [tab("a"), tab("b")] };
  const view: WorkspaceView = { generation: 1, focused: "main", agent_detached: false, windows: [win] };
  const noop = () => {};
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () =>
    root.render(
      createElement(TabBar, {
        view,
        win,
        windows: false,
        onActivate: noop,
        onClose: noop,
        onCloseOthers: noop,
        onNew: noop,
        onReorder: noop,
        onMove: noop,
        onDragOver: async () => {},
        onDragLeave: async () => {},
        onDropOutside: noop,
      }),
    ),
  );
  return host;
}

// t-812 : la barre a un `backdrop-filter` (verre) ; un menu `position: fixed`
// rendu dedans se place par rapport à elle, sous la fenêtre. Rendu dans `body`.
test("TabBar context menu renders outside the glass tab bar", async () => {
  const host = await mount();
  const first = host.querySelector('[role="tab"]')!;
  await act(async () => {
    first.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 880 }));
  });
  expect(document.querySelector(".tab-menu")?.parentElement).toBe(document.body);
});
