// Rendu du bandeau de l'aide dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const HelpBar = (await import("./HelpBar")).default;
const { t } = await import("./i18n");
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

const SLIDES = [{ id: "a" }, { id: "b" }, { id: "c" }];

async function mount(current: string | null, calls: string[] = [], closable = true) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const props = {
    title: "Oku Slides",
    slides: SLIDES,
    current,
    onShow: (id: string) => calls.push(id),
    search: async () => [],
    onClose: closable ? () => calls.push("close") : undefined,
  };
  await act(async () => root.render(createElement(HelpBar, props)));
  return host;
}

function button(host: HTMLElement, key: string): HTMLButtonElement | null {
  return host.querySelector<HTMLButtonElement>(`button[aria-label="${t(key)}"]`);
}

async function press(key: string, target: EventTarget = window) {
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  });
}

describe("HelpBar", () => {
  test("HelpBar buttons show icons only", async () => {
    const host = await mount("b");
    for (const k of ["ui-slide-previous", "ui-slide-next", "ui-help-close"]) expectIconOnly(button(host, k)!);
  });

  test("HelpBar caption counts slides from one", async () => {
    const host = await mount("b");
    expect(host.querySelector(".caption")?.textContent).toBe(t("ui-help-caption", { index: 2, count: 3 }));
  });

  test("HelpBar before the first slide is known shows slide one", async () => {
    const host = await mount(null);
    expect(host.querySelector(".caption")?.textContent).toBe(t("ui-help-caption", { index: 1, count: 3 }));
  });

  test("HelpBar on the first slide disables previous only", async () => {
    const host = await mount("a");
    expect(button(host, "ui-slide-previous")!.disabled).toBe(true);
    expect(button(host, "ui-slide-next")!.disabled).toBe(false);
  });

  test("HelpBar on the last slide disables next", async () => {
    const host = await mount("c");
    expect(button(host, "ui-slide-next")!.disabled).toBe(true);
  });

  test("HelpBar buttons show the neighbours and close", async () => {
    const calls: string[] = [];
    const host = await mount("b", calls);
    await act(async () => {
      button(host, "ui-slide-previous")!.click();
      button(host, "ui-slide-next")!.click();
      button(host, "ui-help-close")!.click();
    });
    expect(calls).toEqual(["a", "c", "close"]);
  });

  test("HelpBar without onClose has no close button", async () => {
    const host = await mount("b", [], false);
    expect(button(host, "ui-help-close")).toBeNull();
  });

  test("HelpBar arrow keys show the neighbour slide", async () => {
    const calls: string[] = [];
    await mount("b", calls);
    await press("ArrowRight");
    await press("ArrowLeft");
    expect(calls).toEqual(["c", "a"]);
  });

  test("HelpBar Space on a focused button is left to the button", async () => {
    const calls: string[] = [];
    const host = await mount("b", calls);
    await press(" ", button(host, "ui-help-close")!);
    expect(calls).toEqual([]);
  });
});
