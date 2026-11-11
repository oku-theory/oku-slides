// The data pane: the selected chart's or table's host-data surface, its close and maximize buttons
// and its height divider.

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

async function mount(height: number | null = null) {
  const host = document.createElement("div");
  host.className = "sheet-area";
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  const heights: (number | null)[] = [];
  const render = (surface: ReturnType<typeof realSurface> | null) => act(async () =>
    root.render(
      createElement(DataPane, {
        data: { surface, onAction: () => true, outcome: { current: null } },
        onAction: () => true,
        onError: () => {},
        onClose: () => void calls.push("close"),
        height,
        onHeight: (h: number | null) => void heights.push(h),
      }),
    ),
  );
  await render(realSurface("host-data"));
  return { host, calls, heights, render };
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

  test("DataPane kept height sets the sheet's data row", async () => {
    const m = await mount(280);
    expect([m.host.style.getPropertyValue("--data-h"), m.host.querySelector(".data-pane.sized") !== null]).toEqual(["280px", true]);
  });

  test("DataPane default height leaves the sheet's data row to its content", async () => {
    const m = await mount();
    expect([m.host.style.getPropertyValue("--data-h"), m.host.querySelector(".data-pane.sized") !== null]).toEqual(["", false]);
  });

  test("DataPane maximize button gives the pane the stage's room and toggles back", async () => {
    const m = await mount();
    const button = () => m.host.querySelector<HTMLButtonElement>(".data-pane-max")!;
    await act(async () => button().click());
    const maximized = [m.host.querySelector(".data-pane.max") !== null, button().getAttribute("aria-label")];
    await act(async () => button().click());
    expect([...maximized, m.host.querySelector(".data-pane.max") !== null]).toEqual([true, t("ui-data-pane-restore"), false]);
  });

  test("DataPane divider double-click resets the height to the default", async () => {
    const m = await mount(280);
    const divider = m.host.querySelector(`[role="separator"][aria-label="${t("ui-data-pane-resize")}"]`)!;
    await act(async () => void divider.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(m.heights).toEqual([null]);
  });

  test("DataPane maximized then without a surface opens the next one at its kept height", async () => {
    const m = await mount();
    await act(async () => m.host.querySelector<HTMLButtonElement>(".data-pane-max")!.click());
    await m.render(null);
    await m.render(realSurface("host-data"));
    expect(m.host.querySelector(".data-pane.max")).toBeNull();
  });
});
