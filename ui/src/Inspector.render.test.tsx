// The inspector tab: the deck's theme without a selection, the element's settings, its Data button, an offer first.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { ReactNode } from "react";
import type { HostSettings } from "./useHostSettings";

// See ./a2ui/catalog.render.test.tsx: React loads after the DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { Inspector } = await import("./Inspector");
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

const settings = (surface: HostSettings["surface"]): HostSettings => ({ surface, onAction: () => true, outcome: { current: null } });

interface Opts { element?: HostSettings; data?: boolean; theme?: string | null; offer?: ReactNode }

async function mount(o: Opts = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const calls: string[] = [];
  await act(async () =>
    root.render(
      createElement(Inspector, {
        element: o.element ?? settings(null),
        data: !!o.data,
        onData: () => void calls.push("data"),
        theme: o.theme === undefined ? "Papier" : o.theme,
        offer: o.offer,
        onAction: () => true,
        onError: () => {},
      }),
    ),
  );
  // SurfaceView loads separately.
  for (let i = 0; i < 50 && o.element?.surface && !host.querySelector("[data-surface-id] select, [data-surface-id] button"); i++) {
    await act(() => new Promise((r) => setTimeout(r, 10)));
  }
  return { host, calls };
}

const button = (host: HTMLElement, label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.getAttribute("aria-label") === label || b.textContent === label);

describe("Inspector", () => {
  test("Inspector without a selection shows the theme name", async () => {
    const m = await mount();
    expect(m.host.querySelector(".inspector")?.textContent).toContain("Papier");
  });

  test("Inspector with an untitled theme says so", async () => {
    const m = await mount({ theme: null });
    expect(m.host.querySelector(".inspector")?.textContent).toContain(t("ui-inspector-no-theme"));
  });

  test("Inspector with an element shows its host-element surface", async () => {
    const m = await mount({ element: settings(realSurface("host-element")) });
    expect([!!m.host.querySelector('[data-surface-id="host-element"]'), m.host.querySelector(".inspector")?.textContent?.includes("Papier")]).toEqual([true, false]);
  });

  test("Inspector for a chart or table shows a Data button calling onData", async () => {
    const m = await mount({ element: settings(realSurface("host-element")), data: true });
    await act(async () => button(m.host, t("ui-inspector-data"))!.click());
    expect(m.calls).toEqual(["data"]);
  });

  test("Inspector without data shows no Data button", async () => {
    const m = await mount({ element: settings(realSurface("host-element")) });
    expect(button(m.host, t("ui-inspector-data"))).toBeUndefined();
  });

  test("Inspector puts an offer before the element's settings, in its labelled frame", async () => {
    const m = await mount({ element: settings(realSurface("host-element")), offer: createElement("p", { className: "offered" }, "offre") });
    const frame = m.host.querySelector(`[aria-label="${t("ui-inspector-offer")}"]`);
    const settingsBox = m.host.querySelector('[data-surface-id="host-element"]')!;
    expect(!!frame?.querySelector(".offered")).toBe(true);
    expect(frame!.compareDocumentPosition(settingsBox) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
