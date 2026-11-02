// The data pane: the selected chart's or table's host-data surface, and its close button.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// See ./a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DataPane } = await import("./DataPane");
const { realSurface } = await import("./a2ui/fixtures/agentSurfaces");
const { t } = await import("./i18n");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
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
  const calls: string[] = [];
  await act(async () =>
    root.render(
      createElement(DataPane, {
        data: { surface: realSurface("host-data"), onAction: () => true, outcome: { current: null } },
        onAction: () => true,
        onError: () => {},
        onClose: () => void calls.push("close"),
      }),
    ),
  );
  return { host, calls };
}

describe("DataPane", () => {
  test("DataPane shows the host-data surface", async () => {
    const m = await mount();
    expect(!!m.host.querySelector('.data-pane [data-surface-id="host-data"]')).toBe(true);
  });

  test("DataPane close button calls onClose", async () => {
    const m = await mount();
    const close = m.host.querySelector<HTMLButtonElement>(`button[aria-label="${t("ui-data-pane-close")}"]`)!;
    await act(async () => close.click());
    expect(m.calls).toEqual(["close"]);
  });
});
